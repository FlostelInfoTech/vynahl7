//! Validation result types

/// Result of conformance profile validation
#[derive(Debug, Clone)]
pub struct ConformanceValidationResult {
    /// Whether the message is valid according to the profile
    is_valid: bool,
    /// List of validation errors
    pub errors: Vec<ConformanceValidationError>,
    /// List of validation warnings
    pub warnings: Vec<ConformanceValidationWarning>,
    /// Informational messages
    pub info: Vec<ConformanceValidationInfo>,
}

impl ConformanceValidationResult {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    /// Check if the message is valid
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Add an error
    pub fn add_error(&mut self, error: ConformanceValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: ConformanceValidationWarning) {
        self.warnings.push(warning);
    }

    /// Add an info message
    pub fn add_info(&mut self, info: ConformanceValidationInfo) {
        self.info.push(info);
    }

    /// Get total issue count
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

impl Default for ConformanceValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Conformance validation error
#[derive(Debug, Clone)]
pub struct ConformanceValidationError {
    /// Location in the message (e.g., "PID-3", "PV1")
    pub location: ValidationLocation,
    /// Type of error
    pub error_type: ConformanceErrorType,
    /// Human-readable error message
    pub message: String,
    /// Severity level
    pub severity: Severity,
    /// Profile rule that was violated (optional)
    pub rule: Option<String>,
}

impl ConformanceValidationError {
    /// Create a new validation error
    pub fn new(
        location: ValidationLocation,
        error_type: ConformanceErrorType,
        message: String,
    ) -> Self {
        Self {
            location,
            error_type,
            message,
            severity: Severity::Error,
            rule: None,
        }
    }

    /// Set the severity
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the rule reference
    pub fn with_rule(mut self, rule: String) -> Self {
        self.rule = Some(rule);
        self
    }
}

/// Conformance validation warning
#[derive(Debug, Clone)]
pub struct ConformanceValidationWarning {
    /// Location in the message
    pub location: ValidationLocation,
    /// Human-readable warning message
    pub message: String,
    /// Profile rule reference (optional)
    pub rule: Option<String>,
}

/// Conformance validation info message
#[derive(Debug, Clone)]
pub struct ConformanceValidationInfo {
    /// Location in the message
    pub location: Option<ValidationLocation>,
    /// Human-readable info message
    pub message: String,
}

/// Location within an HL7 message
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationLocation {
    /// Segment ID (e.g., "PID", "MSH")
    pub segment: String,
    /// Segment occurrence index (0-based, for repeating segments)
    pub segment_index: Option<usize>,
    /// Field position (1-based)
    pub field: Option<usize>,
    /// Component position (1-based)
    pub component: Option<usize>,
}

impl ValidationLocation {
    /// Create a new location for a segment
    pub fn segment(segment: String) -> Self {
        Self {
            segment,
            segment_index: None,
            field: None,
            component: None,
        }
    }

    /// Create a new location for a segment with index
    pub fn segment_indexed(segment: String, index: usize) -> Self {
        Self {
            segment,
            segment_index: Some(index),
            field: None,
            component: None,
        }
    }

    /// Create a new location for a field
    pub fn field(segment: String, field: usize) -> Self {
        Self {
            segment,
            segment_index: None,
            field: Some(field),
            component: None,
        }
    }

    /// Create a new location for a component
    pub fn component(segment: String, field: usize, component: usize) -> Self {
        Self {
            segment,
            segment_index: None,
            field: Some(field),
            component: Some(component),
        }
    }
}

impl std::fmt::Display for ValidationLocation {
    // Format as a human-readable string (e.g., "PID-3", "PV1(2)-2.1")
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.segment)?;

        if let Some(idx) = self.segment_index {
            write!(f, "({})", idx + 1)?; // Display as 1-based
        }

        if let Some(field) = self.field {
            write!(f, "-{}", field)?;

            if let Some(comp) = self.component {
                write!(f, ".{}", comp)?;
            }
        }

        Ok(())
    }
}

/// Type of conformance error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceErrorType {
    /// Required element is missing
    RequiredElementMissing,
    /// Required if known element is missing (warning level)
    RequiredIfKnownMissing,
    /// Element marked as "Not Used" (X) is present
    NotUsedElementPresent,
    /// Below minimum cardinality
    BelowMinimumOccurrences,
    /// Exceeds maximum cardinality
    ExceedsMaximumOccurrences,
    /// Exceeds maximum length
    ExceedsMaxLength,
    /// Invalid data type format
    InvalidDataTypeFormat,
    /// Data type mismatch
    DataTypeMismatch,
    /// Invalid code from vocabulary
    InvalidCode,
    /// Value not in value set
    ValueNotInValueSet,
    /// Character not representable in the MSH-18 character set
    UnmappableCharacter,
}

impl ConformanceErrorType {
    /// Get a human-readable description of this error type
    pub fn description(&self) -> &'static str {
        match self {
            Self::RequiredElementMissing => "Required element is missing",
            Self::RequiredIfKnownMissing => "Required if known element is missing",
            Self::NotUsedElementPresent => "Element marked as not used (X) is present",
            Self::BelowMinimumOccurrences => "Below minimum number of occurrences",
            Self::ExceedsMaximumOccurrences => "Exceeds maximum number of occurrences",
            Self::ExceedsMaxLength => "Exceeds maximum length",
            Self::InvalidDataTypeFormat => "Invalid data type format",
            Self::DataTypeMismatch => "Data type does not match profile",
            Self::InvalidCode => "Invalid code from vocabulary",
            Self::ValueNotInValueSet => "Value not in value set",
            Self::UnmappableCharacter => {
                "Character cannot be encoded in the declared character set"
            }
        }
    }
}

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational only
    Info,
    /// Warning - should be addressed but not critical
    Warning,
    /// Error - must be fixed
    Error,
}

impl Severity {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Warning => "WARNING",
            Severity::Error => "ERROR",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_location_formatting() {
        let loc = ValidationLocation::segment("PID".to_string());
        assert_eq!(loc.to_string(), "PID");

        let loc = ValidationLocation::field("PID".to_string(), 3);
        assert_eq!(loc.to_string(), "PID-3");

        let loc = ValidationLocation::component("PID".to_string(), 3, 1);
        assert_eq!(loc.to_string(), "PID-3.1");

        let loc = ValidationLocation::segment_indexed("PV1".to_string(), 1);
        assert_eq!(loc.to_string(), "PV1(2)"); // Display as 1-based
    }

    #[test]
    fn test_validation_result() {
        let mut result = ConformanceValidationResult::new();
        assert!(result.is_valid());
        assert_eq!(result.total_issues(), 0);

        result.add_error(ConformanceValidationError::new(
            ValidationLocation::field("PID".to_string(), 3),
            ConformanceErrorType::RequiredElementMissing,
            "Patient ID is required".to_string(),
        ));

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.total_issues(), 1);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Info < Severity::Warning);
        assert!(Severity::Warning < Severity::Error);
    }
}
