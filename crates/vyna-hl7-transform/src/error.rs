//! Error types for message transformation

use thiserror::Error;

/// Result type for transformation operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during message transformation
#[derive(Debug, Error)]
pub enum Error {
    /// Field access error (from terser)
    #[error("Field access error: {0}")]
    FieldAccess(String),

    /// Transformation function error
    #[error("Transform function error: {0}")]
    TransformFn(String),

    /// Invalid transformation rule
    #[error("Invalid transformation rule: {0}")]
    InvalidRule(String),

    /// Configuration error (serde feature)
    #[cfg(feature = "serde")]
    #[error("Configuration error: {0}")]
    Config(String),

    /// YAML parsing error
    #[cfg(feature = "serde")]
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// JSON parsing error
    #[cfg(feature = "serde")]
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// VynaHL7 core error (includes terser errors)
    #[error("VynaHL7 core error: {0}")]
    Core(#[from] vyna_hl7_core::Error),
}

impl Error {
    /// Create a field access error
    pub fn field_access<S: Into<String>>(msg: S) -> Self {
        Error::FieldAccess(msg.into())
    }

    /// Create a transform function error
    pub fn transform_fn<S: Into<String>>(msg: S) -> Self {
        Error::TransformFn(msg.into())
    }

    /// Create an invalid rule error
    pub fn invalid_rule<S: Into<String>>(msg: S) -> Self {
        Error::InvalidRule(msg.into())
    }

    /// Create a configuration error (serde feature)
    #[cfg(feature = "serde")]
    pub fn config<S: Into<String>>(msg: S) -> Self {
        Error::Config(msg.into())
    }
}
