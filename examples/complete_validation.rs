//! Complete validation example with data type and vocabulary validation

use vyna_hl7_core::{field::Field, segment::Segment, Message, Version};
use vyna_hl7_validator::{FieldDefinition, MessageSchema, SegmentDefinition, Validator};
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Complete HL7 Message Validation Example ===\n");

    // Create a custom schema with vocabulary table mappings
    let schema = create_schema_with_tables();

    // Test 1: Valid message with correct codes
    println!("Test 1: Valid message with correct vocabulary codes");
    let valid_msg = create_valid_message();
    validate_with_schema(&valid_msg, schema.clone())?;
    println!();

    // Test 2: Invalid gender code
    println!("Test 2: Invalid gender code (should be M/F/O/U)");
    let invalid_gender = create_message_with_invalid_gender();
    validate_with_schema(&invalid_gender, schema.clone())?;
    println!();

    // Test 3: Invalid patient class
    println!("Test 3: Invalid patient class (should be I/O/E/etc)");
    let invalid_class = create_message_with_invalid_class();
    validate_with_schema(&invalid_class, schema.clone())?;
    println!();

    // Test 4: Invalid processing ID
    println!("Test 4: Invalid processing ID (should be P/D/T)");
    let invalid_processing = create_message_with_invalid_processing();
    validate_with_schema(&invalid_processing, schema)?;
    println!();

    println!("✓ Complete validation examples finished!");
    Ok(())
}

fn create_schema_with_tables() -> MessageSchema {
    let mut schema = MessageSchema {
        message_type: "ADT".to_string(),
        trigger_event: "A01".to_string(),
        version: "2.5".to_string(),
        segments: HashMap::new(),
    };

    // MSH segment
    let mut msh_fields = HashMap::new();
    msh_fields.insert(
        11,
        FieldDefinition {
            name: "Processing ID".to_string(),
            data_type: "PT".to_string(),
            required: true,
            repeating: false,
            max_length: Some(3),
            table_id: Some("0103".to_string()), // Table 0103: Processing ID
            components: None,
        },
    );

    schema.segments.insert(
        "MSH".to_string(),
        SegmentDefinition {
            name: "Message Header".to_string(),
            required: true,
            repeating: false,
            fields: msh_fields,
        },
    );

    // PID segment
    let mut pid_fields = HashMap::new();
    pid_fields.insert(
        8,
        FieldDefinition {
            name: "Administrative Sex".to_string(),
            data_type: "IS".to_string(),
            required: false,
            repeating: false,
            max_length: Some(1),
            table_id: Some("0001".to_string()), // Table 0001: Administrative Sex
            components: None,
        },
    );

    schema.segments.insert(
        "PID".to_string(),
        SegmentDefinition {
            name: "Patient Identification".to_string(),
            required: true,
            repeating: false,
            fields: pid_fields,
        },
    );

    // PV1 segment
    let mut pv1_fields = HashMap::new();
    pv1_fields.insert(
        2,
        FieldDefinition {
            name: "Patient Class".to_string(),
            data_type: "IS".to_string(),
            required: true,
            repeating: false,
            max_length: Some(1),
            table_id: Some("0004".to_string()), // Table 0004: Patient Class
            components: None,
        },
    );

    schema.segments.insert(
        "PV1".to_string(),
        SegmentDefinition {
            name: "Patient Visit".to_string(),
            required: false,
            repeating: false,
            fields: pv1_fields,
        },
    );

    schema
}

