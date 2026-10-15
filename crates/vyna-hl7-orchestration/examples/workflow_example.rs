//! Complete workflow example
//!
//! This example demonstrates a complete message processing workflow combining
//! filtering, orchestration steps with retry logic, and content-based routing.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_orchestration::{
    OrchestrationError,
    filtering::MessageFilter,
    orchestration::{MessageOrchestrator, RetryConfig},
    routing::ContentRouter,
};
use vyna_hl7_terser::Terser;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Complete Workflow Example ===\n");

    // Step 1: Create a filter for production messages only
    println!("Step 1: Setting up message filter (Production messages only)");
    let mut filter = MessageFilter::new();
    filter.add_rule("production_only", |msg| {
        let terser = Terser::new(msg);
        terser.get("MSH-11").ok().flatten() == Some("P")
    });

    // Step 2: Create an orchestrator for multi-step processing
    println!("Step 2: Setting up orchestration workflow");
    let mut orchestrator = MessageOrchestrator::new();

    // Add validation step
    orchestrator.add_step("validate", |msg| {
        Box::pin(async move {
            println!("   ✓ Validating message structure");
            let terser = Terser::new(&msg);

            // Check required fields
            if terser.get("MSH-3").ok().flatten().is_none() {
                return Err(OrchestrationError::custom("Missing sending application"));
            }

            Ok(msg)
        })
    });

    // Add enrichment step with retry logic
    let attempt_counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = attempt_counter.clone();

    orchestrator.add_step_with_retry(
        "enrich",
        move |msg| {
            let counter = counter_clone.clone();
            Box::pin(async move {
                let attempt = counter.fetch_add(1, Ordering::SeqCst);

                // Simulate intermittent failure (fail first 2 attempts)
                if attempt < 2 {
                    println!(
                        "   ⚠ Enrichment failed (attempt {}/3), retrying...",
                        attempt + 1
                    );
                    return Err(OrchestrationError::custom("Temporary enrichment failure"));
                }

                println!("   ✓ Enriching message with additional data");
                Ok(msg)
            })
        },
        RetryConfig::standard(), // 3 attempts, 100ms delay
    );

    // Add transformation step
    orchestrator.add_step("transform", |msg| {
        Box::pin(async move {
            println!("   ✓ Transforming message format");
            // In a real scenario, this might normalize fields, convert codes, etc.
            Ok(msg)
        })
    });

    // Set error handler
    orchestrator.set_error_handler(|step_name, error, _msg| {
        Box::pin(async move {
            println!("   ❌ Error in step '{}': {}", step_name, error);
        })
    });

    // Step 3: Create a router for delivery
    println!("Step 3: Setting up content-based router");
    let mut router = ContentRouter::new();

    router.add_route(
        "adt_destination",
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
                println!("   ✓ Routing to ADT destination");
                Ok(msg)
            })
        },
    );

    router.add_route(
        "oru_destination",
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
                println!("   ✓ Routing to ORU (Lab) destination");
                Ok(msg)
            })
        },
    );

    router.set_default_handler(|msg| {
        Box::pin(async move {
            println!("   ✓ Routing to default destination");
            Ok(msg)
        })
    });

    // Execute the complete workflow
    println!("\n=== Processing Messages ===\n");

    let messages = vec![
        ("Production ADT", create_production_adt()),
        ("Test ADT (will be filtered)", create_test_adt()),
        ("Production ORU", create_production_oru()),
    ];

    for (description, message) in messages {
        println!("Processing: {}", description);

        // Step 1: Filter
        match filter.filter(&message) {
            Ok(_) => {
                println!("  Filter: PASSED");

                // Step 2: Orchestrate
                match orchestrator.execute(message.clone()).await {
                    Ok(processed_msg) => {
                        println!("  Orchestration: SUCCESS");

                        // Step 3: Route
                        match router.route(processed_msg).await {
                            Ok(_) => {
                                println!("  Routing: SUCCESS");
                                println!("  ✅ Message processed successfully!\n");
                            }
                            Err(e) => {
                                println!("  Routing: FAILED - {}\n", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("  Orchestration: FAILED - {}\n", e);
                    }
                }
            }
            Err(e) => {
                println!("  Filter: REJECTED - {}", e);
                println!("  ⏭️  Message skipped\n");
            }
        }

        // Reset attempt counter for next message
        attempt_counter.store(0, Ordering::SeqCst);
    }

    println!("=== Workflow Statistics ===");
    println!("Total orchestration steps: {}", orchestrator.step_count());
    println!("Total routes configured: {}", router.route_count());
    println!("Total filters configured: {}", filter.filter_count());

    Ok(())
}

fn create_production_adt() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("HospitalSystem"));
    msh.fields.push(Field::from_value("MainHospital"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ADT^A01"));
    msh.fields.push(Field::from_value("MSG001"));
    msh.fields.push(Field::from_value("P")); // Production
    msg.segments.push(msh);
    msg
}

fn create_test_adt() -> Message {
    let mut msg = Message::default();
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("TestSystem"));
    msh.fields.push(Field::from_value("TestFacility"));
    msh.fields.push(Field::from_value("ReceivingApp"));
    msh.fields.push(Field::from_value("ReceivingFac"));
    msh.fields.push(Field::from_value("20231101120000"));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ADT^A01"));
    msh.fields.push(Field::from_value("MSG002"));
    msh.fields.push(Field::from_value("T")); // Test
    msg.segments.push(msh);
    msg
}

fn create_production_oru() -> Message {
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
    msh.fields.push(Field::from_value("MSG003"));
    msh.fields.push(Field::from_value("P")); // Production
    msg.segments.push(msh);
    msg
}
