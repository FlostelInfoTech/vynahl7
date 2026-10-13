//! Template validation for messages.

use crate::MessageTemplate;
use vyna_hl7_core::Message;

/// Validation result for a message against a template
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the message is valid
    pub valid: bool,

    /// List of validation errors
    pub errors: Vec<ValidationError>,

    /// List of validation warnings
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Create a new valid result
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add an error to the result
    pub fn add_error(&mut self, error: ValidationError) {
        self.valid = false;
        self.errors.push(error);
    }

    /// Add a warning to the result
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Check if the result has any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if the result has any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// A validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error message
    pub message: String,

    /// Location in message (e.g., "PID-5")
    pub location: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
        }
    }

    /// Set the location
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

/// A validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,

    /// Location in message (e.g., "PID-5")
    pub location: Option<String>,
}

impl ValidationWarning {
    /// Create a new validation warning
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
        }
    }

    /// Set the location
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

/// Validator for messages against templates
pub struct TemplateValidator;

impl TemplateValidator {
    /// Validate a message against a template
    ///
    /// Checks the message type/trigger event, required segments, required
    /// fields, required components (1-based positions), and field length
    /// limits. The `datatype` tag on [`crate::FieldTemplate`] is
    /// informational only and is not enforced.
    ///
    /// # Example
    ///
    /// ```
    /// use vyna_hl7_templates::{TemplateValidator, MessageTemplate};
    /// use vyna_hl7_core::Message;
    ///
    /// let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
    /// let message = Message::new();
    ///
    /// let result = TemplateValidator::validate(&message, &template);
    /// ```
    pub fn validate(message: &Message, template: &MessageTemplate) -> ValidationResult {
        let mut result = ValidationResult::valid();

        // Validate message type and trigger event
        if let Some(msh) = message.segments.iter().find(|s| s.id == "MSH") {
            // MSH-9 holds the message type (component 1) and trigger
            // event (component 2). Read the components structurally:
            // get_field_value returns only the first component, so
            // splitting it on '^' can never see the trigger event on
            // real parsed messages.
            let repetition = msh.get_field(9).and_then(|f| f.get_repetition(0));
            let msg_type = repetition
                .and_then(|r| r.get_component(0))
                .and_then(|c| c.value());
            let trigger = repetition
                .and_then(|r| r.get_component(1))
                .and_then(|c| c.value());
            match msg_type {
                Some(t) if t == template.message_type => {}
                Some(t) => {
                    result.add_error(
                        ValidationError::new(format!(
                            "Message type mismatch: expected {}, got {}",
                            template.message_type, t
                        ))
                        .with_location("MSH-9.1"),
                    );
                }
                // An absent MSH-9 cannot pass the check: like an empty
                // one, it fails instead of silently skipping.
                None => {
                    result.add_error(
                        ValidationError::new(format!(
                            "Message type is missing: expected {}",
                            template.message_type
                        ))
                        .with_location("MSH-9.1"),
                    );
                }
            }
            match trigger {
                Some(t) if t == template.trigger_event => {}
                Some(t) => {
                    result.add_error(
                        ValidationError::new(format!(
                            "Trigger event mismatch: expected {}, got {}",
                            template.trigger_event, t
                        ))
                        .with_location("MSH-9.2"),
                    );
                }
                None => {
                    result.add_error(
                        ValidationError::new(format!(
                            "Trigger event is missing: expected {}",
                            template.trigger_event
                        ))
                        .with_location("MSH-9.2"),
                    );
                }
            }
        } else {
            result.add_error(ValidationError::new("MSH segment is required"));
        }

        // Validate required segments
        for seg_template in &template.segments {
            if seg_template.required {
                let found = message.segments.iter().any(|s| s.id == seg_template.id);
                if !found {
                    result.add_error(ValidationError::new(format!(
                        "Required segment '{}' not found",
                        seg_template.id
                    )));
                }
            }

            // If segment has field templates, validate them
            if let Some(field_templates) = &seg_template.fields {
                // Find all matching segments
                let matching_segments: Vec<&vyna_hl7_core::Segment> = message
                    .segments
                    .iter()
                    .filter(|s| s.id == seg_template.id)
                    .collect();

                for segment in matching_segments {
                    Self::validate_segment_fields(segment, field_templates, &mut result);
                }
            }
        }

        result
    }

