//! Authentication middleware for HTTP endpoints
//!
//! This module provides middleware functions for authenticating HTTP requests
//! using various methods including Basic Auth, API Keys, and JWT tokens.

#[cfg(feature = "auth")]
use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode, header::WWW_AUTHENTICATE},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// HTTP Basic Authentication middleware state
///
/// The `Debug` implementation redacts the password so credentials never
/// leak into logs via debug formatting.
#[cfg(feature = "auth")]
#[derive(Clone)]
pub struct BasicAuthState {
    /// Expected username for HTTP Basic authentication
    pub username: String,
    /// Expected password for HTTP Basic authentication
    pub password: String,
}

#[cfg(feature = "auth")]
impl std::fmt::Debug for BasicAuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BasicAuthState")
            .field("username", &self.username)
            .field("password", &"<redacted>")
            .finish()
    }
}

#[cfg(feature = "auth")]
impl BasicAuthState {
    /// Create new basic auth state
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "auth")]
    /// # {
    /// use vyna_hl7_http::middleware::BasicAuthState;
    ///
    /// let auth = BasicAuthState::new("user", "pass");
    /// # }
    /// ```
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

/// Build a 401 response carrying the given `WWW-Authenticate` challenge
/// (RFC 9110 section 11.6.1 MUST).
#[cfg(feature = "auth")]
fn unauthorized_with_challenge(challenge: &'static str, message: String) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(WWW_AUTHENTICATE, HeaderValue::from_static(challenge));
    (StatusCode::UNAUTHORIZED, headers, message).into_response()
}

/// Basic authentication middleware
///
/// Validates HTTP Basic Authentication credentials from the Authorization header.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "auth")]
/// # {
/// use axum::{Router, routing::post};
/// use vyna_hl7_http::middleware::{basic_auth_middleware, BasicAuthState};
///
/// # async fn handler() -> &'static str { "OK" }
/// # async fn example() {
/// let auth_state = BasicAuthState::new("admin", "secret");
///
/// let app: Router = Router::new()
///     .route("/", post(handler))
///     .layer(axum::middleware::from_fn_with_state(
///         auth_state,
///         basic_auth_middleware
///     ));
/// # }
/// # }
/// ```
#[cfg(feature = "auth")]
pub async fn basic_auth_middleware(
    axum::extract::State(auth_state): axum::extract::State<BasicAuthState>,
    request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) => {
            use crate::auth::verify_basic_auth;
            if verify_basic_auth(header, &auth_state.username, &auth_state.password) {
                next.run(request).await
            } else {
                unauthorized_with_challenge(
                    "Basic realm=\"vyna-hl7\"",
                    "Invalid credentials".to_string(),
                )
            }
        }
        None => unauthorized_with_challenge(
            "Basic realm=\"vyna-hl7\"",
            "Missing Authorization header".to_string(),
        ),
    }
}

/// API Key authentication middleware state
///
/// The `Debug` implementation redacts the key values (reporting only how
/// many are configured) so secrets never leak into logs via debug
/// formatting.
#[cfg(feature = "auth")]
#[derive(Clone)]
pub struct ApiKeyAuthState {
    /// API keys accepted by the middleware
    pub valid_keys: Vec<String>,
    /// Request header carrying the API key (default: `X-API-Key`)
    pub header_name: String,
}

#[cfg(feature = "auth")]
impl std::fmt::Debug for ApiKeyAuthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiKeyAuthState")
            .field(
                "valid_keys",
                &format!("<{} redacted>", self.valid_keys.len()),
            )
            .field("header_name", &self.header_name)
            .finish()
    }
}

#[cfg(feature = "auth")]
impl ApiKeyAuthState {
    /// Create new API key auth state
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "auth")]
    /// # {
    /// use vyna_hl7_http::middleware::ApiKeyAuthState;
    ///
    /// let auth = ApiKeyAuthState::new(vec!["key1".into(), "key2".into()]);
    /// # }
    /// ```
    pub fn new(valid_keys: Vec<String>) -> Self {
        Self {
            valid_keys,
            header_name: "X-API-Key".to_string(),
        }
    }

    /// Set a custom header name (default: "X-API-Key")
    pub fn with_header_name(mut self, header_name: impl Into<String>) -> Self {
        self.header_name = header_name.into();
        self
    }
}

/// API Key authentication middleware
///
/// Validates API keys from a custom header (default: X-API-Key).
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "auth")]
/// # {
/// use axum::{Router, routing::post};
/// use vyna_hl7_http::middleware::{api_key_middleware, ApiKeyAuthState};
///
/// # async fn handler() -> &'static str { "OK" }
/// # async fn example() {
/// let auth_state = ApiKeyAuthState::new(vec!["secret-key".into()]);
///
/// let app: Router = Router::new()
///     .route("/", post(handler))
///     .layer(axum::middleware::from_fn_with_state(
///         auth_state,
///         api_key_middleware
///     ));
/// # }
/// # }
/// ```
#[cfg(feature = "auth")]
pub async fn api_key_middleware(
    axum::extract::State(auth_state): axum::extract::State<ApiKeyAuthState>,
    request: Request,
    next: Next,
) -> Response {
    let api_key = request
        .headers()
        .get(&auth_state.header_name)
        .and_then(|v| v.to_str().ok());

    match api_key {
        Some(key) => {
            use crate::auth::verify_api_key_secure;
            if verify_api_key_secure(key, &auth_state.valid_keys) {
                next.run(request).await
            } else {
                unauthorized_with_challenge(
                    "ApiKey realm=\"vyna-hl7\"",
                    "Invalid API key".to_string(),
                )
            }
        }
        None => unauthorized_with_challenge(
            "ApiKey realm=\"vyna-hl7\"",
            format!("Missing {} header", auth_state.header_name),
        ),
    }
}

