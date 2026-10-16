//! HTTP client for sending HL7 messages

use crate::{CONTENT_TYPE_HL7_ER7, Error, Result, decode_body};
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use vyna_hl7_core::{CharacterSet, Message};
use vyna_hl7_parser::parse_message;

#[cfg(feature = "tls")]
use crate::tls::TlsClientConfig;

/// Default maximum HTTP response body size (10 MB).
///
/// Bounds how much the client buffers in `post_bytes` (via `send_message`
/// and `send_message_raw`). Override per client with
/// [`HttpClient::with_max_response_size`].
pub const DEFAULT_MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

/// Authentication configuration for the HTTP client
///
/// The `Debug` implementation redacts secrets (passwords, API keys,
/// bearer credentials) so they never leak into logs via debug formatting.
#[derive(Clone)]
pub enum AuthConfig {
    /// HTTP Basic Authentication (username, password)
    Basic(String, String),
    /// API key sent in a custom header (header_name, api_key)
    ApiKey(String, String),
    /// Bearer token sent in the Authorization header
    Bearer(String),
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Basic(username, _) => f
                .debug_tuple("Basic")
                .field(username)
                .field(&"<redacted>")
                .finish(),
            Self::ApiKey(header_name, _) => f
                .debug_tuple("ApiKey")
                .field(header_name)
                .field(&"<redacted>")
                .finish(),
            Self::Bearer(_) => f.debug_tuple("Bearer").field(&"<redacted>").finish(),
        }
    }
}

/// HTTP client for sending HL7 v2.x messages
///
/// # Example
///
/// ```no_run
/// use vyna_hl7_http::HttpClient;
/// use vyna_hl7_core::builders::adt::AdtBuilder;
/// use vyna_hl7_core::Version;
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = HttpClient::new("http://example.com/hl7")?
///     .with_auth("username".into(), "password".into())
///     .with_timeout(Duration::from_secs(30))?;
///
/// let message = AdtBuilder::a01(Version::V2_5)
///     .patient_id("12345")
///     .build()?;
///
/// let ack = client.send_message(&message).await?;
/// # Ok(())
/// # }
/// ```
pub struct HttpClient {
    endpoint: String,
    client: Client,
    auth: Option<AuthConfig>,
    #[cfg(feature = "tls")]
    tls_config: Option<TlsClientConfig>,
    http2_only: bool,
    timeout: Duration,
    max_response_size: usize,
}

impl HttpClient {
    /// Create a new HTTP client
    ///
    /// # Arguments
    /// * `endpoint` - The HTTP endpoint URL (e.g., <https://api.example.com/hl7>)
    ///
    /// # Errors
    /// Returns an error if the URL is invalid or the HTTP client cannot be created
    pub fn new(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = endpoint.into();

        // Validate URL (schemes are case-insensitive per RFC 9110 4.2.3).
        let is_http = endpoint
            .get(..7)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("http://"));
        let is_https = endpoint
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"));
        if !is_http && !is_https {
            return Err(Error::InvalidUrl(
                "URL must start with http:// or https://".to_string(),
            ));
        }
        // Reject malformed URLs eagerly so callers get InvalidUrl here
        // instead of a network error at send time.
        reqwest::Url::parse(&endpoint)
            .map_err(|e| Error::InvalidUrl(format!("Invalid endpoint URL: {e}")))?;

        let timeout = Duration::from_secs(30);
        let client = ClientBuilder::new().timeout(timeout).build()?;

