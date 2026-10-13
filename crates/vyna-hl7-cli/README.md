# VynaHL7 CLI - Command-Line Interface for HL7 v2.x Messages

A powerful command-line tool for parsing, validating, and analyzing HL7 v2.x messages.

## Installation

```bash
cargo install --path crates/vyna-hl7-cli

# Or with FHIR conversion support:
cargo install --path crates/vyna-hl7-cli --features fhir
```

## Usage

```bash
vynahl7 <COMMAND> [OPTIONS]
```

## Commands

### Parse - Parse and display HL7 messages

Parse an HL7 message and display its structure:

```bash
# Parse from file (text format)
vynahl7 parse message.hl7

# Parse from stdin
cat message.hl7 | vynahl7 parse -

# JSON output
vynahl7 parse message.hl7 --format json

# Pretty formatted output with colors
vynahl7 parse message.hl7 --format pretty

# Show detailed segment information
vynahl7 parse message.hl7 --detailed
```

**Example Output (text format):**
```
HL7 Message Parsed Successfully
Version: V2_5
Message Type: ADT^A01
Control ID: MSG123
Segments: 4
```

**Example Output (pretty format with `--detailed`):**
```
Message Structure:
  Version: V2_5
  Type: ADT^A01
  Control ID: MSG123
  Segments: 4

  1. MSH (Message Header) - 12 fields
      MSH-1 Field Separator (ST): |
      MSH-2 Encoding Characters (ST): ^~\&
      MSH-3 Sending Application (HD): SendApp
      ...
  2. EVN (Event Type) - 2 fields
      EVN-1 Event Type Code (ID): A01
      ...
```

### Validate - Validate messages against HL7 standards

Validate an HL7 message:

```bash
# Validate with auto-detected version
vynahl7 validate message.hl7

# Validate with specific version
vynahl7 validate message.hl7 --hl7-version 2.5

# JSON output
vynahl7 validate message.hl7 --format json
```

**Example Output:**
```
✓ Message is valid
Validation mode: strict (HL7 specification compliance)

No issues found.
```

**With errors:**
```
✗ Message validation failed
Validation mode: strict (HL7 specification compliance)

Errors:
  • MSH-9 - Missing required field: Message Type

Warnings:
  • PID-8 - Deprecated code used in Administrative Sex
```

Validation is strict by default. `--standard` downgrades version drift
to a warning; `--lenient` fails only on structural problems. The exit
code is 1 when the message is invalid, so scripts can rely on it.

### Extract - Extract field values using Terser paths

Extract specific fields from a message:

```bash
# Extract single field
vynahl7 extract message.hl7 PID-5

# Extract multiple fields
vynahl7 extract message.hl7 PID-5 PID-3 MSH-10

# Extract with component indexing (1-based)
vynahl7 extract message.hl7 "PID-5-1" "PID-5-2"

# Extract from indexed segments (1-based)
vynahl7 extract message.hl7 "OBX(1)-5" "OBX(2)-5"

# JSON output
vynahl7 extract message.hl7 PID-5 PID-3 --format json
```

**Example Output:**
```
PID-5: DOE
PID-3: MRN123
MSH-10: MSG123
```

A field-level path returns the field's first component; use component
paths (e.g. `PID-5-1`, `PID-5-2`) to address the other parts.

**Terser Path Syntax:**
- `SEG-field`: Field in first occurrence of segment (e.g., `PID-5`)
- `SEG(index)-field`: Field in indexed segment, 1-based (e.g., `OBX(1)-5` for first OBX, `OBX(2)-5` for second)
- `SEG-field-component`: Component within field (e.g., `PID-5-1` for family name)
- `SEG-field-component-subcomponent`: Subcomponent (e.g., `PID-11-1-1`)

**Note:** Segment and component indexing is 1-based (matching HL7 standards):
- `PID-5-1` = Family name (first component)
- `PID-5-2` = Given name (second component)
- `PID-5-3` = Middle name (third component)

A malformed path is reported on stderr and exits 1. A well-formed path
with no value prints `N/A` (empty string with `--format json`) and exits 0.

### Convert - Convert messages to different formats

Convert HL7 messages to JSON or FHIR:

```bash
# Convert to JSON
vynahl7 convert message.hl7 --to json

# Convert to JSON (pretty-printed)
vynahl7 convert message.hl7 --to json --pretty

# Convert to FHIR R4 (requires 'fhir' feature)
vynahl7 convert message.hl7 --to fhir --pretty
```

**Example JSON Output:**
```json
{
  "version": "V2_5",
  "message_type": "ADT^A01",
  "control_id": "MSG123",
  "sending_application": "SendApp",
  "segments": [
    {
      "id": "MSH",
      "fields": ["|", "^~\\&", "SendApp", ...]
    },
    ...
  ]
}
```

### Info - Display comprehensive message information

Show detailed message information:

```bash
vynahl7 info message.hl7
```

**Example Output:**
```
HL7 Message Information
==================================================

Header Information:
  Version:              V2_5
  Message Type:         ADT^A01
  Control ID:           MSG123
  Sending Application:  SendApp
  Sending Facility:     Hospital
  Receiving Application: RecvApp
  Receiving Facility:   Lab

Message Structure:
  Total Segments:       8
  Segment Types:        5

  Segment Breakdown:
    1 x MSH
    1 x EVN
    1 x PID
    2 x OBX
    1 x PV1

Size Information:
  Encoded Size:         1024 bytes
  Average Segment Size: 128 bytes
```

