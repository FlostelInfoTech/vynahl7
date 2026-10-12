//! Basic conformance profile validation example
//!
//! This example demonstrates how to:
//! 1. Create a conformance profile programmatically
//! 2. Build an HL7 message manually
//! 3. Validate the message against the profile
//! 4. Handle validation results

use vyna_hl7_conformance::{
    Cardinality, ConformanceProfile, ConformanceValidator, FieldProfile, MessageProfile,
    ProfileMetadata, SegmentProfile, Usage,
};
use vyna_hl7_core::{Field, Message, Segment, Version};

fn main() {
    println!("=== VynaHL7 Conformance Profile Validation Example ===\n");

    // Create a conformance profile for ADT^A01
    let profile = create_adt_a01_profile();
    println!("Created conformance profile:");
    println!("  Name: {}", profile.metadata.name);
    println!("  Version: {}", profile.metadata.version);
    println!("  HL7 Version: {:?}", profile.metadata.hl7_version);
    println!(
        "  Segments: {}",
        profile.message.segments.len()
    );
    println!();

    // Create validator
    let validator = ConformanceValidator::new(profile);

    // Test Case 1: Valid message
    println!("Test Case 1: Valid Message");
    println!("---------------------------");
    let valid_message = create_valid_message();
    let result = validator.validate(&valid_message);
    print_validation_result(&result);

    // Test Case 2: Missing required segment
    println!("\nTest Case 2: Missing Required Segment (PID)");
    println!("--------------------------------------------");
    let invalid_message_1 = create_message_missing_pid();
    let result = validator.validate(&invalid_message_1);
    print_validation_result(&result);

    // Test Case 3: Missing required field
    println!("\nTest Case 3: Missing Required Field (PID-3)");
    println!("--------------------------------------------");
    let invalid_message_2 = create_message_missing_field();
    let result = validator.validate(&invalid_message_2);
    print_validation_result(&result);

    // Test Case 4: Extra segments (optional)
    println!("\nTest Case 4: Message with Optional Segments");
    println!("--------------------------------------------");
    let message_with_optional = create_message_with_optional();
    let result = validator.validate(&message_with_optional);
    print_validation_result(&result);
}

/// Create a simple ADT^A01 conformance profile
fn create_adt_a01_profile() -> ConformanceProfile {
    let metadata = ProfileMetadata::new(
        "ADT_A01_Example".to_string(),
        "1.0.0".to_string(),
        Version::V2_5,
    );

    let mut message = MessageProfile::new("ADT".to_string(), "A01".to_string());

    // MSH segment - Required [1..1]
    let mut msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());
    msh.add_field(FieldProfile::new(
        9,
        Usage::Required,
        Cardinality::one(),
    ));
    message.add_segment(msh);

    // EVN segment - Required [1..1]
    let evn = SegmentProfile::new("EVN".to_string(), Usage::Required, Cardinality::one());
    message.add_segment(evn);

    // PID segment - Required [1..1]
    let mut pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());
    // Patient ID is required
    pid.add_field(FieldProfile::new(
        3,
        Usage::Required,
        Cardinality::one(),
    ));
    // Patient Name is required
    pid.add_field(FieldProfile::new(
        5,
        Usage::Required,
        Cardinality::one(),
    ));
    message.add_segment(pid);

    // NK1 segment - Optional [0..*] (Next of Kin)
    let nk1 = SegmentProfile::new(
        "NK1".to_string(),
        Usage::Optional,
        Cardinality::zero_or_more(),
    );
    message.add_segment(nk1);

    // PV1 segment - Required [1..1] (Patient Visit)
    let pv1 = SegmentProfile::new("PV1".to_string(), Usage::Required, Cardinality::one());
    message.add_segment(pv1);

    ConformanceProfile::new(metadata, message)
}

