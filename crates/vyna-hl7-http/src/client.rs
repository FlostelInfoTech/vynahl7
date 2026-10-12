//! HTTP client for sending HL7 messages

use crate::{decode_body, Error, Result, CONTENT_TYPE_HL7_ER7};
use reqwest::{Client, ClientBuilder};
use vyna_hl7_core::{CharacterSet, Message};
use vyna_hl7_parser::parse_message;
use std::time::Duration;

#[cfg(feature = "tls")]
use crate::tls::TlsClientConfig;

/// Authentication configuration for the HTTP client
#[derive(Clone, Debug)]
pub enum AuthConfig {
    /// HTTP Basic Authentication (username, password)
    Basic(String, String),
    /// API key sent in a custom header (header_name, api_key)
    ApiKey(String, String),
    /// Bearer token sent in the Authorization header
    Bearer(String),
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
}

impl HttpClient {
    /// Create a new HTTP client
    ///
    /// # Arguments
    /// * `endpoint` - The HTTP endpoint URL (e.g., "https://api.example.com/hl7")
    ///
    /// # Errors
    /// Returns an error if the URL is invalid or the HTTP client cannot be created
    pub fn new(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = endpoint.into();

        // Validate URL
        if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            return Err(Error::InvalidUrl(
                "URL must start with http:// or https://".to_string(),
            ));
        }

        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            endpoint,
            client,
            auth: None,
            #[cfg(feature = "tls")]
            tls_config: None,
            http2_only: false,
        })
    }

    /// Enable HTTP/2 only mode
    ///
    /// When enabled, the client will only use HTTP/2 protocol.
    /// By default, the client uses HTTP/1.1 with HTTP/2 upgrade.
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
        // Rebuild client with HTTP/2 only
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .http2_prior_knowledge()
            .build()?;

        self.client = client;
        self.http2_only = true;
        Ok(self)
    }

    /// Configure TLS/mTLS for the HTTP client
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
        // Rebuild client with TLS configuration. reqwest downcasts the
        // preconfigured backend to an owned `rustls::ClientConfig`, so the
        // shared Arc must be unwrapped (or cloned) first — passing the Arc
        // fails the build with "Unknown TLS backend".
        let rustls_config = std::sync::Arc::try_unwrap(tls_config.config.clone())
            .unwrap_or_else(|shared| (*shared).clone());
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .use_preconfigured_tls(rustls_config)
            .build()?;

        self.client = client;
        self.tls_config = Some(tls_config);
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
        let mut builder = ClientBuilder::new().timeout(timeout);

        // Preserve HTTP/2 setting
        if self.http2_only {
            builder = builder.http2_prior_knowledge();
        }

        // Preserve TLS configuration (unwrapping the shared Arc, as in
        // with_tls, or reqwest rejects the preconfigured backend).
        #[cfg(feature = "tls")]
        if let Some(ref tls_config) = self.tls_config {
            let rustls_config = std::sync::Arc::try_unwrap(tls_config.config.clone())
                .unwrap_or_else(|shared| (*shared).clone());
            builder = builder.use_preconfigured_tls(rustls_config);
        }

        self.client = builder.build()?;
        Ok(self)
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
        let content_type = format!(
            "{}; charset={}",
            CONTENT_TYPE_HL7_ER7,
            charset.iana_name()
        );
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
        let content_type = format!(
            "{}; charset={}",
            CONTENT_TYPE_HL7_ER7,
            charset.iana_name()
        );
        self.post_bytes(&body, &content_type).await
    }

    /// POST pre-encoded bytes and decode the response
    ///
    /// The response is decoded using its Content-Type `charset` parameter,
    /// falling back to MSH-18 detection when the parameter is absent.
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
        let response = request.send().await?;

        // Check status code
        let status = response.status();
        if !status.is_success() {
            return Err(Error::Http(format!(
                "HTTP {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        // Verify content type
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if !content_type.contains("hl7") {
            return Err(Error::ContentType {
                expected: CONTENT_TYPE_HL7_ER7.to_string(),
                actual: content_type.clone(),
            });
        }

        // Read and decode the response body
        let body = response.bytes().await?;
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
    fn test_with_auth() {
        let client = HttpClient::new("http://example.com")
            .unwrap()
            .with_auth("user".to_string(), "pass".to_string());

        assert!(matches!(client.auth, Some(AuthConfig::Basic(ref u, ref p)) if u == "user" && p == "pass"));
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
}
