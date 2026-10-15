//! Example demonstrating Phase 2 conformance profile features
//!
//! This example shows:
//! - Conditional usage with predicates (C usage code)
//! - Predicate evaluation for field dependencies
//! - Component-level validation
//! - Value set bindings
//!
//! Run with:
//! ```bash
//! cargo run --example predicate_validation -p vyna-hl7-conformance
//! ```

use vyna_hl7_conformance::{
    BindingStrength, Cardinality, ComponentProfile, ConditionalUsage, ConformanceProfile,
    ConformanceValidator, FieldProfile, MessageProfile, Predicate, ProfileMetadata, SegmentProfile,
    Usage, ValueSetBinding,
};
use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Phase 2 Conformance Profile Example ===\n");

    // Create a conformance profile with conditional usage
    let profile = create_adt_profile_with_predicates();

    println!("Profile created: {}", profile.metadata.name);
    println!(
        "  Message Type: {}^{}\n",
        profile.message.message_type, profile.message.trigger_event
    );

    // Test case 1: Message with male patient
    println!("--- Test Case 1: Male Patient ---");
    let male_message = "MSH|^~\\&|SENDING_APP|SENDING_FAC|REC_APP|REC_FAC|20240101120000||ADT^A01|MSG001|P|2.5\r\
                        PID|||12345||Doe^John||19900101|M\r\
                        PV1||I|4N^401";

    let message = parse_message(male_message)?;
    let validator = ConformanceValidator::new(profile.clone());
    let result = validator.validate(&message);

    println!(
        "Validation result: {}",
        if result.is_valid() {
            "VALID"
        } else {
            "INVALID"
        }
    );
    if !result.is_valid() {
        println!("Errors:");
        for error in &result.errors {
            println!("  - {}", error.message);
        }
    }
    println!();

    // Test case 2: Message with inpatient - PV1 required
    println!("--- Test Case 2: Inpatient (PV1 Required) ---");
    let inpatient_message = "MSH|^~\\&|SENDING_APP|SENDING_FAC|REC_APP|REC_FAC|20240101120000||ADT^A01|MSG002|P|2.5\r\
                             PID|||12345||Doe^Jane||19850515|F\r\
                             PV1||I|4N^402";

    let message2 = parse_message(inpatient_message)?;
    let result2 = validator.validate(&message2);

    println!(
        "Validation result: {}",
        if result2.is_valid() {
            "VALID"
        } else {
            "INVALID"
        }
    );
    println!();

    // Test case 3: Message missing PV1 when patient class is Inpatient
    println!("--- Test Case 3: Inpatient Missing PV1 (Should Fail) ---");
    let invalid_message = "MSH|^~\\&|SENDING_APP|SENDING_FAC|REC_APP|REC_FAC|20240101120000||ADT^A01|MSG003|P|2.5\r\
                           PID|||12345||Doe^Bob||19920301|M";

    let message3 = parse_message(invalid_message)?;
    let result3 = validator.validate(&message3);

    println!(
        "Validation result: {}",
        if result3.is_valid() {
            "VALID"
        } else {
            "INVALID"
        }
    );
    if !result3.is_valid() {
        println!("Expected errors (PV1 might be required based on predicates):");
        for error in &result3.errors {
            println!("  - {}", error.message);
        }
    }
    println!();

    // Demonstrate predicate parsing and evaluation
    println!("--- Predicate Evaluation Demo ---");
    demonstrate_predicate_evaluation()?;

    Ok(())
}

