//! SFTP transport tests against an in-process russh server backed by a
//! filesystem SFTP handler.

#![cfg(feature = "sftp")]

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use russh::server::{Auth, Msg, Session};
use russh::{Channel, ChannelId};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode,
};
use vyna_hl7_batch::sftp::{HostKeyPolicy, SftpAuth, SftpConfig, SftpTransport};
use vyna_hl7_batch::{BatchError, FileTransport, Poller};

const MSH: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5";

// ---------------------------------------------------------------------------
// Filesystem-backed SFTP subsystem handler (test only)
// ---------------------------------------------------------------------------

struct OpenTestFile {
    file: std::fs::File,
    append: bool,
}

struct FsSftp {
    root: PathBuf,
    files: HashMap<String, OpenTestFile>,
    dirs: HashMap<String, VecDeque<(String, FileAttributes)>>,
    next_handle: u64,
}

impl FsSftp {
    fn new(root: PathBuf) -> Self {
        Self {
            root,
            files: HashMap::new(),
            dirs: HashMap::new(),
            next_handle: 0,
        }
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, StatusCode> {
        let mut resolved = self.root.clone();
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => return Err(StatusCode::PermissionDenied),
                _ => resolved.push(part),
            }
        }
        Ok(resolved)
    }

    fn take_handle(&mut self) -> String {
        self.next_handle += 1;
        format!("h{}", self.next_handle)
    }
}

fn ok_status(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: "ok".to_string(),
        language_tag: "en-US".to_string(),
    }
}

fn io_to_status(e: std::io::Error) -> StatusCode {
    match e.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        std::io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
        _ => StatusCode::Failure,
    }
}

fn attrs_of(path: &Path) -> Result<FileAttributes, StatusCode> {
    std::fs::metadata(path)
        .map(|m| FileAttributes::from(&m))
        .map_err(io_to_status)
}

