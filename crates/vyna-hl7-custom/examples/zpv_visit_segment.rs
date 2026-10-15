//! Example: ZPV - Patient Visit Custom Segment
//!
//! This example demonstrates a custom Z-segment for tracking patient visit information
//! that extends the standard PV1 segment with organization-specific fields.

use vyna_hl7_custom::{CustomSegment, MessageExt, z_segment};
use vyna_hl7_parser::parse_message;

// Define the ZPV custom segment
z_segment! {
    ZPV,
    id = "ZPV",
    fields = {
        1 => visit_type: String,           // INPATIENT, OUTPATIENT, EMERGENCY
        2 => visit_number: String,          // Organization visit ID
        3 => patient_class: Option<String>, // E, I, O, etc.
        4 => department_code: Option<String>,
        5 => attending_physician: Option<String>,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a ZPV segment using the builder
    let zpv = ZPV::builder()
        .visit_type("OUTPATIENT")
        .visit_number("V202401-12345")
        .patient_class("O")
        .department_code("CARDIO")
        .attending_physician("DR^SMITH^JOHN")
        .build()?;

    println!("Created ZPV segment:");
    println!("  Visit Type: {}", zpv.visit_type);
    println!("  Visit Number: {}", zpv.visit_number);
    println!("  Patient Class: {:?}", zpv.patient_class);
    println!("  Department: {:?}", zpv.department_code);
    println!("  Attending: {:?}", zpv.attending_physician);

    // Parse a message containing a ZPV segment
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG001|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       PV1|1|O|CARDIO^101^1||||DR^SMITH^JOHN|||||||||||V202401-12345\r\
                       ZPV|OUTPATIENT|V202401-12345|O|CARDIO|DR^SMITH^JOHN";

    let message = parse_message(hl7_message)?;

    // Extract the ZPV segment from the parsed message
    if let Some(zpv) = message.get_custom_segment::<ZPV>()? {
        println!("\nExtracted ZPV from message:");
        println!("  Visit Type: {}", zpv.visit_type);
        println!("  Visit Number: {}", zpv.visit_number);

        // Convert back to segment
        let segment = zpv.to_segment();
        println!("\nEncoded segment: {}", segment.encode(&message.delimiters));
    }

    Ok(())
}
