//! SFTP drop transport (`feature = "sftp"`).
//!
//! Encrypted file transfer over SSH with password or public-key
//! authentication and server host-key pinning.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use russh::Disconnect;
use russh::keys::{PrivateKeyWithHashAlg, decode_secret_key, parse_public_key_base64};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::{BatchError, FileEntry, FileTransport};

/// Client authentication for [`SftpTransport`].
#[derive(Clone)]
pub enum SftpAuth {
    /// Password authentication.
    Password(String),
    /// Public-key authentication from a PEM/OpenSSH private key file.
    KeyFile {
        /// Path to the private key.
        path: PathBuf,
        /// Key passphrase, if encrypted.
        passphrase: Option<String>,
    },
    /// Public-key authentication from in-memory key text.
    KeyText {
        /// PEM/OpenSSH private key text.
        pem: String,
        /// Key passphrase, if encrypted.
        passphrase: Option<String>,
    },
}

// Secrets must never leak into logs via `{:?}`. Passphrase presence
// (Some vs None) is shown; values are not.
impl std::fmt::Debug for SftpAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SftpAuth::Password(_) => write!(f, "Password(<redacted>)"),
            SftpAuth::KeyFile { path, passphrase } => f
                .debug_struct("KeyFile")
                .field("path", path)
                .field("passphrase", &passphrase.as_ref().map(|_| "<redacted>"))
                .finish(),
            SftpAuth::KeyText { passphrase, .. } => f
                .debug_struct("KeyText")
                .field("pem", &"<redacted>")
                .field("passphrase", &passphrase.as_ref().map(|_| "<redacted>"))
                .finish(),
        }
    }
}

/// How the server's host key is trusted.
#[derive(Debug, Clone)]
pub enum HostKeyPolicy {
    /// Trust any host key. Vulnerable to man-in-the-middle attacks; only for
    /// testing against known-local servers.
    AcceptAny,
    /// Trust only these OpenSSH public key lines (`"ssh-ed25519 AAAA..."`).
    Pinned(Vec<String>),
}

/// Connection parameters for [`SftpTransport`].
#[derive(Clone)]
pub struct SftpConfig {
    /// Server address as `host:port`.
    pub addr: String,
    /// Login username.
    pub username: String,
    /// Client authentication.
    pub auth: SftpAuth,
    /// Server host-key trust policy.
    pub host_keys: HostKeyPolicy,
    /// Remote base directory; transport paths resolve under it.
    pub root: String,
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

// Authentication secrets stay redacted via `SftpAuth`'s `Debug` impl.
impl std::fmt::Debug for SftpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SftpConfig")
            .field("addr", &self.addr)
            .field("username", &self.username)
            .field("auth", &self.auth)
            .field("host_keys", &self.host_keys)
            .field("root", &self.root)
            .field("connect_timeout", &self.connect_timeout)
            .field("command_timeout", &self.command_timeout)
            .field("max_file_bytes", &self.max_file_bytes)
            .finish()
    }
}

impl SftpConfig {
    /// Connect with the given client authentication; set host keys and
    /// other options via the builder methods.
    pub fn new(addr: &str, username: &str, auth: SftpAuth) -> Self {
        Self {
            addr: addr.to_string(),
            username: username.to_string(),
            auth,
            host_keys: HostKeyPolicy::Pinned(Vec::new()),
            root: String::new(),
            connect_timeout: Duration::from_secs(10),
            command_timeout: Some(crate::DEFAULT_COMMAND_TIMEOUT),
            max_file_bytes: crate::DEFAULT_MAX_FILE_BYTES,
        }
    }

    /// Set the host-key trust policy.
    pub fn with_host_keys(mut self, policy: HostKeyPolicy) -> Self {
        self.host_keys = policy;
        self
    }

    /// Resolve transport paths under this remote directory. A leading
    /// `/` is preserved (absolute remote path); trailing slashes are
    /// stripped. `"/"` stays `"/"` (the filesystem root, not no root).
    pub fn with_root(mut self, root: &str) -> Self {
        let trimmed = root.trim_end_matches('/');
        self.root = if trimmed.is_empty() && root.starts_with('/') {
            "/".to_string()
        } else {
            trimmed.to_string()
        };
        self
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

struct VerifyHandler {
    policy: HostKeyPolicy,
}

impl russh::client::Handler for VerifyHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        match &self.policy {
            HostKeyPolicy::AcceptAny => Ok(true),
            HostKeyPolicy::Pinned(pins) => {
                Ok(pins.iter().any(|pin| key_matches(pin, server_public_key)))
            }
        }
    }
}

