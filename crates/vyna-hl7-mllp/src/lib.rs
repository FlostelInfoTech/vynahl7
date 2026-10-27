//! MLLP (Minimal Lower Layer Protocol) support
//!
//! MLLP is a simple framing protocol used for transmitting HL7 messages over TCP.
//! Format: \<VT\>message\<FS\>\<CR\>
//! - VT (Vertical Tab): 0x0B - Start of block
//! - FS (File Separator): 0x1C - End of block
//! - CR (Carriage Return): 0x0D - End of message

// Streaming codec (tokio_util Framed support)
pub mod codec;

// Message routing
pub mod router;

// Connection pooling
pub mod pool;

// TLS/mTLS support (optional)
#[cfg(feature = "tls")]
pub mod tls;

// Testing utilities (optional)
#[cfg(feature = "testing")]
pub mod testing;

use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, ReadHalf, WriteHalf},
    net::{TcpListener, TcpStream},
};
use vyna_hl7_core::{
    charset::CharacterSet,
    error::{Error, Result},
    message::Message,
};
use vyna_hl7_parser::parse_message;

#[cfg(feature = "tls")]
use rustls::pki_types::ServerName;
#[cfg(feature = "tls")]
use tokio_rustls::{TlsAcceptor, TlsConnector};

// Stream wrapper to support both plain TCP and TLS
enum MllpStream {
    Plain(TcpStream),
    #[cfg(feature = "tls")]
    TlsClient(tokio_rustls::client::TlsStream<TcpStream>),
    #[cfg(feature = "tls")]
    TlsServer(tokio_rustls::server::TlsStream<TcpStream>),
}

impl MllpStream {
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            MllpStream::Plain(stream) => stream.read(buf).await,
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => stream.read(buf).await,
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => stream.read(buf).await,
        }
    }

    async fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        match self {
            MllpStream::Plain(stream) => stream.write_all(buf).await,
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => stream.write_all(buf).await,
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => stream.write_all(buf).await,
        }
    }

    async fn flush(&mut self) -> std::io::Result<()> {
        match self {
            MllpStream::Plain(stream) => stream.flush().await,
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => stream.flush().await,
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => stream.flush().await,
        }
    }

    async fn shutdown(&mut self) -> std::io::Result<()> {
        match self {
            MllpStream::Plain(stream) => stream.shutdown().await,
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => stream.shutdown().await,
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => stream.shutdown().await,
        }
    }
}

// All variants wrap types that are `Unpin` (TcpStream, and tokio-rustls
// streams over TcpStream), so delegating through `Pin::new` is sound and
// `tokio::io::split` can be used on the enum directly.
impl AsyncRead for MllpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            MllpStream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => Pin::new(stream).poll_read(cx, buf),
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for MllpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            MllpStream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => Pin::new(stream).poll_write(cx, buf),
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            MllpStream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => Pin::new(stream).poll_flush(cx),
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            MllpStream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => Pin::new(stream).poll_shutdown(cx),
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

// Re-export pool types
pub use pool::{MllpPool, PoolConfig, PoolStats};

/// MLLP start-of-block marker: Vertical Tab (VT), `0x0B`
pub const START_OF_BLOCK: u8 = 0x0B;
/// MLLP end-of-block marker: File Separator (FS), `0x1C`
pub const END_OF_BLOCK: u8 = 0x1C;
/// MLLP frame terminator: Carriage Return (CR), `0x0D`
pub const CARRIAGE_RETURN: u8 = 0x0D;

/// Default maximum message size (10 MB)
/// This prevents DoS attacks via unbounded buffer growth
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

/// Default read timeout (30 seconds)
pub const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Default write timeout (30 seconds)
pub const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Default connection timeout (10 seconds)
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Hook called on each accepted TCP stream before any TLS handshake.
///
/// The hook receives the raw [`TcpStream`] so the caller can apply socket
/// options (e.g. TCP keep-alive via `socket2::SockRef`). An error fails
/// the accept.
pub type PostAcceptHook = Arc<dyn Fn(&TcpStream) -> Result<()> + Send + Sync>;

/// Configuration for MLLP client and server
#[derive(Clone)]
pub struct MllpConfig {
    /// Maximum message size in bytes (default: 10 MB)
    pub max_message_size: usize,
    /// Read timeout (default: 30 seconds)
    pub read_timeout: Duration,
    /// Write timeout (default: 30 seconds)
    pub write_timeout: Duration,
    /// TCP connect and TLS handshake timeout (default: 10 seconds).
    ///
    /// Each phase gets the full timeout; on the server it bounds the
    /// handshake in `accept` so one stalled peer cannot wedge the loop.
    pub connect_timeout: Duration,
    /// Forced inbound character set, overriding MSH-18 detection.
    /// Set this only for peers that misdeclare their encoding;
    /// `None` (default) decodes per MSH-18.
    pub charset_override: Option<CharacterSet>,
    /// Optional hook called on each accepted TCP stream before any TLS
    /// handshake. Intended for applying socket options (e.g. TCP keep-alive).
    pub post_accept_hook: Option<PostAcceptHook>,
}

impl std::fmt::Debug for MllpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MllpConfig")
            .field("max_message_size", &self.max_message_size)
            .field("read_timeout", &self.read_timeout)
            .field("write_timeout", &self.write_timeout)
            .field("connect_timeout", &self.connect_timeout)
            .field("charset_override", &self.charset_override)
            .field(
                "post_accept_hook",
                &self.post_accept_hook.as_ref().map(|_| "<hook>"),
            )
            .finish()
    }
}

impl Default for MllpConfig {
    fn default() -> Self {
        Self {
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            read_timeout: DEFAULT_READ_TIMEOUT,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            charset_override: None,
            post_accept_hook: None,
        }
    }
}

impl MllpConfig {
    /// Create a new configuration with custom settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum message size
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Set the read timeout
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Set the write timeout
    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    /// Set the connection timeout
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Force inbound decoding to `charset`, ignoring MSH-18
    ///
    /// Use only for peers that misdeclare their encoding. Outbound
    /// messages are always encoded in their own MSH-18 declaration.
    pub fn with_charset_override(mut self, charset: CharacterSet) -> Self {
        self.charset_override = Some(charset);
        self
    }

    /// Install a post-accept hook invoked on each accepted TCP stream before
    /// any TLS handshake. Typical use: apply `SO_KEEPALIVE` and related socket
    /// options via `socket2::SockRef`.
    pub fn with_post_accept_hook(mut self, hook: PostAcceptHook) -> Self {
        self.post_accept_hook = Some(hook);
        self
    }
}

/// MLLP message framing
pub struct MllpFrame;

impl MllpFrame {
    /// Wrap an HL7 message in MLLP framing, encoded as UTF-8
    ///
    /// For any other character set, encode first (see
    /// [`wrap_with_charset`](Self::wrap_with_charset)) or frame
    /// pre-encoded bytes with [`wrap_bytes`](Self::wrap_bytes).
    pub fn wrap(message: &str) -> Vec<u8> {
        Self::wrap_bytes(message.as_bytes())
    }

    /// Wrap pre-encoded wire bytes in MLLP framing
    pub fn wrap_bytes(message: &[u8]) -> Vec<u8> {
        let mut framed = Vec::with_capacity(message.len() + 3);
        framed.push(START_OF_BLOCK);
        framed.extend_from_slice(message);
        framed.push(END_OF_BLOCK);
        framed.push(CARRIAGE_RETURN);
        framed
    }

