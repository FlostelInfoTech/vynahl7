//! HTTP server for receiving HL7 messages

use crate::{decode_body, Error, Result, CONTENT_TYPE_HL7_ER7};
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Router,
};
use vyna_hl7_core::Message;
use vyna_hl7_parser::parse_message;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[cfg(feature = "compression")]
use tower_http::compression::CompressionLayer;

#[cfg(feature = "tls")]
use crate::tls::TlsServerConfig;
#[cfg(feature = "tls")]
use tokio_rustls::TlsAcceptor;
#[cfg(feature = "tls")]
use tower::Service;

/// Message handler function type
///
/// Takes an HL7 message and returns a response message (typically an ACK).
/// The handler is wrapped in an Arc for thread-safe sharing.
pub type MessageHandler = Arc<dyn Fn(Message) -> Result<Message> + Send + Sync>;

/// HTTP server for receiving HL7 v2.x messages
///
/// # Example
///
/// ```no_run
/// use vyna_hl7_http::{HttpServer, MessageHandler};
/// use vyna_hl7_core::Message;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let handler: MessageHandler = Arc::new(|message: Message| {
///     println!("Received: {:?}", message.get_message_type());
///     // Create and return ACK
///     Ok(message) // Simplified for example
/// });
///
/// let server = HttpServer::new()
///     .with_handler(handler)
///     .with_auth("username".into(), "password".into());
///
/// server.serve("127.0.0.1:8080").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct HttpServer {
    handler: MessageHandler,
    auth: Option<(String, String)>,
    #[cfg(feature = "tls")]
    tls_config: Option<TlsServerConfig>,
    #[cfg(feature = "compression")]
    pub(crate) enable_compression: bool,
}

impl HttpServer {
    /// Create a new HTTP server with a default handler
    ///
    /// The default handler simply echoes the received message back.
    pub fn new() -> Self {
        Self {
            handler: Arc::new(Ok),
            auth: None,
            #[cfg(feature = "tls")]
            tls_config: None,
            #[cfg(feature = "compression")]
            enable_compression: false,
        }
    }

    /// Set the message handler function
    ///
    /// # Arguments
    /// * `handler` - Function that processes incoming messages and returns responses
    pub fn with_handler(mut self, handler: MessageHandler) -> Self {
        self.handler = handler;
        self
    }

    /// Enable HTTP Basic Authentication
    ///
    /// # Arguments
    /// * `username` - Required username
    /// * `password` - Required password
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.auth = Some((username, password));
        self
    }

    /// Configure TLS/mTLS for the HTTP server
    ///
    /// # Arguments
    /// * `tls_config` - TLS configuration including optional client certificate verification
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::{HttpServer, tls::TlsServerConfig};
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tls_config = TlsServerConfig::with_mtls(
    ///     "server-cert.pem",
    ///     "server-key.pem",
    ///     "ca-cert.pem"
    /// )?;
    ///
    /// let server = HttpServer::new()
    ///     .with_tls(tls_config);
    ///
    /// server.serve_tls("127.0.0.1:8443").await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, tls_config: TlsServerConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }

    /// Enable gzip/brotli compression for responses
    ///
    /// When enabled, the server will automatically compress responses using gzip or brotli
    /// based on the client's Accept-Encoding header.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "compression")]
    /// # {
    /// use vyna_hl7_http::HttpServer;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = HttpServer::new()
    ///     .with_compression();
    ///
    /// server.serve("127.0.0.1:8080").await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "compression")]
    pub fn with_compression(mut self) -> Self {
        self.enable_compression = true;
        self
    }

    /// Start the HTTP server
    ///
    /// # Arguments
    /// * `addr` - The address to bind to (e.g., "127.0.0.1:8080" or "0.0.0.0:8080")
    ///
    /// # Errors
    /// Returns an error if the server cannot bind to the specified address
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use vyna_hl7_http::HttpServer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let server = HttpServer::new();
    /// server.serve("127.0.0.1:8080").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve(self, addr: &str) -> Result<()> {
        // Build router
        let app = Router::new()
            .route("/", post(handle_message))
            .route("/{*path}", post(handle_message))
            .layer(TraceLayer::new_for_http());

        // Add compression layer if enabled
        #[cfg(feature = "compression")]
        let app = if self.enable_compression {
            app.layer(CompressionLayer::new())
        } else {
            app
        };

        let app = app.with_state(self);

        // Bind server
        let listener = tokio::net::TcpListener::bind(addr).await?;

        println!("HTTP server listening on {}", addr);

        // Serve
        axum::serve(listener, app)
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        Ok(())
    }

    /// Start the HTTPS server with TLS/mTLS
    ///
    /// # Arguments
    /// * `addr` - The address to bind to (e.g., "127.0.0.1:8443" or "0.0.0.0:8443")
    ///
    /// # Errors
    /// Returns an error if the server cannot bind to the specified address or TLS is not configured
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::{HttpServer, tls::TlsServerConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    ///
    /// let server = HttpServer::new()
    ///     .with_tls(tls_config);
    ///
    /// server.serve_tls("127.0.0.1:8443").await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub async fn serve_tls(self, addr: &str) -> Result<()> {
        // Extract TLS config or return error
        let tls_config = self.tls_config.clone()
            .ok_or_else(|| Error::InvalidUrl("TLS not configured. Use with_tls() first.".to_string()))?;

        // Build router
        let app = Router::new()
            .route("/", post(handle_message))
            .route("/{*path}", post(handle_message))
            .layer(TraceLayer::new_for_http());

        // Add compression layer if enabled
        #[cfg(feature = "compression")]
        let app = if self.enable_compression {
            app.layer(CompressionLayer::new())
        } else {
            app
        };

        let app = app.with_state(self);

        // Create TLS acceptor
        let tls_acceptor = TlsAcceptor::from(tls_config.config.clone());

        // Bind server
        let listener = tokio::net::TcpListener::bind(addr).await?;

        println!("HTTPS server listening on {} (TLS enabled)", addr);

        // Accept connections and handle them with TLS
        loop {
            let (tcp_stream, _remote_addr) = listener.accept().await?;

            // Clone acceptor and app for the task
            let tls_acceptor = tls_acceptor.clone();
            let app = app.clone();

            tokio::spawn(async move {
                // Perform TLS handshake
                match tls_acceptor.accept(tcp_stream).await {
                    Ok(tls_stream) => {
                        // Serve the connection using hyper
                        let tower_service = app;
                        let hyper_service = hyper::service::service_fn(move |request| {
                            tower_service.clone().call(request)
                        });

                        if let Err(err) = hyper::server::conn::http1::Builder::new()
                            .serve_connection(hyper_util::rt::TokioIo::new(tls_stream), hyper_service)
                            .await
                        {
                            eprintln!("Error serving connection: {}", err);
                        }
                    }
                    Err(err) => {
                        eprintln!("TLS handshake failed: {}", err);
                    }
                }
            });
        }
    }
}

