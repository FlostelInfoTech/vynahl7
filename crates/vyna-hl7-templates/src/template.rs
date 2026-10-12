//! Template data structures for HL7 messages and segments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A complete message template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    /// Template name
    pub name: String,

    /// Template description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// HL7 version (e.g., "2.5", "2.5.1")
    pub version: String,

    /// Message type (e.g., "ADT")
    pub message_type: String,

    /// Trigger event (e.g., "A01")
    pub trigger_event: String,

    /// Base template to inherit from (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// Segment templates
    pub segments: Vec<SegmentTemplate>,

    /// Template variables with default values
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub variables: Option<HashMap<String, String>>,
}

impl MessageTemplate {
    /// Create a new message template
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        message_type: impl Into<String>,
        trigger_event: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            version: version.into(),
            message_type: message_type.into(),
            trigger_event: trigger_event.into(),
            extends: None,
            segments: Vec::new(),
            variables: None,
        }
    }

    /// Set template description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set base template for inheritance
    pub fn with_extends(mut self, extends: impl Into<String>) -> Self {
        self.extends = Some(extends.into());
        self
    }

    /// Add a segment template
    pub fn add_segment(&mut self, segment: SegmentTemplate) {
        self.segments.push(segment);
    }

    /// Add a segment template (builder style)
    pub fn with_segment(mut self, segment: SegmentTemplate) -> Self {
        self.segments.push(segment);
        self
    }

    /// Set template variables
    pub fn with_variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables = Some(variables);
        self
    }

    /// Add a single variable with default value
    pub fn add_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
    }
}

/// A segment template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentTemplate {
    /// Segment ID (e.g., "MSH", "PID")
    pub id: String,

    /// Whether the segment is required
    #[serde(default)]
    pub required: bool,

    /// Whether the segment can repeat
    #[serde(default)]
    pub repeating: bool,

    /// Field templates (keyed by field position)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<usize, FieldTemplate>>,

    /// Segment description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl SegmentTemplate {
    /// Create a new segment template
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            required: false,
            repeating: false,
            fields: None,
            description: None,
        }
    }

    /// Mark segment as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Mark segment as repeating
    pub fn repeating(mut self) -> Self {
        self.repeating = true;
        self
    }

    /// Set segment description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a field template
    pub fn add_field(&mut self, position: usize, field: FieldTemplate) {
        self.fields
            .get_or_insert_with(HashMap::new)
            .insert(position, field);
    }

    /// Add a field template (builder style)
    pub fn with_field(mut self, position: usize, field: FieldTemplate) -> Self {
        self.fields
            .get_or_insert_with(HashMap::new)
            .insert(position, field);
        self
    }
}

/// A field template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldTemplate {
    /// Whether the field is required
    #[serde(default)]
    pub required: bool,

    /// Field data type (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datatype: Option<String>,

    /// Maximum length (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,

    /// Placeholder value or variable reference (e.g., "{{patient_name}}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    /// Default value if no placeholder provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Field description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Component templates (for composite fields)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ComponentTemplate>>,
}

impl FieldTemplate {
    /// Create a new field template
    pub fn new() -> Self {
        Self {
            required: false,
            datatype: None,
            length: None,
            placeholder: None,
            default: None,
            description: None,
            components: None,
        }
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set field data type
    pub fn with_datatype(mut self, datatype: impl Into<String>) -> Self {
        self.datatype = Some(datatype.into());
        self
    }

    /// Set maximum length
    pub fn with_length(mut self, length: usize) -> Self {
        self.length = Some(length);
        self
    }

    /// Set placeholder value
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Set field description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add component templates
    pub fn with_components(mut self, components: Vec<ComponentTemplate>) -> Self {
        self.components = Some(components);
        self
    }
}

impl Default for FieldTemplate {
    fn default() -> Self {
        Self::new()
    }
}

/// A component template (for composite fields)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTemplate {
    /// Component position (1-based)
    pub position: usize,

    /// Whether the component is required
    #[serde(default)]
    pub required: bool,

    /// Placeholder value or variable reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Component description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ComponentTemplate {
    /// Create a new component template
    pub fn new(position: usize) -> Self {
        Self {
            position,
            required: false,
            placeholder: None,
            default: None,
            description: None,
        }
    }

    /// Mark component as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set placeholder value
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Set component description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_template_creation() {
        let template = MessageTemplate::new("Basic ADT", "2.5", "ADT", "A01")
            .with_description("Basic admission message");

        assert_eq!(template.name, "Basic ADT");
        assert_eq!(template.version, "2.5");
        assert_eq!(template.message_type, "ADT");
        assert_eq!(template.trigger_event, "A01");
        assert_eq!(template.description, Some("Basic admission message".to_string()));
    }

    #[test]
    fn test_segment_template_creation() {
        let segment = SegmentTemplate::new("PID")
            .required()
            .with_description("Patient identification");

        assert_eq!(segment.id, "PID");
        assert!(segment.required);
        assert!(!segment.repeating);
        assert_eq!(segment.description, Some("Patient identification".to_string()));
    }

    #[test]
    fn test_field_template_creation() {
        let field = FieldTemplate::new()
            .required()
            .with_datatype("XPN")
            .with_placeholder("{{patient_name}}")
            .with_length(250);

        assert!(field.required);
        assert_eq!(field.datatype, Some("XPN".to_string()));
        assert_eq!(field.placeholder, Some("{{patient_name}}".to_string()));
        assert_eq!(field.length, Some(250));
    }

    #[test]
    fn test_component_template_creation() {
        let component = ComponentTemplate::new(1)
            .required()
            .with_placeholder("{{last_name}}")
            .with_description("Last name");

        assert_eq!(component.position, 1);
        assert!(component.required);
        assert_eq!(component.placeholder, Some("{{last_name}}".to_string()));
        assert_eq!(component.description, Some("Last name".to_string()));
    }

    #[test]
    fn test_template_with_segments_and_fields() {
        let mut template = MessageTemplate::new("ADT A01", "2.5", "ADT", "A01");

        let mut pid_segment = SegmentTemplate::new("PID").required();
        pid_segment.add_field(
            5,
            FieldTemplate::new()
                .required()
                .with_placeholder("{{patient_name}}")
        );

        template.add_segment(pid_segment);

        assert_eq!(template.segments.len(), 1);
        assert_eq!(template.segments[0].id, "PID");
        assert!(template.segments[0].fields.is_some());
    }
}
