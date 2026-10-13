//! Test doubles for transport consumers.
//!
//! [`MemTransport`] is an in-memory [`FileTransport`]
//! for unit-testing pollers and processors without touching the network or
//! the filesystem. Available with `feature = "testing"`.

use std::collections::BTreeMap;

use crate::{BatchError, FileEntry, FileTransport};

/// In-memory file transport for tests.
///
/// Paths behave like [`FilesystemTransport`](crate::FilesystemTransport)
/// paths: `/`-separated, relative, `..` rejected.
#[derive(Debug, Default)]
pub struct MemTransport {
    files: BTreeMap<String, (Vec<u8>, u64)>,
    closed: bool,
    /// Logical clock bumped on every write; reported as `modified_ms` so
    /// pollers can observe mtime stability and redrops.
    tick: u64,
}

impl MemTransport {
    /// Empty transport.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed one file.
    ///
    /// Panics when `path` is absolute, escapes the root with `..`, or is
    /// empty, mirroring the [`BatchError::Invalid`] rejections of the ops.
    pub fn with_file(mut self, path: &str, data: &[u8]) -> Self {
        let normalized = normalize(path).expect("invalid path");
        assert!(!normalized.is_empty(), "invalid path: empty path");
        self.tick += 1;
        let tick = self.tick;
        self.files.insert(normalized, (data.to_vec(), tick));
        self
    }

    fn check_open(&self) -> Result<(), BatchError> {
        if self.closed {
            return Err(BatchError::NotConnected);
        }
        Ok(())
    }
}

fn normalize(path: &str) -> Result<String, BatchError> {
    Ok(crate::transport::clean_relative_parts(path)?.join("/"))
}

impl FileTransport for MemTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        self.check_open()?;
        let prefix = normalize(dir)?;
        let mut entries = Vec::new();
        for (path, (data, mtime)) in &self.files {
            let parent = match path.rfind('/') {
                Some(i) => &path[..i],
                None => "",
            };
            if parent == prefix {
                entries.push(FileEntry {
                    path: path.clone(),
                    size: Some(data.len() as u64),
                    modified_ms: Some(*mtime),
                });
            }
        }
        Ok(entries)
    }

    async fn get(&mut self, path: &str) -> Result<Vec<u8>, BatchError> {
        self.check_open()?;
        self.files
            .get(&normalize(path)?)
            .map(|(data, _)| data.clone())
            .ok_or_else(|| BatchError::Invalid(format!("no such file: '{path}'")))
    }

    async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        self.check_open()?;
        let normalized = normalize(path)?;
        if normalized.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        self.tick += 1;
        let tick = self.tick;
        self.files.insert(normalized, (data.to_vec(), tick));
        Ok(())
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        self.check_open()?;
        self.files.remove(&normalize(path)?);
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
        self.check_open()?;
        let from = normalize(from)?;
        let to = normalize(to)?;
        if to.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        match self.files.remove(&from) {
            Some(data) => {
                self.files.insert(to, data);
                Ok(())
            }
            None => Err(BatchError::Invalid(format!("no such file: '{from}'"))),
        }
    }

    async fn close(&mut self) -> Result<(), BatchError> {
        self.closed = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Poller;

    const MSH: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5";

    #[tokio::test]
    async fn poller_consumes_from_memory_transport() {
        let transport = MemTransport::new().with_file("inbox/a.hl7", MSH.as_bytes());
        let mut poller = Poller::new(transport)
            .with_inbox("inbox")
            .with_require_stable(false);
        let report = poller.poll_once().await.unwrap();
        assert_eq!(report.files.len(), 1);
        assert_eq!(report.files[0].ok_count(), 1);
        // Deleted after consumption by default.
        let report = poller.poll_once().await.unwrap();
        assert!(report.files.is_empty());
    }

    #[tokio::test]
    async fn closed_transport_refuses_work() {
        let mut transport = MemTransport::new().with_file("a.hl7", MSH.as_bytes());
        transport.close().await.unwrap();
        assert!(matches!(
            transport.list("").await,
            Err(BatchError::NotConnected)
        ));
    }

    #[tokio::test]
    async fn mem_rejects_escape_paths_in_every_op() {
        // Mirrors FilesystemTransport: `..` and absolute paths are
        // BatchError::Invalid in every op, never silently normalized.
        let mut transport = MemTransport::new().with_file("inbox/a.hl7", MSH.as_bytes());
        for dir in ["..", "../evil", "/abs"] {
            assert!(
                matches!(transport.list(dir).await, Err(BatchError::Invalid(_))),
                "list({dir:?}) must be rejected"
            );
        }
        for path in ["..", "../evil", "/abs"] {
            assert!(
                matches!(transport.get(path).await, Err(BatchError::Invalid(_))),
                "get({path:?}) must be rejected"
            );
            assert!(
                matches!(transport.put(path, b"x").await, Err(BatchError::Invalid(_))),
                "put({path:?}) must be rejected"
            );
            assert!(
                matches!(transport.delete(path).await, Err(BatchError::Invalid(_))),
                "delete({path:?}) must be rejected"
            );
        }
        assert!(matches!(
            transport.rename("inbox/a.hl7", "..").await,
            Err(BatchError::Invalid(_))
        ));
        assert!(matches!(
            transport.rename("../evil", "b.hl7").await,
            Err(BatchError::Invalid(_))
        ));
        assert!(matches!(
            transport.rename("inbox/a.hl7", "").await,
            Err(BatchError::Invalid(_))
        ));
        // The rejected ops changed nothing.
        assert_eq!(transport.get("inbox/a.hl7").await.unwrap(), MSH.as_bytes());
        assert_eq!(transport.list("").await.unwrap().len(), 0);
    }

    #[test]
    #[should_panic(expected = "invalid path")]
    fn mem_with_file_rejects_escape_paths() {
        let _ = MemTransport::new().with_file("../evil.hl7", MSH.as_bytes());
    }

    #[tokio::test]
    async fn listings_report_mtime_that_advances_on_rewrite() {
        // Pollers key redrops on (path, size, mtime): a same-size rewrite
        // must surface a new mtime so it reprocesses.
        let mut transport = MemTransport::new().with_file("a.hl7", MSH.as_bytes());
        let first = transport.list("").await.unwrap();
        assert_eq!(first.len(), 1);
        let before = first[0].modified_ms.expect("mtime must be reported");

        transport.put("a.hl7", MSH.as_bytes()).await.unwrap();
        let second = transport.list("").await.unwrap();
        let after = second[0].modified_ms.expect("mtime must be reported");
        assert_eq!(second[0].size, first[0].size);
        assert!(after > before, "rewrite must advance mtime");
    }
}
