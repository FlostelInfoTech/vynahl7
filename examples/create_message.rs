//! Example: Creating an HL7 message from scratch
//!
//! This example demonstrates how to:
//! - Build an HL7 message programmatically
//! - Set fields using the builder pattern
//! - Use the Terser API to set values
//! - Generate an ACK message

use vyna_hl7_core::{
    delimiters::Delimiters,
    field::Field,
    message::Message,
    segment::Segment,
    Version,
};
use vyna_hl7_terser::TerserMut;
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HL7 Message Builder Example ===\n");

    // Create a new message
    let mut message = Message::new();

    // Build MSH segment
    println!("Building MSH segment...");
    let mut msh = Segment::new("MSH");
    let delims = Delimiters::default();

    msh.add_field(Field::from_value(delims.field_separator.to_string())); // MSH-1
    msh.add_field(Field::from_value(delims.encoding_characters())); // MSH-2
    msh.set_field_value(3, "SendingApp")?; // MSH-3
    msh.set_field_value(4, "SendingFac")?; // MSH-4
    msh.set_field_value(5, "ReceivingApp")?; // MSH-5
    msh.set_field_value(6, "ReceivingFac")?; // MSH-6
    msh.set_field_value(7, Utc::now().format("%Y%m%d%H%M%S").to_string())?; // MSH-7
    msh.set_field_value(9, "ORU^R01")?; // MSH-9 (message type)
    msh.set_field_value(10, "MSG12345")?; // MSH-10 (message control ID)
    msh.set_field_value(11, "P")?; // MSH-11 (processing ID)
    msh.set_field_value(12, Version::V2_5.as_str())?; // MSH-12 (version)

    message.add_segment(msh);
    println!("✓ MSH segment created\n");

    // Build PID segment
    println!("Building PID segment...");
    let mut pid = Segment::new("PID");
    pid.set_field_value(1, "1")?; // Set ID
    pid.set_field_value(2, "PATIENT001")?; // Patient ID
    pid.set_field_value(3, "MRN123456^^^MRN")?; // Alternate ID
    pid.set_field_value(5, "SMITH^JANE^M")?; // Patient name
    pid.set_field_value(7, "19750515")?; // DOB
    pid.set_field_value(8, "F")?; // Gender
    pid.set_field_value(11, "456 OAK AVE^^SPRINGFIELD^IL^62701")?; // Address

    message.add_segment(pid);
    println!("✓ PID segment created\n");

    // Build OBR segment (Observation Request)
    println!("Building OBR segment...");
    let mut obr = Segment::new("OBR");
    obr.set_field_value(1, "1")?; // Set ID
    obr.set_field_value(4, "CBC^Complete Blood Count^LN")?; // Universal Service ID
    obr.set_field_value(7, Utc::now().format("%Y%m%d%H%M%S").to_string())?; // Observation Date/Time

    message.add_segment(obr);
    println!("✓ OBR segment created\n");

    // Build OBX segments (Observation Results)
    println!("Building OBX segments...");

    let mut obx1 = Segment::new("OBX");
    obx1.set_field_value(1, "1")?; // Set ID
    obx1.set_field_value(2, "NM")?; // Value type
    obx1.set_field_value(3, "WBC^White Blood Count^LN")?; // Observation ID
    obx1.set_field_value(5, "7.5")?; // Observation value
    obx1.set_field_value(6, "10*3/uL")?; // Units
    obx1.set_field_value(8, "N")?; // Abnormal flags
    message.add_segment(obx1);

    let mut obx2 = Segment::new("OBX");
    obx2.set_field_value(1, "2")?;
    obx2.set_field_value(2, "NM")?;
    obx2.set_field_value(3, "RBC^Red Blood Count^LN")?;
    obx2.set_field_value(5, "4.2")?;
    obx2.set_field_value(6, "10*6/uL")?;
    obx2.set_field_value(8, "N")?;
    message.add_segment(obx2);

    println!("✓ OBX segments created\n");

    // Display the message
    println!("--- Created Message ---");
    let encoded = message.encode();
    println!("{}\n", encoded.replace('\r', "\r\n"));

    // Use Terser to modify values
    println!("--- Using Terser to Modify Message ---");
    let mut terser = TerserMut::new(&mut message);

    terser.set("PID-5-1", "JONES")?; // Change family name
    terser.set("PID-8", "M")?; // Change gender
    terser.set("OBX(2)-5", "8.2")?; // Change second OBX value (1-based segment indexing)

    println!("✓ Values modified using Terser\n");

    println!("--- Modified Message ---");
    let modified_encoded = message.encode();
    println!("{}\n", modified_encoded.replace('\r', "\r\n"));

    // Generate an ACK message
    println!("--- Generating ACK Message ---");
    let ack = create_ack(&message, "AA", "")?;
    println!("{}\n", ack.encode().replace('\r', "\r\n"));

    println!("=== Example Complete ===");

    Ok(())
}

/// Create an ACK (acknowledgment) message
fn create_ack(
    original: &Message,
    ack_code: &str,
    error_message: &str,
) -> Result<Message, Box<dyn std::error::Error>> {
    let mut ack = Message::new();

    // Build MSH for ACK
    let mut msh = Segment::new("MSH");
    let delims = Delimiters::default();

    msh.add_field(Field::from_value(delims.field_separator.to_string()));
    msh.add_field(Field::from_value(delims.encoding_characters()));

    // Swap sender and receiver
    if let Some(recv_app) = original.get_receiving_application() {
        msh.set_field_value(3, recv_app)?;
    }
    if let Some(recv_fac) = original.get_receiving_facility() {
        msh.set_field_value(4, recv_fac)?;
    }
    if let Some(send_app) = original.get_sending_application() {
        msh.set_field_value(5, send_app)?;
    }
    if let Some(send_fac) = original.get_sending_facility() {
        msh.set_field_value(6, send_fac)?;
    }

    msh.set_field_value(7, Utc::now().format("%Y%m%d%H%M%S").to_string())?;
    msh.set_field_value(9, "ACK")?;
    msh.set_field_value(10, format!("ACK{}", Utc::now().timestamp()))?;
    msh.set_field_value(11, "P")?;
    if let Some(version) = original.get_version() {
        msh.set_field_value(12, version.as_str())?;
    }

    ack.add_segment(msh);

    // Build MSA segment
    let mut msa = Segment::new("MSA");
    msa.set_field_value(1, ack_code)?; // Acknowledgment code (AA, AE, AR)
    if let Some(control_id) = original.get_control_id() {
        msa.set_field_value(2, control_id)?; // Message control ID from original
    }

    if !error_message.is_empty() {
        msa.set_field_value(3, error_message)?;
    }

    ack.add_segment(msa);

    Ok(ack)
}
