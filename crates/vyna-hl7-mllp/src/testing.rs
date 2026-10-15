//! Testing utilities for MLLP
//!
//! This module provides mock servers and test helpers for integration testing.

use crate::{MllpConfig, MllpServer};
use std::net::SocketAddr;
use std::sync::Arc;
use vyna_hl7_core::{Message, builders::ack::AckBuilder, error::Result};

/// Message handler function type
pub type MessageHandler = Arc<dyn Fn(Message) -> Result<Message> + Send + Sync>;

/// Mock MLLP server for integration testing
///
/// Provides an in-process MLLP server with configurable message handlers,
/// automatic port allocation, and cleanup.
///
/// # Example
///
/// ```no_run
/// use vyna_hl7_mllp::testing::MockMllpServer;
/// use vyna_hl7_core::Message;
///
/// #[tokio::test]
/// async fn test_mllp_client() {
///     // Create mock server with custom handler
///     let mut server = MockMllpServer::new()
///         .with_handler(|msg| {
///             // Echo the message back
///             Ok(msg)
///         })
///         .start()
///         .await
///         .unwrap();
///
///     let addr = server.local_addr();
///
///     // Use the server in tests...
///     // The server will automatically shut down when dropped
/// }
/// ```
pub struct MockMllpServer {
    handler: MessageHandler,
    config: MllpConfig,
    #[cfg(feature = "tls")]
    tls_config: Option<crate::tls::TlsServerConfig>,
    server_task: Option<tokio::task::JoinHandle<()>>,
    local_addr: Option<SocketAddr>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockMllpServer {
    /// Create a new mock MLLP server with default configuration
    ///
    /// By default, the server echoes received messages back to the client.
    pub fn new() -> Self {
        Self {
            handler: Arc::new(|msg| Ok(msg.clone())),
            config: MllpConfig::default(),
            #[cfg(feature = "tls")]
            tls_config: None,
            server_task: None,
            local_addr: None,
            shutdown_tx: None,
        }
    }

    /// Set a custom message handler
    ///
    /// The handler receives each message and returns a response message.
    /// A handler error answers with an AE NACK (like a real responder)
    /// and the connection stays open.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_mllp::testing::MockMllpServer;
    /// use vyna_hl7_core::{Message, Segment, Field};
    ///
    /// let server = MockMllpServer::new()
    ///     .with_handler(|_msg| {
    ///         // Return a custom ACK
    ///         let mut ack = Message::default();
    ///         let mut msa = Segment::new("MSA");
    ///         msa.fields.push(Field::from_value("AA"));
    ///         ack.segments.push(msa);
    ///         Ok(ack)
    ///     });
    /// ```
    pub fn with_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(Message) -> Result<Message> + Send + Sync + 'static,
    {
        self.handler = Arc::new(handler);
        self
    }

    /// Set custom MLLP configuration
    pub fn with_config(mut self, config: MllpConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable TLS with the given configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # async fn example() -> vyna_hl7_core::error::Result<()> {
    /// use vyna_hl7_mllp::testing::MockMllpServer;
    /// use vyna_hl7_mllp::tls::TlsServerConfig;
    ///
    /// let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    /// let server = MockMllpServer::new()
    ///     .with_tls(tls_config)
    ///     .start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, tls_config: crate::tls::TlsServerConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }

    /// Start the mock server
    ///
    /// The server will bind to a random available port on localhost and
    /// begin accepting connections.
    pub async fn start(mut self) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Bind to random port
        #[cfg(feature = "tls")]
        let server = if let Some(ref tls_config) = self.tls_config {
            MllpServer::bind_tls_with_config("127.0.0.1:0", tls_config.clone(), self.config.clone())
                .await?
        } else {
            MllpServer::bind_with_config("127.0.0.1:0", self.config.clone()).await?
        };

        #[cfg(not(feature = "tls"))]
        let server = MllpServer::bind_with_config("127.0.0.1:0", self.config.clone()).await?;

        let local_addr = server.local_addr()?;
        self.local_addr = Some(local_addr);

        let handler = self.handler.clone();

        // Spawn server task
        let server_task = tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;

            loop {
                // `select!` on shutdown instead of wrapping `accept()` in
                // a short timeout: the timeout would also bound (and
                // abort) a TLS handshake in progress, while `accept()`
                // alone honors the configured `connect_timeout` like the
                // real server.
                let accepted = tokio::select! {
                    biased;

                    _ = &mut shutdown_rx => break,

                    accepted = server.accept() => accepted,
                };

                let mut conn = match accepted {
                    Ok(conn) => conn,
                    Err(_e) => continue, // Accept error
                };

                let handler = handler.clone();

                // Handle connection
                tokio::spawn(async move {
                    // Receive error or connection closed ends the loop.
                    while let Ok(msg) = conn.receive_message().await {
                        // Process message with handler
                        match handler(msg.clone()) {
                            Ok(response) => {
                                if let Err(_e) = conn.send_message(&response).await {
                                    break; // Send failed
                                }
                            }
                            Err(e) => {
                                // Application errors get an AE NACK like a
                                // real responder; the connection stays open.
                                let nack = AckBuilder::for_message(&msg)
                                    .error(&format!("Mock handler error: {e}"))
                                    .build();
                                match nack {
                                    Ok(nack) => {
                                        if let Err(_e) = conn.send_message(&nack).await {
                                            break; // Send failed
                                        }
                                    }
                                    Err(_e) => break, // Cannot NACK
                                }
                            }
                        }
                    }
                });
            }
        });

