//! Example: Schema-based Validation
//!
//! This example demonstrates how to:
//! - Load schemas for different HL7 versions
//! - Validate messages against schemas
//! - Handle validation errors and warnings

use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;
use vyna_hl7_validator::{Validator, list_available_schemas};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HL7 Schema Validation Example ===\n");

    // List available schemas for different versions
    println!("--- Available Schemas ---");
    for version in [
        Version::V2_3,
        Version::V2_4,
        Version::V2_5,
        Version::V2_6,
        Version::V2_7,
    ] {
        let schemas = list_available_schemas(version);
        println!("{}: {:?}", version.as_str(), schemas);
    }
    println!();

    // Sample ADT^A01 message
    let adt_message = r"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20240315143000||ADT^A01|MSG0001|P|2.5
EVN|A01|20240315143000
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||(555)555-5555|||S||67890|123-45-6789
PV1|1|I|WARD^ROOM^BED|||ATTEND^DOCTOR^A|||MED||||1|||ATTEND^DOCTOR^A||VN12345";

    println!("--- Validating ADT^A01 Message with Schema ---");
    let message = parse_message(adt_message)?;

    // Create validator with schema for ADT^A01
    let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01")?;
    let result = validator.validate(&message);

    if result.is_valid() {
        println!("✓ Message is valid according to ADT^A01 schema");
    } else {
        println!("✗ Message validation failed:");
        for error in &result.errors {
            println!(
                "  {} [{}]: {}",
                error.location, error.error_type as i32, error.message
            );
        }
    }

    if !result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            println!("  {}: {}", warning.location, warning.message);
        }
    }
    println!();

    // Sample ORU^R01 message (lab results)
    let oru_message = r"MSH|^~\&|LAB|LabFacility|EHR|Hospital|20240315120000||ORU^R01|LAB12345|P|2.5
PID|1||MRN987654^^^MRN||SMITH^JANE^M||19750515|F
OBR|1|||CBC^Complete Blood Count^LN|||20240315120000
OBX|1|NM|WBC^White Blood Count^LN||8.5|10*3/uL|4.0-11.0|N|||F
OBX|2|NM|RBC^Red Blood Count^LN||4.5|10*6/uL|4.2-5.4|N|||F";

    println!("--- Validating ORU^R01 Message with Schema ---");
    let oru_msg = parse_message(oru_message)?;

    let oru_validator = Validator::for_message_type(Version::V2_5, "ORU", "R01")?;
    let oru_result = oru_validator.validate(&oru_msg);

    if oru_result.is_valid() {
        println!("✓ ORU message is valid according to ORU^R01 schema");
    } else {
        println!("✗ ORU message validation failed:");
        for error in &oru_result.errors {
            println!("  {}: {}", error.location, error.message);
        }
    }
    println!();

    // Demonstrate cross-version compatibility
    println!("--- Cross-Version Schema Validation ---");
    for version in [
        Version::V2_3,
        Version::V2_4,
        Version::V2_5,
        Version::V2_6,
        Version::V2_7,
    ] {
        match Validator::for_message_type(version, "ADT", "A01") {
            Ok(val) => {
                let res = val.validate(&message);
                println!(
                    "{}: {} (errors: {}, warnings: {})",
                    version.as_str(),
                    if res.is_valid() { "✓" } else { "✗" },
                    res.errors.len(),
                    res.warnings.len()
                );
            }
            Err(e) => println!("{}: Error loading schema - {}", version.as_str(), e),
        }
    }
    println!();

    // Test with invalid message
    println!("--- Testing Invalid Message ---");
    let invalid_message = r"MSH|^~\&|App|Fac||||20240315||ADT^A01|12345|P|2.5
PID|1|MISSING_REQUIRED_FIELDS";

    let inv_msg = parse_message(invalid_message)?;
    let inv_validator = Validator::for_message_type(Version::V2_5, "ADT", "A01")?;
    let inv_result = inv_validator.validate(&inv_msg);

    if !inv_result.is_valid() {
        println!("✗ Message validation failed (as expected):");
        for error in &inv_result.errors {
            println!("  {}: {}", error.location, error.message);
        }
    }
    println!();

    println!("=== Example Complete ===");

    Ok(())
}
