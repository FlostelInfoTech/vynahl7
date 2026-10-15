//! Declarative rule configuration from YAML/JSON
//!
//! This module provides functionality to load validation rules from YAML or JSON configuration files.
//! Rules can be defined declaratively using a simple expression language for conditions.
//!
//! ## Configuration Format
//!
//! Rules can be defined in YAML:
//!
//! ```yaml
//! rules:
//!   - name: "patient_gender_required"
//!     description: "Patient gender must be provided"
//!     severity: error
//!     condition:
//!       type: field_valued
//!       field: "PID-8"
//!
//!   - name: "male_requires_ssn"
//!     description: "Male patients must have SSN"
//!     severity: warning
//!     condition:
//!       type: if_then
//!       if_field: "PID-8"
//!       if_value: "M"
//!       then_field: "PID-19"
//!
//!   - name: "valid_patient_class"
//!     description: "Patient class must be I, O, or E"
//!     severity: error
//!     condition:
//!       type: field_in_set
//!       field: "PV1-2"
//!       values: ["I", "O", "E"]
//! ```
//!
//! Or in JSON:
//!
//! ```json
//! {
//!   "rules": [
//!     {
//!       "name": "patient_gender_required",
//!       "description": "Patient gender must be provided",
//!       "severity": "error",
//!       "condition": {
//!         "type": "field_valued",
//!         "field": "PID-8"
//!       }
//!     }
//!   ]
//! }
//! ```
//!
//! ## Supported Condition Types
//!
//! - `field_valued` - Field must have a non-empty value
//! - `if_then` - If field equals value, then another field must be valued
//! - `mutually_exclusive` - Fields cannot both be valued
//! - `at_least_one` - At least one field must be valued
//! - `all_or_none` - All fields valued or all empty
//! - `field_in_set` - Field value must be in a specified set
//! - `dependent_fields` - If primary field valued, dependent must be too

use super::{CrossFieldValidator, RuleSeverity, ValidationRule};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Error type for declarative rule parsing
#[derive(Debug, thiserror::Error)]
pub enum DeclarativeError {
    /// Rule file could not be read
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Rule file is not valid YAML
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_saphyr::Error),

    /// Rule file is not valid JSON
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// `severity` is not a severity name (`error`, `warning`, `info`, or short forms)
    #[error("Invalid severity: {0}")]
    InvalidSeverity(String),

    /// A condition is missing a key its type requires (e.g. `then_field`)
    #[error("Missing required field in condition: {0}")]
    MissingField(String),

    /// A condition `type` matches no known condition
    #[error("Unknown condition type: {0}")]
    UnknownConditionType(String),
}

/// Result type for declarative rule parsing
pub type Result<T> = std::result::Result<T, DeclarativeError>;

/// Root structure for rule configuration files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// List of validation rules
    pub rules: Vec<RuleDefinition>,
}

/// A single rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDefinition {
    /// Unique rule name
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Severity level (error, warning, info)
    pub severity: String,

    /// Rule condition
    pub condition: ConditionConfig,
}

/// Condition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConditionConfig {
    /// Field must be valued
    FieldValued {
        /// Terser path of the field that must carry a value
        field: String,
    },

    /// If-then conditional
    IfThen {
        /// Terser path of the field to test
        if_field: String,
        /// Value that triggers the requirement
        if_value: String,
        /// Terser path of the field required when triggered
        then_field: String,
    },

    /// Mutually exclusive fields
    MutuallyExclusive {
        /// Terser paths of which at most one may be valued
        fields: Vec<String>,
    },

    /// At least one field must be valued
    AtLeastOne {
        /// Terser paths of which at least one must be valued
        fields: Vec<String>,
    },

    /// All fields valued or all empty
    AllOrNone {
        /// Terser paths that must be valued together or all empty
        fields: Vec<String>,
    },

    /// Field value must be in set
    FieldInSet {
        /// Terser path of the field to test
        field: String,
        /// Allowed values
        values: Vec<String>,
    },

    /// Dependent fields
    DependentFields {
        /// Terser path of the field that requires the dependent one
        primary_field: String,
        /// Terser path of the field required when the primary is valued
        dependent_field: String,
    },
}