        self.server_task = Some(server_task);
        self.shutdown_tx = Some(shutdown_tx);

        Ok(self)
    }

    /// Get the local address the server is bound to
    ///
    /// # Panics
    ///
    /// Panics if the server has not been started.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr.expect("Server not started")
    }

    /// Get the URL to connect to this server
    pub fn url(&self) -> String {
        format!("{}", self.local_addr())
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(task) = self.server_task.take() {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), task).await;
        }

        Ok(())
    }
}

impl Default for MockMllpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MockMllpServer {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(task) = self.server_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MllpClient;
    use vyna_hl7_core::{Field, Segment};

    #[tokio::test]
    async fn test_mock_server_basic() {
        // Create and start mock server
        let server = MockMllpServer::new().start().await.unwrap();

        let addr = server.url();

        // Connect client
        let mut client = MllpClient::connect(&addr).await.unwrap();

        // Create test message
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msh.fields.push(Field::from_value("TEST"));
        msg.segments.push(msh);

        // Send and receive
        let response = client.send_message(&msg).await.unwrap();

        // Server echoes by default
        assert_eq!(msg.encode(), response.encode());

        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_server_custom_handler() {
        // Create server with custom handler
        let server = MockMllpServer::new()
            .with_handler(|_msg| {
                let mut ack = Message::default();

                // Create MSH segment
                let mut msh = Segment::new("MSH");
                msh.fields.push(Field::from_value("|"));
                msh.fields.push(Field::from_value("^~\\&"));
                ack.segments.push(msh);

                // Create MSA segment
                let mut msa = Segment::new("MSA");
                msa.fields.push(Field::from_value("AA"));
                msa.fields.push(Field::from_value("TEST001"));
                ack.segments.push(msa);

                Ok(ack)
            })
            .start()
            .await
            .unwrap();

        let addr = server.url();

        // Connect and send message
        let mut client = MllpClient::connect(&addr).await.unwrap();

        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msg.segments.push(msh);

        let response = client.send_message(&msg).await.unwrap();

        // Check custom response - should have MSH first, then MSA
        assert_eq!(response.segments[0].id, "MSH");
        assert_eq!(response.segments[1].id, "MSA");
        assert_eq!(response.segments[1].fields[0].value(), Some("AA"));

        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_server_handler_error_sends_ae_nack() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use vyna_hl7_core::error::Error;

        // A handler error answers with an AE NACK (not a dropped
        // connection) and the connection stays open for the next
        // message.
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_handler = Arc::clone(&calls);
        let server = MockMllpServer::new()
            .with_handler(move |_msg| {
                if calls_handler.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err(Error::parse("boom"))
                } else {
                    let mut ack = Message::default();
                    let mut msh = Segment::new("MSH");
                    msh.fields.push(Field::from_value("|"));
                    msh.fields.push(Field::from_value("^~\\&"));
                    ack.segments.push(msh);
                    let mut msa = Segment::new("MSA");
                    msa.fields.push(Field::from_value("AA"));
                    ack.segments.push(msa);
                    Ok(ack)
                }
            })
            .start()
            .await
            .unwrap();

        let addr = server.url();
        let mut client = MllpClient::connect(&addr).await.unwrap();

        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msg.segments.push(msh);

        let nack = client.send_message(&msg).await.unwrap();
        let msa = nack.get_segment_by_id("MSA").unwrap();
        assert_eq!(msa.fields[0].value(), Some("AE"));

        // Connection still open: the second message gets its reply.
        let ack = client.send_message(&msg).await.unwrap();
        assert_eq!(
            ack.get_segment_by_id("MSA").unwrap().fields[0].value(),
            Some("AA")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }

    /// Generate a CA plus a server certificate signed by it (requires
    /// openssl). Returns `(ca_cert, server_cert, server_key)`.
    #[cfg(feature = "tls")]
    fn generate_test_certs(dir: &std::path::Path) -> (String, String, String) {
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

    /// A client that connects and then takes longer than the mock's old
    /// 100 ms poll quantum to speak TLS must still complete the
    /// handshake: the mock must honor the configured `connect_timeout`
    /// like the real `MllpServer` instead of dropping the handshake.
    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn test_tls_mock_server_tolerates_slow_handshake() {
        use crate::tls::{TlsClientConfig, TlsServerConfig};
        use crate::{CARRIAGE_RETURN, END_OF_BLOCK, START_OF_BLOCK};
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let dir = std::env::temp_dir().join(format!(
            "vyna_mllp_mocktls_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (ca, cert, key) = generate_test_certs(&dir);

        let server = MockMllpServer::new()
            .with_tls(TlsServerConfig::new(&cert, &key).unwrap())
            .start()
            .await
            .unwrap();
        let addr = server.local_addr();

        let flow = async {
            // Connect, then stall past the old 100 ms accept quantum
            // before speaking TLS — well within the 10 s default.
            let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;

            let tls_client = TlsClientConfig::with_ca_cert(&ca).unwrap();
            let connector = tokio_rustls::TlsConnector::from(tls_client.config.clone());
            let name = rustls::pki_types::ServerName::try_from("localhost")
                .unwrap()
                .to_owned();
            let mut tls = connector.connect(name, tcp).await.unwrap();

            // MLLP round trip over the delayed handshake.
            let hl7 = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5\r";
            let mut framed = vec![START_OF_BLOCK];
            framed.extend_from_slice(hl7.as_bytes());
            framed.extend_from_slice(&[END_OF_BLOCK, CARRIAGE_RETURN]);
            tls.write_all(&framed).await.unwrap();

            let mut reply = Vec::new();
            let mut byte = [0u8; 1];
            loop {
                tls.read_exact(&mut byte).await.unwrap();
                reply.push(byte[0]);
                let n = reply.len();
                if n >= 2 && reply[n - 2] == END_OF_BLOCK && reply[n - 1] == CARRIAGE_RETURN {
                    break;
                }
            }
            let text = String::from_utf8(reply).unwrap();
            assert!(text.contains("ADT^A01"), "unexpected reply: {text}");
        };
        tokio::time::timeout(Duration::from_secs(15), flow)
            .await
            .expect("slow-handshake round trip timed out");

        server.shutdown().await.unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
