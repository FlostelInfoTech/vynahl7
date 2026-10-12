# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VynaHL7 is a comprehensive Rust library for parsing, validating, and creating HL7 v2.x healthcare messages, inspired by the Java HAPI library. The project uses a workspace architecture with 10 specialized crates.

## Build, Test, and Development Commands

### Building
```bash
# Build entire workspace
cargo build --workspace

# Build specific crate
cargo build -p vyna-hl7-core
cargo build -p vyna-hl7-parser

# Build with release optimizations
cargo build --release --workspace

# Build CLI tool
cargo build -p vyna-hl7-cli --release
```

### Testing
```bash
# Run all tests in workspace
cargo test --workspace

# Run tests for specific crate
cargo test -p vyna-hl7-parser
cargo test -p vyna-hl7-validator

# Run specific test with output
cargo test test_name -- --nocapture

# Run tests with all features enabled
cargo test --workspace --all-features
```

### Running Examples
```bash
# Core examples
cargo run --example parse_adt
cargo run --example create_message
cargo run --example message_builders
cargo run --example schema_validation
cargo run --example datatype_validation
cargo run --example complete_validation

# MLLP examples (requires two terminals)
cargo run --example mllp_server    # Terminal 1
cargo run --example mllp_client    # Terminal 2

# HTTP examples (requires two terminals)
cargo run --example http_server -p vyna-hl7-http   # Terminal 1
cargo run --example http_client -p vyna-hl7-http   # Terminal 2

# FHIR conversion examples
cargo run --example convert_adt -p vyna-hl7-fhir
cargo run --example convert_oru -p vyna-hl7-fhir
```

### CLI Tool
```bash
# Install CLI
cargo install --path crates/vyna-hl7-cli

# CLI commands
vynahl7 parse message.hl7 --format pretty
vynahl7 validate message.hl7
vynahl7 extract message.hl7 PID-5 PID-7 PID-8
vynahl7 convert message.hl7 --to json --pretty
vynahl7 info message.hl7
vynahl7 send message.hl7 --to mllp://127.0.0.1:2575
vynahl7 send message.hl7 --to http://127.0.0.1:8080/hl7 --user u --password p
vynahl7 diff old.hl7 new.hl7 [--format json] [--ignore MSH-7,MSH-10]  # exit 1 when different
```

### Benchmarking
```bash
# Run all benchmarks
cargo bench --workspace

# Benchmark specific crate
cargo bench -p vyna-hl7-parser
cargo bench -p vyna-hl7-terser
```

### WebAssembly
```bash
# Build WASM package
cd crates/vyna-hl7-wasm
wasm-pack build --target web
wasm-pack build --target nodejs
```

## Architecture

### Workspace Structure

```
vyna-hl7/
├── vyna-hl7-core       - Core data structures (Message, Segment, Field, Component, Subcomponent)
├── vyna-hl7-parser     - HL7 message parser (delimiter-based parsing)
├── vyna-hl7-validator  - Schema-based validation, data type validation, vocabulary validation
├── vyna-hl7-terser     - Path-based field access API (similar to HAPI's Terser)
├── vyna-hl7-mllp       - MLLP protocol for intra-organization network transmission
├── vyna-hl7-http       - HTTP transport for inter-organization communication
├── vyna-hl7-fhir       - HL7 v2 to FHIR R4 conversion (9 converters)
├── vyna-hl7-wasm       - WebAssembly bindings for JavaScript/TypeScript
├── vyna-hl7-cli        - Command-line interface for message analysis
└── vyna-hl7-macros     - Derive macros for message types (placeholder)
```

### Core Concepts

**Message Hierarchy:**
```
Message
  └─ Segment (MSH, PID, OBX, etc.)
       └─ Field (separated by |)
            └─ Repetition (separated by ~)
                 └─ Component (separated by ^)
                      └─ Subcomponent (separated by &)
```

**Supported HL7 Versions:** v2.3, v2.3.1, v2.4, v2.5, v2.5.1, v2.6, v2.7, v2.7.1, v2.8, v2.8.1, v2.8.2

**Message Types:** ADT (A01-A40), SIU (S12-S15), MDM (T01-T04), DFT (P03, P11), QRY (A19, Q01-Q02), BAR (P01-P02), Pharmacy (RDE, RAS, RDS, RGV, RRD, RRA), Laboratory (OUL, OML), ORM, ORU, ACK, MFN

### Key Design Patterns

