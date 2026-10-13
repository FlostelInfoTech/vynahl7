//! Inbox polling: download, parse leniently, settle.

use std::collections::{HashMap, HashSet};

use crate::intake::Intake;
use crate::{BatchError, FileEntry, FileTransport};

/// A consumed inbox file with its lenient parse result.
#[derive(Debug)]
pub struct PolledFile {
    /// Transport-relative path the file was read from.
    pub name: String,
    /// Raw file bytes, kept for auditing or reprocessing.
    pub bytes: Vec<u8>,
    /// Lenient parse result; individual message failures stay inside.
    pub intake: Intake,
    /// Trailer count mismatch, populated only when the poller was built with
    /// [`Poller::with_verify_counts`] and the counts disagree.
    pub count_error: Option<BatchError>,
}

impl PolledFile {
    /// Number of successfully parsed messages.
    pub fn ok_count(&self) -> usize {
        self.intake.ok_count()
    }

    /// Number of failed message slots.
    pub fn error_count(&self) -> usize {
        self.intake.error_count()
    }
}

/// An inbox file the poller refused to consume.
#[derive(Debug, Clone)]
pub struct SkippedFile {
    /// Transport-relative path.
    pub path: String,
    /// Why it was skipped (unstable, unparseable, ...).
    pub reason: String,
}

/// A per-file operational failure (download or settle). The poll continued
/// with the remaining files; failed files stay unsettled and are retried
/// on the next poll.
#[derive(Debug, Clone)]
pub struct PollFileError {
    /// Transport-relative path.
    pub path: String,
    /// What failed.
    pub reason: String,
}

/// Outcome of one [`Poller::poll_once`] call.
#[derive(Debug, Default)]
pub struct PollReport {
    /// Consumed files. Content is always delivered here even when the
    /// subsequent settle failed (see `errors`): delivery is at-least-once.
    pub files: Vec<PolledFile>,
    /// Files seen but not consumed.
    pub skipped: Vec<SkippedFile>,
    /// Per-file operational failures; the poll continued past them.
    pub errors: Vec<PollFileError>,
}

/// Identity of a consumed file. A redrop at the same path with a different
/// size or mtime is a new file and must be reprocessed, so the key carries
/// the listing metadata — never the path alone.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct SettledKey {
    path: String,
    size: Option<u64>,
    modified_ms: Option<u64>,
}

impl SettledKey {
    fn of(entry: &FileEntry) -> Self {
        Self {
            path: entry.path.clone(),
            size: entry.size,
            modified_ms: entry.modified_ms,
        }
    }
}

/// Poll an inbox directory for batch files.
///
/// Each poll lists the inbox, downloads stable files, parses them with
/// [`Intake`], and settles what was consumed: archived when an archive dir is
/// configured, deleted when `delete_after` is set (the default), otherwise
/// left in place and remembered for the session so it is not reprocessed.
///
/// Stability (size and, when the transport reports one, modification time
/// unchanged across two consecutive polls) guards against reading files that
/// are still being written; disable it for transports that publish
/// atomically.
///
/// Delivery is at-least-once: consumed content is always returned in the
/// report even when settling fails (see [`PollReport::errors`]), and files
/// that fail to download or settle stay unsettled for retry. A redrop at an
/// already-consumed path is treated as a new file when its size or mtime
/// differs.
///
/// Same-second, same-size redrops are a known blind spot: FTP/SFTP
/// timestamps have 1-second granularity, so a redrop that lands within
/// the same second with the same byte count keeps the consumed key and
/// is not reprocessed. Content-aware redetection would require
/// re-downloading every settled file on every poll, which defeats the
/// settled cache; senders that redrop in place should change the size
/// (or wait a second) when the content changed.
pub struct Poller<T> {
    transport: T,
    inbox: String,
    archive_dir: Option<String>,
    error_dir: Option<String>,
    delete_after: bool,
    suffix_filter: Option<String>,
    require_stable: bool,
    verify_counts: bool,
    max_files_per_poll: usize,
    max_bytes_per_poll: u64,
    max_file_bytes: u64,
    seen_stability: HashMap<String, (Option<u64>, Option<u64>)>,
    settled: HashSet<SettledKey>,
}