        Ok(Self {
            endpoint,
            client,
            auth: None,
            #[cfg(feature = "tls")]
            tls_config: None,
            http2_only: false,
            timeout,
            max_response_size: DEFAULT_MAX_RESPONSE_SIZE,
        })
    }

    /// Enable HTTP/2 only mode
    ///
    /// When enabled, the client will only use HTTP/2 protocol.
    /// By default, the client uses HTTP/1.1 with HTTP/2 upgrade.
    /// Any TLS configuration set via `with_tls` is preserved.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_http::HttpClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = HttpClient::new("https://example.com/hl7")?
    ///     .with_http2_only()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_http2_only(mut self) -> Result<Self> {
        self.http2_only = true;
        self.client = self.rebuild_client()?;
        Ok(self)
    }

    /// Rebuild the inner client from the stored settings so builder calls
    /// never drop each other's configuration (TLS, HTTP/2, timeout) by
    /// call order.
    fn rebuild_client(&self) -> Result<Client> {
        let mut builder = ClientBuilder::new().timeout(self.timeout);

        if self.http2_only {
            builder = builder.http2_prior_knowledge();
        }

        // reqwest downcasts the preconfigured backend to an owned
        // `rustls::ClientConfig`, so the shared Arc must be unwrapped (or
        // cloned) first — passing the Arc fails with "Unknown TLS backend".
        #[cfg(feature = "tls")]
        if let Some(ref tls_config) = self.tls_config {
            let rustls_config = std::sync::Arc::try_unwrap(tls_config.config.clone())
                .unwrap_or_else(|shared| (*shared).clone());
            builder = builder.use_preconfigured_tls(rustls_config);
        }

        Ok(builder.build()?)
    }

    /// Configure TLS/mTLS for the HTTP client
    ///
    /// The HTTP/2-only flag set via `with_http2_only` is preserved.
    ///
    /// # Arguments
    /// * `tls_config` - TLS configuration including optional client certificates
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # {
    /// use vyna_hl7_http::{HttpClient, tls::TlsClientConfig};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tls_config = TlsClientConfig::with_mtls(
    ///     "ca-cert.pem",
    ///     "client-cert.pem",
    ///     "client-key.pem"
    /// )?;
    ///
    /// let client = HttpClient::new("https://example.com/hl7")?
    ///     .with_tls(tls_config)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, tls_config: TlsClientConfig) -> Result<Self> {
        self.tls_config = Some(tls_config);
        self.client = self.rebuild_client()?;
        Ok(self)
    }

    /// Set HTTP Basic Authentication credentials
    ///
    /// # Arguments
    /// * `username` - Username for authentication
    /// * `password` - Password for authentication
    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.auth = Some(AuthConfig::Basic(username, password));
        self
    }

    /// Set API key authentication via a custom header
    ///
    /// The API key will be sent as-is in the specified header on every request.
    ///
    /// # Arguments
    /// * `header_name` - The header name (e.g., "X-API-Key")
    /// * `api_key` - The API key value
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_http::HttpClient;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = HttpClient::new("https://api.example.com/hl7")?
    ///     .with_api_key("X-API-Key", "my-secret-key");
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_api_key(
        mut self,
        header_name: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        self.auth = Some(AuthConfig::ApiKey(header_name.into(), api_key.into()));
        self
    }

    /// Set Bearer token authentication
    ///
    /// The token will be sent in the `Authorization: Bearer <token>` header.
    ///
    /// # Arguments
    /// * `token` - The bearer token value
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_http::HttpClient;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = HttpClient::new("https://api.example.com/hl7")?
    ///     .with_bearer_token("my-jwt-or-api-token");
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(AuthConfig::Bearer(token.into()));
        self
    }

    /// Set request timeout
    ///
    /// # Arguments
    /// * `timeout` - Timeout duration for HTTP requests
    ///
    /// # Note
    /// This method rebuilds the HTTP client while preserving existing configuration
    /// (TLS, HTTP/2, etc.) to avoid losing settings.
    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self> {
        self.timeout = timeout;
        self.client = self.rebuild_client()?;
        Ok(self)
    }

    /// Set the maximum response body size in bytes
    ///
    /// Responses larger than this are rejected with
    /// [`Error::ResponseTooLarge`] before buffering completes, so a
    /// malicious or malfunctioning server cannot exhaust client memory.
    /// The declared `Content-Length` is checked first, then the body is
    /// streamed with the same cap in case the header lies or is absent.
    ///
    /// Defaults to [`DEFAULT_MAX_RESPONSE_SIZE`] (10 MB).
    ///
    /// # Arguments
    /// * `size` - Maximum response body size in bytes
    pub fn with_max_response_size(mut self, size: usize) -> Self {
        self.max_response_size = size;
        self
    }

    /// Send an HL7 message and receive the response
    ///
    /// # Arguments
    /// * `message` - The HL7 message to send
    ///
    /// # Returns
    /// The response message (typically an ACK)
    ///
    /// # Errors
    /// Returns an error if:
    /// - The network request fails
    /// - The server returns a non-2xx status code
    /// - The response content type is invalid
    /// - The response cannot be parsed as an HL7 message
    pub async fn send_message(&self, message: &Message) -> Result<Message> {
        let charset = message.get_character_set()?;
        let body = charset.encode(&message.encode())?;
        let content_type = format!("{}; charset={}", CONTENT_TYPE_HL7_ER7, charset.iana_name());
        let ack_text = self.post_bytes(&body, &content_type).await?;
        Ok(parse_message(&ack_text)?)
    }

    /// Send an HL7 message as a raw string
    ///
    /// This is a lower-level method that sends the HL7 message text directly
    /// and returns the raw response text. The text is encoded using the
    /// character set declared in its MSH-18, defaulting to UTF-8.
    ///
    /// # Arguments
    /// * `hl7_text` - The HL7 message as a string
    ///
    /// # Returns
    /// The response message text
    ///
    /// # Errors
    /// Returns an error if the request fails or the response is invalid
    pub async fn send_message_raw(&self, hl7_text: &str) -> Result<String> {
        let charset = CharacterSet::detect(hl7_text.as_bytes())?;
        let body = charset.encode(hl7_text)?;
        let content_type = format!("{}; charset={}", CONTENT_TYPE_HL7_ER7, charset.iana_name());
        self.post_bytes(&body, &content_type).await
    }

    /// Read an error response body for diagnostics, bounded by `cap`.
    ///
    /// Returns the body as lossy text plus whether it was truncated. A
    /// mid-body stream failure yields whatever was read so far rather
    /// than masking the status error with a transport error.
    async fn read_error_body(response: &mut reqwest::Response, cap: usize) -> (String, bool) {
        let mut body = Vec::new();
        let mut truncated = false;
        while let Ok(Some(chunk)) = response.chunk().await {
            if body.len() >= cap {
                truncated = true;
                break;
            }
            let take = chunk.len().min(cap - body.len());
            body.extend_from_slice(&chunk[..take]);
            if take < chunk.len() {
                truncated = true;
                break;
            }
        }
        (String::from_utf8_lossy(&body).into_owned(), truncated)
    }

    /// POST pre-encoded bytes and decode the response
    ///
    /// The response is decoded using its Content-Type `charset` parameter,
    /// falling back to MSH-18 detection when the parameter is absent.
    /// Non-2xx responses carry the bounded response body in the error.
    async fn post_bytes(&self, body: &[u8], content_type: &str) -> Result<String> {
        // Build request
        let mut request = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", content_type)
            .header("Date", chrono::Utc::now().to_rfc2822())
            .body(body.to_vec());

        // Add authentication if configured
        match &self.auth {
            Some(AuthConfig::Basic(username, password)) => {
                request = request.basic_auth(username, Some(password));
            }
            Some(AuthConfig::ApiKey(header_name, api_key)) => {
                request = request.header(header_name.as_str(), api_key.as_str());
            }
            Some(AuthConfig::Bearer(token)) => {
                request = request.bearer_auth(token);
            }
            None => {}
        }

        // Send request
        let mut response = request.send().await?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            let reason = status.canonical_reason().unwrap_or("Unknown");
            let (detail, truncated) =
                Self::read_error_body(&mut response, self.max_response_size).await;
            let mut message = format!("HTTP {} {}", status.as_u16(), reason);
            if !detail.is_empty() {
                message.push_str(": ");
                message.push_str(&detail);
                if truncated {
                    message.push_str("... (truncated)");
                }
            }
            return Err(Error::Http(message));
        }

        // Verify content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if !crate::server::is_hl7_content_type(&content_type) {
            return Err(Error::ContentType {
                expected: CONTENT_TYPE_HL7_ER7.to_string(),
                actual: content_type.clone(),
            });
        }

        // Reject over-cap responses up front when they declare it.
        if let Some(declared) = response.content_length()
            && declared > self.max_response_size as u64
        {
            return Err(Error::ResponseTooLarge {
                limit: self.max_response_size,
            });
        }

        // Stream the body so a missing or lying Content-Length cannot
        // exhaust memory via a single unbounded read.
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            body.extend_from_slice(&chunk);
            if body.len() > self.max_response_size {
                return Err(Error::ResponseTooLarge {
                    limit: self.max_response_size,
                });
            }
        }
        Ok(decode_body(&body, &content_type)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client_valid_url() {
        assert!(HttpClient::new("http://example.com").is_ok());
        assert!(HttpClient::new("https://example.com").is_ok());
        assert!(HttpClient::new("https://example.com/path").is_ok());
    }

    #[test]
    fn test_new_client_invalid_url() {
        assert!(HttpClient::new("ftp://example.com").is_err());
        assert!(HttpClient::new("example.com").is_err());
        assert!(HttpClient::new("").is_err());
    }

    #[test]
    fn test_new_client_accepts_uppercase_scheme() {
        // URL schemes are case-insensitive (RFC 9110 section 4.2.3).
        for endpoint in [
            "HTTP://example.com/hl7",
            "HTTPS://example.com/hl7",
            "HtTp://example.com/hl7",
            "hTtPs://example.com/hl7",
        ] {
            assert!(HttpClient::new(endpoint).is_ok(), "{endpoint:?} rejected");
        }
        // Non-http(s) schemes stay rejected regardless of case.
        for endpoint in [
            "FTP://example.com",
            "ftp://example.com",
            "httpx://example.com",
            "gopher://example.com",
        ] {
            let err = HttpClient::new(endpoint)
                .err()
                .unwrap_or_else(|| panic!("{endpoint:?} accepted"));
            assert!(
                matches!(err, Error::InvalidUrl(_)),
                "unexpected error for {endpoint:?}: {err:?}"
            );
        }
    }

    #[test]
    fn test_new_client_rejects_malformed_url() {
        // Well-formed scheme but unparseable remainder must fail here,
        // not later at send time with a network error.
        for endpoint in ["http://[::1", "https://", "http://exa mple.com"] {
            let err = HttpClient::new(endpoint)
                .err()
                .unwrap_or_else(|| panic!("malformed URL accepted: {endpoint:?}"));
            assert!(
                matches!(err, Error::InvalidUrl(_)),
                "unexpected error for {endpoint:?}: {err:?}"
            );
        }
    }

    #[test]
    fn test_with_auth() {
        let client = HttpClient::new("http://example.com")
            .unwrap()
            .with_auth("user".to_string(), "pass".to_string());

        assert!(
            matches!(client.auth, Some(AuthConfig::Basic(ref u, ref p)) if u == "user" && p == "pass")
        );
    }

    #[test]
    fn test_with_api_key() {
        let client = HttpClient::new("http://example.com")
            .unwrap()
            .with_api_key("X-API-Key", "secret-123");

        assert!(matches!(
            client.auth,
            Some(AuthConfig::ApiKey(ref h, ref k)) if h == "X-API-Key" && k == "secret-123"
        ));
    }

    #[test]
    fn test_with_bearer_token() {
        let client = HttpClient::new("http://example.com")
            .unwrap()
            .with_bearer_token("my-token");

        assert!(matches!(
            client.auth,
            Some(AuthConfig::Bearer(ref t)) if t == "my-token"
        ));
    }

    #[test]
    fn test_auth_override() {
        // Setting a new auth method should replace the previous one
        let client = HttpClient::new("http://example.com")
            .unwrap()
            .with_auth("user".to_string(), "pass".to_string())
            .with_api_key("X-API-Key", "key-456");

        assert!(matches!(client.auth, Some(AuthConfig::ApiKey(_, _))));
    }

    #[test]
    fn test_auth_config_debug_redacts_secrets() {
        let basic = format!("{:?}", AuthConfig::Basic("user".into(), "s3cr3t".into()));
        assert!(basic.contains("user"));
        assert!(!basic.contains("s3cr3t"));

        let api_key = format!(
            "{:?}",
            AuthConfig::ApiKey("X-API-Key".into(), "key-456".into())
        );
        assert!(api_key.contains("X-API-Key"));
        assert!(!api_key.contains("key-456"));

        let bearer = format!("{:?}", AuthConfig::Bearer("jwt-value".into()));
        assert!(!bearer.contains("jwt-value"));

        for rendered in [&basic, &api_key, &bearer] {
            assert!(rendered.contains("<redacted>"), "{rendered}");
        }
    }

    #[test]
    fn test_max_response_size_default_and_override() {
        let client = HttpClient::new("http://example.com").unwrap();
        assert_eq!(client.max_response_size, DEFAULT_MAX_RESPONSE_SIZE);

        let client = client.with_max_response_size(1024);
        assert_eq!(client.max_response_size, 1024);
    }

    #[test]
    fn test_timeout_default_and_override() {
        let client = HttpClient::new("http://example.com").unwrap();
        assert_eq!(client.timeout, Duration::from_secs(30));

        let client = client.with_timeout(Duration::from_secs(7)).unwrap();
        assert_eq!(client.timeout, Duration::from_secs(7));
    }

    #[test]
    fn test_http2_only_preserves_custom_timeout() {
        // Timeout set before the rebuild must survive it ...
        let client = HttpClient::new("http://example.com")
            .unwrap()
            .with_timeout(Duration::from_secs(5))
            .unwrap()
            .with_http2_only()
            .unwrap();
        assert_eq!(client.timeout, Duration::from_secs(5));
        assert!(client.http2_only);

        // ... and setting the timeout last must win regardless of order.
        let client = HttpClient::new("http://example.com")
            .unwrap()
            .with_http2_only()
            .unwrap()
            .with_timeout(Duration::from_secs(9))
            .unwrap();
        assert_eq!(client.timeout, Duration::from_secs(9));
        assert!(client.http2_only);
    }

    #[cfg(feature = "tls")]
    #[test]
    fn test_tls_preserves_custom_timeout() {
        use crate::tls::TlsClientConfig;

        let client = HttpClient::new("https://example.com")
            .unwrap()
            .with_timeout(Duration::from_secs(5))
            .unwrap()
            .with_tls(TlsClientConfig::new().unwrap())
            .unwrap();
        assert_eq!(client.timeout, Duration::from_secs(5));
    }

    #[tokio::test]
    async fn test_spoof_response_content_type_rejected() {
        use axum::{Router, routing::post};

        async fn spoof() -> impl axum::response::IntoResponse {
            ([("content-type", "application/hl7-evil")], "MSH|^~\\&|A\r")
        }

        async fn valid_with_charset() -> impl axum::response::IntoResponse {
            (
                [("content-type", "x-application/hl7-v2+er7; charset=utf-8")],
                "MSH|^~\\&|A\r",
            )
        }

        let app = Router::new()
            .route("/spoof", post(spoof))
            .route("/valid", post(valid_with_charset));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // A substring "hl7" must not be enough to pass the response gate.
        let spoofed = HttpClient::new(format!("http://{addr}/spoof")).unwrap();
        let err = spoofed.send_message_raw("MSH|^~\\&|A\r").await.unwrap_err();
        assert!(
            matches!(err, Error::ContentType { .. }),
            "spoof type accepted: {err:?}"
        );

        // The exact media type with parameters is still accepted.
        let valid = HttpClient::new(format!("http://{addr}/valid")).unwrap();
        let body = valid.send_message_raw("MSH|^~\\&|A\r").await.unwrap();
        assert_eq!(body, "MSH|^~\\&|A\r");
    }

    #[tokio::test]
    async fn test_error_response_carries_diagnostic_body() {
        use axum::{Router, http::StatusCode, routing::post};

        async fn bad_request() -> impl axum::response::IntoResponse {
            (
                StatusCode::BAD_REQUEST,
                "Failed to parse HL7 message: bad MSH",
            )
        }

        let app = Router::new().route("/", post(bad_request));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = HttpClient::new(format!("http://{addr}")).unwrap();
        let err = client.send_message_raw("MSH|^~\\&|A\r").await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("400"), "status lost: {msg}");
        assert!(
            msg.contains("Failed to parse HL7 message: bad MSH"),
            "diagnostic body discarded: {msg}"
        );
    }

    #[tokio::test]
    async fn test_error_response_body_truncated_to_cap() {
        use axum::{Router, http::StatusCode, routing::post};

        let big_detail = "E".repeat(2048);
        let app = Router::new().route(
            "/",
            post(move || {
                let big_detail = big_detail.clone();
                async move { (StatusCode::INTERNAL_SERVER_ERROR, big_detail) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = HttpClient::new(format!("http://{addr}"))
            .unwrap()
            .with_max_response_size(64);
        let err = client.send_message_raw("MSH|^~\\&|A\r").await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("500"), "status lost: {msg}");
        assert!(msg.contains("truncated"), "no truncation marker: {msg}");
        assert!(
            msg.len() < 2048,
            "error body not bounded to the cap: {} bytes",
            msg.len()
        );
    }

    #[tokio::test]
    async fn test_response_size_limit_enforced() {
        use axum::{Router, routing::post};

        let big_body = "X".repeat(2048);
        let app = Router::new().route(
            "/",
            post(move || {
                let big_body = big_body.clone();
                async move { ([("content-type", crate::CONTENT_TYPE_HL7_ER7)], big_body) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // A 2 KiB body exceeds a 1 KiB cap.
        let capped = HttpClient::new(format!("http://{addr}"))
            .unwrap()
            .with_max_response_size(1024);
        let err = capped.send_message_raw("MSH|^~\\&|A\r").await.unwrap_err();
        assert!(
            matches!(err, Error::ResponseTooLarge { limit: 1024 }),
            "unexpected error: {err:?}"
        );

        // The same body passes under the default cap.
        let default = HttpClient::new(format!("http://{addr}")).unwrap();
        let body = default.send_message_raw("MSH|^~\\&|A\r").await.unwrap();
        assert_eq!(body.len(), 2048);
    }

    /// Generate a CA plus a server certificate signed by it (requires
    /// openssl), mirroring the proven flow in `testing.rs`. Returns
    /// `(ca_cert, server_cert, server_key)`.
    #[cfg(feature = "tls")]
    fn generate_probe_certs(dir: &std::path::Path) -> (String, String, String) {
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
            "/CN=Probe CA",
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

    /// What the probe TLS server observed for one connection.
    #[cfg(feature = "tls")]
    #[derive(Debug)]
    enum ProbeOutcome {
        /// The TLS handshake failed (e.g. the client did not trust us).
        HandshakeFailed,
        /// The handshake succeeded; the client's first application bytes
        /// (`PRI * HTTP/2.0...` for HTTP/2 prior knowledge, `POST ...`
        /// for HTTP/1.1).
        FirstBytes(Vec<u8>),
    }

    /// Run one client built by `build` against a probe TLS server and report
    /// what the server observed. The client's request always fails (the probe
    /// answers no HTTP); assertions are on the server-side observation.
    #[cfg(feature = "tls")]
    async fn probe_client(
        build: impl FnOnce(String) -> HttpClient,
        cert: &str,
        key: &str,
    ) -> ProbeOutcome {
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
        use tokio::io::AsyncReadExt;

        let certs: Vec<CertificateDer<'static>> =
            CertificateDer::pem_slice_iter(&std::fs::read(cert).unwrap())
                .map(|r| r.unwrap().into_owned())
                .collect();
        let key = PrivateKeyDer::pem_slice_iter(&std::fs::read(key).unwrap())
            .next()
            .unwrap()
            .unwrap()
            .clone_key();
        let config = crate::tls::server_config_builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(config));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let client = build(format!("https://{addr}"));

        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let accepted =
                tokio::time::timeout(Duration::from_secs(10), acceptor.accept(tcp)).await;
            let mut tls = match accepted {
                Ok(Ok(tls)) => tls,
                _ => return ProbeOutcome::HandshakeFailed,
            };
            let mut buf = [0u8; 64];
            match tokio::time::timeout(Duration::from_secs(10), tls.read(&mut buf)).await {
                Ok(Ok(n)) => ProbeOutcome::FirstBytes(buf[..n].to_vec()),
                _ => ProbeOutcome::FirstBytes(Vec::new()),
            }
        });

        let _ = tokio::time::timeout(
            Duration::from_secs(15),
            client.send_message_raw("MSH|^~\\&|A\r"),
        )
        .await;
        server.await.unwrap()
    }

    /// Repro for finding 2: `with_tls` must not silently reset a custom
    /// timeout. The probe completes the handshake, then closes without
    /// answering; only a client that kept its 300 ms timeout reports a
    /// timeout error.
    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn with_tls_preserves_custom_timeout() {
        use crate::tls::TlsClientConfig;
        use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
        use tokio::io::AsyncReadExt;

        let dir = std::env::temp_dir().join(format!(
            "vyna_http_timeout_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (ca_cert, srv_cert, srv_key) = generate_probe_certs(&dir);

        let certs: Vec<CertificateDer<'static>> =
            CertificateDer::pem_slice_iter(&std::fs::read(&srv_cert).unwrap())
                .map(|r| r.unwrap().into_owned())
                .collect();
        let key = PrivateKeyDer::pem_slice_iter(&std::fs::read(&srv_key).unwrap())
            .next()
            .unwrap()
            .unwrap()
            .clone_key();
        let config = crate::tls::server_config_builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(std::sync::Arc::new(config));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut tls = acceptor.accept(tcp).await.unwrap();
            // Read (part of) the request, then stall past the client's
            // custom timeout and close without answering.
            let mut buf = [0u8; 4096];
            let _ = tokio::time::timeout(Duration::from_secs(10), tls.read(&mut buf)).await;
            tokio::time::sleep(Duration::from_millis(1500)).await;
        });

        let client = HttpClient::new(format!("https://{addr}"))
            .unwrap()
            .with_timeout(Duration::from_millis(300))
            .unwrap()
            .with_tls(TlsClientConfig::with_ca_cert(&ca_cert).unwrap())
            .unwrap();
        let err = client.send_message_raw("MSH|^~\\&|A\r").await.unwrap_err();
        assert!(
            matches!(&err, Error::Network(e) if e.is_timeout()),
            "custom timeout was lost across with_tls: {err:?}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(feature = "tls")]
    #[tokio::test]
    async fn tls_and_http2_builder_calls_preserve_each_other() {
        use crate::tls::TlsClientConfig;

        let dir = std::env::temp_dir().join(format!(
            "vyna_http_probe_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (ca_cert, srv_cert, srv_key) = generate_probe_certs(&dir);

        // `with_tls` after `with_http2_only` must keep HTTP/2 enabled:
        // the probe must see the HTTP/2 preface, not an HTTP/1.1 request.
        let outcome = probe_client(
            |url| {
                HttpClient::new(url)
                    .unwrap()
                    .with_http2_only()
                    .unwrap()
                    .with_tls(TlsClientConfig::with_ca_cert(&ca_cert).unwrap())
                    .unwrap()
            },
            &srv_cert,
            &srv_key,
        )
        .await;
        assert!(
            matches!(
                outcome,
                ProbeOutcome::FirstBytes(ref b) if b.starts_with(b"PRI * HTTP/2.0")
            ),
            "`with_tls` dropped the HTTP/2 setting: {outcome:?}"
        );

        // `with_http2_only` after `with_tls` must keep the pinned CA: the
        // probe's certificate must verify, so the handshake must succeed
        // (it fails when the client falls back to system roots).
        let outcome = probe_client(
            |url| {
                HttpClient::new(url)
                    .unwrap()
                    .with_tls(TlsClientConfig::with_ca_cert(&ca_cert).unwrap())
                    .unwrap()
                    .with_http2_only()
                    .unwrap()
            },
            &srv_cert,
            &srv_key,
        )
        .await;
        assert!(
            matches!(outcome, ProbeOutcome::FirstBytes(_)),
            "`with_http2_only` dropped the pinned TLS config: {outcome:?}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[tokio::test]
    async fn test_xml_response_content_type_rejected() {
        use axum::{Router, routing::post};

        async fn xml() -> impl axum::response::IntoResponse {
            (
                [("content-type", "x-application/hl7-v2+xml")],
                "MSH|^~\\&|A\r",
            )
        }

        let app = Router::new().route("/", post(xml));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        // The crate only speaks ER7: an XML media type must be rejected
        // even though the payload itself would parse.
        let client = HttpClient::new(format!("http://{addr}")).unwrap();
        let err = client.send_message_raw("MSH|^~\\&|A\r").await.unwrap_err();
        assert!(
            matches!(err, Error::ContentType { .. }),
            "XML type accepted: {err:?}"
        );
    }
}
