//! Advanced message transformation example
//!
//! This example demonstrates advanced transformation features including:
//! - Custom transformation functions
//! - Context data for parameterized transformations
//! - Conditional transformations with defaults
//! - String manipulation (prefix, suffix, regex)

use vyna_hl7_core::Version;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;
use vyna_hl7_transform::{MessageTransformer, rule::TransformationRule, transforms};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Advanced Message Transformation Example ===\n");

    // Create a source message with some empty fields
    let source_msg = AdtBuilder::a01(Version::V2_5)
        .sending_application("lab_system")
        .patient_id("12345")
        .date_of_birth("19900515")
        .build()?;

    // Encode and parse to get proper field structure
    let encoded = source_msg.encode();
    println!("Source message:");
    println!("{}\n", encoded.replace('\r', "\n"));

    let source = parse_message(&encoded)?;

    // Create transformer with advanced rules
    let mut transformer = MessageTransformer::new();

    // Add prefix to patient ID
    transformer.set_context_data("prefix".to_string(), "MRN-".to_string());
    transformer.add_transform("PID-3", "PID-3", transforms::prefix);

    // Format date of birth to YYYY-MM-DD
    transformer.set_context_data("format".to_string(), "YYYY-MM-DD".to_string());
    transformer.add_transform("PID-7", "PID-7", transforms::format_date);

    // Replace underscore with hyphen in sending application
    transformer.set_context_data("pattern".to_string(), "_".to_string());
    transformer.set_context_data("replacement".to_string(), "-".to_string());
    transformer.add_transform("MSH-3", "MSH-3", transforms::replace);

    // Add default value for sex if not present
    let sex_rule = TransformationRule::new("PID-8", "PID-8")
        .with_default("U") // Unknown
        .skip_if_empty(false);
    transformer.add_rule(sex_rule);

    // Transform to uppercase after adding default
    transformer.add_transform("PID-8", "PID-8", transforms::uppercase);

    // Transform the message
    let target = transformer.transform(&source)?;

    // Display transformed message
    let target_encoded = target.encode();
    println!("Transformed message:");
    println!("{}\n", target_encoded.replace('\r', "\n"));

    // Verify transformations
    let terser = Terser::new(&target);

    println!("Verification:");
    println!(
        "  Patient ID (with prefix): {}",
        terser.get("PID-3")?.unwrap_or("N/A")
    );
    println!(
        "  Date of Birth (formatted): {}",
        terser.get("PID-7")?.unwrap_or("N/A")
    );
    println!(
        "  Sending App (replaced): {}",
        terser.get("MSH-3")?.unwrap_or("N/A")
    );
    println!(
        "  Sex (default + uppercase): {}",
        terser.get("PID-8")?.unwrap_or("N/A")
    );

    println!("\n=== Custom Transform Function Example ===\n");

    // Example with custom transformation function
    fn anonymize(
        value: &str,
        _ctx: &vyna_hl7_transform::rule::TransformContext,
    ) -> vyna_hl7_transform::Result<String> {
        // Replace all characters except first and last with '*'
        if value.len() <= 2 {
            return Ok("***".to_string());
        }
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        let middle = "*".repeat(value.len() - 2);
        Ok(format!("{}{}{}", first, middle, last))
    }

    // Create new transformer for anonymization
    let mut anon_transformer = MessageTransformer::new();
    anon_transformer.add_transform("PID-3", "PID-3", anonymize);

    let anon_target = anon_transformer.transform(&source)?;
    let anon_terser = Terser::new(&anon_target);

    println!(
        "Original Patient ID: {}",
        Terser::new(&source).get("PID-3")?.unwrap_or("N/A")
    );
    println!(
        "Anonymized Patient ID: {}",
        anon_terser.get("PID-3")?.unwrap_or("N/A")
    );

    Ok(())
}
