//! Basic Rules Engine Example
//!
//! This example demonstrates how to use the rules engine for custom business validation.
//!
//! Run with: cargo run --example rules_basic

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_validator::rules::{CrossFieldValidator, RuleSeverity, RulesEngine, ValidationRule};

fn main() {
    println!("=== Basic Rules Engine Example ===\n");

    // Create a test ADT^A01 message
    let message = create_test_adt_message();

    // Example 1: Simple field validation
    println!("Example 1: Simple Field Validation");
    println!("------------------------------------");
    simple_field_validation(&message);
    println!();

    // Example 2: Cross-field validation
    println!("Example 2: Cross-Field Validation");
    println!("----------------------------------");
    cross_field_validation(&message);
    println!();

    // Example 3: Custom rule with closure
    println!("Example 3: Custom Rule with Closure");
    println!("------------------------------------");
    custom_rule_validation(&message);
    println!();

    // Example 4: Multiple rules with different severities
    println!("Example 4: Multiple Rules with Mixed Severities");
    println!("------------------------------------------------");
    mixed_severity_validation(&message);
}

/// Example 1: Simple field validation using CrossFieldValidator
fn simple_field_validation(message: &Message) {
    let mut engine = RulesEngine::new();

    // Add a rule that checks if patient gender is provided
    engine.add_rule(CrossFieldValidator::field_valued(
        "gender_required",
        "Patient gender (PID-8) must be provided",
        RuleSeverity::Error,
        "PID-8",
    ));

    // Add a rule that checks if patient name is provided
    engine.add_rule(CrossFieldValidator::field_valued(
        "name_required",
        "Patient name (PID-5) must be provided",
        RuleSeverity::Error,
        "PID-5",
    ));

    // Validate the message
    let result = engine.validate(message);

    println!("Validation passed: {}", result.passed());
    println!("Total violations: {}", result.violations.len());

    for violation in &result.violations {
        println!(
            "  [{:?}] {}: {}",
            violation.severity, violation.rule_name, violation.message
        );
    }
}

/// Example 2: Cross-field validation patterns
fn cross_field_validation(message: &Message) {
    let mut engine = RulesEngine::new();

    // If-then rule: if patient is male (PID-8 = 'M'), then SSN (PID-19) should be provided
    engine.add_rule(CrossFieldValidator::if_then(
        "male_requires_ssn",
        "Male patients should have SSN (PID-19) provided",
        RuleSeverity::Warning,
        "PID-8",
        "M",
        "PID-19",
    ));

    // At-least-one rule: patient must have at least one identifier
    engine.add_rule(CrossFieldValidator::at_least_one(
        "patient_id_required",
        "Patient must have at least one identifier (PID-2, PID-3, or PID-4)",
        RuleSeverity::Error,
        vec!["PID-2", "PID-3", "PID-4"],
    ));

    // Field-in-set rule: patient class must be valid
    engine.add_rule(CrossFieldValidator::field_in_set(
        "valid_patient_class",
        "Patient class (PV1-2) must be I, O, or E",
        RuleSeverity::Error,
        "PV1-2",
        vec!["I", "O", "E"],
    ));

    // Validate the message
    let result = engine.validate(message);

    println!("Validation passed: {}", result.passed());
    println!("Errors: {}", result.errors().len());
    println!("Warnings: {}", result.warnings().len());

    for violation in &result.violations {
        println!(
            "  [{:?}] {}: {}",
            violation.severity, violation.rule_name, violation.message
        );
    }
}