    /// Validate segment fields against field templates
    fn validate_segment_fields(
        segment: &vyna_hl7_core::Segment,
        field_templates: &std::collections::HashMap<usize, crate::FieldTemplate>,
        result: &mut ValidationResult,
    ) {
        for (pos, field_template) in field_templates {
            let location = format!("{}-{}", segment.id, pos);

            // Check if field exists (template positions use HL7 1-based numbering)
            let Some(field) = segment.get_field(*pos) else {
                if field_template.required {
                    result.add_error(
                        ValidationError::new("Required field not found").with_location(&location),
                    );
                }
                continue;
            };

            // Check if field is empty
            let is_empty = field.is_empty();

            if is_empty && field_template.required {
                result.add_error(
                    ValidationError::new("Required field is empty").with_location(&location),
                );
            }

            // Check field length if specified (in characters, not
            // bytes: multibyte text must not warn under the limit). The
            // whole field value is measured: `value()` sees only the
            // first component and would miss long trailing components.
            if let Some(max_length) = field_template.length
                && !is_empty
            {
                let text = field
                    .repetitions
                    .iter()
                    .map(|r| r.full_value().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join("~");
                if text.chars().count() > max_length {
                    result.add_warning(
                        ValidationWarning::new(format!(
                            "Field length {} exceeds maximum {}",
                            text.chars().count(),
                            max_length
                        ))
                        .with_location(&location),
                    );
                }
            }

            // Check required components against component templates.
            // (The `datatype` tag is informational only and not enforced.)
            if let Some(components) = &field_template.components {
                Self::validate_field_components(&segment.id, *pos, field, components, result);
            }
        }
    }

    /// Validate a field's first repetition against component templates
    fn validate_field_components(
        segment_id: &str,
        field_pos: usize,
        field: &vyna_hl7_core::Field,
        components: &[crate::ComponentTemplate],
        result: &mut ValidationResult,
    ) {
        let repetition = field.get_repetition(0);
        for comp_template in components {
            let location = format!("{}-{}-{}", segment_id, field_pos, comp_template.position);
            if comp_template.position == 0 {
                result.add_error(
                    ValidationError::new("Component position must be 1-based (got 0)")
                        .with_location(&location),
                );
                continue;
            }
            if comp_template.required {
                let empty = repetition
                    .and_then(|rep| rep.get_component(comp_template.position - 1))
                    .map(|c| c.is_empty())
                    .unwrap_or(true);
                if empty {
                    result.add_error(
                        ValidationError::new("Required component is empty")
                            .with_location(&location),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FieldTemplate, SegmentTemplate};
    use vyna_hl7_core::Field;

    fn create_test_message() -> Message {
        // A real parsed message: MSH-9 is structural ("ADT" and "A01"
        // components), unlike Field::from_value("ADT^A01") which stores
        // one literal component and masked the trigger-event check.
        vyna_hl7_parser::parse_message(
            "MSH|^~\\&|APP|FACILITY|||20250115||ADT^A01|MSG001|P|2.5\rPID|1||12345\r",
        )
        .expect("test fixture parses")
    }

    #[test]
    fn test_validate_valid_message() {
        let message = create_test_message();
        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01")
            .with_segment(SegmentTemplate::new("MSH").required())
            .with_segment(SegmentTemplate::new("PID").required());

        let result = TemplateValidator::validate(&message, &template);
        assert!(result.valid);
        assert!(!result.has_errors());
    }

    #[test]
    fn test_validate_missing_required_segment() {
        let mut message = create_test_message();
        message.segments.retain(|s| s.id != "PID"); // Remove PID segment

        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01")
            .with_segment(SegmentTemplate::new("MSH").required())
            .with_segment(SegmentTemplate::new("PID").required());

        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("Required segment 'PID'"))
        );
    }

    #[test]
    fn test_validate_message_type_mismatch() {
        let mut message = create_test_message();
        // Change message type to ORU
        message.segments[0].fields[8] = Field::from_delimited("ORU^R01");

        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("Message type mismatch"))
        );
    }

    #[test]
    fn test_validate_trigger_event_mismatch() {
        let mut message = create_test_message();
        // Change trigger event to A02
        message.segments[0].fields[8] = Field::from_delimited("ADT^A02");

        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("Trigger event mismatch"))
        );
    }

    #[test]
    fn test_validate_required_field() {
        let message = create_test_message();

        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID").required();
        pid_segment.add_field(3, FieldTemplate::new().required()); // Patient ID is required
        template.add_segment(pid_segment);

        let result = TemplateValidator::validate(&message, &template);
        assert!(result.valid); // Patient ID exists at position 3
    }

    #[test]
    fn test_validate_missing_required_field() {
        let message = create_test_message();

        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID").required();
        pid_segment.add_field(5, FieldTemplate::new().required()); // Require field 5 which doesn't exist
        template.add_segment(pid_segment);

        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.location == Some("PID-5".to_string()))
        );
    }

    #[test]
    fn test_validate_field_length() {
        let mut message = create_test_message();
        // Add a long value to PID-3
        message.segments[1].fields[2] = Field::from_value("A".repeat(100));

        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(3, FieldTemplate::new().with_length(20)); // Max length 20
        template.add_segment(pid_segment);

        let result = TemplateValidator::validate(&message, &template);
        assert!(result.valid); // Still valid, but has warning
        assert!(result.has_warnings());
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.message.contains("exceeds maximum"))
        );
    }

    #[test]
    fn test_validate_required_components() {
        let mut message = create_test_message();
        message.segments[1].fields.push(Field::new()); // PID-4
        message.segments[1]
            .fields
            .push(Field::from_delimited("DOE^JOHN")); // PID-5

        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(
            5,
            FieldTemplate::new().with_components(vec![crate::ComponentTemplate::new(2).required()]),
        );
        template.add_segment(pid_segment);

        let result = TemplateValidator::validate(&message, &template);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_missing_required_component() {
        let mut message = create_test_message();
        message.segments[1].fields.push(Field::new()); // PID-4
        message.segments[1]
            .fields
            .push(Field::from_delimited("DOE^JOHN")); // PID-5

        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(
            5,
            FieldTemplate::new().with_components(vec![crate::ComponentTemplate::new(3).required()]),
        );
        template.add_segment(pid_segment);

        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.location == Some("PID-5-3".to_string()))
        );
    }

    #[test]
    fn test_validate_component_gap_positions() {
        let mut message = create_test_message();
        message.segments[1].fields.push(Field::new()); // PID-4
        message.segments[1]
            .fields
            .push(Field::from_delimited("DOE^^JR")); // PID-5

        // Required component 3 is filled despite the gap at 2: valid.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(
            5,
            FieldTemplate::new().with_components(vec![crate::ComponentTemplate::new(3).required()]),
        );
        template.add_segment(pid_segment);
        assert!(TemplateValidator::validate(&message, &template).valid);

        // Required component 2 is empty: invalid.
        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(
            5,
            FieldTemplate::new().with_components(vec![crate::ComponentTemplate::new(2).required()]),
        );
        template.add_segment(pid_segment);
        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.location == Some("PID-5-2".to_string()))
        );
    }

    #[test]
    fn test_validation_error_creation() {
        let error = ValidationError::new("Test error").with_location("PID-5");
        assert_eq!(error.message, "Test error");
        assert_eq!(error.location, Some("PID-5".to_string()));
    }

    #[test]
    fn test_validation_warning_creation() {
        let warning = ValidationWarning::new("Test warning").with_location("MSH-9");
        assert_eq!(warning.message, "Test warning");
        assert_eq!(warning.location, Some("MSH-9".to_string()));
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::valid();
        assert!(result.valid);
        assert!(!result.has_errors());
        assert!(!result.has_warnings());

        result.add_error(ValidationError::new("Error"));
        assert!(!result.valid);
        assert!(result.has_errors());

        result.add_warning(ValidationWarning::new("Warning"));
        assert!(result.has_warnings());
    }

    #[test]
    fn test_trigger_event_checked_on_parsed_message() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|APP|FACILITY|||20250115||ADT^A02|MSG001|P|2.5\rPID|1||12345\r",
        )
        .unwrap();
        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("Trigger event mismatch"))
        );
    }

    #[test]
    fn test_absent_msh9_fails_type_trigger_check() {
        // MSH present but MSH-9 absent: fails like an empty MSH-9,
        // instead of passing with valid=true.
        let mut msh = vyna_hl7_core::Segment::new("MSH");
        msh.set_field(3, Field::from_value("APP")).unwrap();
        let mut message = Message::new();
        message.add_segment(msh);
        let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");

        let result = TemplateValidator::validate(&message, &template);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("Message type is missing")),
            "unexpected errors: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("Trigger event is missing")),
            "unexpected errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_field_length_counts_characters_not_bytes() {
        // 100 chars (200 bytes) under a 150 limit: no warning.
        let mut message = create_test_message();
        message.segments[1].fields[2] = Field::from_value("ü".repeat(100));

        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(3, FieldTemplate::new().with_length(150));
        template.add_segment(pid_segment);

        let result = TemplateValidator::validate(&message, &template);
        assert!(result.valid);
        assert!(!result.has_warnings());

        // 200 chars over the limit still warns, reporting the count.
        message.segments[1].fields[2] = Field::from_value("ü".repeat(200));
        let result = TemplateValidator::validate(&message, &template);
        assert!(result.has_warnings());
        assert!(
            result.warnings.iter().any(|w| w.message.contains("200")),
            "unexpected warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_field_length_measures_whole_composite_value() {
        // PID-5 "DOE^JOHNATHONEXCEEDINGLYLONG" is 27 chars: the limit
        // applies to the whole composite value, not just "DOE".
        let mut message = create_test_message();
        message.segments[1].fields.push(Field::new()); // PID-4
        message.segments[1]
            .fields
            .push(Field::from_delimited("DOE^JOHNATHONEXCEEDINGLYLONG")); // PID-5

        let mut template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
        let mut pid_segment = SegmentTemplate::new("PID");
        pid_segment.add_field(5, FieldTemplate::new().with_length(10));
        template.add_segment(pid_segment);

        let result = TemplateValidator::validate(&message, &template);
        assert!(result.valid);
        assert!(result.has_warnings());
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.location == Some("PID-5".to_string())),
            "unexpected warnings: {:?}",
            result.warnings
        );
    }
}
