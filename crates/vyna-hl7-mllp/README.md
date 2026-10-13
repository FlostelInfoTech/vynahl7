# vyna-hl7-mllp

MLLP (Minimal Lower Layer Protocol) support for HL7 v2.x message transmission over TCP.

## Overview

`vyna-hl7-mllp` provides async client and server implementations for the MLLP protocol, the standard for intra-organization HL7 message transmission. Includes support for TLS/mTLS security and testing utilities.

## Features

- **Async I/O**: Built on Tokio for high-performance concurrent connections
- **MLLP Protocol**: Proper framing with Start Block (0x0B) and End Block (0x1C 0x0D)
- **TLS/mTLS Support**: Secure transmission with optional client certificate authentication
- **Client & Server**: Complete implementations for both roles
- **Message Acknowledgment**: Router-generated AA/AE/AR acknowledgments (`RouteResult`), auto-reject for unhandled routes
- **Testing Utilities**: MockMllpServer for integration testing
- **Connection Pooling**: Reuse connections for improved performance

## Installation

```toml
[dependencies]
vyna-hl7-mllp = "0.9"

# For TLS support
vyna-hl7-mllp = { version = "0.9", features = ["tls"] }

# For testing
vyna-hl7-mllp = { version = "0.9", features = ["testing"] }
```

## Quick Start - Client

```rust
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;
use vyna_hl7_mllp::MllpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MLLP server
    let mut client = MllpClient::connect("127.0.0.1:2575").await?;

    // Create an HL7 message
    let message = AdtBuilder::a01(Version::V2_5)
        .sending_application("MY_APP")
        .patient_id("12345")
        .patient_name("Doe", "John")
        .build()?;

    // Send message and wait for ACK
    let ack = client.send_message(&message).await?;

    println!("Received ACK: {}", ack.encode());

    // Close connection
    client.close().await?;

    Ok(())
}
```

## Quick Start - Server

```rust
use vyna_hl7_core::builders::ack::AckBuilder;
use vyna_hl7_mllp::MllpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to port
    let server = MllpServer::bind("0.0.0.0:2575").await?;
    println!("MLLP server listening on port 2575");

    // Accept connections indefinitely
    loop {
        let mut conn = server.accept().await?;

        tokio::spawn(async move {
            while let Ok(message) = conn.receive_message().await {
                println!(
                    "Received message type: {:?}",
                    message.get_message_type()
                );

                // Process message and return ACK
                let ack = match AckBuilder::for_message(&message)
                    .accept()
                    .text_message("Message accepted")
                    .build()
                {
                    Ok(ack) => ack,
                    Err(e) => {
                        eprintln!("Failed to build ACK: {e}");
                        break;
                    }
                };
                if let Err(e) = conn.send_message(&ack).await {
                    eprintln!("Failed to send ACK: {e}");
                    break;
                }
            }
        });
    }
}
```

## TLS/mTLS Support

### TLS Server

```rust
use vyna_hl7_mllp::{MllpServer, tls::TlsServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create TLS configuration
    let tls_config = TlsServerConfig::new(
        "server-cert.pem",
        "server-key.pem"
    )?;

    // Bind with TLS (accepting connections works like plain TCP)
    let server = MllpServer::bind_tls("0.0.0.0:2576", tls_config).await?;
    println!("Secure MLLP server listening on port 2576");

    loop {
        let mut conn = server.accept().await?;
        // ... receive_message / send_message as in the plain server
    }
}
```

### TLS Client

```rust
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;
use vyna_hl7_mllp::{MllpClient, tls::TlsClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create TLS configuration
    let tls_config = TlsClientConfig::with_ca_cert("ca-cert.pem")?;

    // Connect with TLS
    let mut client = MllpClient::connect_tls(
        "127.0.0.1:2576",
        "localhost",  // SNI hostname
        tls_config
    ).await?;

    // Send messages securely
    let message = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .build()?;
    let ack = client.send_message(&message).await?;

    Ok(())
}
```