/// Map SSH connect failures, surfacing host-key pin rejections as
/// [`BatchError::HostKey`]. russh reports `check_server_key = false`
/// as [`russh::Error::UnknownKey`].
fn map_connect_error(e: russh::Error, addr: &str) -> BatchError {
    match e {
        russh::Error::UnknownKey => {
            BatchError::HostKey(format!("{addr}: host key does not match pinned keys"))
        }
        other => BatchError::Ssh(other),
    }
}

/// Compare a pinned OpenSSH public key line against the presented key.
/// Only the key material is compared; comments are ignored.
fn key_matches(pinned_line: &str, presented: &russh::keys::PublicKeyOrCertificate) -> bool {
    let russh::keys::PublicKeyOrCertificate::PublicKey { key, .. } = presented else {
        return false;
    };
    let mut fields = pinned_line.split_whitespace();
    let (Some(_alg), Some(base64), ..) = (fields.next(), fields.next(), fields.next()) else {
        return false;
    };
    match parse_public_key_base64(base64) {
        Ok(pinned) => pinned.key_data() == key.key_data(),
        Err(_) => false,
    }
}

/// SFTP [`FileTransport`].
///
/// Holds one SSH connection with an SFTP subsystem channel. Uploads go to a
/// temp name and are renamed into place so pollers never see half-written
/// files.
pub struct SftpTransport {
    config: SftpConfig,
    ssh: Option<russh::client::Handle<VerifyHandler>>,
    sftp: Option<SftpSession>,
}

/// Disambiguates staging names across same-process transports sharing one
/// clock tick (per-instance counters all restart at 0 and collide).
static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

impl SftpTransport {
    /// Connect, authenticate, and open the SFTP subsystem.
    pub async fn connect(config: SftpConfig) -> Result<Self, BatchError> {
        let handler = VerifyHandler {
            policy: config.host_keys.clone(),
        };
        let ssh_config = Arc::new(russh::client::Config::default());
        let timeout = config.connect_timeout;
        let connected = tokio::time::timeout(
            timeout,
            russh::client::connect(ssh_config, config.addr.as_str(), handler),
        )
        .await
        .map_err(|_| BatchError::Timeout(timeout))?;
        let mut ssh = connected.map_err(|e| map_connect_error(e, &config.addr))?;

        let timeout = config.command_timeout;
        let authenticated = match &config.auth {
            SftpAuth::Password(password) => crate::timed(
                timeout,
                ssh.authenticate_password(config.username.as_str(), password.as_str()),
            )
            .await?
            .success(),
            SftpAuth::KeyFile { path, passphrase } => {
                let pem = tokio::fs::read_to_string(path).await?;
                crate::timed(
                    timeout,
                    Self::authenticate_key(&mut ssh, &config.username, &pem, passphrase.as_deref()),
                )
                .await?
            }
            SftpAuth::KeyText { pem, passphrase } => {
                crate::timed(
                    timeout,
                    Self::authenticate_key(&mut ssh, &config.username, pem, passphrase.as_deref()),
                )
                .await?
            }
        };
        if !authenticated {
            return Err(BatchError::Auth {
                user: config.username.clone(),
            });
        }

        let channel = crate::timed(timeout, ssh.channel_open_session()).await?;
        crate::timed(timeout, channel.request_subsystem(true, "sftp")).await?;
        let sftp = crate::timed(timeout, SftpSession::new(channel.into_stream())).await?;

        Ok(Self {
            config,
            ssh: Some(ssh),
            sftp: Some(sftp),
        })
    }

    async fn authenticate_key(
        ssh: &mut russh::client::Handle<VerifyHandler>,
        username: &str,
        pem: &str,
        passphrase: Option<&str>,
    ) -> Result<bool, BatchError> {
        let key = decode_secret_key(pem, passphrase)
            .map_err(|e| BatchError::Invalid(format!("cannot decode private key: {e}")))?;
        let key = PrivateKeyWithHashAlg::new(Arc::new(key), None);
        Ok(ssh.authenticate_publickey(username, key).await?.success())
    }

    fn sftp(&mut self) -> Result<&SftpSession, BatchError> {
        self.sftp.as_ref().ok_or(BatchError::NotConnected)
    }

