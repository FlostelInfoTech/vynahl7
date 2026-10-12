# Network Transport: MLLP and HTTP for HL7 Message Exchange

*Part 8 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./07-custom-z-segments.md), we covered custom Z-segments. Now let's explore how to actually send and receive HL7 messages over the network.

VynaHL7 provides two transport protocols:
- **MLLP** (Minimal Lower Layer Protocol) - The traditional HL7 transport for intra-organization communication
- **HTTP** - Modern transport for inter-organization and web-based communication

Both support TLS encryption and mutual TLS (mTLS) for secure healthcare data exchange.

## Understanding MLLP

MLLP is the standard transport protocol for HL7 v2 messages. It wraps HL7 messages in a simple frame:

```
<VT> HL7 Message <FS><CR>
│                │    │
│                │    └── Carriage Return (0x0D)
│                └── File Separator (0x1C)
└── Vertical Tab (0x0B) - Start byte
```

This framing allows receivers to detect message boundaries over a TCP stream.

## MLLP Performance

VynaHL7's MLLP implementation is optimized for high-throughput scenarios:

| Metric | Value |
|--------|-------|
| Single client throughput | ~5,800 msg/sec |
| 4 concurrent clients | ~4,600 msg/sec per client |
| Round-trip latency (p50) | ~170 µs |
| Round-trip latency (p99) | ~250 µs |

**Key Optimizations:**
- **TCP_NODELAY** - Disabled Nagle's algorithm for immediate packet transmission
- **Chunk-based I/O** - Process incoming data in 4KB chunks instead of byte-by-byte
- **Buffered writes** - Use BufWriter for efficient ACK responses
- **Connection reuse** - Persistent connections eliminate handshake overhead

*Benchmarks measured with ADT^A01 messages (316 bytes) on localhost.*

## MLLP Server

Here's how to create an MLLP server that receives HL7 messages:

```rust
use vyna_hl7_mllp::MllpServer;
use vyna_hl7_core::{Message, Segment, Field, delimiters::Delimiters};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Bind to a port
    let server = MllpServer::bind("0.0.0.0:2575").await?;
    println!("MLLP server listening on {}", server.local_addr()?);

    loop {
        // Accept incoming connections
        let mut connection = server.accept().await?;
        println!("Client connected");

        // Handle connection in a separate task
        tokio::spawn(async move {
            loop {
                // Receive message (automatically parsed)
                match connection.receive_message().await {
                    Ok(message) => {
                        println!("Received: {:?}", message.get_message_type());
                        println!("Control ID: {:?}", message.get_control_id());

                        // Process and create ACK
                        let ack = create_ack(&message, "AA");

                        // Send ACK back
                        if let Err(e) = connection.send_message(&ack).await {
                            eprintln!("Failed to send ACK: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Connection error: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

fn create_ack(original: &Message, ack_code: &str) -> Message {
    let mut ack = Message::new();
    let delims = Delimiters::default();

    // MSH segment
    let mut msh = Segment::new("MSH");
    msh.add_field(Field::from_value("|"));
    msh.add_field(Field::from_value("^~\\&"));

    // Swap sender/receiver
    if let Some(v) = original.get_receiving_application() {
        msh.set_field_value(3, v).unwrap();
    }
    if let Some(v) = original.get_sending_application() {
        msh.set_field_value(5, v).unwrap();
    }

    msh.set_field_value(7, Utc::now().format("%Y%m%d%H%M%S").to_string()).unwrap();
    msh.set_field_value(9, "ACK").unwrap();
    msh.set_field_value(10, format!("ACK{}", Utc::now().timestamp())).unwrap();
    msh.set_field_value(11, "P").unwrap();

    if let Some(v) = original.get_version() {
        msh.set_field_value(12, v.as_str()).unwrap();
    }
    ack.add_segment(msh);

    // MSA segment
    let mut msa = Segment::new("MSA");
    msa.set_field_value(1, ack_code).unwrap();
    if let Some(id) = original.get_control_id() {
        msa.set_field_value(2, id).unwrap();
    }
    ack.add_segment(msa);

    ack
}
```

## MLLP Client

Here's how to send messages to an MLLP server:

```rust
use vyna_hl7_mllp::MllpClient;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a message to send
    let message = AdtBuilder::a01(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RemoteApp")
        .receiving_facility("RemoteFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800515")
        .sex("M")
        .patient_class("I")
        .build()?;

    // Connect to MLLP server
    let mut client = MllpClient::connect("192.168.1.100:2575").await?;
    println!("Connected to server");

    // Send message and wait for ACK
    let ack = client.send_message(&message).await?;
    println!("Received ACK");

    // Check ACK status
    if let Some(msa) = ack.get_segments_by_id("MSA").first() {
        match msa.get_field_value(1) {
            Some("AA") => println!("Message accepted"),
            Some("AE") => println!("Application error"),
            Some("AR") => println!("Message rejected"),
            other => println!("Unknown status: {:?}", other),
        }
    }

    // Close connection
    client.close().await?;

    Ok(())
}
```

