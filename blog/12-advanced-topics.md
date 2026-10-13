# Advanced Topics: CLI Tool, WebAssembly, and the Future

*Part 12 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./11-real-world-use-cases.md), we explored real-world integration scenarios. In this final post, we'll cover advanced features: the CLI tool for quick analysis, WebAssembly for browser-based applications, and what's coming next.

## The VynaHL7 CLI Tool

The `vyna-hl7-cli` crate provides a command-line tool for quick message analysis and manipulation—perfect for debugging, testing, and ad-hoc tasks.

### Installation

```bash
# Install from source
cargo install --path crates/vyna-hl7-cli

# Or from crates.io (when published)
cargo install vyna-hl7-cli
```

### Parsing Messages

View a parsed message in a human-readable format:

```bash
# Parse and display
vynahl7 parse message.hl7

# Pretty-print with indentation
vynahl7 parse message.hl7 --format pretty

# Output as JSON
vynahl7 parse message.hl7 --format json
```

Example output:
```
=== HL7 Message ===
Type: ADT^A01
Control ID: MSG001
Version: 2.5
Segments: 3

MSH
  1: |
  2: ^~\&
  3: SendingApp
  4: SendingFac
  5: ReceivingApp
  6: ReceivingFac
  7: 20240315143000
  9: ADT^A01
  10: MSG001
  11: P
  12: 2.5

PID
  1: 1
  3: 123456^^^MRN
  5: DOE^JOHN^M
  7: 19800515
  8: M

PV1
  1: 1
  2: I
  3: ICU^101^A
```

### Extracting Fields

Extract specific fields using Terser notation:

```bash
# Extract single field
vynahl7 extract message.hl7 PID-5-1
# Output: DOE

# Extract multiple fields
vynahl7 extract message.hl7 PID-5-1 PID-5-2 PID-7 PID-8
# Output:
# PID-5-1: DOE
# PID-5-2: JOHN
# PID-7: 19800515
# PID-8: M

# Extract from all OBX segments
vynahl7 extract results.hl7 "OBX(*)-3-1" "OBX(*)-5"
```

### Validating Messages

Validate against HL7 standards:

```bash
# Basic validation
vynahl7 validate message.hl7

# Specify version
vynahl7 validate message.hl7 --version 2.5

# Strict mode
vynahl7 validate message.hl7 --strict

# Output format
vynahl7 validate message.hl7 --format json
```

Example output:
```
Validating message.hl7...

Errors:
  - MSH-7: Invalid date format "20241301" (invalid month)
  - PID-8: Value "X" not in table 0001 (Administrative Sex)

Warnings:
  - PV1-7: Attending doctor name is empty

Result: INVALID (2 errors, 1 warning)
```

### Converting Formats

Convert between formats:

```bash
# To JSON
vynahl7 convert message.hl7 --to json --pretty

# To FHIR Bundle
vynahl7 convert message.hl7 --to fhir --output bundle.json

# Re-encode (normalize)
vynahl7 convert message.hl7 --to hl7 --output normalized.hl7
```

### Message Information

Quick summary of a message:

```bash
vynahl7 info message.hl7
```

Output:
```
=== Message Summary ===
File: message.hl7
Type: ADT^A01 (Admit/Visit Notification)
Version: 2.5
Control ID: MSG001
Timestamp: 2024-03-15 14:30:00

Sending: MyApp @ MyFacility
Receiving: TheirApp @ TheirFacility

Segments: MSH, PID, PV1
Total segments: 3

Patient:
  MRN: 123456
  Name: JOHN DOE
  DOB: 1980-05-15
  Sex: Male
```

### Batch Processing

Process multiple files:

```bash
# Validate all files in directory
vynahl7 validate ./messages/*.hl7 --summary

# Extract from multiple files
vynahl7 extract ./messages/*.hl7 PID-3-1 PID-5 --csv > patients.csv

# Convert all to JSON
for f in ./messages/*.hl7; do
    vynahl7 convert "$f" --to json > "${f%.hl7}.json"
done
```