    /// Resolve a transport path under the configured remote root.
    ///
    /// An empty resolution (default root, empty path) maps to `"."`:
    /// real OpenSSH answers `""` with an empty listing even when the
    /// login dir is full, which would stall the poller with no error.
    fn remote(&self, path: &str) -> Result<String, BatchError> {
        let relative = crate::transport::clean_relative_parts(path)?.join("/");
        if self.config.root.is_empty() {
            if relative.is_empty() {
                Ok(".".to_string())
            } else {
                Ok(relative)
            }
        } else if relative.is_empty() {
            Ok(self.config.root.clone())
        } else if self.config.root == "/" {
            Ok(format!("/{relative}"))
        } else {
            Ok(format!("{}/{}", self.config.root, relative))
        }
    }

    /// Create parent directories one level at a time, ignoring "already
    /// exists" failures.
    async fn mkdir_p(&mut self, remote_dir: &str) -> Result<(), BatchError> {
        // Absolute-aware: keep the leading `/` so absolute roots create
        // absolute parents instead of resolving under the login dir.
        let mut prefix = if remote_dir.starts_with('/') {
            "/".to_string()
        } else {
            String::new()
        };
        for part in remote_dir.split('/').filter(|p| !p.is_empty()) {
            if !prefix.is_empty() && !prefix.ends_with('/') {
                prefix.push('/');
            }
            prefix.push_str(part);
            let timeout = self.config.command_timeout;
            let exists = crate::timed(timeout, self.sftp()?.try_exists(&prefix)).await?;
            if !exists {
                match crate::timed(timeout, self.sftp()?.create_dir(&prefix)).await {
                    Ok(()) => {}
                    Err(BatchError::Sftp(e)) if is_already_exists(&e) => {}
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(())
    }
}

fn is_no_such_file(e: &russh_sftp::client::error::Error) -> bool {
    matches!(
        e,
        russh_sftp::client::error::Error::Status(status)
            if status.status_code == StatusCode::NoSuchFile
    )
}

/// True for "already exists" mkdir failures: a concurrent put winning the
/// try_exists/create_dir race reports generic `Failure` (this client only
/// speaks draft-02 status codes, which have no `FileAlreadyExists`), so
/// `mkdir_p` tolerates that class like the FTP transport tolerates 550.
/// Best-effort only: genuine failures surface at the subsequent file
/// create or rename.
fn is_already_exists(e: &russh_sftp::client::error::Error) -> bool {
    matches!(
        e,
        russh_sftp::client::error::Error::Status(status)
            if status.status_code == StatusCode::Failure
    )
}

// Staging-name helpers mirror `ftp.rs` (each transport file owns its copy):
// temp names carry the `.tmp-` prefix so pollers skip them, and listings
// additionally hide `{name}.tmp-{pid}-{counter}` crash leftovers staged
// by older versions.

/// Staging name for an upload: a sibling of `remote` carrying the `.tmp-`
/// prefix, so pollers skip it via [`is_staged_upload`]. The pid, clock
/// nanos, and a process-wide counter keep names unique across concurrent
/// puts from any same-process transport, so two uploads never interleave
/// into one file.
fn staging_temp_name(remote: &str) -> String {
    let (parent, base) = match remote.rsplit_once('/') {
        Some((parent, base)) => (parent, base),
        None => ("", remote),
    };
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let counter = STAGING_COUNTER.fetch_add(1, Ordering::SeqCst);
    if parent.is_empty() {
        format!(".tmp-{pid}-{nanos}-{counter}-{base}")
    } else {
        format!("{parent}/.tmp-{pid}-{nanos}-{counter}-{base}")
    }
}

/// True for staged-upload names, which listings must hide from pollers.
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

/// Strip the configured root from a listed server path so entries
/// report transport-relative. Root `"/"` strips one slash (not
/// `"//"`); no root drops the `"./"` that `read_dir(".")` joins on.
fn strip_list_prefix<'a>(root: &str, path: &'a str) -> &'a str {
    if root.is_empty() {
        return path.strip_prefix("./").unwrap_or(path);
    }
    if root == "/" {
        return path.strip_prefix('/').unwrap_or(path);
    }
    match path.strip_prefix(root) {
        Some(rest) => rest.strip_prefix('/').unwrap_or(path),
        None => path,
    }
}

impl FileTransport for SftpTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        let remote = self.remote(dir)?;
        let timeout = self.config.command_timeout;
        let read_dir = match crate::timed(timeout, self.sftp()?.read_dir(&remote)).await {
            Ok(rd) => rd,
            Err(BatchError::Sftp(e)) if is_no_such_file(&e) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut entries = Vec::new();
        for entry in read_dir {
            let meta = entry.metadata();
            if meta.is_dir() {
                continue;
            }
            // Report paths relative to the configured root.
            let mut path = entry.path();
            path = strip_list_prefix(&self.config.root, &path).to_string();
            if is_staged_upload(&path) {
                continue;
            }
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
        use tokio::io::AsyncReadExt;

        let remote = self.remote(path)?;
        let timeout = self.config.command_timeout;
        let limit = self.config.max_file_bytes;
        let file = match crate::timed(timeout, self.sftp()?.open(&remote)).await {
            Ok(file) => file,
            Err(BatchError::Sftp(e)) if is_no_such_file(&e) => {
                return Err(BatchError::Invalid(format!("no such file: '{path}'")));
            }
            Err(e) => return Err(e),
        };
        let mut capped = file.take(limit.saturating_add(1));
        let mut buf = Vec::new();
        crate::timed(timeout, async {
            capped
                .read_to_end(&mut buf)
                .await
                .map_err(russh_sftp::client::error::Error::from)
        })
        .await?;
        let file = capped.into_inner();
        if buf.len() as u64 > limit {
            let _ = crate::timed(timeout, async {
                file.close()
                    .await
                    .map_err(russh_sftp::client::error::Error::from)
            })
            .await;
            return Err(BatchError::TooLarge {
                path: path.to_string(),
                limit,
            });
        }
        crate::timed(timeout, async {
            file.close()
                .await
                .map_err(russh_sftp::client::error::Error::from)
        })
        .await?;
        Ok(buf)
    }

    async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        // Reject before any I/O, mirroring ftp.rs: with a configured root,
        // `remote("")` is the root dir itself and would otherwise pass a
        // post-resolution check and stage a stray file on the root name.
        if crate::transport::clean_relative_parts(path)?.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        let remote = self.remote(path)?;
        if let Some((parent, _)) = remote.rsplit_once('/') {
            self.mkdir_p(parent).await?;
        }
        let temp = staging_temp_name(&remote);
        // `create` (not `write`) so the temp file is created and truncated.
        let timeout = self.config.command_timeout;
        let upload = async {
            let sftp = self.sftp()?;
            let mut file = sftp.create(&temp).await?;
            file.write_all(data)
                .await
                .map_err(russh_sftp::client::error::Error::from)?;
            file.close()
                .await
                .map_err(russh_sftp::client::error::Error::from)?;
            Ok::<(), BatchError>(())
        };
        if let Err(e) = crate::timed(timeout, upload).await {
            let _ = crate::timed(timeout, self.sftp()?.remove_file(&temp)).await;
            return Err(e);
        }
        if let Err(e) = crate::timed(timeout, self.sftp()?.rename(&temp, &remote)).await {
            let _ = crate::timed(timeout, self.sftp()?.remove_file(&temp)).await;
            return Err(e);
        }
        Ok(())
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        let remote = self.remote(path)?;
        let timeout = self.config.command_timeout;
        match crate::timed(timeout, self.sftp()?.remove_file(&remote)).await {
            Ok(()) => Ok(()),
            Err(BatchError::Sftp(e)) if is_no_such_file(&e) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
        // Like MemTransport: an empty cleaned destination is Invalid, not
        // a rename onto the login dir / configured root.
        if crate::transport::clean_relative_parts(to)?.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        let from_remote = self.remote(from)?;
        let to_remote = self.remote(to)?;
        if let Some((parent, _)) = to_remote.rsplit_once('/') {
            self.mkdir_p(parent).await?;
        }
        let timeout = self.config.command_timeout;
        match crate::timed(timeout, self.sftp()?.rename(&from_remote, &to_remote)).await {
            Ok(()) => Ok(()),
            Err(BatchError::Sftp(e)) if is_no_such_file(&e) => {
                Err(BatchError::Invalid(format!("no such file: '{from}'")))
            }
            Err(e) => Err(e),
        }
    }

    async fn close(&mut self) -> Result<(), BatchError> {
        let timeout = self.config.command_timeout;
        if let Some(sftp) = self.sftp.take() {
            crate::timed(timeout, sftp.close()).await?;
        }
        if let Some(ssh) = self.ssh.take() {
            crate::timed(timeout, ssh.disconnect(Disconnect::ByApplication, "", "")).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_all_secret_shapes() {
        let password = format!("{:?}", SftpAuth::Password("hunter2".to_string()));
        assert!(!password.contains("hunter2"), "leaked: {password}");

        let key_file = format!(
            "{:?}",
            SftpAuth::KeyFile {
                path: PathBuf::from("/keys/id"),
                passphrase: Some("file-phrase".to_string()),
            }
        );
        assert!(!key_file.contains("file-phrase"), "leaked: {key_file}");
        assert!(key_file.contains("/keys/id"));

        let key_text = format!(
            "{:?}",
            SftpAuth::KeyText {
                pem: "PEM-BYTES".to_string(),
                passphrase: Some("text-phrase".to_string()),
            }
        );
        assert!(!key_text.contains("PEM-BYTES"), "leaked: {key_text}");
        assert!(!key_text.contains("text-phrase"), "leaked: {key_text}");

        let config = SftpConfig::new("host:22", "user", SftpAuth::Password("hunter2".into()));
        let debug = format!("{config:?}");
        assert!(!debug.contains("hunter2"), "leaked: {debug}");
        assert!(debug.contains("host:22"));
    }

    #[test]
    fn defaults_carry_size_cap_and_command_timeout() {
        let config = SftpConfig::new("host:22", "u", SftpAuth::Password("p".into()));
        assert_eq!(config.max_file_bytes, crate::DEFAULT_MAX_FILE_BYTES);
        assert_eq!(config.command_timeout, Some(crate::DEFAULT_COMMAND_TIMEOUT));
        let config = config.with_max_file_bytes(512).with_command_timeout(None);
        assert_eq!(config.max_file_bytes, 512);
        assert_eq!(config.command_timeout, None);
    }

    #[test]
    fn staged_upload_names_are_detected() {
        // Current staging names: `.tmp-{pid}-{nanos}-{counter}-{file}`.
        assert!(is_staged_upload("inbox/.tmp-123-1-a.hl7"));
        assert!(is_staged_upload(".tmp-123-1-a.hl7"));
        // Legacy staging names: `{file}.tmp-{pid}-{counter}`.
        assert!(is_staged_upload("inbox/b.hl7.tmp-123-1"));
        // Ordinary files are never hidden.
        assert!(!is_staged_upload("inbox/a.hl7"));
        assert!(!is_staged_upload("report.tmp-backup.hl7"));
        assert!(!is_staged_upload("inbox/x.tmp-1-2-3"));
    }

    #[test]
    fn unknown_key_maps_to_host_key_error() {
        let err = map_connect_error(russh::Error::UnknownKey, "host:22");
        assert!(matches!(err, BatchError::HostKey(_)));
        assert!(err.to_string().contains("host:22"));
    }

    #[test]
    fn mkdir_tolerates_only_raced_create_failures() {
        use russh_sftp::client::error::Error as SftpError;
        use russh_sftp::protocol::{Status, StatusCode};

        let status = |code| {
            SftpError::Status(Status {
                id: 0,
                status_code: code,
                error_message: String::new(),
                language_tag: String::new(),
            })
        };
        // A concurrent put winning the mkdir race reports generic
        // Failure: tolerated (no live-server repro available).
        assert!(is_already_exists(&status(StatusCode::Failure)));
        // Anything else still propagates from mkdir_p.
        for code in [
            StatusCode::Ok,
            StatusCode::Eof,
            StatusCode::NoSuchFile,
            StatusCode::PermissionDenied,
            StatusCode::BadMessage,
            StatusCode::NoConnection,
            StatusCode::ConnectionLost,
            StatusCode::OpUnsupported,
        ] {
            assert!(!is_already_exists(&status(code)), "{code:?}");
        }
        assert!(!is_already_exists(&SftpError::Timeout));
    }

    #[test]
    fn other_ssh_errors_stay_ssh_errors() {
        let err = map_connect_error(russh::Error::Disconnect, "host:22");
        assert!(matches!(err, BatchError::Ssh(_)));
    }

    #[test]
    fn two_instances_must_not_share_staging_names() {
        // Two same-process instances staging concurrent puts must never
        // pick the same temp name, or their bytes interleave into one
        // file while both puts report Ok.
        let names: Vec<_> = (0..64)
            .map(|_| staging_temp_name("root/inbox/a.hl7"))
            .collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), names.len(), "staging names collided");
    }

    fn unconnected(config: SftpConfig) -> SftpTransport {
        SftpTransport {
            config,
            ssh: None,
            sftp: None,
        }
    }

    #[test]
    fn empty_remote_resolves_to_login_dir() {
        // Real OpenSSH lists nothing for "" against a non-empty login
        // dir, so the transport root (""/".") must map to "." like the
        // FTP (None dir) and filesystem transports list the login dir.
        let config = SftpConfig::new("host:22", "u", SftpAuth::Password("p".into()));
        let transport = unconnected(config);
        assert_eq!(transport.remote("").unwrap(), ".");
        assert_eq!(transport.remote(".").unwrap(), ".");
        assert_eq!(transport.remote("inbox").unwrap(), "inbox");
        // A configured root still anchors every path.
        let config =
            SftpConfig::new("host:22", "u", SftpAuth::Password("p".into())).with_root("base");
        let transport = unconnected(config);
        assert_eq!(transport.remote("").unwrap(), "base");
        assert_eq!(transport.remote("inbox").unwrap(), "base/inbox");
    }

    #[tokio::test]
    async fn put_still_rejects_empty_paths() {
        // `remote("")` is now "." (the login dir), but uploading to it
        // must keep failing as an empty path, not stage a temp file.
        let config = SftpConfig::new("host:22", "u", SftpAuth::Password("p".into()));
        let mut transport = unconnected(config);
        assert!(matches!(
            transport.put("", b"x").await,
            Err(BatchError::Invalid(_))
        ));
    }

    #[tokio::test]
    async fn put_with_root_rejects_empty_path_before_io() {
        // With a configured root, `remote("")` resolves to the root dir
        // itself, which used to pass the post-resolution empty check and
        // stage a stray file squatting on the root dir name. Like
        // MemTransport/FTP/filesystem, put("") must fail as Invalid before
        // any I/O (unconnected: no NotConnected allowed).
        let config =
            SftpConfig::new("host:22", "u", SftpAuth::Password("p".into())).with_root("base");
        let mut transport = unconnected(config);
        match transport.put("", b"X-STRAY").await {
            Err(BatchError::Invalid(msg)) => assert_eq!(msg, "empty path"),
            other => panic!("with-root put(\"\") gave {other:?}"),
        }
    }

    #[test]
    fn with_root_slash_stays_absolute() {
        // "/" is the filesystem root, not "no root": it must survive
        // with_root, anchor remote() paths absolutely, and strip as a
        // single "/" in list() rather than "//" or nothing.
        let config = SftpConfig::new("host:22", "u", SftpAuth::Password("p".into())).with_root("/");
        assert_eq!(config.root, "/");
        let transport = unconnected(config);
        assert_eq!(transport.remote("").unwrap(), "/");
        assert_eq!(transport.remote("inbox").unwrap(), "/inbox");
        assert_eq!(strip_list_prefix("/", "/inbox/a.hl7"), "inbox/a.hl7");
        assert_eq!(
            strip_list_prefix("/data", "/data/inbox/a.hl7"),
            "inbox/a.hl7"
        );
        assert_eq!(strip_list_prefix("", "./a.hl7"), "a.hl7");
    }

    #[test]
    fn with_root_preserves_leading_slash() {
        // An absolute root must stay absolute; only trailing slashes go.
        let config =
            SftpConfig::new("host:22", "u", SftpAuth::Password("p".into())).with_root("/data/drop");
        assert_eq!(config.root, "/data/drop");
        let config = SftpConfig::new("host:22", "u", SftpAuth::Password("p".into()))
            .with_root("/data/drop///");
        assert_eq!(config.root, "/data/drop");
        let transport = unconnected(config);
        assert_eq!(transport.remote("inbox").unwrap(), "/data/drop/inbox");
    }

    #[tokio::test]
    async fn rename_rejects_empty_destination() {
        // No connection: the empty-path check must fire before any I/O,
        // mirroring MemTransport's Invalid("empty path").
        let config = SftpConfig::new("host:22", "u", SftpAuth::Password("p".into()));
        let mut transport = unconnected(config);
        match transport.rename("inbox/a.hl7", "").await {
            Err(BatchError::Invalid(msg)) => assert_eq!(msg, "empty path"),
            other => panic!("rename to empty path gave {other:?}"),
        }
    }

    #[test]
    fn staging_temp_name_is_a_hidden_sibling() {
        let pid = std::process::id();
        for temp in [
            staging_temp_name("root/inbox/a.hl7"),
            staging_temp_name("a.hl7"),
        ] {
            let base = temp.rsplit('/').next().unwrap();
            assert!(base.starts_with(&format!(".tmp-{pid}-")), "{temp}");
            assert!(base.ends_with("-a.hl7"), "{temp}");
            // Everything `put` stages must be invisible to `list`.
            assert!(is_staged_upload(&temp));
        }
        assert!(staging_temp_name("root/inbox/a.hl7").starts_with("root/inbox/"));
    }
}