    /// Wrap an HL7 message in MLLP framing, encoded in `charset`
    pub fn wrap_with_charset(message: &str, charset: CharacterSet) -> Result<Vec<u8>> {
        Ok(Self::wrap_bytes(&charset.encode(message)?))
    }

    /// Unwrap an MLLP frame to get the HL7 message
    ///
    /// The payload is decoded using the character set declared in MSH-18
    /// (HL7 table 0211), defaulting to UTF-8 when MSH-18 is empty.
    pub fn unwrap(framed: &[u8]) -> Result<String> {
        let message_bytes = Self::payload(framed)?;
        CharacterSet::decode_message(message_bytes)
    }

    /// Unwrap an MLLP frame, decoding the payload in `charset`
    ///
    /// Unlike [`unwrap`](Self::unwrap), the MSH-18 declaration is ignored.
    /// Use only for peers that misdeclare their encoding.
    pub fn unwrap_with_charset(framed: &[u8], charset: CharacterSet) -> Result<String> {
        let message_bytes = Self::payload(framed)?;
        charset.decode(message_bytes)
    }

    /// Validate framing and return the payload bytes
    fn payload(framed: &[u8]) -> Result<&[u8]> {
        if framed.len() < 3 {
            return Err(Error::Mllp("Frame too short".to_string()));
        }

        if framed[0] != START_OF_BLOCK {
            return Err(Error::Mllp("Missing start-of-block marker".to_string()));
        }

        let end_pos = framed.len() - 2;
        if framed[end_pos] != END_OF_BLOCK {
            return Err(Error::Mllp("Missing end-of-block marker".to_string()));
        }

        if framed[framed.len() - 1] != CARRIAGE_RETURN {
            return Err(Error::Mllp("Missing carriage return".to_string()));
        }

        Ok(&framed[1..end_pos])
    }
}

/// Unwrap a received frame, honoring a configured charset override
fn unwrap_received(buffer: &[u8], charset_override: Option<CharacterSet>) -> Result<String> {
    match charset_override {
        Some(charset) => MllpFrame::unwrap_with_charset(buffer, charset),
        None => MllpFrame::unwrap(buffer),
    }
}

/// Extract the first complete MLLP frame from `buf`, if one is present.
///
/// Leading bytes before the first `VT` are discarded as line noise; any
/// bytes after the frame's closing `FS CR` stay in `buf` for the next
/// call, so pipelined frames that arrive in a single TCP read are all
/// delivered instead of only the first. Decoding mirrors
/// [`codec::MllpCodec`]: the frame ends at the first `FS CR` pair and
/// frames (markers included) larger than `max_message_size` are rejected.
fn take_complete_frame(buf: &mut Vec<u8>, max_message_size: usize) -> Result<Option<Vec<u8>>> {
    let start = match buf.iter().position(|b| *b == START_OF_BLOCK) {
        Some(pos) => pos,
        None => {
            buf.clear();
            return Ok(None);
        }
    };
    if start > 0 {
        buf.drain(..start);
    }

    let mut idx = 1;
    while idx + 1 < buf.len() {
        if buf[idx] == END_OF_BLOCK && buf[idx + 1] == CARRIAGE_RETURN {
            let frame_len = idx + 2;
            if frame_len > max_message_size {
                return Err(Error::Mllp(format!(
                    "Message exceeds maximum size of {} bytes",
                    max_message_size
                )));
            }
            return Ok(Some(buf.drain(..frame_len).collect()));
        }
        idx += 1;
    }

    if buf.len() > max_message_size {
        return Err(Error::Mllp(format!(
            "Message exceeds maximum size of {} bytes",
            max_message_size
        )));
    }
    // Avoid re-scanning a huge partial frame one byte at a time.
    buf.reserve(8192);
    Ok(None)
}

/// MLLP client for sending messages
pub struct MllpClient {
    stream: MllpStream,
    max_message_size: usize,
    read_timeout: Duration,
    write_timeout: Duration,
    charset_override: Option<CharacterSet>,
    /// Bytes already read past the last delivered frame's `FS CR`.
    pending: Vec<u8>,
}

impl MllpClient {
    /// Connect to an MLLP server with default settings
    pub async fn connect(addr: &str) -> Result<Self> {
        Self::connect_with_config(addr, MllpConfig::default()).await
    }

    /// Connect to an MLLP server with custom configuration
    pub async fn connect_with_config(addr: &str, config: MllpConfig) -> Result<Self> {
        let tcp_stream = tokio::time::timeout(config.connect_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| {
                Error::Network(format!(
                    "Connection timeout after {:?}",
                    config.connect_timeout
                ))
            })?
            .map_err(|e| Error::Network(format!("Failed to connect: {}", e)))?;

        // Disable Nagle's algorithm for low-latency messaging
        tcp_stream
            .set_nodelay(true)
            .map_err(|e| Error::Network(format!("Failed to set TCP_NODELAY: {}", e)))?;

        Ok(Self {
            stream: MllpStream::Plain(tcp_stream),
            max_message_size: config.max_message_size,
            read_timeout: config.read_timeout,
            write_timeout: config.write_timeout,
            charset_override: config.charset_override,
            pending: Vec::new(),
        })
    }

    /// Connect to an MLLP server with TLS encryption
    ///
    /// # Arguments
    ///
    /// * `addr` - Server address (e.g., "localhost:2575")
    /// * `server_name` - Server name for SNI (e.g., "localhost" or "example.com")
    /// * `tls_config` - TLS configuration (basic TLS or mTLS)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # async fn example() -> vyna_hl7_core::error::Result<()> {
    /// use vyna_hl7_mllp::{MllpClient, tls::TlsClientConfig};
    ///
    /// // Basic TLS
    /// let tls_config = TlsClientConfig::new()?;
    /// let mut client = MllpClient::connect_tls(
    ///     "localhost:2575",
    ///     "localhost",
    ///     tls_config
    /// ).await?;
    ///
    /// // Or with mTLS
    /// let mtls_config = TlsClientConfig::with_mtls(
    ///     "ca-cert.pem",
    ///     "client-cert.pem",
    ///     "client-key.pem"
    /// )?;
    /// let mut client = MllpClient::connect_tls(
    ///     "localhost:2575",
    ///     "localhost",
    ///     mtls_config
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub async fn connect_tls(
        addr: &str,
        server_name: &str,
        tls_config: crate::tls::TlsClientConfig,
    ) -> Result<Self> {
        Self::connect_tls_with_config(addr, server_name, tls_config, MllpConfig::default()).await
    }

