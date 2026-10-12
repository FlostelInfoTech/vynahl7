//! Example: MLLP Client
//!
//! This example demonstrates how to:
//! - Create an HL7 message
//! - Connect to an MLLP server
//! - Send the message and receive an ACK

use vyna_hl7_core::{delimiters::Delimiters, field::Field, message::Message, segment::Segment, Version};
use vyna_hl7_mllp::MllpClient;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MLLP Client Example ===\n");

    // Create a sample HL7 message
    let message = create_sample_message()?;

    println!("--- Message to Send ---");
    let encoded = message.encode();
    println!("{}\n", encoded.replace('\r', "\r\n"));

    // Connect to MLLP server
    let addr = "127.0.0.1:2575";
    println!("Connecting to MLLP server at {}...", addr);

    match MllpClient::connect(addr).await {
        Ok(mut client) => {
            println!("✓ Connected to server\n");

            // Send message and wait for ACK
            println!("Sending message...");
            match client.send_message(&message).await {
                Ok(ack) => {
                    println!("✓ Message sent successfully\n");

                    println!("--- Received ACK ---");
                    println!("{}\n", ack.encode().replace('\r', "\r\n"));

                    // Parse ACK details
                    if let Some(msa) = ack.get_segments_by_id("MSA").first() {
                        if let Some(ack_code) = msa.get_field_value(1) {
                            println!("Acknowledgment Code: {}", ack_code);
                            match ack_code {
                                "AA" => println!("Status: Application Accept (Success)"),
                                "AE" => println!("Status: Application Error"),
                                "AR" => println!("Status: Application Reject"),
                                _ => println!("Status: Unknown"),
                            }
                        }

                        if let Some(msg_id) = msa.get_field_value(2) {
                            println!("Message Control ID: {}", msg_id);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Failed to send message: {}", e);
                }
            }

            // Close connection
            client.close().await?;
            println!("\nConnection closed");
        }
        Err(e) => {
            eprintln!("✗ Failed to connect: {}", e);
            eprintln!("\nMake sure the MLLP server is running!");
            eprintln!("Start it with: cargo run --example mllp_server");
        }
    }

    println!("\n=== Example Complete ===");

    Ok(())
}

/// Create a sample ADT^A01 message
fn create_sample_message() -> Result<Message, Box<dyn std::error::Error>> {
    let mut message = Message::new();

    // Build MSH segment
    let mut msh = Segment::new("MSH");
    let delims = Delimiters::default();

    msh.add_field(Field::from_value(delims.field_separator.to_string()));
    msh.add_field(Field::from_value(delims.encoding_characters()));
    msh.set_field_value(3, "ClientApp")?;
    msh.set_field_value(4, "ClientFacility")?;
    msh.set_field_value(5, "ServerApp")?;
    msh.set_field_value(6, "ServerFacility")?;
    msh.set_field_value(7, Utc::now().format("%Y%m%d%H%M%S").to_string())?;
    msh.set_field_value(9, "ADT^A01")?;
    msh.set_field_value(10, format!("MSG{}", Utc::now().timestamp()))?;
    msh.set_field_value(11, "P")?;
    msh.set_field_value(12, Version::V2_5.as_str())?;

    message.add_segment(msh);

    // Build PID segment
    let mut pid = Segment::new("PID");
    pid.set_field_value(1, "1")?;
    pid.set_field_value(2, "PATIENT12345")?;
    pid.set_field_value(3, "MRN987654^^^MRN")?;
    pid.set_field_value(5, "DOE^JOHN^M")?;
    pid.set_field_value(7, "19850714")?;
    pid.set_field_value(8, "M")?;

    message.add_segment(pid);

    // Build PV1 segment
    let mut pv1 = Segment::new("PV1");
    pv1.set_field_value(1, "1")?;
    pv1.set_field_value(2, "I")?; // Inpatient
    pv1.set_field_value(3, "ICU^201^A")?;

    message.add_segment(pv1);

    Ok(message)
}
