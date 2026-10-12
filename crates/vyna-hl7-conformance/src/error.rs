//! Error types for conformance profile validation

use thiserror::Error;

/// Result type for conformance operations
pub type Result<T> = std::result::Result<T, ConformanceError>;

/// Errors that can occur during conformance profile operations
#[derive(Error, Debug)]
pub enum ConformanceError {
    /// Error parsing XML conformance profile
    #[error("Failed to parse conformance profile: {0}")]
    ParseError(String),

    /// Invalid profile structure
    #[error("Invalid profile structure: {0}")]
    InvalidProfile(String),

    /// IO error reading profile file
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// XML parsing error
    #[error("XML error: {0}")]
    XmlError(#[from] quick_xml::Error),

    /// Missing required element in profile
    #[error("Missing required element: {0}")]
    MissingElement(String),

    /// Invalid usage code
    #[error("Invalid usage code: {0}")]
    InvalidUsage(String),

    /// Invalid cardinality specification
    #[error("Invalid cardinality: {0}")]
    InvalidCardinality(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Invalid predicate expression
    #[error("Invalid predicate: {0}")]
    InvalidPredicate(String),

    /// Invalid binding strength
    #[error("Invalid binding strength: {0}")]
    InvalidBindingStrength(String),
}
