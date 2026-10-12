//! Example: Creating and parsing HL7 batch messages
//!
//! This example demonstrates how to:
//! - Create batch messages using the BatchBuilder
//! - Add multiple messages to a batch
//! - Parse batch messages from text
//! - Validate batch structure
//! - Encode batches for transmission

use vyna_hl7_core::{
    builders::{
        adt::AdtBuilder,
        batch::BatchBuilder,
    },
    Version,
};
use vyna_hl7_parser::parse_batch;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HL7 Batch Message Example ===\n");

    // Example 1: Create a batch using the builder
    println!("--- Example 1: Building a Batch ---");
    create_batch_example()?;

    // Example 2: Parse a batch from text
    println!("\n--- Example 2: Parsing a Batch ---");
    parse_batch_example()?;

    // Example 3: Batch validation
    println!("\n--- Example 3: Batch Validation ---");
    validation_example()?;

    // Example 4: Encoding batches
    println!("\n--- Example 4: Encoding Batches ---");
    encoding_example()?;

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Example 1: Create a batch using the builder
fn create_batch_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create some ADT messages
    let msg1 = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800101")
        .sex("M")
        .build()?;

    let msg2 = AdtBuilder::a01(Version::V2_5)
        .patient_id("67890")
        .patient_name("SMITH", "JANE")
        .date_of_birth("19900202")
        .sex("F")
        .build()?;

    let msg3 = AdtBuilder::a01(Version::V2_5)
        .patient_id("11111")
        .patient_name("BROWN", "BOB")
        .date_of_birth("19850315")
        .sex("M")
        .build()?;

    // Build a batch containing these messages
    let batch = BatchBuilder::new()
        .sending_application("LAB")
        .sending_facility("HOSPITAL")
        .receiving_application("EMR")
        .receiving_facility("CLINIC")
        .control_id("B20240315001")
        .batch_name("ADMIT_BATCH")
        .comment("Daily admission batch")
        .add_message(msg1)
        .add_message(msg2)
        .add_message(msg3)
        .build()?;

    println!("✓ Created batch with {} messages", batch.messages.len());
    println!("  Batch ID: {}", batch.header.control_id.as_ref().unwrap());
    println!("  Sending: {}", batch.header.sending_application.as_ref().unwrap());
    println!("  Receiving: {}", batch.header.receiving_application.as_ref().unwrap());
    println!("  Message count in trailer: {}", batch.trailer.message_count.unwrap());

    Ok(())
}

/// Example 2: Parse a batch from text
fn parse_batch_example() -> Result<(), Box<dyn std::error::Error>> {
    let batch_text = "\
BHS|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315143000||BATCH001|Daily batch|B12345
MSH|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315143000||ADT^A01|MSG001|P|2.5
PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M
PV1|1|I|Ward^Room^Bed
MSH|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315143100||ADT^A01|MSG002|P|2.5
PID|1|54321|09876^^^MRN|SMITH^JANE^B||19900202|F
PV1|1|O|Clinic^Room1^
BTS|2|Batch complete";

    let batch = parse_batch(batch_text)?;

    println!("✓ Parsed batch successfully");
    println!("  Batch control ID: {}", batch.header.control_id.as_ref().unwrap());
    println!("  Batch name: {}", batch.header.batch_name_id_type.as_ref().unwrap());
    println!("  Number of messages: {}", batch.messages.len());
    println!("  Trailer comment: {}", batch.trailer.comment.as_ref().unwrap());

    // Access individual messages
    for (i, msg) in batch.messages.iter().enumerate() {
        println!("\n  Message {}:", i + 1);
        println!("    Control ID: {}", msg.get_control_id().unwrap());
        if let Some((msg_type, trigger)) = msg.get_message_type() {
            println!("    Message type: {}^{}", msg_type, trigger);
        }
        println!("    Segments: {}", msg.segments.len());
    }

    Ok(())
}

/// Example 3: Batch validation
fn validation_example() -> Result<(), Box<dyn std::error::Error>> {
    // Valid batch
    let valid_batch = "\
BHS|^~\\&|APP|FAC|||20240315||BATCH||B001
MSH|^~\\&|APP|FAC|SYS|HOSP|20240315||ADT^A01|MSG001|P|2.5
PID|1|12345
MSH|^~\\&|APP|FAC|SYS|HOSP|20240315||ADT^A01|MSG002|P|2.5
PID|1|67890
BTS|2";

    match parse_batch(valid_batch) {
        Ok(batch) => {
            println!("✓ Valid batch passed validation");
            println!("  Messages: {}", batch.messages.len());
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    // Invalid batch - message count mismatch
    let invalid_batch = "\
BHS|^~\\&|APP|FAC|||20240315||BATCH||B002
MSH|^~\\&|APP|FAC|SYS|HOSP|20240315||ADT^A01|MSG001|P|2.5
PID|1|12345
BTS|5";  // Says 5 messages but only 1 present

    match parse_batch(invalid_batch) {
        Ok(_) => println!("✗ Invalid batch should have failed!"),
        Err(e) => {
            println!("✓ Invalid batch correctly rejected");
            println!("  Error: {}", e);
        }
    }

    Ok(())
}

/// Example 4: Encoding batches
fn encoding_example() -> Result<(), Box<dyn std::error::Error>> {
    let msg = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .patient_name("TEST", "PATIENT")
        .build()?;

    let batch = BatchBuilder::new()
        .sending_application("SYS1")
        .receiving_application("SYS2")
        .control_id("B999")
        .add_message(msg)
        .trailer_comment("Test batch complete")
        .build()?;

    // Encode with newline separator for display
    let encoded = batch.encode_with_separator("\n");
    println!("✓ Encoded batch:\n");
    println!("{}", encoded);

    // Encode with standard \r separator for transmission
    let transmission_format = batch.encode_with_separator("\r");
    println!("\n✓ Transmission format length: {} bytes", transmission_format.len());

    // Parse the encoded batch to verify round-trip
    let parsed = parse_batch(&transmission_format)?;
    println!("✓ Round-trip successful: {} messages", parsed.messages.len());

    Ok(())
}
