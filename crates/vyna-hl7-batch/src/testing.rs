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
    files: BTreeMap<String, Vec<u8>>,
    closed: bool,
}

impl MemTransport {
    /// Empty transport.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed one file.
    pub fn with_file(mut self, path: &str, data: &[u8]) -> Self {
        self.files.insert(normalize(path), data.to_vec());
        self
    }

    fn check_open(&self) -> Result<(), BatchError> {
        if self.closed {
            return Err(BatchError::NotConnected);
        }
        Ok(())
    }
}

fn normalize(path: &str) -> String {
    crate::transport::clean_relative_parts(path)
        .unwrap_or_default()
        .join("/")
}

impl FileTransport for MemTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        self.check_open()?;
        let prefix = normalize(dir);
        let mut entries = Vec::new();
        for (path, data) in &self.files {
            let parent = match path.rfind('/') {
                Some(i) => &path[..i],
                None => "",
            };
            if parent == prefix {
                entries.push(FileEntry {
                    path: path.clone(),
                    size: Some(data.len() as u64),
                    modified_ms: None,
                });
            }
        }
        Ok(entries)
    }

    async fn get(&mut self, path: &str) -> Result<Vec<u8>, BatchError> {
        self.check_open()?;
        self.files
            .get(&normalize(path))
            .cloned()
            .ok_or_else(|| BatchError::Invalid(format!("no such file: '{path}'")))
    }

    async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        self.check_open()?;
        let normalized = normalize(path);
        if normalized.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        self.files.insert(normalized, data.to_vec());
        Ok(())
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        self.check_open()?;
        self.files.remove(&normalize(path));
        Ok(())
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
        self.check_open()?;
        let from = normalize(from);
        let to = normalize(to);
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
}
