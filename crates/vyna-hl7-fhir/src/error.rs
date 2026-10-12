//! Error types for FHIR conversion

use thiserror::Error;

/// Result type for FHIR conversion operations
pub type ConversionResult<T> = Result<T, ConversionError>;

/// Errors that can occur during HL7 v2 to FHIR conversion
#[derive(Debug, Error)]
pub enum ConversionError {
    /// Required segment is missing
    #[error("Required segment '{0}' not found in message")]
    MissingSegment(String),

    /// Required field is missing
    #[error("Required field '{0}' not found in segment '{1}'")]
    MissingField(String, String),

    /// Invalid field format
    #[error("Invalid format for field '{0}' in segment '{1}': {2}")]
    InvalidFormat(String, String, String),

    /// Invalid data type
    #[error("Invalid data type for field '{0}': expected {1}, got {2}")]
    InvalidDataType(String, String, String),

    /// Unsupported HL7 version
    #[error("Unsupported HL7 version: {0}")]
    UnsupportedVersion(String),

    /// Unsupported message type
    #[error("Unsupported message type: {0}^{1}")]
    UnsupportedMessageType(String, String),

    /// Invalid resource state
    #[error("Invalid resource state: {0}")]
    InvalidResourceState(String),

    /// Terser path error
    #[error("Terser error: {0}")]
    TerserError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Generic conversion error
    #[error("Conversion error: {0}")]
    Generic(String),

    /// Declarative mapping error (invalid mapping or rule failure)
    #[error("Mapping error: {0}")]
    Mapping(String),
}

// Note: vyna-hl7-terser doesn't export TerserError, so we handle errors via Result directly