/// Default cap (256 MiB) on the raw bytes one [`Poller::poll_once`] call
/// consumes. Parsed [`Intake`]s are retained alongside the bytes, so the
/// cap bounds total retained memory per poll to a small multiple of this.
const DEFAULT_MAX_BYTES_PER_POLL: u64 = 256 * 1024 * 1024;

impl<T: FileTransport> Poller<T> {
    /// Poll `inbox` ("", the transport root, by default) with deletion after
    /// consumption and stability checks enabled.
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            inbox: String::new(),
            archive_dir: None,
            error_dir: None,
            delete_after: true,
            suffix_filter: None,
            require_stable: true,
            verify_counts: true,
            max_files_per_poll: crate::DEFAULT_MAX_FILES_PER_POLL,
            max_bytes_per_poll: DEFAULT_MAX_BYTES_PER_POLL,
            max_file_bytes: crate::DEFAULT_MAX_FILE_BYTES,
            seen_stability: HashMap::new(),
            settled: HashSet::new(),
        }
    }

    /// Inbox directory to list (default: transport root).
    ///
    /// # Panics
    ///
    /// Panics when the inbox equals a previously configured archive or
    /// error dir (compared after normalization).
    pub fn with_inbox(mut self, inbox: &str) -> Self {
        if let Some(dir) = &self.archive_dir {
            assert!(
                normalize_dir(dir) != normalize_dir(inbox),
                "inbox ({inbox:?}) must differ from archive_dir: settling into \
                 the inbox re-consumes versioned files forever"
            );
        }
        if let Some(dir) = &self.error_dir {
            assert!(
                normalize_dir(dir) != normalize_dir(inbox),
                "inbox ({inbox:?}) must differ from error_dir: quarantining \
                 into the inbox re-consumes versioned files forever"
            );
        }
        self.inbox = inbox.to_string();
        self
    }

    /// Move consumed files under this directory (preserving their path
    /// relative to the inbox) instead of deleting them. When the destination
    /// already holds a file, the newcomer is versioned (`name.1`, ...) so a
    /// redrop never destroys a prior archive.
    ///
    /// # Panics
    ///
    /// Panics when the dir equals the inbox (compared after
    /// normalization): archiving into the inbox re-consumes the
    /// versioned files (`a.hl7.1`, `a.hl7.1.1`, ...) forever.
    pub fn with_archive_dir(mut self, dir: &str) -> Self {
        assert!(
            normalize_dir(dir) != normalize_dir(&self.inbox),
            "archive_dir ({dir:?}) must differ from inbox: archiving into the \
             inbox re-consumes versioned files forever"
        );
        self.archive_dir = Some(dir.to_string());
        self
    }

    /// Move unparseable files under this directory instead of leaving them in
    /// the inbox. Files whose every message failed to parse are quarantined
    /// here too: they carry nothing deliverable, so settling them as
    /// consumed would silently drop the failure.
    ///
    /// # Panics
    ///
    /// Panics when the dir equals the inbox (compared after
    /// normalization): quarantining into the inbox re-consumes the
    /// versioned files forever.
    pub fn with_error_dir(mut self, dir: &str) -> Self {
        assert!(
            normalize_dir(dir) != normalize_dir(&self.inbox),
            "error_dir ({dir:?}) must differ from inbox: quarantining into \
             the inbox re-consumes versioned files forever"
        );
        self.error_dir = Some(dir.to_string());
        self
    }

    /// Delete consumed files when no archive dir is configured (default true).
    pub fn with_delete_after(mut self, delete: bool) -> Self {
        self.delete_after = delete;
        self
    }

    /// Only consume files whose name ends with this suffix (e.g. `".hl7"`).
    pub fn with_suffix_filter(mut self, suffix: &str) -> Self {
        self.suffix_filter = Some(suffix.to_string());
        self
    }

    /// Require a file's size and modification time (when the transport
    /// reports one) to be unchanged across two consecutive polls before
    /// consuming it (default true). Transports that publish atomically
    /// can disable this.
    pub fn with_require_stable(mut self, require: bool) -> Self {
        self.require_stable = require;
        self
    }

    /// Record BTS/FTS count mismatches on [`PolledFile::count_error`]
    /// (default true). Mismatches never block consumption.
    pub fn with_verify_counts(mut self, verify: bool) -> Self {
        self.verify_counts = verify;
        self
    }

    /// Consume at most this many files per [`Poller::poll_once`] call
    /// (default [`DEFAULT_MAX_FILES_PER_POLL`](crate::DEFAULT_MAX_FILES_PER_POLL)).
    /// Leftovers stay for the next poll, so this bounds memory and work
    /// without losing files.
    pub fn with_max_files_per_poll(mut self, max: usize) -> Self {
        self.max_files_per_poll = max;
        self
    }

    /// Consume at most this many raw bytes per [`Poller::poll_once`] call
    /// (default 256 MiB). File counts alone cannot bound memory (100
    /// files at the 64 MiB per-file cap would retain gigabytes of bytes
    /// plus parsed [`Intake`]s), so the poll stops once the next file
    /// would exceed the budget. Leftovers stay for the next poll without
    /// losing files; a lone file bigger than the remaining budget is
    /// still consumed when the poll has taken nothing yet, so one large
    /// file can never stall the inbox.
    pub fn with_max_bytes_per_poll(mut self, max: u64) -> Self {
        self.max_bytes_per_poll = max;
        self
    }

    /// Refuse files larger than this (default
    /// [`DEFAULT_MAX_FILE_BYTES`](crate::DEFAULT_MAX_FILE_BYTES)). Oversize
    /// files are reported in [`PollReport::skipped`] and remembered for the
    /// session so they are not downloaded again.
    pub fn with_max_file_bytes(mut self, max: u64) -> Self {
        self.max_file_bytes = max;
        self
    }

    /// Borrow the underlying transport (e.g. to [`FileTransport::close`]).
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Settle destination that never destroys a prior file: when `dest`
    /// already exists, suffix `.1`, `.2`, ... until free. A redrop at an
    /// inbox path must not silently wipe the earlier archive (or the
    /// earlier quarantine) holding the same name.
    async fn free_dest(&mut self, dest: &str) -> Result<String, BatchError> {
        let parent = dest.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        let listed = self.transport.list(parent).await?;
        if !listed.iter().any(|entry| entry.path == dest) {
            return Ok(dest.to_string());
        }
        for n in 1.. {
            let candidate = format!("{dest}.{n}");
            if !listed.iter().any(|entry| entry.path == candidate) {
                return Ok(candidate);
            }
        }
        unreachable!("version suffixes are unbounded")
    }

    /// Move a poison file to the error dir (when configured) and record it
    /// as skipped. A file that failed quarantine stays unsettled for retry.
    async fn quarantine(&mut self, entry: &FileEntry, report: &mut PollReport, reason: String) {
        if let Some(dir) = &self.error_dir {
            let dest = join_dir(dir, &relative_to(&self.inbox, &entry.path));
            let dest = match self.free_dest(&dest).await {
                Ok(free) => free,
                Err(e) => {
                    report.errors.push(PollFileError {
                        path: entry.path.clone(),
                        reason: format!("quarantine failed: {e}"),
                    });
                    return;
                }
            };
            if let Err(e) = self.transport.rename(&entry.path, &dest).await {
                report.errors.push(PollFileError {
                    path: entry.path.clone(),
                    reason: format!("quarantine failed: {e}"),
                });
                return;
            }
        }
        self.settled.insert(SettledKey::of(entry));
        report.skipped.push(SkippedFile {
            path: entry.path.clone(),
            reason,
        });
    }

    /// List, download, parse, and settle inbox files.
    ///
    /// Only the initial inbox listing aborts the poll with an error (nothing
    /// was consumed yet). Per-file download/parse/settle failures are
    /// collected into the report and the poll continues: unparseable files
    /// (and files whose every message failed) are quarantined (or left in
    /// place) and reported in [`PollReport::skipped`], operational failures
    /// in [`PollReport::errors`], never fatal.
    pub async fn poll_once(&mut self) -> Result<PollReport, BatchError> {
        let mut report = PollReport::default();
        let mut bytes_used: u64 = 0;
        let entries = self.transport.list(&self.inbox).await?;

        // Forget state for paths that left the inbox (consumed elsewhere,
        // rotated away): this bounds memory and lets redrops process.
        let listed: HashSet<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        self.settled.retain(|k| listed.contains(k.path.as_str()));
        self.seen_stability
            .retain(|p, _| listed.contains(p.as_str()));

        for entry in entries {
            if report.files.len() >= self.max_files_per_poll {
                break;
            }
            if bytes_used >= self.max_bytes_per_poll {
                break;
            }
            if self.settled.contains(&SettledKey::of(&entry)) {
                report.skipped.push(SkippedFile {
                    path: entry.path.clone(),
                    reason: "already settled".to_string(),
                });
                continue;
            }
            if let Some(suffix) = &self.suffix_filter
                && !entry.path.ends_with(suffix)
            {
                continue;
            }
            // Refuse huge files before downloading when the size is known.
            if let Some(size) = entry.size
                && size > self.max_file_bytes
            {
                self.settled.insert(SettledKey::of(&entry));
                report.skipped.push(SkippedFile {
                    path: entry.path.clone(),
                    reason: format!("file size {size} exceeds limit of {}", self.max_file_bytes),
                });
                continue;
            }
            // Stability covers size-unknown entries too (FTP's NLST
            // fallback reports no size): then the mtime alone must repeat
            // across two polls before the file is consumed.
            if self.require_stable
                && self.seen_stability.get(&entry.path) != Some(&(entry.size, entry.modified_ms))
            {
                let seen = self
                    .seen_stability
                    .insert(entry.path.clone(), (entry.size, entry.modified_ms));
                // Same size but a moved mtime means a stalled slow writer:
                // the size-only check would consume it mid-write.
                let reason = if seen.is_some_and(|(prev_size, _)| prev_size == entry.size) {
                    "modification time not yet stable"
                } else if entry.size.is_some() {
                    "size not yet stable"
                } else {
                    "metadata not yet stable"
                };
                report.skipped.push(SkippedFile {
                    path: entry.path.clone(),
                    reason: reason.to_string(),
                });
                continue;
            }
            // Byte budget: stop before a download that would exceed it,
            // unless nothing was consumed yet (a lone over-budget file
            // must still make progress instead of stalling the inbox).
            if bytes_used > 0
                && entry
                    .size
                    .is_some_and(|size| bytes_used + size > self.max_bytes_per_poll)
            {
                break;
            }

            let bytes = match self.transport.get(&entry.path).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    report.errors.push(PollFileError {
                        path: entry.path.clone(),
                        reason: format!("download failed: {e}"),
                    });
                    continue;
                }
            };
            // The transport may not report sizes; enforce the cap here too.
            if bytes.len() as u64 > self.max_file_bytes {
                self.settled.insert(SettledKey::of(&entry));
                report.skipped.push(SkippedFile {
                    path: entry.path.clone(),
                    reason: format!(
                        "file size {} exceeds limit of {}",
                        bytes.len(),
                        self.max_file_bytes
                    ),
                });
                continue;
            }
            let intake = match Intake::parse_bytes(&bytes) {
                Ok(intake) => intake,
                Err(e) => {
                    self.quarantine(&entry, &mut report, format!("unparseable: {e}"))
                        .await;
                    continue;
                }
            };
            // Every message failed: nothing deliverable, so quarantine the
            // file instead of settling it as consumed.
            if intake.message_count() > 0 && intake.ok_count() == 0 {
                self.quarantine(
                    &entry,
                    &mut report,
                    format!("all {} message(s) failed to parse", intake.message_count()),
                )
                .await;
                continue;
            }

            let count_error = if self.verify_counts {
                intake.verify_counts().err()
            } else {
                None
            };
            bytes_used = bytes_used.saturating_add(bytes.len() as u64);
            report.files.push(PolledFile {
                name: entry.path.clone(),
                bytes,
                intake,
                count_error,
            });

            // Content is already in the report: a settle failure must not
            // discard it. The file stays unsettled for at-least-once retry.
            let settled = if let Some(dir) = &self.archive_dir {
                let dest = join_dir(dir, &relative_to(&self.inbox, &entry.path));
                match self.free_dest(&dest).await {
                    Ok(free) => match self.transport.rename(&entry.path, &free).await {
                        Ok(()) => true,
                        Err(e) => {
                            report.errors.push(PollFileError {
                                path: entry.path.clone(),
                                reason: format!("archive failed: {e}"),
                            });
                            false
                        }
                    },
                    Err(e) => {
                        report.errors.push(PollFileError {
                            path: entry.path.clone(),
                            reason: format!("archive failed: {e}"),
                        });
                        false
                    }
                }
            } else if self.delete_after {
                match self.transport.delete(&entry.path).await {
                    Ok(()) => true,
                    Err(e) => {
                        report.errors.push(PollFileError {
                            path: entry.path.clone(),
                            reason: format!("delete failed: {e}"),
                        });
                        false
                    }
                }
            } else {
                true
            };
            if settled {
                self.settled.insert(SettledKey::of(&entry));
                self.seen_stability.remove(&entry.path);
            }
        }

        Ok(report)
    }
}

