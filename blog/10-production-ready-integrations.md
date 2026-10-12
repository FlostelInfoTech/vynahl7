# Building Production-Ready HL7 Integrations

*Part 10 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./09-hl7-to-fhir-conversion.md), we covered FHIR conversion. Now let's discuss how to build production-ready HL7 integrations that are reliable, performant, and maintainable.

## Performance Optimization

VynaHL7 is designed for high throughput. Here's how to get the most out of it.

### Parser Performance

VynaHL7's parser achieves high throughput through zero-copy parsing with minimal allocations:

| Message Type | Parse Time | Throughput |
|--------------|------------|------------|
| Small ADT (2-3 segments) | ~9 µs | ~107,000 msgs/sec |
| Medium ORU (10-15 segments) | ~30 µs | ~34,000 msgs/sec |
| Large message (100 segments) | ~443 µs | ~2,300 msgs/sec |

*Benchmarks run on a standard development machine. Your results may vary.*

Key optimizations:
- Zero-copy parsing with minimal allocations
- Efficient `nom` parser combinators
- Pre-allocation strategies

### Network Performance (MLLP)

VynaHL7's MLLP implementation is optimized for production workloads:

| Scenario | Throughput | Latency (p50) |
|----------|------------|---------------|
| Single client | ~5,800 msg/sec | ~170 µs |
| 4 concurrent clients | ~18,400 msg/sec total | ~215 µs |
| Connection establishment | N/A | ~85 µs |

Key network optimizations:
- **TCP_NODELAY** - Immediate packet transmission (no Nagle delay)
- **Chunk-based I/O** - 4KB buffer reads instead of byte-by-byte
- **Buffered writes** - Efficient ACK responses with BufWriter
- **Connection reuse** - Persistent connections eliminate handshake overhead

```rust
use vyna_hl7_parser::parse_message;
use std::time::Instant;

fn benchmark_parsing(messages: &[&str]) {
    let start = Instant::now();

    for msg in messages {
        let _ = parse_message(msg).unwrap();
    }

    let elapsed = start.elapsed();
    let rate = messages.len() as f64 / elapsed.as_secs_f64();
    println!("Parsed {} messages in {:?} ({:.0} msg/sec)",
        messages.len(), elapsed, rate);
}
```

### CachedTerser for Repeated Access

When extracting the same fields from multiple messages, use `CachedTerser`:

```rust
use vyna_hl7_terser::CachedTerser;

// Process a batch of messages
fn process_batch(messages: Vec<Message>) {
    for message in messages {
        // CachedTerser caches path parsing and segment lookups
        let mut terser = CachedTerser::new(&message);

        // Pre-warm cache for known fields
        terser.warm_cache(&["PID-5-1", "PID-5-2", "PID-7", "PID-8", "PV1-2"]).ok();

        // Access is now 5-10x faster
        let name = terser.get("PID-5-1").ok().flatten();
        let dob = terser.get("PID-7").ok().flatten();
        // ...
    }
}
```

### Connection Pooling

For high-throughput scenarios, reuse connections:

```rust
use vyna_hl7_mllp::MllpClient;
use std::sync::Arc;
use tokio::sync::Mutex;

struct MllpPool {
    clients: Vec<Arc<Mutex<MllpClient>>>,
}

impl MllpPool {
    async fn new(addr: &str, size: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let mut clients = Vec::with_capacity(size);
        for _ in 0..size {
            let client = MllpClient::connect(addr).await?;
            clients.push(Arc::new(Mutex::new(client)));
        }
        Ok(Self { clients })
    }

    async fn send(&self, message: &Message) -> Result<Message, Box<dyn std::error::Error>> {
        // Round-robin or least-busy selection
        let client = &self.clients[rand::random::<usize>() % self.clients.len()];
        let mut guard = client.lock().await;
        Ok(guard.send_message(message).await?)
    }
}
```

## Error Handling

Robust error handling is critical in healthcare systems.

