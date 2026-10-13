//! Advanced validation rules engine
//!
//! This module provides a flexible rules engine for executing custom business validation logic
//! against HL7 messages. Rules can be defined programmatically or loaded from YAML/JSON configuration.
//!
//! ## Features
//!
//! - **Custom Business Rules**: Define rules with closures or condition expressions
//! - **Severity Levels**: Error, Warning, or Info classification
//! - **Cross-Field Validation**: Validate dependencies between multiple fields
//! - **Rule Composition**: Combine multiple rules together
//! - **Declarative Configuration**: Load rules from YAML/JSON files
//!
//! ## Example
//!
//! ```
//! use vyna_hl7_validator::rules::{ValidationRule, RulesEngine, RuleSeverity};
//! use vyna_hl7_parser::parse_message;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a rules engine
//! let mut engine = RulesEngine::new();
//!
//! // Add a custom rule
//! let rule = ValidationRule::new(
//!     "patient_gender_required",
//!     "Patient gender must be provided",
//!     RuleSeverity::Error,
//! ).with_condition(|msg| {
//!     // Check if PID-8 is valued
//!     use vyna_hl7_terser::Terser;
//!     let terser = Terser::new(msg);
//!     terser.get("PID-8").ok().flatten().map(|v| !v.trim().is_empty()).unwrap_or(false)
//! });
//!
//! engine.add_rule(rule);
//!
//! // Validate a message
//! let message = parse_message("MSH|^~\\&|...\rPID|||12345||Doe^John||M")?;
//! let result = engine.validate(&message);
//!
//! if !result.passed() {
//!     for violation in &result.violations {
//!         println!("{}: {}", violation.severity, violation.message);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod builtin;
pub mod cross_field;
pub mod declarative;

pub use builtin::BuiltinRules;
pub use cross_field::CrossFieldValidator;
pub use declarative::{RuleConfig, RuleDefinition, ConditionConfig, DeclarativeError};

use vyna_hl7_core::Message;
use std::sync::Arc;

/// Severity level for validation rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RuleSeverity {
    /// Informational message - no validation failure
    Info,
    /// Warning - validation concern but not a failure
    Warning,
    /// Error - validation failure
    Error,
}

impl RuleSeverity {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleSeverity::Info => "INFO",
            RuleSeverity::Warning => "WARNING",
            RuleSeverity::Error => "ERROR",
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "INFO" | "I" => Some(RuleSeverity::Info),
            "WARNING" | "WARN" | "W" => Some(RuleSeverity::Warning),
            "ERROR" | "ERR" | "E" => Some(RuleSeverity::Error),
            _ => None,
        }
    }
}

impl std::fmt::Display for RuleSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Result of validating a message against rules
#[derive(Debug, Clone)]
pub struct RulesValidationResult {
    /// List of rule violations
    pub violations: Vec<RuleViolation>,
}

impl RulesValidationResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }

    /// Add a violation
    pub fn add_violation(&mut self, violation: RuleViolation) {
        self.violations.push(violation);
    }

    /// Check if validation passed (no errors)
    pub fn passed(&self) -> bool {
        !self.violations.iter().any(|v| v.severity == RuleSeverity::Error)
    }

    /// Get all errors
    pub fn errors(&self) -> Vec<&RuleViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity == RuleSeverity::Error)
            .collect()
    }

    /// Get all warnings
    pub fn warnings(&self) -> Vec<&RuleViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity == RuleSeverity::Warning)
            .collect()
    }

    /// Get all info messages
    pub fn infos(&self) -> Vec<&RuleViolation> {
        self.violations
            .iter()
            .filter(|v| v.severity == RuleSeverity::Info)
            .collect()
    }
}

impl Default for RulesValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A rule violation detected during validation
#[derive(Debug, Clone)]
pub struct RuleViolation {
    /// Rule that was violated
    pub rule_name: String,
    /// Severity of the violation
    pub severity: RuleSeverity,
    /// Description of the violation
    pub message: String,
    /// Optional location in the message where violation occurred
    pub location: Option<String>,
}

