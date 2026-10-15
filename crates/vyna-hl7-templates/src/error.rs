//! Error types for template operations.

use thiserror::Error;

/// Result type for template operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during template operations
#[derive(Debug, Error)]
pub enum Error {
    /// Template parsing error
    #[error("Template parsing error: {0}")]
    Parse(String),

    /// Template validation error
    #[error("Template validation error: {0}")]
    Validation(String),

    /// Variable substitution error
    #[error("Variable substitution error: {0}")]
    Substitution(String),

    /// Template not found error
    #[error("Template not found: {0}")]
    NotFound(String),

    /// Template inheritance error
    #[error("Template inheritance error: {0}")]
    Inheritance(String),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_saphyr::Error),

    /// YAML serialization error
    #[error("YAML serialization error: {0}")]
    YamlSer(#[from] serde_saphyr::SerializeError),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// VynaHL7 core error
    #[error("VynaHL7 core error: {0}")]
    Core(#[from] vyna_hl7_core::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    /// Create a parse error
    pub fn parse(msg: impl Into<String>) -> Self {
        Error::Parse(msg.into())
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a substitution error
    pub fn substitution(msg: impl Into<String>) -> Self {
        Error::Substitution(msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }

    /// Create an inheritance error
    pub fn inheritance(msg: impl Into<String>) -> Self {
        Error::Inheritance(msg.into())
    }
}