/// Example 3: Custom rule using a closure
fn custom_rule_validation(message: &Message) {
    let mut engine = RulesEngine::new();

    // Create a custom rule with a closure
    let rule = ValidationRule::new(
        "valid_message_type",
        "Message type must be ADT",
        RuleSeverity::Error,
    )
    .with_condition(|msg| {
        use vyna_hl7_terser::Terser;
        let terser = Terser::new(msg);

        // Check if MSH-9-1 (message type) equals "ADT"
        if let Ok(Some(msg_type)) = terser.get("MSH-9-1") {
            msg_type.trim() == "ADT"
        } else {
            false
        }
    });

    engine.add_rule(rule);

    // Add another custom rule
    let age_rule = ValidationRule::new(
        "dob_reasonable",
        "Patient date of birth should be reasonable (within last 150 years)",
        RuleSeverity::Warning,
    )
    .with_condition(|msg| {
        use vyna_hl7_terser::Terser;
        let terser = Terser::new(msg);

        // Check if PID-7 (DOB) is valued and reasonable
        if let Ok(Some(dob)) = terser.get("PID-7") {
            let dob = dob.trim();
            if dob.len() >= 4
                && let Ok(year) = dob[0..4].parse::<i32>()
            {
                let current_year = 2025; // Could use chrono for actual current year
                return year >= current_year - 150 && year <= current_year;
            }
        }
        true // If no DOB or can't parse, pass the rule
    });

    engine.add_rule(age_rule);

    // Validate the message
    let result = engine.validate(message);

    println!("Validation passed: {}", result.passed());
    println!("Total violations: {}", result.violations.len());

    for violation in &result.violations {
        println!(
            "  [{:?}] {}: {}",
            violation.severity, violation.rule_name, violation.message
        );
    }
}

/// Example 4: Multiple rules with different severities
fn mixed_severity_validation(message: &Message) {
    let mut engine = RulesEngine::new();

    // Error-level rules
    engine.add_rule(CrossFieldValidator::field_valued(
        "msh_sending_app",
        "Sending application (MSH-3) must be provided",
        RuleSeverity::Error,
        "MSH-3",
    ));

    // Warning-level rules
    engine.add_rule(CrossFieldValidator::field_valued(
        "pid_dob",
        "Patient date of birth (PID-7) should be provided",
        RuleSeverity::Warning,
        "PID-7",
    ));

    // Info-level rules
    let info_rule = ValidationRule::new(
        "pid_email_present",
        "Patient email address is present",
        RuleSeverity::Info,
    )
    .with_condition(|msg| {
        use vyna_hl7_terser::Terser;
        let terser = Terser::new(msg);
        terser
            .get("PID-13")
            .ok()
            .flatten()
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    });

    engine.add_rule(info_rule);

    // Validate the message
    let result = engine.validate(message);

    println!("Validation passed: {}", result.passed());
    println!();
    println!("Errors: {}", result.errors().len());
    for err in result.errors() {
        println!("  [ERROR] {}: {}", err.rule_name, err.message);
    }

    println!("\nWarnings: {}", result.warnings().len());
    for warn in result.warnings() {
        println!("  [WARNING] {}: {}", warn.rule_name, warn.message);
    }

    println!("\nInfo: {}", result.infos().len());
    for info in result.infos() {
        println!("  [INFO] {}: {}", info.rule_name, info.message);
    }
}

/// Create a test ADT^A01 message
fn create_test_adt_message() -> Message {
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
    msh.fields.push(Field::from_value("P")); // MSH-11
    msh.fields.push(Field::from_value("2.5")); // MSH-12
    msg.segments.push(msh);

    // PID segment
    let mut pid = Segment::new("PID");
    pid.fields.push(Field::from_value("1")); // PID-1
    pid.fields.push(Field::from_value("")); // PID-2
    pid.fields.push(Field::from_value("123456")); // PID-3
    pid.fields.push(Field::from_value("")); // PID-4
    pid.fields.push(Field::from_value("Doe^John^A")); // PID-5
    pid.fields.push(Field::from_value("")); // PID-6
    pid.fields.push(Field::from_value("19900101")); // PID-7 (DOB)
    pid.fields.push(Field::from_value("M")); // PID-8 (Gender)
    msg.segments.push(pid);

    // PV1 segment
    let mut pv1 = Segment::new("PV1");
    pv1.fields.push(Field::from_value("1")); // PV1-1
    pv1.fields.push(Field::from_value("I")); // PV1-2 (Patient Class - Inpatient)
    pv1.fields.push(Field::from_value("Ward^Room^Bed")); // PV1-3
    msg.segments.push(pv1);

    msg
}
