//! Error types for HTTP transport

use thiserror::Error;

/// Result type alias for vyna-hl7-http operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for HTTP transport
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP error with status code and message
    #[error("HTTP error: {0}")]
    Http(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Auth(String),

    /// Invalid content type
    #[error("Invalid content type: expected {expected}, got {actual}")]
    ContentType {
        /// The expected HL7 content type
        expected: String,
        /// The content type actually received
        actual: String,
    },

    /// HL7 parsing error
    #[error("HL7 parsing error: {0}")]
    Parse(#[from] vyna_hl7_core::Error),

    /// Network error from reqwest
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Missing required header
    #[error("Missing required header: {0}")]
    MissingHeader(String),

    /// Response body exceeded the client's maximum response size
    #[error("Response body exceeds maximum size of {limit} bytes")]
    ResponseTooLarge {
        /// The configured maximum response size in bytes
        limit: usize,
    },
}
