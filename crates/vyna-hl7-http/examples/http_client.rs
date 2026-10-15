//! HTTP Client Example
//!
//! This example demonstrates how to use the HTTP client to send HL7 v2.x messages
//! to an HTTP server and receive ACK responses.
//!
//! # Prerequisites
//!
//! Before running this example, start the HTTP server:
//!
//! ```bash
//! cargo run --example http_server
//! ```
//!
//! # Running the Client
//!
//! In a separate terminal:
//!
//! ```bash
//! cargo run --example http_client
//! ```

use std::time::Duration;
use vyna_hl7_core::Version;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_http::HttpClient;
use vyna_hl7_terser::Terser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("HL7 HTTP Client Example\n");

    // Create HTTP client
    let client = HttpClient::new("http://127.0.0.1:8080")?.with_timeout(Duration::from_secs(30))?;
    // Optionally enable authentication if server requires it:
    // .with_auth("username".into(), "password".into());

    println!("Connected to: http://127.0.0.1:8080\n");

    // Example 1: Send ADT^A01 (Patient Admission)
    println!("=== Example 1: ADT^A01 (Patient Admission) ===");
    send_adt_a01(&client).await?;

    // Wait a bit between messages
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Example 2: Send ADT^A08 (Patient Update)
    println!("\n=== Example 2: ADT^A08 (Patient Update) ===");
    send_adt_a08(&client).await?;

    // Wait a bit between messages
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Example 3: Send ADT^A03 (Patient Discharge)
    println!("\n=== Example 3: ADT^A03 (Patient Discharge) ===");
    send_adt_a03(&client).await?;

    println!("\n✓ All messages sent successfully!");

    Ok(())
}

/// Send an ADT^A01 (Patient Admission) message
async fn send_adt_a01(client: &HttpClient) -> Result<(), Box<dyn std::error::Error>> {
    let message = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800115")
        .sex("M")
        .build()?;

    println!("Sending ADT^A01 message...");
    println!("Patient: JOHN DOE (ID: 12345)");
    println!(
        "Message Control ID: {}",
        message.get_control_id().unwrap_or("N/A")
    );

    let ack = client.send_message(&message).await?;
    let terser = Terser::new(&ack);

    println!("✓ ACK received: {}", ack.get_control_id().unwrap_or("N/A"));
    println!("  Acknowledgment Code: {:?}", terser.get("MSA-1"));

    Ok(())
}

/// Send an ADT^A08 (Patient Update) message
async fn send_adt_a08(client: &HttpClient) -> Result<(), Box<dyn std::error::Error>> {
    let message = AdtBuilder::a08(Version::V2_5)
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800115")
        .sex("M")
        .build()?;

    println!("Sending ADT^A08 message...");
    println!("Patient: JOHN DOE (ID: 12345) - UPDATE");
    println!(
        "Message Control ID: {}",
        message.get_control_id().unwrap_or("N/A")
    );

    let ack = client.send_message(&message).await?;
    let terser = Terser::new(&ack);

    println!("✓ ACK received: {}", ack.get_control_id().unwrap_or("N/A"));
    println!("  Acknowledgment Code: {:?}", terser.get("MSA-1"));

    Ok(())
}

/// Send an ADT^A03 (Patient Discharge) message
async fn send_adt_a03(client: &HttpClient) -> Result<(), Box<dyn std::error::Error>> {
    let message = AdtBuilder::a03(Version::V2_5)
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    println!("Sending ADT^A03 message...");
    println!("Patient: JOHN DOE (ID: 12345) - DISCHARGE");
    println!(
        "Message Control ID: {}",
        message.get_control_id().unwrap_or("N/A")
    );

    let ack = client.send_message(&message).await?;
    let terser = Terser::new(&ack);

    println!("✓ ACK received: {}", ack.get_control_id().unwrap_or("N/A"));
    println!("  Acknowledgment Code: {:?}", terser.get("MSA-1"));

    Ok(())
}