## WebAssembly Support

VynaHL7 compiles to WebAssembly, enabling HL7 parsing directly in web browsers—useful for:
- Patient portal message display
- Browser-based HL7 validators
- Educational tools
- Integration testing UIs

### Building the WASM Package

```bash
cd crates/vyna-hl7-wasm

# For web browsers
wasm-pack build --target web

# For Node.js
wasm-pack build --target nodejs

# Optimized release build
wasm-pack build --target web --release
```

### Using in JavaScript

```html
<!DOCTYPE html>
<html>
<head>
    <title>HL7 Parser</title>
</head>
<body>
    <textarea id="hl7Input" rows="10" cols="80">
MSH|^~\&|APP|FAC|RECV|DEST|20240315||ADT^A01|MSG001|P|2.5
PID|1||123456||DOE^JOHN||19800515|M
    </textarea>
    <button onclick="parseMessage()">Parse</button>
    <pre id="output"></pre>

    <script type="module">
        import init, { parseMessage, getTerserValue, encodeMessage } from './pkg/vyna_hl7_wasm.js';

        await init();

        window.parseMessage = function() {
            const hl7 = document.getElementById('hl7Input').value;

            try {
                const message = parseMessage(hl7);

                document.getElementById('output').textContent =
                    encodeMessage(message);

                // Extract specific fields
                const name = getTerserValue(message, "PID-5-1");
                console.log("Patient name:", name);
            } catch (e) {
                document.getElementById('output').textContent = "Error: " + e;
            }
        };
    </script>
</body>
</html>
```

### React Component Example

```jsx
import { useEffect, useState } from 'react';
import init, { parseMessage, validateMessage, getTerserValue, encodeMessage } from 'vyna-hl7-wasm';

function HL7Viewer({ message }) {
    const [parsed, setParsed] = useState(null);
    const [validation, setValidation] = useState(null);
    const [wasmReady, setWasmReady] = useState(false);

    useEffect(() => {
        init().then(() => setWasmReady(true));
    }, []);

    useEffect(() => {
        if (wasmReady && message) {
            try {
                const msg = parseMessage(message);
                setParsed({
                    encoded: encodeMessage(msg),
                    type: getTerserValue(msg, "MSH-9"),
                    controlId: getTerserValue(msg, "MSH-10"),
                    familyName: getTerserValue(msg, "PID-5-1"),
                    givenName: getTerserValue(msg, "PID-5-2"),
                    mrn: getTerserValue(msg, "PID-3"),
                    dob: getTerserValue(msg, "PID-7"),
                });

                const result = validateMessage(msg);
                setValidation({ isValid: result.isValid(), json: result.toJson() });
            } catch (e) {
                console.error("Parse error:", e);
            }
        }
    }, [wasmReady, message]);

    if (!wasmReady) return <div>Loading...</div>;
    if (!parsed) return <div>No message</div>;

    return (
        <div className="hl7-viewer">
            <div className="header">
                <span>Type: {parsed.type}</span>
                <span>Control ID: {parsed.controlId}</span>
            </div>

            {validation && !validation.isValid && (
                <div className="errors">
                    {(validation.json.errors || []).map((err, i) => (
                        <div key={i} className="error">
                            {err.location}: {err.message}
                        </div>
                    ))}
                </div>
            )}

            <div className="patient-info">
                <h3>Patient</h3>
                <p>Name: {parsed.givenName} {parsed.familyName}</p>
                <p>MRN: {parsed.mrn}</p>
                <p>DOB: {parsed.dob}</p>
            </div>

            <details>
                <summary>Raw Parsed Data</summary>
                <pre>{parsed.encoded}</pre>
            </details>
        </div>
    );
}
```

### WASM API Reference

