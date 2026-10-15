//! HTTP Server Example
//!
//! This example demonstrates how to set up an HTTP server that receives HL7 v2.x messages
//! and returns ACK responses.
//!
//! The server listens on http://127.0.0.1:8080 and accepts POST requests with HL7 messages.
//!
//! # Running the Server
//!
//! ```bash
//! cargo run --example http_server
//! ```
//!
//! # Testing with curl
//!
//! ```bash
//! curl -X POST http://127.0.0.1:8080 \
//!   -H "Content-Type: x-application/hl7-v2+er7" \
//!   -d "MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250126120000||ADT^A01|MSG00001|P|2.5"
//! ```

use chrono::Utc;
use std::sync::Arc;
use vyna_hl7_core::{delimiters::Delimiters, field::Field, message::Message, segment::Segment};
use vyna_hl7_http::{HttpServer, MessageHandler};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting HL7 HTTP Server...");
    println!("Listening on http://127.0.0.1:8080");
    println!("\nSend HL7 messages via HTTP POST with Content-Type: x-application/hl7-v2+er7");
    println!("\nPress Ctrl+C to stop the server\n");

    // Create a message handler that processes incoming messages
    let handler: MessageHandler = Arc::new(|message: Message| {
        // Log the received message
        println!("=== Received HL7 Message ===");
        println!("Message Type: {:?}", message.get_message_type());
        println!("Control ID: {:?}", message.get_control_id());
        println!("Version: {:?}", message.get_version());
        println!(
            "Sending Application: {:?}",
            message.get_sending_application()
        );
        println!("Sending Facility: {:?}", message.get_sending_facility());
        println!("===========================\n");

        // Create an ACK response
        let ack = create_ack(&message, "AA", "")?;
        println!("Sending ACK: {}\n", ack.get_control_id().unwrap_or("N/A"));

        Ok(ack)
    });

    // Create and configure the HTTP server
    let server = HttpServer::new().with_handler(handler);
    // Optionally enable authentication:
    // .with_auth("username".into(), "password".into());

    // Start the server
    server.serve("127.0.0.1:8080").await?;

    Ok(())
}

/// Create an ACK (acknowledgment) message
fn create_ack(
    original: &Message,
    ack_code: &str,
    error_message: &str,
) -> Result<Message, vyna_hl7_core::Error> {
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
