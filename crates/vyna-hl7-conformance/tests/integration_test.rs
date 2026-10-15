//! Integration tests for vyna-hl7-conformance

use vyna_hl7_conformance::{
    Cardinality, ConformanceProfile, ConformanceValidator, FieldProfile, MessageProfile,
    ProfileMetadata, SegmentProfile, Usage,
};
use vyna_hl7_core::{Field, Message, Segment, Version};

/// Helper to create a simple test message
fn create_test_message() -> Message {
    let mut message = Message::new();

    // MSH segment
    let mut msh = Segment::new("MSH".to_string());
    msh.add_field(Field::from_value("^~\\&")); // Field 1
    msh.add_field(Field::from_value("SendingApp")); // Field 2
    msh.add_field(Field::from_value("SendingFacility")); // Field 3
    msh.add_field(Field::new()); // Field 4
    msh.add_field(Field::new()); // Field 5
    msh.add_field(Field::new()); // Field 6
    msh.add_field(Field::new()); // Field 7
    msh.add_field(Field::new()); // Field 8
    msh.add_field(Field::from_value("ADT^A01^ADT_A01")); // Field 9 - Message Type
    message.add_segment(msh);

    // PID segment
    let mut pid = Segment::new("PID".to_string());
    pid.add_field(Field::new()); // Field 1
    pid.add_field(Field::new()); // Field 2
    pid.add_field(Field::from_value("123456^^^MRN")); // Field 3 - Patient ID
    message.add_segment(pid);

    message
}

/// Helper to create a simple conformance profile for ADT^A01
fn create_adt_a01_profile() -> ConformanceProfile {
    let metadata =
        ProfileMetadata::new("ADT_A01_Test".to_string(), "1.0".to_string(), Version::V2_5);

    let mut message = MessageProfile::new("ADT".to_string(), "A01".to_string());

    // MSH segment - Required [1..1]
    let mut msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());
    msh.add_field(FieldProfile::new(9, Usage::Required, Cardinality::one()));
    message.add_segment(msh);

    // PID segment - Required [1..1]
    let mut pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());
    pid.add_field(FieldProfile::new(3, Usage::Required, Cardinality::one()));
    message.add_segment(pid);

    // NK1 segment - Optional [0..*]
    let nk1 = SegmentProfile::new(
        "NK1".to_string(),
        Usage::Optional,
        Cardinality::zero_or_more(),
    );
    message.add_segment(nk1);

    ConformanceProfile::new(metadata, message)
}

#[test]
fn test_valid_message() {
    let profile = create_adt_a01_profile();
    let validator = ConformanceValidator::new(profile);
    let message = create_test_message();

    let result = validator.validate(&message);

    assert!(result.is_valid(), "Message should be valid");
    assert_eq!(result.errors.len(), 0, "Should have no errors");
}

#[test]
fn test_missing_required_segment() {
    let profile = create_adt_a01_profile();
    let validator = ConformanceValidator::new(profile);

    // Create message without PID segment
    let mut message = Message::new();

    let mut msh = Segment::new("MSH".to_string());
    msh.add_field(Field::from_value("^~\\&"));
    // Add fields 2-8
    for _ in 0..7 {
        msh.add_field(Field::new());
    }
    msh.add_field(Field::from_value("ADT^A01^ADT_A01")); // Field 9
    message.add_segment(msh);

    let result = validator.validate(&message);

    assert!(!result.is_valid(), "Message should be invalid");
    assert!(!result.errors.is_empty(), "Should have errors");

    // Check that error is about missing PID segment
    let has_pid_error = result
        .errors
        .iter()
        .any(|e| e.location.segment == "PID" && e.message.contains("missing"));
    assert!(has_pid_error, "Should have error about missing PID segment");
}

#[test]
fn test_missing_required_field() {
    let profile = create_adt_a01_profile();
    let validator = ConformanceValidator::new(profile);

    let mut message = Message::new();

    // MSH with field 9
    let mut msh = Segment::new("MSH".to_string());
    for _ in 0..8 {
        msh.add_field(Field::new());
    }
    msh.add_field(Field::from_value("ADT^A01^ADT_A01")); // Field 9
    message.add_segment(msh);

    // PID without field 3 (Patient ID)
    let mut pid = Segment::new("PID".to_string());
    pid.add_field(Field::new()); // Field 1
    pid.add_field(Field::new()); // Field 2
    // Field 3 is missing (empty)
    message.add_segment(pid);

    let result = validator.validate(&message);

    assert!(!result.is_valid(), "Message should be invalid");

    // Check that error is about missing PID-3
    let has_field_error = result.errors.iter().any(|e| {
        e.location.segment == "PID" && e.location.field == Some(3) && e.message.contains("missing")
    });
    assert!(has_field_error, "Should have error about missing PID-3");
}

