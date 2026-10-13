//! Conformance profile validator

pub mod result;

pub use result::{
    ConformanceErrorType, ConformanceValidationError, ConformanceValidationInfo,
    ConformanceValidationResult, ConformanceValidationWarning, Severity, ValidationLocation,
};

use crate::profile::{ConditionalUsage, ConformanceProfile, FieldProfile, SegmentProfile, Usage};
use vyna_hl7_core::charset::CharacterSet;
use vyna_hl7_core::{Message, Segment};

/// Conformance profile validator
pub struct ConformanceValidator {
    profile: ConformanceProfile,
}

/// Context for field-level validation helpers.
struct FieldValidationContext<'a> {
    segment_id: &'a str,
    segment_index: usize,
    field_position: usize,
    occurrence_count: usize,
    field_profile: &'a FieldProfile,
    message: &'a Message,
    result: &'a mut ConformanceValidationResult,
}

impl ConformanceValidator {
    /// Create a new conformance validator
    pub fn new(profile: ConformanceProfile) -> Self {
        Self { profile }
    }

    /// Get the profile
    pub fn profile(&self) -> &ConformanceProfile {
        &self.profile
    }

    /// Validate a message against the conformance profile
    pub fn validate(&self, message: &Message) -> ConformanceValidationResult {
        let mut result = ConformanceValidationResult::new();

        // Validate message structure
        self.validate_message_structure(message, &mut result);

        // Validate content against the MSH-18 character set declaration
        Self::validate_charset_content(message, &mut result);

        // Validate each segment in the profile
        for segment_profile in &self.profile.message.segments {
            self.validate_segment(message, segment_profile, &mut result);
        }

        result
    }

    /// Validate overall message structure
    fn validate_message_structure(
        &self,
        _message: &Message,
        result: &mut ConformanceValidationResult,
    ) {
        // Check message type matches profile
        let _profile = &self.profile.message;

        result.add_info(ConformanceValidationInfo {
            location: None,
            message: format!(
                "Validating against profile: {} {}",
                self.profile.metadata.name, self.profile.metadata.version
            ),
        });

        // Note: In MVP, we're not strictly enforcing message type match
        // This would be added in a future enhancement
    }