```typescript
// TypeScript definitions (generated by wasm-pack)

export class WasmMessage {
    messageType(): string | undefined;
    sendingApplication(): string | undefined;
    sendingFacility(): string | undefined;
    receivingApplication(): string | undefined;
    receivingFacility(): string | undefined;
    controlId(): string | undefined;
}

export class WasmValidationResult {
    isValid(): boolean;
    errorCount(): number;
    warningCount(): number;
    toJson(): any;
}

/**
 * Parse an HL7 message; throws on invalid input
 */
export function parseMessage(input: string): WasmMessage;

/**
 * Re-encode a message to ER7
 */
export function encodeMessage(message: WasmMessage): string;

/**
 * HL7 version (MSH-12) and message type (MSH-9)
 */
export function getVersion(message: WasmMessage): string | undefined;
export function getMessageType(message: WasmMessage): string | undefined;

/**
 * Get field value(s) using Terser path notation
 */
export function getTerserValue(message: WasmMessage, path: string): string | undefined;
export function getTerserValues(message: WasmMessage, paths: string[]): any;

/**
 * Set a field value in place
 */
export function setTerserValue(message: WasmMessage, path: string, value: string): void;

/**
 * Validate a message against its version schema
 */
export function validateMessage(message: WasmMessage): WasmValidationResult;

/**
 * Create a message with an MSH segment
 */
export function createMessage(version: string, messageType: string, sendingApp: string, sendingFac: string): WasmMessage;

/**
 * Convenience extractors
 */
export function extractPatientDemographics(message: WasmMessage): any;
export function extractObservations(message: WasmMessage): any;
```

## Performance Benchmarks

VynaHL7's performance characteristics:

### Parsing Performance

| Message Size | Messages/sec | Parse time |
|--------------|--------------|---------------|
| Small ADT (3 segments) | ~60,000 | ~16 µs |
| Medium ORU (13 segments) | ~19,000 | ~52 µs |

See `docs/performance.md` for the measured benchmark tables.

### Terser Performance

| Operation | Regular Terser | CachedTerser |
|-----------|----------------|--------------|
| Repeated lookup (10 paths x 5) | ~80 ns | ~60 ns (~1.4x faster) |

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --workspace

# Benchmark specific crate
cargo bench -p vyna-hl7-parser
cargo bench -p vyna-hl7-terser

# With detailed output
cargo bench -- --verbose
```

## Extending VynaHL7

### Custom Segment Handlers

```rust
use vyna_hl7_core::{Message, Segment};

trait SegmentHandler {
    fn segment_id(&self) -> &str;
    fn process(&self, segment: &Segment) -> Result<(), Box<dyn std::error::Error>>;
}

struct ZCustomHandler;

impl SegmentHandler for ZCustomHandler {
    fn segment_id(&self) -> &str { "ZCU" }

    fn process(&self, segment: &Segment) -> Result<(), Box<dyn std::error::Error>> {
        // Handle custom Z-segment
        let custom_field = segment.get_field_value(1);
        println!("Custom data: {:?}", custom_field);
        Ok(())
    }
}
```

### Custom Validators

```rust
use vyna_hl7_validator::{ValidationResult, ValidationIssue, Severity};

fn validate_organization_rules(message: &Message) -> ValidationResult {
    let mut result = ValidationResult::new();
    let terser = Terser::new(message);

    // Rule: MRN must be 10 digits
    if let Ok(Some(mrn)) = terser.get("PID-3-1") {
        if !mrn.chars().all(|c| c.is_ascii_digit()) || mrn.len() != 10 {
            result.errors.push(ValidationIssue {
                location: "PID-3-1".into(),
                message: "MRN must be exactly 10 digits".into(),
                severity: Severity::Error,
                code: "ORG_MRN_FORMAT".into(),
            });
        }
    }

    // Rule: Attending physician required for inpatients
    if let Ok(Some("I")) = terser.get("PV1-2") {
        if terser.get("PV1-7").ok().flatten().is_none() {
            result.errors.push(ValidationIssue {
                location: "PV1-7".into(),
                message: "Attending physician required for inpatients".into(),
                severity: Severity::Error,
                code: "ORG_ATTENDING_REQUIRED".into(),
            });
        }
    }

    result
}
```

### Custom FHIR Converters

```rust
use vyna_hl7_core::Message;
use serde_json::Value as JsonValue;

