//! Advanced HTTP Client Example
//!
//! This example demonstrates advanced client features including:
//! - TLS/mTLS for secure communication
//! - HTTP/2 support
//! - Retry logic with exponential backoff
//! - Authentication (Basic Auth)
//! - Message compression
//! - WebSocket client
//!
//! # Running the Example
//!
//! ## Send a single message with retry
//! ```bash
//! cargo run --example advanced_client --features retry -- send
//! ```
//!
//! ## Connect via WebSocket
//! ```bash
//! cargo run --example advanced_client --features websocket -- websocket
//! ```
//!
//! ## With TLS
//! ```bash
//! cargo run --example advanced_client --features tls,retry -- send --url https://localhost:8443/hl7
//! ```
//!
//! ## Environment Variables
//! - `HL7_SERVER_URL`: Server URL (default: http://localhost:8080)
//! - `AUTH_USERNAME`: Basic auth username
//! - `AUTH_PASSWORD`: Basic auth password
//! - `TLS_CA_CERT`: CA certificate path for TLS
//! - `TLS_CLIENT_CERT`: Client certificate path for mTLS
//! - `TLS_CLIENT_KEY`: Client key path for mTLS

use vyna_hl7_core::{builders::adt::AdtBuilder, Version};
use vyna_hl7_http::{HttpClient, Result};

#[cfg(feature = "retry")]
use vyna_hl7_http::retry::{RetryExecutor, RetryPolicy};

#[cfg(feature = "websocket")]
use vyna_hl7_http::websocket::WebSocketClient;

#[cfg(feature = "tls")]
use vyna_hl7_http::tls::TlsClientConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("send");

    match command {
        "send" => send_with_retry().await,
        "websocket" => {
            #[cfg(feature = "websocket")]
            {
                connect_websocket().await
            }
            #[cfg(not(feature = "websocket"))]
            {
                eprintln!("WebSocket feature not enabled. Rebuild with --features websocket");
                Ok(())
            }
        }
        "batch" => send_batch().await,
        "http2" => {
            #[cfg(feature = "tls")]
            {
                send_http2().await
            }
            #[cfg(not(feature = "tls"))]
            {
                eprintln!("HTTP/2 requires TLS feature. Rebuild with --features tls");
                Ok(())
            }
        }
        _ => {
            eprintln!("Usage: advanced_client [send|websocket|batch|http2]");
            Ok(())
        }
    }
}

