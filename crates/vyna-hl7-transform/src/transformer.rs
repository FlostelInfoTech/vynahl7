//! Message transformer with fluent API

use crate::error::{Error, Result};
use crate::rule::{TransformContext, TransformFn, TransformationRule};
use std::borrow::Cow;
use std::collections::HashMap;
use vyna_hl7_core::Message;
use vyna_hl7_terser::{Terser, TerserMut};

/// Message transformer that applies transformation rules to HL7 messages
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{MessageTransformer, transforms};
/// use vyna_hl7_core::Message;
///
/// let mut transformer = MessageTransformer::new();
///
/// // Simple field mapping
/// transformer.add_mapping("PID-5-1", "PID-5-1");
///
/// // Field mapping with transformation
/// transformer.add_transform("PID-5-1", "PID-5-1", transforms::uppercase);
///
/// // Transform a message
/// # let source = Message::new();
/// let result = transformer.transform(&source);
/// ```
#[derive(Debug)]
pub struct MessageTransformer {
    /// List of transformation rules to apply
    rules: Vec<TransformationRule>,

    /// Per-rule params, parallel to `rules`: each rule's private overlay
    /// over the shared context so rules using the same transform with
    /// different params stay independent.
    rule_params: Vec<HashMap<String, String>>,

    /// Context for transformations
    context: TransformContext,
}