impl Default for HttpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle incoming HTTP POST requests with HL7 messages
pub(crate) async fn handle_message(
    State(server): State<HttpServer>,
    headers: HeaderMap,
    body: Bytes,
) -> std::result::Result<(StatusCode, HeaderMap, Vec<u8>), (StatusCode, String)> {
    // Verify content type
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !content_type.contains("hl7") {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Invalid content type: expected {}, got {}", CONTENT_TYPE_HL7_ER7, content_type),
        ));
    }

    // Check authentication if required
    if let Some((_username, _password)) = &server.auth {
        if let Some(_auth_header) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
            #[cfg(feature = "auth")]
            {
                use crate::auth::verify_basic_auth;
                if !verify_basic_auth(_auth_header, _username, _password) {
                    return Err((StatusCode::UNAUTHORIZED, "Authentication failed".to_string()));
                }
            }
            #[cfg(not(feature = "auth"))]
            {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Authentication configured but 'auth' feature not enabled".to_string(),
                ));
            }
        } else {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Authentication required but no Authorization header provided".to_string(),
            ));
        }
    }

    // Decode the body, preferring the Content-Type charset parameter
    let text = decode_body(&body, content_type).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to decode HL7 message: {}", e),
        )
    })?;

    // Parse HL7 message
    let message = parse_message(&text).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Failed to parse HL7 message: {}", e),
        )
    })?;

    // Handle message
    let response = (server.handler)(message).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to handle message: {}", e),
        )
    })?;

    // Encode the response in its own MSH-18 character set
    let response_charset = response.get_character_set().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode HL7 response: {}", e),
        )
    })?;
    let response_body = response_charset.encode(&response.encode()).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode HL7 response: {}", e),
        )
    })?;

    // Build response headers
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "content-type",
        format!(
            "{}; charset={}",
            CONTENT_TYPE_HL7_ER7,
            response_charset.iana_name()
        )
        .parse()
        .unwrap(),
    );
    response_headers.insert(
        "date",
        chrono::Utc::now()
            .to_rfc2822()
            .parse()
            .unwrap(),
    );

    // Return response
    Ok((StatusCode::OK, response_headers, response_body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_server() {
        let server = HttpServer::new();
        assert!(server.auth.is_none());
    }

    #[test]
    fn test_with_auth() {
        let server = HttpServer::new()
            .with_auth("user".to_string(), "pass".to_string());

        assert!(server.auth.is_some());
        let (username, password) = server.auth.unwrap();
        assert_eq!(username, "user");
        assert_eq!(password, "pass");
    }

    #[test]
    fn test_with_handler() {
        let custom_handler: MessageHandler = Arc::new(|msg| {
            // Custom handler logic
            Ok(msg)
        });

        let server = HttpServer::new().with_handler(custom_handler);
        // Just verify it compiles and doesn't panic
        assert!(server.auth.is_none());
    }

    #[tokio::test]
    async fn test_latin1_client_server_round_trip() {
        use crate::client::HttpClient;

        // Echo handler: the response carries the same MSH-18 declaration.
        let handler: MessageHandler = Arc::new(Ok);
        let server = HttpServer::new().with_handler(handler);
        let app = Router::new()
            .route("/", post(handle_message))
            .with_state(server);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let hl7 = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1||12345^^^FAC^MR||Müller^Jörg||19800101|M\r";
        let message = parse_message(hl7).unwrap();
        let client = HttpClient::new(format!("http://{addr}")).unwrap();
        let echoed = client.send_message(&message).await.unwrap();
        let pid = echoed.get_segment_by_id("PID").unwrap();
        let name = pid.get_field(5).unwrap().get_repetition(0).unwrap();
        assert_eq!(name.get_component(0).unwrap().value(), Some("Müller"));
        assert_eq!(name.get_component(1).unwrap().value(), Some("Jörg"));

        // The raw-text path detects MSH-18 the same way.
        let raw = client.send_message_raw(hl7).await.unwrap();
        assert!(raw.contains("Müller"));
    }
}