**Builders Pattern:**
The project uses fluent builders for message creation. Builders are located in `vyna-hl7-core/src/builders/`:
- `adt.rs` - ADT message builders (A01-A13, A17, A28, A31, A40)
- `siu.rs` - Scheduling messages
- `orm.rs`, `oru.rs` - Order/result messages
- `pharmacy.rs`, `laboratory.rs` - Specialized message builders
- `fields.rs` - Complex field builders (XPN, XAD, XTN, CX, XCN)

**Validation Architecture:**
Three-layer validation system in `vyna-hl7-validator`:
1. Schema validation - Message structure against HL7 standards (JSON schemas in `schemas/v2_X/`)
2. Data type validation - Format checking (dates, times, numerics, coded values) in `datatype.rs`
3. Vocabulary validation - Code set validation against HL7 tables (Table 0001, etc.) in `vocabulary.rs`

**Terser API:**
Field access uses path notation (e.g., `PID-5-1`, `OBX(2)-5`). Two implementations:
- `Terser` - Standard field access
- `CachedTerser` - 5-10x faster for repeated access to same fields

**Parser Design:**
Uses delimiter-based splitting with minimal allocations. The `optimized.rs` module contains pre-allocation strategies for common patterns.

## Important Implementation Details

### Terser Path Notation

**IMPORTANT: Terser segment indexing is 1-based** (e.g., `OBX(1)` for first OBX, `OBX(2)` for second OBX). This matches HAPI conventions and HL7's 1-based field numbering.

Path examples:
- `PID-5` - PID segment, field 5
- `PID-5-1` - PID segment, field 5, component 1
- `PID-5-1-2` - PID segment, field 5, component 1, subcomponent 2
- `OBX(1)-5` - First OBX segment, field 5
- `OBX(2)-5` - Second OBX segment, field 5
- `PID-11(1)-1` - PID segment, field 11, second repetition, component 1

**Note:** Repetition indexing remains 0-based internally (first repetition is index 0).

### MSH Segment Field Numbering

**IMPORTANT: The MSH segment has special field numbering.** Unlike all other segments, MSH-1 is the Field Separator character itself (typically `|`), and MSH-2 contains the four Encoding Characters (typically `^~\&`). This means:

- `MSH|^~\&|...` — the `|` after `MSH` is MSH-1 (field separator), and `^~\&` is MSH-2 (encoding characters)
- MSH-1 is NOT the first value after the first `|`; it IS the `|` delimiter itself
- The remaining MSH fields (MSH-3 onward) follow normal counting: MSH-3 is the Sending Application, MSH-4 is the Sending Facility, etc.
- When parsing MSH, field counting is offset by one compared to other segments because MSH-1 occupies the delimiter position

For example, in: `MSH|^~\&|SendApp|SendFac|RecvApp|RecvFac|20230101||ADT^A01|123|P|2.5`
- MSH-1 = `|` (field separator)
- MSH-2 = `^~\&` (encoding characters)
- MSH-3 = `SendApp`
- MSH-9 = `ADT^A01` (message type)

### Workspace Dependencies

All shared dependencies are defined in the root `Cargo.toml` under `[workspace.dependencies]`. When adding dependencies to individual crates, use `dep.workspace = true` to reference the workspace version.

### Schema Organization

Validation schemas are JSON files organized by HL7 version:
```
crates/vyna-hl7-validator/schemas/
├── v2_3/
├── v2_4/
├── v2_5/
├── v2_6/
├── v2_7/
└── v2_8/
```

Each contains message-specific schemas (e.g., `ADT_A01.json`, `ORU_R01.json`).

### Performance Considerations

The library is optimized for high-throughput processing:
- Zero-copy parsing with minimal allocations
- CachedTerser for repeated field access (5-10x faster)
- Pre-allocation strategies in parser
- Typical performance: 40,000-100,000 messages/sec depending on size

When making changes that could affect performance, run benchmarks before and after.

### Error Handling

Errors use `thiserror` for consistent error types:
- `vyna_hl7_core::Error` - Core errors (parsing, encoding)
- `vyna_hl7_validator::ValidationError` - Validation-specific errors
- Results use `vyna_hl7_core::Result<T>` type alias

### Testing Patterns

Tests are co-located with source code in `#[cfg(test)] mod tests` blocks. Property-based testing uses `proptest` for data structure validation.

#### Integration Testing with Mock Servers

**MockMllpServer** (vyna-hl7-mllp):
```rust
use vyna_hl7_mllp::testing::MockMllpServer;
use vyna_hl7_mllp::MllpClient;

#[tokio::test]
async fn test_mllp_with_mock_server() {
    // Start mock server with automatic port allocation
    let server = MockMllpServer::new()
        .with_handler(|msg| {
            // Custom message processing
            Ok(create_ack(&msg))
        })
        .start()
        .await
        .unwrap();

    // Connect client to mock server
    let mut client = MllpClient::connect(&server.url()).await.unwrap();
    let ack = client.send_message(&test_message).await.unwrap();

    // Assertions...
    assert_eq!(ack.segments[0].id, "MSH");

    // Automatic cleanup on drop
    server.shutdown().await.unwrap();
}
```

