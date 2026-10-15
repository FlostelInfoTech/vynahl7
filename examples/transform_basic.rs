//! Basic message transformation example
//!
//! This example demonstrates simple field-to-field mappings and transformations.

use vyna_hl7_core::Version;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;
use vyna_hl7_transform::{MessageTransformer, transforms};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Message Transformation Example ===\n");

    // Create a source message
    let source_msg = AdtBuilder::a01(Version::V2_5)
        .sending_application("LAB")
        .sending_facility("HOSPITAL")
        .patient_id("pat123")
        .sex("m")
        .build()?;

    // Encode and parse to get proper field structure
    let encoded = source_msg.encode();
    println!("Source message:");
    println!("{}\n", encoded.replace('\r', "\n"));

    let source = parse_message(&encoded)?;

    // Create transformer with simple mappings
    let mut transformer = MessageTransformer::new();

    // Copy patient ID as-is
    transformer.add_mapping("PID-3", "PID-3");

    // Transform sex to uppercase
    transformer.add_transform("PID-8", "PID-8", transforms::uppercase);

    // Copy MSH fields
    transformer.add_mapping("MSH-3", "MSH-3"); // Sending application
    transformer.add_mapping("MSH-4", "MSH-4"); // Sending facility

    // Transform the message
    let target = transformer.transform(&source)?;

    // Display transformed message
    let target_encoded = target.encode();
    println!("Transformed message:");
    println!("{}\n", target_encoded.replace('\r', "\n"));

    // Verify transformations
    let terser = Terser::new(&target);

    println!("Verification:");
    println!("  Patient ID: {}", terser.get("PID-3")?.unwrap_or("N/A"));
    println!(
        "  Sex (uppercase): {}",
        terser.get("PID-8")?.unwrap_or("N/A")
    );
    println!("  Sending App: {}", terser.get("MSH-3")?.unwrap_or("N/A"));
    println!(
        "  Sending Facility: {}",
        terser.get("MSH-4")?.unwrap_or("N/A")
    );

    Ok(())
}