/// Create a valid ADT^A01 message
fn create_valid_message() -> Message {
    let mut message = Message::new();

    // MSH segment
    let mut msh = Segment::new("MSH".to_string());
    msh.add_field(Field::from_value("^~\\&")); // Field 1
    msh.add_field(Field::from_value("SendingApp")); // Field 2
    msh.add_field(Field::from_value("SendingFacility")); // Field 3
    msh.add_field(Field::from_value("ReceivingApp")); // Field 4
    msh.add_field(Field::from_value("ReceivingFacility")); // Field 5
    msh.add_field(Field::from_value("20250119120000")); // Field 6
    msh.add_field(Field::new()); // Field 7
    msh.add_field(Field::from_value("ADT^A01^ADT_A01")); // Field 8
    msh.add_field(Field::from_value("MSG00001")); // Field 9
    message.add_segment(msh);

    // EVN segment
    let mut evn = Segment::new("EVN".to_string());
    evn.add_field(Field::from_value("A01")); // Field 1
    evn.add_field(Field::from_value("20250119120000")); // Field 2
    message.add_segment(evn);

    // PID segment
    let mut pid = Segment::new("PID".to_string());
    pid.add_field(Field::from_value("1")); // Field 1
    pid.add_field(Field::new()); // Field 2
    pid.add_field(Field::from_value("12345^^^MRN")); // Field 3 - Patient ID
    pid.add_field(Field::new()); // Field 4
    pid.add_field(Field::from_value("Doe^John^A")); // Field 5 - Patient Name
    message.add_segment(pid);

    // PV1 segment
    let mut pv1 = Segment::new("PV1".to_string());
    pv1.add_field(Field::from_value("1")); // Field 1
    pv1.add_field(Field::from_value("I")); // Field 2 - Inpatient
    message.add_segment(pv1);

    message
}

/// Create message missing PID segment
fn create_message_missing_pid() -> Message {
    let mut message = Message::new();

    // MSH segment
    let mut msh = Segment::new("MSH".to_string());
    for _ in 0..8 {
        msh.add_field(Field::new());
    }
    msh.add_field(Field::from_value("ADT^A01^ADT_A01"));
    message.add_segment(msh);

    // EVN segment
    let evn = Segment::new("EVN".to_string());
    message.add_segment(evn);

    // PV1 segment (but missing PID!)
    let pv1 = Segment::new("PV1".to_string());
    message.add_segment(pv1);

    message
}

/// Create message with PID but missing required field
fn create_message_missing_field() -> Message {
    let mut message = Message::new();

    // MSH segment
    let mut msh = Segment::new("MSH".to_string());
    for _ in 0..8 {
        msh.add_field(Field::new());
    }
    msh.add_field(Field::from_value("ADT^A01^ADT_A01"));
    message.add_segment(msh);

    // EVN segment
    let evn = Segment::new("EVN".to_string());
    message.add_segment(evn);

    // PID segment without field 3 (Patient ID)
    let mut pid = Segment::new("PID".to_string());
    pid.add_field(Field::new()); // Field 1
    pid.add_field(Field::new()); // Field 2
    // Field 3 missing!
    pid.add_field(Field::new()); // Field 4
    pid.add_field(Field::from_value("Doe^John^A")); // Field 5
    message.add_segment(pid);

    // PV1 segment
    let pv1 = Segment::new("PV1".to_string());
    message.add_segment(pv1);

    message
}

/// Create message with optional segments
fn create_message_with_optional() -> Message {
    let mut message = create_valid_message();

    // Add NK1 segments (optional)
    let mut nk1 = Segment::new("NK1".to_string());
    nk1.add_field(Field::from_value("1")); // Field 1 - Set ID
    nk1.add_field(Field::from_value("Doe^Jane")); // Field 2 - Name
    nk1.add_field(Field::from_value("SPO")); // Field 3 - Relationship (Spouse)
    message.add_segment(nk1);

    message
}

/// Print validation results
fn print_validation_result(result: &vyna_hl7_conformance::ConformanceValidationResult) {
    if result.is_valid() {
        println!("✓ Message is VALID");
    } else {
        println!("✗ Message is INVALID");
    }

    println!("  Total issues: {}", result.total_issues());

    if !result.errors.is_empty() {
        println!("\n  Errors:");
        for error in &result.errors {
            println!("    - [{}] {}: {}",
                error.severity,
                error.location,
                error.message
            );
            if let Some(rule) = &error.rule {
                println!("      Rule: {}", rule);
            }
        }
    }

    if !result.warnings.is_empty() {
        println!("\n  Warnings:");
        for warning in &result.warnings {
            println!("    - {}: {}", warning.location, warning.message);
            if let Some(rule) = &warning.rule {
                println!("      Rule: {}", rule);
            }
        }
    }

    if !result.info.is_empty() {
        println!("\n  Info:");
        for info in &result.info {
            if let Some(location) = &info.location {
                println!("    - {}: {}", location, info.message);
            } else {
                println!("    - {}", info.message);
            }
        }
    }
}