## MLLP with TLS

For secure communication, VynaHL7 supports TLS and mutual TLS (mTLS).

### TLS Server

```rust
use vyna_hl7_mllp::{MllpServer, tls::TlsServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Basic TLS (server certificate only)
    let tls_config = TlsServerConfig::new(
        "server-cert.pem",
        "server-key.pem"
    )?;

    // Or mutual TLS (client certificate verification)
    let mtls_config = TlsServerConfig::with_mtls(
        "server-cert.pem",
        "server-key.pem",
        "ca-cert.pem"  // CA that signed client certs
    )?;

    // Bind with TLS
    let server = MllpServer::bind_tls("0.0.0.0:2575", tls_config).await?;
    println!("TLS server listening on {}", server.local_addr()?);

    loop {
        // TLS handshake happens on accept
        let mut conn = server.accept().await?;
        println!("Client connected (TLS handshake successful)");

        tokio::spawn(async move {
            // Handle messages same as non-TLS
            while let Ok(message) = conn.receive_message().await {
                let ack = create_ack(&message, "AA");
                if conn.send_message(&ack).await.is_err() {
                    break;
                }
            }
        });
    }
}
```

### TLS Client

```rust
use vyna_hl7_mllp::{MllpClient, tls::TlsClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Basic TLS (verify server certificate)
    let tls_config = TlsClientConfig::with_ca_cert("ca-cert.pem")?;

    // Or mutual TLS (provide client certificate)
    let mtls_config = TlsClientConfig::with_mtls(
        "ca-cert.pem",
        "client-cert.pem",
        "client-key.pem"
    )?;

    // Connect with TLS
    let mut client = MllpClient::connect_tls(
        "secure-server.hospital.org:2575",
        "secure-server.hospital.org",  // Server name for SNI
        tls_config
    ).await?;

    // Send messages same as non-TLS
    let ack = client.send_message(&message).await?;

    Ok(())
}
```

### Generating Test Certificates

For development and testing:

```bash
# Generate CA
openssl genrsa -out ca-key.pem 4096
openssl req -new -x509 -days 365 -key ca-key.pem -out ca-cert.pem \
    -subj "/CN=Test CA"

# Generate server certificate
openssl genrsa -out server-key.pem 4096
openssl req -new -key server-key.pem -out server.csr \
    -subj "/CN=localhost"
openssl x509 -req -days 365 -in server.csr -CA ca-cert.pem \
    -CAkey ca-key.pem -CAcreateserial -out server-cert.pem

# Generate client certificate (for mTLS)
openssl genrsa -out client-key.pem 4096
openssl req -new -key client-key.pem -out client.csr \
    -subj "/CN=client"
openssl x509 -req -days 365 -in client.csr -CA ca-cert.pem \
    -CAkey ca-key.pem -CAcreateserial -out client-cert.pem
```

## HTTP Transport

HTTP transport is useful for:
- Inter-organization communication through firewalls
- Web-based HL7 services
- Load balancing and routing
- RESTful architecture integration

### HTTP Server

```rust
use vyna_hl7_http::{HttpServer, MessageHandler};
use vyna_hl7_core::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define message handler
    let handler: MessageHandler = Arc::new(|message: Message| {
        println!("Received: {:?}", message.get_message_type());
        println!("From: {:?}", message.get_sending_application());

        // Process and return ACK
        let ack = create_ack(&message, "AA")?;
        Ok(ack)
    });

    // Create server
    let server = HttpServer::new()
        .with_handler(handler);

    println!("HTTP server listening on http://127.0.0.1:8080");
    server.serve("127.0.0.1:8080").await?;

    Ok(())
}
```

### HTTP Server with Authentication

```rust
let server = HttpServer::new()
    .with_handler(handler)
    .with_auth("username".into(), "password".into());

server.serve("127.0.0.1:8080").await?;
```

### HTTP Client

```rust
use vyna_hl7_http::HttpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = HttpClient::new("http://hl7-gateway.partner.org:8080")?;

    // Or with authentication
    let client = HttpClient::new("http://hl7-gateway.partner.org:8080")?
        .with_auth("username".to_string(), "password".to_string());

    // Or with timeout
    let client = HttpClient::new("http://hl7-gateway.partner.org:8080")?
        .with_timeout(std::time::Duration::from_secs(30));

    // Send message
    let ack = client.send_message(&message).await?;

    Ok(())
}
```

### HTTP with TLS

```rust
use vyna_hl7_http::{HttpServer, HttpClient};

// HTTPS Server
let server = HttpServer::new()
    .with_handler(handler)
    .with_tls("server-cert.pem", "server-key.pem");

server.serve_tls("0.0.0.0:8443").await?;

// HTTPS Client
let client = HttpClient::new_tls(
    "https://secure-gateway.hospital.org:8443",
    "ca-cert.pem"
)?;

let ack = client.send_message(&message).await?;
```

