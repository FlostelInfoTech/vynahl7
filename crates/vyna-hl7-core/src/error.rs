//! Error types for HL7 message processing
//!
//! This module provides detailed error types with location context
//! for improved debugging of HL7 message processing issues.

use thiserror::Error;

/// Result type alias for vyna_hl7 operations
pub type Result<T> = std::result::Result<T, Error>;

/// Main error type for the vyna_hl7 library
#[derive(Error, Debug)]
pub enum Error {
    #[error("Parse error: {0}")]
    Parse(String),

    /// Parse error with location context
    #[error("Parse error at {location}: {message}")]
    ParseWithContext {
        message: String,
        location: Box<ErrorLocation>,
    },

    #[error("Invalid delimiter configuration: {0}")]
    InvalidDelimiters(String),

    #[error("Invalid encoding: {0}")]
    InvalidEncoding(String),

    #[error("Invalid segment: {0}")]
    InvalidSegment(String),

    #[error("Invalid field access: {0}")]
    InvalidFieldAccess(String),

    #[error("Missing required field: {0}")]
    MissingRequiredField(String),

    #[error("Validation error: {0}")]
    Validation(String),

    /// Validation error with location context
    #[error("Validation error at {location}: {message}")]
    ValidationWithContext {
        message: String,
        location: Box<ErrorLocation>,
        severity: ErrorSeverity,
    },

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),

    #[error("Message type error: {0}")]
    MessageType(String),

    #[error("Encoding error: {0}")]
    Encoding(String),

    #[error("Decoding error: {0}")]
    Decoding(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MLLP error: {0}")]
    Mllp(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Terser path error: {0}")]
    TerserPath(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Location information for errors
#[derive(Debug, Clone, Default)]
pub struct ErrorLocation {
    /// Segment ID (e.g., "PID", "OBX")
    pub segment: Option<String>,
    /// 1-based segment index in the message
    pub segment_index: Option<usize>,
    /// 1-based field number
    pub field: Option<usize>,
    /// 0-based repetition index
    pub repetition: Option<usize>,
    /// 1-based component number
    pub component: Option<usize>,
    /// 1-based subcomponent number
    pub subcomponent: Option<usize>,
    /// Line number in the source (if applicable)
    pub line: Option<usize>,
    /// Column number in the source (if applicable)
    pub column: Option<usize>,
    /// The actual value that caused the error
    pub value: Option<String>,
}

impl ErrorLocation {
    /// Create a new empty location
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the segment
    pub fn segment(mut self, id: &str) -> Self {
        self.segment = Some(id.to_string());
        self
    }

    /// Set the segment index
    pub fn segment_index(mut self, idx: usize) -> Self {
        self.segment_index = Some(idx);
        self
    }

    /// Set the field number
    pub fn field(mut self, num: usize) -> Self {
        self.field = Some(num);
        self
    }

    /// Set the repetition index
    pub fn repetition(mut self, idx: usize) -> Self {
        self.repetition = Some(idx);
        self
    }

    /// Set the component number
    pub fn component(mut self, num: usize) -> Self {
        self.component = Some(num);
        self
    }

    /// Set the subcomponent number
    pub fn subcomponent(mut self, num: usize) -> Self {
        self.subcomponent = Some(num);
        self
    }

    /// Set the line number
    pub fn line(mut self, num: usize) -> Self {
        self.line = Some(num);
        self
    }

    /// Set the column number
    pub fn column(mut self, num: usize) -> Self {
        self.column = Some(num);
        self
    }

    /// Set the error value
    pub fn value(mut self, val: &str) -> Self {
        self.value = Some(val.to_string());
        self
    }

    /// Generate a path-like string for this location
    ///
    /// Returns something like "PID-5-1" or "OBX(2)-5-1"
    pub fn to_path(&self) -> String {
        let mut path = String::new();

        if let Some(ref seg) = self.segment {
            path.push_str(seg);
            if let Some(idx) = self.segment_index {
                path.push_str(&format!("({})", idx));
            }
        }

        if let Some(field) = self.field {
            if !path.is_empty() {
                path.push('-');
            }
            path.push_str(&field.to_string());
        }

        if let Some(comp) = self.component {
            path.push('-');
            path.push_str(&comp.to_string());
        }

        if let Some(sub) = self.subcomponent {
            path.push('-');
            path.push_str(&sub.to_string());
        }

        if path.is_empty() {
            "unknown location".to_string()
        } else {
            path
        }
    }
}

impl std::fmt::Display for ErrorLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = self.to_path();

        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, "{} (line {}, column {})", path, line, col)
        } else if let Some(line) = self.line {
            write!(f, "{} (line {})", path, line)
        } else {
            write!(f, "{}", path)
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ErrorSeverity {
    /// Informational - processing continues normally
    Info,
    /// Warning - processing continues but result may be affected
    Warning,
    /// Error - processing may have failed partially
    #[default]
    Error,
    /// Fatal - processing cannot continue
    Fatal,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "INFO"),
            ErrorSeverity::Warning => write!(f, "WARNING"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Fatal => write!(f, "FATAL"),
        }
    }
}

impl Error {
    /// Create a parse error
    pub fn parse<S: Into<String>>(msg: S) -> Self {
        Error::Parse(msg.into())
    }

    /// Create a parse error with location context
    pub fn parse_at<S: Into<String>>(msg: S, location: ErrorLocation) -> Self {
        Error::ParseWithContext {
            message: msg.into(),
            location: Box::new(location),
        }
    }

