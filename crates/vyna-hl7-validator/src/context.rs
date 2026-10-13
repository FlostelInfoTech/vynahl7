//! Validation contexts: severity profiles for validation findings.
//!
//! A [`ValidationContext`] decides what happens with each class of finding:
//! report it as an error (failing validation), downgrade it to a warning,
//! or ignore it. Three presets cover the common cases:
//!
//! - [`ValidationContext::strict`]: the default, for HL7 specification
//!   compliance. Everything is an error, including an MSH-12 version that
//!   differs from the validated version.
//! - [`ValidationContext::standard`]: strict except the version mismatch,
//!   which is a warning. For feeds validated against a pinned version where
//!   the sender's MSH-12 may legitimately differ.
//! - [`ValidationContext::lenient`]: only structural problems (empty message,
//!   missing MSH, unparseable segments) fail. Everything else becomes a
//!   warning, for feeds that are known-messy but must still flow.
//!
//! ```no_run
//! use vyna_hl7_core::Version;
//! use vyna_hl7_validator::{Validator, ValidationContext};
//!
//! let validator = Validator::new(Version::V2_5)
//!     .with_context(ValidationContext::lenient());
//! ```
//!
//! Custom contexts start from a preset and override per-class severities
//! with the `with_*` builders. Rule-engine violations keep the severity
//! their rule declares and are unaffected by the context.

use serde::{Deserialize, Serialize};

/// What to do with a class of validation findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Record an error. The message is invalid.
    Error,
    /// Record a warning. The message stays valid.
    Warning,
    /// Drop the finding silently.
    Ignore,
}

/// Severity profile for [`Validator`](crate::Validator).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationContext {
    /// Missing required segments, fields, or components.
    pub missing_required: Severity,
    /// Malformed datatype values (bad dates, non-numeric numerics, ...).
    pub invalid_datatype: Severity,
    /// Values longer than the schema allows.
    pub invalid_length: Severity,
    /// Cardinality violations (reserved for schema cardinality checks; the
    /// rule engine reports its own cardinality findings separately).
    pub invalid_cardinality: Severity,
    /// Vocabulary, coded-value, and character-set problems.
    pub invalid_value: Severity,
    /// Empty messages, missing MSH, unparseable segments.
    pub structural: Severity,
    /// MSH-12 differing from the validator version.
    pub version_mismatch: Severity,
}

impl ValidationContext {
    /// Strict except version drift warns (see [`ValidationContext::strict`],
    /// the constructor default, for spec compliance).
    pub fn standard() -> Self {
        Self {
            missing_required: Severity::Error,
            invalid_datatype: Severity::Error,
            invalid_length: Severity::Error,
            invalid_cardinality: Severity::Error,
            invalid_value: Severity::Error,
            structural: Severity::Error,
            version_mismatch: Severity::Warning,
        }
    }

    /// Spec compliance: everything fails, including version drift. This is
    /// what [`Validator`](crate::Validator) constructors use by default.
    pub fn strict() -> Self {
        Self {
            missing_required: Severity::Error,
            invalid_datatype: Severity::Error,
            invalid_length: Severity::Error,
            invalid_cardinality: Severity::Error,
            invalid_value: Severity::Error,
            structural: Severity::Error,
            version_mismatch: Severity::Error,
        }
    }

    /// Only structural problems fail; everything else warns.
    pub fn lenient() -> Self {
        Self {
            missing_required: Severity::Warning,
            invalid_datatype: Severity::Warning,
            invalid_length: Severity::Warning,
            invalid_cardinality: Severity::Warning,
            invalid_value: Severity::Warning,
            structural: Severity::Error,
            version_mismatch: Severity::Warning,
        }
    }

    /// Override the severity for missing required items.
    pub fn with_missing_required(mut self, severity: Severity) -> Self {
        self.missing_required = severity;
        self
    }

    /// Override the severity for malformed datatype values.
    pub fn with_invalid_datatype(mut self, severity: Severity) -> Self {
        self.invalid_datatype = severity;
        self
    }

    /// Override the severity for over-length values.
    pub fn with_invalid_length(mut self, severity: Severity) -> Self {
        self.invalid_length = severity;
        self
    }

    /// Override the severity for cardinality violations.
    pub fn with_invalid_cardinality(mut self, severity: Severity) -> Self {
        self.invalid_cardinality = severity;
        self
    }

    /// Override the severity for vocabulary and coded-value problems.
    pub fn with_invalid_value(mut self, severity: Severity) -> Self {
        self.invalid_value = severity;
        self
    }

    /// Override the severity for structural problems.
    pub fn with_structural(mut self, severity: Severity) -> Self {
        self.structural = severity;
        self
    }

    /// Override the severity for version drift.
    pub fn with_version_mismatch(mut self, severity: Severity) -> Self {
        self.version_mismatch = severity;
        self
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_differ_as_documented() {
        let strict = ValidationContext::strict();
        let standard = ValidationContext::standard();
        let lenient = ValidationContext::lenient();

        assert_eq!(standard, ValidationContext::default());
        assert_eq!(strict.version_mismatch, Severity::Error);
        assert_eq!(standard.version_mismatch, Severity::Warning);
        assert_eq!(lenient.invalid_value, Severity::Warning);
        assert_eq!(lenient.invalid_length, Severity::Warning);
        assert_eq!(lenient.missing_required, Severity::Warning);
        assert_eq!(lenient.structural, Severity::Error);
    }

    #[test]
    fn builders_override_per_class() {
        let context = ValidationContext::strict()
            .with_invalid_value(Severity::Ignore)
            .with_version_mismatch(Severity::Warning);
        assert_eq!(context.invalid_value, Severity::Ignore);
        assert_eq!(context.version_mismatch, Severity::Warning);
        assert_eq!(context.invalid_length, Severity::Error);
    }
}