    /// Connect to an MLLP server with TLS encryption and custom configuration
    ///
    /// # Arguments
    ///
    /// * `addr` - Server address (e.g., "localhost:2575")
    /// * `server_name` - Server name for SNI (e.g., "localhost" or "example.com")
    /// * `tls_config` - TLS configuration (basic TLS or mTLS)
    /// * `config` - MLLP configuration (timeouts, buffer sizes)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # async fn example() -> vyna_hl7_core::error::Result<()> {
    /// use vyna_hl7_mllp::{MllpClient, MllpConfig, tls::TlsClientConfig};
    /// use std::time::Duration;
    ///
    /// let tls_config = TlsClientConfig::new()?;
    /// let mllp_config = MllpConfig::new()
    ///     .with_read_timeout(Duration::from_secs(60))
    ///     .with_write_timeout(Duration::from_secs(60));
    ///
    /// let mut client = MllpClient::connect_tls_with_config(
    ///     "localhost:2575",
    ///     "localhost",
    ///     tls_config,
    ///     mllp_config
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub async fn connect_tls_with_config(
        addr: &str,
        server_name: &str,
        tls_config: crate::tls::TlsClientConfig,
        config: MllpConfig,
    ) -> Result<Self> {
        // Establish TCP connection first
        let tcp_stream = tokio::time::timeout(config.connect_timeout, TcpStream::connect(addr))
            .await
            .map_err(|_| {
                Error::Network(format!(
                    "Connection timeout after {:?}",
                    config.connect_timeout
                ))
            })?
            .map_err(|e| Error::Network(format!("Failed to connect: {}", e)))?;

        // Disable Nagle's algorithm for low-latency messaging
        tcp_stream
            .set_nodelay(true)
            .map_err(|e| Error::Network(format!("Failed to set TCP_NODELAY: {}", e)))?;

        // Perform TLS handshake, bounded so a stalled peer cannot hang
        // the caller forever.
        let connector = TlsConnector::from(tls_config.config.clone());
        let server_name = ServerName::try_from(server_name.to_string())
            .map_err(|e| Error::Network(format!("Invalid server name: {}", e)))?;

        let timeout = config.connect_timeout;
        let tls_stream = tokio::time::timeout(timeout, connector.connect(server_name, tcp_stream))
            .await
            .map_err(|_| Error::Network(format!("TLS handshake timed out after {timeout:?}")))?
            .map_err(|e| Error::Network(format!("TLS handshake failed: {}", e)))?;

        Ok(Self {
            stream: MllpStream::TlsClient(tls_stream),
            max_message_size: config.max_message_size,
            read_timeout: config.read_timeout,
            write_timeout: config.write_timeout,
            charset_override: config.charset_override,
            pending: Vec::new(),
        })
    }

    /// Set the maximum message size
    pub fn set_max_message_size(&mut self, size: usize) {
        self.max_message_size = size;
    }

    /// Set the read timeout
    pub fn set_read_timeout(&mut self, timeout: Duration) {
        self.read_timeout = timeout;
    }

    /// Set the write timeout
    pub fn set_write_timeout(&mut self, timeout: Duration) {
        self.write_timeout = timeout;
    }

    /// Force inbound decoding to `charset`, ignoring MSH-18
    ///
    /// Pass `None` to decode per MSH-18 again.
    pub fn set_charset_override(&mut self, charset: Option<CharacterSet>) {
        self.charset_override = charset;
    }

    /// Test-only probe for the underlying TCP `TCP_NODELAY` state.
    ///
    /// Returns `None` when the state cannot be read.
    #[cfg(all(test, feature = "tls"))]
    fn tcp_nodelay(&self) -> Option<bool> {
        match &self.stream {
            MllpStream::Plain(stream) => stream.nodelay().ok(),
            #[cfg(feature = "tls")]
            MllpStream::TlsClient(stream) => stream.get_ref().0.nodelay().ok(),
            #[cfg(feature = "tls")]
            MllpStream::TlsServer(stream) => stream.get_ref().0.nodelay().ok(),
        }
    }

    /// Send a message and wait for acknowledgment
    ///
    /// The message is encoded in the character set declared in its MSH-18
    /// (HL7 table 0211), defaulting to UTF-8 when MSH-18 is empty.
    ///
    /// Fails without sending when a complete frame is already buffered:
    /// ACKs must arrive after the request, so a pre-send buffered frame
    /// is stale and would be misattributed to this send. Do not reuse
    /// the client after a mid-exchange read timeout either: a late ACK
    /// for the timed-out request may arrive after the retry. Open a new
    /// connection instead.
    pub async fn send_message(&mut self, message: &Message) -> Result<Message> {
        // Fail loudly on a pre-send buffered frame instead of silently
        // returning a stale pipelined response for this request.
        {
            let mut probe = self.pending.clone();
            match take_complete_frame(&mut probe, self.max_message_size) {
                Ok(Some(_)) => {
                    return Err(Error::Mllp(
                        "stale buffered frame before send; open a new connection".to_string(),
                    ));
                }
                Err(e) => return Err(e),
                Ok(None) => {}
            }
        }

        // Encode message in its declared character set
        let hl7_bytes = message.encode_bytes()?;

        // Wrap in MLLP frame
        let framed = MllpFrame::wrap_bytes(&hl7_bytes);

        // Enforce the size limit outbound too, mirroring the inbound
        // check (and MllpCodec::encode_payload), so oversize messages
        // fail before any bytes hit the wire.
        if framed.len() > self.max_message_size {
            return Err(Error::Mllp(format!(
                "Message exceeds maximum size of {} bytes",
                self.max_message_size
            )));
        }

        // Send with timeout
        tokio::time::timeout(self.write_timeout, async {
            self.stream.write_all(&framed).await?;
            self.stream.flush().await
        })
        .await
        .map_err(|_| Error::Network(format!("Write timeout after {:?}", self.write_timeout)))?
        .map_err(|e| Error::Network(format!("Failed to send: {}", e)))?;

        // Receive acknowledgment
        self.receive_message().await
    }

    /// Receive a message with timeout and size limit
    pub async fn receive_message(&mut self) -> Result<Message> {
        tokio::time::timeout(self.read_timeout, self.receive_message_internal())
            .await
            .map_err(|_| Error::Network(format!("Read timeout after {:?}", self.read_timeout)))?
    }

    /// Internal method to receive a message with buffer size protection
    ///
    /// Bytes left over from a previous read (pipelined frames sharing one
    /// TCP read) are drained from `pending` before touching the socket.
    async fn receive_message_internal(&mut self) -> Result<Message> {
        let mut chunk = [0u8; 4096]; // Read in larger chunks for efficiency

        loop {
            if let Some(frame) = take_complete_frame(&mut self.pending, self.max_message_size)? {
                let hl7_text = unwrap_received(&frame, self.charset_override)?;
                return parse_message(&hl7_text);
            }

            let n = self
                .stream
                .read(&mut chunk)
                .await
                .map_err(|e| Error::Network(format!("Failed to read: {}", e)))?;

            if n == 0 {
                // EOF with a partial frame buffered means the peer went
                // away mid-frame (mirrors MllpCodec::decode_eof); a clean
                // EOF with nothing pending keeps the generic error.
                if self.pending.is_empty() {
                    return Err(Error::Network("Connection closed".to_string()));
                }
                return Err(Error::Mllp(
                    "connection closed mid-frame (truncated frame)".to_string(),
                ));
            }

            self.pending.extend_from_slice(&chunk[..n]);
        }
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<()> {
        self.stream
            .shutdown()
            .await
            .map_err(|e| Error::Network(format!("Failed to close: {}", e)))
    }
}

/// MLLP server for receiving messages
pub struct MllpServer {
    listener: TcpListener,
    config: MllpConfig,
    #[cfg(feature = "tls")]
    tls_acceptor: Option<TlsAcceptor>,
}

impl MllpServer {
    /// Bind to an address with default configuration
    pub async fn bind(addr: &str) -> Result<Self> {
        Self::bind_with_config(addr, MllpConfig::default()).await
    }

