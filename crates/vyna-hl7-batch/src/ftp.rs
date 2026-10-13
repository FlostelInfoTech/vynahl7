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
#[derive(Clone)]
pub struct FtpConfig {
    /// Server address as `host:port`.
    pub addr: String,
    /// Login username.
    pub username: String,
    /// Login password.
    pub password: String,
    /// TCP connect timeout.
    pub connect_timeout: Duration,
    /// Per-operation timeout for protocol commands and transfers
    /// (`None` waits forever). Defaults to
    /// [`DEFAULT_COMMAND_TIMEOUT`](crate::DEFAULT_COMMAND_TIMEOUT).
    pub command_timeout: Option<Duration>,
    /// Refuse downloads larger than this. Defaults to
    /// [`DEFAULT_MAX_FILE_BYTES`](crate::DEFAULT_MAX_FILE_BYTES).
    pub max_file_bytes: u64,
}

// The password must never leak into logs via `{:?}`.
impl std::fmt::Debug for FtpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FtpConfig")
            .field("addr", &self.addr)
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .field("connect_timeout", &self.connect_timeout)
            .field("command_timeout", &self.command_timeout)
            .field("max_file_bytes", &self.max_file_bytes)
            .finish()
    }
}

impl FtpConfig {
    /// Connect to `addr` (`host:port`) with the given credentials, a 10s
    /// connect timeout, and default command timeout and size caps.
    pub fn new(addr: &str, username: &str, password: &str) -> Self {
        Self {
            addr: addr.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            connect_timeout: Duration::from_secs(10),
            command_timeout: Some(crate::DEFAULT_COMMAND_TIMEOUT),
            max_file_bytes: crate::DEFAULT_MAX_FILE_BYTES,
        }
    }

