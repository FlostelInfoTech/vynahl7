//! Built-in Rules Example
//!
//! This example demonstrates how to use the built-in validation rules library
//! for common HL7 validation scenarios.
//!
//! Run with: cargo run --example rules_builtin

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_validator::rules::{BuiltinRules, RulesEngine};

fn main() {
    println!("=== Built-in Rules Example ===\n");

    // Create test messages
    let adt_message = create_adt_message();
    let oru_message = create_oru_message();

    // Example 1: Validate ADT message with segment-specific rules
    println!("Example 1: Segment-Specific Rules (MSH + PID + PV1)");
    println!("----------------------------------------------------");
    segment_rules_example(&adt_message);
    println!();

    // Example 2: Validate ADT message with message-type rules
    println!("Example 2: Message-Type Rules (ADT)");
    println!("------------------------------------");
    adt_message_example(&adt_message);
    println!();

    // Example 3: Validate ORU message
    println!("Example 3: ORU Message Validation");
    println!("----------------------------------");
    oru_message_example(&oru_message);
    println!();

    // Example 4: All available built-in rules
    println!("Example 4: All Built-in Rules");
    println!("------------------------------");
    all_rules_example(&adt_message);
}

/// Example 1: Use segment-specific rules
fn segment_rules_example(message: &Message) {
    let mut engine = RulesEngine::new();

    // Add MSH segment rules
    engine.add_rules(BuiltinRules::msh_rules());

    // Add PID segment rules
    engine.add_rules(BuiltinRules::pid_rules());

    // Add PV1 segment rules
    engine.add_rules(BuiltinRules::pv1_rules());

    println!("Loaded {} rules (MSH + PID + PV1)", engine.rule_count());

    // Validate
    let result = engine.validate(message);

    print_validation_result(&result);
}

/// Example 2: Use ADT message rules
fn adt_message_example(message: &Message) {
    let mut engine = RulesEngine::new();

    // Add all ADT message rules (MSH + PID + PV1)
    engine.add_rules(BuiltinRules::adt_rules());

    println!("Loaded {} ADT message rules", engine.rule_count());

    // Validate
    let result = engine.validate(message);

    print_validation_result(&result);
}

/// Example 3: Use ORU message rules
fn oru_message_example(message: &Message) {
    let mut engine = RulesEngine::new();

    // Add all ORU message rules (MSH + PID + OBR + OBX)
    engine.add_rules(BuiltinRules::oru_rules());

    println!("Loaded {} ORU message rules", engine.rule_count());

    // Validate
    let result = engine.validate(message);

    print_validation_result(&result);
}

/// Example 4: Use all available built-in rules
fn all_rules_example(message: &Message) {
    let mut engine = RulesEngine::new();

    // Add all built-in rules
    engine.add_rules(BuiltinRules::all_rules());

    println!("Loaded {} total built-in rules", engine.rule_count());

    // Validate
    let result = engine.validate(message);

    print_validation_result(&result);
}

/// Helper function to print validation results
fn print_validation_result(result: &vyna_hl7_validator::rules::RulesValidationResult) {
    println!("\nValidation Summary:");
    println!("  Passed: {}", result.passed());
    println!("  Errors: {}", result.errors().len());
    println!("  Warnings: {}", result.warnings().len());

    if !result.errors().is_empty() {
        println!("\nErrors:");
        for err in result.errors() {
            println!("  - {}: {}", err.rule_name, err.message);
        }
    }

    if !result.warnings().is_empty() {
        println!("\nWarnings:");
        for warn in result.warnings() {
            println!("  - {}: {}", warn.rule_name, warn.message);
        }
    }
}

