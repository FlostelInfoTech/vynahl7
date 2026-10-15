//! Example: Creating and parsing HL7 file messages
//!
//! This example demonstrates how to:
//! - Create file messages using the FileBuilder
//! - Add multiple batches to a file
//! - Parse file messages from text
//! - Validate file structure
//! - Work with nested batch/file hierarchies

use vyna_hl7_core::{
    Version,
    builders::{
        adt::AdtBuilder,
        batch::{BatchBuilder, FileBuilder},
    },
};
use vyna_hl7_parser::parse_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HL7 File Message Example ===\n");

    // Example 1: Create a file using the builder
    println!("--- Example 1: Building a File ---");
    create_file_example()?;

    // Example 2: Parse a file from text
    println!("\n--- Example 2: Parsing a File ---");
    parse_file_example()?;

    // Example 3: File validation
    println!("\n--- Example 3: File Validation ---");
    validation_example()?;

    // Example 4: Hierarchical structure
    println!("\n--- Example 4: Hierarchical Navigation ---");
    hierarchy_example()?;

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Example 1: Create a file using the builder
fn create_file_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create messages for first batch
    let msg1 = AdtBuilder::a01(Version::V2_5)
        .patient_id("PAT001")
        .patient_name("JONES", "ALICE")
        .build()?;

    let msg2 = AdtBuilder::a01(Version::V2_5)
        .patient_id("PAT002")
        .patient_name("WILLIAMS", "BOB")
        .build()?;

    // Create first batch
    let batch1 = BatchBuilder::new()
        .sending_application("LAB")
        .control_id("B001")
        .batch_name("MORNING_ADMITS")
        .add_message(msg1)
        .add_message(msg2)
        .build()?;

    // Create messages for second batch
    let msg3 = AdtBuilder::a01(Version::V2_5)
        .patient_id("PAT003")
        .patient_name("DAVIS", "CAROL")
        .build()?;

    let msg4 = AdtBuilder::a01(Version::V2_5)
        .patient_id("PAT004")
        .patient_name("MILLER", "DAVID")
        .build()?;

    let msg5 = AdtBuilder::a01(Version::V2_5)
        .patient_id("PAT005")
        .patient_name("WILSON", "EVE")
        .build()?;

    // Create second batch
    let batch2 = BatchBuilder::new()
        .sending_application("LAB")
        .control_id("B002")
        .batch_name("AFTERNOON_ADMITS")
        .add_message(msg3)
        .add_message(msg4)
        .add_message(msg5)
        .build()?;

    // Create file containing both batches
    let file = FileBuilder::new()
        .sending_application("LAB")
        .sending_facility("HOSPITAL")
        .receiving_application("EMR")
        .receiving_facility("CLINIC")
        .control_id("F20240315001")
        .file_name("DAILY_ADMITS_20240315")
        .comment("Daily admission file")
        .add_batch(batch1)
        .add_batch(batch2)
        .trailer_comment("File transmission complete")
        .build()?;

    println!("✓ Created file with {} batches", file.batches.len());
    println!("  File ID: {}", file.header.control_id.as_ref().unwrap());
    println!(
        "  File name: {}",
        file.header.file_name_id.as_ref().unwrap()
    );
    println!("  Total messages: {}", file.total_message_count());
    println!(
        "  Batch count in trailer: {}",
        file.trailer.batch_count.unwrap()
    );

    for (i, batch) in file.batches.iter().enumerate() {
        println!("\n  Batch {}:", i + 1);
        println!(
            "    Control ID: {}",
            batch.header.control_id.as_ref().unwrap()
        );
        println!(
            "    Name: {}",
            batch
                .header
                .batch_name_id_type
                .as_ref()
                .unwrap_or(&"N/A".to_string())
        );
        println!("    Messages: {}", batch.messages.len());
    }

    Ok(())
}

/// Example 2: Parse a file from text
fn parse_file_example() -> Result<(), Box<dyn std::error::Error>> {
    let file_text = "\
FHS|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315080000||DAILY_FILE|Daily transmission|F12345
BHS|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315080000||BATCH_A|Morning batch|B001
MSH|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315080100||ADT^A01|MSG001|P|2.5
PID|1|PAT001|MRN001^^^MRN|JONES^ALICE||19850101|F
PV1|1|I|W1^R1^B1
MSH|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315080200||ADT^A01|MSG002|P|2.5
PID|1|PAT002|MRN002^^^MRN|WILLIAMS^BOB||19900202|M
PV1|1|I|W1^R2^B1
BTS|2|Morning batch complete
BHS|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315140000||BATCH_B|Afternoon batch|B002
MSH|^~\\&|LAB|HOSPITAL|EMR|CLINIC|20240315140100||ADT^A01|MSG003|P|2.5
PID|1|PAT003|MRN003^^^MRN|DAVIS^CAROL||19750315|F
PV1|1|O|C1^R1^
BTS|1|Afternoon batch complete
FTS|2|File transmission complete";

    let file = parse_file(file_text)?;

    println!("✓ Parsed file successfully");
    println!(
        "  File control ID: {}",
        file.header.control_id.as_ref().unwrap()
    );
    println!(
        "  File name: {}",
        file.header.file_name_id.as_ref().unwrap()
    );
    println!("  Number of batches: {}", file.batches.len());
    println!("  Total messages: {}", file.total_message_count());
    println!(
        "  Trailer comment: {}",
        file.trailer.comment.as_ref().unwrap()
    );

    for (i, batch) in file.batches.iter().enumerate() {
        println!("\n  Batch {}:", i + 1);
        println!(
            "    Control ID: {}",
            batch.header.control_id.as_ref().unwrap()
        );
        println!("    Messages: {}", batch.messages.len());
        println!("    Comment: {}", batch.trailer.comment.as_ref().unwrap());
    }

    Ok(())
}