impl RuleDefinition {
    /// Convert this rule definition into a ValidationRule
    pub fn into_validation_rule(self) -> Result<ValidationRule> {
        let severity = RuleSeverity::parse(&self.severity)
            .ok_or_else(|| DeclarativeError::InvalidSeverity(self.severity.clone()))?;

        let rule = match self.condition {
            ConditionConfig::FieldValued { field } => {
                CrossFieldValidator::field_valued(self.name, self.description, severity, field)
            }

            ConditionConfig::IfThen {
                if_field,
                if_value,
                then_field,
            } => CrossFieldValidator::if_then(
                self.name,
                self.description,
                severity,
                if_field,
                if_value,
                then_field,
            ),

            ConditionConfig::MutuallyExclusive { fields } => {
                CrossFieldValidator::mutually_exclusive(
                    self.name,
                    self.description,
                    severity,
                    fields,
                )
            }

            ConditionConfig::AtLeastOne { fields } => {
                CrossFieldValidator::at_least_one(self.name, self.description, severity, fields)
            }

            ConditionConfig::AllOrNone { fields } => {
                CrossFieldValidator::all_or_none(self.name, self.description, severity, fields)
            }

            ConditionConfig::FieldInSet { field, values } => CrossFieldValidator::field_in_set(
                self.name,
                self.description,
                severity,
                field,
                values,
            ),

            ConditionConfig::DependentFields {
                primary_field,
                dependent_field,
            } => CrossFieldValidator::dependent_fields(
                self.name,
                self.description,
                severity,
                primary_field,
                dependent_field,
            ),
        };

        Ok(rule)
    }
}

