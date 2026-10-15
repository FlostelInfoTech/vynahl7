//! Example: MLLP Server
//!
//! This example demonstrates how to create an MLLP server that:
//! - Listens for HL7 messages over TCP
//! - Parses incoming messages
//! - Sends ACK responses

use chrono::Utc;
use vyna_hl7_core::{delimiters::Delimiters, field::Field, message::Message, segment::Segment};
use vyna_hl7_mllp::MllpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MLLP Server Example ===\n");

    // Bind server to localhost
    let addr = "127.0.0.1:2575";
    println!("Starting MLLP server on {}...", addr);
    let server = MllpServer::bind(addr).await?;
    println!("✓ Server listening on {}\n", server.local_addr()?);

    println!("Waiting for connections...");
    println!("(Press Ctrl+C to stop)\n");

    // Accept connections in a loop
    loop {
        match server.accept().await {
            Ok(mut connection) => {
                println!("--- New Connection ---");

                tokio::spawn(async move {
                    loop {
                        match connection.receive_message().await {
                            Ok(message) => {
                                println!("Received message:");
                                println!("  Control ID: {:?}", message.get_control_id());
                                println!("  Message Type: {:?}", message.get_message_type());
                                println!("  Sender: {:?}", message.get_sending_application());

                                // Generate ACK
                                match create_ack(&message, "AA", "") {
                                    Ok(ack) => {
                                        if let Err(e) = connection.send_message(&ack).await {
                                            eprintln!("Failed to send ACK: {}", e);
                                            break;
                                        }
                                        println!("✓ ACK sent\n");
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to create ACK: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error receiving message: {}", e);
                                break;
                            }
                        }
                    }

                    println!("Connection closed");
                });
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }
}

/// Create an ACK (acknowledgment) message
fn create_ack(
    original: &Message,
    ack_code: &str,
    error_message: &str,
) -> Result<Message, vyna_hl7_core::error::Error> {
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

    // Set MSH-9 to ACK^{trigger_event} (e.g., ACK^A01 for ADT^A01)
    let msg_type = if let Some((_event_type, trigger)) = original.get_message_type() {
        format!("ACK^{}", trigger)
    } else {
        String::from("ACK")
    };
    msh.set_field_value(9, &msg_type)?;

    msh.set_field_value(10, format!("ACK{}", Utc::now().timestamp()))?;
    msh.set_field_value(11, "P")?;
    if let Some(version) = original.get_version() {
        msh.set_field_value(12, version.as_str())?;
    }

    ack.add_segment(msh);

    // Build MSA segment
    let mut msa = Segment::new("MSA");
    msa.set_field_value(1, ack_code)?;
    if let Some(control_id) = original.get_control_id() {
        msa.set_field_value(2, control_id)?;
    }

    if !error_message.is_empty() {
        msa.set_field_value(3, error_message)?;
    }

    ack.add_segment(msa);

    Ok(ack)
}
