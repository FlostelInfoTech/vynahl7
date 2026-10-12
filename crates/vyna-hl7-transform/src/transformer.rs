//! Message transformer with fluent API

use crate::error::{Error, Result};
use crate::rule::{TransformContext, TransformFn, TransformationRule};
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

    /// Context for transformations
    context: TransformContext,
}

impl MessageTransformer {
    /// Create a new message transformer
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
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
    pub fn add_mapping<S: Into<String>, T: Into<String>>(&mut self, source_path: S, target_path: T) {
        let rule = TransformationRule::new(source_path, target_path);
        self.rules.push(rule);
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
        let rule = TransformationRule::new(source_path, target_path)
            .with_transform(transform_fn);
        self.rules.push(rule);
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
        self.rules.push(rule);
    }

    /// Add multiple transformation rules
    pub fn add_rules(&mut self, rules: Vec<TransformationRule>) {
        self.rules.extend(rules);
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
        for rule in &self.rules {
            // Validate rule
            rule.validate()?;

            // Get source value
            let source_value = source_terser
                .get(&rule.source_path)
                .map_err(|e| Error::field_access(format!("Failed to get {}: {}", rule.source_path, e)))?
                .unwrap_or_default();

            // Apply transformation
            if let Some(transformed_value) = rule.apply(source_value, &self.context)? {
                // Set target value
                target_terser
                    .set(&rule.target_path, &transformed_value)
                    .map_err(|e| Error::field_access(format!("Failed to set {}: {}", rule.target_path, e)))?;
            }
        }

        Ok(target)
    }

    /// Transform a message in place
    ///
    /// # Arguments
    ///
    /// * `message` - The message to transform
    pub fn transform_in_place(&self, message: &mut Message) -> Result<()> {
        // Create a source clone for reading
        let source = message.clone();
        let source_terser = Terser::new(&source);

        // Create mutable terser for modifying the target
        let mut target_terser = TerserMut::new(message);

        // Apply each rule
        for rule in &self.rules {
            // Validate rule
            rule.validate()?;

            // Get source value
            let source_value = source_terser
                .get(&rule.source_path)
                .map_err(|e| Error::field_access(format!("Failed to get {}: {}", rule.source_path, e)))?
                .unwrap_or_default();

            // Apply transformation
            if let Some(transformed_value) = rule.apply(source_value, &self.context)? {
                // Set target value
                target_terser
                    .set(&rule.target_path, &transformed_value)
                    .map_err(|e| Error::field_access(format!("Failed to set {}: {}", rule.target_path, e)))?;
            }
        }

        Ok(())
    }

    /// Get the number of transformation rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Clear all transformation rules
    pub fn clear_rules(&mut self) {
        self.rules.clear();
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
    use vyna_hl7_core::builders::adt::AdtBuilder;
    use vyna_hl7_core::Version;

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
        let rule = TransformationRule::new("PID-5-1", "PID-5-1")
            .with_transform(transforms::uppercase);
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
    fn test_transform_with_default() {
        let source = AdtBuilder::a01(Version::V2_5).build().unwrap();

        let mut transformer = MessageTransformer::new();
        let rule = TransformationRule::new("PID-5-1", "PID-5-1")
            .with_default("UNKNOWN");
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
}