impl russh_sftp::server::Handler for FsSftp {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, Self::Error> {
        let path = self.resolve(&filename)?;
        let mut options = std::fs::OpenOptions::new();
        options
            .read(pflags.contains(OpenFlags::READ))
            .write(pflags.contains(OpenFlags::WRITE) || pflags.contains(OpenFlags::APPEND))
            .append(pflags.contains(OpenFlags::APPEND))
            .create(pflags.contains(OpenFlags::CREATE))
            .truncate(pflags.contains(OpenFlags::TRUNCATE));
        let file = options.open(&path).map_err(io_to_status)?;
        let handle = self.take_handle();
        self.files.insert(
            handle.clone(),
            OpenTestFile {
                file,
                append: pflags.contains(OpenFlags::APPEND),
            },
        );
        Ok(Handle { id, handle })
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, Self::Error> {
        self.files.remove(&handle);
        self.dirs.remove(&handle);
        Ok(ok_status(id))
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, Self::Error> {
        let open = self.files.get_mut(&handle).ok_or(StatusCode::Failure)?;
        open.file
            .seek(SeekFrom::Start(offset))
            .map_err(io_to_status)?;
        let mut buf = vec![0u8; len.min(65536) as usize];
        let n = open.file.read(&mut buf).map_err(io_to_status)?;
        if n == 0 {
            return Err(StatusCode::Eof);
        }
        buf.truncate(n);
        Ok(Data { id, data: buf })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, Self::Error> {
        let open = self.files.get_mut(&handle).ok_or(StatusCode::Failure)?;
        if open.append {
            open.file.seek(SeekFrom::End(0)).map_err(io_to_status)?;
        } else {
            open.file
                .seek(SeekFrom::Start(offset))
                .map_err(io_to_status)?;
        }
        open.file.write_all(&data).map_err(io_to_status)?;
        Ok(ok_status(id))
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Ok(Attrs {
            id,
            attrs: attrs_of(&self.resolve(&path)?)?,
        })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, Self::Error> {
        Ok(Attrs {
            id,
            attrs: attrs_of(&self.resolve(&path)?)?,
        })
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, Self::Error> {
        let resolved = self.resolve(&path)?;
        let mut entries = VecDeque::new();
        for entry in std::fs::read_dir(&resolved).map_err(io_to_status)? {
            let entry = entry.map_err(io_to_status)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let attrs = FileAttributes::from(&entry.metadata().map_err(io_to_status)?);
            entries.push_back((name, attrs));
        }
        let handle = self.take_handle();
        self.dirs.insert(handle.clone(), entries);
        Ok(Handle { id, handle })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        let entries = self.dirs.get_mut(&handle).ok_or(StatusCode::Failure)?;
        if entries.is_empty() {
            return Err(StatusCode::Eof);
        }
        let files: Vec<File> = entries
            .drain(..)
            .map(|(name, attrs)| File::new(name, attrs))
            .collect();
        Ok(Name { id, files })
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, Self::Error> {
        std::fs::remove_file(self.resolve(&filename)?).map_err(io_to_status)?;
        Ok(ok_status(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, Self::Error> {
        std::fs::create_dir(self.resolve(&path)?).map_err(io_to_status)?;
        Ok(ok_status(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, Self::Error> {
        std::fs::remove_dir(self.resolve(&path)?).map_err(io_to_status)?;
        Ok(ok_status(id))
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, Self::Error> {
        let cleaned = format!(
            "/{}",
            path.split('/')
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join("/")
        );
        Ok(Name {
            id,
            files: vec![File::dummy(cleaned)],
        })
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, Self::Error> {
        let from = self.resolve(&oldpath)?;
        let to = self.resolve(&newpath)?;
        std::fs::rename(&from, &to).map_err(io_to_status)?;
        Ok(ok_status(id))
    }
}

// ---------------------------------------------------------------------------
// Test SSH server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct TestSsh {
    root: PathBuf,
    user: String,
    password: String,
    authorized: Vec<russh::keys::PublicKey>,
    channels: Arc<tokio::sync::Mutex<HashMap<ChannelId, Channel<Msg>>>>,
}

impl russh::server::Server for TestSsh {
    type Handler = Self;

    fn new_client(&mut self, _peer: Option<std::net::SocketAddr>) -> Self::Handler {
        self.clone()
    }
}

impl russh::server::Handler for TestSsh {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        if user == self.user && password == self.password {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::Reject {
                proceed_with_methods: None,
                partial_success: false,
            })
        }
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        public_key: &russh::keys::PublicKey,
    ) -> Result<Auth, Self::Error> {
        if user == self.user
            && self
                .authorized
                .iter()
                .any(|k| k.key_data() == public_key.key_data())
        {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::Reject {
                proceed_with_methods: None,
                partial_success: false,
            })
        }
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.lock().await.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.close(channel)?;
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name == "sftp" {
            let channel = self.channels.lock().await.remove(&channel_id).unwrap();
            let sftp = FsSftp::new(self.root.clone());
            session.channel_success(channel_id)?;
            russh_sftp::server::run(channel.into_stream(), sftp).await;
        } else {
            session.channel_failure(channel_id)?;
        }
        Ok(())
    }
}

struct TestServer {
    addr: std::net::SocketAddr,
    host_key_line: String,
    _dir: PathBuf,
}

/// Start the test SSH server; returns its address and host key line for pinning.
async fn start_server(suffix: &str, authorized: Vec<russh::keys::PublicKey>) -> TestServer {
    let dir = std::env::temp_dir().join(format!("vyna-sftp-test-{}-{suffix}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let host_key =
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap();
    let host_key_line = host_key.public_key().to_openssh().unwrap();
    let config = Arc::new(russh::server::Config {
        auth_rejection_time: std::time::Duration::from_secs(1),
        auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
        keys: vec![host_key],
        ..Default::default()
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handler = TestSsh {
        root: dir.clone(),
        user: "tester".to_string(),
        password: "secret".to_string(),
        authorized,
        channels: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    };
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let config = config.clone();
            let handler = handler.clone();
            tokio::spawn(async move {
                let _ = russh::server::run_stream(config, stream, handler).await;
            });
        }
    });
    TestServer {
        addr,
        host_key_line,
        _dir: dir,
    }
}

fn password_config(server: &TestServer) -> SftpConfig {
    SftpConfig::new(
        &server.addr.to_string(),
        "tester",
        SftpAuth::Password("secret".to_string()),
    )
    .with_host_keys(HostKeyPolicy::Pinned(vec![server.host_key_line.clone()]))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sftp_round_trip_with_pinned_host_key() {
    let server = start_server("roundtrip", vec![]).await;
    let mut sftp = SftpTransport::connect(password_config(&server))
        .await
        .unwrap();

    sftp.put("inbox/a.hl7", MSH.as_bytes()).await.unwrap();
    sftp.put("inbox/b.hl7", MSH.as_bytes()).await.unwrap();

    let entries = sftp.list("inbox").await.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, "inbox/a.hl7");
    assert_eq!(entries[0].size, Some(MSH.len() as u64));
    assert!(entries[0].modified_ms.is_some());

    assert_eq!(sftp.get("inbox/a.hl7").await.unwrap(), MSH.as_bytes());

    sftp.rename("inbox/a.hl7", "archive/a.hl7").await.unwrap();
    assert_eq!(sftp.list("inbox").await.unwrap().len(), 1);
    assert_eq!(sftp.get("archive/a.hl7").await.unwrap(), MSH.as_bytes());

    sftp.delete("inbox/b.hl7").await.unwrap();
    sftp.delete("inbox/b.hl7").await.unwrap();
    assert!(sftp.list("inbox").await.unwrap().is_empty());

    sftp.close().await.unwrap();
    std::fs::remove_dir_all(&server._dir).unwrap();
}

#[tokio::test]
async fn sftp_key_auth() {
    let user_key =
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap();
    let pem = user_key
        .to_openssh(russh::keys::ssh_key::LineEnding::LF)
        .unwrap()
        .to_string();
    let server = start_server("keyauth", vec![user_key.public_key().clone()]).await;

    let config = SftpConfig::new(
        &server.addr.to_string(),
        "tester",
        SftpAuth::KeyText {
            pem,
            passphrase: None,
        },
    )
    .with_host_keys(HostKeyPolicy::Pinned(vec![server.host_key_line.clone()]));
    let mut sftp = SftpTransport::connect(config).await.unwrap();
    sftp.put("k.hl7", MSH.as_bytes()).await.unwrap();
    assert_eq!(sftp.get("k.hl7").await.unwrap(), MSH.as_bytes());
    sftp.close().await.unwrap();
    std::fs::remove_dir_all(&server._dir).unwrap();
}

#[tokio::test]
async fn sftp_wrong_host_key_is_rejected() {
    let server = start_server("wrongkey", vec![]).await;
    let other =
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap();
    let config = SftpConfig::new(
        &server.addr.to_string(),
        "tester",
        SftpAuth::Password("secret".to_string()),
    )
    .with_host_keys(HostKeyPolicy::Pinned(vec![
        other.public_key().to_openssh().unwrap(),
    ]));
    // Key exchange fails: the client rejects the host key.
    assert!(SftpTransport::connect(config).await.is_err());
    std::fs::remove_dir_all(&server._dir).unwrap();
}

#[tokio::test]
async fn sftp_wrong_password_is_rejected() {
    let server = start_server("wrongpass", vec![]).await;
    let config = SftpConfig::new(
        &server.addr.to_string(),
        "tester",
        SftpAuth::Password("nope".to_string()),
    )
    .with_host_keys(HostKeyPolicy::AcceptAny);
    assert!(matches!(
        SftpTransport::connect(config).await,
        Err(BatchError::Auth { .. })
    ));
    std::fs::remove_dir_all(&server._dir).unwrap();
}

#[tokio::test]
async fn sftp_missing_paths_behave_like_filesystem() {
    let server = start_server("missing", vec![]).await;
    let mut sftp = SftpTransport::connect(password_config(&server))
        .await
        .unwrap();

    assert!(sftp.list("no-such-dir").await.unwrap().is_empty());
    sftp.delete("no-such-dir/no.hl7").await.unwrap();
    assert!(matches!(
        sftp.get("no-such-dir/no.hl7").await,
        Err(BatchError::Invalid(_))
    ));
    assert!(matches!(
        sftp.rename("no-such-dir/no.hl7", "elsewhere.hl7").await,
        Err(BatchError::Invalid(_))
    ));

    sftp.close().await.unwrap();
    std::fs::remove_dir_all(&server._dir).unwrap();
}

#[tokio::test]
async fn sftp_poller_consumes_and_archives() {
    let server = start_server("poller", vec![]).await;
    let mut sftp = SftpTransport::connect(password_config(&server))
        .await
        .unwrap();
    sftp.put("inbox/drop.hl7", MSH.as_bytes()).await.unwrap();

    let mut poller = Poller::new(sftp)
        .with_inbox("inbox")
        .with_archive_dir("archive")
        .with_require_stable(false);
    let report = poller.poll_once().await.unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].ok_count(), 1);

    let sftp = poller.transport_mut();
    assert_eq!(sftp.get("archive/drop.hl7").await.unwrap(), MSH.as_bytes());
    sftp.close().await.unwrap();
    std::fs::remove_dir_all(&server._dir).unwrap();
}