## Testing with Mock Servers

VynaHL7 provides mock servers for testing:

### Mock MLLP Server

```rust
use vyna_hl7_mllp::testing::MockMllpServer;

#[tokio::test]
async fn test_mllp_client() {
    // Start mock server
    let server = MockMllpServer::new()
        .with_handler(|msg| {
            // Custom processing logic
            Ok(create_ack(&msg))
        })
        .start()
        .await
        .unwrap();

    // Get server URL (automatically allocated port)
    let url = server.url();

    // Test your client code
    let mut client = MllpClient::connect(&url).await.unwrap();
    let ack = client.send_message(&test_message).await.unwrap();

    assert_eq!(ack.get_segments_by_id("MSA")[0].get_field_value(1), Some("AA"));

    // Clean up
    server.shutdown().await.unwrap();
}
```

### Mock HTTP Server

```rust
use vyna_hl7_http::testing::MockHttpServer;

#[tokio::test]
async fn test_http_client() {
    let server = MockHttpServer::new()
        .with_auth("user".into(), "pass".into())
        .start()
        .await
        .unwrap();

    let client = HttpClient::new(&server.url())
        .unwrap()
        .with_auth("user".into(), "pass".into());

    let ack = client.send_message(&test_message).await.unwrap();

    server.shutdown().await.unwrap();
}
```

## Choosing Between MLLP and HTTP

| Factor | MLLP | HTTP |
|--------|------|------|
| **Use Case** | Internal hospital networks | Cross-organization, internet |
| **Firewall** | Requires port forwarding | Usually allowed (80/443) |
| **Load Balancing** | Complex | Native support |
| **Standards** | HL7 standard | HL7-over-HTTP spec |
| **Security** | TLS/mTLS | TLS/mTLS + OAuth possible |
| **Complexity** | Simple | More features |
| **Latency** | Lower | Slightly higher |

### When to Use MLLP

- Intra-organization communication
- High-throughput scenarios
- Direct system-to-system integration
- Legacy system compatibility

### When to Use HTTP

- Inter-organization communication
- Cloud deployments
- API gateway integration
- Web-based applications

## Real-World Example: Message Router

Here's a complete example of a message router that receives messages via MLLP and forwards them to different destinations:

```rust
use vyna_hl7_mllp::MllpServer;
use vyna_hl7_http::HttpClient;
use vyna_hl7_terser::Terser;
use vyna_hl7_core::Message;
use std::collections::HashMap;

struct MessageRouter {
    routes: HashMap<String, String>,
    http_client: HttpClient,
}

impl MessageRouter {
    fn new() -> Self {
        let mut routes = HashMap::new();
        routes.insert("ADT".to_string(), "http://adt-processor:8080");
        routes.insert("ORU".to_string(), "http://lab-processor:8080");
        routes.insert("ORM".to_string(), "http://order-processor:8080");

        Self {
            routes,
            http_client: HttpClient::new("http://default:8080").unwrap(),
        }
    }

    async fn route(&self, message: &Message) -> Result<Message, Box<dyn std::error::Error>> {
        let (msg_type, _) = message.get_message_type()
            .ok_or("Missing message type")?;

        let destination = self.routes
            .get(msg_type)
            .ok_or_else(|| format!("No route for {}", msg_type))?;

        println!("Routing {} message to {}", msg_type, destination);

        let client = HttpClient::new(destination)?;
        let ack = client.send_message(message).await?;

        Ok(ack)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = MessageRouter::new();
    let server = MllpServer::bind("0.0.0.0:2575").await?;

    println!("Message router listening on {}", server.local_addr()?);

    loop {
        let mut conn = server.accept().await?;
        let router_ref = &router;  // In practice, use Arc

        tokio::spawn(async move {
            while let Ok(message) = conn.receive_message().await {
                let ack = match router_ref.route(&message).await {
                    Ok(ack) => ack,
                    Err(e) => {
                        eprintln!("Routing error: {}", e);
                        create_nack(&message, &e.to_string())
                    }
                };

                if conn.send_message(&ack).await.is_err() {
                    break;
                }
            }
        });
    }
}
```

## Summary

VynaHL7 provides robust network transport options:

- **MLLP** - Traditional HL7 transport with TLS/mTLS support
- **HTTP** - Modern transport for web and cross-organization use
- **Mock servers** - Easy testing without external dependencies

Both protocols support:
- Asynchronous I/O with Tokio
- TLS encryption
- Mutual TLS authentication
- Connection pooling and timeouts

---

*Next in series: [Bridging to Modern Healthcare: HL7 v2 to FHIR Conversion](./09-hl7-to-fhir-conversion.md)*

*Previous: [Custom Z-Segments](./07-custom-z-segments.md)*
