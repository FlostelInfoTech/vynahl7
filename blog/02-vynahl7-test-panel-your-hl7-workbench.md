# VynaHL7 Test Panel: Your Interactive HL7 Workbench

*Part 2 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./01-why-hl7-v2-integration-still-matters.md), we explored why HL7 v2 integration remains essential. Before diving into code, let's introduce the **VynaHL7 Test Panel**—a powerful GUI application that lets you work with HL7 messages interactively.

Whether you're debugging an integration, learning HL7 v2 structure, or building messages for testing, the Test Panel provides an all-in-one environment for healthcare integration developers.

## Why a GUI Tool?

HL7 v2 messages can be challenging to work with:

- **Pipe-delimited format** is hard to read in raw form
- **Nested structure** (segments → fields → components → subcomponents) requires mental parsing
- **Testing integrations** requires sending messages over MLLP/HTTP
- **Validating messages** against HL7 standards needs schema knowledge
- **Converting to FHIR** involves complex mapping rules

The VynaHL7 Test Panel solves these challenges with an intuitive, tabbed interface.

## Quick Start

```bash
# Run directly from source
cargo run -p vyna-hl7-testpanel --release

# Or install globally
cargo install --path crates/vyna-hl7-testpanel
vyna-hl7-testpanel
```

## The Nine Tabs

The Test Panel organizes functionality into nine focused tabs:

### 1. Parser Tab - Understand Any Message

Paste any HL7 message and instantly see:

- **Syntax-highlighted view** with color-coded segments
- **Tree structure** showing the full message hierarchy with complete composite values
- **Field details** with position information and all components/subcomponents
- **Delimiter detection** (handles non-standard delimiters)
- **Raw value display** - delimiters shown as-is (e.g., `^~\&` not escaped)

**Use Case:** When you receive a message from another system and need to understand its structure quickly.

```
Input:
MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315120000||ADT^A01|MSG001|P|2.5
PID|1||12345^^^MRN||Doe^John^A||19800115|M

Output (using Terser path notation):
├── MSH (Segment 1)
│   ├── MSH-1 (Field Separator): |
│   ├── MSH-2 (Encoding Characters): ^~\&
│   ├── MSH-3 (Sending Application): SendApp
│   ├── MSH-9 (Message Type): ADT^A01
│   │   ├── MSH-9-1 (Message Code): ADT
│   │   ├── MSH-9-2 (Trigger Event): A01
│   │   └── MSH-9-3 (Message Structure): ADT_A01
│   └── ...
├── PID (Segment 2)
│   ├── PID-1 (Set ID): 1
│   ├── PID-3 (Patient Identifier List): 12345^^^MRN
│   │   ├── PID-3-1 (ID Number): 12345
│   │   └── PID-3-4 (Assigning Authority): MRN
│   └── ...
```

### 2. Builder Tab - Create Messages Visually

Build HL7 messages using form-based input instead of writing raw pipe-delimited text:

- **Message type selection** (ADT A01, A02, A03, ORU R01, ORM O01, etc.)
- **Field-by-field input** with labels and descriptions
- **Real-time preview** of the generated message
- **Copy to clipboard** for testing

**Supported Message Types:**
- ADT: A01 (Admit), A02 (Transfer), A03 (Discharge), A04 (Register), A08 (Update), A11 (Cancel Admit), A13 (Cancel Discharge)
- ORU R01 (Lab Results)
- ORM O01 (Orders)
- SIU S12 (Scheduling)
- And more...

**Use Case:** Creating test messages for integration testing without memorizing field positions.

### 3. Terser Tab - Extract and Modify Fields

Use path notation to access any field in a message:

| Path | Result |
|------|--------|
| `PID-5` | Full patient name field |
| `PID-5-1` | Family name (component 1) |
| `PID-5-2` | Given name (component 2) |
| `OBX(1)-5` | First OBX observation value |
| `OBX(2)-5` | Second OBX observation value |
| `PID-11(2)-1` | Second address, street (field repetition) |

**Features:**
- **Interactive tree view** - Click any field in the tree to query its value instantly
- **Extract values** from any path using Terser notation
- **Modify values** and see the updated message in real-time
- **Quick path buttons** for common fields (Patient Name, DOB, Gender, etc.)
- **Query history** with re-query buttons to repeat previous lookups

**Use Case:** Quickly extracting specific data points or modifying fields for test scenarios.

### 4. Validator Tab - Ensure Compliance

Validate messages against HL7 standards with detailed error reporting:

**Three-Layer Validation:**
1. **Schema Validation** - Message structure (required segments, cardinality)
2. **Data Type Validation** - Field formats (dates, times, coded values)
3. **Vocabulary Validation** - Code values against HL7 tables

**Example Output:**
```
Validation Results:
✓ Message structure valid for ADT^A01
✓ MSH-7 (Date/Time) format valid
✓ PID-8 (Gender) value 'M' valid for Table 0001
✗ PID-7 (DOB) format invalid: expected YYYYMMDD, got '1980-01-15'
✗ PV1-2 (Patient Class) value 'X' not in Table 0004
```

