//! Testing utilities for HTTP transport
//!
//! This module provides mock servers and test helpers for integration testing.

use crate::{HttpServer, Result};
use vyna_hl7_core::Message;
use std::net::SocketAddr;
use tokio::net::TcpListener;

/// Mock HTTP server for integration testing
///
/// Provides an in-process HTTP server with configurable message handlers,
/// automatic port allocation, and cleanup.
///
/// # Example
///
/// ```no_run
/// use vyna_hl7_http::testing::MockHttpServer;
/// use vyna_hl7_core::Message;
///
/// #[tokio::test]
/// async fn test_http_client() {
///     // Create mock server with custom handler
///     let mut server = MockHttpServer::new()
///         .with_handler(|msg| {
///             // Echo the message back
///             Ok(msg)
///         })
///         .start()
///         .await
///         .unwrap();
///
///     let url = server.url();
///
///     // Use the server in tests...
///     // The server will automatically shut down when dropped
/// }
/// ```
pub struct MockHttpServer {
    http_server: HttpServer,
    server_task: Option<tokio::task::JoinHandle<()>>,
    local_addr: Option<SocketAddr>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockHttpServer {
    /// Create a new mock HTTP server with default configuration
    ///
    /// By default, the server echoes received messages back to the client.
    pub fn new() -> Self {
        Self {
            http_server: HttpServer::new(),
            server_task: None,
            local_addr: None,
            shutdown_tx: None,
        }
    }

    /// Set a custom message handler
    ///
    /// The handler receives each message and returns a response message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_http::testing::MockHttpServer;
    /// use vyna_hl7_core::{Message, Segment, Field};
    /// use std::sync::Arc;
    ///
    /// let server = MockHttpServer::new()
    ///     .with_handler(|_msg| {
    ///         // Return a custom ACK
    ///         let mut ack = Message::default();
    ///         let mut msh = Segment::new("MSH");
    ///         msh.fields.push(Field::from_value("|"));
    ///         msh.fields.push(Field::from_value("^~\\&"));
    ///         ack.segments.push(msh);
    ///
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
        self.http_server = self.http_server.clone().with_handler(std::sync::Arc::new(handler));
        self
    }