    /// Override the TCP connect timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Override the per-operation timeout (`None` waits forever).
    pub fn with_command_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.command_timeout = timeout;
        self
    }

    /// Refuse downloads larger than `limit` bytes.
    pub fn with_max_file_bytes(mut self, limit: u64) -> Self {
        self.max_file_bytes = limit;
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
        let connect_timeout = self.config.connect_timeout;
        let mut addrs =
            tokio::time::timeout(connect_timeout, tokio::net::lookup_host(&self.config.addr))
                .await
                .map_err(|_| BatchError::Timeout(connect_timeout))?
                .map_err(FtpError::ConnectionError)
                .map_err(BatchError::from)?;
        let addr: std::net::SocketAddr = addrs.next().ok_or_else(|| {
            BatchError::from(FtpError::ConnectionError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("could not resolve '{}'", self.config.addr),
            )))
        })?;
        let mut stream = AsyncFtpStream::connect_timeout(addr, self.config.connect_timeout).await?;
        let timeout = self.config.command_timeout;
        crate::timed(
            timeout,
            stream.login(&self.config.username, &self.config.password),
        )
        .await?;
        crate::timed(timeout, stream.transfer_type(FileType::Binary)).await?;
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
            let timeout = self.config.command_timeout;
            match crate::timed(timeout, self.stream()?.mkdir(&prefix)).await {
                Ok(()) => {}
                Err(BatchError::Ftp(e)) if is_file_unavailable(&e) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Single-round-trip listing via MLSD. Falls back to NLST when the
    /// server does not support it.
    async fn list_via_mlsd(&mut self, dir: Option<&str>) -> Result<Vec<FileEntry>, BatchError> {
        let timeout = self.config.command_timeout;
        let lines = crate::timed(timeout, self.stream()?.mlsd(dir)).await?;
        Ok(parse_mlsd_entries(dir, &lines))
    }

    /// Legacy listing: NLST plus SIZE/MDTM per file (2N round trips).
    /// Kept for servers without MLSD.
    async fn list_via_nlst(&mut self, dir: Option<&str>) -> Result<Vec<FileEntry>, BatchError> {
        let timeout = self.config.command_timeout;
        let names = match crate::timed(timeout, self.stream()?.nlst(dir)).await {
            Ok(names) => names,
            Err(BatchError::Ftp(e)) if is_file_unavailable(&e) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut entries = Vec::new();
        for name in names {
            // NLST may return bare names or full paths; only the final
            // segment belongs to this listing.
            let base = name.rsplit('/').next().unwrap_or(&name);
            if base.is_empty() || base == "." || base == ".." {
                continue;
            }
            // Skip before the SIZE/MDTM round trips below.
            if is_staged_upload(base) {
                continue;
            }
            let path = match dir {
                Some(d) => format!("{d}/{base}"),
                None => base.to_string(),
            };
            let size = crate::timed(timeout, self.stream()?.size(&path))
                .await
                .ok()
                .map(|s| s as u64);
            let modified_ms = crate::timed(timeout, self.stream()?.mdtm(&path))
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
}

/// FTP 550 (file unavailable): missing file or directory.
fn is_file_unavailable(e: &FtpError) -> bool {
    matches!(
        e,
        FtpError::UnexpectedResponse(resp) if resp.status == Status::FileUnavailable
    )
}

/// Staging name for an upload: a sibling of `path` carrying the `.tmp-`
/// prefix, so pollers skip it via [`is_staged_upload`] (the same skip
/// convention the filesystem transport uses).
fn staging_temp_name(path: &str, counter: u64) -> String {
    let (parent, base) = match path.rsplit_once('/') {
        Some((parent, base)) => (parent, base),
        None => ("", path),
    };
    let pid = std::process::id();
    if parent.is_empty() {
        format!(".tmp-{pid}-{counter}-{base}")
    } else {
        format!("{parent}/.tmp-{pid}-{counter}-{base}")
    }
}

/// True for staged-upload names, which listings must hide from pollers.
///
/// Current staging names carry the `.tmp-` prefix (see
/// [`staging_temp_name`]); `{name}.tmp-{pid}-{counter}` covers crash
/// leftovers staged by older versions.
fn is_staged_upload(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    if base.starts_with(".tmp-") {
        return true;
    }
    match base.rsplit_once(".tmp-") {
        Some((_, suffix)) => {
            let mut chunks = suffix.split('-');
            matches!(
                (chunks.next(), chunks.next(), chunks.next()),
                (Some(a), Some(b), None)
                    if !a.is_empty()
                        && !b.is_empty()
                        && a.bytes().all(|c| c.is_ascii_digit())
                        && b.bytes().all(|c| c.is_ascii_digit())
            )
        }
        None => false,
    }
}

/// Parse MLSD lines into entries, skipping directories and unparseable
/// lines. A missing `modify` fact parses as the Unix epoch, which is
/// reported as unknown rather than as a bogus 1970 timestamp.
fn parse_mlsd_entries(dir: Option<&str>, lines: &[String]) -> Vec<FileEntry> {
    use suppaftp::list::ListParser;

    let mut entries = Vec::new();
    for line in lines {
        let Ok(file) = ListParser::parse_mlsd(line) else {
            continue;
        };
        if !file.is_file() {
            continue;
        }
        if is_staged_upload(file.name()) {
            continue;
        }
        let path = match dir {
            Some(d) => format!("{d}/{}", file.name()),
            None => file.name().to_string(),
        };
        let modified_ms = file
            .modified()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as u64)
            .filter(|ms| *ms > 0);
        entries.push(FileEntry {
            path,
            size: Some(file.size() as u64),
            modified_ms,
        });
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

impl FileTransport for FtpTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        let dir = match dir.trim_matches('/').is_empty() {
            true => None,
            false => Some(dir.trim_matches('/')),
        };
        match self.list_via_mlsd(dir).await {
            Ok(entries) => Ok(entries),
            // Missing dir behaves like the filesystem transport; servers
            // without MLSD fall back to NLST.
            Err(BatchError::Ftp(e)) if is_file_unavailable(&e) => Ok(Vec::new()),
            Err(BatchError::Ftp(_)) => self.list_via_nlst(dir).await,
            Err(e) => Err(e),
        }
    }

    async fn get(&mut self, path: &str) -> Result<Vec<u8>, BatchError> {
        let timeout = self.config.command_timeout;
        let limit = self.config.max_file_bytes;
        // Refuse huge files before RETR when the server reports a size.
        if let Ok(size) = crate::timed(timeout, self.stream()?.size(path)).await
            && size as u64 > limit
        {
            return Err(BatchError::TooLarge {
                path: path.to_string(),
                limit,
            });
        }
        let download = match crate::timed(timeout, self.stream()?.retr_as_stream(path)).await {
            Ok(s) => s,
            Err(BatchError::Ftp(e)) if is_file_unavailable(&e) => {
                return Err(BatchError::Invalid(format!("no such file: '{path}'")));
            }
            Err(e) => return Err(e),
        };
        // Cap the read as well: SIZE may be unsupported or lie.
        let mut capped = download.take(limit.saturating_add(1));
        let mut buf = Vec::new();
        crate::timed(timeout, async {
            capped
                .read_to_end(&mut buf)
                .await
                .map_err(FtpError::ConnectionError)
        })
        .await?;
        let download = capped.into_inner();
        if buf.len() as u64 > limit {
            // Drain the completion reply (if any) before reporting.
            let _ = crate::timed(timeout, download.finish()).await;
            return Err(BatchError::TooLarge {
                path: path.to_string(),
                limit,
            });
        }
        // Some servers accept RETR and report the missing file (550) only in
        // the transfer completion reply.
        crate::timed(timeout, download.finish())
            .await
            .map_err(|e| {
                if let BatchError::Ftp(inner) = &e
                    && is_file_unavailable(inner)
                {
                    BatchError::Invalid(format!("no such file: '{path}'"))
                } else {
                    e
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
        let temp = staging_temp_name(path, self.temp_counter);
        let mut cursor = std::io::Cursor::new(data);
        let timeout = self.config.command_timeout;
        if let Err(e) = crate::timed(timeout, self.stream()?.put_file(&temp, &mut cursor)).await {
            let _ = crate::timed(timeout, self.stream()?.rm(&temp)).await;
            return Err(e);
        }
        if let Err(e) = crate::timed(timeout, self.stream()?.rename(temp.as_str(), path)).await {
            let _ = crate::timed(timeout, self.stream()?.rm(&temp)).await;
            return Err(e);
        }
        Ok(())
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        let timeout = self.config.command_timeout;
        match crate::timed(timeout, self.stream()?.rm(path)).await {
            Ok(()) => Ok(()),
            Err(BatchError::Ftp(e)) if is_file_unavailable(&e) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
        let parts = crate::transport::clean_relative_parts(to)?;
        if parts.len() > 1 {
            let parent = parts[..parts.len() - 1].join("/");
            self.mkdir_p(&parent).await?;
        }
        let timeout = self.config.command_timeout;
        match crate::timed(timeout, self.stream()?.rename(from, to)).await {
            Ok(()) => Ok(()),
            Err(BatchError::Ftp(e)) if is_file_unavailable(&e) => {
                Err(BatchError::Invalid(format!("no such file: '{from}'")))
            }
            Err(e) => Err(e),
        }
    }

    async fn close(&mut self) -> Result<(), BatchError> {
        if let Some(mut stream) = self.stream.take() {
            crate::timed(self.config.command_timeout, stream.quit()).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_password() {
        let config = FtpConfig::new("host:21", "user", "s3cr3t-pw");
        let debug = format!("{config:?}");
        assert!(!debug.contains("s3cr3t-pw"), "leaked: {debug}");
        assert!(debug.contains("host:21"));
        assert!(debug.contains("user"));
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn defaults_carry_size_cap_and_command_timeout() {
        let config = FtpConfig::new("host:21", "u", "p");
        assert_eq!(config.max_file_bytes, crate::DEFAULT_MAX_FILE_BYTES);
        assert_eq!(config.command_timeout, Some(crate::DEFAULT_COMMAND_TIMEOUT));
        let config = config.with_max_file_bytes(1024).with_command_timeout(None);
        assert_eq!(config.max_file_bytes, 1024);
        assert_eq!(config.command_timeout, None);
    }

    #[test]
    fn mlsd_lines_parse_to_entries() {
        let lines = vec![
            "type=file;size=57;modify=20240101120000; a.hl7".to_string(),
            "type=dir;modify=20240101120000; subdir".to_string(),
            "type=bogus;size=7; nope.hl7".to_string(),
        ];
        let entries = parse_mlsd_entries(Some("inbox"), &lines);
        // Only the file survives; dirs and unparseable lines are skipped.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "inbox/a.hl7");
        assert_eq!(entries[0].size, Some(57));
        assert!(entries[0].modified_ms.is_some());
    }

    #[test]
    fn mlsd_without_modify_reports_unknown_time() {
        let lines = vec!["type=file;size=3; b.hl7".to_string()];
        let entries = parse_mlsd_entries(None, &lines);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "b.hl7");
        assert_eq!(entries[0].modified_ms, None);
    }

    #[test]
    fn mlsd_listing_hides_staged_uploads() {
        let lines = vec![
            "type=file;size=57;modify=20240101120000; a.hl7".to_string(),
            // In-progress staged upload (current naming).
            "type=file;size=12;modify=20240101120000; .tmp-123-1-b.hl7".to_string(),
            // Crash leftover staged by an older version (legacy naming).
            "type=file;size=12;modify=20240101120000; b.hl7.tmp-123-1".to_string(),
        ];
        let entries = parse_mlsd_entries(Some("inbox"), &lines);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "inbox/a.hl7");
    }

    #[test]
    fn staged_upload_names_are_detected() {
        // Current staging names: `.tmp-{pid}-{counter}-{file}`.
        assert!(is_staged_upload("inbox/.tmp-123-1-a.hl7"));
        assert!(is_staged_upload(".tmp-123-1-a.hl7"));
        // Filesystem-style leftovers share the prefix.
        assert!(is_staged_upload("inbox/.tmp-123-456"));
        // Legacy staging names: `{file}.tmp-{pid}-{counter}`.
        assert!(is_staged_upload("inbox/b.hl7.tmp-123-1"));
        // Ordinary files are never hidden.
        assert!(!is_staged_upload("inbox/a.hl7"));
        assert!(!is_staged_upload("report.tmp-backup.hl7"));
        assert!(!is_staged_upload("inbox/x.tmp-1-2-3"));
        // The `.tmp-` prefix hides unconditionally, matching the filesystem
        // transport's convention exactly.
        assert!(is_staged_upload("inbox/.tmp-notes.hl7"));
    }

    #[test]
    fn staging_temp_name_is_a_hidden_sibling() {
        let pid = std::process::id();
        assert_eq!(
            staging_temp_name("inbox/a.hl7", 7),
            format!("inbox/.tmp-{pid}-7-a.hl7")
        );
        assert_eq!(staging_temp_name("a.hl7", 7), format!(".tmp-{pid}-7-a.hl7"));
        // Everything `put` stages must be invisible to `list`.
        assert!(is_staged_upload(&staging_temp_name("inbox/a.hl7", 7)));
        assert!(is_staged_upload(&staging_temp_name("a.hl7", 7)));
    }

    #[tokio::test]
    async fn list_hides_staged_uploads_end_to_end() {
        use libunftp::ServerBuilder;
        use libunftp::auth::AnonymousAuthenticator;
        use unftp_sbe_fs::Filesystem;

        let dir = std::env::temp_dir().join(format!(
            "vyna-ftp-staged-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        tokio::fs::create_dir_all(dir.join("inbox")).await.unwrap();
        // A finished file plus staged-upload leftovers (current and legacy
        // naming) planted as a crashed uploader would leave them.
        std::fs::write(dir.join("inbox/a.hl7"), "MSH").unwrap();
        std::fs::write(dir.join("inbox/.tmp-123-1-b.hl7"), "partial").unwrap();
        std::fs::write(dir.join("inbox/b.hl7.tmp-123-1"), "partial").unwrap();

        let root = dir.clone();
        let server = ServerBuilder::new(Box::new(move || Filesystem::new(root.clone()).unwrap()))
            .authenticator(std::sync::Arc::new(AnonymousAuthenticator))
            .greeting("vyna staged test")
            .passive_ports(33000..=33009)
            .build()
            .unwrap();
        let probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = probe.local_addr().unwrap();
        drop(probe);
        tokio::spawn(async move {
            server.listen(addr.to_string()).await.unwrap();
        });
        for _ in 0..100 {
            if tokio::net::TcpStream::connect(addr).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let config = FtpConfig::new(&addr.to_string(), "anonymous", "test");
        let mut ftp = FtpTransport::connect(config).await.unwrap();
        let listed: Vec<_> = ftp
            .list("inbox")
            .await
            .unwrap()
            .iter()
            .map(|e| e.path.clone())
            .collect();
        assert_eq!(listed, ["inbox/a.hl7"]);

        // A real upload still lands visibly and leaves no temp behind.
        ftp.put("inbox/c.hl7", b"MSH").await.unwrap();
        assert_eq!(ftp.list("inbox").await.unwrap().len(), 2);

        ftp.close().await.unwrap();
        tokio::fs::remove_dir_all(&dir).await.unwrap();
    }

    #[tokio::test]
    async fn hanging_server_hits_command_timeout() {
        use tokio::io::AsyncWriteExt;

        // Fake server: greeting, then silence. Login must time out
        // instead of hanging on the missing USER reply.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket.write_all(b"220 fake\r\n").await.unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let config = FtpConfig::new(&addr.to_string(), "u", "p")
            .with_connect_timeout(Duration::from_secs(5))
            .with_command_timeout(Some(Duration::from_millis(200)));
        let start = std::time::Instant::now();
        let result = FtpTransport::connect(config).await;
        assert!(start.elapsed() < Duration::from_secs(10));
        match result {
            Err(BatchError::Timeout(_)) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
            Ok(_) => panic!("expected a timeout, got a connection"),
        }
    }
}
