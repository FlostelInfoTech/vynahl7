# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

VynaHL7 is a comprehensive Rust library for parsing, validating, and creating HL7 v2.x healthcare messages, inspired by the Java HAPI library. The project uses a workspace architecture with 23 specialized crates.

Specs live in `docs/` and are the authority — read them before designing:

- `docs/requirements.md` — numbered requirements with verification pointers
- `docs/design.md` — architecture, crate catalog, component designs, key decisions
- `docs/integration.md` — install, quickstarts, transports, troubleshooting
- `docs/sbom/` — supply-chain record (regenerate when dependencies change)
- `crates/vyna-hl7-validator/schemas/README.md` — schema provenance and layout

Living-doc rule: code changes update the matching spec in the same commit
(new builder → SRS ID + SDS + `builder_conformance.rs` test; behavior
change → CHANGELOG entry). Full rules: `docs/README.md`.

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
vynahl7 validate message.hl7            # strict: HL7 specification compliance (default)
vynahl7 validate message.hl7 --strict   # restates the default explicitly
vynahl7 validate message.hl7 --standard # strict except version drift warns
vynahl7 validate message.hl7 --lenient  # only structural problems fail, rest warns
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

When making changes that could affect performance, run benchmarks before and after (see `docs/performance.md`).

### WebAssembly
```bash
# Build WASM package
cd crates/vyna-hl7-wasm
wasm-pack build --target web
wasm-pack build --target nodejs
```

## Gotchas (read before touching message code)

### Terser Path Notation

**Terser path indexing is 1-based throughout** (e.g., `OBX(1)` for first OBX, `PID-11(1)` for first repetition), matching HAPI conventions. Component `0` and repetition `(0)` are invalid; only the internal `Vec` storage is 0-based.

Path examples: `PID-5`, `PID-5-1`, `PID-5-1-2`, `OBX(1)-5`, `PID-11(2)-1` (second repetition).

### MSH Segment Field Numbering

**MSH-1 is the Field Separator character itself** (typically `|`), and MSH-2 contains the four Encoding Characters (typically `^~\&`). Field counting is offset by one compared to other segments: in `MSH|^~\&|SendApp|...`, MSH-3 is `SendApp` and MSH-9 is the message type.

### Builder Conformance

Builder conformance tests live in `vyna-hl7-validator/tests/builder_conformance.rs` — never in core (core must not depend on the validator). Composite builder inputs go through `Field::from_delimited` (structure); `Field::from_value` is literal-only (delimiters escaped). Free-text fields (NTE, descriptions) always stay literal.

### Validation Posture

Validation is strict by default (every finding class errors, including MSH-12 drift). `standard()`/`lenient()` contexts and `table_registry_mut` site-table extensions are the explicit escape hatches. The parser never validates; only the `validate` command and Validator API do.

## Repository Conventions

### Workspace Dependencies

All shared dependencies are defined in the root `Cargo.toml` under `[workspace.dependencies]`. When adding dependencies to individual crates, use `dep.workspace = true` to reference the workspace version.

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

**MockHttpServer** (vyna-hl7-http, requires `--features "testing,auth"` for `with_auth`):
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

    // Explicit cleanup removes the temp dir
    certs.cleanup();
}
```

HTTP mock-server TLS is covered by `test_mock_server_with_tls_serves_https` (in `vyna-hl7-http/src/testing.rs`, needs `--features "tls,testing"` plus `openssl` for cert generation).

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
- `auth` (http): Enables server-side Basic auth (`HttpServer::with_auth`, `MockHttpServer::with_auth`)
- Combine features: `cargo test --features "tls,testing,auth"`

## Edition and Toolchain

This project uses Rust edition 2024. Ensure your toolchain is up to date:
```bash
rustup update
```

Required: Rust 1.91.0 or later