    /// Bind to an address with custom configuration
    pub async fn bind_with_config(addr: &str, config: MllpConfig) -> Result<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Network(format!("Failed to bind: {}", e)))?;

        Ok(Self {
            listener,
            config,
            #[cfg(feature = "tls")]
            tls_acceptor: None,
        })
    }

    /// Bind to an address with TLS encryption
    ///
    /// # Arguments
    ///
    /// * `addr` - Server address (e.g., "0.0.0.0:2575")
    /// * `tls_config` - TLS configuration (basic TLS or mTLS)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # async fn example() -> vyna_hl7_core::error::Result<()> {
    /// use vyna_hl7_mllp::{MllpServer, tls::TlsServerConfig};
    ///
    /// // Basic TLS
    /// let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    /// let server = MllpServer::bind_tls("0.0.0.0:2575", tls_config).await?;
    ///
    /// // Or with mTLS (client certificate verification)
    /// let mtls_config = TlsServerConfig::with_mtls(
    ///     "server-cert.pem",
    ///     "server-key.pem",
    ///     "ca-cert.pem"
    /// )?;
    /// let server = MllpServer::bind_tls("0.0.0.0:2575", mtls_config).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub async fn bind_tls(addr: &str, tls_config: crate::tls::TlsServerConfig) -> Result<Self> {
        Self::bind_tls_with_config(addr, tls_config, MllpConfig::default()).await
    }

    /// Bind to an address with TLS encryption and custom configuration
    ///
    /// # Arguments
    ///
    /// * `addr` - Server address (e.g., "0.0.0.0:2575")
    /// * `tls_config` - TLS configuration (basic TLS or mTLS)
    /// * `config` - MLLP configuration (timeouts, buffer sizes)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # async fn example() -> vyna_hl7_core::error::Result<()> {
    /// use vyna_hl7_mllp::{MllpServer, MllpConfig, tls::TlsServerConfig};
    /// use std::time::Duration;
    ///
    /// let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    /// let mllp_config = MllpConfig::new()
    ///     .with_read_timeout(Duration::from_secs(60))
    ///     .with_write_timeout(Duration::from_secs(60));
    ///
    /// let server = MllpServer::bind_tls_with_config(
    ///     "0.0.0.0:2575",
    ///     tls_config,
    ///     mllp_config
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub async fn bind_tls_with_config(
        addr: &str,
        tls_config: crate::tls::TlsServerConfig,
        config: MllpConfig,
    ) -> Result<Self> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Network(format!("Failed to bind: {}", e)))?;

        let tls_acceptor = TlsAcceptor::from(tls_config.config.clone());

        Ok(Self {
            listener,
            config,
            tls_acceptor: Some(tls_acceptor),
        })
    }

    /// Accept a connection and return an MllpConnection with the server's configuration
    ///
    /// If the server was created with `bind_tls()` or `bind_tls_with_config()`, this will
    /// perform TLS handshake on the accepted connection. If a `post_accept_hook` is
    /// installed on the config, it is called before the handshake so the caller can
    /// apply socket options (e.g. TCP keep-alive) on the raw `TcpStream`.
    pub async fn accept(&self) -> Result<MllpConnection> {
        let (tcp_stream, _addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| Error::Network(format!("Failed to accept: {}", e)))?;

        // Disable Nagle's algorithm for low-latency messaging
        tcp_stream
            .set_nodelay(true)
            .map_err(|e| Error::Network(format!("Failed to set TCP_NODELAY: {}", e)))?;

        if let Some(hook) = self.config.post_accept_hook.as_ref() {
            hook(&tcp_stream)?;
        }

        #[cfg(feature = "tls")]
        let stream = if let Some(ref acceptor) = self.tls_acceptor {
            // Bound the handshake so one stalled peer fails fast instead
            // of wedging the accept loop.
            let timeout = self.config.connect_timeout;
            let tls_stream = tokio::time::timeout(timeout, acceptor.accept(tcp_stream))
                .await
                .map_err(|_| Error::Network(format!("TLS handshake timed out after {timeout:?}")))?
                .map_err(|e| Error::Network(format!("TLS handshake failed: {}", e)))?;
            MllpStream::TlsServer(tls_stream)
        } else {
            MllpStream::Plain(tcp_stream)
        };

        #[cfg(not(feature = "tls"))]
        let stream = MllpStream::Plain(tcp_stream);

        Ok(MllpConnection {
            stream,
            max_message_size: self.config.max_message_size,
            read_timeout: self.config.read_timeout,
            write_timeout: self.config.write_timeout,
            charset_override: self.config.charset_override,
            pending: Vec::new(),
        })
    }

    /// Get the local address
    pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
        self.listener
            .local_addr()
            .map_err(|e| Error::Network(format!("Failed to get local addr: {}", e)))
    }
}

/// An MLLP connection
pub struct MllpConnection {
    stream: MllpStream,
    max_message_size: usize,
    read_timeout: Duration,
    write_timeout: Duration,
    charset_override: Option<CharacterSet>,
    /// Bytes already read past the last delivered frame's `FS CR`.
    pending: Vec<u8>,
}

impl MllpConnection {
    /// Receive a message with timeout and size limit
    pub async fn receive_message(&mut self) -> Result<Message> {
        tokio::time::timeout(self.read_timeout, self.receive_message_internal())
            .await
            .map_err(|_| Error::Network(format!("Read timeout after {:?}", self.read_timeout)))?
    }

    /// Internal method to receive a message with buffer size protection
    ///
    /// Bytes left over from a previous read (pipelined frames sharing one
    /// TCP read) are drained from `pending` before touching the socket.
    async fn receive_message_internal(&mut self) -> Result<Message> {
        let mut chunk = [0u8; 4096]; // Read in larger chunks for efficiency

        loop {
            if let Some(frame) = take_complete_frame(&mut self.pending, self.max_message_size)? {
                let hl7_text = unwrap_received(&frame, self.charset_override)?;
                return parse_message(&hl7_text);
            }

            let n = self
                .stream
                .read(&mut chunk)
                .await
                .map_err(|e| Error::Network(format!("Failed to read: {}", e)))?;

            if n == 0 {
                // EOF with a partial frame buffered means the peer went
                // away mid-frame (mirrors MllpCodec::decode_eof); a clean
                // EOF with nothing pending keeps the generic error.
                if self.pending.is_empty() {
                    return Err(Error::Network("Connection closed".to_string()));
                }
                return Err(Error::Mllp(
                    "connection closed mid-frame (truncated frame)".to_string(),
                ));
            }

            self.pending.extend_from_slice(&chunk[..n]);
        }
    }

    /// Send a message with timeout
    ///
    /// The message is encoded in the character set declared in its MSH-18
    /// (HL7 table 0211), defaulting to UTF-8 when MSH-18 is empty.
    pub async fn send_message(&mut self, message: &Message) -> Result<()> {
        let hl7_bytes = message.encode_bytes()?;
        let framed = MllpFrame::wrap_bytes(&hl7_bytes);

        // Enforce the size limit outbound too, mirroring the inbound
        // check (and MllpCodec::encode_payload), so oversize messages
        // fail before any bytes hit the wire.
        if framed.len() > self.max_message_size {
            return Err(Error::Mllp(format!(
                "Message exceeds maximum size of {} bytes",
                self.max_message_size
            )));
        }

        tokio::time::timeout(self.write_timeout, async {
            self.stream.write_all(&framed).await?;
            self.stream.flush().await
        })
        .await
        .map_err(|_| Error::Network(format!("Write timeout after {:?}", self.write_timeout)))?
        .map_err(|e| Error::Network(format!("Failed to send: {}", e)))
    }