### Structured Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IntegrationError {
    #[error("Parse error: {0}")]
    Parse(#[from] vyna_hl7_parser::ParseError),

    #[error("Validation error at {location}: {message}")]
    Validation { location: String, message: String },

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Timeout waiting for acknowledgment")]
    Timeout,

    #[error("NAK received: {0}")]
    NegativeAck(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, IntegrationError>;
```

### ACK/NAK Handling

Always check acknowledgment responses:

```rust
fn handle_ack(ack: &Message, original: &Message) -> Result<()> {
    let msa = ack.get_segments_by_id("MSA")
        .first()
        .ok_or(IntegrationError::MissingField("MSA segment".into()))?;

    let ack_code = msa.get_field_value(1)
        .ok_or(IntegrationError::MissingField("MSA-1".into()))?;

    match ack_code {
        "AA" | "CA" => {
            // Application Accept / Commit Accept
            log::info!("Message {} accepted",
                original.get_control_id().unwrap_or("?"));
            Ok(())
        }
        "AE" | "CE" => {
            // Application Error / Commit Error
            let error_msg = msa.get_field_value(3).unwrap_or("Unknown error");
            log::warn!("Message {} had error: {}",
                original.get_control_id().unwrap_or("?"), error_msg);
            Err(IntegrationError::NegativeAck(error_msg.into()))
        }
        "AR" | "CR" => {
            // Application Reject / Commit Reject
            let error_msg = msa.get_field_value(3).unwrap_or("Unknown rejection");
            log::error!("Message {} rejected: {}",
                original.get_control_id().unwrap_or("?"), error_msg);
            Err(IntegrationError::NegativeAck(error_msg.into()))
        }
        _ => {
            log::warn!("Unknown ACK code: {}", ack_code);
            Err(IntegrationError::NegativeAck(format!("Unknown ACK: {}", ack_code)))
        }
    }
}
```

### Retry Logic

Implement retries with exponential backoff:

```rust
use tokio::time::{sleep, Duration};

async fn send_with_retry(
    client: &mut MllpClient,
    message: &Message,
    max_retries: u32,
) -> Result<Message> {
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < max_retries {
        match client.send_message(message).await {
            Ok(ack) => {
                match handle_ack(&ack, message) {
                    Ok(()) => return Ok(ack),
                    Err(e) => {
                        // NAK - don't retry application errors
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                attempt += 1;
                last_error = Some(IntegrationError::Network(e));

                if attempt < max_retries {
                    let delay = Duration::from_millis(100 * 2u64.pow(attempt));
                    log::warn!("Attempt {} failed, retrying in {:?}", attempt, delay);
                    sleep(delay).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or(IntegrationError::Timeout))
}
```

## Monitoring and Observability

### Structured Logging

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(message), fields(
    control_id = %message.get_control_id().unwrap_or("?"),
    msg_type = ?message.get_message_type()
))]
async fn process_message(message: Message) -> Result<()> {
    info!("Processing message");

    // Validation
    let validator = Validator::new(Version::V2_5);
    let result = validator.validate(&message);

    if !result.is_valid() {
        for err in &result.errors {
            warn!(location = %err.location, "Validation error: {}", err.message);
        }
        return Err(IntegrationError::Validation {
            location: result.errors[0].location.clone(),
            message: result.errors[0].message.clone(),
        });
    }

    // Process...
    info!("Message processed successfully");
    Ok(())
}
```

### Metrics

Track key metrics:

```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};
use lazy_static::lazy_static;

lazy_static! {
    static ref MESSAGES_RECEIVED: Counter = register_counter!(
        "hl7_messages_received_total",
        "Total number of HL7 messages received"
    ).unwrap();

    static ref MESSAGES_PROCESSED: Counter = register_counter!(
        "hl7_messages_processed_total",
        "Total number of HL7 messages successfully processed"
    ).unwrap();

    static ref PROCESSING_TIME: Histogram = register_histogram!(
        "hl7_message_processing_seconds",
        "Time spent processing HL7 messages"
    ).unwrap();
}

async fn process_with_metrics(message: Message) -> Result<()> {
    MESSAGES_RECEIVED.inc();

    let timer = PROCESSING_TIME.start_timer();
    let result = process_message(message).await;
    timer.observe_duration();

    if result.is_ok() {
        MESSAGES_PROCESSED.inc();
    }

    result
}
```

## Testing Strategies

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adt_a01() {
        let hl7 = r"MSH|^~\&|APP|FAC|RECV|DEST|20240315||ADT^A01|MSG001|P|2.5
PID|1||123456||DOE^JOHN||19800515|M";

        let message = parse_message(hl7).unwrap();

        assert_eq!(message.get_message_type(), Some(("ADT", "A01")));
        assert_eq!(message.get_control_id(), Some("MSG001"));

        let terser = Terser::new(&message);
        assert_eq!(terser.get("PID-5-1").unwrap(), Some("DOE"));
    }

    #[test]
    fn test_validation_catches_errors() {
        let invalid = r"MSH|^~\&|APP|FAC|RECV|DEST|invalid_date||ADT^A01|MSG001|P|2.5";

        let message = parse_message(invalid).unwrap();
        let validator = Validator::new(Version::V2_5);
        let result = validator.validate(&message);

        assert!(!result.is_valid());
    }
}
```

### Integration Tests with Mock Servers

```rust
use vyna_hl7_mllp::testing::MockMllpServer;

#[tokio::test]
async fn test_end_to_end_flow() {
    // Start mock server
    let server = MockMllpServer::new()
        .with_handler(|msg| {
            // Verify received message
            assert!(msg.get_segments_by_id("PID").len() > 0);
            Ok(create_ack(&msg, "AA"))
        })
        .start()
        .await
        .unwrap();

    // Test client
    let mut client = MllpClient::connect(&server.url()).await.unwrap();

    let message = create_test_message();
    let ack = client.send_message(&message).await.unwrap();

    // Verify ACK
    let msa = ack.get_segments_by_id("MSA").first().unwrap();
    assert_eq!(msa.get_field_value(1), Some("AA"));

    server.shutdown().await.unwrap();
}
```

### Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_roundtrip_encoding(
        patient_id in "[A-Z0-9]{5,10}",
        name in "[A-Z]{2,20}",
    ) {
        let message = create_message_with_patient(&patient_id, &name);
        let encoded = message.encode();
        let parsed = parse_message(&encoded).unwrap();

        let terser = Terser::new(&parsed);
        prop_assert_eq!(terser.get("PID-3-1").unwrap(), Some(patient_id.as_str()));
    }
}
```

## Configuration Management

### Environment-Based Configuration

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub client: ClientConfig,
    pub processing: ProcessingConfig,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
    pub cert_path: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Deserialize)]
pub struct ClientConfig {
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub connection_pool_size: usize,
}

#[derive(Deserialize)]
pub struct ProcessingConfig {
    pub hl7_version: String,
    pub strict_validation: bool,
    pub forward_on_validation_warning: bool,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::with_prefix("HL7"))
            .build()?
            .try_deserialize()
    }
}
```

### Usage:

```bash
export HL7_SERVER__HOST=0.0.0.0
export HL7_SERVER__PORT=2575
export HL7_SERVER__TLS_ENABLED=true
export HL7_CLIENT__TIMEOUT_MS=30000
export HL7_PROCESSING__STRICT_VALIDATION=true
```

## Graceful Shutdown

Handle shutdown signals properly:

```rust
use tokio::signal;
use tokio::sync::broadcast;

async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    let (shutdown_tx, _) = broadcast::channel(1);

    let server = MllpServer::bind("0.0.0.0:2575").await?;
    println!("Server listening on {}", server.local_addr()?);

    // Spawn shutdown signal handler
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        println!("\nShutdown signal received");
        let _ = shutdown_tx_clone.send(());
    });

    loop {
        let mut shutdown_rx = shutdown_tx.subscribe();

        tokio::select! {
            result = server.accept() => {
                match result {
                    Ok(conn) => {
                        let mut shutdown_rx = shutdown_tx.subscribe();
                        tokio::spawn(async move {
                            handle_connection(conn, shutdown_rx).await;
                        });
                    }
                    Err(e) => eprintln!("Accept error: {}", e),
                }
            }
            _ = shutdown_rx.recv() => {
                println!("Shutting down...");
                break;
            }
        }
    }

    // Allow in-flight requests to complete
    tokio::time::sleep(Duration::from_secs(5)).await;
    println!("Shutdown complete");

    Ok(())
}
```

## Production Checklist

Before deploying to production, ensure:

### Security
- [ ] TLS/mTLS enabled for all connections
- [ ] Certificates properly configured and not expired
- [ ] No PHI in logs
- [ ] Input validation on all messages
- [ ] Proper authentication on HTTP endpoints

### Reliability
- [ ] Retry logic with backoff implemented
- [ ] Connection pooling configured
- [ ] Graceful shutdown handling
- [ ] Dead letter queue for failed messages
- [ ] Health check endpoint

### Monitoring
- [ ] Structured logging configured
- [ ] Metrics exposed (Prometheus/StatsD)
- [ ] Alerting on error rates
- [ ] Message throughput dashboards
- [ ] Latency tracking

### Operations
- [ ] Configuration via environment variables
- [ ] Docker/container ready
- [ ] Horizontal scaling tested
- [ ] Disaster recovery plan
- [ ] Runbook documented

## Summary

Building production-ready HL7 integrations requires attention to:

- **Performance** - Use CachedTerser, connection pooling, efficient parsing
- **Reliability** - Proper error handling, retries, ACK validation
- **Observability** - Structured logging, metrics, tracing
- **Testing** - Unit tests, integration tests, property tests
- **Operations** - Configuration management, graceful shutdown

VynaHL7 provides the building blocks; combining them thoughtfully creates robust healthcare integrations.

---

*Next in series: [Real-World Use Cases: From EHR to Lab Systems](./11-real-world-use-cases.md)*

*Previous: [HL7 v2 to FHIR Conversion](./09-hl7-to-fhir-conversion.md)*