#[test]
fn test_not_used_segment_present() {
    let metadata = ProfileMetadata::new(
        "ADT_A01_Strict".to_string(),
        "1.0".to_string(),
        Version::V2_5,
    );

    let mut message_profile = MessageProfile::new("ADT".to_string(), "A01".to_string());

    // MSH - Required
    let msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());
    message_profile.add_segment(msh);

    // PID - Required
    let pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());
    message_profile.add_segment(pid);

    // NK1 - Not Used (X)
    let nk1 = SegmentProfile::new(
        "NK1".to_string(),
        Usage::NotUsed,
        Cardinality::zero_or_one(),
    );
    message_profile.add_segment(nk1);

    let profile = ConformanceProfile::new(metadata, message_profile);
    let validator = ConformanceValidator::new(profile);

    // Create message with NK1 segment (which is marked as not used)
    let mut message = create_test_message();
    let nk1_segment = Segment::new("NK1".to_string());
    message.add_segment(nk1_segment);

    let result = validator.validate(&message);

    assert!(!result.is_valid(), "Message should be invalid");

    // Check for error about NK1 being present when marked as not used
    let has_not_used_error = result
        .errors
        .iter()
        .any(|e| e.location.segment == "NK1" && e.message.contains("not used"));
    assert!(
        has_not_used_error,
        "Should have error about NK1 being not used"
    );
}

#[test]
fn test_cardinality_validation() {
    let metadata =
        ProfileMetadata::new("ADT_A01_Card".to_string(), "1.0".to_string(), Version::V2_5);

    let mut message_profile = MessageProfile::new("ADT".to_string(), "A01".to_string());

    // MSH - Required
    let msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());
    message_profile.add_segment(msh);

    // PID - Required exactly one
    let pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());
    message_profile.add_segment(pid);

    let profile = ConformanceProfile::new(metadata, message_profile);
    let validator = ConformanceValidator::new(profile);

    // Create message with two PID segments
    let mut message = create_test_message();
    let pid2 = Segment::new("PID".to_string());
    message.add_segment(pid2);

    let result = validator.validate(&message);

    assert!(!result.is_valid(), "Message should be invalid");

    // Check for cardinality error
    let has_card_error = result
        .errors
        .iter()
        .any(|e| e.location.segment == "PID" && e.message.contains("maximum"));
    assert!(has_card_error, "Should have error about PID cardinality");
}

#[test]
fn test_field_length_validation() {
    let metadata = ProfileMetadata::new(
        "ADT_A01_Length".to_string(),
        "1.0".to_string(),
        Version::V2_5,
    );

    let mut message_profile = MessageProfile::new("ADT".to_string(), "A01".to_string());

    // MSH
    let msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());
    message_profile.add_segment(msh);

    // PID with field 3 having max length of 10
    let mut pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());
    let mut field_3 = FieldProfile::new(3, Usage::Required, Cardinality::one());
    field_3.length = Some(10); // Max length 10 characters
    pid.add_field(field_3);
    message_profile.add_segment(pid);

    let profile = ConformanceProfile::new(metadata, message_profile);
    let validator = ConformanceValidator::new(profile);

    // Create message with PID-3 exceeding max length
    let mut message = Message::new();

    let msh = Segment::new("MSH".to_string());
    message.add_segment(msh);

    let mut pid = Segment::new("PID".to_string());
    pid.add_field(Field::new()); // Field 1
    pid.add_field(Field::new()); // Field 2
    // Field 3 with value longer than 10 characters
    pid.add_field(Field::from_value("ThisIsAVeryLongPatientID123456"));
    message.add_segment(pid);

    let result = validator.validate(&message);

    assert!(!result.is_valid(), "Message should be invalid");

    // Check for length error
    let has_length_error = result.errors.iter().any(|e| {
        e.location.segment == "PID" && e.location.field == Some(3) && e.message.contains("length")
    });
    assert!(has_length_error, "Should have error about PID-3 length");
}
