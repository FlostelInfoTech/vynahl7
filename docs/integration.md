# Integration Guide — VynaHL7

How to embed VynaHL7 (0.9.0) in a Rust service. Every Rust snippet below
is executed as a test before release; shell snippets mirror the CLI's
own integration tests.

## 1. Install

```toml
[dependencies]
vyna-hl7-core = "0.9"
vyna-hl7-parser = "0.9"
vyna-hl7-validator = "0.9"
vyna-hl7-terser = "0.9"
# Transports as needed:
vyna-hl7-mllp = "0.9"   # intra-organization, Tokio-based
vyna-hl7-http = "0.9"   # inter-organization, HL7-over-HTTP
```

Edition 2024, Rust 1.91 or later (`rust-version` 1.91 in every
manifest). Transports need Tokio at your binary (`tokio = { version =
"1", features = ["full"] }` covers the examples here). Feature flags:
`tls` (TLS/mTLS), `testing` (mock servers for your own integration
tests).

## 2. Parse and strictly validate

Parsing never validates; validation never blocks parsing. Validate
explicitly, with the schema auto-selected from MSH-9:

```rust
use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;
use vyna_hl7_validator::Validator;

let wire = "MSH|^~\\&|SEND|SFAC|RECV|RFAC|20240115120000||ADT^A01|MSG001|P|2.5\r\
            EVN||20240115120000\rPID|1||12345||Smith^John||19800101|M\rPV1|1|I\r";
let message = parse_message(wire)?;

// Strict by default: a clean result means HL7 specification compliance.
let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01")?;
let result = validator.validate(&message);
if !result.is_valid() {
    for error in &result.errors {
        eprintln!("{}: {}", error.location, error.message);
    }
}
```

Relax severity explicitly when the feed requires it — never silently:

```rust
use vyna_hl7_validator::ValidationContext;

// Pinned-version feeds whose senders drift in MSH-12:
let standard = Validator::new(Version::V2_5)
    .with_context(ValidationContext::standard());
// Known-messy feeds that must still flow (only structural failures):
let lenient = Validator::new(Version::V2_5)
    .with_context(ValidationContext::lenient());
```

## 3. Read fields with the Terser

```rust
use vyna_hl7_terser::Terser;

let terser = Terser::new(&message);
let family: Option<&str> = terser.get("PID-5-1")?;
let given: Option<&str> = terser.get("PID-5-2")?;
// Second OBX, observation value:
let value: Option<&str> = terser.get("OBX(2)-5")?;
```

Paths are 1-based (`OBX(1)` is the first OBX). For hot loops over the
same paths, use `CachedTerser` (same API, faster repeats).

## 4. Build messages

```rust
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;

let message = AdtBuilder::a01(Version::V2_5)
    .sending_application("SEND")
    .sending_facility("SFAC")
    .receiving_application("RECV")
    .receiving_facility("RFAC")
    .patient_id("12345")
    .patient_name("Smith", "John")
    .patient_class("I")
    .build()?;
let wire = message.encode(); // valid ER7, ready to send
```

Builder families cover ADT, BAR, DFT, MDM, ORM, ORU, OML, OUL, OMP,
PPR, QRY, QBP, RSP, pharmacy (RDE/RAS/RDS/RGV/RRD/RRA), SIU, VXU, ACK,
and batch/file envelopes. Every message builder with a schema is
proven by the conformance suite (56 tests) to validate clean as built
and after a parse round-trip.

Composite values: pass delimited content (`"Smith^John"`,
`"08^HEPB^CVX"`) to builder setters — builders split it into real
structure. If you construct fields by hand, use `Field::from_delimited`
for composites and `Field::from_value` only for literal text (whose
delimiters must escape).

## 5. Send over MLLP or HTTP

```rust
// Intra-organization: MLLP over TCP.
use vyna_hl7_mllp::MllpClient;

let mut client = MllpClient::connect("127.0.0.1:2575").await?;
let ack: Message = client.send_message(&message).await?;
```

```rust
// Inter-organization: HL7-over-HTTP.
use vyna_hl7_http::HttpClient;

let client = HttpClient::new("http://127.0.0.1:8080/hl7")?
    .with_auth("user".to_string(), "pass".to_string());
let ack: Message = client.send_message(&message).await?;
```

TLS/mTLS variants exist behind the `tls` feature (`TlsServerConfig`,
`TlsClientConfig`, `HttpServer::serve_tls`, `HttpClient::with_tls`);
see `crates/vyna-hl7-http/examples/` and the MLLP `tls_integration`
tests. Basic auth on the server side needs the `auth` feature
(`HttpServer::with_auth` / `MockHttpServer::with_auth` only exist with
it). For your own tests, `MockMllpServer` / `MockHttpServer` (feature
`testing`) serve on automatic ports — the round-trips above run
against them in this guide's verification; `MockHttpServer::with_tls`
serves real HTTPS when the `tls` feature is enabled.

## 6. Extend code tables for site-local codes

Strict validation enforces standard tables; real feeds extend them
(e.g. NDC drug codes where v2.5 tables RXE-2 to 0292/CVX). Extend the
table — don't weaken the validator:

```rust
let mut validator = Validator::for_message_type(Version::V2_5, "RDE", "O11")?;
validator
    .table_registry_mut()
    .get_table_mut("0292")
    .unwrap()
    .add_value("00093-0058", "Lorazepam 1mg tablet (site NDC)", false);
// Standard CVX codes keep validating alongside the site code.
```

## 7. CLI and workbench

```bash
cargo install --path crates/vyna-hl7-cli

vynahl7 validate message.hl7            # strict (default)
vynahl7 validate message.hl7 --standard # drift warns
vynahl7 validate message.hl7 --lenient  # structure-only failures
vynahl7 validate message.hl7 --format json  # machine-readable incl. "mode"
vynahl7 extract message.hl7 PID-5 OBX\(2\)-5
vynahl7 send message.hl7 --to mllp://127.0.0.1:2575
vynahl7 diff old.hl7 new.hl7
vynahl7 --charset 8859/1 parse latin1.hl7  # force decoding, ignore MSH-18
```

Input decoding follows the MSH-18 declaration by default; the global
`--charset` flag overrides it (MSH-18 spellings such as `8859/1`,
`ASCII`, `UTF-8`) for peers that misdeclare their encoding, and applies
to every command.

The GUI test panel (`cargo run -p vyna-hl7-testpanel`) offers parser,
builder, terser, strict validator, FHIR, MLLP, HTTP, FTP, and XML tabs
with schema-valid sample messages.

## 8. Troubleshooting strict failures

Strict failures name a schema location and the rule. The common ones:

- `RXE-2` / `RXA-5` vs table 0292 with an NDC code: v2.5 tables these
  to CVX in every version. Either send CVX or register site NDC codes
  (section 6).
- `PR1-3` vs table 0088 with a procedure code (`93458`): 0088
  enumerates coding methods (`C4`, `HCPCS`, …); the identifier goes in
  component 1 per the enforced table.
- `*-3` vs table 0396 (`CPT`, `MS-DRG`, `HL7nnnn`): the coding-system
  component must be a 0396 system (`C4`, `I10`, `LN`, `CVX`, `L`, …).
- `MSH-12` version drift: you validated against a different version
  than the sender declared. Validate against the declared version, or
  use `--standard` / `ValidationContext::standard()` for pinned feeds.
- `Required segment/field … missing`: the message genuinely lacks
  schema-required content — check the segment exists and is non-empty
  (empty means absent).
