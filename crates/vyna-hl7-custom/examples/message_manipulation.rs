//! Example: Message Manipulation with Custom Z-Segments
//!
//! This example demonstrates how to:
//! - Extract multiple custom segments from a message
//! - Add custom segments to a message
//! - Replace existing custom segments
//! - Remove custom segments

use vyna_hl7_custom::{CustomSegment, MessageExt, z_segment};
use vyna_hl7_parser::parse_message;

// Define ZLO - Location Extension segment
z_segment! {
    ZLO,
    id = "ZLO",
    fields = {
        1 => building_code: String,
        2 => floor_number: u32,
        3 => wing: Option<String>,
        4 => room_type: Option<String>, // PRIVATE, SEMI, WARD
    }
}

// Define ZNO - Notes segment
z_segment! {
    ZNO,
    id = "ZNO",
    fields = {
        1 => note_type: String,          // CLINICAL, ADMIN, BILLING
        2 => note_text: String,
        3 => author: String,
        4 => timestamp: Option<String>,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Parsing Message with Multiple Z-Segments ===\n");

    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG003|P|2.5\r\
                       EVN|A01|20240315143000\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       PV1|1|I|NORTH^3^301||||DR^JONES^MARY\r\
                       ZLO|NORTH|3|WEST|PRIVATE\r\
                       ZNO|CLINICAL|Patient admitted for observation|DR^JONES^MARY|202403151430\r\
                       ZNO|ADMIN|Insurance verification pending|ADMIN^CLERK|202403151445";

    let mut message = parse_message(hl7_message)?;

    // Extract the location segment
    if let Some(zlo) = message.get_custom_segment::<ZLO>()? {
        println!("Location Information:");
        println!("  Building: {}", zlo.building_code);
        println!("  Floor: {}", zlo.floor_number);
        println!("  Wing: {}", zlo.wing.as_deref().unwrap_or("N/A"));
        println!("  Room Type: {}", zlo.room_type.as_deref().unwrap_or("N/A"));
    }

    // Extract all notes
    let notes = message.get_custom_segments::<ZNO>()?;
    println!("\nNotes ({} total):", notes.len());
    for (i, note) in notes.iter().enumerate() {
        println!("  Note {}:", i + 1);
        println!("    Type: {}", note.note_type);
        println!("    Text: {}", note.note_text);
        println!("    Author: {}", note.author);
    }

    println!("\n=== Adding New Z-Segment ===\n");

    // Add a new note
    let new_note = ZNO::builder()
        .note_type("BILLING")
        .note_text("Patient has verified insurance coverage")
        .author("BILLING^DEPT")
        .timestamp("202403151500")
        .build()?;

    message.add_custom_segment(new_note);

    let updated_notes = message.get_custom_segments::<ZNO>()?;
    println!("Notes after adding new note: {} total", updated_notes.len());

    println!("\n=== Replacing Z-Segment ===\n");

    // Replace the location segment
    let new_location = ZLO::builder()
        .building_code("SOUTH")
        .floor_number(5u32)
        .wing("EAST")
        .room_type("SEMI")
        .build()?;

    message.set_custom_segment(new_location)?;

    if let Some(zlo) = message.get_custom_segment::<ZLO>()? {
        println!("Updated Location:");
        println!("  Building: {}", zlo.building_code);
        println!("  Floor: {}", zlo.floor_number);
        println!("  Wing: {}", zlo.wing.as_deref().unwrap_or("N/A"));
    }

    println!("\n=== Removing Z-Segments ===\n");

    // Check current state
    println!("Before removal:");
    println!("  Has ZLO: {}", message.has_custom_segment::<ZLO>());
    println!(
        "  ZNO count: {}",
        message.get_custom_segments::<ZNO>()?.len()
    );

    // Remove all notes
    let removed = message.remove_custom_segments::<ZNO>();
    println!("\nRemoved {} ZNO segments", removed);

    println!("\nAfter removal:");
    println!("  Has ZLO: {}", message.has_custom_segment::<ZLO>());
    println!("  Has ZNO: {}", message.has_custom_segment::<ZNO>());

    println!("\n=== Final Message ===\n");
    println!("{}", message.encode());

    Ok(())
}
