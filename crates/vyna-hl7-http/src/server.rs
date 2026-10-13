//! HTTP server for receiving HL7 messages

use crate::{CONTENT_TYPE_HL7_ER7, Error, Result, decode_body};
use axum::{
    Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::WWW_AUTHENTICATE},
    response::IntoResponse,
    routing::post,
};
use std::sync::Arc;
use std::time::Duration;
use tower::{Layer, Service};
use tower_http::trace::TraceLayer;
use vyna_hl7_core::Message;
use vyna_hl7_parser::parse_message;

#[cfg(feature = "compression")]
use tower_http::compression::CompressionLayer;

#[cfg(feature = "tls")]
use crate::tls::TlsServerConfig;
#[cfg(feature = "tls")]
use tokio_rustls::TlsAcceptor;

/// Message handler function type
///
/// Takes an HL7 message and returns a response message (typically an ACK).
/// The handler is wrapped in an Arc for thread-safe sharing.
pub type MessageHandler = Arc<dyn Fn(Message) -> Result<Message> + Send + Sync>;

/// Default bound on a single TLS handshake (see
/// [`HttpServer::with_tls_handshake_timeout`]).
#[cfg(feature = "tls")]
pub const DEFAULT_TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Default bound on a single TLS connection lifetime (see
/// [`HttpServer::with_tls_connection_timeout`]).
#[cfg(feature = "tls")]
pub const DEFAULT_TLS_CONNECTION_TIMEOUT: Duration = Duration::from_secs(300);

/// Default cap on a single request body (see [`HttpServer::with_max_body_size`]).
///
/// Matches axum's implicit 2 MB default, now explicit and configurable.
pub const DEFAULT_MAX_BODY_SIZE: usize = 2 * 1024 * 1024;

/// Default bound on one HTTP request (see
/// [`HttpServer::with_request_timeout`]).
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

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
///     .with_handler(handler);
/// # #[cfg(feature = "auth")]
/// # let server = server.with_auth("username".into(), "password".into());
///
/// server.serve("127.0.0.1:8080").await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct HttpServer {
    handler: MessageHandler,
    auth: Option<(String, String)>,
    /// Cap on a single request body in bytes (default: 2 MB).
    max_body_size: usize,
    /// Bound on one request's headers, body, and handler (default: 300 s).
    request_timeout: Duration,
    #[cfg(feature = "tls")]
    tls_config: Option<TlsServerConfig>,
    /// Bound on `TlsAcceptor::accept` per connection (default: 10 s).
    #[cfg(feature = "tls")]
    tls_handshake_timeout: Duration,
    /// Bound on serving one TLS connection (default: 300 s).
    #[cfg(feature = "tls")]
    tls_connection_timeout: Duration,
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
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            #[cfg(feature = "tls")]
            tls_config: None,
            #[cfg(feature = "tls")]
            tls_handshake_timeout: DEFAULT_TLS_HANDSHAKE_TIMEOUT,
            #[cfg(feature = "tls")]
            tls_connection_timeout: DEFAULT_TLS_CONNECTION_TIMEOUT,
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
    /// Requires the `auth` feature; without it every authenticated request
    /// would fail, so the method is only available with the feature enabled.
    ///
    /// # Arguments
    /// * `username` - Required username
    /// * `password` - Required password
    #[cfg(feature = "auth")]
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

    /// Bound how long one TLS handshake may take (default: 10 s).
    ///
    /// Each accepted TCP connection spawns a task that performs the TLS
    /// handshake; without a bound, clients that connect and stall pin a
    /// task forever (task-exhaustion DoS). Connections that exceed the
    /// bound are dropped.
    #[cfg(feature = "tls")]
    pub fn with_tls_handshake_timeout(mut self, timeout: Duration) -> Self {
        self.tls_handshake_timeout = timeout;
        self
    }

    /// Bound how long one TLS connection is served (default: 300 s).
    ///
    /// Caps the lifetime of the per-connection task that runs hyper after
    /// the handshake, so stalled connections (e.g. slowloris-style partial
    /// requests) cannot accumulate without bound. Connections that exceed
    /// the bound are dropped.
    #[cfg(feature = "tls")]
    pub fn with_tls_connection_timeout(mut self, timeout: Duration) -> Self {
        self.tls_connection_timeout = timeout;
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

    /// Cap the request body size in bytes (default: 2 MB).
    ///
    /// Bodies larger than the cap are rejected with `413 Payload Too Large`
    /// before reaching the handler. Raise this for large clinical batches.
    pub fn with_max_body_size(mut self, max_body_size: usize) -> Self {
        self.max_body_size = max_body_size;
        self
    }

    /// Bound how long one request may take (default: 300 s).
    ///
    /// Covers receiving the headers and body plus running the handler;
    /// requests that exceed the bound fail with `408 Request Timeout` and
    /// the connection is closed, so stalled partial requests (e.g.
    /// slowloris-style stalls) cannot pin a connection forever. This is
    /// the plain-HTTP counterpart of `with_tls_connection_timeout`.
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Build the axum router: routes plus tracing, the body-size limit,
    /// the request timeout, optional compression, and this server as state.
    ///
    /// Shared with the `testing` mock server so it enforces the same
    /// limits and timeouts as the real server.
    pub(crate) fn build_router(self) -> Router {
        let app = Router::new()
            .route("/", post(handle_message))
            .route("/{*path}", post(handle_message))
            .layer(TraceLayer::new_for_http())
            .layer(DefaultBodyLimit::max(self.max_body_size))
            .layer(RequestTimeoutLayer::new(self.request_timeout));

        // Add compression layer if enabled
        #[cfg(feature = "compression")]
        let app = if self.enable_compression {
            app.layer(CompressionLayer::new())
        } else {
            app
        };

        app.with_state(self)
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
        let app = self.build_router();

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
        // Bind server
        let listener = tokio::net::TcpListener::bind(addr).await?;

        println!("HTTPS server listening on {} (TLS enabled)", addr);

        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        self.serve_tls_on(listener, shutdown_rx).await
    }

    /// Serve HTTPS on an already-bound listener until `shutdown` fires.
    ///
    /// Used by `MockHttpServer` so integration tests exercise the real TLS path.
    #[cfg(feature = "tls")]
    pub(crate) async fn serve_tls_on(
        self,
        listener: tokio::net::TcpListener,
        mut shutdown: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<()> {
        // Extract TLS config or return error
        let tls_config = self.tls_config.clone().ok_or_else(|| {
            Error::InvalidUrl("TLS not configured. Use with_tls() first.".to_string())
        })?;

        // Copy the per-connection bounds before `build_router` moves `self`.
        let handshake_timeout = self.tls_handshake_timeout;
        let connection_timeout = self.tls_connection_timeout;

        let app = self.build_router();

        // Create TLS acceptor
        let tls_acceptor = TlsAcceptor::from(tls_config.config.clone());

        // Accept connections and handle them with TLS
        loop {
            tokio::select! {
                res = listener.accept() => {
                    let (tcp_stream, _remote_addr) = res?;

                    // Clone acceptor and app for the task
                    let tls_acceptor = tls_acceptor.clone();
                    let app = app.clone();

                    tokio::spawn(async move {
                        // Perform TLS handshake, bounded so stalled clients
                        // cannot pin this task forever.
                        let tls_stream =
                            match tokio::time::timeout(
                                handshake_timeout,
                                tls_acceptor.accept(tcp_stream),
                            )
                            .await
                            {
                                Ok(Ok(tls_stream)) => tls_stream,
                                Ok(Err(err)) => {
                                    eprintln!("TLS handshake failed: {}", err);
                                    return;
                                }
                                Err(_) => {
                                    eprintln!(
                                        "TLS handshake timed out after {:?}",
                                        handshake_timeout
                                    );
                                    return;
                                }
                            };

                        // Serve the connection using hyper, bounded so
                        // stalled connections cannot accumulate without end.
                        let tower_service = app;
                        let hyper_service = hyper::service::service_fn(move |request| {
                            tower_service.clone().call(request)
                        });

                        match tokio::time::timeout(
                            connection_timeout,
                            hyper::server::conn::http1::Builder::new().serve_connection(
                                hyper_util::rt::TokioIo::new(tls_stream),
                                hyper_service,
                            ),
                        )
                        .await
                        {
                            Ok(Ok(())) => {}
                            Ok(Err(err)) => {
                                eprintln!("Error serving connection: {}", err);
                            }
                            Err(_) => {
                                eprintln!(
                                    "TLS connection timed out after {:?}",
                                    connection_timeout
                                );
                            }
                        }
                    });
                }
                _ = &mut shutdown => break,
            }
        }

        Ok(())
    }

    /// True when a TLS configuration was provided via `with_tls`.
    ///
    /// Only used by the `testing` mock server to pick the HTTPS path, so it
    /// is only compiled when both `tls` and `testing` are enabled.
    #[cfg(all(feature = "tls", feature = "testing"))]
    pub(crate) fn has_tls(&self) -> bool {
        self.tls_config.is_some()
    }
}

impl Default for HttpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Tower [`Layer`] bounding each request's total service time.
///
/// The inner service (routing, body collection, handler) gets `timeout`
/// to produce a response; slower requests fail with `408 Request Timeout`
/// and the connection is closed, so stalled partial requests cannot
/// accumulate without bound.
#[derive(Debug, Clone, Copy)]
struct RequestTimeoutLayer {
    timeout: Duration,
}

impl RequestTimeoutLayer {
    fn new(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl<S> Layer<S> for RequestTimeoutLayer {
    type Service = RequestTimeoutService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestTimeoutService {
            inner,
            timeout: self.timeout,
        }
    }
}

/// Service produced by [`RequestTimeoutLayer`].
#[derive(Debug, Clone)]
struct RequestTimeoutService<S> {
    inner: S,
    timeout: Duration,
}

impl<S> Service<axum::extract::Request> for RequestTimeoutService<S>
where
    S: Service<
            axum::extract::Request,
            Response = axum::response::Response,
            Error = std::convert::Infallible,
        > + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    type Response = axum::response::Response;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn Future<Output = std::result::Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: axum::extract::Request) -> Self::Future {
        let mut inner = self.inner.clone();
        std::mem::swap(&mut self.inner, &mut inner);
        let timeout = self.timeout;
        Box::pin(async move {
            match tokio::time::timeout(timeout, inner.call(request)).await {
                Ok(response) => response,
                Err(_) => Ok((StatusCode::REQUEST_TIMEOUT, "request timed out").into_response()),
            }
        })
    }
}

/// True when `content_type` names the ER7 media type.
///
/// Compares the media type (before any `;` parameters) case-insensitively
/// against [`CONTENT_TYPE_HL7_ER7`]. Substring matching is deliberately
/// avoided so spoof types such as `application/hl7-evil` are rejected while
/// parameters (e.g. `; charset=utf-8`) are allowed. The XML media type is
/// rejected: this crate parses ER7 only, so admitting it would misparse
/// ER7 bodies sent under it and 400 XML bodies at the ER7 parser.
pub(crate) fn is_hl7_content_type(content_type: &str) -> bool {
    let media_type = content_type.split(';').next().unwrap_or("").trim();
    media_type.eq_ignore_ascii_case(CONTENT_TYPE_HL7_ER7)
}

/// Build an error triple with no extra headers.
fn plain_error(status: StatusCode, message: String) -> (StatusCode, HeaderMap, String) {
    (status, HeaderMap::new(), message)
}

/// Build a 401 triple carrying the `Basic` challenge (RFC 9110 §11.6.1).
fn unauthorized_error(message: String) -> (StatusCode, HeaderMap, String) {
    let mut headers = HeaderMap::new();
    headers.insert(
        WWW_AUTHENTICATE,
        HeaderValue::from_static("Basic realm=\"vyna-hl7\""),
    );
    (StatusCode::UNAUTHORIZED, headers, message)
}

/// Handle incoming HTTP POST requests with HL7 messages
// `result_large_err` is intentionally allowed: axum boxes the handler
// future, so the `Err` triple's stack size is irrelevant on this path.
#[allow(clippy::result_large_err)]
pub(crate) async fn handle_message(
    State(server): State<HttpServer>,
    headers: HeaderMap,
    body: Bytes,
) -> std::result::Result<(StatusCode, HeaderMap, Vec<u8>), (StatusCode, HeaderMap, String)> {
    // Check authentication first so unauthenticated callers cannot probe
    // validation (content type, parsing) and learn about the endpoint.
    if let Some((_username, _password)) = &server.auth {
        if let Some(_auth_header) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
            #[cfg(feature = "auth")]
            {
                use crate::auth::verify_basic_auth;
                if !verify_basic_auth(_auth_header, _username, _password) {
                    return Err(unauthorized_error("Authentication failed".to_string()));
                }
            }
            #[cfg(not(feature = "auth"))]
            {
                return Err(plain_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Authentication configured but 'auth' feature not enabled".to_string(),
                ));
            }
        } else {
            return Err(unauthorized_error(
                "Authentication required but no Authorization header provided".to_string(),
            ));
        }
    }

    // Verify content type
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !is_hl7_content_type(content_type) {
        return Err(plain_error(
            StatusCode::BAD_REQUEST,
            format!(
                "Invalid content type: expected {}, got {}",
                CONTENT_TYPE_HL7_ER7, content_type
            ),
        ));
    }

    // Decode the body, preferring the Content-Type charset parameter
    let text = decode_body(&body, content_type).map_err(|e| {
        plain_error(
            StatusCode::BAD_REQUEST,
            format!("Failed to decode HL7 message: {}", e),
        )
    })?;

    // Parse HL7 message
    let message = parse_message(&text).map_err(|e| {
        plain_error(
            StatusCode::BAD_REQUEST,
            format!("Failed to parse HL7 message: {}", e),
        )
    })?;

    // Handle message
    let response = (server.handler)(message).map_err(|e| {
        plain_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to handle message: {}", e),
        )
    })?;

    // Encode the response in its own MSH-18 character set
    let response_charset = response.get_character_set().map_err(|e| {
        plain_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode HL7 response: {}", e),
        )
    })?;
    let response_body = response_charset.encode(&response.encode()).map_err(|e| {
        plain_error(
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
    response_headers.insert("date", chrono::Utc::now().to_rfc2822().parse().unwrap());

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
    #[cfg(feature = "auth")]
    fn test_with_auth() {
        let server = HttpServer::new().with_auth("user".to_string(), "pass".to_string());

        assert!(server.auth.is_some());
        let (username, password) = server.auth.unwrap();
        assert_eq!(username, "user");
        assert_eq!(password, "pass");
    }

    #[test]
    fn test_body_limit_default_and_knob() {
        assert_eq!(HttpServer::new().max_body_size, DEFAULT_MAX_BODY_SIZE);
        assert_eq!(DEFAULT_MAX_BODY_SIZE, 2 * 1024 * 1024);
        let server = HttpServer::new().with_max_body_size(1024);
        assert_eq!(server.max_body_size, 1024);
    }

    async fn post_status(app: Router, body: String) -> reqwest::StatusCode {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        reqwest::Client::new()
            .post(format!("http://{addr}/"))
            .header("content-type", CONTENT_TYPE_HL7_ER7)
            .body(body)
            .send()
            .await
            .unwrap()
            .status()
    }

    /// Oversized bodies get 413, and the knob moves the boundary.
    #[tokio::test]
    async fn test_body_limit_enforced_and_configurable() {
        // A valid message just over the 2 MB default: base MSH plus padding.
        let mut big = String::from(VALID_HL7);
        while big.len() <= 2 * 1024 * 1024 + 1024 {
            big.push_str("NTE|1||padding-padding-padding-padding-padding\r");
        }
        let status = post_status(HttpServer::new().build_router(), big.clone()).await;
        assert_eq!(status, reqwest::StatusCode::PAYLOAD_TOO_LARGE);
        let raised = HttpServer::new()
            .with_max_body_size(8 * 1024 * 1024)
            .build_router();
        let status = post_status(raised, big).await;
        assert_eq!(status, reqwest::StatusCode::OK);
    }

    #[test]
    fn test_request_timeout_default_and_override() {
        use std::time::Duration;

        assert_eq!(HttpServer::new().request_timeout, Duration::from_secs(300));
        let server = HttpServer::new().with_request_timeout(Duration::from_secs(7));
        assert_eq!(server.request_timeout, Duration::from_secs(7));
    }

    /// A client that sends complete headers but stalls the body must be
    /// answered with 408 and disconnected once the request bound elapses,
    /// instead of pinning the connection forever.
    #[tokio::test]
    async fn test_request_timeout_closes_stalled_partial_body() {
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let app = HttpServer::new()
            .with_request_timeout(Duration::from_millis(300))
            .build_router();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

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
            .expect("stalled partial body was never closed (slowloris-style DoS)")
            .expect("reading stalled connection failed");
        if n > 0 {
            let head = String::from_utf8_lossy(&buf[..n]);
            assert!(head.contains("408"), "expected 408, got: {head}");
        }
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
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
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

    const VALID_HL7: &str = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5\r";

    fn headers(content_type: Option<&str>, auth: Option<&str>) -> HeaderMap {
        let mut map = HeaderMap::new();
        if let Some(content_type) = content_type {
            map.insert("content-type", content_type.parse().unwrap());
        }
        if let Some(auth) = auth {
            map.insert("authorization", auth.parse().unwrap());
        }
        map
    }

    fn hl7_headers() -> HeaderMap {
        headers(Some(CONTENT_TYPE_HL7_ER7), None)
    }

    #[tokio::test]
    async fn test_handle_message_rejects_bad_content_type() {
        let server = HttpServer::new();

        // Missing content type.
        let result =
            handle_message(State(server.clone()), HeaderMap::new(), VALID_HL7.into()).await;
        let (status, _, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // Wrong content type.
        let result = handle_message(
            State(server),
            headers(Some("application/json"), None),
            VALID_HL7.into(),
        )
        .await;
        let (status, _, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_message_rejects_spoof_content_type() {
        // Substring "hl7" must not be enough to pass the check.
        let server = HttpServer::new();
        for spoof in [
            "application/hl7-evil",
            "text/hl7",
            "x-application/hl7-v2+er7x",
            "application/json; note=hl7",
        ] {
            let result = handle_message(
                State(server.clone()),
                headers(Some(spoof), None),
                VALID_HL7.into(),
            )
            .await;
            let (status, _, _) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST, "spoof accepted: {spoof}");
        }
    }

    #[tokio::test]
    async fn test_handle_message_accepts_content_type_parameters() {
        let server = HttpServer::new();
        for ok_type in [
            CONTENT_TYPE_HL7_ER7,
            "x-application/hl7-v2+er7; charset=utf-8",
            "X-Application/HL7-V2+ER7",
        ] {
            let (status, _, _) = handle_message(
                State(server.clone()),
                headers(Some(ok_type), None),
                VALID_HL7.into(),
            )
            .await
            .unwrap();
            assert_eq!(status, StatusCode::OK, "rejected: {ok_type}");
        }
    }

    #[tokio::test]
    async fn test_handle_message_rejects_xml_content_type() {
        // The crate parses ER7 only: the XML media type must be rejected
        // at the gate instead of misparsing ER7 bodies or 400ing XML
        // bodies at the ER7 parser.
        let server = HttpServer::new();
        for (body, label) in [
            (VALID_HL7, "ER7 body"),
            ("<ADT_A01><MSH/></ADT_A01>", "XML body"),
        ] {
            let result = handle_message(
                State(server.clone()),
                headers(Some("x-application/hl7-v2+xml"), None),
                body.into(),
            )
            .await;
            let (status, _, message) = result.unwrap_err();
            assert_eq!(status, StatusCode::BAD_REQUEST, "{label}");
            assert!(
                message.contains("Invalid content type"),
                "{label}: {message}"
            );
        }
    }

    #[tokio::test]
    async fn test_handle_message_rejects_unparseable_body() {
        let server = HttpServer::new();
        let result = handle_message(State(server), hl7_headers(), "not hl7".into()).await;
        let (status, _, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handle_message_accepts_valid_message() {
        let server = HttpServer::new();
        let (status, headers, body) =
            handle_message(State(server), hl7_headers(), VALID_HL7.into())
                .await
                .unwrap();
        assert_eq!(status, StatusCode::OK);
        assert!(headers["content-type"].to_str().unwrap().contains("hl7"));
        let echoed = parse_message(&decode_body(&body, "").unwrap()).unwrap();
        assert_eq!(echoed.get_control_id(), Some("1"));
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

    /// Repro for finding 3: a client that stalls the TLS handshake must be
    /// disconnected instead of pinning a server task forever.
    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn test_tls_handshake_timeout_closes_stalled_connection() {
        use crate::tls::TlsServerConfig;
        use std::time::Duration;
        use tokio::io::AsyncReadExt;

        let dir = std::env::temp_dir().join(format!(
            "vyna_http_srvtls_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (_ca, cert, key) = generate_test_certs(&dir);

        let server = HttpServer::new()
            .with_tls(TlsServerConfig::new(&cert, &key).unwrap())
            .with_tls_handshake_timeout(Duration::from_millis(200));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let srv = tokio::spawn(async move {
            server.serve_tls_on(listener, shutdown_rx).await.unwrap();
        });

        // Connect and stall: never send the TLS ClientHello.
        let mut tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut buf = [0u8; 16];
        let read = tokio::time::timeout(Duration::from_secs(5), tcp.read(&mut buf)).await;
        assert!(
            read.is_ok(),
            "stalled TLS handshake was never closed (task-exhaustion DoS)"
        );

        drop(_shutdown_tx);
        let _ = tokio::time::timeout(Duration::from_secs(5), srv).await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// A client that completes the handshake but then stalls (partial
    /// request) must be disconnected once the connection bound elapses.
    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn test_tls_connection_timeout_closes_stalled_connection() {
        use crate::tls::{TlsClientConfig, TlsServerConfig};
        use std::time::Duration;
        use tokio::io::AsyncReadExt;

        let dir = std::env::temp_dir().join(format!(
            "vyna_http_srvtls_conn_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (ca, cert, key) = generate_test_certs(&dir);

        let server = HttpServer::new()
            .with_tls(TlsServerConfig::new(&cert, &key).unwrap())
            .with_tls_connection_timeout(Duration::from_millis(300));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let srv = tokio::spawn(async move {
            server.serve_tls_on(listener, shutdown_rx).await.unwrap();
        });

        // Complete the handshake (trusting the test CA), then stall
        // without sending a request.
        let tls_client = TlsClientConfig::with_ca_cert(&ca).unwrap();
        let connector = tokio_rustls::TlsConnector::from(tls_client.config.clone());
        let tcp = tokio::net::TcpStream::connect(addr).await.unwrap();
        let server_name = rustls::pki_types::ServerName::try_from("localhost")
            .unwrap()
            .to_owned();
        let mut tls = connector.connect(server_name, tcp).await.unwrap();

        let mut buf = [0u8; 16];
        let read = tokio::time::timeout(Duration::from_secs(5), tls.read(&mut buf)).await;
        assert!(
            read.is_ok(),
            "stalled TLS connection was never closed (task-exhaustion DoS)"
        );

        drop(_shutdown_tx);
        let _ = tokio::time::timeout(Duration::from_secs(5), srv).await;
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(feature = "tls")]
    #[test]
    fn test_tls_timeout_defaults_and_overrides() {
        use std::time::Duration;

        let server = HttpServer::new();
        assert_eq!(server.tls_handshake_timeout, Duration::from_secs(10));
        assert_eq!(server.tls_connection_timeout, Duration::from_secs(300));

        let server = server
            .with_tls_handshake_timeout(Duration::from_secs(3))
            .with_tls_connection_timeout(Duration::from_secs(60));
        assert_eq!(server.tls_handshake_timeout, Duration::from_secs(3));
        assert_eq!(server.tls_connection_timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    #[cfg(feature = "auth")]
    async fn test_handle_message_requires_auth() {
        use crate::auth::encode_basic_auth;

        let server = HttpServer::new().with_auth("user".to_string(), "pass".to_string());

        // Missing Authorization header.
        let result = handle_message(State(server.clone()), hl7_headers(), VALID_HL7.into()).await;
        let (status, _, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // Wrong credentials.
        let wrong = encode_basic_auth("user", "wrong");
        let result = handle_message(
            State(server.clone()),
            headers(Some(CONTENT_TYPE_HL7_ER7), Some(&wrong)),
            VALID_HL7.into(),
        )
        .await;
        let (status, _, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // Correct credentials succeed.
        let right = encode_basic_auth("user", "pass");
        let (status, _, _) = handle_message(
            State(server),
            headers(Some(CONTENT_TYPE_HL7_ER7), Some(&right)),
            VALID_HL7.into(),
        )
        .await
        .unwrap();
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    #[cfg(feature = "auth")]
    async fn test_handle_message_auth_runs_before_content_type() {
        // Unauthenticated callers must get 401 even when the content type
        // is wrong, so they cannot probe validation without credentials.
        let server = HttpServer::new().with_auth("user".to_string(), "pass".to_string());

        // Wrong content type, no credentials: auth wins.
        let result = handle_message(
            State(server.clone()),
            headers(Some("application/json"), None),
            VALID_HL7.into(),
        )
        .await;
        let (status, _, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        // Missing content type, no credentials: still 401.
        let result = handle_message(State(server), HeaderMap::new(), VALID_HL7.into()).await;
        let (status, _, _) = result.unwrap_err();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    #[cfg(feature = "auth")]
    async fn test_unauthorized_responses_carry_www_authenticate() {
        use crate::auth::encode_basic_auth;

        // RFC 9110 section 11.6.1: a 401 response MUST carry a
        // WWW-Authenticate challenge so clients can complete auth.
        let app = HttpServer::new()
            .with_auth("user".to_string(), "pass".to_string())
            .build_router();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = reqwest::Client::new();

        // Missing credentials.
        let resp = client
            .post(format!("http://{addr}/"))
            .header("content-type", CONTENT_TYPE_HL7_ER7)
            .body(VALID_HL7)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
        let challenge = resp
            .headers()
            .get("www-authenticate")
            .expect("401 lacks WWW-Authenticate")
            .to_str()
            .unwrap()
            .to_string();
        assert!(
            challenge.starts_with("Basic"),
            "unexpected challenge: {challenge}"
        );

        // Wrong credentials.
        let wrong = encode_basic_auth("user", "wrong");
        let resp = client
            .post(format!("http://{addr}/"))
            .header("content-type", CONTENT_TYPE_HL7_ER7)
            .header("authorization", wrong)
            .body(VALID_HL7)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
        let challenge = resp
            .headers()
            .get("www-authenticate")
            .expect("401 lacks WWW-Authenticate")
            .to_str()
            .unwrap()
            .to_string();
        assert!(
            challenge.starts_with("Basic"),
            "unexpected challenge: {challenge}"
        );
    }
}
