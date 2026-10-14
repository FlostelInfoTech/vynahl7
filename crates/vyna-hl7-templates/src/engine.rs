//! Template engine for creating messages from templates.

use crate::{Error, MessageTemplate, Result};
use std::collections::HashMap;
use vyna_hl7_core::{
    Component, Delimiters, Encoding, Field, Message, Repetition, Segment, SubComponent,
};

/// Maximum field or component position accepted from a template.
///
/// Positions size gap-filled vectors directly, so an unbounded
/// position (e.g. from an untrusted YAML or JSON template) would
/// exhaust memory; anything beyond this cap is rejected with an
/// error instead. Real HL7 segments hold dozens of fields at most.
const MAX_TEMPLATE_POSITION: usize = 1000;

/// Template engine for creating messages from templates
pub struct TemplateEngine {
    /// Variables for substitution
    variables: HashMap<String, String>,
    /// Escape substituted variable values as literal text
    literal_variables: bool,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            literal_variables: false,
        }
    }

    /// Treat substituted variable values as literal text
    ///
    /// By default a substituted value is parsed as an ER7 fragment, so
    /// `~^&` characters in untrusted input create field structure. With
    /// literal mode enabled, variable values are ER7-escaped before
    /// substitution and land as literal text instead, while structure
    /// written directly in the template is preserved. Component
    /// templates already store values literally and are unaffected.
    pub fn set_literal_variables(&mut self, literal: bool) {
        self.literal_variables = literal;
    }

    /// Treat substituted variable values as literal text (builder style)
    ///
    /// See [`set_literal_variables`](Self::set_literal_variables).
    pub fn with_literal_variables(mut self, literal: bool) -> Self {
        self.literal_variables = literal;
        self
    }

    /// Whether substituted variable values are treated as literal text
    pub fn literal_variables(&self) -> bool {
        self.literal_variables
    }

    /// Escape free text so it survives template substitution literally
    ///
    /// The escaped value can be substituted with the default (ER7
    /// fragment) mode and still lands as literal text, which is useful
    /// when only some values are untrusted.
    pub fn escape_literal(value: &str) -> String {
        Encoding::encode(value, &Delimiters::default())
    }

    /// Set a variable value
    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    /// Set multiple variables
    pub fn set_variables(&mut self, variables: HashMap<String, String>) {
        self.variables.extend(variables);
    }

    /// Get a variable value
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Create a message from a template
    ///
    /// ## Required fields
    ///
    /// `required` is template completeness: every `{{variable}}` must
    /// be bound (an unbound variable is an error), but a bound
    /// variable may still render empty — `{{name}}` with `name=""`
    /// yields an empty field, not an error. The components path
    /// additionally rejects a required field whose components all
    /// render empty.
    pub fn create_message(&self, template: &MessageTemplate) -> Result<Message> {
        // Merge template default variables with instance variables
        let mut all_vars = template.variables.clone().unwrap_or_default();
        all_vars.extend(self.variables.clone());

        // Create message with segments
        let mut segments = Vec::new();

        for seg_template in &template.segments {
            let segment = self.create_segment(seg_template, &all_vars)?;
            segments.push(segment);
        }

        // Fill routing fields from the template metadata when the template
        // leaves them empty, so the message carries its own identity.
        // Whole-field emptiness (not first-component) decides: a "^A08"
        // default holds content in its second component and must win.
        if let Some(msh) = segments.iter_mut().find(|s| s.id == "MSH") {
            let msh9_empty = msh.get_field(9).map(|f| f.is_empty()).unwrap_or(true);
            if msh9_empty {
                let mut message_type = Field::new();
                let mut repetition = Repetition::new();
                repetition.add_component(Component::from_value(&template.message_type));
                repetition.add_component(Component::from_value(&template.trigger_event));
                message_type.add_repetition(repetition);
                msh.set_field(9, message_type)
                    .map_err(|e| Error::substitution(e.to_string()))?;
            }
            let msh12_empty = msh.get_field(12).map(|f| f.is_empty()).unwrap_or(true);
            if msh12_empty {
                msh.set_field(12, Field::from_value(template.version.clone()))
                    .map_err(|e| Error::substitution(e.to_string()))?;
            }
        }

        // Create the message
        let message = Message {
            segments,
            delimiters: Delimiters::default(),
        };

        Ok(message)
    }

    /// Create a segment from a segment template
    fn create_segment(
        &self,
        seg_template: &crate::SegmentTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<Segment> {
        // Template positions follow HL7 numbering: position N lands in
        // fields[N-1], matching parsed messages, get_field, and encode.
        // (For MSH/FHS/BHS, encode skips fields[0] and fields[1], which
        // are the separator and encoding characters, never template
        // positions.)
        let mut fields = Vec::new();

        // Process field templates if present
        if let Some(field_templates) = &seg_template.fields {
            if field_templates.contains_key(&0) {
                return Err(Error::substitution(
                    "Field position must be 1-based (got 0)",
                ));
            }
            if matches!(seg_template.id.as_str(), "MSH" | "FHS" | "BHS")
                && (field_templates.contains_key(&1) || field_templates.contains_key(&2))
            {
                // MSH/FHS/BHS fields 1 and 2 are the delimiters: encode
                // derives them from the message delimiters and drops
                // anything built here, so templating them is rejected,
                // not ignored.
                let id = seg_template.id.as_str();
                return Err(Error::substitution(format!(
                    "{id}-1 and {id}-2 hold the message delimiters and \
                     cannot be set from a template (delimiters come \
                     from encoding)",
                )));
            }
            // Get the maximum field position to size the fields vector
            let max_pos = field_templates.keys().max().copied().unwrap_or(0);
            if max_pos > MAX_TEMPLATE_POSITION {
                return Err(Error::substitution(format!(
                    "Field position {max_pos} exceeds maximum of {MAX_TEMPLATE_POSITION}"
                )));
            }

            // Create fields up to max position (1-indexed in template, 0-indexed in vector)
            for pos in 1..=max_pos {
                if let Some(field_template) = field_templates.get(&pos) {
                    let field = self.create_field(field_template, variables)?;
                    fields.push(field);
                } else {
                    // Empty field for positions without templates
                    fields.push(Field::new());
                }
            }
        }

        // MSH/FHS/BHS positions 1 and 2 are the delimiter
        // placeholders, which no template may set (rejected above):
        // fill them from the message delimiters like the parser does,
        // so built headers have the same shape as parsed ones.
        if matches!(seg_template.id.as_str(), "MSH" | "FHS" | "BHS") {
            let delimiters = Delimiters::default();
            while fields.len() < 2 {
                fields.push(Field::new());
            }
            fields[0] = Field::from_value(delimiters.field_separator.to_string());
            fields[1] = Field::from_value(delimiters.encoding_characters());
        }

        Ok(Segment {
            id: seg_template.id.clone(),
            fields,
        })
    }

    /// Create a field from a field template
    fn create_field(
        &self,
        field_template: &crate::FieldTemplate,
        variables: &HashMap<String, String>,
    ) -> Result<Field> {
        // Handle component templates if present
        if let Some(components) = &field_template.components {
            // A field-level placeholder/default plus components would
            // silently drop the scalar value (components win); reject
            // the combination instead of losing data.
            if field_template.placeholder.is_some() || field_template.default.is_some() {
                return Err(Error::substitution(
                    "FieldTemplate cannot combine placeholder/default with components",
                ));
            }
            // Positions are 1-based: honor them (including gaps) rather
            // than pushing templates in vector order. A repeated position
            // is overwritten by the later template.
            let max_pos = components.iter().map(|c| c.position).max().unwrap_or(0);
            if max_pos > MAX_TEMPLATE_POSITION {
                return Err(Error::substitution(format!(
                    "Component position {max_pos} exceeds maximum of {MAX_TEMPLATE_POSITION}"
                )));
            }
            let mut comp_vec = vec![Component::new(); max_pos];

            for comp_template in components {
                if comp_template.position == 0 {
                    return Err(Error::substitution(
                        "Component position must be 1-based (got 0)",
                    ));
                }
                let comp_value = if let Some(placeholder) = &comp_template.placeholder {
                    // Component values are stored literally already, so
                    // they are never escaped here.
                    self.substitute_variables(placeholder, variables, false)?
                } else if let Some(default) = &comp_template.default {
                    default.clone()
                } else if comp_template.required {
                    return Err(Error::substitution(
                        "Required component has no placeholder or default value",
                    ));
                } else {
                    String::new()
                };

                comp_vec[comp_template.position - 1] = Component::from_value(comp_value);
            }

            // Create a field with one repetition containing the components
            let mut field = Field::new();
            field.add_repetition(Repetition {
                components: comp_vec,
            });
            // Field-level `required` additionally rejects an all-empty
            // render here (the simple-field path allows empty
            // substitution instead).
            if field_template.required && field.is_empty() {
                return Err(Error::substitution(
                    "Required field rendered empty (all components empty)",
                ));
            }
            Ok(field)
        } else {
            // Simple field with single value. A bound-but-empty value
            // renders an empty field even when required (completeness,
            // not a non-empty check).
            let value = if let Some(placeholder) = &field_template.placeholder {
                let escape = self.literal_variables;
                self.substitute_variables(placeholder, variables, escape)?
            } else if let Some(default) = &field_template.default {
                default.clone()
            } else if field_template.required {
                return Err(Error::substitution(
                    "Required field has no placeholder or default value",
                ));
            } else {
                return Ok(Field::new()); // Empty field
            };

            Self::parse_field_value(&value)
        }
    }

    /// Parse a substituted field value into a structured field.
    ///
    /// Template values are ER7 fragments (`DOE^JOHN`, `a&b`, `x~y`), so
    /// they are split on the standard delimiters and escape-decoded,
    /// mirroring the message parser. An empty value stays an empty field.
    fn parse_field_value(value: &str) -> Result<Field> {
        if value.is_empty() {
            return Ok(Field::new());
        }
        let delimiters = Delimiters::default();
        let mut field = Field::new();
        for repetition_text in value.split(delimiters.repetition_separator) {
            let mut repetition = Repetition::new();
            for component_text in repetition_text.split(delimiters.component_separator) {
                let mut component = Component::new();
                for sub_text in component_text.split(delimiters.subcomponent_separator) {
                    let decoded = Encoding::decode(sub_text, &delimiters)
                        .map_err(|e| Error::substitution(e.to_string()))?;
                    component.add_subcomponent(SubComponent::new(decoded));
                }
                repetition.add_component(component);
            }
            field.add_repetition(repetition);
        }
        Ok(field)
    }

    /// Substitute variables in a placeholder string
    /// Supports {{variable_name}} syntax (inner whitespace allowed).
    /// When `escape_values` is set, each substituted value is
    /// ER7-escaped so it lands as literal text instead of structure.
    /// Expansion is a single pass: substituted values are copied
    /// verbatim and never rescanned, so a value containing
    /// `{{another}}` cannot double-expand.
    /// A `{{` opener with no later `}}`, a nested opener, or an empty
    /// variable name (`{{}}`, `{{ }}`) is an error, never literal text.
    fn substitute_variables(
        &self,
        placeholder: &str,
        variables: &HashMap<String, String>,
        escape_values: bool,
    ) -> Result<String> {
        let mut result = String::with_capacity(placeholder.len());
        let mut rest = placeholder;
        while let Some(start) = rest.find("{{") {
            result.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let Some(end) = after.find("}}") else {
                return Err(Error::substitution(format!(
                    "Unclosed placeholder '{{{{...}}' in template: {placeholder}"
                )));
            };
            let candidate = &after[..end];
            if candidate.contains("{{") {
                // A nested opener leaves the outer one unclosed.
                return Err(Error::substitution(format!(
                    "Unclosed placeholder '{{{{...}}' in template: {placeholder}"
                )));
            }
            let var_name = candidate.trim();
            if var_name.is_empty() {
                return Err(Error::substitution(
                    "Placeholder '{{}}' has an empty variable name",
                ));
            }
            let var_value = variables
                .get(var_name)
                .ok_or_else(|| Error::substitution(format!("Variable '{var_name}' not found")))?;

            if escape_values {
                result.push_str(&Self::escape_literal(var_value));
            } else {
                result.push_str(var_value);
            }
            rest = &after[end + 2..];
        }
        result.push_str(rest);

        Ok(result)
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentTemplate, FieldTemplate, SegmentTemplate};

    #[test]
    fn test_variable_substitution() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "John Doe".to_string());
        vars.insert("id".to_string(), "12345".to_string());

        let result = engine.substitute_variables("Patient {{name}} has ID {{id}}", &vars, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Patient John Doe has ID 12345");
    }

    #[test]
    fn test_variable_substitution_with_surrounding_spaces() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("facility".to_string(), "General".to_string());

        let result = engine.substitute_variables("Site: {{ facility }}!", &vars, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Site: General!");
    }

    #[test]
    fn test_create_field_honors_component_positions() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        // Out of order with a gap: position 2 is intentionally absent.
        let name_field = FieldTemplate::new().with_components(vec![
            ComponentTemplate::new(3).with_placeholder("{{third}}"),
            ComponentTemplate::new(1).with_placeholder("{{first}}"),
        ]);
        pid_segment.add_field(5, name_field);
        template.add_segment(pid_segment);

        let mut engine = TemplateEngine::new();
        engine.set_variable("first", "Doe");
        engine.set_variable("third", "Jr");

        let message = engine.create_message(&template).unwrap();
        let components = &message.segments[0].fields[4].repetitions[0].components;
        assert_eq!(components.len(), 3);
        assert_eq!(components[0].value(), Some("Doe"));
        assert_eq!(components[1].value(), None);
        assert_eq!(components[2].value(), Some("Jr"));
    }

    #[test]
    fn test_create_field_rejects_component_position_zero() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(
            5,
            FieldTemplate::new().with_components(vec![ComponentTemplate::new(0).with_default("x")]),
        );
        template.add_segment(pid_segment);

        let engine = TemplateEngine::new();
        assert!(engine.create_message(&template).is_err());
    }

    #[test]
    fn test_missing_variable() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let result = engine.substitute_variables("Patient {{name}}", &vars, false);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Variable 'name' not found")
        );
    }

    #[test]
    fn test_create_simple_message() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        // Add MSH segment with sending application
        let mut msh_segment = SegmentTemplate::new("MSH").required();
        msh_segment.add_field(
            3,
            FieldTemplate::new()
                .required()
                .with_placeholder("{{sending_app}}"),
        );
        template.add_segment(msh_segment);

        // Create engine with variables
        let mut engine = TemplateEngine::new();
        engine.set_variable("sending_app", "TestApp");

        let message = engine.create_message(&template);
        assert!(message.is_ok());

        let msg = message.unwrap();
        assert_eq!(msg.segments.len(), 1);
        assert_eq!(msg.segments[0].id, "MSH");
        assert_eq!(msg.segments[0].fields[2].value(), Some("TestApp"));
    }

    #[test]
    fn test_create_message_with_default_values() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        // Add PID segment with default sex
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(
            8,
            FieldTemplate::new().with_default("U"), // Unknown
        );
        template.add_segment(pid_segment);

        let engine = TemplateEngine::new();
        let message = engine.create_message(&template);
        assert!(message.is_ok());

        let msg = message.unwrap();
        assert_eq!(msg.segments[0].fields[7].value(), Some("U"));
    }

    #[test]
    fn test_create_message_with_components() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        // Add PID segment with patient name components
        let mut pid_segment = SegmentTemplate::new("PID");

        let name_field = FieldTemplate::new().required().with_components(vec![
            ComponentTemplate::new(1)
                .required()
                .with_placeholder("{{last_name}}"),
            ComponentTemplate::new(2)
                .required()
                .with_placeholder("{{first_name}}"),
        ]);

        pid_segment.add_field(5, name_field);
        template.add_segment(pid_segment);

        let mut engine = TemplateEngine::new();
        engine.set_variable("last_name", "Doe");
        engine.set_variable("first_name", "John");

        let message = engine.create_message(&template);
        assert!(message.is_ok());

        let msg = message.unwrap();
        assert_eq!(msg.segments[0].fields[4].repetitions.len(), 1);
        assert_eq!(msg.segments[0].fields[4].repetitions[0].components.len(), 2);
        assert_eq!(
            msg.segments[0].fields[4].repetitions[0].components[0].value(),
            Some("Doe")
        );
        assert_eq!(
            msg.segments[0].fields[4].repetitions[0].components[1].value(),
            Some("John")
        );
    }

    #[test]
    fn test_missing_required_field() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(
            3,
            FieldTemplate::new().required(), // No placeholder or default
        );
        template.add_segment(pid_segment);

        let engine = TemplateEngine::new();
        let result = engine.create_message(&template);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_default_variables() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        // Set template-level default variable
        template.add_variable("facility", "DefaultHospital");

        let mut msh_segment = SegmentTemplate::new("MSH");
        msh_segment.add_field(4, FieldTemplate::new().with_placeholder("{{facility}}"));
        template.add_segment(msh_segment);

        // Create engine without setting facility variable
        let engine = TemplateEngine::new();
        let message = engine.create_message(&template);

        assert!(message.is_ok());
        let msg = message.unwrap();
        assert_eq!(msg.segments[0].fields[3].value(), Some("DefaultHospital"));
    }

    #[test]
    fn test_engine_variables_override_template_defaults() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        template.add_variable("facility", "DefaultHospital");

        let mut msh_segment = SegmentTemplate::new("MSH");
        msh_segment.add_field(4, FieldTemplate::new().with_placeholder("{{facility}}"));
        template.add_segment(msh_segment);

        // Engine variable should override template default
        let mut engine = TemplateEngine::new();
        engine.set_variable("facility", "CustomHospital");

        let message = engine.create_message(&template);
        assert!(message.is_ok());

        let msg = message.unwrap();
        assert_eq!(msg.segments[0].fields[3].value(), Some("CustomHospital"));
    }

    #[test]
    fn test_built_message_encodes_with_hl7_numbering() {
        let library = crate::TemplateLibrary::new();
        let template = library.get("ADT_A01").unwrap();
        let mut engine = TemplateEngine::new();
        for name in [
            "sending_app",
            "sending_facility",
            "receiving_app",
            "receiving_facility",
            "event_datetime",
            "patient_id",
            "patient_name",
            "date_of_birth",
            "sex",
            "address",
            "patient_class",
            "assigned_location",
            "attending_doctor",
            "visit_number",
        ] {
            engine.set_variable(name, format!("V_{name}"));
        }

        let message = engine.create_message(template).unwrap();
        let text = message.encode_with_separator("\n");
        let msh = text.lines().next().unwrap();
        let fields: Vec<&str> = msh.split('|').collect();
        // MSH-3 holds the sending app (not shifted), MSH-9/MSH-12 come
        // from the template metadata.
        assert_eq!(fields[2], "V_sending_app");
        assert_eq!(fields[8], "ADT^A01");
        assert_eq!(fields[11], "2.5");

        // Regular segments carry no duplicated ID: EVN-1 is first.
        let evn = text.lines().nth(1).unwrap();
        assert!(evn.starts_with("EVN|A01|"), "unexpected EVN: {evn}");

        // Positions match get_field numbering.
        let pid = message.get_segment_by_id("PID").unwrap();
        assert_eq!(pid.get_field_value(3), Some("V_patient_id"));
    }

    #[test]
    fn test_template_msh9_and_version_are_not_overwritten() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut msh = SegmentTemplate::new("MSH");
        msh.add_field(9, FieldTemplate::new().with_default("ADT^A08"));
        msh.add_field(12, FieldTemplate::new().with_default("2.3"));
        template.add_segment(msh);

        let message = TemplateEngine::new().create_message(&template).unwrap();
        let msh = message.get_segment_by_id("MSH").unwrap();
        // The explicit default wins over autofill, parsed into components.
        let field9 = msh.get_field(9).unwrap();
        let components = &field9.repetitions[0].components;
        assert_eq!(components[0].value(), Some("ADT"));
        assert_eq!(components[1].value(), Some("A08"));
        assert_eq!(msh.get_field_value(12), Some("2.3"));
    }

    #[test]
    fn test_substituted_values_parse_as_er7_fragments() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(5, FieldTemplate::new().with_placeholder("{{patient_name}}"));
        template.add_segment(pid);

        let mut engine = TemplateEngine::new();
        engine.set_variable("patient_name", "DOE^JOHN");
        let message = engine.create_message(&template).unwrap();
        let pid = message.get_segment_by_id("PID").unwrap();
        let components = &pid.get_field(5).unwrap().repetitions[0].components;
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].value(), Some("DOE"));
        assert_eq!(components[1].value(), Some("JOHN"));

        // Escapes survive the round trip as literal content.
        let mut engine = TemplateEngine::new();
        engine.set_variable("patient_name", "A\\S\\B");
        let message = engine.create_message(&template).unwrap();
        let pid = message.get_segment_by_id("PID").unwrap();
        let components = &pid.get_field(5).unwrap().repetitions[0].components;
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].value(), Some("A^B"));
    }

    #[test]
    fn test_literal_variables_keep_structure_characters_literal() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        template.add_segment(SegmentTemplate::new("MSH").required());
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(5, FieldTemplate::new().with_placeholder("{{name}}"));
        template.add_segment(pid);

        let mut engine = TemplateEngine::new().with_literal_variables(true);
        assert!(engine.literal_variables());
        engine.set_variable("name", "DOE^JOHN~EVIL&A\\B|C");
        let message = engine.create_message(&template).unwrap();
        let field = message
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(5)
            .unwrap();

        // Untrusted delimiters create no structure: one repetition,
        // one component holding the exact input text.
        assert_eq!(field.repetitions.len(), 1);
        assert_eq!(field.repetitions[0].components.len(), 1);
        assert_eq!(
            field.repetitions[0].components[0].value(),
            Some("DOE^JOHN~EVIL&A\\B|C")
        );
        // The encoded message round-trips back to the same value.
        let parsed = vyna_hl7_parser::parse_message(&message.encode()).unwrap();
        let reparsed = parsed
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(5)
            .unwrap();
        assert_eq!(reparsed.value(), field.value());
    }

    #[test]
    fn test_literal_variables_preserve_template_structure() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(5, FieldTemplate::new().with_placeholder("X^{{v}}"));
        template.add_segment(pid);

        let mut engine = TemplateEngine::new();
        engine.set_literal_variables(true);
        engine.set_variable("v", "a^b");
        let message = engine.create_message(&template).unwrap();
        let components = &message
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(5)
            .unwrap()
            .repetitions[0]
            .components;

        // The template's own separator still splits; only the
        // substituted value is literal.
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].value(), Some("X"));
        assert_eq!(components[1].value(), Some("a^b"));
    }

    #[test]
    fn test_escape_literal_helper_for_default_mode() {
        assert_eq!(
            TemplateEngine::escape_literal("A^B~C&D|E\\F"),
            "A\\S\\B\\R\\C\\T\\D\\F\\E\\E\\F"
        );

        // A pre-escaped value stays literal even in default mode.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(5, FieldTemplate::new().with_placeholder("{{v}}"));
        template.add_segment(pid);

        let mut engine = TemplateEngine::new();
        engine.set_variable("v", TemplateEngine::escape_literal("a^b"));
        let message = engine.create_message(&template).unwrap();
        let field = message
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(5)
            .unwrap();
        assert_eq!(field.repetitions.len(), 1);
        assert_eq!(field.repetitions[0].components.len(), 1);
        assert_eq!(field.value(), Some("a^b"));
    }

    #[test]
    fn test_create_message_rejects_huge_component_position() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(
            5,
            FieldTemplate::new()
                .with_components(vec![ComponentTemplate::new(1_000_000).with_default("x")]),
        );
        template.add_segment(pid);

        let engine = TemplateEngine::new();
        let result = engine.create_message(&template);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[test]
    fn test_create_message_rejects_huge_field_position() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(500_000, FieldTemplate::new().with_default("x"));
        template.add_segment(pid);

        let engine = TemplateEngine::new();
        let result = engine.create_message(&template);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exceeds maximum"));
    }

    #[test]
    fn test_create_message_rejects_field_position_zero() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(0, FieldTemplate::new().with_default("x"));
        template.add_segment(pid);

        let engine = TemplateEngine::new();
        assert!(engine.create_message(&template).is_err());
    }

    #[test]
    fn test_create_message_accepts_boundary_positions() {
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(
            5,
            FieldTemplate::new()
                .with_components(vec![ComponentTemplate::new(1000).with_default("x")]),
        );
        template.add_segment(pid);

        let engine = TemplateEngine::new();
        let message = engine.create_message(&template).unwrap();
        let components = &message
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(5)
            .unwrap()
            .repetitions[0]
            .components;
        assert_eq!(components.len(), 1000);
        assert_eq!(components[999].value(), Some("x"));
    }

    #[test]
    fn test_substitution_does_not_reprocess_values() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("a".to_string(), "{{b}}".to_string());
        vars.insert("b".to_string(), "X".to_string());

        // Single pass: the substituted "{{b}}" is copied verbatim,
        // not expanded again.
        let result = engine.substitute_variables("{{a}} {{b}}", &vars, false);
        assert_eq!(result.unwrap(), "{{b}} X");

        // Repeated placeholders still each expand.
        let result = engine.substitute_variables("{{b}} {{b}}", &vars, false);
        assert_eq!(result.unwrap(), "X X");
    }

    #[test]
    fn test_engine_msh_carries_delimiter_placeholders() {
        // Like the parser, engine-built MSH holds the delimiter
        // placeholders at fields 1 and 2, so MSH-1/MSH-2 readers see
        // the same shape on built and parsed messages.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut msh = SegmentTemplate::new("MSH");
        msh.add_field(3, FieldTemplate::new().with_default("APP"));
        template.add_segment(msh);

        let message = TemplateEngine::new().create_message(&template).unwrap();
        let msh = message.get_segment_by_id("MSH").unwrap();
        assert_eq!(msh.get_field_value(1), Some("|"));
        assert_eq!(msh.get_field_value(2), Some("^~\\&"));
        assert_eq!(msh.get_field_value(3), Some("APP"));
    }

    #[test]
    fn test_engine_bare_msh_carries_delimiter_placeholders() {
        // An MSH template without fields still yields the two
        // delimiter placeholders, matching parsed MSH shape.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        template.add_segment(SegmentTemplate::new("MSH"));

        let message = TemplateEngine::new().create_message(&template).unwrap();
        let msh = message.get_segment_by_id("MSH").unwrap();
        assert_eq!(msh.get_field(1).and_then(|f| f.value()), Some("|"));
        assert_eq!(msh.get_field(2).and_then(|f| f.value()), Some("^~\\&"));
    }

    #[test]
    fn test_create_message_rejects_msh_delimiter_fields() {
        for pos in [1, 2] {
            let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
            let mut msh = SegmentTemplate::new("MSH");
            msh.add_field(pos, FieldTemplate::new().with_default("X"));
            template.add_segment(msh);

            let engine = TemplateEngine::new();
            let result = engine.create_message(&template);
            assert!(result.is_err(), "MSH-{pos} must be rejected");
            assert!(
                result.unwrap_err().to_string().contains("delimiters"),
                "MSH-{pos} error must name the delimiters"
            );
        }

        // Other segments keep position 1: only MSH/FHS/BHS reserve it.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(1, FieldTemplate::new().with_default("1"));
        template.add_segment(pid);
        let message = TemplateEngine::new().create_message(&template).unwrap();
        assert_eq!(
            message.get_segment_by_id("PID").unwrap().get_field_value(1),
            Some("1")
        );
    }

    #[test]
    fn test_create_message_rejects_fhs_bhs_delimiter_fields() {
        // Like MSH, FHS/BHS fields 1 and 2 are the delimiters: encode
        // skips the stored fields, so templating them is rejected.
        for id in ["FHS", "BHS"] {
            for pos in [1, 2] {
                let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
                let mut header = SegmentTemplate::new(id);
                header.add_field(pos, FieldTemplate::new().with_default("X"));
                template.add_segment(header);

                let engine = TemplateEngine::new();
                let result = engine.create_message(&template);
                assert!(result.is_err(), "{id}-{pos} must be rejected");
                assert!(
                    result.unwrap_err().to_string().contains("delimiters"),
                    "{id}-{pos} error must name the delimiters"
                );
            }
        }
    }

    #[test]
    fn test_engine_fhs_bhs_carry_delimiter_placeholders() {
        // Like the parser, engine-built FHS/BHS hold the delimiter
        // placeholders at fields 1 and 2, so readers see the same
        // shape on built and parsed headers.
        for id in ["FHS", "BHS"] {
            let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
            let mut header = SegmentTemplate::new(id);
            header.add_field(3, FieldTemplate::new().with_default("APP"));
            template.add_segment(header);

            let message = TemplateEngine::new().create_message(&template).unwrap();
            let built = message.get_segment_by_id(id).unwrap();
            assert_eq!(built.get_field_value(1), Some("|"));
            assert_eq!(built.get_field_value(2), Some("^~\\&"));
            assert_eq!(built.get_field_value(3), Some("APP"));
        }
    }

    #[test]
    fn test_msh9_partial_default_survives_autofill() {
        // "^A08" holds content in its second component: whole-field
        // emptiness must preserve it instead of autofilling ADT^A01.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut msh = SegmentTemplate::new("MSH");
        msh.add_field(9, FieldTemplate::new().with_default("^A08"));
        template.add_segment(msh);
        let message = TemplateEngine::new().create_message(&template).unwrap();
        let field9 = message
            .get_segment_by_id("MSH")
            .unwrap()
            .get_field(9)
            .unwrap();
        let comps = &field9.repetitions[0].components;
        assert_eq!(comps[0].value(), Some(""));
        assert_eq!(comps[1].value(), Some("A08"));
    }

    #[test]
    fn test_required_field_with_empty_components_fails() {
        // Field-level `required` applies in the components path too.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(
            5,
            FieldTemplate::new()
                .required()
                .with_components(vec![ComponentTemplate::new(1)]),
        );
        template.add_segment(pid);
        assert!(TemplateEngine::new().create_message(&template).is_err());
    }

    #[test]
    fn test_field_placeholder_with_components_is_rejected() {
        // Scalar placeholder/default plus components would silently drop
        // the scalar value; the combination is an error.
        for field in [
            FieldTemplate::new()
                .with_placeholder("SHOULD_APPEAR")
                .with_components(vec![ComponentTemplate::new(1).with_default("C")]),
            FieldTemplate::new()
                .with_default("SHOULD_APPEAR")
                .with_components(vec![ComponentTemplate::new(1).with_default("C")]),
        ] {
            let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
            let mut pid = SegmentTemplate::new("PID");
            pid.add_field(5, field);
            template.add_segment(pid);
            assert!(TemplateEngine::new().create_message(&template).is_err());
        }
    }

    #[test]
    fn test_required_scalar_renders_empty_when_bound() {
        // `required` is template completeness (all variables bound),
        // not a non-empty check: {{name}} with name="" renders an
        // empty field successfully. The testpanel builder relies on
        // this: it binds every variable, empty when unfilled.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(
            5,
            FieldTemplate::new().required().with_placeholder("{{name}}"),
        );
        template.add_segment(pid);
        let mut engine = TemplateEngine::new();
        engine.set_variable("name", "");
        let message = engine.create_message(&template).unwrap();
        assert!(message.segments[0].get_field(5).unwrap().is_empty());
    }

    #[test]
    fn test_substitution_rejects_unclosed_placeholder() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "X".to_string());

        // An unclosed opener must error, never ship literally.
        for bad in ["Patient {{name}", "{{name", "A {{name}} C {{oops"] {
            let err = engine
                .substitute_variables(bad, &vars, false)
                .unwrap_err()
                .to_string();
            assert!(err.contains("Unclosed"), "unexpected: {err}");
        }
        // A nested opener leaves the outer one unclosed too.
        let err = engine
            .substitute_variables("{{a {{b}}", &vars, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unclosed"), "unexpected: {err}");
    }

    #[test]
    fn test_substitution_rejects_empty_variable_name() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        // "{{}}" and "{{ }}" are the same mistake: both error.
        for bad in ["{{}}", "{{ }}", "X {{  }} Y"] {
            let err = engine
                .substitute_variables(bad, &vars, false)
                .unwrap_err()
                .to_string();
            assert!(err.contains("empty"), "unexpected: {err}");
        }
    }

    #[test]
    fn test_create_message_rejects_unclosed_placeholder() {
        // End to end: the broken opener never lands in PID-5.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(
            5,
            FieldTemplate::new().with_placeholder("Patient {{name}"),
        );
        template.add_segment(pid);

        let mut engine = TemplateEngine::new();
        engine.set_variable("name", "X");
        let err = engine.create_message(&template).unwrap_err().to_string();
        assert!(err.contains("Unclosed"), "unexpected: {err}");
    }
}
