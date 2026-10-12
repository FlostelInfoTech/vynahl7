//! Declarative Rules Example
//!
//! This example demonstrates how to load validation rules from YAML configuration files.
//!
//! Run with: cargo run --example rules_declarative

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_validator::rules::{RuleConfig, RulesEngine};

fn main() {
    println!("=== Declarative Rules Example ===\n");

    // Create a test ADT^A01 message
    let message = create_test_adt_message();

    // Example 1: Load rules from YAML string
    println!("Example 1: Load Rules from YAML");
    println!("--------------------------------");
    yaml_rules_example(&message);
    println!();

    // Example 2: Load rules from JSON string
    println!("Example 2: Load Rules from JSON");
    println!("--------------------------------");
    json_rules_example(&message);
    println!();

    // Example 3: Comprehensive YAML ruleset
    println!("Example 3: Comprehensive YAML Ruleset");
    println!("--------------------------------------");
    comprehensive_yaml_example(&message);
}

/// Example 1: Load validation rules from a YAML string
fn yaml_rules_example(message: &Message) {
    let yaml = r#"
rules:
  - name: "patient_gender_required"
    description: "Patient gender (PID-8) must be provided"
    severity: "error"
    condition:
      type: "field_valued"
      field: "PID-8"

  - name: "patient_name_required"
    description: "Patient name (PID-5) must be provided"
    severity: "error"
    condition:
      type: "field_valued"
      field: "PID-5"

  - name: "dob_recommended"
    description: "Patient date of birth (PID-7) should be provided"
    severity: "warning"
    condition:
      type: "field_valued"
      field: "PID-7"
"#;

    // Parse the YAML configuration
    match RuleConfig::from_yaml_str(yaml) {
        Ok(config) => {
            println!("Loaded {} rules from YAML", config.rules.len());

            // Convert to ValidationRules
            match config.into_validation_rules() {
                Ok(rules) => {
                    // Create engine and add rules
                    let mut engine = RulesEngine::new();
                    engine.add_rules(rules);

                    // Validate
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
                Err(e) => println!("Error converting rules: {}", e),
            }
        }
        Err(e) => println!("Error parsing YAML: {}", e),
    }
}

/// Example 2: Load validation rules from a JSON string
fn json_rules_example(message: &Message) {
    let json = r#"
{
  "rules": [
    {
      "name": "patient_id_required",
      "description": "At least one patient identifier must be provided",
      "severity": "error",
      "condition": {
        "type": "at_least_one",
        "fields": ["PID-2", "PID-3", "PID-4"]
      }
    },
    {
      "name": "valid_patient_class",
      "description": "Patient class must be valid",
      "severity": "error",
      "condition": {
        "type": "field_in_set",
        "field": "PV1-2",
        "values": ["I", "O", "E"]
      }
    }
  ]
}
"#;

    // Parse the JSON configuration
    match RuleConfig::from_json_str(json) {
        Ok(config) => {
            println!("Loaded {} rules from JSON", config.rules.len());

            // Convert to ValidationRules and validate
            match config.into_validation_rules() {
                Ok(rules) => {
                    let mut engine = RulesEngine::new();
                    engine.add_rules(rules);

                    let result = engine.validate(message);
                    println!("Validation passed: {}", result.passed());

                    for violation in &result.violations {
                        println!(
                            "  [{:?}] {}: {}",
                            violation.severity, violation.rule_name, violation.message
                        );
                    }
                }
                Err(e) => println!("Error converting rules: {}", e),
            }
        }
        Err(e) => println!("Error parsing JSON: {}", e),
    }
}

/// Example 3: Comprehensive YAML ruleset demonstrating all condition types
fn comprehensive_yaml_example(message: &Message) {
    let yaml = r#"
rules:
  # Field valued rule
  - name: "sending_application"
    description: "Sending application (MSH-3) must be provided"
    severity: "error"
    condition:
      type: "field_valued"
      field: "MSH-3"

  # If-then conditional rule
  - name: "inpatient_needs_location"
    description: "Inpatient must have assigned location"
    severity: "warning"
    condition:
      type: "if_then"
      if_field: "PV1-2"
      if_value: "I"
      then_field: "PV1-3"

  # At-least-one rule
  - name: "patient_identifier"
    description: "Patient must have at least one identifier"
    severity: "error"
    condition:
      type: "at_least_one"
      fields: ["PID-2", "PID-3", "PID-4"]

  # Field-in-set rule
  - name: "valid_gender"
    description: "Patient gender must be valid"
    severity: "warning"
    condition:
      type: "field_in_set"
      field: "PID-8"
      values: ["F", "M", "O", "U", "A", "N"]

  # All-or-none rule
  - name: "complete_name"
    description: "Patient name must be complete or empty"
    severity: "info"
    condition:
      type: "all_or_none"
      fields: ["PID-5-1", "PID-5-2"]

  # Mutually exclusive rule
  - name: "exclusive_ids"
    description: "Patient ID and alternate ID are mutually exclusive"
    severity: "info"
    condition:
      type: "mutually_exclusive"
      fields: ["PID-2", "PID-4"]

  # Dependent fields rule
  - name: "phone_needs_type"
    description: "Phone number requires phone type"
    severity: "warning"
    condition:
      type: "dependent_fields"
      primary_field: "PID-13"
      dependent_field: "PID-13-2"
"#;

    match RuleConfig::from_yaml_str(yaml) {
        Ok(config) => {
            println!("Loaded {} rules demonstrating all condition types", config.rules.len());

            match config.into_validation_rules() {
                Ok(rules) => {
                    let mut engine = RulesEngine::new();
                    engine.add_rules(rules);

                    let result = engine.validate(message);

                    println!("\nValidation Summary:");
                    println!("  Passed: {}", result.passed());
                    println!("  Errors: {}", result.errors().len());
                    println!("  Warnings: {}", result.warnings().len());
                    println!("  Info: {}", result.infos().len());

                    if !result.violations.is_empty() {
                        println!("\nViolations:");
                        for violation in &result.violations {
                            println!(
                                "  [{:?}] {}: {}",
                                violation.severity, violation.rule_name, violation.message
                            );
                        }
                    }
                }
                Err(e) => println!("Error converting rules: {}", e),
            }
        }
        Err(e) => println!("Error parsing YAML: {}", e),
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
    pid.fields.push(Field::from_value("Doe^John")); // PID-5
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