    /// Validate message content against the MSH-18 declaration.
    ///
    /// A message declaring `8859/1` while carrying characters outside that
    /// set is corrupt on the wire for any strict peer, so each offending
    /// component is an error. UTF-8 (explicit or via an empty MSH-18, which
    /// defaults to UTF-8) represents every character and is skipped. An
    /// unparseable MSH-18 value is an error. Reports are capped so one bad
    /// message cannot flood the result.
    fn validate_charset_content(message: &Message, result: &mut ConformanceValidationResult) {
        const MAX_CHARSET_ERRORS: usize = 10;

        let Some(msh) = message.segments.iter().find(|s| s.id == "MSH") else {
            return;
        };
        let declared = msh.get_field_value(18).unwrap_or("").trim();
        if declared.is_empty() {
            return;
        }
        let charset = match CharacterSet::from_msh18(declared) {
            Ok(charset) => charset,
            Err(error) => {
                result.add_error(
                    ConformanceValidationError::new(
                        ValidationLocation::field("MSH".to_string(), 18),
                        ConformanceErrorType::ValueNotInValueSet,
                        format!("MSH-18 declares an unsupported character set: {error}"),
                    )
                    .with_rule("MSH-18 table 0211".to_string()),
                );
                return;
            }
        };
        if charset == CharacterSet::Utf8 {
            return;
        }

        let mut reported = 0usize;
        'segments: for segment in &message.segments {
            for (field_index, field) in segment.fields.iter().enumerate() {
                for repetition in &field.repetitions {
                    for (component_index, component) in repetition.components.iter().enumerate() {
                        for subcomponent in &component.subcomponents {
                            if let Some(offender) = charset.find_unmappable(subcomponent.as_str()) {
                                result.add_error(
                                    ConformanceValidationError::new(
                                        ValidationLocation::component(
                                            segment.id.clone(),
                                            field_index + 1,
                                            component_index + 1,
                                        ),
                                        ConformanceErrorType::UnmappableCharacter,
                                        format!(
                                            "character '{offender}' (U+{:04X}) cannot be encoded in the MSH-18 character set '{declared}'",
                                            offender as u32
                                        ),
                                    )
                                    .with_rule("MSH-18 content match".to_string()),
                                );
                                reported += 1;
                                if reported >= MAX_CHARSET_ERRORS {
                                    break 'segments;
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validate a segment according to its profile
    fn validate_segment(
        &self,
        message: &Message,
        segment_profile: &SegmentProfile,
        result: &mut ConformanceValidationResult,
    ) {
        // Count occurrences of this segment
        let segments: Vec<&Segment> = message
            .segments
            .iter()
            .filter(|s| s.id == segment_profile.name)
            .collect();

        let occurrence_count = segments.len();

        // Validate segment usage
        self.validate_segment_usage(
            &segment_profile.name,
            occurrence_count,
            segment_profile,
            result,
        );

        // Validate segment cardinality
        self.validate_segment_cardinality(
            &segment_profile.name,
            occurrence_count,
            segment_profile,
            result,
        );

        // Validate each occurrence of the segment
        for (index, segment) in segments.iter().enumerate() {
            self.validate_segment_fields(segment, index, segment_profile, message, result);
        }
    }

    /// Validate segment usage (R, RE, O, X)
    fn validate_segment_usage(
        &self,
        segment_name: &str,
        occurrence_count: usize,
        segment_profile: &SegmentProfile,
        result: &mut ConformanceValidationResult,
    ) {
        match segment_profile.usage {
            Usage::Required => {
                if occurrence_count == 0 {
                    result.add_error(
                        ConformanceValidationError::new(
                            ValidationLocation::segment(segment_name.to_string()),
                            ConformanceErrorType::RequiredElementMissing,
                            format!("Required segment {} is missing", segment_name),
                        )
                        .with_rule(format!("{} usage=R", segment_name)),
                    );
                }
            }
            Usage::RequiredIfKnown => {
                if occurrence_count == 0 {
                    result.add_warning(ConformanceValidationWarning {
                        location: ValidationLocation::segment(segment_name.to_string()),
                        message: format!("Required if known segment {} is missing", segment_name),
                        rule: Some(format!("{} usage=RE", segment_name)),
                    });
                }
            }
            Usage::NotUsed => {
                if occurrence_count > 0 {
                    result.add_error(
                        ConformanceValidationError::new(
                            ValidationLocation::segment(segment_name.to_string()),
                            ConformanceErrorType::NotUsedElementPresent,
                            format!(
                                "Segment {} is marked as not used but is present ({} occurrence(s))",
                                segment_name, occurrence_count
                            ),
                        )
                        .with_rule(format!("{} usage=X", segment_name)),
                    );
                }
            }
            Usage::Optional => {
                // No validation needed for optional segments
            }
        }
    }

    /// Validate segment cardinality
    fn validate_segment_cardinality(
        &self,
        segment_name: &str,
        occurrence_count: usize,
        segment_profile: &SegmentProfile,
        result: &mut ConformanceValidationResult,
    ) {
        let cardinality = &segment_profile.cardinality;

        if !cardinality.satisfies(occurrence_count) {
            if occurrence_count < cardinality.min {
                result.add_error(
                    ConformanceValidationError::new(
                        ValidationLocation::segment(segment_name.to_string()),
                        ConformanceErrorType::BelowMinimumOccurrences,
                        format!(
                            "Segment {} has {} occurrence(s), but minimum is {}",
                            segment_name, occurrence_count, cardinality.min
                        ),
                    )
                    .with_rule(format!("{} cardinality={}", segment_name, cardinality)),
                );
            } else if let Some(max) = cardinality.max {
                result.add_error(
                    ConformanceValidationError::new(
                        ValidationLocation::segment(segment_name.to_string()),
                        ConformanceErrorType::ExceedsMaximumOccurrences,
                        format!(
                            "Segment {} has {} occurrence(s), but maximum is {}",
                            segment_name, occurrence_count, max
                        ),
                    )
                    .with_rule(format!("{} cardinality={}", segment_name, cardinality)),
                );
            }
        }
    }

    /// Validate fields within a segment
    fn validate_segment_fields(
        &self,
        segment: &Segment,
        segment_index: usize,
        segment_profile: &SegmentProfile,
        message: &Message,
        result: &mut ConformanceValidationResult,
    ) {
        for field_profile in &segment_profile.fields {
            self.validate_field(segment, segment_index, field_profile, message, result);
        }
    }

    /// Validate a specific field
    fn validate_field(
        &self,
        segment: &Segment,
        segment_index: usize,
        field_profile: &FieldProfile,
        message: &Message,
        result: &mut ConformanceValidationResult,
    ) {
        let field_position = field_profile.position;
        let field = segment.get_field(field_position);

        // Count field occurrences (for repeating fields)
        let occurrence_count = field.map(|f| f.repetitions.len()).unwrap_or(0);

        // Validate field usage
        self.validate_field_usage(FieldValidationContext {
            segment_id: &segment.id,
            segment_index,
            field_position,
            occurrence_count,
            field_profile,
            message,
            result: &mut *result,
        });

        // Validate field cardinality
        self.validate_field_cardinality(FieldValidationContext {
            segment_id: &segment.id,
            segment_index,
            field_position,
            occurrence_count,
            field_profile,
            message,
            result: &mut *result,
        });

        // Validate field length if present
        if let Some(field) = field {
            self.validate_field_length(
                FieldValidationContext {
                    segment_id: &segment.id,
                    segment_index,
                    field_position,
                    occurrence_count,
                    field_profile,
                    message,
                    result: &mut *result,
                },
                field,
            );
        }
    }

    /// Validate field usage
    fn validate_field_usage(&self, ctx: FieldValidationContext<'_>) {
        let FieldValidationContext {
            segment_id,
            segment_index,
            field_position,
            occurrence_count,
            field_profile,
            message,
            result,
        } = ctx;
        let location = if segment_index > 0 {
            ValidationLocation {
                segment: segment_id.to_string(),
                segment_index: Some(segment_index),
                field: Some(field_position),
                component: None,
            }
        } else {
            ValidationLocation::field(segment_id.to_string(), field_position)
        };

        match &field_profile.usage {
            ConditionalUsage::Required => {
                if occurrence_count == 0 {
                    let field_name = field_profile
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Field {}", field_position));
                    result.add_error(
                        ConformanceValidationError::new(
                            location,
                            ConformanceErrorType::RequiredElementMissing,
                            format!("Required field {} is missing", field_name),
                        )
                        .with_rule(format!("{}-{} usage=R", segment_id, field_position)),
                    );
                }
            }
            ConditionalUsage::RequiredIfKnown => {
                if occurrence_count == 0 {
                    let field_name = field_profile
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Field {}", field_position));
                    result.add_warning(ConformanceValidationWarning {
                        location,
                        message: format!("Required if known field {} is missing", field_name),
                        rule: Some(format!("{}-{} usage=RE", segment_id, field_position)),
                    });
                }
            }
            ConditionalUsage::NotUsed => {
                if occurrence_count > 0 {
                    let field_name = field_profile
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Field {}", field_position));
                    result.add_error(
                        ConformanceValidationError::new(
                            location,
                            ConformanceErrorType::NotUsedElementPresent,
                            format!("Field {} is marked as not used but is present", field_name),
                        )
                        .with_rule(format!("{}-{} usage=X", segment_id, field_position)),
                    );
                }
            }
            ConditionalUsage::Optional => {
                // No validation needed
            }
            ConditionalUsage::Conditional(predicate) => {
                // Evaluate predicate to determine actual usage. A
                // predicate that fails to parse is a profile bug, not a
                // message fault: warn so it is visible without failing
                // an otherwise conformant message.
                match crate::predicate::PredicateEvaluator::evaluate(predicate, message) {
                    Ok(actual_usage) => {
                        // Recursively validate with the evaluated usage
                        let temp_profile = FieldProfile {
                            position: field_profile.position,
                            name: field_profile.name.clone(),
                            usage: ConditionalUsage::from_usage(actual_usage),
                            cardinality: field_profile.cardinality,
                            datatype: field_profile.datatype.clone(),
                            length: field_profile.length,
                            table_id: field_profile.table_id.clone(),
                            components: field_profile.components.clone(),
                            value_set: field_profile.value_set.clone(),
                            item_no: field_profile.item_no.clone(),
                        };
                        self.validate_field_usage(FieldValidationContext {
                            segment_id,
                            segment_index,
                            field_position,
                            occurrence_count,
                            field_profile: &temp_profile,
                            message,
                            result,
                        });
                    }
                    Err(error) => {
                        result.add_warning(ConformanceValidationWarning {
                            location,
                            message: format!(
                                "Conditional predicate for {}-{} could not be evaluated: {}",
                                segment_id, field_position, error
                            ),
                            rule: Some(format!("{}-{} usage=C", segment_id, field_position)),
                        });
                    }
                }
            }
        }
    }

    /// Validate field cardinality
    fn validate_field_cardinality(&self, ctx: FieldValidationContext<'_>) {
        let FieldValidationContext {
            segment_id,
            segment_index,
            field_position,
            occurrence_count,
            field_profile,
            result,
            ..
        } = ctx;
        let location = if segment_index > 0 {
            ValidationLocation {
                segment: segment_id.to_string(),
                segment_index: Some(segment_index),
                field: Some(field_position),
                component: None,
            }
        } else {
            ValidationLocation::field(segment_id.to_string(), field_position)
        };

        let cardinality = &field_profile.cardinality;

        if !cardinality.satisfies(occurrence_count) {
            if occurrence_count < cardinality.min {
                result.add_error(
                    ConformanceValidationError::new(
                        location,
                        ConformanceErrorType::BelowMinimumOccurrences,
                        format!(
                            "Field has {} occurrence(s), but minimum is {}",
                            occurrence_count, cardinality.min
                        ),
                    )
                    .with_rule(format!(
                        "{}-{} cardinality={}",
                        segment_id, field_position, cardinality
                    )),
                );
            } else if let Some(max) = cardinality.max {
                result.add_error(
                    ConformanceValidationError::new(
                        location,
                        ConformanceErrorType::ExceedsMaximumOccurrences,
                        format!(
                            "Field has {} occurrence(s), but maximum is {}",
                            occurrence_count, max
                        ),
                    )
                    .with_rule(format!(
                        "{}-{} cardinality={}",
                        segment_id, field_position, cardinality
                    )),
                );
            }
        }
    }

    /// Validate field length
    fn validate_field_length(&self, ctx: FieldValidationContext<'_>, field: &vyna_hl7_core::Field) {
        let FieldValidationContext {
            segment_id,
            segment_index,
            field_position,
            field_profile,
            result,
            ..
        } = ctx;
        if let Some(max_length) = field_profile.length {
            // Check each repetition
            for (rep_index, repetition) in field.repetitions.iter().enumerate() {
                // Get the encoded value of this repetition
                // Use default delimiters for length calculation
                let delimiters = vyna_hl7_core::Delimiters::default();
                let value = repetition.encode(&delimiters);
                let actual_length = value.len();

                if actual_length > max_length {
                    let location = if segment_index > 0 {
                        ValidationLocation {
                            segment: segment_id.to_string(),
                            segment_index: Some(segment_index),
                            field: Some(field_position),
                            component: None,
                        }
                    } else {
                        ValidationLocation::field(segment_id.to_string(), field_position)
                    };

                    let field_name = field_profile
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("Field {}", field_position));

                    result.add_error(
                        ConformanceValidationError::new(
                            location,
                            ConformanceErrorType::ExceedsMaxLength,
                            format!(
                                "Field {} repetition {} has length {}, but maximum is {}",
                                field_name,
                                rep_index + 1,
                                actual_length,
                                max_length
                            ),
                        )
                        .with_rule(format!(
                            "{}-{} length<={}",
                            segment_id, field_position, max_length
                        )),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{
        Cardinality, ConditionalUsage, FieldProfile, MessageProfile, Predicate, ProfileMetadata,
        SegmentProfile, Usage,
    };
    use vyna_hl7_core::Version;
    use vyna_hl7_parser::parse_message;

    fn create_test_profile() -> ConformanceProfile {
        let metadata =
            ProfileMetadata::new("Test Profile".to_string(), "1.0".to_string(), Version::V2_5);

        let mut message = MessageProfile::new("ADT".to_string(), "A01".to_string());

        // MSH segment - required, exactly one
        let mut msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());
        msh.add_field(FieldProfile::new(9, Usage::Required, Cardinality::one()));
        message.add_segment(msh);

        // PID segment - required, exactly one
        let mut pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());
        pid.add_field(FieldProfile::new(3, Usage::Required, Cardinality::one()));
        message.add_segment(pid);

        ConformanceProfile::new(metadata, message)
    }

    #[test]
    fn test_validator_creation() {
        let profile = create_test_profile();
        let validator = ConformanceValidator::new(profile);
        assert_eq!(validator.profile().metadata.name, "Test Profile");
    }

    fn charset_message(msh18: &str, patient_name: &str) -> Message {
        use vyna_hl7_core::Field;

        let mut message = Message::new();
        let mut msh = Segment::new("MSH");
        for index in 1..=18 {
            let value = match index {
                1 => "|",
                2 => "^~\\&",
                9 => "ADT^A01",
                18 => msh18,
                _ => "",
            };
            msh.set_field(index, Field::from_value(value)).unwrap();
        }
        message.add_segment(msh);

        let mut pid = Segment::new("PID");
        pid.set_field(3, Field::from_value("PATID1234")).unwrap();
        pid.set_field(5, Field::from_value(patient_name)).unwrap();
        message.add_segment(pid);
        message
    }

    fn charset_errors(message: &Message) -> Vec<ConformanceValidationError> {
        let validator = ConformanceValidator::new(create_test_profile());
        let result = validator.validate(message);
        result
            .errors
            .iter()
            .filter(|e| {
                matches!(
                    e.error_type,
                    ConformanceErrorType::UnmappableCharacter
                        | ConformanceErrorType::ValueNotInValueSet
                )
            })
            .cloned()
            .collect()
    }

    #[test]
    fn test_charset_content_latin1_mismatch_is_error() {
        let message = charset_message("8859/1", "山田^太郎");
        let errors = charset_errors(&message);
        assert!(
            errors
                .iter()
                .any(|e| e.error_type == ConformanceErrorType::UnmappableCharacter),
            "expected an unmappable-character error, got: {errors:?}"
        );
        assert!(errors[0].message.contains("8859/1"));
    }

    #[test]
    fn test_charset_content_latin1_clean_passes() {
        let message = charset_message("8859/1", "Müller^Jörg");
        assert!(charset_errors(&message).is_empty());
    }

    #[test]
    fn test_charset_content_utf8_accepts_cjk() {
        let message = charset_message("UNICODE UTF-8", "王^小明");
        assert!(charset_errors(&message).is_empty());
    }

    #[test]
    fn test_charset_content_empty_msh18_skipped() {
        let message = charset_message("", "山田^太郎");
        assert!(charset_errors(&message).is_empty());
    }

    #[test]
    fn test_charset_content_unknown_charset_is_error() {
        let message = charset_message("EBCDIC", "Smith^John");
        let errors = charset_errors(&message);
        assert!(
            errors
                .iter()
                .any(|e| e.error_type == ConformanceErrorType::ValueNotInValueSet),
            "expected an unsupported-charset error, got: {errors:?}"
        );
    }

    /// Profile with one conditional field: PID-5 is required when
    /// `condition` holds, optional otherwise.
    fn conditional_profile(condition: &str) -> ConformanceProfile {
        let metadata =
            ProfileMetadata::new("Conditional".to_string(), "1.0".to_string(), Version::V2_5);
        let mut message = MessageProfile::new("ADT".to_string(), "A01".to_string());
        let mut pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());
        pid.add_field(FieldProfile::with_conditional_usage(
            5,
            ConditionalUsage::Conditional(Predicate::new(
                condition.to_string(),
                Usage::Required,
                Usage::Optional,
            )),
            Cardinality::new(0, Some(1)).unwrap(),
        ));
        message.add_segment(pid);
        ConformanceProfile::new(metadata, message)
    }

    fn conditional_message(pid: &str) -> Message {
        parse_message(&format!(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r{pid}"
        ))
        .unwrap()
    }

    #[test]
    fn test_conditional_true_enforces_required() {
        // PID-3 valued, PID-5 absent: condition holds, PID-5 required.
        let validator = ConformanceValidator::new(conditional_profile("PID-3 IS VALUED"));
        let result = validator.validate(&conditional_message("PID|1||PATID1234"));
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == ConformanceErrorType::RequiredElementMissing),
            "expected a required-element error, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_conditional_false_skips_required() {
        // PID-3 absent, PID-5 absent: condition fails, PID-5 optional.
        let validator = ConformanceValidator::new(conditional_profile("PID-3 IS VALUED"));
        let result = validator.validate(&conditional_message("PID|1"));
        assert!(result.is_valid());
    }

    #[test]
    fn test_conditional_unparseable_predicate_warns() {
        // A broken predicate is a profile bug: warn, but do not fail an
        // otherwise conformant message.
        let validator = ConformanceValidator::new(conditional_profile("PID-3 ~~ 'M'"));
        let result = validator.validate(&conditional_message("PID|1"));
        assert!(result.is_valid());
        assert_eq!(result.warnings.len(), 1);
        assert!(
            result.warnings[0]
                .message
                .contains("could not be evaluated"),
            "unexpected warning: {}",
            result.warnings[0].message
        );
        assert_eq!(result.warnings[0].rule.as_deref(), Some("PID-5 usage=C"));
    }
}
