//! XML Conformance Profile Parser Demonstration
//!
//! This example demonstrates how to:
//! 1. Load conformance profiles from XML files
//! 2. Inspect the parsed profile structure
//! 3. Use the profile to validate HL7 messages

use vyna_hl7_conformance::{ConformanceValidator, ProfileParser};
use vyna_hl7_core::{Field, Message, Segment};

fn main() {
    println!("=== VynaHL7 XML Conformance Profile Parser Demo ===\n");

    // Load conformance profile from XML file
    println!("Loading conformance profile from XML...");
    let profile = match ProfileParser::parse_file("profiles/sample_adt_a01.xml") {
        Ok(p) => {
            println!("✓ Successfully loaded profile\n");
            p
        }
        Err(e) => {
            eprintln!("✗ Failed to load profile: {}", e);
            return;
        }
    };

    // Display profile information
    println!("Profile Information:");
    println!("  Name: {}", profile.metadata.name);
    println!("  Version: {}", profile.metadata.version);
    println!(
        "  Organization: {}",
        profile.metadata.organization.as_deref().unwrap_or("N/A")
    );
    println!("  HL7 Version: {:?}", profile.metadata.hl7_version);
    println!(
        "  Message Type: {} ^{}",
        profile.message.message_type, profile.message.trigger_event
    );
    println!("  Number of Segments: {}", profile.message.segments.len());
    println!();

    // Display segment details
    println!("Segment Definitions:");
    for segment in &profile.message.segments {
        println!(
            "  {} - {} [{}, {}]",
            segment.name,
            segment.long_name.as_deref().unwrap_or(""),
            segment.usage.as_str(),
            segment.cardinality
        );
        if !segment.fields.is_empty() {
            println!("    Fields: {}", segment.fields.len());
            for field in segment.fields.iter().take(3) {
                println!(
                    "      {} - {} [{}, {}] {} (max len: {})",
                    field.position,
                    field.name.as_deref().unwrap_or("N/A"),
                    field.usage.as_str(),
                    field.cardinality,
                    field.datatype.as_deref().unwrap_or("N/A"),
                    field
                        .length
                        .map(|l| l.to_string())
                        .unwrap_or_else(|| "N/A".to_string())
                );
            }
            if segment.fields.len() > 3 {
                println!("      ... and {} more fields", segment.fields.len() - 3);
            }
        }
    }
    println!();

    // Create validator from loaded profile
    println!("Creating validator from profile...");
    let validator = ConformanceValidator::new(profile);
    println!("✓ Validator created\n");

    // Test Case 1: Valid message
    println!("Test Case 1: Validating a compliant message");
    println!("-------------------------------------------");
    let valid_msg = create_valid_adt_a01();
    let result = validator.validate(&valid_msg);
    print_validation_summary(&result);

    // Test Case 2: Invalid message (missing required segment)
    println!("\nTest Case 2: Validating message with missing PID segment");
    println!("----------------------------------------------------------");
    let invalid_msg = create_message_missing_pid();
    let result = validator.validate(&invalid_msg);
    print_validation_summary(&result);

    // Test Case 3: Optional segments
    println!("\nTest Case 3: Validating message with optional NK1 segments");
    println!("-----------------------------------------------------------");
    let msg_with_nk1 = create_message_with_nk1();
    let result = validator.validate(&msg_with_nk1);
    print_validation_summary(&result);

    println!("\n=== Demo Complete ===");
}

fn create_valid_adt_a01() -> Message {
    let mut message = Message::new();

    // MSH
    let mut msh = Segment::new("MSH".to_string());
    for _ in 0..11 {
        msh.add_field(Field::from_value("dummy"));
    }
    message.add_segment(msh);

    // EVN
    let evn = Segment::new("EVN".to_string());
    message.add_segment(evn);

    // PID with required fields
    let mut pid = Segment::new("PID".to_string());
    pid.add_field(Field::from_value("1")); // Field 1
    pid.add_field(Field::new()); // Field 2
    pid.add_field(Field::from_value("12345^^^MRN")); // Field 3 - Patient ID (required)
    pid.add_field(Field::new()); // Field 4
    pid.add_field(Field::from_value("Doe^John^A")); // Field 5 - Patient Name (required)
    message.add_segment(pid);

    // PV1
    let pv1 = Segment::new("PV1".to_string());
    message.add_segment(pv1);

    message
}

fn create_message_missing_pid() -> Message {
    let mut message = Message::new();

    // MSH
    let mut msh = Segment::new("MSH".to_string());
    for _ in 0..11 {
        msh.add_field(Field::from_value("dummy"));
    }
    message.add_segment(msh);

    // EVN
    let evn = Segment::new("EVN".to_string());
    message.add_segment(evn);

    // Missing PID!

    // PV1
    let pv1 = Segment::new("PV1".to_string());
    message.add_segment(pv1);

    message
}

fn create_message_with_nk1() -> Message {
    let mut message = create_valid_adt_a01();

    // Add optional NK1 segments
    let mut nk1_1 = Segment::new("NK1".to_string());
    nk1_1.add_field(Field::from_value("1"));
    nk1_1.add_field(Field::from_value("Doe^Jane"));
    nk1_1.add_field(Field::from_value("SPO"));
    message.add_segment(nk1_1);

    let mut nk1_2 = Segment::new("NK1".to_string());
    nk1_2.add_field(Field::from_value("2"));
    nk1_2.add_field(Field::from_value("Doe^Bob"));
    nk1_2.add_field(Field::from_value("CHD"));
    message.add_segment(nk1_2);

    message
}

fn print_validation_summary(result: &vyna_hl7_conformance::ConformanceValidationResult) {
    if result.is_valid() {
        println!("  ✓ Message is VALID");
    } else {
        println!("  ✗ Message is INVALID");
    }

    println!("  Errors: {}", result.errors.len());
    println!("  Warnings: {}", result.warnings.len());

    if !result.errors.is_empty() {
        for (i, error) in result.errors.iter().enumerate().take(3) {
            println!(
                "    Error {}: [{}] {}",
                i + 1,
                error.location,
                error.message
            );
        }
        if result.errors.len() > 3 {
            println!("    ... and {} more errors", result.errors.len() - 3);
        }
    }
}