impl RuleViolation {
    /// Create a new rule violation
    pub fn new(rule_name: String, severity: RuleSeverity, message: String) -> Self {
        Self {
            rule_name,
            severity,
            message,
            location: None,
        }
    }

    /// Add location to the violation
    pub fn with_location(mut self, location: String) -> Self {
        self.location = Some(location);
        self
    }
}

/// Type alias for rule condition function
pub type RuleCondition = Arc<dyn Fn(&Message) -> bool + Send + Sync>;

/// A validation rule that can be executed against a message
pub struct ValidationRule {
    /// Unique identifier for the rule
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Severity if rule is violated
    pub severity: RuleSeverity,
    /// Condition that must be true for the message to be valid
    condition: Option<RuleCondition>,
}

impl ValidationRule {
    /// Create a new validation rule
    pub fn new(name: impl Into<String>, description: impl Into<String>, severity: RuleSeverity) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            severity,
            condition: None,
        }
    }

    /// Set the condition for this rule
    ///
    /// The condition returns `true` when the message passes; a `false`
    /// result records a violation with this rule's name and severity.
    pub fn with_condition<F>(mut self, condition: F) -> Self
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
    {
        self.condition = Some(Arc::new(condition));
        self
    }

    /// Evaluate the rule against a message
    pub fn evaluate(&self, message: &Message) -> Option<RuleViolation> {
        if let Some(condition) = &self.condition
            && !condition(message) {
                return Some(RuleViolation::new(
                    self.name.clone(),
                    self.severity,
                    self.description.clone(),
                ));
            }
        None
    }
}

/// Rules engine for executing validation rules
pub struct RulesEngine {
    /// Collection of validation rules
    rules: Vec<ValidationRule>,
}

impl RulesEngine {
    /// Create a new rules engine
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rule to the engine
    pub fn add_rule(&mut self, rule: ValidationRule) {
        self.rules.push(rule);
    }

    /// Add multiple rules at once
    pub fn add_rules(&mut self, rules: Vec<ValidationRule>) {
        self.rules.extend(rules);
    }

    /// Get the number of rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Validate a message against all rules
    pub fn validate(&self, message: &Message) -> RulesValidationResult {
        let mut result = RulesValidationResult::new();

        for rule in &self.rules {
            if let Some(violation) = rule.evaluate(message) {
                result.add_violation(violation);
            }
        }

        result
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
    }
}