    /// Close the connection
    pub async fn close(mut self) -> Result<()> {
        self.stream
            .shutdown()
            .await
            .map_err(|e| Error::Network(format!("Failed to close: {}", e)))
    }

    /// Split the connection into independent read and write halves for
    /// full-duplex use.
    ///
    /// The read half can block in [`MllpReadHalf::receive_message`] while the
    /// write half concurrently pushes messages (e.g. deferred Application
    /// ACKs) through [`MllpWriteHalf::send_message`]. Both halves inherit the
    /// connection's timeouts, maximum message size and charset override, and
    /// bytes already buffered past the last delivered frame move into the
    /// read half.
    pub fn into_split(self) -> (MllpReadHalf, MllpWriteHalf) {
        let (reader, writer) = tokio::io::split(self.stream);
        (
            MllpReadHalf {
                reader,
                max_message_size: self.max_message_size,
                read_timeout: self.read_timeout,
                charset_override: self.charset_override,
                buffer: self.pending,
            },
            MllpWriteHalf {
                writer,
                write_timeout: self.write_timeout,
                max_message_size: self.max_message_size,
            },
        )
    }
}

/// Read half of a split [`MllpConnection`] (see [`MllpConnection::into_split`]).
///
/// Unlike the request-response [`MllpConnection`], the read half keeps a
/// persistent receive buffer: bytes read from the socket but not yet consumed
/// as a complete MLLP frame (a pipelined next frame, or a partially delivered
/// frame) are retained across `receive_message*` calls.
pub struct MllpReadHalf {
    reader: ReadHalf<MllpStream>,
    max_message_size: usize,
    read_timeout: Duration,
    charset_override: Option<CharacterSet>,
    /// Bytes received but not yet consumed as a complete frame. Survives
    /// across calls so pipelined frames (two frames in one read) and
    /// timed-out mid-frame receives are never lost.
    buffer: Vec<u8>,
}

impl MllpReadHalf {
    /// Receive a message with timeout and size limit.
    ///
    /// Same MLLP framing rules as [`MllpConnection::receive_message`]
    /// (0x0B start scan, size cap, 0x1C 0x0D end detection), but backed by
    /// the half's persistent buffer, so back-to-back frames delivered in a
    /// single read are all received by consecutive calls.
    ///
    /// # Cancel safety
    ///
    /// This method is cancel-safe: if it is dropped (e.g. it times out, or
    /// loses a `select!`) while a frame is partially read, the bytes already
    /// received remain in the internal buffer and the next call resumes
    /// frame reassembly exactly where it left off — no bytes are lost and
    /// the stream does not desynchronize.
    pub async fn receive_message(&mut self) -> Result<Message> {
        match self.receive_message_opt().await? {
            Some(message) => Ok(message),
            None => Err(Error::Network(format!(
                "Read timeout after {:?}",
                self.read_timeout
            ))),
        }
    }

    /// Receive a message, treating the read timeout as idle rather than
    /// as an error.
    ///
    /// Returns `Ok(Some(message))` when a message arrives, `Ok(None)` when
    /// the read timeout elapses with no complete message (idle), and `Err`
    /// on real failures (connection closed, I/O error, oversized or invalid
    /// frame). This lets a duplex reader loop keep looping on idle without
    /// string-matching error messages.
    ///
    /// # Cancel safety
    ///
    /// Cancel-safe with the same guarantee as [`Self::receive_message`]:
    /// an idle `Ok(None)` return (or an outright drop) mid-frame leaves the
    /// partially received bytes buffered for the next call.
    pub async fn receive_message_opt(&mut self) -> Result<Option<Message>> {
        match tokio::time::timeout(self.read_timeout, self.receive_buffered()).await {
            Ok(result) => result.map(Some),
            Err(_elapsed) => Ok(None),
        }
    }

    /// Buffered frame reassembly: drain a complete frame from the internal
    /// buffer if one is present, otherwise read more bytes from the socket.
    ///
    /// The only await point is the socket read, and bytes are appended to
    /// `self.buffer` synchronously after it completes — which is what makes
    /// the public receive methods cancel-safe.
    async fn receive_buffered(&mut self) -> Result<Message> {
        loop {
            if let Some(frame) = take_complete_frame(&mut self.buffer, self.max_message_size)? {
                let hl7_text = unwrap_received(&frame, self.charset_override)?;
                return parse_message(&hl7_text);
            }

            let mut chunk = [0u8; 4096];
            let n = self
                .reader
                .read(&mut chunk)
                .await
                .map_err(|e| Error::Network(format!("Failed to read: {}", e)))?;

            if n == 0 {
                if self.buffer.is_empty() {
                    return Err(Error::Network("Connection closed".to_string()));
                }
                return Err(Error::Mllp(
                    "connection closed mid-frame (truncated frame)".to_string(),
                ));
            }

            self.buffer.extend_from_slice(&chunk[..n]);
        }
    }
}

/// Write half of a split [`MllpConnection`] (see [`MllpConnection::into_split`]).
pub struct MllpWriteHalf {
    writer: WriteHalf<MllpStream>,
    write_timeout: Duration,
    max_message_size: usize,
}

impl MllpWriteHalf {
    /// Send a message with timeout.
    ///
    /// Mirrors [`MllpConnection::send_message`]: encode in the message's
    /// MSH-18 character set, wrap in MLLP framing, enforce the size limit,
    /// write + flush under the write timeout.
    pub async fn send_message(&mut self, message: &Message) -> Result<()> {
        let hl7_bytes = message.encode_bytes()?;
        let framed = MllpFrame::wrap_bytes(&hl7_bytes);

        if framed.len() > self.max_message_size {
            return Err(Error::Mllp(format!(
                "Message exceeds maximum size of {} bytes",
                self.max_message_size
            )));
        }

        tokio::time::timeout(self.write_timeout, async {
            self.writer.write_all(&framed).await?;
            self.writer.flush().await
        })
        .await
        .map_err(|_| Error::Network(format!("Write timeout after {:?}", self.write_timeout)))?
        .map_err(|e| Error::Network(format!("Failed to send: {}", e)))
    }