trait FhirConverter {
    fn resource_type(&self) -> &str;
    fn convert(&self, message: &Message) -> Result<JsonValue, Box<dyn std::error::Error>>;
}

struct CustomResourceConverter;

impl FhirConverter for CustomResourceConverter {
    fn resource_type(&self) -> &str { "CustomResource" }

    fn convert(&self, message: &Message) -> Result<JsonValue, Box<dyn std::error::Error>> {
        let terser = Terser::new(message);

        Ok(serde_json::json!({
            "resourceType": "CustomResource",
            "customField": terser.get("ZCU-1")?.unwrap_or_default()
        }))
    }
}
```

## Roadmap and Future Features

### Coming Soon

- **Conformance profiles** - IHE profile validation
- **Message templates** - Create messages from templates
- **Batch file support** - Process HL7 batch files (FHS/BHS)
- **Additional FHIR converters** - Immunization, Appointment, etc.

### Under Consideration

- **HL7 v2.9 support** - Latest standard version
- **CDA generation** - Create Clinical Document Architecture from HL7 v2
- **Async validation** - Parallel validation for high throughput
- **GraphQL API** - Query HL7 messages using GraphQL

## Contributing to VynaHL7

VynaHL7 is open source and welcomes contributions:

1. **Report issues** - Bug reports and feature requests
2. **Submit PRs** - Code contributions
3. **Improve docs** - Documentation and examples
4. **Share use cases** - Help others learn

### Development Setup

```bash
git clone https://github.com/FlostelInfoTech/vyna-hl7
cd vyna-hl7

# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Run examples
cargo run --example parse_adt
```

### Code Style

- Follow Rust standard formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy`)
- Include tests for new features
- Update documentation

## Conclusion

Over this 12-part series, we've explored VynaHL7 comprehensively:

1. **Why HL7 v2 matters** and VynaHL7's design philosophy
2. **Getting started** with parsing and message creation
3. **Message structure** and VynaHL7's data model
4. **Terser API** for elegant field access
5. **Validation** for compliance checking
6. **Network transport** with MLLP and HTTP
7. **FHIR conversion** for modern interoperability
8. **Production patterns** for reliable systems
9. **Real-world use cases** across healthcare workflows
10. **Advanced topics** including CLI and WASM

VynaHL7 provides a complete toolkit for HL7 v2 integration in Rust, combining safety, performance, and developer ergonomics. Whether you're building a simple integration or a complex healthcare data platform, VynaHL7 has the tools you need.

Thank you for following this series. We hope it helps you build better healthcare integrations!

---

*This concludes the VynaHL7 blog series.*

*For questions, issues, or contributions, visit [GitHub](https://github.com/FlostelInfoTech/vyna-hl7).*

---

## Series Index

1. [Why HL7 v2 Integration Still Matters](./01-why-hl7-v2-integration-still-matters.md)
2. [VynaHL7 Test Panel: Your Interactive HL7 Workbench](./02-vyna-hl7-test-panel-your-hl7-workbench.md)
3. [Getting Started with VynaHL7](./03-getting-started-with-vyna-hl7.md)
4. [Understanding HL7 v2 Messages](./04-understanding-hl7-v2-messages.md)
5. [The Terser API](./05-the-terser-api.md)
6. [Message Validation](./06-message-validation.md)
7. [Custom Z-Segments](./07-custom-z-segments.md)
8. [Network Transport](./08-network-transport.md)
9. [HL7 v2 to FHIR Conversion](./09-hl7-to-fhir-conversion.md)
10. [Building Production-Ready Integrations](./10-production-ready-integrations.md)
11. [Real-World Use Cases](./11-real-world-use-cases.md)
12. [Advanced Topics](./12-advanced-topics.md) (this post)
