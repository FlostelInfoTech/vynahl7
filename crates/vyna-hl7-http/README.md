# vyna-hl7-http

Production-ready HTTP transport for HL7 v2.x messages - enabling secure, scalable inter-organization healthcare data exchange.

## Overview

`vyna-hl7-http` implements the HAPI HL7-over-HTTP specification for sending HL7 v2.x messages over HTTP/HTTPS. This is ideal for inter-organization communication where MLLP (used for intra-organization) is not suitable.

## Features

### Core Transport
- **HTTP Client**: Send HL7 messages via HTTP POST and receive ACK responses
- **HTTP Server**: Receive HL7 messages and send ACK responses
- **Content Types**: Support for `x-application/hl7-v2+er7` (pipe-delimited)
- **Async**: Built on Tokio for high performance

### Security & Authentication 🔒
- **mTLS (Mutual TLS)**: Two-way certificate authentication
- **HTTP Basic Auth**: Simple username/password authentication
- **API Key Auth**: Custom header-based authentication with constant-time comparison
- **JWT/OAuth 2.0**: Token-based authentication (HS256, RS256, ES256)
- **TLS/HTTPS**: Secure encrypted connections

### Protocol & Connection 🌐
- **HTTP/2 Support**: Modern HTTP protocol with multiplexing
- **WebSocket**: Bidirectional real-time message streaming
- **Retry Logic**: Exponential backoff with jitter for transient failures
- **Connection Pooling**: Efficient connection reuse

### Operational Features 📊
- **Structured Logging**: Correlation IDs, tracing spans, HL7 metadata
- **Prometheus Metrics**: Time-series metrics with histogram buckets
- **Rate Limiting**: Configurable request throttling
- **Message Compression**: Automatic gzip/brotli compression

### Advanced Messaging 📨
- **Async Message Queue**: Bounded/unbounded message queuing
- **Batch Processing**: Time/size-based batch message handling
- **Message Routing**: Route by message type and trigger event
- **Webhooks**: HTTP notifications with retry logic

## MLLP vs HTTP

| Feature | MLLP (vyna-hl7-mllp) | HTTP (vyna-hl7-http) |
|---------|-----------------|-----------------|
| Use Case | Intra-organization | Inter-organization |
| Protocol | TCP + framing | HTTP/HTTPS |
| Connection | Persistent | Stateless |
| Security | VPN typically | mTLS + Auth + JWT |
| Firewall | Requires configuration | Works through proxies |
| Cloud/SaaS | Limited | Native support |
| WebSockets | No | Yes |
| Metrics | Basic | Prometheus ready |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-http = "0.6"
vyna-hl7-core = "0.6"

# With all features
vyna-hl7-http = { version = "0.6", features = ["full"] }

# Or selective features
vyna-hl7-http = { version = "0.6", features = ["tls", "auth", "logging", "metrics"] }
```

### Available Feature Flags

**Security Features:**
- `tls` - TLS/HTTPS support with rustls
- `auth` - HTTP Basic Auth and API Key authentication
- `oauth` - JWT/OAuth 2.0 token authentication (requires `auth`)

**Protocol Features:**
- `websocket` - WebSocket bidirectional streaming
- `retry` - Retry logic with exponential backoff

**Operational Features:**
- `logging` - Structured logging with tracing
- `metrics` - Prometheus metrics
- `ratelimit` - Rate limiting configuration helpers
- `compression` - gzip/brotli message compression

**Messaging Features:**
- `queue` - Async message queue
- `batch` - Batch message processing
- `routing` - Message routing by type
- `webhooks` - Webhook notifications

**Meta Feature:**
- `full` - Enable all features

## Quick Start

### Basic HTTP Server

```rust
use vyna_hl7_http::HttpServer;
use vyna_hl7_core::Message;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = HttpServer::new()
        .with_handler(Arc::new(|message: Message| {
            println!("Received: {:?}", message.get_message_type());
            Ok(message) // Echo message as ACK
        }));

    server.serve("127.0.0.1:8080").await?;
    Ok(())
}
```

### Basic HTTP Client

```rust
use vyna_hl7_http::HttpClient;
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = HttpClient::new("http://localhost:8080")?;

    let message = AdtBuilder::a01(Version::V2_5)
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    let ack = client.send_message(&message).await?;
    println!("ACK received: {:?}", ack.get_control_id());
    Ok(())
}
```

## Security Features

### mTLS (Mutual TLS)

#### Server with mTLS

```rust
use vyna_hl7_http::{HttpServer, tls::TlsServerConfig};

