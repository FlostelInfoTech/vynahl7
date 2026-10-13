//! HTTP authentication utilities

#[cfg(feature = "oauth")]
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
#[cfg(feature = "oauth")]
use serde::{Deserialize, Serialize};

/// Encode credentials for HTTP Basic Authentication
///
/// # Arguments
/// * `username` - Username
/// * `password` - Password
///
/// # Returns
/// A formatted `Basic <base64-encoded-credentials>` header value
///
/// # Example
/// ```
/// use vyna_hl7_http::auth::encode_basic_auth;
/// let header = encode_basic_auth("user", "pass");
/// assert!(header.starts_with("Basic "));
/// ```
#[cfg(feature = "auth")]
pub fn encode_basic_auth(username: &str, password: &str) -> String {
    use base64::prelude::*;
    let credentials = format!("{}:{}", username, password);
    format!("Basic {}", BASE64_STANDARD.encode(credentials.as_bytes()))
}

/// Verify HTTP Basic Authentication credentials using constant-time comparison
///
/// This function uses constant-time comparison to prevent timing attacks.
///
/// # Arguments
/// * `header` - The Authorization header value
/// * `username` - Expected username
/// * `password` - Expected password
///
/// # Returns
/// `true` if credentials match, `false` otherwise
///
/// # Example
/// ```
/// use vyna_hl7_http::auth::{encode_basic_auth, verify_basic_auth};
/// let header = encode_basic_auth("user", "pass");
/// assert!(verify_basic_auth(&header, "user", "pass"));
/// assert!(!verify_basic_auth(&header, "wrong", "creds"));
/// ```
#[cfg(feature = "auth")]
pub fn verify_basic_auth(header: &str, username: &str, password: &str) -> bool {
    use base64::prelude::*;

    if let Some(encoded) = header.strip_prefix("Basic ")
        && let Ok(decoded) = BASE64_STANDARD.decode(encoded)
        && let Ok(credentials) = String::from_utf8(decoded)
    {
        let expected = format!("{}:{}", username, password);
        // Use constant-time comparison to prevent timing attacks
        return constant_time_compare(&credentials, &expected);
    }
    false
}

/// Verify API Key from request headers
///
/// Uses constant-time comparison to prevent timing attacks (same as
/// [`verify_api_key_secure`]).
///
/// # Arguments
/// * `header_value` - The API key value from the header (e.g., X-API-Key header)
/// * `valid_api_keys` - List of valid API keys
///
/// # Returns
/// `true` if the API key is valid, `false` otherwise
///
/// # Example
/// ```
/// use vyna_hl7_http::auth::verify_api_key;
/// let valid_keys = vec!["secret-key-1".to_string(), "secret-key-2".to_string()];
/// assert!(verify_api_key("secret-key-1", &valid_keys));
/// assert!(!verify_api_key("invalid-key", &valid_keys));
/// ```
#[cfg(feature = "auth")]
pub fn verify_api_key(header_value: &str, valid_api_keys: &[String]) -> bool {
    verify_api_key_secure(header_value, valid_api_keys)
}

/// Extract API key from Authorization header (Bearer token format)
///
/// # Arguments
/// * `header` - The Authorization header value
///
/// # Returns
/// The extracted API key if present and in Bearer format, None otherwise
///
/// # Example
/// ```
/// use vyna_hl7_http::auth::extract_bearer_token;
/// let header = "Bearer my-api-key-123";
/// assert_eq!(extract_bearer_token(header), Some("my-api-key-123"));
/// assert_eq!(extract_bearer_token("Basic dXNlcjpwYXNz"), None);
/// ```
#[cfg(feature = "auth")]
pub fn extract_bearer_token(header: &str) -> Option<&str> {
    header.strip_prefix("Bearer ")
}

/// Constant-time comparison for API keys to prevent timing attacks
///
/// # Arguments
/// * `a` - First string to compare
/// * `b` - Second string to compare
///
/// # Returns
/// `true` if strings are equal, `false` otherwise
///
/// # Example
/// ```
/// use vyna_hl7_http::auth::constant_time_compare;
/// assert!(constant_time_compare("secret123", "secret123"));
/// assert!(!constant_time_compare("secret123", "secret456"));
/// ```
#[cfg(feature = "auth")]
pub fn constant_time_compare(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    // Fold the length difference into the accumulator instead of
    // early-returning, so mismatched lengths take the same path.
    let mut diff = a_bytes.len() ^ b_bytes.len();
    let max_len = a_bytes.len().max(b_bytes.len());
    for i in 0..max_len {
        let x = *a_bytes.get(i).unwrap_or(&0);
        let y = *b_bytes.get(i).unwrap_or(&0);
        diff |= (x ^ y) as usize;
    }

    diff == 0
}