### Send - Send a message and print the acknowledgment

Send a message over MLLP or HTTP and print the ACK:

```bash
# Send over MLLP (intra-organization)
vynahl7 send message.hl7 --to mllp://127.0.0.1:2575

# Send over HTTP with basic auth (inter-organization)
vynahl7 send message.hl7 --to http://127.0.0.1:8080/hl7 --user u --password p

# MLLP with TLS (requires the 'tls' feature and a CA certificate)
vynahl7 send message.hl7 --to mllp://127.0.0.1:2575 --tls --ca-cert ca.pem
```

A negative acknowledgment (anything but `AA`/`CA` in MSA-1) exits 1,
so scripts can rely on the exit code.

### Diff - Show structural differences between two messages

Compare two messages field by field:

```bash
# Text output (exit 1 when the messages differ)
vynahl7 diff old.hl7 new.hl7

# JSON output
vynahl7 diff old.hl7 new.hl7 --format json

# Ignore volatile fields
vynahl7 diff old.hl7 new.hl7 --ignore MSH-7,MSH-10
```

**Example Output:**
```
~ PID-5-2: JOHN -> JANE
```

## Input Methods

All commands support reading from files or standard input:

```bash
# From file
vynahl7 parse message.hl7

# From stdin
cat message.hl7 | vynahl7 parse -
echo "MSH|^~\&|..." | vynahl7 parse -

# Pipe from other commands
curl http://example.com/message.hl7 | vynahl7 validate -
```

## Output Formats

### Text Format (default)
Human-readable output with colors (when terminal supports it)

### JSON Format
Machine-readable JSON output for programmatic processing:

```bash
vynahl7 parse message.hl7 --format json | jq '.segment_count'
vynahl7 validate message.hl7 --format json | jq '.errors[]'
vynahl7 extract message.hl7 PID-5 --format json
```

### Pretty Format
Enhanced text output with colors and formatting (parse command only):

```bash
vynahl7 parse message.hl7 --format pretty --detailed
```

## Examples

### Basic Workflow

```bash
# 1. Parse and check structure
vynahl7 parse sample.hl7 --format pretty

# 2. Validate against standards
vynahl7 validate sample.hl7

# 3. Extract patient demographics
vynahl7 extract sample.hl7 PID-5 PID-7 PID-8

# 4. Get full message info
vynahl7 info sample.hl7
```

### Processing Multiple Messages

```bash
# Validate all messages in a directory
for f in messages/*.hl7; do
  echo "Validating $f"
  vynahl7 validate "$f"
done

# Extract patient names from all messages
for f in messages/*.hl7; do
  vynahl7 extract "$f" PID-5 --format json
done | jq -s '.'
```

### Integration with Other Tools

```bash
# Use with jq for JSON processing
vynahl7 convert message.hl7 --to json --pretty | jq '.segments[] | select(.id == "PID")'

# Count segment types
vynahl7 parse message.hl7 --format json | jq '.segments[].id' | sort | uniq -c

# Find all observation results (1-based segment indexing)
vynahl7 extract message.hl7 "OBX(1)-5" "OBX(2)-5" "OBX(3)-5"
```

### FHIR Conversion

```bash
# Convert ADT message to FHIR
vynahl7 convert adt_a01.hl7 --to fhir --pretty > patient.json

# Convert ORU message to FHIR
vynahl7 convert oru_r01.hl7 --to fhir --pretty > observations.json
```

## Sample HL7 Message

Create a test message:

```bash
cat > sample.hl7 << 'EOF'
MSH|^~\&|SendApp|SendFac|RecvApp|RecvFac|20250108120000||ADT^A01|MSG123|P|2.5
EVN|A01|20250108120000
PID|1||MRN123||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||555-1234
PV1|1|I|WARD^101^1
EOF

vynahl7 parse sample.hl7 --format pretty
```

## Error Handling

The CLI returns appropriate exit codes:
- `0` - Success
- `1` - Error (parse failure, validation failure, file not found, etc.)

```bash
vynahl7 validate message.hl7
if [ $? -eq 0 ]; then
  echo "Valid message"
else
  echo "Invalid message"
fi
```

## Building from Source

```bash
# Standard build
cargo build --release --package vyna-hl7-cli

# With FHIR support
cargo build --release --package vyna-hl7-cli --features fhir

# The binary will be at: target/release/vynahl7
```

## Features

- **Default**: Core parsing, validation, and extraction
- **fhir**: FHIR R4 conversion support
- **tls**: TLS/mTLS support for `send`

## Performance

The CLI is optimized for speed:
- Parses small messages (ADT, 3 segments) in ~16 microseconds (~60,000 msg/sec)
- Minimal memory footprint
- Suitable for high-throughput batch processing

See the [performance guide](../../docs/performance.md) for full benchmarks.

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.

## Related Projects

- [vyna-hl7](https://github.com/FlostelInfoTech/vynahl7) - The core HL7 v2.x library for Rust
- [vyna-hl7-wasm](../vyna-hl7-wasm) - WebAssembly bindings for browser/Node.js