/// Send a message with retry logic
async fn send_with_retry() -> Result<()> {
    tracing::info!("🚀 Advanced HTTP Client - Send with Retry");

    // Get server URL
    let url = std::env::var("HL7_SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // Create client
    #[cfg_attr(not(any(feature = "auth", feature = "tls")), allow(unused_mut))]
    let mut client = HttpClient::new(&url)?;

    // Configure authentication
    #[cfg(feature = "auth")]
    {
        if let (Ok(username), Ok(password)) = (
            std::env::var("AUTH_USERNAME"),
            std::env::var("AUTH_PASSWORD"),
        ) {
            client = client.with_auth(username.clone(), password);
            tracing::info!("🔐 Basic authentication configured (user: {})", username);
        }
    }

    // Configure TLS
    #[cfg(feature = "tls")]
    {
        if let Ok(ca_cert) = std::env::var("TLS_CA_CERT") {
            let tls_config = if let (Ok(client_cert), Ok(client_key)) = (
                std::env::var("TLS_CLIENT_CERT"),
                std::env::var("TLS_CLIENT_KEY"),
            ) {
                tracing::info!("🔒 Configuring mTLS with client certificate");
                TlsClientConfig::with_mtls(&ca_cert, &client_cert, &client_key)?
            } else {
                tracing::info!("🔒 Configuring TLS with CA certificate");
                TlsClientConfig::with_ca_cert(&ca_cert)?
            };

            client = client.with_tls(tls_config)?;
        }
    }

    // Create test message
    let message = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    tracing::info!("📤 Sending ADT^A01 message");
    tracing::info!("   Patient ID: 12345");
    tracing::info!("   Control ID: {:?}", message.get_control_id());

    // Send with retry
    #[cfg(feature = "retry")]
    {
        let retry_policy = RetryPolicy::exponential()
            .with_max_backoff(std::time::Duration::from_secs(10))
            .with_jitter(true);

        tracing::info!(
            "🔄 Retry policy: {} attempts, exponential backoff with jitter",
            retry_policy.max_attempts
        );

        let executor = RetryExecutor::new(retry_policy);

        let response = executor
            .execute(|| async {
                tracing::info!("⏳ Attempting to send message...");
                client.send_message(&message).await
            })
            .await?;

        tracing::info!("✅ Response received");
        tracing::info!("   Control ID: {:?}", response.get_control_id());
    }

    #[cfg(not(feature = "retry"))]
    {
        let response = client.send_message(&message).await?;
        tracing::info!("✅ Response received");
        tracing::info!("   Control ID: {:?}", response.get_control_id());
    }

    Ok(())
}

/// Connect to WebSocket server
#[cfg(feature = "websocket")]
async fn connect_websocket() -> Result<()> {
    tracing::info!("🔌 Advanced HTTP Client - WebSocket Connection");

    let ws_url =
        std::env::var("WS_SERVER_URL").unwrap_or_else(|_| "ws://localhost:8081/ws".to_string());

    tracing::info!("🔗 Connecting to {}", ws_url);

    let client = WebSocketClient::new(&ws_url);

    // Create test message
    let message = AdtBuilder::a01(Version::V2_5)
        .patient_id("WS001")
        .patient_name("WEBSOCKET", "TEST")
        .build()?;

    tracing::info!("📤 Sending message via WebSocket");

    let response = client.send_message(&message).await?;

    tracing::info!("✅ Response received via WebSocket");
    tracing::info!("   Control ID: {:?}", response.get_control_id());

    Ok(())
}

/// Send multiple messages (batch)
async fn send_batch() -> Result<()> {
    tracing::info!("📦 Advanced HTTP Client - Batch Send");

    let url = std::env::var("HL7_SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let client = HttpClient::new(&url)?;

    let patient_ids = ["P001", "P002", "P003", "P004", "P005"];

    tracing::info!("📤 Sending {} messages", patient_ids.len());

    for (i, patient_id) in patient_ids.iter().enumerate() {
        let message = AdtBuilder::a01(Version::V2_5)
            .patient_id(patient_id)
            .patient_name("BATCH", &format!("PATIENT{}", i + 1))
            .build()?;

        match client.send_message(&message).await {
            Ok(response) => {
                tracing::info!(
                    "✅ [{}/{}] Sent {} - Response: {:?}",
                    i + 1,
                    patient_ids.len(),
                    patient_id,
                    response.get_control_id()
                );
            }
            Err(e) => {
                tracing::error!("❌ [{}/{}] Failed to send {}: {}", i + 1, patient_ids.len(), patient_id, e);
            }
        }

        // Small delay between messages
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    tracing::info!("✅ Batch send complete");

    Ok(())
}

/// Send message using HTTP/2
#[cfg(feature = "tls")]
async fn send_http2() -> Result<()> {
    tracing::info!("🚀 Advanced HTTP Client - HTTP/2");

    let url = std::env::var("HL7_SERVER_URL")
        .unwrap_or_else(|_| "https://localhost:8443".to_string());

    let mut client = HttpClient::new(&url)?;

    // Configure TLS (required for HTTP/2)
    if let Ok(ca_cert) = std::env::var("TLS_CA_CERT") {
        let tls_config = TlsClientConfig::with_ca_cert(&ca_cert)?;
        client = client.with_tls(tls_config)?;
    }

    // Enable HTTP/2
    client = client.with_http2_only()?;

    tracing::info!("🔒 HTTP/2 enabled with TLS");

    // Create test message
    let message = AdtBuilder::a01(Version::V2_5)
        .patient_id("HTTP2_001")
        .patient_name("HTTP2", "TEST")
        .build()?;

    tracing::info!("📤 Sending message via HTTP/2");

    let response = client.send_message(&message).await?;

    tracing::info!("✅ Response received via HTTP/2");
    tracing::info!("   Control ID: {:?}", response.get_control_id());

    Ok(())
}