### Mutual TLS (mTLS)

```rust
use vyna_hl7_mllp::tls::{TlsServerConfig, TlsClientConfig};

// Server requires client certificates
let server_config = TlsServerConfig::with_mtls(
    "server-cert.pem",
    "server-key.pem",
    "ca-cert.pem"
)?;

// Client provides certificate for authentication
let client_config = TlsClientConfig::with_mtls(
    "ca-cert.pem",
    "client-cert.pem",
    "client-key.pem"
)?;
```

## Testing with MockMllpServer

```rust
use vyna_hl7_core::builders::ack::AckBuilder;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;
use vyna_hl7_mllp::testing::MockMllpServer;
use vyna_hl7_mllp::MllpClient;

#[tokio::test]
async fn test_hl7_integration() {
    // Start mock server
    let server = MockMllpServer::new()
        .with_handler(|msg| {
            // Custom ACK logic
            AckBuilder::for_message(&msg)
                .accept()
                .text_message("Test passed")
                .build()
        })
        .start()
        .await
        .unwrap();

    // Connect client to mock server
    let mut client = MllpClient::connect(&server.url()).await.unwrap();

    // Send test message
    let test_message = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .build()
        .unwrap();
    let ack = client.send_message(&test_message).await.unwrap();

    // Verify ACK
    assert_eq!(ack.segments[1].fields[0].value(), Some("AA"));

    // Cleanup
    client.close().await.unwrap();
    server.shutdown().await.unwrap();
}
```

## MLLP Protocol Details

MLLP frames messages with special bytes:
- **Start Block**: `0x0B` (VT - Vertical Tab)
- **End Block**: `0x1C 0x0D` (FS CR - File Separator + Carriage Return)

Example framed message:
```
<VT>MSH|^~\&|APP|FAC|EMR|HOSP|...<FS><CR>
```

The library handles all framing automatically.

## Error Handling

```rust
use vyna_hl7_core::error::Error;
use vyna_hl7_mllp::MllpClient;

match MllpClient::connect("127.0.0.1:2575").await {
    Ok(client) => {
        // Use client
    }
    Err(Error::Network(e)) => {
        eprintln!("Network error: {e}");
    }
    Err(Error::Mllp(e)) => {
        eprintln!("MLLP protocol error: {e}");
    }
    Err(e) => {
        eprintln!("Error: {e:?}");
    }
}
```

## Configuration

```rust
use vyna_hl7_mllp::MllpClient;
use std::time::Duration;

let mut client = MllpClient::connect("127.0.0.1:2575").await?;

// Set read timeout
client.set_read_timeout(Duration::from_secs(30));

// Set write timeout
client.set_write_timeout(Duration::from_secs(10));
```

## Concurrent Connections

```rust
use tokio::task;
use vyna_hl7_core::error::Error;

let mut handles = vec![];

for i in 0..10 {
    let handle = task::spawn(async move {
        let mut client = MllpClient::connect("127.0.0.1:2575").await?;
        let ack = client.send_message(&create_message(i)).await?;
        client.close().await?;
        Ok::<_, Error>(ack)
    });
    handles.push(handle);
}

// Wait for all to complete
for handle in handles {
    let ack = handle.await??;
    println!("Received ACK");
}
```

## Performance

- **Throughput**: 1,000-5,000 messages/second (depends on network and message size)
- **Latency**: < 5ms for localhost connections
- **Concurrent Connections**: Limited by OS resources (typically thousands)

## Features

- `default`: Basic MLLP client and server
- `tls`: TLS/mTLS support (requires tokio-rustls)
- `testing`: MockMllpServer for integration testing
- `full`: All features enabled

## Related Crates

- **vyna-hl7-http**: HTTP transport for inter-organization communication
- **vyna-hl7-parser**: Parse received HL7 messages
- **vyna-hl7-core**: Create HL7 messages to send

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
