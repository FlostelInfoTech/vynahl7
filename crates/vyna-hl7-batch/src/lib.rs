//! Batch file exchange for HL7 v2.x: building batch files, parsing inbound
//! drops leniently, and moving them over filesystem drop folders, FTP, or SFTP.
//!
//! The batch/file data model ([`vyna_hl7_core::batch`]) and strict parsing
//! ([`vyna_hl7_parser::parse_batch`], [`vyna_hl7_parser::parse_file`]) already
//! exist; this crate adds the operational layer around them:
//!
//! - [`BatchFileBuilder`]: wrap messages in `FHS`/`BHS`/`BTS`/`FTS` envelopes
//!   with generated timestamps, control ids, and correct counts.
//! - [`Intake`]: lenient inbound parsing. Auto-detects file, batch, or bare
//!   message sequences and reports per-message failures instead of rejecting
//!   the whole drop because of one bad message.
//! - [`FileTransport`]: async trait for file movement, with a
//!   [`FilesystemTransport`] drop-folder implementation built in,
//!   `FtpTransport` behind `feature = "ftp"`, and `SftpTransport`
//!   behind `feature = "sftp"`.
//! - [`Poller`]: poll an inbox, parse each file with [`Intake`], and settle
//!   (archive or delete) what was consumed.
//!
//! # Example
//!
//! ```no_run
//! use vyna_hl7_batch::{BatchFileBuilder, FileTransport, FilesystemTransport, Poller};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Outbound: wrap messages and drop the file.
//! let file = BatchFileBuilder::new()
//!     .sending_application("SENDER")
//!     .message(vyna_hl7_parser::parse_message(
//!         "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|1|P|2.5",
//!     )?)
//!     .build_file();
//! let mut outbox = FilesystemTransport::open("/data/hl7/outbox")?;
//! outbox.put("adt.hl7", file.encode().as_bytes()).await?;
//!
//! // Inbound: poll, parse leniently, archive what was consumed.
//! let inbox = FilesystemTransport::open("/data/hl7/inbox")?;
//! let mut poller = Poller::new(inbox).with_archive_dir("archive");
//! let report = poller.poll_once().await?;
//! for polled in &report.files {
//!     println!("{}: {} ok, {} failed", polled.name, polled.ok_count(), polled.error_count());
//! }
//! # Ok(())
//! # }
//! ```

mod builder;
mod intake;
mod poller;
mod transport;

#[cfg(feature = "ftp")]
pub mod ftp;
#[cfg(feature = "sftp")]
pub mod sftp;
#[cfg(feature = "testing")]
pub mod testing;

pub use builder::BatchFileBuilder;
pub use intake::{Intake, IntakeBatch, IntakeMessage};
pub use poller::{PollFileError, PollReport, PolledFile, Poller, SkippedFile};
pub use transport::{FileEntry, FileTransport, FilesystemTransport};

/// Errors from batch building, intake parsing, and file transports.
#[derive(Debug, thiserror::Error)]
pub enum BatchError {
    /// Batch/file envelope is malformed.
    #[error("invalid batch: {0}")]
    Invalid(String),
    /// A message inside a batch failed to parse (see [`Intake`] for lenient handling).
    #[error("message {index} failed to parse: {message}")]
    Message {
        /// 0-based index of the message within the intake unit.
        index: usize,
        /// Parser error text.
        message: String,
    },
    /// Declared count (BTS-1/FTS-1) does not match actual content.
    #[error("count mismatch in {location}: declared {declared}, actual {actual}")]
    CountMismatch {
        /// Which trailer disagreed (`BTS` or `FTS`).
        location: &'static str,
        /// Count declared in the trailer.
        declared: usize,
        /// Count actually present.
        actual: usize,
    },
    /// Local filesystem failure.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// FTP protocol failure.
    #[cfg(feature = "ftp")]
    #[error("FTP error: {0}")]
    Ftp(#[from] suppaftp::FtpError),
    /// SSH/SFTP failure.
    #[cfg(feature = "sftp")]
    #[error("SSH error: {0}")]
    Ssh(#[from] russh::Error),
    /// SFTP subsystem failure.
    #[cfg(feature = "sftp")]
    #[error("SFTP error: {0}")]
    Sftp(#[from] russh_sftp::client::error::Error),
    /// Authentication rejected by the server.
    #[error("authentication failed for user '{user}'")]
    Auth {
        /// User the server rejected.
        user: String,
    },
    /// Server host key was not trusted.
    #[error("untrusted host key: {0}")]
    HostKey(String),
    /// Operation timed out.
    #[error("operation timed out after {0:?}")]
    Timeout(std::time::Duration),
    /// Transport is not connected.
    #[error("transport is not connected")]
    NotConnected,
    /// Download refused: the file exceeds the transport's size limit.
    #[error("file '{path}' exceeds size limit of {limit} bytes")]
    TooLarge {
        /// Transport-relative path that was refused.
        path: String,
        /// Configured limit in bytes.
        limit: u64,
    },
}

/// Default cap (64 MiB) for a single downloaded file on the FTP and SFTP
/// transports and for files consumed by [`Poller`].
pub const DEFAULT_MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Default cap (100 files) on how many files one [`Poller::poll_once`] call
/// consumes. Leftovers stay for the next poll, so this bounds work per poll
/// without losing files.
pub const DEFAULT_MAX_FILES_PER_POLL: usize = 100;

/// Default per-operation timeout (60s) for FTP/SFTP protocol commands.
pub const DEFAULT_COMMAND_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

/// Bound a protocol operation by an optional timeout. `None` waits forever
/// (previous behavior); `Some(d)` fails with [`BatchError::Timeout`].
#[cfg(any(feature = "ftp", feature = "sftp"))]
pub(crate) async fn timed<T, E>(
    timeout: Option<std::time::Duration>,
    op: impl Future<Output = Result<T, E>>,
) -> Result<T, BatchError>
where
    BatchError: From<E>,
{
    match timeout {
        Some(d) => tokio::time::timeout(d, op)
            .await
            .map_err(|_| BatchError::Timeout(d))?
            .map_err(BatchError::from),
        None => op.await.map_err(BatchError::from),
    }
}
