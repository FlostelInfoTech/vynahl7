//! File movement: the [`FileTransport`] trait and a drop-folder implementation.
//!
//! Paths are transport-relative, `/`-separated, and must not escape the root:
//! absolute paths and `..` segments are rejected.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::BatchError;

/// A file discovered by [`FileTransport::list`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// Transport-relative path with `/` separators.
    pub path: String,
    /// File size in bytes, when the transport reports it.
    pub size: Option<u64>,
    /// Last modification time as milliseconds since the Unix epoch, when the
    /// transport reports it.
    pub modified_ms: Option<u64>,
}

/// Move files between an integration engine and a drop location.
///
/// Implementations: [`FilesystemTransport`] (always available),
/// `ftp::FtpTransport` (`feature = "ftp"`), `sftp::SftpTransport`
/// (`feature = "sftp"`), and `testing::MemTransport` (`feature = "testing"`).
pub trait FileTransport {
    /// List files directly inside `dir` (non-recursive). Returns an empty vec
    /// for a missing directory.
    fn list(
        &mut self,
        dir: &str,
    ) -> impl Future<Output = Result<Vec<FileEntry>, BatchError>> + Send;

    /// Download a whole file.
    fn get(&mut self, path: &str) -> impl Future<Output = Result<Vec<u8>, BatchError>> + Send;

    /// Upload a whole file, replacing any existing content. Implementations
    /// should publish atomically (temp file + rename) where the protocol
    /// allows it.
    fn put(
        &mut self,
        path: &str,
        data: &[u8],
    ) -> impl Future<Output = Result<(), BatchError>> + Send;

    /// Delete a file. Deleting a missing file is a success (idempotent).
    fn delete(&mut self, path: &str) -> impl Future<Output = Result<(), BatchError>> + Send;

    /// Move a file, replacing the destination. Parent directories of the
    /// destination are created when the transport supports it.
    fn rename(
        &mut self,
        from: &str,
        to: &str,
    ) -> impl Future<Output = Result<(), BatchError>> + Send;

    /// Release the transport (log out, close connections). The default
    /// no-op suits transports without sessions.
    fn close(&mut self) -> impl Future<Output = Result<(), BatchError>> + Send {
        async { Ok(()) }
    }
}

/// Reject absolute paths and `..` escapes; normalize to `/`-separated parts.
pub(crate) fn clean_relative_parts(path: &str) -> Result<Vec<String>, BatchError> {
    let normalized = path.replace('\\', "/");
    if normalized.starts_with('/') {
        return Err(BatchError::Invalid(format!(
            "absolute path rejected: '{path}'"
        )));
    }
    let mut parts = Vec::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                return Err(BatchError::Invalid(format!("path escapes root: '{path}'")));
            }
            _ => parts.push(part.to_string()),
        }
    }
    Ok(parts)
}

/// Drop-folder transport rooted at a local directory.
///
/// Uploads are atomic (temp file + rename + directory fsync), so pollers never
/// see half-written files produced through this transport.
#[derive(Debug, Clone)]
pub struct FilesystemTransport {
    root: PathBuf,
}

/// Run blocking filesystem I/O on the blocking pool so async executors are
/// never stalled by disk waits.
async fn block_on<F, T>(op: F) -> Result<T, BatchError>
where
    F: FnOnce() -> Result<T, BatchError> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(op)
        .await
        .map_err(|e| BatchError::Io(std::io::Error::other(e.to_string())))?
}

/// Disambiguates temp upload names across concurrent puts.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