/// Create an ADT profile with conditional usage predicates
fn create_adt_profile_with_predicates() -> ConformanceProfile {
    // Profile metadata
    let metadata = ProfileMetadata::new(
        "ADT_A01_with_Predicates".to_string(),
        "1.0".to_string(),
        Version::V2_5,
    );

    // Message profile
    let mut message = MessageProfile::new("ADT".to_string(), "A01".to_string());

    // MSH segment (required)
    let mut msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());

    // MSH-9: Message Type (required, with components)
    let mut msg_type_field = FieldProfile::new(9, Usage::Required, Cardinality::one());
    msg_type_field.name = Some("Message Type".to_string());
    msg_type_field.datatype = Some("MSG".to_string());

    // Add components for message type
    let components = vec![
        ComponentProfile::new(1, ConditionalUsage::Required), // Message Code
        ComponentProfile::new(2, ConditionalUsage::Required), // Trigger Event
    ];
    msg_type_field = msg_type_field.with_components(components);

    msh.add_field(msg_type_field);
    message.add_segment(msh);

    // PID segment (required)
    let mut pid = SegmentProfile::new("PID".to_string(), Usage::Required, Cardinality::one());

    // PID-5: Patient Name (required)
    let mut name_field = FieldProfile::new(5, Usage::Required, Cardinality::one());
    name_field.name = Some("Patient Name".to_string());
    name_field.datatype = Some("XPN".to_string());
    pid.add_field(name_field);

    // PID-7: Date of Birth (required if known)
    let mut dob_field = FieldProfile::new(7, Usage::RequiredIfKnown, Cardinality::one());
    dob_field.name = Some("Date of Birth".to_string());
    dob_field.datatype = Some("TS".to_string());
    dob_field.length = Some(8);
    pid.add_field(dob_field);

    // PID-8: Administrative Sex (required with value set binding)
    let mut sex_field = FieldProfile::new(8, Usage::Required, Cardinality::one());
    sex_field.name = Some("Administrative Sex".to_string());
    sex_field.datatype = Some("IS".to_string());
    sex_field.length = Some(1);

    // Add value set binding for administrative sex
    let value_set = ValueSetBinding::new(
        "HL70001".to_string(), // HL7 Table 0001 - Administrative Sex
        BindingStrength::Required,
    );
    sex_field = sex_field.with_value_set(value_set);

    pid.add_field(sex_field);

    message.add_segment(pid);

    // PV1 segment with conditional usage
    // Condition: Required if patient class is "I" (Inpatient), otherwise optional
    // Note: This is simplified - in reality would check PV1-2
    let mut pv1 = SegmentProfile::new("PV1".to_string(), Usage::Optional, Cardinality::one());

    // PV1-2: Patient Class
    let mut patient_class_field = FieldProfile::new(2, Usage::Required, Cardinality::one());
    patient_class_field.name = Some("Patient Class".to_string());
    patient_class_field.datatype = Some("IS".to_string());
    patient_class_field.length = Some(1);
    pv1.add_field(patient_class_field);

    // PV1-3: Assigned Patient Location (conditional)
    // Required if patient class is "I", otherwise optional
    let predicate = Predicate::new("PV1-2 = 'I'".to_string(), Usage::Required, Usage::Optional)
        .with_description("Assigned location required for inpatients".to_string());

    let mut location_field = FieldProfile::with_conditional_usage(
        3,
        ConditionalUsage::Conditional(predicate),
        Cardinality::one(),
    );
    location_field.name = Some("Assigned Patient Location".to_string());
    location_field.datatype = Some("PL".to_string());

    pv1.add_field(location_field);

    message.add_segment(pv1);

    ConformanceProfile::new(metadata, message)
}

/// Demonstrate predicate parsing and evaluation
fn demonstrate_predicate_evaluation() -> Result<(), Box<dyn std::error::Error>> {
    use vyna_hl7_conformance::predicate::PredicateParser;

    println!("Parsing predicates:");

    // Example 1: IS VALUED
    let cond1 = PredicateParser::parse("PID-8 IS VALUED")?;
    println!("  Parsed: 'PID-8 IS VALUED' -> {:?}", cond1);

    // Example 2: Equality
    let cond2 = PredicateParser::parse("PV1-2 = 'I'")?;
    println!("  Parsed: \"PV1-2 = 'I'\" -> {:?}", cond2);

    // Example 3: Boolean logic
    let cond3 = PredicateParser::parse("PID-8 IS VALUED AND PV1-2 = 'I'")?;
    println!(
        "  Parsed: \"PID-8 IS VALUED AND PV1-2 = 'I'\" -> {:?}",
        cond3
    );

    // Example 4: Numeric comparison
    let cond4 = PredicateParser::parse("PID-7 > 18")?;
    println!("  Parsed: 'PID-7 > 18' -> {:?}", cond4);

    println!();

    // Evaluate against a test message
    println!("Evaluating against test message:");
    let test_msg = "MSH|^~\\&|APP|FAC|APP|FAC|20240101120000||ADT^A01|MSG001|P|2.5\r\
                    PID|||12345||Doe^John||19900101|M\r\
                    PV1||I|4N^401";

    let message = parse_message(test_msg)?;

    println!("  'PID-8 IS VALUED' -> {}", cond1.evaluate(&message));
    println!("  \"PV1-2 = 'I'\" -> {}", cond2.evaluate(&message));
    println!(
        "  'PID-8 IS VALUED AND PV1-2 = I' -> {}",
        cond3.evaluate(&message)
    );

    Ok(())
}
