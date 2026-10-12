//! FTP transport tests against an in-process libunftp server.

#![cfg(feature = "ftp")]

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

use libunftp::ServerBuilder;
use libunftp::auth::AnonymousAuthenticator;
use unftp_sbe_fs::Filesystem;
use vyna_hl7_batch::ftp::{FtpConfig, FtpTransport};
use vyna_hl7_batch::{BatchError, FileTransport, Poller};

const MSH: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5";

static PORT: AtomicU16 = AtomicU16::new(21210);

fn scratch_dir(suffix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("vyna-ftp-test-{}-{suffix}", std::process::id()))
}

/// Start an anonymous FTP server rooted at `root`; returns the control address.
async fn start_server(root: &std::path::Path) -> SocketAddr {
    std::fs::create_dir_all(root).unwrap();
    let port = PORT.fetch_add(1, Ordering::SeqCst);
    // Distinct passive range per server so parallel tests never share ports.
    let base: u16 = 30000 + (port - 21210) * 20;
    let root = root.to_path_buf();
    let server = ServerBuilder::new(Box::new(move || Filesystem::new(root.clone()).unwrap()))
        .authenticator(Arc::new(AnonymousAuthenticator))
        .greeting("vyna test ftp")
        .passive_ports(base..=base + 19)
        .build()
        .unwrap();
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
    tokio::spawn(async move {
        server.listen(addr.to_string()).await.unwrap();
    });
    // Wait until the control port accepts.
    for _ in 0..100 {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            return addr;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("ftp test server did not start on {addr}");
}

async fn connect(addr: SocketAddr) -> FtpTransport {
    FtpTransport::connect(FtpConfig::new(&addr.to_string(), "anonymous", "test"))
        .await
        .unwrap()
}

#[tokio::test]
async fn ftp_round_trip() {
    let dir = scratch_dir("roundtrip");
    let addr = start_server(&dir).await;
    let mut ftp = connect(addr).await;

    ftp.put("inbox/a.hl7", MSH.as_bytes()).await.unwrap();
    ftp.put("inbox/b.hl7", MSH.as_bytes()).await.unwrap();

    let entries = ftp.list("inbox").await.unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].path, "inbox/a.hl7");
    assert_eq!(entries[0].size, Some(MSH.len() as u64));

    assert_eq!(ftp.get("inbox/a.hl7").await.unwrap(), MSH.as_bytes());

    ftp.rename("inbox/a.hl7", "archive/a.hl7").await.unwrap();
    assert_eq!(ftp.list("inbox").await.unwrap().len(), 1);
    assert_eq!(ftp.get("archive/a.hl7").await.unwrap(), MSH.as_bytes());

    ftp.delete("inbox/b.hl7").await.unwrap();
    ftp.delete("inbox/b.hl7").await.unwrap(); // idempotent
    assert!(ftp.list("inbox").await.unwrap().is_empty());

    ftp.close().await.unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn ftp_missing_paths_behave_like_filesystem() {
    let dir = scratch_dir("missing");
    let addr = start_server(&dir).await;
    let mut ftp = connect(addr).await;

    assert!(ftp.list("no-such-dir").await.unwrap().is_empty());
    ftp.delete("no-such-dir/no.hl7").await.unwrap();
    assert!(matches!(
        ftp.get("no-such-dir/no.hl7").await,
        Err(BatchError::Invalid(_))
    ));
    assert!(matches!(
        ftp.rename("no-such-dir/no.hl7", "elsewhere.hl7").await,
        Err(BatchError::Invalid(_))
    ));

    ftp.close().await.unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn ftp_poller_consumes_and_archives() {
    let dir = scratch_dir("poller");
    let addr = start_server(&dir).await;
    let mut ftp = connect(addr).await;
    ftp.put("inbox/drop.hl7", MSH.as_bytes()).await.unwrap();

    let mut poller = Poller::new(ftp)
        .with_inbox("inbox")
        .with_archive_dir("archive");
    assert!(poller.poll_once().await.unwrap().files.is_empty()); // unstable
    let report = poller.poll_once().await.unwrap();
    assert_eq!(report.files.len(), 1);
    assert_eq!(report.files[0].ok_count(), 1);

    let ftp = poller.transport_mut();
    assert_eq!(ftp.get("archive/drop.hl7").await.unwrap(), MSH.as_bytes());
    ftp.close().await.unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
}

#[tokio::test]
async fn ftp_connection_refused_is_an_error() {
    // Nothing listens here: must fail fast, not hang.
    let config =
        FtpConfig::new("127.0.0.1:9", "u", "p").with_connect_timeout(Duration::from_secs(2));
    let result = FtpTransport::connect(config).await;
    assert!(matches!(result, Err(BatchError::Ftp(_))));
}