impl MessageTransformer {
    /// Create a new message transformer
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rule_params: Vec::new(),
            context: TransformContext::new(),
        }
    }

    /// Add a simple field-to-field mapping
    ///
    /// # Arguments
    ///
    /// * `source_path` - Source field path (terser notation)
    /// * `target_path` - Target field path (terser notation)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vyna_hl7_transform::MessageTransformer;
    ///
    /// let mut transformer = MessageTransformer::new();
    /// transformer.add_mapping("PID-5-1", "PID-5-1"); // Copy family name
    /// ```
    pub fn add_mapping<S: Into<String>, T: Into<String>>(
        &mut self,
        source_path: S,
        target_path: T,
    ) {
        let rule = TransformationRule::new(source_path, target_path);
        self.add_rule(rule);
    }

    /// Add a field mapping with a transformation function
    ///
    /// # Arguments
    ///
    /// * `source_path` - Source field path (terser notation)
    /// * `target_path` - Target field path (terser notation)
    /// * `transform_fn` - Transformation function to apply
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vyna_hl7_transform::{MessageTransformer, transforms};
    ///
    /// let mut transformer = MessageTransformer::new();
    /// transformer.add_transform("PID-5-1", "PID-5-1", transforms::uppercase);
    /// ```
    pub fn add_transform<S: Into<String>, T: Into<String>>(
        &mut self,
        source_path: S,
        target_path: T,
        transform_fn: TransformFn,
    ) {
        let rule = TransformationRule::new(source_path, target_path).with_transform(transform_fn);
        self.add_rule(rule);
    }

    /// Add a pre-configured transformation rule
    ///
    /// # Arguments
    ///
    /// * `rule` - The transformation rule to add
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vyna_hl7_transform::{MessageTransformer, rule::TransformationRule, transforms};
    ///
    /// let mut transformer = MessageTransformer::new();
    /// let rule = TransformationRule::new("PID-5-1", "PID-5-1")
    ///     .with_transform(transforms::uppercase)
    ///     .with_default("UNKNOWN");
    /// transformer.add_rule(rule);
    /// ```
    pub fn add_rule(&mut self, rule: TransformationRule) {
        self.add_rule_with_params(rule, HashMap::new());
    }

    /// Add a transformation rule with parameters private to that rule.
    ///
    /// The rule's params overlay the shared context when the rule runs:
    /// a rule param shadows a same-named shared key for that rule only,
    /// and keys absent from the rule's params resolve from shared data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use vyna_hl7_transform::{MessageTransformer, rule::TransformationRule, transforms};
    ///
    /// let mut transformer = MessageTransformer::new();
    /// let mut params = HashMap::new();
    /// params.insert("length".to_string(), "3".to_string());
    /// params.insert("start".to_string(), "0".to_string());
    /// let rule = TransformationRule::new("PID-3", "PID-3")
    ///     .with_transform(transforms::substring);
    /// transformer.add_rule_with_params(rule, params);
    /// ```
    pub fn add_rule_with_params(
        &mut self,
        rule: TransformationRule,
        params: HashMap<String, String>,
    ) {
        self.rules.push(rule);
        self.rule_params.push(params);
    }

    /// Add multiple transformation rules
    pub fn add_rules(&mut self, rules: Vec<TransformationRule>) {
        for rule in rules {
            self.add_rule(rule);
        }
    }

    /// Set context data for transformations
    ///
    /// # Arguments
    ///
    /// * `key` - Context key
    /// * `value` - Context value
    pub fn set_context_data(&mut self, key: String, value: String) {
        self.context.data.insert(key, value);
    }

    /// Transform a message by applying all rules
    ///
    /// # Arguments
    ///
    /// * `source` - The source message to transform
    ///
    /// # Returns
    ///
    /// A new transformed message
    pub fn transform(&self, source: &Message) -> Result<Message> {
        // Create target message as a clone of source
        let mut target = source.clone();

        // Create terser for source (read-only)
        let source_terser = Terser::new(source);

        // Create mutable terser for target
        let mut target_terser = TerserMut::new(&mut target);

        // Apply each rule
        for (rule, params) in self.rules.iter().zip(self.rule_params.iter()) {
            // Validate rule
            rule.validate()?;

            // Get source value
            let source_value = source_terser
                .get(&rule.source_path)
                .map_err(|e| {
                    Error::field_access(format!("Failed to get {}: {}", rule.source_path, e))
                })?
                .unwrap_or_default();

            // Apply transformation with this rule's params overlaid
            let context = Self::overlay_context(&self.context, params);
            if let Some(transformed_value) = rule.apply(source_value, &context)? {
                // Set target value
                target_terser
                    .set(&rule.target_path, &transformed_value)
                    .map_err(|e| {
                        Error::field_access(format!("Failed to set {}: {}", rule.target_path, e))
                    })?;
            }
        }

        Ok(target)
    }

    /// Transform a message in place
    ///
    /// This is atomic: rules apply to a staged copy, and the message is
    /// only updated when every rule succeeds. A failing rule leaves the
    /// message unchanged.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to transform
    pub fn transform_in_place(&self, message: &mut Message) -> Result<()> {
        // Create a source clone for reading
        let source = message.clone();
        let source_terser = Terser::new(&source);

        // Apply rules to a staged copy; commit only on full success.
        let mut staged = source.clone();
        let mut target_terser = TerserMut::new(&mut staged);

        // Apply each rule
        for (rule, params) in self.rules.iter().zip(self.rule_params.iter()) {
            // Validate rule
            rule.validate()?;

            // Get source value
            let source_value = source_terser
                .get(&rule.source_path)
                .map_err(|e| {
                    Error::field_access(format!("Failed to get {}: {}", rule.source_path, e))
                })?
                .unwrap_or_default();

            // Apply transformation with this rule's params overlaid
            let context = Self::overlay_context(&self.context, params);
            if let Some(transformed_value) = rule.apply(source_value, &context)? {
                // Set target value
                target_terser
                    .set(&rule.target_path, &transformed_value)
                    .map_err(|e| {
                        Error::field_access(format!("Failed to set {}: {}", rule.target_path, e))
                    })?;
            }
        }

        *message = staged;

        Ok(())
    }

    /// Get the number of transformation rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Merge one rule's private params over the shared context.
    ///
    /// Borrows the shared context when the rule has no params so the
    /// common no-params path pays no clone.
    fn overlay_context<'a>(
        base: &'a TransformContext,
        params: &HashMap<String, String>,
    ) -> Cow<'a, TransformContext> {
        if params.is_empty() {
            Cow::Borrowed(base)
        } else {
            let mut merged = base.clone();
            merged.data.extend(params.clone());
            Cow::Owned(merged)
        }
    }

    /// Clear all transformation rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
        self.rule_params.clear();
    }

    /// Validate all rules
    pub fn validate_rules(&self) -> Result<()> {
        for rule in &self.rules {
            rule.validate()?;
        }
        Ok(())
    }
}

