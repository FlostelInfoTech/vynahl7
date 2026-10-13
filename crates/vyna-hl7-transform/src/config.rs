//! Declarative transformation configuration using YAML/JSON
//!
//! This module allows defining transformation rules in YAML or JSON files.
//!
//! # Examples
//!
//! YAML configuration:
//! ```yaml
//! rules:
//!   - source: PID-5-1
//!     target: PID-5-1
//!     transform: uppercase
//!   - source: PID-3-1
//!     target: PID-3-1
//!     default: UNKNOWN
//! ```
//!
//! Loading configuration:
//! ```rust,ignore
//! use vyna_hl7_transform::config::TransformConfig;
//!
//! let config = TransformConfig::from_yaml_file("transform.yaml")?;
//! let transformer = config.build()?;
//! ```

use crate::error::{Error, Result};
use crate::rule::{TransformContext, TransformationRule};
use crate::transformer::MessageTransformer;
use crate::transforms;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a single transformation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleConfig {
    /// Source field path (terser notation)
    pub source: String,

    /// Target field path (terser notation)
    pub target: String,

    /// Optional transformation function name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<String>,

    /// Optional default value if source is empty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Whether to skip if source is empty (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_if_empty: Option<bool>,

    /// Optional transformation parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<HashMap<String, String>>,
}

impl RuleConfig {
    /// Create a new rule configuration
    pub fn new<S: Into<String>, T: Into<String>>(source: S, target: T) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            transform: None,
            default: None,
            skip_if_empty: None,
            params: None,
        }
    }

    /// Set the transformation function
    pub fn with_transform<S: Into<String>>(mut self, transform: S) -> Self {
        self.transform = Some(transform.into());
        self
    }

    /// Set the default value
    pub fn with_default<S: Into<String>>(mut self, default: S) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Set transformation parameters
    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = Some(params);
        self
    }

    /// Convert to a TransformationRule, merging this rule's params
    /// into the given context.
    ///
    /// Kept for single-rule use. [`TransformConfig::build`] uses
    /// [`to_rule_with_params`](Self::to_rule_with_params) instead so each
    /// rule keeps its own params rather than sharing one map.
    pub fn to_rule(&self, context: &mut TransformContext) -> Result<TransformationRule> {
        let (rule, params) = self.to_rule_with_params()?;
        for (key, value) in params {
            context.data.insert(key, value);
        }
        Ok(rule)
    }

    /// Convert to a TransformationRule plus its private params.
    ///
    /// The params are returned alongside the rule instead of being merged
    /// into a shared map, so rules using the same transform function with
    /// different params do not clobber each other.
    pub fn to_rule_with_params(&self) -> Result<(TransformationRule, HashMap<String, String>)> {
        let mut rule = TransformationRule::new(&self.source, &self.target);

        // Set default value if provided
        if let Some(default) = &self.default {
            rule = rule.with_default(default);
        }

        // Set skip_if_empty if provided
        if let Some(skip) = self.skip_if_empty {
            rule = rule.skip_if_empty(skip);
        }

        // Set transformation function if provided
        if let Some(transform_name) = &self.transform {
            let transform_fn = match transform_name.as_str() {
                "uppercase" => transforms::uppercase,
                "lowercase" => transforms::lowercase,
                "trim" => transforms::trim,
                "trim_start" => transforms::trim_start,
                "trim_end" => transforms::trim_end,
                "remove_whitespace" => transforms::remove_whitespace,
                "substring" => transforms::substring,
                "format_date" => transforms::format_date,
                "format_datetime" => transforms::format_datetime,
                "replace" => transforms::replace,
                "regex_replace" => transforms::regex_replace,
                "prefix" => transforms::prefix,
                "suffix" => transforms::suffix,
                "pad" => transforms::pad,
                "default_if_empty" => transforms::default_if_empty,
                _ => {
                    return Err(Error::config(format!(
                        "Unknown transformation function: {}",
                        transform_name
                    )))
                }
            };
            rule = rule.with_transform(transform_fn);
        }

        Ok((rule, self.params.clone().unwrap_or_default()))
    }
}

