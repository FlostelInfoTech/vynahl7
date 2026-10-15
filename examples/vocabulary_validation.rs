//! Example demonstrating HL7 vocabulary/code set validation

use vyna_hl7_validator::{Hl7Table, TableRegistry};

fn main() {
    println!("=== HL7 Vocabulary/Code Set Validation Example ===\n");

    let registry = TableRegistry::new();

    // Test Administrative Sex (Table 0001)
    println!("--- Table 0001: Administrative Sex ---");
    test_code(&registry, "0001", "M", "Male");
    test_code(&registry, "0001", "F", "Female");
    test_code(&registry, "0001", "U", "Unknown");
    test_code(&registry, "0001", "X", "Invalid code");
    println!();

    // Test Patient Class (Table 0004)
    println!("--- Table 0004: Patient Class ---");
    test_code(&registry, "0004", "I", "Inpatient");
    test_code(&registry, "0004", "O", "Outpatient");
    test_code(&registry, "0004", "E", "Emergency");
    test_code(&registry, "0004", "Z", "Invalid code");
    println!();

    // Test Processing ID (Table 0103)
    println!("--- Table 0103: Processing ID ---");
    test_code(&registry, "0103", "P", "Production");
    test_code(&registry, "0103", "D", "Debugging");
    test_code(&registry, "0103", "T", "Training");
    test_code(&registry, "0103", "X", "Invalid code");
    println!();

    // Test Observation Result Status (Table 0085)
    println!("--- Table 0085: Observation Result Status ---");
    test_code(&registry, "0085", "F", "Final results");
    test_code(&registry, "0085", "P", "Preliminary");
    test_code(&registry, "0085", "C", "Correction");
    test_code(&registry, "0085", "Z", "Invalid code");
    println!();

    // Test Order Control Codes (Table 0119)
    println!("--- Table 0119: Order Control Codes ---");
    test_code(&registry, "0119", "NW", "New order");
    test_code(&registry, "0119", "CA", "Cancel order");
    test_code(&registry, "0119", "OK", "Order accepted");
    test_code(&registry, "0119", "INVALID", "Invalid code");
    println!();

    // Test Identifier Type (Table 0203)
    println!("--- Table 0203: Identifier Type ---");
    test_code(&registry, "0203", "MR", "Medical record number");
    test_code(&registry, "0203", "SS", "Social Security number");
    test_code(&registry, "0203", "DL", "Driver's license");
    test_code(&registry, "0203", "ZZ", "Invalid code");
    println!();

    // Demonstrate custom table
    println!("--- Custom Table Example ---");
    let mut custom_registry = TableRegistry::new();

    let mut custom_table = Hl7Table::new("9000", "Custom Facility Codes", "Local facility codes");
    custom_table.add_value("MAIN", "Main Hospital", false);
    custom_table.add_value("EAST", "East Wing", false);
    custom_table.add_value("WEST", "West Wing", false);
    custom_table.add_value("OLD", "Old Building (Deprecated)", true);

    custom_registry.add_table(custom_table);

    test_code(&custom_registry, "9000", "MAIN", "Main Hospital");
    test_code(&custom_registry, "9000", "EAST", "East Wing");
    test_code(&custom_registry, "9000", "OLD", "Deprecated code");
    test_code(&custom_registry, "9000", "INVALID", "Invalid custom code");
    println!();

    println!("✓ Vocabulary validation examples completed!");
}

fn test_code(registry: &TableRegistry, table_id: &str, code: &str, description: &str) {
    let result = registry.validate(table_id, code);

    let status = if result.is_valid() {
        "✓ VALID"
    } else {
        "✗ INVALID"
    };

    print!("  {} - {} ({})", status, description, code);

    if let Some(err) = result.error_message() {
        println!();
        println!("    Error: {}", err);
    } else {
        println!();
    }
}