**Use Case:** Validating messages before sending to production systems.

### 5. MLLP Tab - Network Testing

Built-in MLLP server and client for network testing:

**Server Mode:**
- Start a listener on any port
- View incoming messages in real-time
- Automatic ACK generation
- Message logging with timestamps

**Client Mode:**
- Connect to any MLLP endpoint
- Send messages and view responses
- Connection status monitoring
- Support for multiple sends

**Use Case:** Testing MLLP connections without setting up separate tools.

### 6. HTTP Tab - HL7 over HTTP

Send and receive HL7 messages via HTTP/HTTPS:

- POST messages to a configurable HTTP/HTTPS endpoint
- Client authentication options (none/basic/API key/bearer)
- View response messages and status codes

**Use Case:** Testing inter-organization HTTP interfaces without curl.

### 7. FTP Tab - File Transfer Testing

Transfer HL7 batch files via FTP or SFTP (client and server modes):

- Upload/download message files
- Embedded test server for loopback testing
- Transfer logging with timestamps

**Use Case:** Validating file-drop integrations before go-live.

### 8. FHIR Tab - HL7 v2 to FHIR Conversion

Convert HL7 v2 messages to FHIR R4 resources:

**Supported Conversions:**
| HL7 v2 Segment | FHIR R4 Resource |
|----------------|------------------|
| PID | Patient |
| PV1 | Encounter |
| OBX | Observation |
| OBR | DiagnosticReport |
| AL1 | AllergyIntolerance |
| DG1 | Condition |
| PR1 | Procedure |
| RXA | Immunization |
| ORC | ServiceRequest |
| SPM | Specimen |
| And more... | |

**Features:**
- **Live preview** of FHIR JSON output
- **Multiple resource extraction** from a single message
- **Pretty-printed JSON** for readability
- **Copy to clipboard** for API testing

**Use Case:** Bridging legacy HL7 v2 systems with modern FHIR APIs.

### 9. XML Tab - Format Conversion

Convert between HL7 pipe-delimited (ER7) and XML formats:

**ER7 to XML:**
```xml
<ADT_A01>
  <MSH>
    <MSH.1>|</MSH.1>
    <MSH.2>^~\&amp;</MSH.2>
    <MSH.3>SendApp</MSH.3>
    ...
  </MSH>
  <PID>
    <PID.1>1</PID.1>
    <PID.5>
      <XPN.1>Doe</XPN.1>
      <XPN.2>John</XPN.2>
    </PID.5>
    ...
  </PID>
</ADT_A01>
```

**XML to ER7:**
Convert XML-formatted HL7 messages back to pipe-delimited format.

**Use Case:** Working with systems that require XML format or debugging XML-based HL7 interfaces.

## Sample Messages

The Test Panel includes built-in sample messages for quick testing:

- **ADT A01** - Patient admission with demographics
- **ADT A02** - Patient transfer
- **ADT A03** - Patient discharge
- **ADT A08** - Patient information update
- **ORU R01** - Lab results with multiple OBX segments
- **ORM O01** - Order message
- **SIU S12** - Scheduling message

Access these via the "Samples" dropdown to quickly load realistic test data.

## Keyboard Shortcuts

For power users, the Test Panel supports keyboard shortcuts:

| Shortcut | Action |
|----------|--------|
| `Ctrl+P` | Parse current message |
| `Ctrl+V` | Validate current message |
| `Ctrl+C` | Copy output to clipboard |
| `Ctrl+1-7` | Switch between tabs |
| `F1` | Show help |

## Cross-Platform Support

Built with [egui](https://github.com/emilk/egui), the Test Panel runs natively on:

- **Linux** (X11 and Wayland)
- **macOS** (Intel and Apple Silicon)
- **Windows** (10 and 11)

## Performance

The Test Panel leverages VynaHL7's high-performance parsing engine:

| Operation | Performance |
|-----------|-------------|
| Parse small ADT | ~9 µs (~107,000 msgs/sec) |
| Parse medium ORU | ~30 µs (~34,000 msgs/sec) |
| Terser field access | ~100 ns per path |
| Validation | Real-time for typical messages |

Even large messages with hundreds of segments parse in milliseconds.

## What's Next?

In the [next post](./03-getting-started-with-vyna-hl7.md), we'll dive into the code and build your first HL7 integration programmatically.

---

*Next in series: [Getting Started with VynaHL7](./03-getting-started-with-vyna-hl7.md)*

*Previous: [Why HL7 v2 Integration Still Matters](./01-why-hl7-v2-integration-still-matters.md)*

---

## Resources

- **GitHub:** https://github.com/FlostelInfoTech/vyna-hl7
- **Test Panel Source:** `crates/vyna-hl7-testpanel/`
- **Report Issues:** https://github.com/FlostelInfoTech/vyna-hl7/issues

---

*Have questions or feedback? Open an issue on GitHub or reach out to the team.*