fn create_valid_message() -> Message {
    let mut msg = Message::new();

    // MSH segment
    let mut msh = Segment::new("MSH");
    msh.add_field(Field::from_value("|"));
    msh.add_field(Field::from_value("^~\\&"));
    msh.add_field(Field::from_value("SendApp"));
    msh.add_field(Field::from_value("SendFac"));
    msh.add_field(Field::from_value("RecApp"));
    msh.add_field(Field::from_value("RecFac"));
    msh.add_field(Field::from_value("20240315143000"));
    msh.add_field(Field::from_value(""));
    msh.add_field(Field::from_value("ADT^A01"));
    msh.add_field(Field::from_value("MSG12345"));
    msh.add_field(Field::from_value("P")); // Valid: Production
    msh.add_field(Field::from_value("2.5"));
    msg.add_segment(msh);

    // PID segment
    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("12345"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("DOE^JOHN"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("19800101"));
    pid.add_field(Field::from_value("M")); // Valid: Male
    msg.add_segment(pid);

    // PV1 segment
    let mut pv1 = Segment::new("PV1");
    pv1.add_field(Field::from_value("1"));
    pv1.add_field(Field::from_value("I")); // Valid: Inpatient
    msg.add_segment(pv1);

    msg
}

fn create_message_with_invalid_gender() -> Message {
    let mut msg = Message::new();

    let mut msh = Segment::new("MSH");
    msh.add_field(Field::from_value("|"));
    msh.add_field(Field::from_value("^~\\&"));
    msh.add_field(Field::from_value("SendApp"));
    msh.add_field(Field::from_value("SendFac"));
    msh.add_field(Field::from_value("RecApp"));
    msh.add_field(Field::from_value("RecFac"));
    msh.add_field(Field::from_value("20240315143000"));
    msh.add_field(Field::from_value(""));
    msh.add_field(Field::from_value("ADT^A01"));
    msh.add_field(Field::from_value("MSG12345"));
    msh.add_field(Field::from_value("P"));
    msh.add_field(Field::from_value("2.5"));
    msg.add_segment(msh);

    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("12345"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("DOE^JOHN"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("19800101"));
    pid.add_field(Field::from_value("X")); // INVALID: Not in table 0001
    msg.add_segment(pid);

    let mut pv1 = Segment::new("PV1");
    pv1.add_field(Field::from_value("1"));
    pv1.add_field(Field::from_value("I"));
    msg.add_segment(pv1);

    msg
}

fn create_message_with_invalid_class() -> Message {
    let mut msg = Message::new();

    let mut msh = Segment::new("MSH");
    msh.add_field(Field::from_value("|"));
    msh.add_field(Field::from_value("^~\\&"));
    msh.add_field(Field::from_value("SendApp"));
    msh.add_field(Field::from_value("SendFac"));
    msh.add_field(Field::from_value("RecApp"));
    msh.add_field(Field::from_value("RecFac"));
    msh.add_field(Field::from_value("20240315143000"));
    msh.add_field(Field::from_value(""));
    msh.add_field(Field::from_value("ADT^A01"));
    msh.add_field(Field::from_value("MSG12345"));
    msh.add_field(Field::from_value("P"));
    msh.add_field(Field::from_value("2.5"));
    msg.add_segment(msh);

    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("12345"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("DOE^JOHN"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("19800101"));
    pid.add_field(Field::from_value("M"));
    msg.add_segment(pid);

    let mut pv1 = Segment::new("PV1");
    pv1.add_field(Field::from_value("1"));
    pv1.add_field(Field::from_value("Z")); // INVALID: Not in table 0004
    msg.add_segment(pv1);

    msg
}

fn create_message_with_invalid_processing() -> Message {
    let mut msg = Message::new();

    let mut msh = Segment::new("MSH");
    msh.add_field(Field::from_value("|"));
    msh.add_field(Field::from_value("^~\\&"));
    msh.add_field(Field::from_value("SendApp"));
    msh.add_field(Field::from_value("SendFac"));
    msh.add_field(Field::from_value("RecApp"));
    msh.add_field(Field::from_value("RecFac"));
    msh.add_field(Field::from_value("20240315143000"));
    msh.add_field(Field::from_value(""));
    msh.add_field(Field::from_value("ADT^A01"));
    msh.add_field(Field::from_value("MSG12345"));
    msh.add_field(Field::from_value("X")); // INVALID: Not in table 0103
    msh.add_field(Field::from_value("2.5"));
    msg.add_segment(msh);

    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("12345"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("DOE^JOHN"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("19800101"));
    pid.add_field(Field::from_value("M"));
    msg.add_segment(pid);

    let mut pv1 = Segment::new("PV1");
    pv1.add_field(Field::from_value("1"));
    pv1.add_field(Field::from_value("I"));
    msg.add_segment(pv1);

    msg
}

fn validate_with_schema(message: &Message, schema: MessageSchema) -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::with_schema(Version::V2_5, schema);
    let result = validator.validate(message);

    if result.is_valid() {
        println!("  ✓ Message is valid!");
    } else {
        println!("  ✗ Validation errors found:");
        for error in &result.errors {
            println!("    - [{}] {}", error.location, error.message);
        }
    }

    Ok(())
}
