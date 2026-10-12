//! Error types for custom Z-segment operations

use thiserror::Error;

/// Result type for custom segment operations
pub type Result<T> = std::result::Result<T, CustomSegmentError>;

/// Errors that can occur when working with custom Z-segments
#[derive(Error, Debug)]
pub enum CustomSegmentError {
    /// Segment ID is invalid (must start with 'Z')
    #[error("Invalid segment ID '{0}': must start with 'Z'")]
    InvalidSegmentId(String),

    /// Required field is missing
    #[error("Missing required field {field} in segment {segment}")]
    MissingField { field: String, segment: String },

    /// Field value is invalid
    #[error("Invalid value for field {field} in segment {segment}: {reason}")]
    InvalidFieldValue {
        field: String,
        segment: String,
        reason: String,
    },

    /// Segment is already registered
    #[error("Segment '{0}' is already registered")]
    DuplicateRegistration(String),

    /// Segment validation failed
    #[error("Validation failed for segment {segment}: {reason}")]
    ValidationFailed { segment: String, reason: String },

    /// Failed to parse segment from HL7 structure
    #[error("Failed to parse {segment_id}: {reason}")]
    ParseError {
        segment_id: String,
        reason: String,
    },

    /// Failed to convert segment to HL7 structure
    #[error("Failed to encode {segment_id}: {reason}")]
    EncodeError {
        segment_id: String,
        reason: String,
    },

    /// Core library error
    #[error("VynaHL7 core error: {0}")]
    CoreError(#[from] vyna_hl7_core::error::Error),

    /// Parser library error
    #[error("VynaHL7 parser error: {0}")]
    ParserError(String),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl CustomSegmentError {
    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>, segment: impl Into<String>) -> Self {
        CustomSegmentError::MissingField {
            field: field.into(),
            segment: segment.into(),
        }
    }

    /// Create an invalid field value error
    pub fn invalid_field(
        field: impl Into<String>,
        segment: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        CustomSegmentError::InvalidFieldValue {
            field: field.into(),
            segment: segment.into(),
            reason: reason.into(),
        }
    }

    /// Create a validation failed error
    pub fn validation_failed(segment: impl Into<String>, reason: impl Into<String>) -> Self {
        CustomSegmentError::ValidationFailed {
            segment: segment.into(),
            reason: reason.into(),
        }
    }

    /// Create a parse error
    pub fn parse_error(segment_id: impl Into<String>, reason: impl Into<String>) -> Self {
        CustomSegmentError::ParseError {
            segment_id: segment_id.into(),
            reason: reason.into(),
        }
    }

    /// Create an encode error
    pub fn encode_error(segment_id: impl Into<String>, reason: impl Into<String>) -> Self {
        CustomSegmentError::EncodeError {
            segment_id: segment_id.into(),
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_field_error() {
        let err = CustomSegmentError::missing_field("ZPV-1", "ZPV");
        assert!(err.to_string().contains("Missing required field"));
    }

    #[test]
    fn test_invalid_field_error() {
        let err = CustomSegmentError::invalid_field("ZPV-3", "ZPV", "Invalid format");
        assert!(err.to_string().contains("Invalid value"));
    }

    #[test]
    fn test_validation_failed_error() {
        let err = CustomSegmentError::validation_failed("ZPV", "Balance cannot be negative");
        assert!(err.to_string().contains("Validation failed"));
    }
}
