//! Transformation rule types and function signatures

use crate::error::{Error, Result};

/// A transformation function that converts a string value
///
/// # Arguments
///
/// * `value` - The input value to transform
/// * `context` - Optional context for the transformation (e.g., source message)
///
/// # Returns
///
/// The transformed value or an error
pub type TransformFn = fn(&str, &TransformContext) -> Result<String>;

/// Context information available during transformation
#[derive(Debug, Clone)]
pub struct TransformContext {
    /// The source message being transformed (optional)
    pub source_message: Option<String>,

    /// Additional context data as key-value pairs
    pub data: std::collections::HashMap<String, String>,
}

impl TransformContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self {
            source_message: None,
            data: std::collections::HashMap::new(),
        }
    }

    /// Create a context with a source message
    pub fn with_message(message: String) -> Self {
        Self {
            source_message: Some(message),
            data: std::collections::HashMap::new(),
        }
    }

    /// Add a data entry to the context
    pub fn add_data(mut self, key: String, value: String) -> Self {
        self.data.insert(key, value);
        self
    }

    /// Get a data entry from the context
    pub fn get_data(&self, key: &str) -> Option<&String> {
        self.data.get(key)
    }
}

impl Default for TransformContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A transformation rule that maps a source field to a target field
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::rule::TransformationRule;
///
/// // Simple field copy
/// let rule = TransformationRule::new("PID-5-1", "PID-5-1");
///
/// // Field mapping with transformation
/// let rule = TransformationRule::new("PID-5-1", "PID-5-1")
///     .with_transform(|value, _ctx| Ok(value.to_uppercase()));
/// ```
#[derive(Clone)]
pub struct TransformationRule {
    /// Source field path (terser notation)
    pub source_path: String,

    /// Target field path (terser notation)
    pub target_path: String,

    /// Optional transformation function
    pub transform_fn: Option<TransformFn>,

    /// Optional default value if source is empty
    pub default_value: Option<String>,

    /// Whether to skip if source is empty
    pub skip_if_empty: bool,
}

impl TransformationRule {
    /// Create a new transformation rule
    ///
    /// # Arguments
    ///
    /// * `source_path` - Source field path in terser notation (e.g., "PID-5-1")
    /// * `target_path` - Target field path in terser notation (e.g., "PID-5-1")
    pub fn new<S: Into<String>, T: Into<String>>(source_path: S, target_path: T) -> Self {
        Self {
            source_path: source_path.into(),
            target_path: target_path.into(),
            transform_fn: None,
            default_value: None,
            skip_if_empty: true,
        }
    }

    /// Set the transformation function
    pub fn with_transform(mut self, transform_fn: TransformFn) -> Self {
        self.transform_fn = Some(transform_fn);
        self
    }

    /// Set a default value to use if the source field is empty
    ///
    /// The default applies regardless of [`skip_if_empty`](Self::skip_if_empty):
    /// with the default skip setting [`apply`](Self::apply) returns it for
    /// empty values without calling the transform function.
    pub fn with_default<S: Into<String>>(mut self, default_value: S) -> Self {
        self.default_value = Some(default_value.into());
        self
    }

    /// Set whether to skip the rule if the source field is empty
    ///
    /// Default is true. If false, the transformation will be applied even to empty values.
    /// Keep the default and use [`with_default`](Self::with_default) for plain defaults;
    /// set false only when the transform function itself must see empty input (e.g.
    /// [`default_if_empty`](crate::transforms::default_if_empty)).
    pub fn skip_if_empty(mut self, skip: bool) -> Self {
        self.skip_if_empty = skip;
        self
    }

    /// Apply the transformation to a value
    ///
    /// An empty value never reaches the transform function when `skip_if_empty`
    /// is true (the default): the rule's default value is returned instead
    /// (`None` when unset, so the target is left untouched).
    ///
    /// # Arguments
    ///
    /// * `value` - The input value from the source field
    /// * `context` - Context for the transformation
    ///
    /// # Returns
    ///
    /// The transformed value or None if the rule should be skipped
    pub fn apply(&self, value: &str, context: &TransformContext) -> Result<Option<String>> {
        // Handle empty values
        if value.is_empty() {
            if self.skip_if_empty {
                return Ok(self.default_value.clone());
            }
            if let Some(default) = &self.default_value {
                return Ok(Some(default.clone()));
            }
        }

        // Apply transformation if present
        let result = if let Some(transform) = self.transform_fn {
            transform(value, context)?
        } else {
            value.to_string()
        };

        Ok(Some(result))
    }

    /// Validate the rule
    pub fn validate(&self) -> Result<()> {
        if self.source_path.is_empty() {
            return Err(Error::invalid_rule("Source path cannot be empty"));
        }
        if self.target_path.is_empty() {
            return Err(Error::invalid_rule("Target path cannot be empty"));
        }

        // Basic validation of terser path format
        if !self.is_valid_terser_path(&self.source_path) {
            return Err(Error::invalid_rule(format!(
                "Invalid source path format: {}",
                self.source_path
            )));
        }
        if !self.is_valid_terser_path(&self.target_path) {
            return Err(Error::invalid_rule(format!(
                "Invalid target path format: {}",
                self.target_path
            )));
        }

        Ok(())
    }

    /// Check if a path is a valid terser path
    fn is_valid_terser_path(&self, path: &str) -> bool {
        // Basic validation: should match paths like PID-5, PID-5-1,
        // OBX(1)-5, or PID-11(1)-1. The pattern is compiled once and
        // mirrors what the terser accepts: an optional segment
        // occurrence, a field with an optional repetition, then optional
        // component/subcomponent indices.
        static PATH_RE: std::sync::OnceLock<Option<regex::Regex>> = std::sync::OnceLock::new();
        let compiled = PATH_RE
            .get_or_init(|| regex::Regex::new(r"^[A-Z]{2,3}(\(\d+\))?-\d+(\(\d+\))?(-\d+)*$").ok());
        compiled.as_ref().is_some_and(|re| re.is_match(path))
    }
}

