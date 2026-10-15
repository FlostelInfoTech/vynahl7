//! Template inheritance example
//!
//! This example demonstrates:
//! - Creating base templates
//! - Extending templates with inheritance
//! - Using the template resolver

use vyna_hl7_templates::{
    FieldTemplate, MessageTemplate, SegmentTemplate, TemplateEngine, TemplateResolver,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Template Inheritance Example ===\n");

    // Create a base ADT template with common segments
    println!("Creating base template...");
    let mut base_adt = MessageTemplate::new("BaseADT", "2.5", "ADT", "A01")
        .with_description("Base ADT template with core segments");

    // MSH segment
    let mut msh = SegmentTemplate::new("MSH").required();
    msh.add_field(
        3,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{sending_app}}"),
    );
    msh.add_field(
        4,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{sending_facility}}"),
    );
    base_adt.add_segment(msh);

    // PID segment
    let mut pid = SegmentTemplate::new("PID").required();
    pid.add_field(
        3,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{patient_id}}"),
    );
    pid.add_field(
        5,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{patient_name}}"),
    );
    base_adt.add_segment(pid);

    // Set some default variables
    base_adt.add_variable("sending_facility", "DefaultHospital");

    println!("  Base template has {} segments\n", base_adt.segments.len());

    // Create an extended template for inpatient admissions
    println!("Creating extended template for inpatient admissions...");
    let mut inpatient_adt = MessageTemplate::new("InpatientADT", "2.5", "ADT", "A01")
        .with_extends("BaseADT")
        .with_description("Extended ADT for inpatient admissions");

    // Add PV1 segment (visit information)
    let mut pv1 = SegmentTemplate::new("PV1").required();
    pv1.add_field(2, FieldTemplate::new().required().with_default("I")); // Inpatient
    pv1.add_field(
        3,
        FieldTemplate::new().with_placeholder("{{assigned_location}}"),
    );
    pv1.add_field(
        7,
        FieldTemplate::new().with_placeholder("{{attending_doctor}}"),
    );
    inpatient_adt.add_segment(pv1);

    // Override a variable
    inpatient_adt.add_variable("sending_facility", "InpatientWing");

    println!(
        "  Extended template has {} segments (before resolution)\n",
        inpatient_adt.segments.len()
    );

    // Create a specialized ICU template extending the inpatient template
    println!("Creating ICU-specific template...");
    let mut icu_adt = MessageTemplate::new("ICU_ADT", "2.5", "ADT", "A01")
        .with_extends("InpatientADT")
        .with_description("ICU admission template");

    // Add DG1 segment for diagnosis
    let mut dg1 = SegmentTemplate::new("DG1").required();
    dg1.add_field(
        3,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{diagnosis_code}}"),
    );
    dg1.add_field(
        4,
        FieldTemplate::new().with_placeholder("{{diagnosis_description}}"),
    );
    icu_adt.add_segment(dg1);

    // Override PV1 field for ICU location
    let mut pv1_override = SegmentTemplate::new("PV1").required();
    pv1_override.add_field(2, FieldTemplate::new().required().with_default("I"));
    pv1_override.add_field(3, FieldTemplate::new().required().with_default("ICU^1^01")); // ICU location
    pv1_override.add_field(
        7,
        FieldTemplate::new().with_placeholder("{{attending_doctor}}"),
    );
    icu_adt.add_segment(pv1_override);

    icu_adt.add_variable("sending_facility", "ICU");

    println!(
        "  ICU template has {} segments (before resolution)\n",
        icu_adt.segments.len()
    );

    // Create template resolver and register all templates
    println!("Resolving template hierarchy...");
    let mut resolver = TemplateResolver::new();
    resolver.register(base_adt);
    resolver.register(inpatient_adt);
    resolver.register(icu_adt);

    // Resolve the ICU template (should include all inherited segments)
    let resolved_icu = resolver.resolve("ICU_ADT")?;

    println!("  Resolved ICU template:");
    println!("    Segments: {}", resolved_icu.segments.len());
    for seg in &resolved_icu.segments {
        println!("      - {} (required: {})", seg.id, seg.required);
    }
    println!();

    // Check variables
    println!("  Variables in resolved template:");
    if let Some(vars) = &resolved_icu.variables {
        for (key, value) in vars {
            println!("    {} = {}", key, value);
        }
    }
    println!();

    // Create a message using the resolved template
    println!("Creating message from resolved ICU template...");
    let mut engine = TemplateEngine::new();
    engine.set_variable("sending_app", "ICU_System");
    engine.set_variable("patient_id", "ICU-9876");
    engine.set_variable("patient_name", "Critical^Patient^A");
    engine.set_variable("attending_doctor", "67890^Jones^Robert^^Dr.");
    engine.set_variable("diagnosis_code", "I21.0");
    engine.set_variable("diagnosis_description", "Acute MI");

    let message = engine.create_message(&resolved_icu)?;

    println!("\nGenerated ICU admission message:");
    println!("{}\n", message.encode().replace('\r', "\n"));

    println!("Message structure:");
    for (i, segment) in message.segments.iter().enumerate() {
        println!(
            "  {}: {} ({} fields)",
            i + 1,
            segment.id,
            segment.fields.len()
        );
    }

    // Demonstrate multi-level variable inheritance
    println!("\n=== Variable Inheritance ===");
    println!("sending_facility value in each template:");
    println!("  BaseADT: DefaultHospital (defined)");
    println!("  InpatientADT: InpatientWing (overridden)");
    println!("  ICU_ADT: ICU (overridden again)");
    println!(
        "  Resolved value: {}",
        resolved_icu
            .variables
            .as_ref()
            .and_then(|v| v.get("sending_facility"))
            .unwrap_or(&"N/A".to_string())
    );

    Ok(())
}