    /// Create a validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a validation error with location context
    pub fn validation_at<S: Into<String>>(
        msg: S,
        location: ErrorLocation,
        severity: ErrorSeverity,
    ) -> Self {
        Error::ValidationWithContext {
            message: msg.into(),
            location: Box::new(location),
            severity,
        }
    }

    /// Create a terser path error
    pub fn terser_path<S: Into<String>>(msg: S) -> Self {
        Error::TerserPath(msg.into())
    }

    /// Get the location context if available
    pub fn location(&self) -> Option<&ErrorLocation> {
        match self {
            Error::ParseWithContext { location, .. } => Some(location.as_ref()),
            Error::ValidationWithContext { location, .. } => Some(location.as_ref()),
            _ => None,
        }
    }

    /// Get the severity if available
    pub fn severity(&self) -> Option<ErrorSeverity> {
        match self {
            Error::ValidationWithContext { severity, .. } => Some(*severity),
            Error::ParseWithContext { .. } => Some(ErrorSeverity::Error),
            _ => None,
        }
    }

    /// Add location context to an existing error
    pub fn with_location(self, location: ErrorLocation) -> Self {
        match self {
            Error::Parse(msg) => Error::ParseWithContext {
                message: msg,
                location: Box::new(location),
            },
            Error::Validation(msg) => Error::ValidationWithContext {
                message: msg,
                location: Box::new(location),
                severity: ErrorSeverity::Error,
            },
            other => other,
        }
    }
}

/// Builder for creating errors with context
#[derive(Default)]
pub struct ErrorBuilder {
    location: ErrorLocation,
    severity: ErrorSeverity,
}

impl ErrorBuilder {
    /// Create a new error builder
    pub fn new() -> Self {
        Self {
            location: ErrorLocation::new(),
            severity: ErrorSeverity::Error,
        }
    }

    /// Set the segment
    pub fn segment(mut self, id: &str) -> Self {
        self.location.segment = Some(id.to_string());
        self
    }

    /// Set the segment index
    pub fn segment_index(mut self, idx: usize) -> Self {
        self.location.segment_index = Some(idx);
        self
    }

    /// Set the field number
    pub fn field(mut self, num: usize) -> Self {
        self.location.field = Some(num);
        self
    }

    /// Set the component number
    pub fn component(mut self, num: usize) -> Self {
        self.location.component = Some(num);
        self
    }

    /// Set the value that caused the error
    pub fn value(mut self, val: &str) -> Self {
        self.location.value = Some(val.to_string());
        self
    }

    /// Set the line number
    pub fn line(mut self, num: usize) -> Self {
        self.location.line = Some(num);
        self
    }

    /// Set the severity
    pub fn severity(mut self, sev: ErrorSeverity) -> Self {
        self.severity = sev;
        self
    }

    /// Build a parse error
    pub fn parse<S: Into<String>>(self, msg: S) -> Error {
        Error::ParseWithContext {
            message: msg.into(),
            location: Box::new(self.location),
        }
    }

    /// Build a validation error
    pub fn validation<S: Into<String>>(self, msg: S) -> Error {
        Error::ValidationWithContext {
            message: msg.into(),
            location: Box::new(self.location),
            severity: self.severity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_location_display() {
        let loc = ErrorLocation::new()
            .segment("PID")
            .field(5)
            .component(1);

        assert_eq!(loc.to_path(), "PID-5-1");
        assert_eq!(format!("{}", loc), "PID-5-1");
    }

    #[test]
    fn test_error_location_with_index() {
        let loc = ErrorLocation::new()
            .segment("OBX")
            .segment_index(2)
            .field(5);

        assert_eq!(loc.to_path(), "OBX(2)-5");
    }

    #[test]
    fn test_error_location_with_line() {
        let loc = ErrorLocation::new()
            .segment("PID")
            .field(5)
            .line(10)
            .column(25);

        assert_eq!(format!("{}", loc), "PID-5 (line 10, column 25)");
    }

    #[test]
    fn test_parse_error_with_context() {
        let err = Error::parse_at(
            "Invalid date format",
            ErrorLocation::new()
                .segment("PID")
                .field(7)
                .value("invalid-date"),
        );

        assert!(matches!(err, Error::ParseWithContext { .. }));
        assert!(err.location().is_some());
        assert_eq!(err.location().unwrap().segment, Some("PID".to_string()));
    }

    #[test]
    fn test_error_builder() {
        let err = ErrorBuilder::new()
            .segment("PID")
            .field(5)
            .value("bad-value")
            .parse("Required field missing");

        assert!(matches!(err, Error::ParseWithContext { .. }));
        if let Error::ParseWithContext { location, .. } = err {
            assert_eq!(location.segment, Some("PID".to_string()));
            assert_eq!(location.field, Some(5));
            assert_eq!(location.value, Some("bad-value".to_string()));
        }
    }

    #[test]
    fn test_validation_error_with_severity() {
        let err = ErrorBuilder::new()
            .segment("OBX")
            .field(5)
            .severity(ErrorSeverity::Warning)
            .validation("Value out of range");

        assert_eq!(err.severity(), Some(ErrorSeverity::Warning));
    }

    #[test]
    fn test_error_with_location() {
        let original = Error::parse("Something failed");
        let enhanced = original.with_location(
            ErrorLocation::new().segment("MSH").field(9),
        );

        assert!(enhanced.location().is_some());
    }
}

