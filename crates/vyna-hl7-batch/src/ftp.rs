//! FTP drop transport (`feature = "ftp"`).
//!
//! Plain FTP over Tokio with passive data connections and binary transfer
//! type. For encrypted transfer use the SFTP transport; explicit FTPS is not
//! yet supported.

use std::time::Duration;

use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::FileType;
use suppaftp::{FtpError, Status};
use tokio::io::AsyncReadExt;

use crate::{BatchError, FileEntry, FileTransport};

/// Connection parameters for [`FtpTransport`].
#[derive(Debug, Clone)]
pub struct FtpConfig {
    /// Server address as `host:port`.
    pub addr: String,
    /// Login username.
    pub username: String,
    /// Login password.
    pub password: String,
    /// TCP connect timeout.
    pub connect_timeout: Duration,
}

impl FtpConfig {
    /// Connect to `addr` (`host:port`) with the given credentials and a 10s
    /// connect timeout.
    pub fn new(addr: &str, username: &str, password: &str) -> Self {
        Self {
            addr: addr.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            connect_timeout: Duration::from_secs(10),
        }
    }

    /// Override the TCP connect timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }
}

/// FTP [`FileTransport`].
///
/// Holds one control connection; data connections are opened per transfer.
/// Uploads go to a temp name and are renamed into place so pollers never see
/// half-written files. Missing files/directories (FTP 550) behave like the
/// filesystem transport: `list` on a missing dir is empty, `delete` of a
/// missing file succeeds, `get`/`rename` of a missing file is invalid.
pub struct FtpTransport {
    config: FtpConfig,
    stream: Option<AsyncFtpStream>,
    temp_counter: u64,
}

impl FtpTransport {
    /// Log in and select binary transfer type.
    pub async fn connect(config: FtpConfig) -> Result<Self, BatchError> {
        let mut transport = Self {
            config,
            stream: None,
            temp_counter: 0,
        };
        transport.reconnect().await?;
        Ok(transport)
    }

    async fn reconnect(&mut self) -> Result<(), BatchError> {
        let addr: std::net::SocketAddr = tokio::net::lookup_host(&self.config.addr)
            .await
            .map_err(FtpError::ConnectionError)
            .map_err(BatchError::from)?
            .next()
            .ok_or_else(|| {
                BatchError::from(FtpError::ConnectionError(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("could not resolve '{}'", self.config.addr),
                )))
            })?;
        let mut stream = AsyncFtpStream::connect_timeout(addr, self.config.connect_timeout).await?;
        stream
            .login(&self.config.username, &self.config.password)
            .await?;
        stream.transfer_type(FileType::Binary).await?;
        self.stream = Some(stream);
        Ok(())
    }

    fn stream(&mut self) -> Result<&mut AsyncFtpStream, BatchError> {
        self.stream.as_mut().ok_or(BatchError::NotConnected)
    }

    /// Create parent directories one level at a time, ignoring "already
    /// exists" failures.
    async fn mkdir_p(&mut self, dir: &str) -> Result<(), BatchError> {
        let mut prefix = String::new();
        for part in crate::transport::clean_relative_parts(dir)? {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(&part);
            match self.stream()?.mkdir(&prefix).await {
                Ok(()) => {}
                Err(e) if is_file_unavailable(&e) => {}
                Err(e) => return Err(BatchError::from(e)),
            }
        }
        Ok(())
    }
}

/// FTP 550 (file unavailable): missing file or directory.
fn is_file_unavailable(e: &FtpError) -> bool {
    matches!(
        e,
        FtpError::UnexpectedResponse(resp) if resp.status == Status::FileUnavailable
    )
}

impl FileTransport for FtpTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        let dir = match dir.trim_matches('/').is_empty() {
            true => None,
            false => Some(dir.trim_matches('/')),
        };
        let names = match self.stream()?.nlst(dir).await {
            Ok(names) => names,
            Err(e) if is_file_unavailable(&e) => return Ok(Vec::new()),
            Err(e) => return Err(BatchError::from(e)),
        };
        let mut entries = Vec::new();
        for name in names {
            // NLST may return bare names or full paths; only the final
            // segment belongs to this listing.
            let base = name.rsplit('/').next().unwrap_or(&name);
            if base.is_empty() || base == "." || base == ".." {
                continue;
            }
            let path = match dir {
                Some(d) => format!("{d}/{base}"),
                None => base.to_string(),
            };
            let size = self.stream()?.size(&path).await.ok().map(|s| s as u64);
            let modified_ms = self
                .stream()?
                .mdtm(&path)
                .await
                .ok()
                .and_then(|dt| dt.and_utc().timestamp_millis().try_into().ok());
            entries.push(FileEntry {
                path,
                size,
                modified_ms,
            });
        }
        entries.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(entries)
    }

    async fn get(&mut self, path: &str) -> Result<Vec<u8>, BatchError> {
        let mut download = match self.stream()?.retr_as_stream(path).await {
            Ok(s) => s,
            Err(e) if is_file_unavailable(&e) => {
                return Err(BatchError::Invalid(format!("no such file: '{path}'")));
            }
            Err(e) => return Err(BatchError::from(e)),
        };
        let mut buf = Vec::new();
        download
            .read_to_end(&mut buf)
            .await
            .map_err(|e| BatchError::from(FtpError::ConnectionError(e)))?;
        // Some servers accept RETR and report the missing file (550) only in
        // the transfer completion reply.
        download.finish().await.map_err(|e| {
            if is_file_unavailable(&e) {
                BatchError::Invalid(format!("no such file: '{path}'"))
            } else {
                BatchError::from(e)
            }
        })?;
        Ok(buf)
    }

    async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        let parts = crate::transport::clean_relative_parts(path)?;
        if parts.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        if parts.len() > 1 {
            let parent = parts[..parts.len() - 1].join("/");
            self.mkdir_p(&parent).await?;
        }
        self.temp_counter += 1;
        let temp = format!("{path}.tmp-{}-{}", std::process::id(), self.temp_counter);
        let mut cursor = std::io::Cursor::new(data);
        if let Err(e) = self.stream()?.put_file(&temp, &mut cursor).await {
            let _ = self.stream()?.rm(&temp).await;
            return Err(BatchError::from(e));
        }
        self.stream()?.rename(temp.as_str(), path).await?;
        Ok(())
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        match self.stream()?.rm(path).await {
            Ok(()) => Ok(()),
            Err(e) if is_file_unavailable(&e) => Ok(()),
            Err(e) => Err(BatchError::from(e)),
        }
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
        let parts = crate::transport::clean_relative_parts(to)?;
        if parts.len() > 1 {
            let parent = parts[..parts.len() - 1].join("/");
            self.mkdir_p(&parent).await?;
        }
        match self.stream()?.rename(from, to).await {
            Ok(()) => Ok(()),
            Err(e) if is_file_unavailable(&e) => {
                Err(BatchError::Invalid(format!("no such file: '{from}'")))
            }
            Err(e) => Err(BatchError::from(e)),
        }
    }

    async fn close(&mut self) -> Result<(), BatchError> {
        if let Some(mut stream) = self.stream.take() {
            stream.quit().await?;
        }
        Ok(())
    }
}