impl Default for RulesEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::{Field, Segment};

    fn create_test_message_with_gender() -> Message {
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msg.segments.push(msh);

        let mut pid = Segment::new("PID");
        pid.fields.push(Field::new()); // PID-1
        pid.fields.push(Field::new()); // PID-2
        pid.fields.push(Field::from_value("12345")); // PID-3
        pid.fields.push(Field::new()); // PID-4
        pid.fields.push(Field::from_value("Doe^John")); // PID-5
        pid.fields.push(Field::new()); // PID-6
        pid.fields.push(Field::from_value("19900101")); // PID-7
        pid.fields.push(Field::from_value("M")); // PID-8
        msg.segments.push(pid);

        msg
    }

    #[test]
    fn test_rule_severity_parse() {
        assert_eq!(RuleSeverity::parse("ERROR"), Some(RuleSeverity::Error));
        assert_eq!(RuleSeverity::parse("error"), Some(RuleSeverity::Error));
        assert_eq!(RuleSeverity::parse("E"), Some(RuleSeverity::Error));
        assert_eq!(RuleSeverity::parse("WARNING"), Some(RuleSeverity::Warning));
        assert_eq!(RuleSeverity::parse("W"), Some(RuleSeverity::Warning));
        assert_eq!(RuleSeverity::parse("INFO"), Some(RuleSeverity::Info));
        assert_eq!(RuleSeverity::parse("I"), Some(RuleSeverity::Info));
        assert_eq!(RuleSeverity::parse("INVALID"), None);
    }

    #[test]
    fn test_rule_severity_ordering() {
        assert!(RuleSeverity::Info < RuleSeverity::Warning);
        assert!(RuleSeverity::Warning < RuleSeverity::Error);
    }

    #[test]
    fn test_validation_rule_creation() {
        let rule = ValidationRule::new(
            "test_rule",
            "Test rule description",
            RuleSeverity::Error,
        );

        assert_eq!(rule.name, "test_rule");
        assert_eq!(rule.description, "Test rule description");
        assert_eq!(rule.severity, RuleSeverity::Error);
    }

    #[test]
    fn test_validation_rule_with_condition() {
        let rule = ValidationRule::new(
            "always_pass",
            "Rule that always passes",
            RuleSeverity::Error,
        ).with_condition(|_msg| true);

        let msg = create_test_message_with_gender();
        assert!(rule.evaluate(&msg).is_none());
    }

    #[test]
    fn test_validation_rule_with_failing_condition() {
        let rule = ValidationRule::new(
            "always_fail",
            "Rule that always fails",
            RuleSeverity::Error,
        ).with_condition(|_msg| false);

        let msg = create_test_message_with_gender();
        let violation = rule.evaluate(&msg);
        assert!(violation.is_some());

        let v = violation.unwrap();
        assert_eq!(v.rule_name, "always_fail");
        assert_eq!(v.severity, RuleSeverity::Error);
        assert_eq!(v.message, "Rule that always fails");
    }

    #[test]
    fn test_rules_engine_add_and_validate() {
        let mut engine = RulesEngine::new();
        assert_eq!(engine.rule_count(), 0);

        let rule1 = ValidationRule::new(
            "rule1",
            "First rule",
            RuleSeverity::Error,
        ).with_condition(|_| true);

        let rule2 = ValidationRule::new(
            "rule2",
            "Second rule",
            RuleSeverity::Warning,
        ).with_condition(|_| false);

        engine.add_rule(rule1);
        engine.add_rule(rule2);
        assert_eq!(engine.rule_count(), 2);

        let msg = create_test_message_with_gender();
        let result = engine.validate(&msg);

        assert_eq!(result.violations.len(), 1);
        assert_eq!(result.violations[0].rule_name, "rule2");
        assert_eq!(result.violations[0].severity, RuleSeverity::Warning);
    }

    #[test]
    fn test_rules_validation_result_passed() {
        let mut result = RulesValidationResult::new();
        assert!(result.passed());

        result.add_violation(RuleViolation::new(
            "test".to_string(),
            RuleSeverity::Warning,
            "Warning message".to_string(),
        ));
        assert!(result.passed()); // Still passes with warnings

        result.add_violation(RuleViolation::new(
            "test2".to_string(),
            RuleSeverity::Error,
            "Error message".to_string(),
        ));
        assert!(!result.passed()); // Fails with errors
    }

    #[test]
    fn test_rules_validation_result_filtering() {
        let mut result = RulesValidationResult::new();

        result.add_violation(RuleViolation::new(
            "info1".to_string(),
            RuleSeverity::Info,
            "Info".to_string(),
        ));
        result.add_violation(RuleViolation::new(
            "warn1".to_string(),
            RuleSeverity::Warning,
            "Warning".to_string(),
        ));
        result.add_violation(RuleViolation::new(
            "err1".to_string(),
            RuleSeverity::Error,
            "Error".to_string(),
        ));

        assert_eq!(result.errors().len(), 1);
        assert_eq!(result.warnings().len(), 1);
        assert_eq!(result.infos().len(), 1);
    }

    #[test]
    fn test_rules_engine_clear() {
        let mut engine = RulesEngine::new();
        engine.add_rule(ValidationRule::new("r1", "Rule 1", RuleSeverity::Error));
        assert_eq!(engine.rule_count(), 1);

        engine.clear();
        assert_eq!(engine.rule_count(), 0);
    }
}