/// Example 3: File validation
fn validation_example() -> Result<(), Box<dyn std::error::Error>> {
    // Valid file
    let valid_file = "\
FHS|^~\\&|APP|FAC|||20240315||FILE||F001
BHS|^~\\&|APP|FAC|||20240315||BATCH||B001
MSH|^~\\&|APP|FAC|SYS|HOSP|20240315||ADT^A01|MSG001|P|2.5
PID|1|12345
BTS|1
BHS|^~\\&|APP|FAC|||20240315||BATCH||B002
MSH|^~\\&|APP|FAC|SYS|HOSP|20240315||ADT^A01|MSG002|P|2.5
PID|1|67890
BTS|1
FTS|2";

    match parse_file(valid_file) {
        Ok(file) => {
            println!("✓ Valid file passed validation");
            println!("  Batches: {}", file.batches.len());
            println!("  Total messages: {}", file.total_message_count());
        }
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    // Invalid file - batch count mismatch
    let invalid_file = "\
FHS|^~\\&|APP|FAC|||20240315||FILE||F002
BHS|^~\\&|APP|FAC|||20240315||BATCH||B001
MSH|^~\\&|APP|FAC|SYS|HOSP|20240315||ADT^A01|MSG001|P|2.5
PID|1|12345
BTS|1
FTS|10"; // Says 10 batches but only 1 present

    match parse_file(invalid_file) {
        Ok(_) => println!("✗ Invalid file should have failed!"),
        Err(e) => {
            println!("✓ Invalid file correctly rejected");
            println!("  Error: {}", e);
        }
    }

    Ok(())
}

/// Example 4: Hierarchical navigation
fn hierarchy_example() -> Result<(), Box<dyn std::error::Error>> {
    // Create a complex file structure
    let mut batches = Vec::new();

    for batch_num in 1..=3 {
        let mut messages = Vec::new();

        for msg_num in 1..=2 {
            let patient_id = format!("PAT{:03}", batch_num * 10 + msg_num);
            let msg = AdtBuilder::a01(Version::V2_5)
                .patient_id(&patient_id)
                .patient_name("PATIENT", &patient_id)
                .build()?;
            messages.push(msg);
        }

        let batch = BatchBuilder::new()
            .control_id(&format!("B{:03}", batch_num))
            .add_messages(messages)
            .build()?;
        batches.push(batch);
    }

    let file = FileBuilder::new()
        .control_id("F001")
        .file_name("HIERARCHICAL_EXAMPLE")
        .add_batches(batches)
        .build()?;

    println!("✓ Created hierarchical file structure:");
    println!("  File: {}", file.header.control_id.as_ref().unwrap());
    println!("  Total batches: {}", file.batches.len());
    println!("  Total messages: {}", file.total_message_count());

    // Navigate the hierarchy
    for (batch_idx, batch) in file.batches.iter().enumerate() {
        println!(
            "\n  Batch {} [{}]:",
            batch_idx + 1,
            batch.header.control_id.as_ref().unwrap()
        );

        for (msg_idx, message) in batch.messages.iter().enumerate() {
            let control_id = message.get_control_id().unwrap_or("N/A");
            let msg_type = if let Some((msg_type, trigger)) = message.get_message_type() {
                format!("{}^{}", msg_type, trigger)
            } else {
                "Unknown".to_string()
            };

            println!(
                "    Message {} - Type: {}, Control ID: {}",
                msg_idx + 1,
                msg_type,
                control_id
            );
        }
    }

    // Encode and verify round-trip
    let encoded = file.encode_with_separator("\r");
    let parsed = parse_file(&encoded)?;

    println!("\n✓ Round-trip verification:");
    println!(
        "  Original batches: {}, Parsed batches: {}",
        file.batches.len(),
        parsed.batches.len()
    );
    println!(
        "  Original messages: {}, Parsed messages: {}",
        file.total_message_count(),
        parsed.total_message_count()
    );

    Ok(())
}