let tls_config = TlsServerConfig::with_mtls(
    "server-cert.pem",
    "server-key.pem",
    "ca-cert.pem"  // Client CA certificate
)?;

let server = HttpServer::new()
    .with_handler(handler)
    .with_tls(tls_config);

server.serve_tls("0.0.0.0:8443").await?;
```

#### Client with mTLS

```rust
use vyna_hl7_http::{HttpClient, tls::TlsClientConfig};

let tls_config = TlsClientConfig::with_mtls(
    "ca-cert.pem",      // Server CA certificate
    "client-cert.pem",  // Client certificate
    "client-key.pem"    // Client private key
)?;

let client = HttpClient::new("https://server.example.com/hl7")?
    .with_tls(tls_config)?;
```

### API Key Authentication

```rust
use vyna_hl7_http::middleware::{api_key_middleware, ApiKeyAuthState};
use axum::{Router, routing::post, middleware::from_fn_with_state};

let auth_state = ApiKeyAuthState::new(vec!["secret-key-123".into()])
    .with_header_name("X-API-Key");

let app = Router::new()
    .route("/hl7", post(handler))
    .layer(from_fn_with_state(auth_state, api_key_middleware));
```

### JWT Authentication

```rust
use vyna_hl7_http::auth::JwtConfig;
use vyna_hl7_http::middleware::{jwt_middleware, JwtAuthState};
use std::sync::Arc;

let jwt_config = JwtConfig::new_hs256(b"your-secret-key");
let auth_state = JwtAuthState::new(Arc::new(jwt_config));

let app = Router::new()
    .route("/hl7", post(handler))
    .layer(from_fn_with_state(auth_state, jwt_middleware));
```

## Protocol Features

### HTTP/2

```rust
let client = HttpClient::new("https://server.example.com")?
    .with_http2_only()?;
```

### WebSocket

#### Server

```rust
use vyna_hl7_http::websocket::{websocket_handler, WebSocketConfig};
use axum::{Router, routing::get};

let ws_config = WebSocketConfig::new(handler)
    .with_max_message_size(2 * 1024 * 1024)  // 2 MB
    .with_broadcast();

let app = Router::new()
    .route("/ws", get(websocket_handler))
    .with_state(ws_config);
```

#### Client

```rust
use vyna_hl7_http::websocket::WebSocketClient;

let client = WebSocketClient::new("ws://localhost:8081/ws");
let response = client.send_message(&message).await?;
```

### Retry Logic

```rust
use vyna_hl7_http::retry::{RetryExecutor, RetryPolicy};

let policy = RetryPolicy::exponential()  // 3 retries, 100ms initial
    .with_max_backoff(Duration::from_secs(10))
    .with_jitter(true);

let executor = RetryExecutor::new(policy);

let response = executor.execute(|| async {
    client.send_message(&message).await
}).await?;
```

## Operational Features

### Structured Logging

```rust
use vyna_hl7_http::logging;

// Initialize logging
tracing_subscriber::fmt()
    .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
    .init();

// Automatically logs HL7 messages with metadata
logging::log_hl7_message(&message, "received");
```

### Prometheus Metrics

```rust
use vyna_hl7_http::metrics;

// Initialize metrics
let metrics_handle = metrics::init_metrics();

// Serve metrics endpoint
let app = Router::new()
    .route("/metrics", get(|| async move { metrics_handle.render() }));

// Metrics are automatically recorded by middleware
```

### Message Compression

```rust
let server = HttpServer::new()
    .with_handler(handler)
    .with_compression();  // Enables gzip/brotli
```

### Rate Limiting

```rust
use vyna_hl7_http::ratelimit;

let config = ratelimit::presets::moderate();  // 60 req/min
println!("Rate limit: {} requests per {:?}", config.requests, config.period);

// Note: Requires manual integration with governor crate
```

## Advanced Messaging

### Async Message Queue

```rust
use vyna_hl7_http::queue::MessageQueue;

let queue = MessageQueue::bounded(1000);

// Producer
queue.send(message).await?;

// Consumer
let receiver = queue.receiver();
while let Ok(message) = receiver.recv().await {
    process_message(message)?;
}
```

### Batch Processing

```rust
use vyna_hl7_http::batch::{BatchProcessor, BatchConfig};

let config = BatchConfig::medium();  // 50 messages or 15 seconds

let mut processor = BatchProcessor::new(
    config.max_size,
    config.max_age,
    |messages| {
        println!("Processing batch of {} messages", messages.len());
        // Batch processing logic
    }
);