impl RuleConfig {
    /// Load rules from a YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml_str(&content)
    }

    /// Load rules from a YAML string
    pub fn from_yaml_str(content: &str) -> Result<Self> {
        let config: RuleConfig = serde_saphyr::from_str(content)?;
        Ok(config)
    }

    /// Load rules from a JSON file
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json_str(&content)
    }

    /// Load rules from a JSON string
    pub fn from_json_str(content: &str) -> Result<Self> {
        let config: RuleConfig = serde_json::from_str(content)?;
        Ok(config)
    }

    /// Convert all rule definitions into ValidationRules
    pub fn into_validation_rules(self) -> Result<Vec<ValidationRule>> {
        self.rules
            .into_iter()
            .map(|rule_def| rule_def.into_validation_rule())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_field_valued_yaml() {
        let yaml = r#"
rules:
  - name: "gender_required"
    description: "Patient gender must be provided"
    severity: "error"
    condition:
      type: "field_valued"
      field: "PID-8"
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].name, "gender_required");
        assert_eq!(config.rules[0].severity, "error");
    }

    #[test]
    fn test_parse_if_then_yaml() {
        let yaml = r#"
rules:
  - name: "male_requires_ssn"
    description: "Male patients must have SSN"
    severity: "warning"
    condition:
      type: "if_then"
      if_field: "PID-8"
      if_value: "M"
      then_field: "PID-19"
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.rules.len(), 1);

        let rule = &config.rules[0];
        assert_eq!(rule.name, "male_requires_ssn");

        match &rule.condition {
            ConditionConfig::IfThen {
                if_field,
                if_value,
                then_field,
            } => {
                assert_eq!(if_field, "PID-8");
                assert_eq!(if_value, "M");
                assert_eq!(then_field, "PID-19");
            }
            _ => panic!("Expected IfThen condition"),
        }
    }

    #[test]
    fn test_parse_field_in_set_yaml() {
        let yaml = r#"
rules:
  - name: "valid_patient_class"
    description: "Patient class must be I, O, or E"
    severity: "error"
    condition:
      type: "field_in_set"
      field: "PV1-2"
      values: ["I", "O", "E"]
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.rules.len(), 1);

        match &config.rules[0].condition {
            ConditionConfig::FieldInSet { field, values } => {
                assert_eq!(field, "PV1-2");
                assert_eq!(values.len(), 3);
                assert!(values.contains(&"I".to_string()));
            }
            _ => panic!("Expected FieldInSet condition"),
        }
    }

    #[test]
    fn test_parse_multiple_rules_yaml() {
        let yaml = r#"
rules:
  - name: "rule1"
    description: "First rule"
    severity: "error"
    condition:
      type: "field_valued"
      field: "PID-8"

  - name: "rule2"
    description: "Second rule"
    severity: "warning"
    condition:
      type: "field_valued"
      field: "PID-19"
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].name, "rule1");
        assert_eq!(config.rules[1].name, "rule2");
    }

    #[test]
    fn test_parse_mutually_exclusive_yaml() {
        let yaml = r#"
rules:
  - name: "address_exclusive"
    description: "Home and work address are mutually exclusive"
    severity: "error"
    condition:
      type: "mutually_exclusive"
      fields: ["PID-11", "PID-12"]
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        match &config.rules[0].condition {
            ConditionConfig::MutuallyExclusive { fields } => {
                assert_eq!(fields.len(), 2);
            }
            _ => panic!("Expected MutuallyExclusive condition"),
        }
    }

    #[test]
    fn test_parse_at_least_one_yaml() {
        let yaml = r#"
rules:
  - name: "patient_id_required"
    description: "Patient must have at least one identifier"
    severity: "error"
    condition:
      type: "at_least_one"
      fields: ["PID-2", "PID-3", "PID-4"]
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        match &config.rules[0].condition {
            ConditionConfig::AtLeastOne { fields } => {
                assert_eq!(fields.len(), 3);
            }
            _ => panic!("Expected AtLeastOne condition"),
        }
    }

    #[test]
    fn test_parse_all_or_none_yaml() {
        let yaml = r#"
rules:
  - name: "complete_address"
    description: "Address must be complete or empty"
    severity: "warning"
    condition:
      type: "all_or_none"
      fields: ["PID-11-1", "PID-11-3", "PID-11-4"]
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        match &config.rules[0].condition {
            ConditionConfig::AllOrNone { fields } => {
                assert_eq!(fields.len(), 3);
            }
            _ => panic!("Expected AllOrNone condition"),
        }
    }

    #[test]
    fn test_parse_dependent_fields_yaml() {
        let yaml = r#"
rules:
  - name: "phone_requires_type"
    description: "Phone number requires phone type"
    severity: "warning"
    condition:
      type: "dependent_fields"
      primary_field: "PID-13"
      dependent_field: "PID-13-2"
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        match &config.rules[0].condition {
            ConditionConfig::DependentFields {
                primary_field,
                dependent_field,
            } => {
                assert_eq!(primary_field, "PID-13");
                assert_eq!(dependent_field, "PID-13-2");
            }
            _ => panic!("Expected DependentFields condition"),
        }
    }

    #[test]
    fn test_parse_json() {
        let json = r#"
{
  "rules": [
    {
      "name": "gender_required",
      "description": "Patient gender must be provided",
      "severity": "error",
      "condition": {
        "type": "field_valued",
        "field": "PID-8"
      }
    }
  ]
}
"#;

        let config = RuleConfig::from_json_str(json).unwrap();
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].name, "gender_required");
    }

    #[test]
    fn test_into_validation_rule() {
        let yaml = r#"
rules:
  - name: "gender_required"
    description: "Patient gender must be provided"
    severity: "error"
    condition:
      type: "field_valued"
      field: "PID-8"
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        let rules = config.into_validation_rules().unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "gender_required");
        assert_eq!(rules[0].description, "Patient gender must be provided");
        assert_eq!(rules[0].severity, RuleSeverity::Error);
    }

    #[test]
    fn test_invalid_severity() {
        let yaml = r#"
rules:
  - name: "test"
    description: "Test"
    severity: "invalid"
    condition:
      type: "field_valued"
      field: "PID-8"
"#;

        let config = RuleConfig::from_yaml_str(yaml).unwrap();
        let result = config.into_validation_rules();
        assert!(result.is_err());
    }
}
