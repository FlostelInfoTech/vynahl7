//! Example: Parsing an ADT^A01 message
//!
//! This example demonstrates how to:
//! - Parse an HL7 ADT^A01 message
//! - Access message fields using direct access
//! - Access fields using the Terser API
//! - Validate the message

use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;
use vyna_hl7_validator::Validator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample ADT^A01 message (Admit/Visit Notification)
    let hl7_message = r"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20240315143000||ADT^A01|MSG0001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||(555)555-5555|||S||67890|123-45-6789
PV1|1|I|WARD^ROOM^BED|||ATTEND^DOCTOR^A|||MED||||1|||ATTEND^DOCTOR^A||VN12345|||||||||||||||||||||||||20240315143000";

    println!("=== HL7 Message Parser Example ===\n");

    // Parse the message
    println!("Parsing message...");
    let message = parse_message(hl7_message)?;
    println!("✓ Message parsed successfully\n");

    // Display basic message info
    println!("--- Message Information ---");
    println!("Number of segments: {}", message.segment_count());
    println!(
        "Sending Application: {:?}",
        message.get_sending_application()
    );
    println!(
        "Receiving Application: {:?}",
        message.get_receiving_application()
    );
    println!("Message Control ID: {:?}", message.get_control_id());
    println!("HL7 Version: {:?}", message.get_version());

    if let Some((msg_type, trigger)) = message.get_message_type() {
        println!("Message Type: {}^{}", msg_type, trigger);
    }
    println!();

    // Access fields directly
    println!("--- Direct Field Access ---");
    if let Some(pid) = message.get_segments_by_id("PID").first() {
        println!("PID Segment:");
        println!("  Set ID: {:?}", pid.get_field_value(1));
        println!("  Patient ID: {:?}", pid.get_field_value(2));
        println!("  Alternate ID: {:?}", pid.get_field_value(3));

        // Access patient name components
        if let Some(name_field) = pid.get_field(5)
            && let Some(rep) = name_field.get_repetition(0)
        {
            println!("  Patient Name:");
            println!(
                "    Family: {:?}",
                rep.get_component(0).and_then(|c| c.value())
            );
            println!(
                "    Given: {:?}",
                rep.get_component(1).and_then(|c| c.value())
            );
            println!(
                "    Middle: {:?}",
                rep.get_component(2).and_then(|c| c.value())
            );
        }
    }
    println!();

    // Use Terser API for easier access
    println!("--- Terser API Access ---");
    let terser = Terser::new(&message);

    println!("Patient Demographics:");
    println!("  Family Name: {:?}", terser.get("PID-5-1")?);
    println!("  Given Name: {:?}", terser.get("PID-5-2")?);
    println!("  DOB: {:?}", terser.get("PID-7")?);
    println!("  Gender: {:?}", terser.get("PID-8")?);
    println!("  SSN: {:?}", terser.get("PID-19")?);

    println!("\nPatient Visit:");
    println!("  Patient Class: {:?}", terser.get("PV1-2")?);
    println!("  Assigned Location: {:?}", terser.get("PV1-3-1")?);
    println!("  Attending Doctor: {:?}", terser.get("PV1-7-2")?);
    println!("  Visit Number: {:?}", terser.get("PV1-19")?);
    println!();

    // Validate the message
    println!("--- Message Validation ---");
    let validator = Validator::new(Version::V2_5);
    let validation_result = validator.validate(&message);

    if validation_result.is_valid() {
        println!("✓ Message is valid");
    } else {
        println!("✗ Message validation failed");
        for error in &validation_result.errors {
            println!("  Error at {}: {}", error.location, error.message);
        }
    }

    if !validation_result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &validation_result.warnings {
            println!("  Warning at {}: {}", warning.location, warning.message);
        }
    }
    println!();

    // Re-encode the message
    println!("--- Message Encoding ---");
    let encoded = message.encode();
    println!("Encoded message (first 100 chars):");
    println!("{}\n", encoded.chars().take(100).collect::<String>());

    println!("=== Example Complete ===");

    Ok(())
}