/// Bearer token authentication middleware (supports JWT or API keys)
///
/// Validates Bearer tokens from the Authorization header.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "auth")]
/// # {
/// use axum::{Router, routing::post};
/// use vyna_hl7_http::middleware::{bearer_token_middleware, ApiKeyAuthState};
///
/// # async fn handler() -> &'static str { "OK" }
/// # async fn example() {
/// let auth_state = ApiKeyAuthState::new(vec!["token123".into()]);
///
/// let app: Router = Router::new()
///     .route("/", post(handler))
///     .layer(axum::middleware::from_fn_with_state(
///         auth_state,
///         bearer_token_middleware
///     ));
/// # }
/// # }
/// ```
#[cfg(feature = "auth")]
pub async fn bearer_token_middleware(
    axum::extract::State(auth_state): axum::extract::State<ApiKeyAuthState>,
    request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) => {
            use crate::auth::extract_bearer_token;
            if let Some(token) = extract_bearer_token(header) {
                use crate::auth::verify_api_key_secure;
                if verify_api_key_secure(token, &auth_state.valid_keys) {
                    return next.run(request).await;
                }
            }
            unauthorized_with_challenge("Bearer realm=\"vyna-hl7\"", "Invalid token".to_string())
        }
        None => unauthorized_with_challenge(
            "Bearer realm=\"vyna-hl7\"",
            "Missing Authorization header".to_string(),
        ),
    }
}

/// JWT authentication middleware state
#[cfg(feature = "oauth")]
#[derive(Clone)]
pub struct JwtAuthState {
    /// JWT decoding/validation configuration shared with the middleware
    pub config: std::sync::Arc<crate::auth::JwtConfig>,
}

#[cfg(feature = "oauth")]
impl JwtAuthState {
    /// Create new JWT auth state
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "oauth")]
    /// # {
    /// use vyna_hl7_http::middleware::JwtAuthState;
    /// use vyna_hl7_http::auth::JwtConfig;
    /// use std::sync::Arc;
    ///
    /// let config = JwtConfig::new_hs256(b"secret");
    /// let auth = JwtAuthState::new(Arc::new(config));
    /// # }
    /// ```
    pub fn new(config: std::sync::Arc<crate::auth::JwtConfig>) -> Self {
        Self { config }
    }
}

/// JWT authentication middleware
///
/// Validates JWT tokens from the Authorization header (Bearer scheme).
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "oauth")]
/// # {
/// use axum::{Router, routing::post};
/// use vyna_hl7_http::middleware::{jwt_middleware, JwtAuthState};
/// use vyna_hl7_http::auth::JwtConfig;
/// use std::sync::Arc;
///
/// # async fn handler() -> &'static str { "OK" }
/// # async fn example() {
/// let config = JwtConfig::new_hs256(b"secret");
/// let auth_state = JwtAuthState::new(Arc::new(config));
///
/// let app: Router = Router::new()
///     .route("/", post(handler))
///     .route_layer(axum::middleware::from_fn_with_state(
///         auth_state.clone(),
///         jwt_middleware
///     ))
///     .with_state(auth_state);
/// # }
/// # }
/// ```
#[cfg(feature = "oauth")]
pub async fn jwt_middleware(
    axum::extract::State(auth_state): axum::extract::State<JwtAuthState>,
    request: Request,
    next: Next,
) -> Response {
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) => {
            use crate::auth::verify_jwt_from_header;
            match verify_jwt_from_header(header, &auth_state.config) {
                Ok(_claims) => {
                    // Could add claims to request extensions here
                    next.run(request).await
                }
                Err(_) => unauthorized_with_challenge(
                    "Bearer realm=\"vyna-hl7\"",
                    "Invalid JWT token".to_string(),
                ),
            }
        }
        None => unauthorized_with_challenge(
            "Bearer realm=\"vyna-hl7\"",
            "Missing Authorization header".to_string(),
        ),
    }
}

#[cfg(test)]
#[cfg(feature = "auth")]
mod tests {
    use super::*;

    #[test]
    fn test_basic_auth_state() {
        let auth = BasicAuthState::new("user", "pass");
        assert_eq!(auth.username, "user");
        assert_eq!(auth.password, "pass");
    }

    #[test]
    fn test_api_key_auth_state() {
        let auth = ApiKeyAuthState::new(vec!["key1".into()]).with_header_name("X-Custom-Key");
        assert_eq!(auth.valid_keys.len(), 1);
        assert_eq!(auth.header_name, "X-Custom-Key");
    }

