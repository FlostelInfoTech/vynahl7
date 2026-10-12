//! Content-based routing example
//!
//! This example demonstrates how to route HL7 messages to different handlers
//! based on message content using the ContentRouter.

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_orchestration::routing::ContentRouter;
use vyna_hl7_terser::Terser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Content-Based Routing Example ===\n");

    // Create a router
    let mut router = ContentRouter::new();

    // Add route for ADT messages
    router.add_route(
        "adt_handler",
        |msg| {
            let terser = Terser::new(msg);
            terser
                .get("MSH-9-1")
                .ok()
                .flatten()
                .map(|v| v.contains("ADT"))
                .unwrap_or(false)
        },
        |msg| {
            Box::pin(async move {
                println!("📋 Processing ADT message");
                let terser = Terser::new(&msg);
                if let Some(event) = terser.get("MSH-9-2").ok().flatten() {
                    println!("   Event Type: ADT^{}", event);
                }
                if let Some(patient_id) = terser.get("PID-3-1").ok().flatten() {
                    println!("   Patient ID: {}", patient_id);
                }
                Ok(msg)
            })
        },
    );

    // Add route for ORU messages
    router.add_route(
        "oru_handler",
        |msg| {
            let terser = Terser::new(msg);
            terser
                .get("MSH-9-1")
                .ok()
                .flatten()
                .map(|v| v.contains("ORU"))
                .unwrap_or(false)
        },
        |msg| {
            Box::pin(async move {
                println!("🔬 Processing ORU (lab results) message");
                let terser = Terser::new(&msg);
                if let Some(event) = terser.get("MSH-9-2").ok().flatten() {
                    println!("   Event Type: ORU^{}", event);
                }
                Ok(msg)
            })
        },
    );

    // Set a default handler for unmatched messages
    router.set_default_handler(|msg| {
        Box::pin(async move {
            println!("⚠️  Unrecognized message type - using default handler");
            Ok(msg)
        })
    });

    // Create test messages
    let adt_message = create_adt_message();
    let oru_message = create_oru_message();
    let unknown_message = create_unknown_message();

    // Route the messages
    println!("Routing ADT message:");
    router.route(adt_message).await?;
    println!();

    println!("Routing ORU message:");
    router.route(oru_message).await?;
    println!();

    println!("Routing unknown message:");
    router.route(unknown_message).await?;
    println!();

    println!("✅ Routing completed successfully!");

    Ok(())
}

fn create_adt_message() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("HospitalSystem"));
    msh.fields.push(Field::from_value("Hospital"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ADT^A01"));
    msg.segments.push(msh);

    let mut pid = Segment::new("PID");
    pid.fields.push(Field::from_value(""));
    pid.fields.push(Field::from_value(""));
    pid.fields.push(Field::from_value("12345^^^Hospital^MR"));
    msg.segments.push(pid);

    msg
}

fn create_oru_message() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("LabSystem"));
    msh.fields.push(Field::from_value("Hospital"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ORU^R01"));
    msg.segments.push(msh);
    msg
}

fn create_unknown_message() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("System"));
    msh.fields.push(Field::from_value("Hospital"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("XXX^X99"));
    msg.segments.push(msh);
    msg
}