/// Verify API Key with constant-time comparison to prevent timing attacks
///
/// # Arguments
/// * `header_value` - The API key value from the header
/// * `valid_api_keys` - List of valid API keys
///
/// # Returns
/// `true` if the API key is valid, `false` otherwise
///
/// # Example
/// ```
/// use vyna_hl7_http::auth::verify_api_key_secure;
/// let valid_keys = vec!["secret-key-1".to_string(), "secret-key-2".to_string()];
/// assert!(verify_api_key_secure("secret-key-1", &valid_keys));
/// assert!(!verify_api_key_secure("invalid-key", &valid_keys));
/// ```
#[cfg(feature = "auth")]
pub fn verify_api_key_secure(header_value: &str, valid_api_keys: &[String]) -> bool {
    // Compare against every key and accumulate a single decision, so the
    // match position does not leak via early exit.
    let mut matched = 0u8;
    for key in valid_api_keys {
        matched |= u8::from(constant_time_compare(header_value, key));
    }
    matched != 0
}

/// Standard JWT claims structure
///
/// This represents the standard JWT claims as defined in RFC 7519.
/// Custom claims can be added by extending this structure.
#[cfg(feature = "oauth")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (typically user ID)
    pub sub: String,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// Expiration time (as UTC timestamp)
    pub exp: Option<usize>,
    /// Not before time (as UTC timestamp)
    pub nbf: Option<usize>,
    /// Issued at time (as UTC timestamp)
    pub iat: Option<usize>,
    /// JWT ID
    pub jti: Option<String>,
}

/// JWT configuration for validation
#[cfg(feature = "oauth")]
pub struct JwtConfig {
    /// The secret key for HMAC algorithms or public key for RSA/ECDSA
    pub secret: Vec<u8>,
    /// The algorithm to use for validation
    pub algorithm: Algorithm,
    /// The expected issuer (optional)
    pub issuer: Option<String>,
    /// The expected audience (optional)
    pub audience: Option<String>,
}

#[cfg(feature = "oauth")]
impl JwtConfig {
    /// Create a new JWT configuration with HS256 algorithm
    ///
    /// # Arguments
    /// * `secret` - The secret key for HS256
    ///
    /// # Example
    /// ```
    /// # #[cfg(feature = "oauth")]
    /// # {
    /// use vyna_hl7_http::auth::JwtConfig;
    ///
    /// let config = JwtConfig::new_hs256(b"my-secret-key");
    /// # }
    /// ```
    pub fn new_hs256(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
            algorithm: Algorithm::HS256,
            issuer: None,
            audience: None,
        }
    }

    /// Create a new JWT configuration with RS256 algorithm
    ///
    /// # Arguments
    /// * `public_key_pem` - The RSA public key in PEM format
    ///
    /// # Example
    /// ```no_run
    /// # #[cfg(feature = "oauth")]
    /// # {
    /// use vyna_hl7_http::auth::JwtConfig;
    ///
    /// let public_key = std::fs::read("public_key.pem").unwrap();
    /// let config = JwtConfig::new_rs256(&public_key);
    /// # }
    /// ```
    pub fn new_rs256(public_key_pem: &[u8]) -> Self {
        Self {
            secret: public_key_pem.to_vec(),
            algorithm: Algorithm::RS256,
            issuer: None,
            audience: None,
        }
    }

    /// Set the expected issuer
    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set the expected audience
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }
}

/// Verify JWT token
///
/// # Arguments
/// * `token` - The JWT token string
/// * `config` - JWT configuration including secret/public key and validation rules
///
/// # Returns
/// The decoded JWT claims if valid, error otherwise
///
/// # Example
/// ```no_run
/// # #[cfg(feature = "oauth")]
/// # {
/// use vyna_hl7_http::auth::{verify_jwt, JwtConfig};
///
/// let config = JwtConfig::new_hs256(b"my-secret-key")
///     .with_issuer("https://auth.example.com");
///
/// let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
/// match verify_jwt(token, &config) {
///     Ok(claims) => println!("User: {}", claims.sub),
///     Err(e) => eprintln!("Invalid token: {}", e),
/// }
/// # }
/// ```
#[cfg(feature = "oauth")]
pub fn verify_jwt(
    token: &str,
    config: &JwtConfig,
) -> Result<JwtClaims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(config.algorithm);

    if let Some(ref issuer) = config.issuer {
        validation.set_issuer(&[issuer]);
    }

    if let Some(ref audience) = config.audience {
        validation.set_audience(&[audience]);
    }

    let decoding_key = match config.algorithm {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            DecodingKey::from_secret(&config.secret)
        }
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
            DecodingKey::from_rsa_pem(&config.secret)?
        }
        Algorithm::ES256 | Algorithm::ES384 => DecodingKey::from_ec_pem(&config.secret)?,
        _ => return Err(jsonwebtoken::errors::ErrorKind::InvalidAlgorithm.into()),
    };

    let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