    /// Shut down the write side of the connection.
    pub async fn close(mut self) -> Result<()> {
        self.writer
            .shutdown()
            .await
            .map_err(|e| Error::Network(format!("Failed to close: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_frame() {
        let message = "MSH|^~\\&|TEST";
        let framed = MllpFrame::wrap(message);

        assert_eq!(framed[0], START_OF_BLOCK);
        assert_eq!(framed[framed.len() - 2], END_OF_BLOCK);
        assert_eq!(framed[framed.len() - 1], CARRIAGE_RETURN);
    }

    #[test]
    fn test_unwrap_frame() {
        let message = "MSH|^~\\&|TEST";
        let framed = MllpFrame::wrap(message);
        let unwrapped = MllpFrame::unwrap(&framed).unwrap();

        assert_eq!(unwrapped, message);
    }

    #[test]
    fn test_unwrap_invalid_frame() {
        let invalid = vec![0x00, 0x01, 0x02];
        assert!(MllpFrame::unwrap(&invalid).is_err());
    }

    #[test]
    fn test_unwrap_missing_start() {
        let mut framed = MllpFrame::wrap("TEST");
        framed[0] = 0x00; // Corrupt start marker
        assert!(MllpFrame::unwrap(&framed).is_err());
    }

    #[test]
    fn test_unwrap_missing_end() {
        let mut framed = MllpFrame::wrap("TEST");
        let len = framed.len();
        framed[len - 2] = 0x00; // Corrupt end marker
        assert!(MllpFrame::unwrap(&framed).is_err());
    }

    #[test]
    fn test_wrap_unwrap_with_charset_round_trip() {
        let hl7 = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1||12345^^^FAC^MR||Müller||19800101|M\r";
        let framed = MllpFrame::wrap_with_charset(hl7, CharacterSet::Iso8859(1)).unwrap();

        // Latin-1 0xFC on the wire, not the UTF-8 sequence 0xC3 0xBC.
        assert!(framed.contains(&0xFC));
        assert!(!framed.windows(2).any(|w| w == [0xC3, 0xBC]));
        assert_eq!(MllpFrame::unwrap(&framed).unwrap(), hl7);
    }

    #[test]
    fn test_unwrap_decodes_declared_charset() {
        // Latin-1 framed bytes, which previously failed as invalid UTF-8.
        let mut payload =
            b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1||12345^^^FAC^MR||M"
                .to_vec();
        payload.push(0xFC);
        payload.extend_from_slice(b"ller||19800101|M\r");
        let framed = MllpFrame::wrap_bytes(&payload);

        let text = MllpFrame::unwrap(&framed).unwrap();
        assert!(text.contains("Müller"));
    }

    #[tokio::test]
    async fn test_send_receive_latin1_message() {
        let hl7 = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1||12345^^^FAC^MR||Müller^Jörg||19800101|M\r";
        let message = parse_message(hl7).unwrap();

        let server = MllpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            let received = conn.receive_message().await.unwrap();
            let ack = parse_message(
                "MSH|^~\\&|RCV|RFAC|SND|FAC|20240115120100||ACK^A01^ACK|ACK1|P|2.5.1\rMSA|AA|1\r",
            )
            .unwrap();
            conn.send_message(&ack).await.unwrap();
            received
        });

        let mut client = MllpClient::connect(&addr.to_string()).await.unwrap();
        let ack = client.send_message(&message).await.unwrap();
        assert!(ack.is_acknowledgment());

        let received = server_task.await.unwrap();
        let pid = received.get_segment_by_id("PID").unwrap();
        let name = pid.get_field(5).unwrap().get_repetition(0).unwrap();
        assert_eq!(name.get_component(0).unwrap().value(), Some("Müller"));
        assert_eq!(name.get_component(1).unwrap().value(), Some("Jörg"));
    }

    const PIPELINED_MSG1: &str = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|PIPE1|P|2.5\r";
    const PIPELINED_MSG2: &str = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|PIPE2|P|2.5\r";

    /// Two frames in one TCP read: the server must deliver both, not drop
    /// the bytes past the first frame's `FS CR`.
    #[tokio::test]
    async fn test_connection_delivers_pipelined_frames() {
        let config = MllpConfig::new().with_read_timeout(Duration::from_secs(5));
        let server = MllpServer::bind_with_config("127.0.0.1:0", config)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            let first = conn.receive_message().await.unwrap();
            let second = conn.receive_message().await.unwrap();
            assert_eq!(first.get_control_id(), Some("PIPE1"));
            assert_eq!(second.get_control_id(), Some("PIPE2"));
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut both = MllpFrame::wrap(PIPELINED_MSG1);
        both.extend_from_slice(&MllpFrame::wrap(PIPELINED_MSG2));
        stream.write_all(&both).await.unwrap();

        server_task.await.unwrap();
    }

    /// Two frames in one TCP read: the client must deliver both, not drop
    /// the bytes past the first frame's `FS CR`.
    #[tokio::test]
    async fn test_client_delivers_pipelined_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut both = MllpFrame::wrap(PIPELINED_MSG1);
            both.extend_from_slice(&MllpFrame::wrap(PIPELINED_MSG2));
            socket.write_all(&both).await.unwrap();
        });

        let config = MllpConfig::new().with_read_timeout(Duration::from_secs(5));
        let mut client = MllpClient::connect_with_config(&addr.to_string(), config)
            .await
            .unwrap();
        let first = client.receive_message().await.unwrap();
        let second = client.receive_message().await.unwrap();
        assert_eq!(first.get_control_id(), Some("PIPE1"));
        assert_eq!(second.get_control_id(), Some("PIPE2"));

        server_task.await.unwrap();
    }

    #[test]
    fn test_charset_override_defaults_to_none() {
        assert!(MllpConfig::default().charset_override.is_none());
        let config = MllpConfig::new().with_charset_override(CharacterSet::Iso8859(1));
        assert_eq!(config.charset_override, Some(CharacterSet::Iso8859(1)));
    }

    #[test]
    fn test_unwrap_with_charset_ignores_declaration() {
        // Latin-1 bytes that wrongly declare ASCII.
        let mut payload =
            b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||ASCII\rPID|1||12345^^^FAC^MR||M"
                .to_vec();
        payload.push(0xFC);
        payload.extend_from_slice(b"ller||19800101|M\r");
        let framed = MllpFrame::wrap_bytes(&payload);

        assert!(MllpFrame::unwrap(&framed).is_err());
        let text = MllpFrame::unwrap_with_charset(&framed, CharacterSet::Iso8859(1)).unwrap();
        assert!(text.contains("Müller"));
    }

    #[tokio::test]
    async fn test_server_charset_override_decodes_misdeclared_peer() {
        // Latin-1 bytes that wrongly declare ASCII.
        let mut payload =
            b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||ASCII\rPID|1||12345^^^FAC^MR||M"
                .to_vec();
        payload.push(0xFC);
        payload.extend_from_slice(b"ller||19800101|M\r");
        let framed = MllpFrame::wrap_bytes(&payload);

        let config = MllpConfig::new().with_charset_override(CharacterSet::Iso8859(1));
        let server = MllpServer::bind_with_config("127.0.0.1:0", config)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            conn.receive_message().await.unwrap()
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream.write_all(&framed).await.unwrap();

        let received = server_task.await.unwrap();
        let pid = received.get_segment_by_id("PID").unwrap();
        assert_eq!(pid.get_field_value(5), Some("Müller"));
    }

    /// Outbound size limit (client): a message over `max_message_size`
    /// must fail before any bytes hit the wire, not only when the echo
    /// trips the inbound limit.
    #[tokio::test]
    async fn test_client_send_rejects_oversize_before_transmit() {
        let server = MllpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            let res =
                tokio::time::timeout(Duration::from_millis(500), conn.receive_message()).await;
            assert!(res.is_err(), "oversize message was transmitted: {res:?}");
        });

        let config = MllpConfig::new().with_max_message_size(10);
        let mut client = MllpClient::connect_with_config(&addr.to_string(), config)
            .await
            .unwrap();
        let msg = parse_message(PIPELINED_MSG1).unwrap();
        assert!(MllpFrame::wrap(&msg.encode()).len() > 10);
        let err = client.send_message(&msg).await.unwrap_err();
        assert!(
            err.to_string().contains("exceeds maximum size"),
            "unexpected: {err}"
        );

        server_task.await.unwrap();
    }

    /// Outbound size limit (connection): same guarantee server-side.
    #[tokio::test]
    async fn test_connection_send_rejects_oversize_before_transmit() {
        use tokio::io::AsyncReadExt;

        let config = MllpConfig::new().with_max_message_size(10);
        let server = MllpServer::bind_with_config("127.0.0.1:0", config)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();
        let server_task = tokio::spawn(async move {
            let mut conn = server.accept().await.unwrap();
            let msg = parse_message(PIPELINED_MSG1).unwrap();
            let err = conn.send_message(&msg).await.unwrap_err();
            assert!(
                err.to_string().contains("exceeds maximum size"),
                "unexpected: {err}"
            );
        });

        // The server must not transmit anything: either no bytes arrive
        // or the connection closes without data.
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 64];
        let res = tokio::time::timeout(Duration::from_millis(500), stream.read(&mut buf)).await;
        match res {
            Err(_) | Ok(Ok(0)) => {}
            other => panic!("oversize message was transmitted: {other:?}"),
        }

        server_task.await.unwrap();
    }

    /// EOF after a partial frame (client): must report a truncated-frame
    /// error, not the generic "Connection closed".
    #[tokio::test]
    async fn test_client_receive_partial_frame_eof_is_truncated() {
        async fn receive_after(peer_writes: Option<Vec<u8>>) -> Result<Message> {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let server_task = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                if let Some(bytes) = peer_writes {
                    use tokio::io::AsyncWriteExt;
                    socket.write_all(&bytes).await.unwrap();
                }
                socket.shutdown().await.unwrap();
            });
            let mut client = MllpClient::connect_with_config(
                &addr.to_string(),
                MllpConfig::new().with_read_timeout(Duration::from_secs(5)),
            )
            .await
            .unwrap();
            let res = client.receive_message().await;
            server_task.await.unwrap();
            res
        }

        // Partial frame then EOF: truncation-specific Mllp error.
        let partial = b"\x0BMSH|partial-no-terminator".to_vec();
        let err = receive_after(Some(partial)).await.unwrap_err();
        assert!(matches!(err, Error::Mllp(_)), "unexpected variant: {err:?}");
        assert!(err.to_string().contains("mid-frame"), "unexpected: {err}");

        // Clean EOF with nothing pending: still the generic error.
        let err = receive_after(None).await.unwrap_err();
        assert!(
            matches!(err, Error::Network(_)),
            "unexpected variant: {err:?}"
        );
        assert!(
            err.to_string().contains("Connection closed"),
            "unexpected: {err}"
        );
    }

    /// EOF after a partial frame (connection): same guarantee server-side.
    #[tokio::test]
    async fn test_connection_receive_partial_frame_eof_is_truncated() {
        async fn receive_after(peer_writes: Option<Vec<u8>>) -> Result<Message> {
            let server = MllpServer::bind_with_config(
                "127.0.0.1:0",
                MllpConfig::new().with_read_timeout(Duration::from_secs(5)),
            )
            .await
            .unwrap();
            let addr = server.local_addr().unwrap();
            let server_task = tokio::spawn(async move {
                let mut conn = server.accept().await.unwrap();
                conn.receive_message().await
            });
            let mut stream = TcpStream::connect(addr).await.unwrap();
            if let Some(bytes) = peer_writes {
                use tokio::io::AsyncWriteExt;
                stream.write_all(&bytes).await.unwrap();
            }
            stream.shutdown().await.unwrap();
            server_task.await.unwrap()
        }

        // Partial frame then EOF: truncation-specific Mllp error.
        let partial = b"\x0BMSH|partial-no-terminator".to_vec();
        let err = receive_after(Some(partial)).await.unwrap_err();
        assert!(matches!(err, Error::Mllp(_)), "unexpected variant: {err:?}");
        assert!(err.to_string().contains("mid-frame"), "unexpected: {err}");

        // Clean EOF with nothing pending: still the generic error.
        let err = receive_after(None).await.unwrap_err();
        assert!(
            matches!(err, Error::Network(_)),
            "unexpected variant: {err:?}"
        );
        assert!(
            err.to_string().contains("Connection closed"),
            "unexpected: {err}"
        );
    }

    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn tls_connect_handshake_times_out() {
        // Blackhole: accept TCP, then never speak TLS.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (_socket, _peer) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(60)).await;
        });

        let tls_config = crate::tls::TlsClientConfig::new().unwrap();
        let config = MllpConfig::new().with_connect_timeout(Duration::from_millis(200));
        let addr = addr.to_string();
        let connect = MllpClient::connect_tls_with_config(&addr, "localhost", tls_config, config);
        // The outer timeout only bounds a wedged (pre-fix) handshake.
        let result = tokio::time::timeout(Duration::from_secs(15), connect)
            .await
            .expect("connect_tls() wedged on a stalled TLS handshake");
        match result {
            Ok(_) => panic!("expected a handshake timeout, got a connection"),
            Err(err) => assert!(err.to_string().contains("timed out"), "unexpected: {err}"),
        }
    }

    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn tls_accept_handshake_times_out() {
        use std::process::Command;

        // Self-signed server cert (never verified here: the stalled peer
        // sends nothing, so the server only needs something loadable).
        let dir = std::env::temp_dir().join(format!(
            "vyna_mllp_probe_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let cert = dir.join("srv-cert.pem").to_str().unwrap().to_string();
        let key = dir.join("srv-key.pem").to_str().unwrap().to_string();
        let status = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                &key,
                "-out",
                &cert,
                "-days",
                "2",
                "-subj",
                "/CN=localhost",
            ])
            .status()
            .expect("openssl must be installed for TLS tests");
        assert!(status.success(), "openssl req -x509 failed");

        let tls_config = crate::tls::TlsServerConfig::new(&cert, &key).unwrap();
        let config = MllpConfig::new().with_connect_timeout(Duration::from_millis(200));
        let server = MllpServer::bind_tls_with_config("127.0.0.1:0", tls_config, config)
            .await
            .unwrap();
        let addr = server.local_addr().unwrap();

        // Stalled peer: connect, then never speak TLS. The socket must stay
        // open (an EOF would fail the handshake immediately instead).
        let _stall = TcpStream::connect(addr).await.unwrap();
        // The outer timeout only bounds a wedged (pre-fix) handshake; the
        // accept loop must stay usable for the next connection instead.
        let result = tokio::time::timeout(Duration::from_secs(15), server.accept())
            .await
            .expect("accept() wedged on a stalled TLS handshake");
        match result {
            Ok(_) => panic!("expected a handshake timeout, got a connection"),
            Err(err) => assert!(err.to_string().contains("timed out"), "unexpected: {err}"),
        }

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Generate a CA plus a server certificate signed by it (requires
    /// openssl). Returns `(ca_cert, server_cert, server_key)`.
    #[cfg(feature = "tls")]
    fn generate_tls_test_certs(dir: &std::path::Path) -> (String, String, String) {
        use std::process::Command;

        let path = |name: &str| dir.join(name).to_str().unwrap().to_string();
        let (ca_key, ca_cert) = (path("ca-key.pem"), path("ca-cert.pem"));
        let (srv_key, srv_csr, srv_cert, srv_ext) = (
            path("srv-key.pem"),
            path("srv.csr"),
            path("srv-cert.pem"),
            path("srv-ext.cnf"),
        );
        std::fs::write(
            &srv_ext,
            "[v3_req]\nbasicConstraints = CA:FALSE\n\
             keyUsage = digitalSignature, keyEncipherment\n\
             extendedKeyUsage = serverAuth\n\
             subjectAltName = DNS:localhost,IP:127.0.0.1\n",
        )
        .unwrap();
        let run = |args: &[&str]| {
            let status = Command::new("openssl")
                .args(args)
                .status()
                .expect("openssl must be installed for TLS tests");
            assert!(status.success(), "openssl {args:?} failed");
        };
        run(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            &ca_key,
            "-out",
            &ca_cert,
            "-days",
            "2",
            "-subj",
            "/CN=Test CA",
        ]);
        run(&[
            "req",
            "-new",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            &srv_key,
            "-out",
            &srv_csr,
            "-subj",
            "/CN=localhost",
        ]);
        run(&[
            "x509",
            "-req",
            "-in",
            &srv_csr,
            "-CA",
            &ca_cert,
            "-CAkey",
            &ca_key,
            "-CAcreateserial",
            "-out",
            &srv_cert,
            "-days",
            "2",
            "-extfile",
            &srv_ext,
            "-extensions",
            "v3_req",
        ]);
        (ca_cert, srv_cert, srv_key)
    }

    /// The TLS connect path must disable Nagle like the plain path, so
    /// small request/ACK turns do not stall on delayed-ACK interaction.
    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn tls_client_enables_tcp_nodelay() {
        let dir = std::env::temp_dir().join(format!(
            "vyna_mllp_nodelay_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (ca_cert, srv_cert, srv_key) = generate_tls_test_certs(&dir);

        let server_tls = crate::tls::TlsServerConfig::new(&srv_cert, &srv_key).unwrap();
        let server = MllpServer::bind_tls_with_config("127.0.0.1:0", server_tls, MllpConfig::new())
            .await
            .unwrap();
        let addr = server.local_addr().unwrap().to_string();
        let accept = tokio::spawn(async move { server.accept().await });

        let client_tls = crate::tls::TlsClientConfig::with_ca_cert(&ca_cert).unwrap();
        let client =
            MllpClient::connect_tls_with_config(&addr, "localhost", client_tls, MllpConfig::new())
                .await
                .unwrap();
        accept.await.unwrap().unwrap();

        assert_eq!(
            client.tcp_nodelay(),
            Some(true),
            "TLS connect left Nagle enabled"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Pipelined extra ACK for request 1 must not be silently returned
    /// as the ACK for request 2: the second send fails loudly instead.
    #[tokio::test]
    async fn test_send_rejects_stale_buffered_frame() {
        async fn read_frame(socket: &mut TcpStream) -> Vec<u8> {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0, "peer closed before sending frame");
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.iter().position(|b| *b == 0x0B) {
                    buf.drain(..pos);
                    break;
                }
            }
            loop {
                for i in 0..buf.len().saturating_sub(1) {
                    if buf[i] == 0x1C && buf[i + 1] == 0x0D {
                        return buf.drain(..i + 2).collect();
                    }
                }
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0, "peer closed mid-frame");
                buf.extend_from_slice(&chunk[..n]);
            }
        }

        let ack1 = "MSH|^~\\&|R|F|S|F|20240115120100||ACK^A01^ACK|ACK1|P|2.5\rMSA|AA|REQ1\r";
        let stale = "MSH|^~\\&|R|F|S|F|20240115120100||ACK^A01^ACK|ACK-STALE|P|2.5\rMSA|AA|STALE\r";
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let server_task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _req1 = read_frame(&mut socket).await;
            let mut both = MllpFrame::wrap(ack1);
            both.extend_from_slice(&MllpFrame::wrap(stale));
            socket.write_all(&both).await.unwrap();
            // Client must refuse to send request 2 once the stale frame
            // is buffered, so this read must time out.
            let second =
                tokio::time::timeout(Duration::from_millis(500), read_frame(&mut socket)).await;
            assert!(second.is_err(), "stale frame did not block send #2");
        });

        let mut client = MllpClient::connect(&addr).await.unwrap();
        let req1 = parse_message("MSH|^~\\&|S|F|R|F|20240115||||ADT^A01|REQ1|P|2.5\r").unwrap();
        let req2 = parse_message("MSH|^~\\&|S|F|R|F|20240115||||ADT^A01|REQ2|P|2.5\r").unwrap();
        let first = client.send_message(&req1).await.unwrap();
        assert_eq!(first.get_control_id(), Some("ACK1"));
        let err = client.send_message(&req2).await.unwrap_err();
        assert!(
            err.to_string().contains("stale buffered"),
            "unexpected: {err}"
        );
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn test_post_accept_hook_runs_on_accepted_stream() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = Arc::new(AtomicUsize::new(0));
        let calls_clone = calls.clone();

        let hook: PostAcceptHook = Arc::new(move |stream: &TcpStream| {
            // Verify we can read the peer address from the raw stream, then
            // bump a counter so the test can observe the hook ran.
            stream
                .peer_addr()
                .map_err(|e| Error::Network(format!("peer_addr failed: {e}")))?;
            calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let config = MllpConfig::default().with_post_accept_hook(hook);
        let server = MllpServer::bind_with_config("127.0.0.1:0", config)
            .await
            .expect("bind should succeed");
        let addr = server.local_addr().expect("local_addr");

        let accept_task = tokio::spawn(async move {
            let _ = server.accept().await;
        });

        let _client = TcpStream::connect(addr).await.expect("client connect");
        // Give the accept task a chance to run the hook.
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);

        accept_task.abort();
        let _ = accept_task.await;
    }

    /// Split halves duplex independently: the read half delivers two
    /// pipelined frames from one TCP write, and the write half sends an
    /// ACK the peer reads back.
    #[tokio::test]
    async fn test_into_split_duplex_with_pipelined_frames() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let server = MllpServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();

        let server_task = tokio::spawn(async move {
            let conn = server.accept().await.unwrap();
            let (mut read_half, mut write_half) = conn.into_split();
            let first = read_half.receive_message().await.unwrap();
            let second = read_half.receive_message().await.unwrap();
            let ack = parse_message("MSH|^~\\&|R|F|S|F|20240115||ACK^A01^ACK|A1|P|2.5\rMSA|AA|1\r")
                .unwrap();
            write_half.send_message(&ack).await.unwrap();
            (first, second)
        });

        let mut sock = TcpStream::connect(addr).await.unwrap();
        let f1 = MllpFrame::wrap("MSH|^~\\&|S|F|R|F|20240115||ADT^A01|M1|P|2.5\r");
        let f2 = MllpFrame::wrap("MSH|^~\\&|S|F|R|F|20240115||ADT^A01|M2|P|2.5\r");
        let mut both = f1;
        both.extend_from_slice(&f2);
        sock.write_all(&both).await.unwrap();

        let mut buf = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let n = sock.read(&mut chunk).await.unwrap();
            assert!(n > 0, "server closed before ACK");
            buf.extend_from_slice(&chunk[..n]);
            if buf.ends_with(&[END_OF_BLOCK, CARRIAGE_RETURN]) {
                break;
            }
        }
        let ack_text = MllpFrame::unwrap(&buf).unwrap();
        assert!(ack_text.contains("MSA|AA|1"));

        let (first, second) = server_task.await.unwrap();
        assert_eq!(first.get_control_id(), Some("M1"));
        assert_eq!(second.get_control_id(), Some("M2"));
    }
}
