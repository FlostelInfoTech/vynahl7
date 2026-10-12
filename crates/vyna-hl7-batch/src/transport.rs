//! File movement: the [`FileTransport`] trait and a drop-folder implementation.
//!
//! Paths are transport-relative, `/`-separated, and must not escape the root:
//! absolute paths and `..` segments are rejected.

use std::path::{Path, PathBuf};

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
#[derive(Debug)]
pub struct FilesystemTransport {
    root: PathBuf,
}

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
}

impl FileTransport for FilesystemTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
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

    async fn get(&mut self, path: &str) -> Result<Vec<u8>, BatchError> {
        Ok(std::fs::read(self.resolve(path)?)?)
    }

    async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        let dest = self.resolve(path)?;
        if dest.components().count() == 0 {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        let parent = dest
            .parent()
            .ok_or_else(|| BatchError::Invalid(format!("path has no parent: '{path}'")))?;
        std::fs::create_dir_all(parent)?;

        // Atomic publish: write temp, fsync, rename, fsync directory.
        let temp_name = format!(
            ".tmp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let temp = parent.join(temp_name);
        std::fs::write(&temp, data)?;
        std::fs::File::open(&temp)?.sync_all()?;
        std::fs::rename(&temp, &dest)?;
        std::fs::File::open(parent)?.sync_all()?;
        Ok(())
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        match std::fs::remove_file(self.resolve(path)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(BatchError::Io(e)),
        }
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
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

        std::fs::remove_dir_all(&dir).unwrap();
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

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