impl FilesystemTransport {
    /// Open (creating if needed) a drop folder.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, BatchError> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    /// Root directory of the drop folder.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, BatchError> {
        let mut resolved = self.root.clone();
        for part in clean_relative_parts(path)? {
            // Parts came from split('/'), so no separators or `..` remain.
            resolved.push(part);
        }
        Ok(resolved)
    }

    fn list_sync(&self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        let resolved = self.resolve(dir)?;
        let read_dir = match std::fs::read_dir(&resolved) {
            Ok(rd) => rd,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(BatchError::Io(e)),
        };
        let prefix = clean_relative_parts(dir)?.join("/");
        let mut entries = Vec::new();
        for entry in read_dir {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            // Crash leftovers and concurrent uploads are never consumable.
            if name.starts_with(".tmp-") {
                continue;
            }
            let meta = entry.metadata()?;
            let path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let modified_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);
            entries.push(FileEntry {
                path,
                size: Some(meta.len()),
                modified_ms,
            });
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    fn get_sync(&self, path: &str) -> Result<Vec<u8>, BatchError> {
        let resolved = self.resolve(path)?;
        // Listings skip symlinks; reads must refuse them too, or a planted
        // link inside the root would let a read escape it.
        let is_link = std::fs::symlink_metadata(&resolved)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false);
        if is_link {
            return Err(BatchError::Invalid(format!(
                "refusing symlinked path: '{path}'"
            )));
        }
        Ok(std::fs::read(resolved)?)
    }

    fn put_sync(&self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        // The cleaned parts decide: `dest` always contains the root, so a
        // component count on it can never detect an empty path.
        if clean_relative_parts(path)?.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        let dest = self.resolve(path)?;
        let parent = dest
            .parent()
            .ok_or_else(|| BatchError::Invalid(format!("path has no parent: '{path}'")))?;
        std::fs::create_dir_all(parent)?;

        // Atomic publish: write temp, fsync, rename, fsync directory. The
        // counter keeps temp names unique across concurrent puts sharing
        // one clock tick (pid + nanos alone can repeat).
        let temp_name = format!(
            ".tmp-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            TEMP_COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let temp = parent.join(temp_name);
        std::fs::write(&temp, data)?;
        std::fs::File::open(&temp)?.sync_all()?;
        std::fs::rename(&temp, &dest)?;
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    }

    fn delete_sync(&self, path: &str) -> Result<(), BatchError> {
        match std::fs::remove_file(self.resolve(path)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(BatchError::Io(e)),
        }
    }

    fn rename_sync(&self, from: &str, to: &str) -> Result<(), BatchError> {
        // Like MemTransport: an empty cleaned destination is Invalid, not
        // a rename onto the root directory.
        if clean_relative_parts(to)?.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        let from = self.resolve(from)?;
        let to = self.resolve(to)?;
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        match std::fs::rename(&from, &to) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(BatchError::Invalid(
                format!("rename source does not exist: '{}'", from.display()),
            )),
            Err(e) => Err(BatchError::Io(e)),
        }
    }
}

