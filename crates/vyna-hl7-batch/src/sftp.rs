//! SFTP drop transport (`feature = "sftp"`).
//!
//! Encrypted file transfer over SSH with password or public-key
//! authentication and server host-key pinning.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use russh::Disconnect;
use russh::keys::{PrivateKeyWithHashAlg, decode_secret_key, parse_public_key_base64};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::StatusCode;
use tokio::io::AsyncWriteExt;

use crate::{BatchError, FileEntry, FileTransport};

/// Client authentication for [`SftpTransport`].
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
}

impl SftpConfig {
    /// Password authentication with pinned host keys required to be set
    /// separately; use the builder methods to finish configuration.
    pub fn new(addr: &str, username: &str, auth: SftpAuth) -> Self {
        Self {
            addr: addr.to_string(),
            username: username.to_string(),
            auth,
            host_keys: HostKeyPolicy::Pinned(Vec::new()),
            root: String::new(),
            connect_timeout: Duration::from_secs(10),
        }
    }

    /// Set the host-key trust policy.
    pub fn with_host_keys(mut self, policy: HostKeyPolicy) -> Self {
        self.host_keys = policy;
        self
    }

    /// Resolve transport paths under this remote directory.
    pub fn with_root(mut self, root: &str) -> Self {
        self.root = root.trim_matches('/').to_string();
        self
    }

    /// Override the TCP connect timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
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
    temp_counter: u64,
}

impl SftpTransport {
    /// Connect, authenticate, and open the SFTP subsystem.
    pub async fn connect(config: SftpConfig) -> Result<Self, BatchError> {
        let handler = VerifyHandler {
            policy: config.host_keys.clone(),
        };
        let ssh_config = Arc::new(russh::client::Config::default());
        let timeout = config.connect_timeout;
        let mut ssh = tokio::time::timeout(
            timeout,
            russh::client::connect(ssh_config, config.addr.as_str(), handler),
        )
        .await
        .map_err(|_| BatchError::Timeout(timeout))??;

        let authenticated = match &config.auth {
            SftpAuth::Password(password) => ssh
                .authenticate_password(config.username.as_str(), password.as_str())
                .await?
                .success(),
            SftpAuth::KeyFile { path, passphrase } => {
                let pem = std::fs::read_to_string(path)?;
                Self::authenticate_key(&mut ssh, &config.username, &pem, passphrase.as_deref())
                    .await?
            }
            SftpAuth::KeyText { pem, passphrase } => {
                Self::authenticate_key(&mut ssh, &config.username, pem, passphrase.as_deref())
                    .await?
            }
        };
        if !authenticated {
            return Err(BatchError::Auth {
                user: config.username.clone(),
            });
        }

        let channel = ssh.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;

        Ok(Self {
            config,
            ssh: Some(ssh),
            sftp: Some(sftp),
            temp_counter: 0,
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
    fn remote(&self, path: &str) -> Result<String, BatchError> {
        let relative = crate::transport::clean_relative_parts(path)?.join("/");
        if self.config.root.is_empty() {
            Ok(relative)
        } else if relative.is_empty() {
            Ok(self.config.root.clone())
        } else {
            Ok(format!("{}/{}", self.config.root, relative))
        }
    }

    /// Create parent directories one level at a time.
    async fn mkdir_p(&mut self, remote_dir: &str) -> Result<(), BatchError> {
        let mut prefix = String::new();
        for part in remote_dir.split('/').filter(|p| !p.is_empty()) {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            let exists = self.sftp()?.try_exists(&prefix).await?;
            if !exists {
                self.sftp()?.create_dir(&prefix).await?;
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

impl FileTransport for SftpTransport {
    async fn list(&mut self, dir: &str) -> Result<Vec<FileEntry>, BatchError> {
        let remote = self.remote(dir)?;
        let read_dir = match self.sftp()?.read_dir(&remote).await {
            Ok(rd) => rd,
            Err(e) if is_no_such_file(&e) => return Ok(Vec::new()),
            Err(e) => return Err(BatchError::from(e)),
        };
        let mut entries = Vec::new();
        for entry in read_dir {
            let meta = entry.metadata();
            if meta.is_dir() {
                continue;
            }
            // Report paths relative to the configured root.
            let mut path = entry.path();
            if !self.config.root.is_empty() {
                let prefix = format!("{}/", self.config.root);
                path = path.strip_prefix(&prefix).unwrap_or(&path).to_string();
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
        let remote = self.remote(path)?;
        match self.sftp()?.read(&remote).await {
            Ok(bytes) => Ok(bytes),
            Err(e) if is_no_such_file(&e) => {
                Err(BatchError::Invalid(format!("no such file: '{path}'")))
            }
            Err(e) => Err(BatchError::from(e)),
        }
    }

    async fn put(&mut self, path: &str, data: &[u8]) -> Result<(), BatchError> {
        let remote = self.remote(path)?;
        if remote.is_empty() {
            return Err(BatchError::Invalid("empty path".to_string()));
        }
        if let Some((parent, _)) = remote.rsplit_once('/') {
            self.mkdir_p(parent).await?;
        }
        self.temp_counter += 1;
        let temp = format!("{remote}.tmp-{}-{}", std::process::id(), self.temp_counter);
        // `create` (not `write`) so the temp file is created and truncated.
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
        if let Err(e) = upload.await {
            let _ = self.sftp()?.remove_file(&temp).await;
            return Err(e);
        }
        self.sftp()?.rename(&temp, &remote).await?;
        Ok(())
    }

    async fn delete(&mut self, path: &str) -> Result<(), BatchError> {
        let remote = self.remote(path)?;
        match self.sftp()?.remove_file(&remote).await {
            Ok(()) => Ok(()),
            Err(e) if is_no_such_file(&e) => Ok(()),
            Err(e) => Err(BatchError::from(e)),
        }
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), BatchError> {
        let from_remote = self.remote(from)?;
        let to_remote = self.remote(to)?;
        if let Some((parent, _)) = to_remote.rsplit_once('/') {
            self.mkdir_p(parent).await?;
        }
        match self.sftp()?.rename(&from_remote, &to_remote).await {
            Ok(()) => Ok(()),
            Err(e) if is_no_such_file(&e) => {
                Err(BatchError::Invalid(format!("no such file: '{from}'")))
            }
            Err(e) => Err(BatchError::from(e)),
        }
    }

    async fn close(&mut self) -> Result<(), BatchError> {
        if let Some(sftp) = self.sftp.take() {
            sftp.close().await?;
        }
        if let Some(ssh) = self.ssh.take() {
            ssh.disconnect(Disconnect::ByApplication, "", "").await?;
        }
        Ok(())
    }
}
