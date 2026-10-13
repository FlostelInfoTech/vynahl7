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

use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
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

/// Configuration for MLLP client and server
#[derive(Debug, Clone)]
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
}

impl Default for MllpConfig {
    fn default() -> Self {
        Self {
            max_message_size: DEFAULT_MAX_MESSAGE_SIZE,
            read_timeout: DEFAULT_READ_TIMEOUT,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            charset_override: None,
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

    /// Send a message and wait for acknowledgment
    ///
    /// The message is encoded in the character set declared in its MSH-18
    /// (HL7 table 0211), defaulting to UTF-8 when MSH-18 is empty.
    pub async fn send_message(&mut self, message: &Message) -> Result<Message> {
        // Encode message in its declared character set
        let hl7_bytes = message.encode_bytes()?;

        // Wrap in MLLP frame
        let framed = MllpFrame::wrap_bytes(&hl7_bytes);

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
                return Err(Error::Network("Connection closed".to_string()));
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
    /// perform TLS handshake on the accepted connection.
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
                return Err(Error::Network("Connection closed".to_string()));
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
}