impl Default for MessageTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms;
    use vyna_hl7_core::Version;
    use vyna_hl7_core::builders::adt::AdtBuilder;

    #[test]
    fn test_transformer_creation() {
        let transformer = MessageTransformer::new();
        assert_eq!(transformer.rule_count(), 0);
    }

    #[test]
    fn test_add_mapping() {
        let mut transformer = MessageTransformer::new();
        transformer.add_mapping("PID-5-1", "PID-5-1");
        assert_eq!(transformer.rule_count(), 1);
    }

    #[test]
    fn test_add_transform() {
        let mut transformer = MessageTransformer::new();
        transformer.add_transform("PID-5-1", "PID-5-1", transforms::uppercase);
        assert_eq!(transformer.rule_count(), 1);
    }

    #[test]
    fn test_add_rule() {
        let mut transformer = MessageTransformer::new();
        let rule =
            TransformationRule::new("PID-5-1", "PID-5-1").with_transform(transforms::uppercase);
        transformer.add_rule(rule);
        assert_eq!(transformer.rule_count(), 1);
    }

    #[test]
    fn test_clear_rules() {
        let mut transformer = MessageTransformer::new();
        transformer.add_mapping("PID-5-1", "PID-5-1");
        transformer.add_mapping("PID-7", "PID-7");
        assert_eq!(transformer.rule_count(), 2);

        transformer.clear_rules();
        assert_eq!(transformer.rule_count(), 0);
    }

    #[test]
    fn test_transform_simple_copy() {
        use vyna_hl7_parser::parse_message;

        // Build and encode the message, then parse it to get proper component structure
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();
        let encoded = msg.encode();
        let source = parse_message(&encoded).unwrap();

        let mut transformer = MessageTransformer::new();
        transformer.add_mapping("PID-3", "PID-3");

        let target = transformer.transform(&source).unwrap();

        let terser = Terser::new(&target);
        assert_eq!(terser.get("PID-3").unwrap(), Some("12345"));
    }

    #[test]
    fn test_transform_with_uppercase() {
        use vyna_hl7_parser::parse_message;

        // Build and encode the message, then parse it to get proper component structure
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("abc123")
            .build()
            .unwrap();
        let encoded = msg.encode();
        let source = parse_message(&encoded).unwrap();

        let mut transformer = MessageTransformer::new();
        transformer.add_transform("PID-3", "PID-3", transforms::uppercase);

        let target = transformer.transform(&source).unwrap();

        let terser = Terser::new(&target);
        assert_eq!(terser.get("PID-3").unwrap(), Some("ABC123"));
    }

    #[test]
    fn test_transform_multiple_rules() {
        use vyna_hl7_parser::parse_message;

        // Build and encode the message, then parse it to get proper component structure
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("abc123")
            .sex("m")
            .build()
            .unwrap();
        let encoded = msg.encode();
        let source = parse_message(&encoded).unwrap();

        let mut transformer = MessageTransformer::new();
        transformer.add_transform("PID-3", "PID-3", transforms::uppercase);
        transformer.add_mapping("PID-8", "PID-8");

        let target = transformer.transform(&source).unwrap();

        let terser = Terser::new(&target);
        assert_eq!(terser.get("PID-3").unwrap(), Some("ABC123"));
        assert_eq!(terser.get("PID-8").unwrap(), Some("m"));
    }

    #[test]
    fn test_transform_in_place() {
        use vyna_hl7_parser::parse_message;

        // Build and encode the message, then parse it to get proper component structure
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("abc123")
            .build()
            .unwrap();
        let encoded = msg.encode();
        let mut message = parse_message(&encoded).unwrap();

        let mut transformer = MessageTransformer::new();
        transformer.add_transform("PID-3", "PID-3", transforms::uppercase);

        transformer.transform_in_place(&mut message).unwrap();

        let terser = Terser::new(&message);
        assert_eq!(terser.get("PID-3").unwrap(), Some("ABC123"));
    }

    #[test]
    fn test_transform_in_place_is_atomic_on_later_rule_failure() {
        use vyna_hl7_parser::parse_message;

        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("abc123")
            .build()
            .unwrap();
        let encoded = msg.encode();
        let mut message = parse_message(&encoded).unwrap();

        let mut transformer = MessageTransformer::new();
        transformer.add_transform("PID-3", "PID-3", transforms::uppercase);
        // Second rule fails only when the transform runs.
        transformer.add_mapping("INVALID", "PID-8");

        let before = message.encode();
        assert!(transformer.transform_in_place(&mut message).is_err());
        assert_eq!(message.encode(), before);
    }

    #[test]
    fn test_transform_with_default() {
        let source = AdtBuilder::a01(Version::V2_5).build().unwrap();

        let mut transformer = MessageTransformer::new();
        let rule = TransformationRule::new("PID-5-1", "PID-5-1").with_default("UNKNOWN");
        transformer.add_rule(rule);

        let target = transformer.transform(&source).unwrap();

        let terser = Terser::new(&target);
        assert_eq!(terser.get("PID-5-1").unwrap(), Some("UNKNOWN"));
    }

    #[test]
    fn test_validate_rules() {
        let mut transformer = MessageTransformer::new();
        transformer.add_mapping("PID-5-1", "PID-5-1");
        transformer.add_mapping("MSH-9-1", "MSH-9-1");

        assert!(transformer.validate_rules().is_ok());
    }

    #[test]
    fn test_validate_rules_invalid() {
        let mut transformer = MessageTransformer::new();
        transformer.add_mapping("INVALID", "PID-5-1");

        assert!(transformer.validate_rules().is_err());
    }

    #[test]
    fn test_transform_repetition_path() {
        use vyna_hl7_parser::parse_message;

        let source = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r\
             PID|1||7||Smith^John||||||home^street~work^ave\r",
        )
        .unwrap();

        let mut transformer = MessageTransformer::new();
        transformer.add_transform("PID-11(1)-1", "PID-11(1)-1", transforms::uppercase);

        let target = transformer.transform(&source).unwrap();

        let terser = Terser::new(&target);
        assert_eq!(terser.get("PID-11(1)-1").unwrap(), Some("HOME"));
        assert_eq!(terser.get("PID-11(2)-1").unwrap(), Some("work"));
    }

    #[test]
    fn test_rule_params_overlay_shared_context() {
        use vyna_hl7_parser::parse_message;

        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("ABCDEFGH")
            .build()
            .unwrap();
        let encoded = msg.encode();
        let source = parse_message(&encoded).unwrap();

        let mut transformer = MessageTransformer::new();
        transformer.set_context_data("start".to_string(), "0".to_string());
        transformer.set_context_data("length".to_string(), "2".to_string());
        let mut params = std::collections::HashMap::new();
        params.insert("length".to_string(), "4".to_string());
        transformer.add_rule_with_params(
            TransformationRule::new("PID-3", "PID-3").with_transform(transforms::substring),
            params,
        );
        transformer.add_transform("PID-3", "PID-8", transforms::substring);

        let target = transformer.transform(&source).unwrap();

        let terser = Terser::new(&target);
        // The rule's own params shadow the shared length for that rule only.
        assert_eq!(terser.get("PID-3").unwrap(), Some("ABCD"));
        // The plain rule still resolves start/length from shared context.
        assert_eq!(terser.get("PID-8").unwrap(), Some("AB"));
    }
}
