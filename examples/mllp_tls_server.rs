//! MLLP TLS Server Example
//!
//! This example demonstrates how to create an MLLP server with TLS/mTLS support.
//!
//! ## Generating Test Certificates
//!
//! Before running this example, you need to generate test certificates:
//!
//! ```bash
//! # Generate CA certificate
//! openssl genrsa -out ca-key.pem 4096
//! openssl req -new -x509 -days 365 -key ca-key.pem -out ca-cert.pem \
//!     -subj "/CN=Test CA"
//!
//! # Generate server certificate
//! openssl genrsa -out server-key.pem 4096
//! openssl req -new -key server-key.pem -out server.csr \
//!     -subj "/CN=localhost"
//! openssl x509 -req -days 365 -in server.csr -CA ca-cert.pem \
//!     -CAkey ca-key.pem -CAcreateserial -out server-cert.pem
//!
//! # For mTLS, generate client certificate (optional)
//! openssl genrsa -out client-key.pem 4096
//! openssl req -new -key client-key.pem -out client.csr \
//!     -subj "/CN=client"
//! openssl x509 -req -days 365 -in client.csr -CA ca-cert.pem \
//!     -CAkey ca-key.pem -CAcreateserial -out client-cert.pem
//! ```
//!
//! ## Running the Example
//!
//! Terminal 1 (Server with basic TLS):
//! ```bash
//! cargo run --example mllp_tls_server --features tls
//! ```
//!
//! Terminal 2 (Client):
//! ```bash
//! cargo run --example mllp_tls_client --features tls
//! ```

use vyna_hl7_core::{Field, Message, Segment};
use vyna_hl7_mllp::{MllpServer, tls::TlsServerConfig};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MLLP TLS Server Example ===\n");

    // Check for certificate files
    let use_mtls = env::args().any(|arg| arg == "--mtls");
    let server_cert = env::var("SERVER_CERT").unwrap_or_else(|_| "server-cert.pem".to_string());
    let server_key = env::var("SERVER_KEY").unwrap_or_else(|_| "server-key.pem".to_string());
    let ca_cert = env::var("CA_CERT").unwrap_or_else(|_| "ca-cert.pem".to_string());

    // Create TLS configuration
    let tls_config = if use_mtls {
        println!("Starting MLLP server with mTLS (client certificate verification)...");
        println!("Server cert: {}", server_cert);
        println!("Server key: {}", server_key);
        println!("CA cert: {}\n", ca_cert);
        TlsServerConfig::with_mtls(&server_cert, &server_key, &ca_cert)?
    } else {
        println!("Starting MLLP server with TLS...");
        println!("Server cert: {}", server_cert);
        println!("Server key: {}\n", server_key);
        TlsServerConfig::new(&server_cert, &server_key)?
    };

    // Start MLLP TLS server
    let server = MllpServer::bind_tls("127.0.0.1:2575", tls_config).await?;
    let addr = server.local_addr()?;
    println!("MLLP TLS server listening on {}", addr);
    println!("Waiting for connections...\n");

    loop {
        // Accept connection (TLS handshake happens here)
        let mut conn = server.accept().await?;
        println!("✓ Client connected (TLS handshake successful)");

        // Spawn a task to handle this connection
        tokio::spawn(async move {
            loop {
                // Receive message
                match conn.receive_message().await {
                    Ok(message) => {
                        println!("\n📨 Received HL7 message:");
                        println!("{}", message.encode());

                        // Create ACK
                        let ack = create_ack(&message);
                        println!("\n📤 Sending ACK:");
                        println!("{}", ack.encode());

                        // Send ACK
                        if let Err(e) = conn.send_message(&ack).await {
                            eprintln!("❌ Failed to send ACK: {}", e);
                            break;
                        }

                        println!("✓ ACK sent successfully\n");
                    }
                    Err(e) => {
                        eprintln!("❌ Error receiving message: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

/// Create an ACK message from the received message
fn create_ack(msg: &Message) -> Message {
    let mut ack = Message::default();

    // Create MSH segment
    let mut msh = Segment::new("MSH");
    msh.fields.push(Field::from_value("|"));
    msh.fields.push(Field::from_value("^~\\&"));

    // Swap sending/receiving systems
    if let Some(receiving_app) = msg.segments.first().and_then(|s| s.fields.get(4)) {
        msh.fields.push(receiving_app.clone());
    } else {
        msh.fields.push(Field::from_value("RECEIVING_APP"));
    }

    if let Some(receiving_fac) = msg.segments.first().and_then(|s| s.fields.get(5)) {
        msh.fields.push(receiving_fac.clone());
    } else {
        msh.fields.push(Field::from_value("RECEIVING_FAC"));
    }

    if let Some(sending_app) = msg.segments.first().and_then(|s| s.fields.get(2)) {
        msh.fields.push(sending_app.clone());
    } else {
        msh.fields.push(Field::from_value("SENDING_APP"));
    }

    if let Some(sending_fac) = msg.segments.first().and_then(|s| s.fields.get(3)) {
        msh.fields.push(sending_fac.clone());
    } else {
        msh.fields.push(Field::from_value("SENDING_FAC"));
    }

    // Timestamp
    msh.fields.push(Field::from_value(&format!("{}", chrono::Utc::now().format("%Y%m%d%H%M%S"))));

    // Security
    msh.fields.push(Field::from_value(""));

    // Message type: ACK
    msh.fields.push(Field::from_value("ACK"));

    // Message control ID (copy from original)
    if let Some(msg_ctrl_id) = msg.segments.first().and_then(|s| s.fields.get(9)) {
        msh.fields.push(msg_ctrl_id.clone());
    } else {
        msh.fields.push(Field::from_value("MSG001"));
    }

    // Processing ID
    msh.fields.push(Field::from_value("P"));

    // Version
    msh.fields.push(Field::from_value("2.5"));

    ack.segments.push(msh);

    // Create MSA segment
    let mut msa = Segment::new("MSA");
    msa.fields.push(Field::from_value("AA")); // Acknowledgment code: Application Accept
    if let Some(msg_ctrl_id) = msg.segments.first().and_then(|s| s.fields.get(9)) {
        msa.fields.push(msg_ctrl_id.clone());
    } else {
        msa.fields.push(Field::from_value("MSG001"));
    }

    ack.segments.push(msa);

    ack
}