processor.add(message);
processor.flush_if_needed();
```

### Message Routing

```rust
use vyna_hl7_http::router::RouterBuilder;

let router = RouterBuilder::new()
    .route("ADT", "A01", Arc::new(handle_admission))
    .route("ADT", "A04", Arc::new(handle_registration))
    .route("ORU", "R01", Arc::new(handle_lab_results))
    .type_route("SIU", Arc::new(handle_scheduling))
    .default(Arc::new(handle_default))
    .build();

let response = router.route(message)?;
```

### Webhooks

```rust
use vyna_hl7_http::webhook::{WebhookClient, WebhookConfig};

let config = WebhookConfig {
    url: "https://example.com/webhook".to_string(),
    timeout: Duration::from_secs(5),
    max_retries: 3,
    include_message_content: false,
    headers: vec![("X-Service".into(), "VynaHL7-HTTP".into())],
};

let webhook = WebhookClient::new(config)?;
webhook.notify_received(&message).await?;
```

## Examples

The crate includes comprehensive examples:

### Basic Examples

Run the basic HTTP server:
```bash
cargo run --example http_server
```

Run the basic HTTP client:
```bash
cargo run --example http_client
```

### Advanced Examples

Run the advanced server with all features:
```bash
# With logging, metrics, compression, routing
cargo run --example advanced_server --features logging,metrics,compression,routing,webhooks,queue,batch

# With all features including TLS and WebSocket
cargo run --example advanced_server --features full
```

Run the advanced client:
```bash
# Send with retry logic
cargo run --example advanced_client --features retry -- send

# Connect via WebSocket
cargo run --example advanced_client --features websocket -- websocket

# Send batch messages
cargo run --example advanced_client --features retry -- batch
```

### Test with curl

```bash
# Basic HTTP POST
curl -X POST http://127.0.0.1:8080 \
  -H "Content-Type: x-application/hl7-v2+er7" \
  -d "MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250126120000||ADT^A01|MSG00001|P|2.5"

# With API Key authentication
curl -X POST http://127.0.0.1:8080 \
  -H "Content-Type: x-application/hl7-v2+er7" \
  -H "X-API-Key: secret-key-123" \
  -d "MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250126120000||ADT^A01|MSG00001|P|2.5"
```

## Error Handling

```rust
use vyna_hl7_http::{Error, Result};

match client.send_message(&message).await {
    Ok(ack) => println!("Success: {:?}", ack),
    Err(Error::Network(e)) => eprintln!("Network error: {}", e),
    Err(Error::Auth(e)) => eprintln!("Authentication failed: {}", e),
    Err(Error::ContentType { expected, actual }) => {
        eprintln!("Wrong content type. Expected {}, got {}", expected, actual);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Configuration Reference

### Server Configuration

```rust
let server = HttpServer::new()
    .with_handler(message_handler)           // Required: Message handler
    .with_auth(username, password)           // Optional: Basic auth
    .with_tls(tls_config)                    // Optional: TLS/mTLS
    .with_compression();                     // Optional: Compression

// HTTP
server.serve("0.0.0.0:8080").await?;

// HTTPS (requires TLS configuration)
server.serve_tls("0.0.0.0:8443").await?;
```

### Client Configuration

```rust
let client = HttpClient::new("https://server.example.com")?
    .with_auth(username, password)           // Optional: Basic auth
    .with_tls(tls_config)?                   // Optional: TLS/mTLS
    .with_http2_only()?                      // Optional: HTTP/2
    .with_timeout(Duration::from_secs(30))?; // Optional: Timeout
```

## Performance Considerations

- **Connection Pooling**: Client automatically pools connections
- **Async I/O**: Non-blocking operations with Tokio
- **Compression**: Reduces bandwidth by 60-80% for large messages
- **HTTP/2**: Multiplexing reduces connection overhead
- **Batch Processing**: Reduces per-message overhead
- **Message Queue**: Decouples message receipt from processing

## Specification

This implementation follows the HAPI HL7-over-HTTP specification:
<https://hapifhir.github.io/hapi-hl7v2/hapi-hl7overhttp/specification.html>

## Dependencies

Built with modern, well-maintained crates:
- `axum` 0.8 - Web framework
- `tokio` - Async runtime
- `rustls` 0.23 - TLS implementation
- `reqwest` 0.12 - HTTP client
- `tracing` - Structured logging
- `metrics` - Metrics collection

## License

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.

## Contributing

This crate is developed in the private [VynaHL7 repository](https://gitlab.flostel.com/alexshao/VynaHL7). Internal contributors: please create a branch and submit a merge request.
