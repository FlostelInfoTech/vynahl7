//! Template parsing from YAML and JSON files.

use crate::{Error, MessageTemplate, Result};
use std::path::Path;

impl MessageTemplate {
    /// Parse a template from YAML string
    ///
    /// # Example
    ///
    /// ```
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let yaml = r#"
    /// name: "Basic ADT"
    /// version: "2.5"
    /// message_type: "ADT"
    /// trigger_event: "A01"
    /// segments:
    ///   - id: "MSH"
    ///     required: true
    /// "#;
    ///
    /// let template = MessageTemplate::from_yaml(yaml).unwrap();
    /// assert_eq!(template.name, "Basic ADT");
    /// ```
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_saphyr::from_str(yaml).map_err(Error::Yaml)
    }

    /// Parse a template from YAML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let template = MessageTemplate::from_yaml_file("templates/adt_a01.yaml").unwrap();
    /// ```
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_yaml(&content)
    }

    /// Parse a template from JSON string
    ///
    /// # Example
    ///
    /// ```
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let json = r#"{
    ///   "name": "Basic ADT",
    ///   "version": "2.5",
    ///   "message_type": "ADT",
    ///   "trigger_event": "A01",
    ///   "segments": []
    /// }"#;
    ///
    /// let template = MessageTemplate::from_json(json).unwrap();
    /// assert_eq!(template.name, "Basic ADT");
    /// ```
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Error::Json)
    }

    /// Parse a template from JSON file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let template = MessageTemplate::from_json_file("templates/adt_a01.json").unwrap();
    /// ```
    pub fn from_json_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
    }

    /// Serialize template to YAML string
    ///
    /// # Example
    ///
    /// ```
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
    /// let yaml = template.to_yaml().unwrap();
    /// assert!(yaml.contains("name: Test"));
    /// ```
    pub fn to_yaml(&self) -> Result<String> {
        serde_saphyr::to_string(self).map_err(Error::YamlSer)
    }

    /// Serialize template to YAML file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
    /// template.to_yaml_file("output/template.yaml").unwrap();
    /// ```
    pub fn to_yaml_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = self.to_yaml()?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// Serialize template to JSON string
    ///
    /// # Example
    ///
    /// ```
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
    /// let json = template.to_json().unwrap();
    /// assert!(json.contains("\"name\":\"Test\""));
    /// ```
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string(self).map_err(Error::Json)
    }

    /// Serialize template to pretty JSON string
    ///
    /// # Example
    ///
    /// ```
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
    /// let json = template.to_json_pretty().unwrap();
    /// assert!(json.contains("  \"name\": \"Test\""));
    /// ```
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Error::Json)
    }

    /// Serialize template to JSON file
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_templates::MessageTemplate;
    ///
    /// let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
    /// template.to_json_file("output/template.json").unwrap();
    /// ```
    pub fn to_json_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = self.to_json_pretty()?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldTemplate, SegmentTemplate};

    #[test]
    fn test_from_yaml() {
        let yaml = r#"
name: "Test Template"
version: "2.5"
message_type: "ADT"
trigger_event: "A01"
segments:
  - id: "MSH"
    required: true
  - id: "PID"
    required: true
"#;

        let template = MessageTemplate::from_yaml(yaml).unwrap();
        assert_eq!(template.name, "Test Template");
        assert_eq!(template.version, "2.5");
        assert_eq!(template.message_type, "ADT");
        assert_eq!(template.trigger_event, "A01");
        assert_eq!(template.segments.len(), 2);
        assert_eq!(template.segments[0].id, "MSH");
        assert!(template.segments[0].required);
    }

    #[test]
    fn test_from_json() {
        let json = r#"{
  "name": "Test Template",
  "version": "2.5",
  "message_type": "ADT",
  "trigger_event": "A01",
  "segments": [
    {
      "id": "MSH",
      "required": true,
      "repeating": false
    }
  ]
}"#;

        let template = MessageTemplate::from_json(json).unwrap();
        assert_eq!(template.name, "Test Template");
        assert_eq!(template.segments.len(), 1);
        assert_eq!(template.segments[0].id, "MSH");
    }

    #[test]
    fn test_to_yaml() {
        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01")
            .with_segment(SegmentTemplate::new("MSH").required());

        let yaml = template.to_yaml().unwrap();
        // Quoting style is serializer-defined; assert content keys plus a
        // full round trip instead of exact formatting.
        for key in ["name:", "version:", "message_type:", "id:", "required:"] {
            assert!(yaml.contains(key), "missing {key} in:\n{yaml}");
        }
        assert!(yaml.contains("2.5"));
        let back = MessageTemplate::from_yaml(&yaml).unwrap();
        assert_eq!(back, template);
    }

    #[test]
    fn test_to_json() {
        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let json = template.to_json().unwrap();

        assert!(json.contains("\"name\":\"Test\""));
        assert!(json.contains("\"version\":\"2.5\""));
        assert!(json.contains("\"message_type\":\"ADT\""));
    }

    #[test]
    fn test_to_json_pretty() {
        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let json = template.to_json_pretty().unwrap();

        assert!(json.contains("  \"name\": \"Test\""));
        assert!(json.contains("  \"version\": \"2.5\""));
    }

    #[test]
    fn test_yaml_with_field_templates() {
        let yaml = r#"
name: "ADT with Fields"
version: "2.5"
message_type: "ADT"
trigger_event: "A01"
segments:
  - id: "PID"
    required: true
    fields:
      5:
        required: true
        placeholder: "{{patient_name}}"
        datatype: "XPN"
      7:
        required: false
        default: "19900101"
"#;

        let template = MessageTemplate::from_yaml(yaml).unwrap();
        let pid = &template.segments[0];
        assert!(pid.fields.is_some());

        let fields = pid.fields.as_ref().unwrap();
        assert!(fields.contains_key(&5));
        assert!(fields.contains_key(&7));

        let field5 = &fields[&5];
        assert!(field5.required);
        assert_eq!(field5.placeholder, Some("{{patient_name}}".to_string()));
        assert_eq!(field5.datatype, Some("XPN".to_string()));

        let field7 = &fields[&7];
        assert!(!field7.required);
        assert_eq!(field7.default, Some("19900101".to_string()));
    }

    #[test]
    fn test_yaml_with_variables() {
        let yaml = r#"
name: "Template with Vars"
version: "2.5"
message_type: "ADT"
trigger_event: "A01"
variables:
  facility: "TestHospital"
  app: "TestApp"
segments:
  - id: "MSH"
    required: true
"#;

        let template = MessageTemplate::from_yaml(yaml).unwrap();
        assert!(template.variables.is_some());

        let vars = template.variables.as_ref().unwrap();
        assert_eq!(vars.get("facility"), Some(&"TestHospital".to_string()));
        assert_eq!(vars.get("app"), Some(&"TestApp".to_string()));
    }

    #[test]
    fn test_yaml_with_inheritance() {
        let yaml = r#"
name: "Extended ADT"
version: "2.5"
message_type: "ADT"
trigger_event: "A01"
extends: "BaseADT"
segments:
  - id: "PID"
    required: true
"#;

        let template = MessageTemplate::from_yaml(yaml).unwrap();
        assert_eq!(template.extends, Some("BaseADT".to_string()));
    }

    #[test]
    fn test_roundtrip_yaml() {
        let original = MessageTemplate::new("Test", "2.5", "ADT", "A01")
            .with_description("Test template")
            .with_segment(SegmentTemplate::new("MSH").required())
            .with_segment(SegmentTemplate::new("PID").required().with_field(
                5,
                FieldTemplate::new().required().with_placeholder("{{name}}"),
            ));

        let yaml = original.to_yaml().unwrap();
        let parsed = MessageTemplate::from_yaml(&yaml).unwrap();

        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.description, original.description);
        assert_eq!(parsed.segments.len(), original.segments.len());
    }

    #[test]
    fn test_roundtrip_json() {
        let original = MessageTemplate::new("Test", "2.5", "ADT", "A01")
            .with_segment(SegmentTemplate::new("MSH").required());

        let json = original.to_json().unwrap();
        let parsed = MessageTemplate::from_json(&json).unwrap();

        assert_eq!(parsed.name, original.name);
        assert_eq!(parsed.segments.len(), original.segments.len());
    }
}