/// Extract and verify JWT from Authorization header
///
/// # Arguments
/// * `header` - The Authorization header value (should be `Bearer <token>`)
/// * `config` - JWT configuration
///
/// # Returns
/// The decoded JWT claims if valid, error otherwise
///
/// # Example
/// ```no_run
/// # #[cfg(feature = "oauth")]
/// # {
/// use vyna_hl7_http::auth::{verify_jwt_from_header, JwtConfig};
///
/// let config = JwtConfig::new_hs256(b"my-secret-key");
/// let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...";
///
/// match verify_jwt_from_header(header, &config) {
///     Ok(claims) => println!("Authenticated user: {}", claims.sub),
///     Err(e) => eprintln!("Authentication failed: {}", e),
/// }
/// # }
/// ```
#[cfg(feature = "oauth")]
pub fn verify_jwt_from_header(
    header: &str,
    config: &JwtConfig,
) -> Result<JwtClaims, Box<dyn std::error::Error + Send + Sync>> {
    let token = extract_bearer_token(header).ok_or("Invalid Authorization header format")?;

    verify_jwt(token, config).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

#[cfg(test)]
#[cfg(feature = "auth")]
mod tests {
    use super::*;

    #[test]
    fn test_encode_basic_auth() {
        let header = encode_basic_auth("user", "pass");
        assert!(header.starts_with("Basic "));
        // "user:pass" in base64 is "dXNlcjpwYXNz"
        assert_eq!(header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_verify_basic_auth_success() {
        let header = encode_basic_auth("testuser", "testpass");
        assert!(verify_basic_auth(&header, "testuser", "testpass"));
    }

    #[test]
    fn test_verify_basic_auth_wrong_password() {
        let header = encode_basic_auth("testuser", "testpass");
        assert!(!verify_basic_auth(&header, "testuser", "wrongpass"));
    }

    #[test]
    fn test_verify_basic_auth_wrong_username() {
        let header = encode_basic_auth("testuser", "testpass");
        assert!(!verify_basic_auth(&header, "wronguser", "testpass"));
    }

    #[test]
    fn test_verify_basic_auth_invalid_header() {
        assert!(!verify_basic_auth("Invalid Header", "user", "pass"));
        assert!(!verify_basic_auth("Bearer token", "user", "pass"));
        assert!(!verify_basic_auth("Basic !!invalid!!", "user", "pass"));
    }

    #[test]
    fn test_verify_api_key() {
        let valid_keys = vec!["secret-key-1".to_string(), "secret-key-2".to_string()];
        assert!(verify_api_key("secret-key-1", &valid_keys));
        assert!(verify_api_key("secret-key-2", &valid_keys));
        assert!(!verify_api_key("invalid-key", &valid_keys));
        assert!(!verify_api_key("", &valid_keys));
    }

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(
            extract_bearer_token("Bearer my-api-key"),
            Some("my-api-key")
        );
        assert_eq!(extract_bearer_token("Bearer key-123"), Some("key-123"));
        assert_eq!(extract_bearer_token("Basic xyz"), None);
        assert_eq!(extract_bearer_token("Bearer "), Some(""));
        assert_eq!(extract_bearer_token("Bearer"), None);
        assert_eq!(extract_bearer_token(""), None);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("secret123", "secret123"));
        assert!(!constant_time_compare("secret123", "secret456"));
        assert!(!constant_time_compare("short", "longer_string"));
        assert!(constant_time_compare("", ""));
    }

    #[test]
    fn test_verify_api_key_secure() {
        let valid_keys = vec![
            "very-secret-key-1".to_string(),
            "very-secret-key-2".to_string(),
        ];
        assert!(verify_api_key_secure("very-secret-key-1", &valid_keys));
        assert!(verify_api_key_secure("very-secret-key-2", &valid_keys));
        assert!(!verify_api_key_secure("invalid-key", &valid_keys));
        assert!(!verify_api_key_secure("very-secret-key-", &valid_keys));
    }

    #[test]
    fn test_verify_api_key_secure_matches_at_any_position() {
        let valid_keys = vec![
            "key-alpha".to_string(),
            "key-beta".to_string(),
            "key-gamma".to_string(),
        ];
        assert!(verify_api_key_secure("key-alpha", &valid_keys));
        assert!(verify_api_key_secure("key-beta", &valid_keys));
        assert!(verify_api_key_secure("key-gamma", &valid_keys));
        assert!(!verify_api_key_secure("key-delta", &valid_keys));
        assert!(!verify_api_key_secure("key-alpha", &[]));
        assert!(!verify_api_key_secure("", &valid_keys));
    }

    #[test]
    fn test_constant_time_compare_length_mismatch() {
        assert!(!constant_time_compare("short", "a-much-longer-key"));
        assert!(!constant_time_compare("a-much-longer-key", "short"));
        assert!(!constant_time_compare("", "nonempty"));
        assert!(!constant_time_compare("nonempty", ""));
        assert!(constant_time_compare("", ""));
        assert!(!constant_time_compare("same-length-1", "same-length-2"));
        assert!(constant_time_compare("identical", "identical"));
    }
}
