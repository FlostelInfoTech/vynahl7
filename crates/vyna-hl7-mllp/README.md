# vyna-hl7-mllp

MLLP (Minimal Lower Layer Protocol) support for HL7 v2.x message transmission over TCP.

## Overview

`vyna-hl7-mllp` provides async client and server implementations for the MLLP protocol, the standard for intra-organization HL7 message transmission. Includes support for TLS/mTLS security and testing utilities.

## Features

- **Async I/O**: Built on Tokio for high-performance concurrent connections
- **MLLP Protocol**: Proper framing with Start Block (0x0B) and End Block (0x1C 0x0D)
- **TLS/mTLS Support**: Secure transmission with optional client certificate authentication
- **Client & Server**: Complete implementations for both roles
- **Message Acknowledgment**: Automatic ACK generation and handling
- **Testing Utilities**: MockMllpServer for integration testing
- **Connection Pooling**: Reuse connections for improved performance

## Installation

```toml
[dependencies]
vyna-hl7-mllp = "0.19"

# For TLS support
vyna-hl7-mllp = { version = "0.19", features = ["tls"] }

# For testing
vyna-hl7-mllp = { version = "0.19", features = ["testing"] }
```

## Quick Start - Client

```rust
use vyna_hl7_mllp::MllpClient;
use vyna_hl7_core::builders::adt::AdtA01Builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to MLLP server
    let mut client = MllpClient::connect("127.0.0.1:2575").await?;

    // Create an HL7 message
    let message = AdtA01Builder::new()
        .msh_sending_application("MY_APP")
        .msh_receiving_application("EMR")
        .pid_patient_id("12345", "MR", "HOSPITAL")
        .pid_patient_name("Doe", "John", "M")
        .build();

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
use vyna_hl7_mllp::MllpServer;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to port
    let listener = TcpListener::bind("0.0.0.0:2575").await?;
    println!("MLLP server listening on port 2575");

    // Define message handler
    let handler = |message| {
        println!("Received message type: {}",
            message.message_type().unwrap_or("Unknown"));

        // Process message and return ACK
        vyna_hl7_core::ack::create_ack(&message, "AA", "Message accepted")
    };

    // Serve indefinitely
    MllpServer::serve(listener, handler).await?;

    Ok(())
}
```

## TLS/mTLS Support

### TLS Server

```rust
use vyna_hl7_mllp::{MllpServer, tls::TlsServerConfig};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create TLS configuration
    let tls_config = TlsServerConfig::new(
        "server-cert.pem",
        "server-key.pem"
    )?;

    let listener = TcpListener::bind("0.0.0.0:2576").await?;
    println!("Secure MLLP server listening on port 2576");

    // Serve with TLS
    MllpServer::serve_tls(listener, tls_config, handler).await?;

    Ok(())
}
```

### TLS Client

```rust
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
use vyna_hl7_mllp::testing::MockMllpServer;
use vyna_hl7_mllp::MllpClient;

#[tokio::test]
async fn test_hl7_integration() {
    // Start mock server
    let server = MockMllpServer::new()
        .with_handler(|msg| {
            // Custom ACK logic
            vyna_hl7_core::ack::create_ack(&msg, "AA", "Test passed")
        })
        .start()
        .await
        .unwrap();

    // Connect client to mock server
    let mut client = MllpClient::connect(&server.url()).await.unwrap();

    // Send test message
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
use vyna_hl7_mllp::{MllpClient, Error};

match MllpClient::connect("127.0.0.1:2575").await {
    Ok(client) => {
        // Use client
    }
    Err(Error::Network(e)) => {
        eprintln!("Network error: {}", e);
    }
    Err(Error::Timeout) => {
        eprintln!("Connection timeout");
    }
    Err(e) => {
        eprintln!("Error: {:?}", e);
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

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.