impl FileTransport for FilesystemTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        let this = self.clone();
        let dir = dir.to_string();
        block_on(move || this.list_sync(&dir)).await
    }

    async fn get(&mut self, path: &str) -> Result<Vec<u8>, BatchError> {
        let this = self.clone();
        let path = path.to_string();
        block_on(move || this.get_sync(&path)).await
    }

    async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        let this = self.clone();
        let path = path.to_string();
        let data = data.to_vec();
        block_on(move || this.put_sync(&path, &data)).await
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        let this = self.clone();
        let path = path.to_string();
        block_on(move || this.delete_sync(&path)).await
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
        let this = self.clone();
        let from = from.to_string();
        let to = to.to_string();
        block_on(move || this.rename_sync(&from, &to)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn scratch_dir() -> PathBuf {
        let seq = DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("vyna-batch-test-{}-{seq}", std::process::id()))
    }

    #[tokio::test]
    async fn filesystem_round_trip() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();

        assert_eq!(t.list("").await.unwrap(), Vec::new());

        t.put("inbox/a.hl7", b"MSH").await.unwrap();
        t.put("inbox/b.hl7", b"MSH2").await.unwrap();
        let entries = t.list("inbox").await.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "inbox/a.hl7");
        assert_eq!(entries[0].size, Some(3));
        assert!(entries[0].modified_ms.is_some());

        assert_eq!(t.get("inbox/a.hl7").await.unwrap(), b"MSH");

        t.rename("inbox/a.hl7", "archive/a.hl7").await.unwrap();
        assert_eq!(t.list("inbox").await.unwrap().len(), 1);
        assert_eq!(t.get("archive/a.hl7").await.unwrap(), b"MSH");

        t.delete("inbox/b.hl7").await.unwrap();
        t.delete("inbox/b.hl7").await.unwrap(); // idempotent
        assert!(t.list("inbox").await.unwrap().is_empty());

        // No temp files leak into listings.
        assert!(
            t.list("archive")
                .await
                .unwrap()
                .iter()
                .all(|e| !e.path.contains(".tmp-"))
        );

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn paths_cannot_escape_root() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();

        assert!(matches!(
            t.put("../evil.hl7", b"x").await,
            Err(BatchError::Invalid(_))
        ));
        assert!(matches!(
            t.get("a/../../evil").await,
            Err(BatchError::Invalid(_))
        ));
        assert!(t.list("missing").await.unwrap().is_empty());

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn absolute_paths_are_rejected() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/a.hl7", b"MSH").await.unwrap();

        // The module docs promise absolute paths are rejected; a leading
        // `/` must not silently resolve under the root.
        for path in ["/a.hl7", "/inbox/a.hl7", "//inbox//a.hl7"] {
            assert!(
                matches!(t.get(path).await, Err(BatchError::Invalid(_))),
                "get({path:?}) must be rejected"
            );
            assert!(
                matches!(t.put(path, b"x").await, Err(BatchError::Invalid(_))),
                "put({path:?}) must be rejected"
            );
        }
        assert!(matches!(
            clean_relative_parts("/a"),
            Err(BatchError::Invalid(_))
        ));

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn reads_refuse_planted_symlinks() {
        let dir = scratch_dir();
        let outside = scratch_dir();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.txt"), b"outside-root").unwrap();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/real.hl7", b"MSH").await.unwrap();
        // Plant a symlink inside the root pointing outside it.
        std::os::unix::fs::symlink(
            outside.join("secret.txt"),
            dir.join("inbox").join("evil.hl7"),
        )
        .unwrap();

        // Listings already skip symlinks; reads must refuse them too
        // instead of following them out of the root.
        let listed: Vec<_> = t
            .list("inbox")
            .await
            .unwrap()
            .iter()
            .map(|e| e.path.clone())
            .collect();
        assert_eq!(listed, ["inbox/real.hl7"]);
        assert!(
            matches!(t.get("inbox/evil.hl7").await, Err(BatchError::Invalid(_))),
            "get() must refuse symlinked paths"
        );

        tokio::fs::remove_dir_all(&dir).await.unwrap();
        tokio::fs::remove_dir_all(&outside).await.unwrap();
    }

    #[tokio::test]
    async fn list_skips_temp_leftovers() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/a.hl7", b"MSH").await.unwrap();
        // Simulate a crash between temp write and rename.
        std::fs::write(dir.join("inbox").join(".tmp-123-456"), b"partial").unwrap();

        let entries = t.list("inbox").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "inbox/a.hl7");

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn rename_rejects_empty_destination() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(&dir).unwrap();
        t.put("inbox/a.hl7", b"MSH").await.unwrap();

        // Like MemTransport, an empty cleaned destination is Invalid,
        // not an attempt to rename onto the root directory.
        for dest in ["", "."] {
            match t.rename("inbox/a.hl7", dest).await {
                Err(BatchError::Invalid(msg)) => assert_eq!(msg, "empty path"),
                other => panic!("rename to {dest:?} gave {other:?}"),
            }
        }
        assert!(matches!(
            t.rename("inbox/a.hl7", "/").await,
            Err(BatchError::Invalid(_))
        ));
        // The source is untouched by the rejected renames.
        assert_eq!(t.get("inbox/a.hl7").await.unwrap(), b"MSH");

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn put_rejects_empty_paths_without_leaking_temp_files() {
        let dir = scratch_dir();
        let mut t = FilesystemTransport::open(dir.join("root")).unwrap();

        for path in ["", "/", "."] {
            assert!(
                matches!(t.put(path, b"MSH").await, Err(BatchError::Invalid(_))),
                "put({path:?}) must be rejected"
            );
        }
        // Nothing staged anywhere: the root holds no files and its parent
        // holds no leaked temp files either.
        assert!(t.list("").await.unwrap().is_empty());
        let mut leftover: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        leftover.sort();
        assert_eq!(leftover, vec![std::ffi::OsString::from("root")]);

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn concurrent_puts_do_not_share_temp_files() {
        let dir = scratch_dir();
        let transport = FilesystemTransport::open(&dir).unwrap();

        let mut handles = Vec::new();
        for i in 0..64 {
            let mut t = transport.clone();
            handles.push(tokio::spawn(async move {
                let path = format!("inbox/f{i:03}.hl7");
                let data = format!("payload-{i}").into_bytes();
                t.put(&path, &data).await.unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        let mut t = transport.clone();
        for i in 0..64 {
            let path = format!("inbox/f{i:03}.hl7");
            assert_eq!(
                t.get(&path).await.unwrap(),
                format!("payload-{i}").into_bytes()
            );
        }

        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }
}