**MockHttpServer** (vyna-hl7-http):
```rust
use vyna_hl7_http::testing::MockHttpServer;
use vyna_hl7_http::HttpClient;

#[tokio::test]
async fn test_http_with_mock_server() {
    let server = MockHttpServer::new()
        .with_auth("user".to_string(), "pass".to_string())
        .start()
        .await
        .unwrap();

    let client = HttpClient::new(&server.url())
        .unwrap()
        .with_auth("user".to_string(), "pass".to_string());

    let ack = client.send_message(&test_message).await.unwrap();
    // Assertions...

    server.shutdown().await.unwrap();
}
```

#### TLS Integration Tests

MLLP TLS integration tests are in `crates/vyna-hl7-mllp/tests/tls_integration.rs`. Test certificate generation utilities automatically create X.509 v3 certificates with proper extensions for rustls compatibility.

**Running TLS integration tests:**
```bash
# Requires openssl to be installed
cargo test -p vyna-hl7-mllp --features "tls,testing" --test tls_integration
```

**Test Certificate Generation:**
```rust
// Located in tests/test_certs.rs module
mod test_certs;

#[tokio::test]
async fn test_with_tls() {
    // Automatic certificate generation with cleanup
    let certs = test_certs::generate_test_certs().await;

    let server_config = TlsServerConfig::new(
        &certs.server_cert_path,
        &certs.server_key_path
    )?;

    // Use certificates for testing...

    // Automatic cleanup via Drop trait
    certs.cleanup();
}
```

#### Testing Best Practices

1. **Use mock servers for integration tests** instead of external dependencies
2. **Enable features explicitly** when testing: `--features "tls,testing"`
3. **Test both plain and TLS configurations** for network transports
4. **Use automatic port allocation** (bind to "127.0.0.1:0") for test isolation
5. **Clean up resources** with `.shutdown()` or rely on Drop trait
6. **Test negative cases** (e.g., connection refused without proper CA)
7. **Test concurrent connections** to verify thread safety
8. **Use test certificates** generated dynamically (don't commit certificates to repo)

#### Feature Flags for Testing

- `testing`: Enables MockMllpServer and MockHttpServer
- `tls`: Enables TLS/mTLS support for testing secure connections
- Combine features: `cargo test --features "tls,testing"`

## Network Protocols

### MLLP (Minimal Lower Layer Protocol)
Intra-organization communication on TCP. MLLP frames messages with:
- Start byte: 0x0B (VT)
- End bytes: 0x1C 0x0D (FS CR)

Implementation in `vyna-hl7-mllp` uses Tokio for async I/O.

**TLS/mTLS Support:**
- `TlsServerConfig::new()` - Basic TLS with server certificate
- `TlsServerConfig::with_mtls()` - Mutual TLS with client certificate verification
- `TlsClientConfig::with_ca_cert()` - Client with CA certificate
- `TlsClientConfig::with_mtls()` - Client with client certificate for mTLS
- Feature flag: `tls`

### HTTP Transport
Inter-organization communication using HL7-over-HTTP. Implementation in `vyna-hl7-http` supports:
- Basic authentication
- Configurable timeouts
- Automatic ACK generation

**TLS/mTLS Support:**
- `HttpServer::with_tls()` and `.serve_tls()` for HTTPS
- `HttpClient::new_tls()` for HTTPS clients
- Both basic TLS and mutual TLS (client certificate) supported
- Feature flag: `tls`

## FHIR Conversion

Located in `vyna-hl7-fhir`, provides converters from HL7 v2 to FHIR R4:
- Patient (PID → Patient)
- Observation (OBX → Observation)
- Encounter (PV1 → Encounter)
- DiagnosticReport (OBR → DiagnosticReport)
- Practitioner (PV1/ORC → Practitioner)
- AllergyIntolerance (AL1 → AllergyIntolerance)
- MedicationAdministration (RXA → MedicationAdministration)
- Condition (PRB/DG1 → Condition)
- Procedure (PR1 → Procedure)

Each converter is in a separate module and uses the standard FHIR R4 resource structure.

## Edition and Toolchain

This project uses Rust edition 2024. Ensure your toolchain is up to date:
```bash
rustup update
```

Required: Rust 1.91.0 or later