/// Main transformation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    /// List of transformation rules
    pub rules: Vec<RuleConfig>,

    /// Optional global context data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, String>>,
}

impl TransformConfig {
    /// Create a new transformation configuration
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            context: None,
        }
    }

    /// Add a rule to the configuration
    pub fn add_rule(&mut self, rule: RuleConfig) {
        self.rules.push(rule);
    }

    /// Set global context data
    pub fn set_context(&mut self, context: HashMap<String, String>) {
        self.context = Some(context);
    }

    /// Build a MessageTransformer from this configuration
    pub fn build(&self) -> Result<MessageTransformer> {
        let mut transformer = MessageTransformer::new();

        // Add global context data (shared by all rules)
        if let Some(global_context) = &self.context {
            for (key, value) in global_context {
                transformer.set_context_data(key.clone(), value.clone());
            }
        }

        // Convert and add all rules, keeping each rule's params private
        // so same-named params on different rules do not clobber each other.
        for rule_config in &self.rules {
            let (rule, params) = rule_config.to_rule_with_params()?;
            transformer.add_rule_with_params(rule, params);
        }

        Ok(transformer)
    }

    /// Load configuration from a YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).map_err(Error::Yaml)
    }

    /// Load configuration from a YAML file
    pub fn from_yaml_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::config(format!("Failed to read file {}: {}", path, e)))?;
        Self::from_yaml(&contents)
    }

    /// Load configuration from a JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Error::Json)
    }

    /// Load configuration from a JSON file
    pub fn from_json_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::config(format!("Failed to read file {}: {}", path, e)))?;
        Self::from_json(&contents)
    }

    /// Save configuration to a YAML string
    pub fn to_yaml(&self) -> Result<String> {
        serde_yaml::to_string(self).map_err(Error::Yaml)
    }

    /// Save configuration to a YAML file
    pub fn to_yaml_file(&self, path: &str) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)
            .map_err(|e| Error::config(format!("Failed to write file {}: {}", path, e)))?;
        Ok(())
    }

    /// Save configuration to a JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Error::Json)
    }

    /// Save configuration to a JSON file
    pub fn to_json_file(&self, path: &str) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)
            .map_err(|e| Error::config(format!("Failed to write file {}: {}", path, e)))?;
        Ok(())
    }
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_config_creation() {
        let rule = RuleConfig::new("PID-5-1", "PID-5-1");
        assert_eq!(rule.source, "PID-5-1");
        assert_eq!(rule.target, "PID-5-1");
        assert!(rule.transform.is_none());
    }

    #[test]
    fn test_rule_config_with_transform() {
        let rule = RuleConfig::new("PID-5-1", "PID-5-1")
            .with_transform("uppercase");
        assert_eq!(rule.transform, Some("uppercase".to_string()));
    }

    #[test]
    fn test_config_creation() {
        let config = TransformConfig::new();
        assert_eq!(config.rules.len(), 0);
    }

    #[test]
    fn test_config_add_rule() {
        let mut config = TransformConfig::new();
        config.add_rule(RuleConfig::new("PID-5-1", "PID-5-1"));
        assert_eq!(config.rules.len(), 1);
    }

    #[test]
    fn test_yaml_serialization() {
        let mut config = TransformConfig::new();
        config.add_rule(
            RuleConfig::new("PID-5-1", "PID-5-1")
                .with_transform("uppercase")
        );

        let yaml = config.to_yaml().unwrap();
        assert!(yaml.contains("source: PID-5-1"));
        assert!(yaml.contains("target: PID-5-1"));
        assert!(yaml.contains("transform: uppercase"));
    }

    #[test]
    fn test_yaml_deserialization() {
        let yaml = r#"
rules:
  - source: PID-5-1
    target: PID-5-1
    transform: uppercase
  - source: PID-3-1
    target: PID-3-1
    default: UNKNOWN
"#;

        let config = TransformConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].source, "PID-5-1");
        assert_eq!(config.rules[0].transform, Some("uppercase".to_string()));
        assert_eq!(config.rules[1].default, Some("UNKNOWN".to_string()));
    }

    #[test]
    fn test_json_serialization() {
        let mut config = TransformConfig::new();
        config.add_rule(
            RuleConfig::new("PID-5-1", "PID-5-1")
                .with_transform("uppercase")
        );

        let json = config.to_json().unwrap();
        assert!(json.contains("\"source\": \"PID-5-1\""));
        assert!(json.contains("\"target\": \"PID-5-1\""));
        assert!(json.contains("\"transform\": \"uppercase\""));
    }

    #[test]
    fn test_json_deserialization() {
        let json = r#"{
  "rules": [
    {
      "source": "PID-5-1",
      "target": "PID-5-1",
      "transform": "uppercase"
    },
    {
      "source": "PID-3-1",
      "target": "PID-3-1",
      "default": "UNKNOWN"
    }
  ]
}"#;

        let config = TransformConfig::from_json(json).unwrap();
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].source, "PID-5-1");
        assert_eq!(config.rules[0].transform, Some("uppercase".to_string()));
        assert_eq!(config.rules[1].default, Some("UNKNOWN".to_string()));
    }

    #[test]
    fn test_build_transformer() {
        let mut config = TransformConfig::new();
        config.add_rule(
            RuleConfig::new("PID-5-1", "PID-5-1")
                .with_transform("uppercase")
        );
        config.add_rule(RuleConfig::new("PID-3-1", "PID-3-1"));

        let transformer = config.build().unwrap();
        assert_eq!(transformer.rule_count(), 2);
    }

    #[test]
    fn test_rule_config_with_params() {
        let mut params = HashMap::new();
        params.insert("format".to_string(), "YYYY-MM-DD".to_string());

        let rule = RuleConfig::new("PID-7", "PID-7")
            .with_transform("format_date")
            .with_params(params);

        let mut context = TransformContext::new();
        let transform_rule = rule.to_rule(&mut context).unwrap();

        assert!(transform_rule.transform_fn.is_some());
        assert_eq!(
            context.get_data("format"),
            Some(&"YYYY-MM-DD".to_string())
        );
    }

    #[test]
    fn test_config_with_context() {
        let mut config = TransformConfig::new();
        let mut global_context = HashMap::new();
        global_context.insert("facility".to_string(), "HOSPITAL".to_string());
        config.set_context(global_context);

        config.add_rule(RuleConfig::new("PID-5-1", "PID-5-1"));

        let transformer = config.build().unwrap();
        assert_eq!(transformer.rule_count(), 1);
    }

    #[test]
    fn test_unknown_transform_function() {
        let rule = RuleConfig::new("PID-5-1", "PID-5-1")
            .with_transform("unknown_function");

        let mut context = TransformContext::new();
        let result = rule.to_rule(&mut context);
        assert!(result.is_err());
    }

    #[test]
    fn test_rules_with_different_params_are_independent() {
        use vyna_hl7_core::Version;
        use vyna_hl7_core::builders::adt::AdtBuilder;
        use vyna_hl7_parser::parse_message;
        use vyna_hl7_terser::Terser;

        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("ABCDEFGH")
            .sex("12345678")
            .build()
            .unwrap();
        let encoded = msg.encode();
        let source = parse_message(&encoded).unwrap();

        let mut config = TransformConfig::new();
        let mut first = HashMap::new();
        first.insert("start".to_string(), "0".to_string());
        first.insert("length".to_string(), "3".to_string());
        config.add_rule(
            RuleConfig::new("PID-3", "PID-3")
                .with_transform("substring")
                .with_params(first),
        );
        let mut second = HashMap::new();
        second.insert("start".to_string(), "0".to_string());
        second.insert("length".to_string(), "5".to_string());
        config.add_rule(
            RuleConfig::new("PID-8", "PID-8")
                .with_transform("substring")
                .with_params(second),
        );

        let transformer = config.build().unwrap();
        let target = transformer.transform(&source).unwrap();
        let terser = Terser::new(&target);
        assert_eq!(terser.get("PID-3").unwrap(), Some("ABC"));
        assert_eq!(terser.get("PID-8").unwrap(), Some("12345"));
    }
}