/// Create a test ADT^A01 message
fn create_adt_message() -> Message {
    let mut msg = Message::default();

    // MSH segment
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("SendingApp")); // MSH-3
    msh.fields.push(Field::from_value("SendingFac")); // MSH-4
    msh.fields.push(Field::from_value("ReceivingApp")); // MSH-5
    msh.fields.push(Field::from_value("ReceivingFac")); // MSH-6
    msh.fields.push(Field::from_value("20231101120000")); // MSH-7
    msh.fields.push(Field::from_value("")); // MSH-8
    msh.fields.push(Field::from_value("ADT^A01^ADT_A01")); // MSH-9
    msh.fields.push(Field::from_value("MSG00001")); // MSH-10
    msh.fields.push(Field::from_value("P")); // MSH-11 (Production)
    msh.fields.push(Field::from_value("2.5")); // MSH-12
    msg.segments.push(msh);

    // PID segment
    let mut pid = Segment::new("PID");
    pid.fields.push(Field::from_value("1")); // PID-1
    pid.fields.push(Field::from_value("")); // PID-2
    pid.fields.push(Field::from_value("123456")); // PID-3 (Patient ID)
    pid.fields.push(Field::from_value("")); // PID-4
    pid.fields.push(Field::from_value("Doe^John^A")); // PID-5 (Patient Name)
    pid.fields.push(Field::from_value("")); // PID-6
    pid.fields.push(Field::from_value("19900101")); // PID-7 (DOB)
    pid.fields.push(Field::from_value("M")); // PID-8 (Gender)
    msg.segments.push(pid);

    // PV1 segment
    let mut pv1 = Segment::new("PV1");
    pv1.fields.push(Field::from_value("1")); // PV1-1
    pv1.fields.push(Field::from_value("I")); // PV1-2 (Patient Class - Inpatient)
    pv1.fields.push(Field::from_value("Ward^Room^Bed")); // PV1-3 (Location)
    pv1.fields.push(Field::from_value("")); // PV1-4
    pv1.fields.push(Field::from_value("")); // PV1-5
    pv1.fields.push(Field::from_value("")); // PV1-6
    pv1.fields.push(Field::from_value("Smith^John^MD")); // PV1-7 (Attending Doctor)
    for _ in 8..19 {
        pv1.fields.push(Field::from_value(""));
    }
    pv1.fields.push(Field::from_value("VISIT123")); // PV1-19 (Visit Number)
    msg.segments.push(pv1);

    msg
}

/// Create a test ORU^R01 message
fn create_oru_message() -> Message {
    let mut msg = Message::default();

    // MSH segment
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("LabSystem")); // MSH-3
    msh.fields.push(Field::from_value("Hospital")); // MSH-4
    msh.fields.push(Field::from_value("RecvApp")); // MSH-5
    msh.fields.push(Field::from_value("RecvFac")); // MSH-6
    msh.fields.push(Field::from_value("20231101120000")); // MSH-7
    msh.fields.push(Field::from_value("")); // MSH-8
    msh.fields.push(Field::from_value("ORU^R01^ORU_R01")); // MSH-9
    msh.fields.push(Field::from_value("MSG00002")); // MSH-10
    msh.fields.push(Field::from_value("P")); // MSH-11
    msh.fields.push(Field::from_value("2.5")); // MSH-12
    msg.segments.push(msh);

    // PID segment
    let mut pid = Segment::new("PID");
    pid.fields.push(Field::from_value("1")); // PID-1
    pid.fields.push(Field::from_value("")); // PID-2
    pid.fields.push(Field::from_value("LAB456")); // PID-3
    pid.fields.push(Field::from_value("")); // PID-4
    pid.fields.push(Field::from_value("Smith^Jane^M")); // PID-5
    pid.fields.push(Field::from_value("")); // PID-6
    pid.fields.push(Field::from_value("19850615")); // PID-7
    pid.fields.push(Field::from_value("F")); // PID-8
    msg.segments.push(pid);

    // OBR segment
    let mut obr = Segment::new("OBR");
    obr.fields.push(Field::from_value("1")); // OBR-1
    obr.fields.push(Field::from_value("ORDER123")); // OBR-2
    obr.fields.push(Field::from_value("FILLER456")); // OBR-3
    obr.fields.push(Field::from_value("CBC^Complete Blood Count^LN")); // OBR-4 (Universal Service ID)
    obr.fields.push(Field::from_value("")); // OBR-5
    obr.fields.push(Field::from_value("")); // OBR-6
    obr.fields.push(Field::from_value("20231101080000")); // OBR-7 (Observation DateTime)
    for _ in 8..16 {
        obr.fields.push(Field::from_value(""));
    }
    obr.fields.push(Field::from_value("Jones^Robert^MD")); // OBR-16 (Ordering Provider)
    for _ in 17..25 {
        obr.fields.push(Field::from_value(""));
    }
    obr.fields.push(Field::from_value("F")); // OBR-25 (Result Status - Final)
    msg.segments.push(obr);

    // OBX segment
    let mut obx = Segment::new("OBX");
    obx.fields.push(Field::from_value("1")); // OBX-1
    obx.fields.push(Field::from_value("NM")); // OBX-2 (Value Type - Numeric)
    obx.fields.push(Field::from_value("WBC^White Blood Cell Count^LN")); // OBX-3 (Observation Identifier)
    obx.fields.push(Field::from_value("")); // OBX-4
    obx.fields.push(Field::from_value("7.5")); // OBX-5 (Observation Value)
    obx.fields.push(Field::from_value("10^3/uL")); // OBX-6
    obx.fields.push(Field::from_value("4.0-11.0")); // OBX-7
    obx.fields.push(Field::from_value("N")); // OBX-8
    obx.fields.push(Field::from_value("")); // OBX-9
    obx.fields.push(Field::from_value("")); // OBX-10
    obx.fields.push(Field::from_value("F")); // OBX-11 (Result Status - Final)
    msg.segments.push(obx);

    msg
}
