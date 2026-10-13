//! Authentication middleware for HTTP endpoints
//!
//! This module provides middleware functions for authenticating HTTP requests
//! using various methods including Basic Auth, API Keys, and JWT tokens.

#[cfg(feature = "auth")]
use axum::{
    extract::Request,
    http::StatusCode,
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
    pub username: String,
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
                (StatusCode::UNAUTHORIZED, "Invalid credentials").into_response()
            }
        }
        None => (StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response(),
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
    pub valid_keys: Vec<String>,
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
                (StatusCode::UNAUTHORIZED, "Invalid API key").into_response()
            }
        }
        None => (
            StatusCode::UNAUTHORIZED,
            format!("Missing {} header", auth_state.header_name),
        )
            .into_response(),
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
            (StatusCode::UNAUTHORIZED, "Invalid token").into_response()
        }
        None => (StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response(),
    }
}

/// JWT authentication middleware state
#[cfg(feature = "oauth")]
#[derive(Clone)]
pub struct JwtAuthState {
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
                Err(_) => (StatusCode::UNAUTHORIZED, "Invalid JWT token").into_response(),
            }
        }
        None => (StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response(),
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
}
