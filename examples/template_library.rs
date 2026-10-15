//! Standard template library example
//!
//! This example demonstrates:
//! - Using the standard template library
//! - Creating messages from pre-built templates
//! - Validating messages against templates

use vyna_hl7_templates::{TemplateEngine, TemplateLibrary, TemplateValidator};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Standard Template Library Example ===\n");

    // Create the standard library
    let library = TemplateLibrary::new();

    // List available templates
    let templates = library.list_templates();
    println!("Available templates:");
    for name in &templates {
        if let Some(template) = library.get(name) {
            println!(
                "  {} - {}^{} ({})",
                name,
                template.message_type,
                template.trigger_event,
                template
                    .description
                    .as_ref()
                    .unwrap_or(&"No description".to_string())
            );
        }
    }
    println!();

    // Use the ADT_A01 template
    println!("=== Creating ADT^A01 Message ===\n");
    let adt_template = library.get("ADT_A01").unwrap();

    let mut engine = TemplateEngine::new();
    engine.set_variable("sending_app", "HospitalApp");
    engine.set_variable("sending_facility", "MainHospital");
    engine.set_variable("receiving_app", "CentralSystem");
    engine.set_variable("receiving_facility", "DataCenter");
    engine.set_variable("event_datetime", "20250120120000");
    engine.set_variable("patient_id", "MRN-12345");
    engine.set_variable("patient_name", "Doe^John^M^^Dr.");
    engine.set_variable("date_of_birth", "19850312");
    engine.set_variable("sex", "M");
    engine.set_variable("address", "123 Main St^^Springfield^IL^62701");
    engine.set_variable("patient_class", "I"); // Inpatient
    engine.set_variable("assigned_location", "4N^401^01");
    engine.set_variable("attending_doctor", "12345^Smith^Jane");
    engine.set_variable("visit_number", "V123456");

    let adt_message = engine.create_message(adt_template)?;

    println!("ADT^A01 message created:");
    println!("{}\n", adt_message.encode().replace('\r', "\n"));

    // Validate the message
    let validation_result = TemplateValidator::validate(&adt_message, adt_template);

    println!("Validation result:");
    println!("  Valid: {}", validation_result.valid);
    println!("  Errors: {}", validation_result.errors.len());
    println!("  Warnings: {}", validation_result.warnings.len());

    if !validation_result.errors.is_empty() {
        println!("\nErrors:");
        for error in &validation_result.errors {
            println!("  - {}", error.message);
            if let Some(location) = &error.location {
                println!("    Location: {}", location);
            }
        }
    }

    if !validation_result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &validation_result.warnings {
            println!("  - {}", warning.message);
            if let Some(location) = &warning.location {
                println!("    Location: {}", location);
            }
        }
    }

    // Use the ORU_R01 template
    println!("\n=== Creating ORU^R01 Message ===\n");
    let oru_template = library.get("ORU_R01").unwrap();

    let mut oru_engine = TemplateEngine::new();
    oru_engine.set_variable("sending_app", "LabSystem");
    oru_engine.set_variable("sending_facility", "MainLab");
    oru_engine.set_variable("patient_id", "LAB-789");
    oru_engine.set_variable("patient_name", "Johnson^Mary^L");
    oru_engine.set_variable("set_id", "1");
    oru_engine.set_variable("universal_service_id", "GLU^Glucose");
    oru_engine.set_variable("observation_datetime", "20250120080000");
    oru_engine.set_variable("value_type", "NM");
    oru_engine.set_variable("observation_id", "GLU^Glucose");
    oru_engine.set_variable("observation_value", "105");
    oru_engine.set_variable("units", "mg/dL");

    let oru_message = oru_engine.create_message(oru_template)?;

    println!("ORU^R01 message created:");
    println!("{}\n", oru_message.encode().replace('\r', "\n"));

    // Validate ORU message
    let oru_validation = TemplateValidator::validate(&oru_message, oru_template);
    println!(
        "ORU validation result: {}",
        if oru_validation.valid {
            "✓ Valid"
        } else {
            "✗ Invalid"
        }
    );

    Ok(())
}