    #[test]
    fn test_basic_auth_state_debug_redacts_password() {
        let auth = BasicAuthState::new("admin", "s3cr3t");
        let rendered = format!("{auth:?}");
        assert!(rendered.contains("admin"));
        assert!(!rendered.contains("s3cr3t"));
        assert!(rendered.contains("<redacted>"));
    }

    #[test]
    fn test_api_key_auth_state_debug_redacts_keys() {
        let auth = ApiKeyAuthState::new(vec!["key-1".into(), "key-2".into()]);
        let rendered = format!("{auth:?}");
        assert!(!rendered.contains("key-1"));
        assert!(!rendered.contains("key-2"));
        assert!(rendered.contains("X-API-Key"));
        assert!(rendered.contains("<2 redacted>"));
    }

    use axum::{Router, routing::post};

    async fn spawn_app(app: Router) -> (reqwest::Client, String) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (reqwest::Client::new(), base)
    }

    async fn handler() -> &'static str {
        "OK"
    }

    /// Assert a 401 carrying a WWW-Authenticate challenge for `scheme`
    /// (RFC 9110 section 11.6.1 MUST).
    fn require_challenge(resp: &reqwest::Response, scheme: &str) {
        assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
        let challenge = resp
            .headers()
            .get("www-authenticate")
            .expect("401 lacks WWW-Authenticate")
            .to_str()
            .unwrap()
            .to_string();
        assert!(
            challenge.starts_with(scheme),
            "unexpected challenge: {challenge}"
        );
    }

    #[tokio::test]
    async fn test_basic_auth_middleware_401_carries_www_authenticate() {
        use crate::auth::encode_basic_auth;

        let app =
            Router::new()
                .route("/", post(handler))
                .layer(axum::middleware::from_fn_with_state(
                    BasicAuthState::new("admin", "secret"),
                    basic_auth_middleware,
                ));
        let (client, base) = spawn_app(app).await;

        // Missing credentials.
        let resp = client.post(format!("{base}/")).send().await.unwrap();
        require_challenge(&resp, "Basic");

        // Wrong credentials.
        let wrong = encode_basic_auth("admin", "wrong");
        let resp = client
            .post(format!("{base}/"))
            .header("authorization", wrong)
            .send()
            .await
            .unwrap();
        require_challenge(&resp, "Basic");

        // Correct credentials still pass.
        let right = encode_basic_auth("admin", "secret");
        let resp = client
            .post(format!("{base}/"))
            .header("authorization", right)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_key_middleware_401_carries_www_authenticate() {
        let app =
            Router::new()
                .route("/", post(handler))
                .layer(axum::middleware::from_fn_with_state(
                    ApiKeyAuthState::new(vec!["secret-key".into()]),
                    api_key_middleware,
                ));
        let (client, base) = spawn_app(app).await;

        // Missing key.
        let resp = client.post(format!("{base}/")).send().await.unwrap();
        require_challenge(&resp, "ApiKey");

        // Wrong key.
        let resp = client
            .post(format!("{base}/"))
            .header("X-API-Key", "wrong-key")
            .send()
            .await
            .unwrap();
        require_challenge(&resp, "ApiKey");

        // Correct key still passes.
        let resp = client
            .post(format!("{base}/"))
            .header("X-API-Key", "secret-key")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_bearer_token_middleware_401_carries_www_authenticate() {
        let app =
            Router::new()
                .route("/", post(handler))
                .layer(axum::middleware::from_fn_with_state(
                    ApiKeyAuthState::new(vec!["token123".into()]),
                    bearer_token_middleware,
                ));
        let (client, base) = spawn_app(app).await;

        // Missing token.
        let resp = client.post(format!("{base}/")).send().await.unwrap();
        require_challenge(&resp, "Bearer");

        // Wrong token.
        let resp = client
            .post(format!("{base}/"))
            .header("authorization", "Bearer wrong")
            .send()
            .await
            .unwrap();
        require_challenge(&resp, "Bearer");

        // Correct token still passes.
        let resp = client
            .post(format!("{base}/"))
            .header("authorization", "Bearer token123")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), reqwest::StatusCode::OK);
    }

    #[tokio::test]
    #[cfg(feature = "oauth")]
    async fn test_jwt_middleware_401_carries_www_authenticate() {
        use std::sync::Arc;

        let config = Arc::new(crate::auth::JwtConfig::new_hs256(b"secret"));
        let app =
            Router::new()
                .route("/", post(handler))
                .layer(axum::middleware::from_fn_with_state(
                    JwtAuthState::new(config),
                    jwt_middleware,
                ));
        let (client, base) = spawn_app(app).await;

        // Missing token.
        let resp = client.post(format!("{base}/")).send().await.unwrap();
        require_challenge(&resp, "Bearer");

        // Malformed token.
        let resp = client
            .post(format!("{base}/"))
            .header("authorization", "Bearer not-a-jwt")
            .send()
            .await
            .unwrap();
        require_challenge(&resp, "Bearer");
    }
}