impl std::fmt::Debug for TransformationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformationRule")
            .field("source_path", &self.source_path)
            .field("target_path", &self.target_path)
            .field("has_transform_fn", &self.transform_fn.is_some())
            .field("default_value", &self.default_value)
            .field("skip_if_empty", &self.skip_if_empty)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_creation() {
        let rule = TransformationRule::new("PID-5-1", "PID-5-1");
        assert_eq!(rule.source_path, "PID-5-1");
        assert_eq!(rule.target_path, "PID-5-1");
        assert!(rule.transform_fn.is_none());
        assert!(rule.skip_if_empty);
    }

    #[test]
    fn test_rule_with_transform() {
        fn uppercase(value: &str, _ctx: &TransformContext) -> Result<String> {
            Ok(value.to_uppercase())
        }

        let rule = TransformationRule::new("PID-5-1", "PID-5-1").with_transform(uppercase);

        assert!(rule.transform_fn.is_some());
    }

    #[test]
    fn test_rule_with_default() {
        let rule = TransformationRule::new("PID-5-1", "PID-5-1").with_default("UNKNOWN");

        assert_eq!(rule.default_value, Some("UNKNOWN".to_string()));
    }

    #[test]
    fn test_apply_simple() {
        let rule = TransformationRule::new("PID-5-1", "PID-5-1");
        let context = TransformContext::new();

        let result = rule.apply("SMITH", &context).unwrap();
        assert_eq!(result, Some("SMITH".to_string()));
    }

    #[test]
    fn test_apply_with_transform() {
        fn uppercase(value: &str, _ctx: &TransformContext) -> Result<String> {
            Ok(value.to_uppercase())
        }

        let rule = TransformationRule::new("PID-5-1", "PID-5-1").with_transform(uppercase);
        let context = TransformContext::new();

        let result = rule.apply("smith", &context).unwrap();
        assert_eq!(result, Some("SMITH".to_string()));
    }

    #[test]
    fn test_apply_empty_with_default() {
        let rule = TransformationRule::new("PID-5-1", "PID-5-1").with_default("UNKNOWN");
        let context = TransformContext::new();

        let result = rule.apply("", &context).unwrap();
        assert_eq!(result, Some("UNKNOWN".to_string()));
    }

    #[test]
    fn test_apply_empty_skip() {
        let rule = TransformationRule::new("PID-5-1", "PID-5-1");
        let context = TransformContext::new();

        let result = rule.apply("", &context).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_apply_skips_transform_fn_for_empty_values_by_default() {
        fn fail_on_call(_value: &str, _ctx: &TransformContext) -> Result<String> {
            panic!("transform function must not run for empty values when skipping");
        }

        // Default skip: the transform never runs, so no panic and None.
        let rule = TransformationRule::new("PID-5-1", "PID-5-1").with_transform(fail_on_call);
        let context = TransformContext::new();
        assert_eq!(rule.apply("", &context).unwrap(), None);

        // Opted out of skip: the transform runs on the empty value.
        let rule = TransformationRule::new("PID-5-1", "PID-5-1")
            .with_transform(|value, _ctx| Ok(format!("got:{value}")))
            .skip_if_empty(false);
        assert_eq!(rule.apply("", &context).unwrap(), Some("got:".to_string()));
    }

    #[test]
    fn test_validate_valid_paths() {
        let rule = TransformationRule::new("PID-5-1", "PID-5-1");
        assert!(rule.validate().is_ok());

        let rule = TransformationRule::new("OBX(1)-5", "OBX(1)-5");
        assert!(rule.validate().is_ok());

        let rule = TransformationRule::new("MSH-9-1", "MSH-9-2");
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_paths() {
        let rule = TransformationRule::new("", "PID-5-1");
        assert!(rule.validate().is_err());

        let rule = TransformationRule::new("PID-5-1", "");
        assert!(rule.validate().is_err());

        let rule = TransformationRule::new("INVALID", "PID-5-1");
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_validate_repetition_paths() {
        // Mid-path repetitions are valid terser paths.
        let rule = TransformationRule::new("PID-11(1)-1", "PID-11(1)-1");
        assert!(rule.validate().is_ok());

        let rule = TransformationRule::new("PID-11(2)", "PID-11(2)");
        assert!(rule.validate().is_ok());

        let rule = TransformationRule::new("OBX(2)-5-1", "OBX(2)-5-1");
        assert!(rule.validate().is_ok());
    }

    #[test]
    fn test_validate_rejects_misplaced_repetition() {
        // A repetition marker after a component is not a terser path.
        let rule = TransformationRule::new("PID-5-1(2)", "PID-5-1");
        assert!(rule.validate().is_err());
    }

    #[test]
    fn test_context_creation() {
        let ctx = TransformContext::new();
        assert!(ctx.source_message.is_none());
        assert!(ctx.data.is_empty());
    }

    #[test]
    fn test_context_with_message() {
        let ctx = TransformContext::with_message("MSH|...".to_string());
        assert!(ctx.source_message.is_some());
    }

    #[test]
    fn test_context_add_data() {
        let ctx = TransformContext::new()
            .add_data("key1".to_string(), "value1".to_string())
            .add_data("key2".to_string(), "value2".to_string());

        assert_eq!(ctx.get_data("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.get_data("key2"), Some(&"value2".to_string()));
        assert_eq!(ctx.get_data("key3"), None);
    }
}
