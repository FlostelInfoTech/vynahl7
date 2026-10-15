//! MLLP TLS Client Example
//!
//! This example demonstrates how to connect to an MLLP server with TLS/mTLS support.
//!
//! ## Generating Test Certificates
//!
//! See the mllp_tls_server.rs example for certificate generation instructions.
//!
//! ## Running the Example
//!
//! Make sure the TLS server is running first:
//! ```bash
//! cargo run --example mllp_tls_server --features tls
//! ```
//!
//! Then run the client:
//! ```bash
//! # Basic TLS
//! cargo run --example mllp_tls_client --features tls
//!
//! # Or with mTLS (client certificate)
//! cargo run --example mllp_tls_client --features tls -- --mtls
//! ```

use std::env;
use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_mllp::{MllpClient, tls::TlsClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MLLP TLS Client Example ===\n");

    // Check for certificate files
    let use_mtls = env::args().any(|arg| arg == "--mtls");
    let ca_cert = env::var("CA_CERT").unwrap_or_else(|_| "ca-cert.pem".to_string());
    let client_cert = env::var("CLIENT_CERT").unwrap_or_else(|_| "client-cert.pem".to_string());
    let client_key = env::var("CLIENT_KEY").unwrap_or_else(|_| "client-key.pem".to_string());

    // Create TLS configuration
    let tls_config = if use_mtls {
        println!("Connecting with mTLS (client certificate authentication)...");
        println!("CA cert: {}", ca_cert);
        println!("Client cert: {}", client_cert);
        println!("Client key: {}\n", client_key);
        TlsClientConfig::with_mtls(&ca_cert, &client_cert, &client_key)?
    } else {
        println!("Connecting with TLS...");
        println!("CA cert: {}\n", ca_cert);
        TlsClientConfig::with_ca_cert(&ca_cert)?
    };

    // Connect to MLLP TLS server
    println!("Connecting to MLLP TLS server at 127.0.0.1:2575...");
    let mut client = MllpClient::connect_tls(
        "127.0.0.1:2575",
        "localhost", // Server name for SNI
        tls_config,
    )
    .await?;
    println!("✓ Connected (TLS handshake successful)\n");

    // Create a test HL7 message
    let message = create_test_message();
    println!("📤 Sending HL7 message:");
    println!("{}\n", message.encode());

    // Send message and receive ACK
    println!("Waiting for ACK...");
    let ack = client.send_message(&message).await?;
    println!("✓ Received ACK:");
    println!("{}\n", ack.encode());

    // Close connection
    client.close().await?;
    println!("✓ Connection closed");

    Ok(())
}

/// Create a test ADT^A01 message
fn create_test_message() -> Message {
    let mut msg = Message::default();

    // MSH segment
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));
    msh.fields.push(Field::from_value("SENDING_APP"));
    msh.fields.push(Field::from_value("SENDING_FAC"));
    msh.fields.push(Field::from_value("RECEIVING_APP"));
    msh.fields.push(Field::from_value("RECEIVING_FAC"));
    msh.fields.push(Field::from_value(format!(
        "{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    )));
    msh.fields.push(Field::from_value(""));
    msh.fields.push(Field::from_value("ADT^A01"));
    msh.fields.push(Field::from_value("MSG001"));
    msh.fields.push(Field::from_value("P"));
    msh.fields.push(Field::from_value("2.5"));
    msg.segments.push(msh);

    // EVN segment
    let mut evn = Segment::new("EVN");
    evn.fields.push(Field::from_value("A01"));
    evn.fields.push(Field::from_value(format!(
        "{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    )));
    msg.segments.push(evn);

    // PID segment
    let mut pid = Segment::new("PID");
    pid.fields.push(Field::from_value(""));
    pid.fields.push(Field::from_value("12345"));
    pid.fields.push(Field::from_value("67890^^^MRN"));
    pid.fields.push(Field::from_value(""));
    pid.fields.push(Field::from_value("DOE^JOHN^A"));
    pid.fields.push(Field::from_value(""));
    pid.fields.push(Field::from_value("19800101"));
    pid.fields.push(Field::from_value("M"));
    msg.segments.push(pid);

    msg
}
