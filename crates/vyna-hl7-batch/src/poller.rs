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
/// Stability (size unchanged across two consecutive polls) guards against
/// reading files that are still being written; disable it for transports that
/// publish atomically.
///
/// Delivery is at-least-once: consumed content is always returned in the
/// report even when settling fails (see [`PollReport::errors`]), and files
/// that fail to download or settle stay unsettled for retry. A redrop at an
/// already-consumed path is treated as a new file when its size or mtime
/// differs.
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
    max_file_bytes: u64,
    seen_sizes: HashMap<String, u64>,
    settled: HashSet<SettledKey>,
}

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
            max_file_bytes: crate::DEFAULT_MAX_FILE_BYTES,
            seen_sizes: HashMap::new(),
            settled: HashSet::new(),
        }
    }

    /// Inbox directory to list (default: transport root).
    pub fn with_inbox(mut self, inbox: &str) -> Self {
        self.inbox = inbox.to_string();
        self
    }

    /// Move consumed files under this directory (preserving their path
    /// relative to the inbox) instead of deleting them.
    pub fn with_archive_dir(mut self, dir: &str) -> Self {
        self.archive_dir = Some(dir.to_string());
        self
    }

    /// Move unparseable files under this directory instead of leaving them in
    /// the inbox.
    pub fn with_error_dir(mut self, dir: &str) -> Self {
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

    /// Require a file's size to be unchanged across two consecutive polls
    /// before consuming it (default true). Transports that publish atomically
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

    /// List, download, parse, and settle inbox files.
    ///
    /// Only the initial inbox listing aborts the poll with an error (nothing
    /// was consumed yet). Per-file download/parse/settle failures are
    /// collected into the report and the poll continues: unparseable files
    /// are quarantined (or left in place) and reported in
    /// [`PollReport::skipped`], operational failures in
    /// [`PollReport::errors`], never fatal.
    pub async fn poll_once(&mut self) -> Result<PollReport, BatchError> {
        let mut report = PollReport::default();
        let entries = self.transport.list(&self.inbox).await?;

        // Forget state for paths that left the inbox (consumed elsewhere,
        // rotated away): this bounds memory and lets redrops process.
        let listed: HashSet<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        self.settled.retain(|k| listed.contains(k.path.as_str()));
        self.seen_sizes.retain(|p, _| listed.contains(p.as_str()));

        for entry in entries {
            if report.files.len() >= self.max_files_per_poll {
                break;
            }
            if self.settled.contains(&SettledKey::of(&entry)) {
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
            if self.require_stable
                && let Some(size) = entry.size
                && self.seen_sizes.get(&entry.path) != Some(&size)
            {
                self.seen_sizes.insert(entry.path.clone(), size);
                report.skipped.push(SkippedFile {
                    path: entry.path.clone(),
                    reason: "size not yet stable".to_string(),
                });
                continue;
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
                    if let Some(dir) = &self.error_dir {
                        let dest = join_dir(dir, &relative_to(&self.inbox, &entry.path));
                        if let Err(e) = self.transport.rename(&entry.path, &dest).await {
                            report.errors.push(PollFileError {
                                path: entry.path.clone(),
                                reason: format!("quarantine failed: {e}"),
                            });
                            continue;
                        }
                    }
                    self.settled.insert(SettledKey::of(&entry));
                    report.skipped.push(SkippedFile {
                        path: entry.path.clone(),
                        reason: format!("unparseable: {e}"),
                    });
                    continue;
                }
            };

            let count_error = if self.verify_counts {
                intake.verify_counts().err()
            } else {
                None
            };
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
                match self.transport.rename(&entry.path, &dest).await {
                    Ok(()) => true,
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
                self.seen_sizes.remove(&entry.path);
            }
        }

        Ok(report)
    }
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

        // Settled: the oversize file is not re-downloaded or re-reported.
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
        assert!(report.skipped.is_empty());

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
}
