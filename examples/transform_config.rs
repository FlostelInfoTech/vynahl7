//! Configuration-based message transformation example
//!
//! This example demonstrates using YAML configuration to define transformation rules.

use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;
use vyna_hl7_transform::config::TransformConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Configuration-Based Transformation Example ===\n");

    // Create a source message
    let source_msg = AdtBuilder::a01(Version::V2_5)
        .sending_application("lab")
        .sending_facility("hospital")
        .patient_id("pat-456")
        .sex("f")
        .date_of_birth("19850312")
        .build()?;

    // Encode and parse to get proper field structure
    let encoded = source_msg.encode();
    println!("Source message:");
    println!("{}\n", encoded.replace('\r', "\n"));

    let source = parse_message(&encoded)?;

    // Define transformation configuration in YAML
    let yaml_config = r#"
rules:
  # Copy patient ID
  - source: PID-3
    target: PID-3

  # Transform sex to uppercase
  - source: PID-8
    target: PID-8
    transform: uppercase

  # Transform sending application to uppercase
  - source: MSH-3
    target: MSH-3
    transform: uppercase

  # Transform sending facility to uppercase
  - source: MSH-4
    target: MSH-4
    transform: uppercase

  # Format date of birth
  - source: PID-7
    target: PID-7
    transform: format_date
    params:
      format: "YYYY-MM-DD"
"#;

    println!("Transformation configuration:");
    println!("{}\n", yaml_config);

    // Load configuration
    let config = TransformConfig::from_yaml(yaml_config)?;

    // Build transformer from configuration
    let transformer = config.build()?;
    println!("Loaded {} transformation rules\n", transformer.rule_count());

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
    println!("  Sex (uppercase): {}", terser.get("PID-8")?.unwrap_or("N/A"));
    println!("  Sending App (uppercase): {}", terser.get("MSH-3")?.unwrap_or("N/A"));
    println!("  Sending Facility (uppercase): {}", terser.get("MSH-4")?.unwrap_or("N/A"));
    println!("  Date of Birth (formatted): {}", terser.get("PID-7")?.unwrap_or("N/A"));

    Ok(())
}