/// Transport dirs are `/`-separated; surrounding slashes are
/// insignificant, so `inbox`, `/inbox`, and `inbox/` name one directory.
fn normalize_dir(dir: &str) -> &str {
    dir.trim_matches('/')
}

/// Path of `path` relative to inbox `base` (both `/`-separated).
fn relative_to(base: &str, path: &str) -> String {
    let base = base.trim_matches('/');
    if base.is_empty() {
        return path.to_string();
    }
    path.strip_prefix(base)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(path)
        .to_string()
}

fn join_dir(dir: &str, relative: &str) -> String {
    let dir = dir.trim_matches('/');
    if dir.is_empty() {
        relative.to_string()
    } else {
        format!("{dir}/{relative}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::FilesystemTransport;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn scratch_dir() -> std::path::PathBuf {
        let seq = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("vyna-poller-test-{}-{seq}", std::process::id()))
    }

    const MSH: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5";

    #[tokio::test]
    async fn poll_consumes_stable_files_and_archives() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/a.hl7", MSH.as_bytes()).await.unwrap();
        t.put("inbox/notes.txt", b"not hl7").await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_archive_dir("archive")
            .with_suffix_filter(".hl7");
        // First poll: file not yet stable.
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
        assert_eq!(report.skipped.len(), 1);

        // Second poll: stable, consumed, archived.
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].name, "inbox/a.hl7");
        assert_eq!(report.files[0].ok_count(), 1);
        assert!(report.files[0].count_error.is_none());

        let t = poller.transport_mut();
        assert!(
            t.list("inbox")
                .await
                .unwrap()
                .iter()
                .any(|e| e.path == "inbox/notes.txt")
        );
        assert_eq!(t.get("archive/a.hl7").await.unwrap(), MSH.as_bytes());

        // Third poll: nothing new (settled + suffix filter).
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn unparseable_files_quarantine_without_aborting_poll() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/good.hl7", MSH.as_bytes()).await.unwrap();
        t.put("inbox/bad.hl7", b"\xff\xfe").await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_error_dir("error")
            .with_require_stable(false);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].name, "inbox/good.hl7");
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "inbox/bad.hl7");

        let t = poller.transport_mut();
        assert_eq!(t.get("error/bad.hl7").await.unwrap(), b"\xff\xfe");
        // Consumed without archive dir: deleted by default.
        assert!(t.list("inbox").await.unwrap().is_empty());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn all_failed_files_route_to_error_dir() {
        // A bare "MSH" line parses structurally but its only message
        // fails: 100% failed, nothing deliverable.
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/broken.hl7", b"MSH").await.unwrap();
        t.put("inbox/good.hl7", MSH.as_bytes()).await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_error_dir("error")
            .with_require_stable(false);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].name, "inbox/good.hl7");
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "inbox/broken.hl7");
        assert!(report.skipped[0].reason.contains("failed"));

        let t = poller.transport_mut();
        assert_eq!(t.get("error/broken.hl7").await.unwrap(), b"MSH");

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn all_failed_files_settle_in_place_without_error_dir() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/broken.hl7", b"MSH").await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_require_stable(false);
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "inbox/broken.hl7");

        // Left in the inbox but remembered for the session.
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].reason, "already settled");

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn max_files_per_poll_bounds_work_without_losing_files() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/a.hl7", MSH.as_bytes()).await.unwrap();
        t.put("inbox/b.hl7", MSH.as_bytes()).await.unwrap();
        t.put("inbox/c.hl7", MSH.as_bytes()).await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_require_stable(false)
            .with_max_files_per_poll(2);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 2);
        // The leftover is picked up on the next poll.
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn max_bytes_per_poll_stops_before_blowing_the_budget() {
        // Three files at exactly the budget each: one consumed per poll,
        // leftovers waiting without loss.
        let size = MSH.len() as u64;
        let entry = |name: &str| FileEntry {
            path: format!("inbox/{name}"),
            size: Some(size),
            modified_ms: Some(1),
        };
        let transport = ScriptedTransport {
            listings: vec![vec![entry("a.hl7"), entry("b.hl7"), entry("c.hl7")]],
            polls: 0,
            body: MSH.as_bytes().to_vec(),
        };
        let mut poller = Poller::new(transport)
            .with_inbox("inbox")
            .with_require_stable(false)
            .with_delete_after(false)
            .with_max_bytes_per_poll(size);

        for expected in ["inbox/a.hl7", "inbox/b.hl7", "inbox/c.hl7"] {
            let report = poller.poll_once().await.unwrap();
            assert_eq!(report.files.len(), 1);
            assert_eq!(report.files[0].name, expected);
        }
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
    }

    #[tokio::test]
    async fn oversize_files_are_skipped_once_and_never_downloaded_again() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/small.hl7", MSH.as_bytes()).await.unwrap();
        t.put("inbox/big.hl7", &[b'x'; 4096]).await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_require_stable(false)
            .with_max_file_bytes(1024);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].name, "inbox/small.hl7");
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "inbox/big.hl7");
        assert!(report.skipped[0].reason.contains("exceeds limit"));

        // Settled: the oversize file is not re-downloaded. Re-listings
        // are still reported, as already settled, instead of silently
        // skipped.
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "inbox/big.hl7");
        assert_eq!(report.skipped[0].reason, "already settled");

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[test]
    fn count_mismatch_is_recorded_not_fatal() {
        let intake = Intake::parse(
            "BHS|^~\\&|S|F|R|D|20240101||||B1\rMSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rBTS|9",
        )
        .unwrap();
        assert!(matches!(
            intake.verify_counts(),
            Err(BatchError::CountMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn archive_collision_preserves_prior_archive() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("archive/feed.hl7", b"OLD-ARCHIVE-BYTES")
            .await
            .unwrap();
        t.put("inbox/feed.hl7", MSH.as_bytes()).await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_archive_dir("archive")
            .with_require_stable(false);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);

        // The prior archive must survive; the new bytes land versioned
        // alongside instead of destroying it.
        let t = poller.transport_mut();
        assert_eq!(
            t.get("archive/feed.hl7").await.unwrap(),
            b"OLD-ARCHIVE-BYTES"
        );
        let archived: Vec<_> = t
            .list("archive")
            .await
            .unwrap()
            .iter()
            .map(|e| e.path.clone())
            .collect();
        assert_eq!(archived.len(), 2, "new bytes lost: {archived:?}");
        assert!(t.list("inbox").await.unwrap().is_empty());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn redrop_at_consumed_path_is_reprocessed() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/feed.hl7", MSH.as_bytes()).await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_require_stable(false);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);

        // Hourly redrop at the same path with different content (size).
        let bigger = format!("{MSH}\rPID|1||BIGGEST_ID_VALUE_HERE");
        poller
            .transport_mut()
            .put("inbox/feed.hl7", bigger.as_bytes())
            .await
            .unwrap();
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].bytes, bigger.as_bytes());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    /// Transport wrapper that fails downloads for one path.
    struct FailGet<T> {
        inner: T,
        fail_path: String,
    }

    impl<T: FileTransport + Send> FileTransport for FailGet<T> {
        async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
            self.inner.list(dir).await
        }

        async fn get(&mut self, path: &str) -> Result<Vec<u8>, BatchError> {
            if path == self.fail_path {
                return Err(BatchError::Invalid("boom".to_string()));
            }
            self.inner.get(path).await
        }

        async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
            self.inner.put(path, data).await
        }

        async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
            self.inner.delete(path).await
        }

        async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
            self.inner.rename(from, to).await
        }
    }

    #[tokio::test]
    async fn download_failure_keeps_consumed_content_and_continues() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/good.hl7", MSH.as_bytes()).await.unwrap();
        t.put("inbox/bad.hl7", MSH.as_bytes()).await.unwrap();

        let failing = FailGet {
            inner: t,
            fail_path: "inbox/bad.hl7".to_string(),
        };
        let mut poller = Poller::new(failing)
            .with_inbox("inbox")
            .with_require_stable(false);
        // No abort: the good file's content is delivered alongside the error.
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].name, "inbox/good.hl7");
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].path, "inbox/bad.hl7");
        assert!(report.errors[0].reason.contains("download failed"));

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    /// Transport with scripted listings (one per poll) for stability tests.
    struct ScriptedTransport {
        listings: Vec<Vec<FileEntry>>,
        polls: usize,
        body: Vec<u8>,
    }

    impl FileTransport for ScriptedTransport {
        async fn list(&mut self, _dir: &str) -> Result<Vec<FileEntry>, BatchError> {
            let index = self.polls.min(self.listings.len() - 1);
            self.polls += 1;
            Ok(self.listings[index].clone())
        }

        async fn get(&mut self, _path: &str) -> Result<Vec<u8>, BatchError> {
            Ok(self.body.clone())
        }

        async fn put(&mut self, _path: &str, _data: &[u8]) -> Result<(), BatchError> {
            Ok(())
        }

        async fn delete(&mut self, _path: &str) -> Result<(), BatchError> {
            Ok(())
        }

        async fn rename(&mut self, _from: &str, _to: &str) -> Result<(), BatchError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn stability_gate_tracks_mtime_not_just_size() {
        let entry = |modified_ms| FileEntry {
            path: "inbox/slow.hl7".to_string(),
            size: Some(MSH.len() as u64),
            modified_ms,
        };
        let transport = ScriptedTransport {
            listings: vec![
                vec![entry(Some(1000))],
                vec![entry(Some(2000))], // same size, newer mtime: still writing
                vec![entry(Some(2000))], // now stable
            ],
            polls: 0,
            body: MSH.as_bytes().to_vec(),
        };
        let mut poller = Poller::new(transport)
            .with_inbox("inbox")
            .with_delete_after(false);

        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty()); // first sighting: unstable

        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty(), "mtime moved: still being written");
        assert_eq!(report.skipped.len(), 1);

        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1); // size and mtime stable: consumed
    }

    #[tokio::test]
    async fn stability_gate_tracks_entries_with_unknown_size() {
        // FTP's NLST fallback reports no size: those entries must still
        // wait out the stability gate (mtime-only) instead of being
        // consumed on first sight.
        let entry = |modified_ms| FileEntry {
            path: "inbox/nlst.hl7".to_string(),
            size: None,
            modified_ms,
        };
        let transport = ScriptedTransport {
            listings: vec![
                vec![entry(Some(1000))],
                vec![entry(Some(2000))], // mtime moved: still writing
                vec![entry(Some(2000))], // now stable
            ],
            polls: 0,
            body: MSH.as_bytes().to_vec(),
        };
        let mut poller = Poller::new(transport)
            .with_inbox("inbox")
            .with_delete_after(false);

        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty(), "first sighting: unstable");

        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty(), "mtime moved: still writing");

        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1, "stable mtime: consumed");
    }

    #[tokio::test]
    async fn unknown_size_files_hit_the_cap_when_bytes_materialize() {
        // Size unknown at listing time, but the download itself exceeds
        // the cap: skipped, settled, never consumed.
        let transport = ScriptedTransport {
            listings: vec![vec![FileEntry {
                path: "inbox/big.hl7".to_string(),
                size: None,
                modified_ms: None,
            }]],
            polls: 0,
            body: vec![b'x'; 2048],
        };
        let mut poller = Poller::new(transport)
            .with_inbox("inbox")
            .with_require_stable(false)
            .with_max_file_bytes(1024);

        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].reason.contains("exceeds limit"));
    }

    fn scripted() -> ScriptedTransport {
        ScriptedTransport {
            listings: vec![vec![]],
            polls: 0,
            body: vec![],
        }
    }

    #[test]
    #[should_panic(expected = "must differ from inbox")]
    fn archive_dir_equal_to_inbox_is_rejected() {
        let _ = Poller::new(scripted())
            .with_inbox("inbox")
            .with_archive_dir("inbox");
    }

    #[test]
    #[should_panic(expected = "must differ from inbox")]
    fn archive_dir_equal_to_inbox_after_normalization_is_rejected() {
        let _ = Poller::new(scripted())
            .with_inbox("/inbox/")
            .with_archive_dir("inbox");
    }

    #[test]
    #[should_panic(expected = "must differ from inbox")]
    fn error_dir_equal_to_inbox_is_rejected() {
        let _ = Poller::new(scripted())
            .with_inbox("inbox")
            .with_error_dir("inbox/");
    }

    #[test]
    #[should_panic(expected = "must differ from archive_dir")]
    fn inbox_equal_to_archive_dir_is_rejected_regardless_of_order() {
        let _ = Poller::new(scripted())
            .with_archive_dir("inbox")
            .with_inbox("inbox");
    }

    #[test]
    fn distinct_dirs_build_without_panicking() {
        // Sibling and nested dirs settle outside the inbox listing.
        let _ = Poller::new(scripted())
            .with_inbox("inbox")
            .with_archive_dir("archive")
            .with_error_dir("error");
        let _ = Poller::new(scripted())
            .with_inbox("inbox")
            .with_archive_dir("inbox/archive");
    }

    #[tokio::test]
    async fn settled_files_are_reported_not_silently_skipped() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/a.hl7", MSH.as_bytes()).await.unwrap();

        let mut poller = Poller::new(t)
            .with_inbox("inbox")
            .with_require_stable(false)
            .with_delete_after(false);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);

        // Still listed with the same key: reported as already settled.
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].path, "inbox/a.hl7");

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}
