//! Basic template usage example
//!
//! This example demonstrates:
//! - Creating a message template programmatically
//! - Using the template engine with variables
//! - Creating a message from a template

use vyna_hl7_templates::{FieldTemplate, MessageTemplate, SegmentTemplate, TemplateEngine};


fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Template Usage Example ===\n");

    // Create a simple ADT^A01 template
    let mut template = MessageTemplate::new("Simple ADT", "2.5", "ADT", "A01")
        .with_description("Simple admission message template");

    // Add MSH segment template
    let mut msh = SegmentTemplate::new("MSH").required();
    msh.add_field(
        3,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{sending_app}}")
            .with_description("Sending Application"),
    );
    msh.add_field(
        4,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{sending_facility}}")
            .with_description("Sending Facility"),
    );
    template.add_segment(msh);

    // Add PID segment template
    let mut pid = SegmentTemplate::new("PID").required();
    pid.add_field(
        3,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{patient_id}}")
            .with_description("Patient ID"),
    );
    pid.add_field(
        5,
        FieldTemplate::new()
            .required()
            .with_placeholder("{{patient_name}}")
            .with_description("Patient Name"),
    );
    pid.add_field(
        7,
        FieldTemplate::new()
            .with_placeholder("{{date_of_birth}}")
            .with_description("Date of Birth"),
    );
    pid.add_field(
        8,
        FieldTemplate::new()
            .with_placeholder("{{sex}}")
            .with_description("Sex"),
    );
    template.add_segment(pid);

    println!("Template created: {}", template.name);
    println!("  Message Type: {}^{}", template.message_type, template.trigger_event);
    println!("  Segments: {}\n", template.segments.len());

    // Create template engine and set variables
    let mut engine = TemplateEngine::new();
    engine.set_variable("sending_app", "MyApp");
    engine.set_variable("sending_facility", "MyHospital");
    engine.set_variable("patient_id", "12345");
    engine.set_variable("patient_name", "Smith^John^A");
    engine.set_variable("date_of_birth", "19900515");
    engine.set_variable("sex", "M");

    println!("Variables set:");
    println!("  Sending Application: MyApp");
    println!("  Sending Facility: MyHospital");
    println!("  Patient ID: 12345");
    println!("  Patient Name: Smith^John^A");
    println!("  Date of Birth: 19900515");
    println!("  Sex: M\n");

    // Create message from template
    let message = engine.create_message(&template)?;

    println!("Message created successfully!");
    println!("  Segments: {}\n", message.segments.len());

    // Display the generated message
    let encoded = message.encode();
    println!("Generated HL7 message:");
    println!("{}\n", encoded.replace('\r', "\n"));

    // Verify field values
    println!("Field verification:");
    if message.segments.len() >= 2 {
        println!("  MSH-3 (Sending App): {:?}", message.segments[0].fields.get(3).and_then(|f| f.value()));
        println!("  PID-3 (Patient ID): {:?}", message.segments[1].fields.get(3).and_then(|f| f.value()));
        println!("  PID-5 (Patient Name): {:?}", message.segments[1].fields.get(5).and_then(|f| f.value()));
        println!("  PID-7 (DOB): {:?}", message.segments[1].fields.get(7).and_then(|f| f.value()));
        println!("  PID-8 (Sex): {:?}", message.segments[1].fields.get(8).and_then(|f| f.value()));
    }

    Ok(())
}
