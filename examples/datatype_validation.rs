//! Example demonstrating HL7 data type validation

use vyna_hl7_core::types::DataType;
use vyna_hl7_validator::validate_data_type;

fn main() {
    println!("=== HL7 Data Type Validation Example ===\n");

    // Date validation (DT)
    println!("--- Date (DT) Validation ---");
    test_validation("20240315", DataType::DT, "Valid full date");
    test_validation("202403", DataType::DT, "Valid year-month");
    test_validation("2024", DataType::DT, "Valid year only");
    test_validation("2024031", DataType::DT, "Invalid date (wrong length)");
    test_validation("20241301", DataType::DT, "Invalid date (bad month)");
    println!();

    // Time validation (TM)
    println!("--- Time (TM) Validation ---");
    test_validation("14", DataType::TM, "Valid hours only");
    test_validation("1430", DataType::TM, "Valid hours and minutes");
    test_validation("143000", DataType::TM, "Valid full time");
    test_validation("143000.123", DataType::TM, "Valid time with fractions");
    test_validation("2530", DataType::TM, "Invalid time (bad hours)");
    test_validation("1460", DataType::TM, "Invalid time (bad minutes)");
    println!();

    // Timestamp validation (DTM/TS)
    println!("--- Timestamp (DTM/TS) Validation ---");
    test_validation("20240315", DataType::DTM, "Valid date only");
    test_validation("20240315143000", DataType::DTM, "Valid full timestamp");
    test_validation("20240315143000.1234", DataType::DTM, "Valid timestamp with fractions");
    test_validation("202403151", DataType::DTM, "Invalid timestamp (wrong length)");
    println!();

    // Numeric validation (NM)
    println!("--- Numeric (NM) Validation ---");
    test_validation("123", DataType::NM, "Valid integer");
    test_validation("123.45", DataType::NM, "Valid decimal");
    test_validation("-123.45", DataType::NM, "Valid negative");
    test_validation("+123", DataType::NM, "Valid with plus sign");
    test_validation("abc", DataType::NM, "Invalid numeric");
    println!();

    // Sequence ID validation (SI)
    println!("--- Sequence ID (SI) Validation ---");
    test_validation("1", DataType::SI, "Valid sequence ID");
    test_validation("123", DataType::SI, "Valid sequence ID");
    test_validation("0", DataType::SI, "Invalid (must be positive)");
    test_validation("-1", DataType::SI, "Invalid (negative)");
    test_validation("abc", DataType::SI, "Invalid (not numeric)");
    println!();

    // Identifier validation (ID)
    println!("--- Identifier (ID) Validation ---");
    test_validation("ABC123", DataType::ID, "Valid alphanumeric");
    test_validation("test_id", DataType::ID, "Valid with underscore");
    test_validation("test-id", DataType::ID, "Valid with hyphen");
    test_validation("test id", DataType::ID, "Invalid (space)");
    test_validation("test@id", DataType::ID, "Invalid (special char)");
    println!();

    // Message Type validation (MSG)
    println!("--- Message Type (MSG) Validation ---");
    test_validation("ADT^A01", DataType::MSG, "Valid message type");
    test_validation("ORU^R01^ORU_R01", DataType::MSG, "Valid with structure");
    test_validation("ad^A01", DataType::MSG, "Invalid (not 3 chars)");
    test_validation("adt^A01", DataType::MSG, "Invalid (not uppercase)");
    println!();

    // Processing Type validation (PT)
    println!("--- Processing Type (PT) Validation ---");
    test_validation("P", DataType::PT, "Valid production");
    test_validation("D", DataType::PT, "Valid debugging");
    test_validation("T", DataType::PT, "Valid training");
    test_validation("X", DataType::PT, "Invalid processing type");
    println!();

    // Numeric Array validation (NA)
    println!("--- Numeric Array (NA) Validation ---");
    test_validation("1~2~3", DataType::NA, "Valid numeric array");
    test_validation("1.5~2.7~3.9", DataType::NA, "Valid with decimals");
    test_validation("1~abc~3", DataType::NA, "Invalid (non-numeric)");
    println!();

    println!("✓ Data type validation examples completed!");
}

fn test_validation(value: &str, data_type: DataType, description: &str) {
    let result = validate_data_type(value, data_type);
    let status = if result.is_valid() {
        "✓ VALID"
    } else {
        "✗ INVALID"
    };

    print!("  {} - {}: '{}'", status, description, value);
    if let Some(err) = result.error_message() {
        print!(" - {}", err);
    }
    println!();
}
