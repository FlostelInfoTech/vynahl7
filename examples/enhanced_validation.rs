//! Example demonstrating enhanced validation with data type checking

use vyna_hl7_core::{field::Field, segment::Segment, Message, Version};
use vyna_hl7_validator::Validator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Enhanced HL7 Message Validation Example ===\n");

    // Test 1: Valid ADT message
    println!("Test 1: Valid ADT^A01 message");
    let valid_message = create_valid_adt_message();
    validate_message(&valid_message, Version::V2_5, "ADT", "A01")?;
    println!();

    // Test 2: Invalid date format
    println!("Test 2: Invalid date format in PID-7");
    let invalid_date_message = create_message_with_invalid_date();
    validate_message(&invalid_date_message, Version::V2_5, "ADT", "A01")?;
    println!();

    // Test 3: Invalid numeric format
    println!("Test 3: Invalid numeric in PID-1");
    let invalid_numeric_message = create_message_with_invalid_numeric();
    validate_message(&invalid_numeric_message, Version::V2_5, "ADT", "A01")?;
    println!();

    // Test 4: Invalid message type format
    println!("Test 4: Invalid message type format");
    let invalid_msg_type = create_message_with_invalid_msg_type();
    validate_message(&invalid_msg_type, Version::V2_5, "ADT", "A01")?;
    println!();

    println!("✓ Enhanced validation examples completed!");
    Ok(())
}

fn create_valid_adt_message() -> Message {
    let mut msg = Message::new();

    // MSH segment
    let mut msh = Segment::new("MSH");
    msh.add_field(Field::from_value("|"));
    msh.add_field(Field::from_value("^~\\&"));
    msh.add_field(Field::from_value("SendApp"));
    msh.add_field(Field::from_value("SendFac"));
    msh.add_field(Field::from_value("RecApp"));
    msh.add_field(Field::from_value("RecFac"));
    msh.add_field(Field::from_value("20240315143000")); // Valid timestamp
    msh.add_field(Field::from_value(""));
    msh.add_field(Field::from_value("ADT^A01")); // Valid message type
    msh.add_field(Field::from_value("MSG12345"));
    msh.add_field(Field::from_value("P")); // Valid processing type
    msh.add_field(Field::from_value("2.5"));
    msg.add_segment(msh);

    // EVN segment
    let mut evn = Segment::new("EVN");
    evn.add_field(Field::from_value("A01"));
    evn.add_field(Field::from_value("20240315143000")); // Valid timestamp
    msg.add_segment(evn);

    // PID segment
    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1")); // Valid sequence ID
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("12345"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("DOE^JOHN"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("19800101")); // Valid date
    pid.add_field(Field::from_value("M"));
    msg.add_segment(pid);

    // PV1 segment
    let mut pv1 = Segment::new("PV1");
    pv1.add_field(Field::from_value("1"));
    pv1.add_field(Field::from_value("I"));
    msg.add_segment(pv1);

    msg
}

fn create_message_with_invalid_date() -> Message {
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

    let mut evn = Segment::new("EVN");
    evn.add_field(Field::from_value("A01"));
    evn.add_field(Field::from_value("20240315143000"));
    msg.add_segment(evn);

    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("12345"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("DOE^JOHN"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value("19801301")); // Invalid date (month 13)
    pid.add_field(Field::from_value("M"));
    msg.add_segment(pid);

    let mut pv1 = Segment::new("PV1");
    pv1.add_field(Field::from_value("1"));
    pv1.add_field(Field::from_value("I"));
    msg.add_segment(pv1);

    msg
}

fn create_message_with_invalid_numeric() -> Message {
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

    let mut evn = Segment::new("EVN");
    evn.add_field(Field::from_value("A01"));
    evn.add_field(Field::from_value("20240315143000"));
    msg.add_segment(evn);

    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("ABC")); // Invalid - should be numeric (SI type)
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

fn create_message_with_invalid_msg_type() -> Message {
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
    msh.add_field(Field::from_value("adt^A01")); // Invalid - should be uppercase
    msh.add_field(Field::from_value("MSG12345"));
    msh.add_field(Field::from_value("P"));
    msh.add_field(Field::from_value("2.5"));
    msg.add_segment(msh);

    let mut evn = Segment::new("EVN");
    evn.add_field(Field::from_value("A01"));
    evn.add_field(Field::from_value("20240315143000"));
    msg.add_segment(evn);

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

fn validate_message(
    message: &Message,
    version: Version,
    msg_type: &str,
    trigger_event: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let validator = Validator::for_message_type(version, msg_type, trigger_event)?;
    let result = validator.validate(message);

    if result.is_valid() {
        println!("  ✓ Message is valid!");
    } else {
        println!("  ✗ Message has {} validation error(s):", result.errors.len());
        for error in &result.errors {
            println!("    - [{}] {}", error.location, error.message);
        }
    }

    if !result.warnings.is_empty() {
        println!("  ⚠ Warnings ({}):", result.warnings.len());
        for warning in &result.warnings {
            println!("    - [{}] {}", warning.location, warning.message);
        }
    }

    Ok(())
}