    /// Enable HTTP Basic Authentication
    ///
    /// Requires the `auth` feature; without it every authenticated request
    /// would fail, so the method is only available with the feature enabled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "auth")]
    /// # {
    /// use vyna_hl7_http::testing::MockHttpServer;
    ///
    /// let server = MockHttpServer::new()
    ///     .with_auth("testuser".to_string(), "testpass".to_string());
    /// # }
    /// ```
    #[cfg(feature = "auth")]
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.http_server = self.http_server.clone().with_auth(username, password);
        self
    }

    /// Enable TLS with the given configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # async fn example() -> vyna_hl7_http::error::Result<()> {
    /// use vyna_hl7_http::testing::MockHttpServer;
    /// use vyna_hl7_http::tls::TlsServerConfig;
    ///
    /// let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    /// let server = MockHttpServer::new()
    ///     .with_tls(tls_config)
    ///     .start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, tls_config: crate::tls::TlsServerConfig) -> Self {
        self.http_server = self.http_server.clone().with_tls(tls_config);
        self
    }

    /// Enable compression
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "compression")]
    /// # async fn example() -> vyna_hl7_http::error::Result<()> {
    /// use vyna_hl7_http::testing::MockHttpServer;
    ///
    /// let server = MockHttpServer::new()
    ///     .with_compression()
    ///     .start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "compression")]
    pub fn with_compression(mut self) -> Self {
        self.http_server = self.http_server.clone().with_compression();
        self
    }

    /// Cap the request body size in bytes (default: 2 MB).
    ///
    /// Bodies larger than the cap are rejected with `413 Payload Too Large`
    /// before reaching the handler. Mirrors
    /// [`HttpServer::with_max_body_size`](crate::HttpServer::with_max_body_size).
    pub fn with_max_body_size(mut self, max_body_size: usize) -> Self {
        self.http_server = self.http_server.clone().with_max_body_size(max_body_size);
        self
    }

    /// Bound how long one request may take (default: 300 s).
    ///
    /// Requests that exceed the bound fail with `408 Request Timeout`.
    /// Mirrors [`HttpServer::with_request_timeout`](crate::HttpServer::with_request_timeout).
    pub fn with_request_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.http_server = self.http_server.clone().with_request_timeout(timeout);
        self
    }

    /// Start the mock server
    ///
    /// The server will bind to a random available port on localhost and
    /// begin accepting connections.
    pub async fn start(mut self) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Bind to random port
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let local_addr = listener.local_addr()?;
        self.local_addr = Some(local_addr);

        // Clone the server for the task
        let server = self.http_server.clone();

        // Serve HTTPS when a TLS configuration was provided; otherwise the
        // stored TLS config would be silently ignored.
        #[cfg(feature = "tls")]
        if server.has_tls() {
            let server_task = tokio::spawn(async move {
                server
                    .serve_tls_on(listener, shutdown_rx)
                    .await
                    .expect("TLS server error");
            });
            self.server_task = Some(server_task);
            self.shutdown_tx = Some(shutdown_tx);
            return Ok(self);
        }

        // Build the router through the same code path as HttpServer so
        // the mock enforces the same body limit, request timeout, and
        // optional compression as the real server.
        let app = server.build_router();

        // Spawn server task with graceful shutdown
        let server_task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("Server error");
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
    ///
    /// Uses the `https` scheme when the server was configured with TLS.
    pub fn url(&self) -> String {
        #[cfg(feature = "tls")]
        if self.http_server.has_tls() {
            return format!("https://{}", self.local_addr());
        }
        format!("http://{}", self.local_addr())
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

impl Default for MockHttpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MockHttpServer {
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
    use crate::HttpClient;
    use vyna_hl7_core::{Field, Segment};

    #[tokio::test]
    async fn test_mock_server_basic() {
        // Create and start mock server
        let server = MockHttpServer::new().start().await.unwrap();

        let url = server.url();

        // Connect client
        let client = HttpClient::new(&url).unwrap();

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

        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_server_custom_handler() {
        // Create server with custom handler
        let server = MockHttpServer::new()
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

        let url = server.url();

        // Connect and send message
        let client = HttpClient::new(&url).unwrap();

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

        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    #[cfg(feature = "auth")]
    async fn test_mock_server_with_auth() {
        // Create server with authentication
        let server = MockHttpServer::new()
            .with_auth("testuser".to_string(), "testpass".to_string())
            .start()
            .await
            .unwrap();

        let url = server.url();

        // Create client with matching credentials
        let client = HttpClient::new(&url)
            .unwrap()
            .with_auth("testuser".to_string(), "testpass".to_string());

        // Create test message
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msg.segments.push(msh);

        // Should succeed with correct credentials
        let response = client.send_message(&msg).await.unwrap();
        assert_eq!(response.segments[0].id, "MSH");

        server.shutdown().await.unwrap();
    }

    /// Generate a CA plus a server certificate signed by it (requires openssl).
    #[cfg(feature = "tls")]
    fn generate_test_certs() -> (std::path::PathBuf, String, String, String) {
        use std::process::Command;

        let dir = std::env::temp_dir().join(format!(
            "vyna_http_tls_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = |name: &str| dir.join(name).to_str().unwrap().to_string();
        let (ca_key, ca_cert) = (path("ca-key.pem"), path("ca-cert.pem"));
        let (server_key, server_cert, server_csr, server_ext) = (
            path("server-key.pem"),
            path("server-cert.pem"),
            path("server.csr"),
            path("server-ext.cnf"),
        );

        let run = |args: &[&str]| {
            let status = Command::new("openssl").args(args).status().unwrap();
            assert!(status.success(), "openssl {args:?} failed");
        };
        run(&["genrsa", "-out", &ca_key, "2048"]);
        run(&[
            "req",
            "-new",
            "-x509",
            "-days",
            "2",
            "-key",
            &ca_key,
            "-out",
            &ca_cert,
            "-subj",
            "/CN=Test CA",
        ]);
        run(&["genrsa", "-out", &server_key, "2048"]);
        run(&[
            "req",
            "-new",
            "-key",
            &server_key,
            "-out",
            &server_csr,
            "-subj",
            "/CN=localhost",
        ]);
        std::fs::write(
            &server_ext,
            "[v3_req]\nbasicConstraints = CA:FALSE\nkeyUsage = digitalSignature, keyEncipherment\n\
             extendedKeyUsage = serverAuth\nsubjectAltName = @alt_names\n\n\
             [alt_names]\nDNS.1 = localhost\nIP.1 = 127.0.0.1\n",
        )
        .unwrap();
        run(&[
            "x509",
            "-req",
            "-days",
            "2",
            "-in",
            &server_csr,
            "-CA",
            &ca_cert,
            "-CAkey",
            &ca_key,
            "-CAcreateserial",
            "-out",
            &server_cert,
            "-extfile",
            &server_ext,
            "-extensions",
            "v3_req",
        ]);

        (dir, ca_cert, server_cert, server_key)
    }

    #[tokio::test]
    #[cfg(feature = "tls")]
    async fn test_mock_server_with_tls_serves_https() {
        use crate::tls::{TlsClientConfig, TlsServerConfig};

        let (dir, ca_cert, server_cert, server_key) = generate_test_certs();

        let server = MockHttpServer::new()
            .with_tls(TlsServerConfig::new(&server_cert, &server_key).unwrap())
            .start()
            .await
            .unwrap();

        // The URL must use the https scheme so clients negotiate TLS.
        let url = server.url();
        assert!(url.starts_with("https://"), "{url}");

        let client = HttpClient::new(&url)
            .unwrap()
            .with_tls(TlsClientConfig::with_ca_cert(&ca_cert).unwrap())
            .unwrap();

        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msh.fields.push(Field::from_value("TEST"));
        msg.segments.push(msh);

        // Server echoes by default, over a real TLS connection.
        let response = client.send_message(&msg).await.unwrap();
        assert_eq!(msg.encode(), response.encode());

        server.shutdown().await.unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// The mock must answer a stalled partial body with 408 like the real
    /// server instead of leaving the connection hanging.
    #[tokio::test]
    async fn test_mock_server_request_timeout_closes_stalled_body() {
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let server = MockHttpServer::new()
            .with_request_timeout(Duration::from_millis(300))
            .start()
            .await
            .unwrap();
        let addr = server.local_addr();

        // Complete headers promising 100 body bytes, then only 7 bytes.
        let partial = concat!(
            "POST / HTTP/1.1\r\nHost: x\r\n",
            "Content-Type: x-application/hl7-v2+er7\r\n",
            "Content-Length: 100\r\n\r\n1234567"
        );
        let mut tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
        tcp.write_all(partial.as_bytes()).await.unwrap();

        let mut buf = [0u8; 1024];
        let read = tokio::time::timeout(Duration::from_secs(5), tcp.read(&mut buf)).await;
        let n = read
            .expect("mock left stalled partial body open (missing timeout layer)")
            .expect("reading stalled connection failed");
        if n > 0 {
            let head = String::from_utf8_lossy(&buf[..n]);
            assert!(head.contains("408"), "expected 408, got: {head}");
        }

        server.shutdown().await.unwrap();
    }

    /// The mock must enforce the body-size limit, and the knob must move it.
    #[tokio::test]
    async fn test_mock_server_body_limit_enforced_and_configurable() {
        let mut big = String::from("MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5\r");
        while big.len() <= 64 * 1024 {
            big.push_str("NTE|1||padding-padding-padding-padding-padding\r");
        }

        // A ~64 KiB body exceeds a 1 KiB cap: expect 413.
        let server = MockHttpServer::new()
            .with_max_body_size(1024)
            .start()
            .await
            .unwrap();
        let status = reqwest::Client::new()
            .post(format!("{}/", server.url()))
            .header("content-type", crate::CONTENT_TYPE_HL7_ER7)
            .body(big.clone())
            .send()
            .await
            .unwrap()
            .status();
        assert_eq!(status, reqwest::StatusCode::PAYLOAD_TOO_LARGE);
        server.shutdown().await.unwrap();

        // A raised cap accepts the same body.
        let server = MockHttpServer::new()
            .with_max_body_size(8 * 1024 * 1024)
            .start()
            .await
            .unwrap();
        let status = reqwest::Client::new()
            .post(format!("{}/", server.url()))
            .header("content-type", crate::CONTENT_TYPE_HL7_ER7)
            .body(big)
            .send()
            .await
            .unwrap()
            .status();
        assert_eq!(status, reqwest::StatusCode::OK);
        server.shutdown().await.unwrap();
    }
}
