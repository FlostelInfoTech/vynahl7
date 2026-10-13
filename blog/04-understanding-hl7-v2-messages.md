# Understanding HL7 v2 Messages: VynaHL7's Core Architecture

*Part 4 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./03-getting-started-with-vyna-hl7.md), we got hands-on with VynaHL7. Now let's dive deeper into HL7 v2 message structure and understand how VynaHL7 models these messages internally.

## The Anatomy of an HL7 v2 Message

Every HL7 v2 message follows a hierarchical structure with five levels:

```
Message
  └── Segment (MSH, PID, OBX, etc.)
       └── Field (separated by |)
            └── Repetition (separated by ~)
                 └── Component (separated by ^)
                      └── Subcomponent (separated by &)
```

Let's examine a real message:

```
MSH|^~\&|LABSYS|LAB|EMR|HOSP|20240315120000||ORU^R01|MSG001|P|2.5
PID|1||123456^^^MRN~9876543^^^SSN||DOE^JOHN^MICHAEL^JR^DR||19800515|M
OBX|1|NM|GLU^Glucose^LN||95|mg/dL|70-100|N|||F
OBX|2|CE|ABO^Blood Type^LN||A+^A Positive^HL70005|||N|||F
```

### Level 1: The Message

A message is a collection of segments, always starting with MSH (Message Header). The MSH segment defines:
- Who sent the message
- Who should receive it
- What type of message it is
- The HL7 version

### Level 2: Segments

Each line is a segment, identified by a 3-character ID:

| Segment | Purpose |
|---------|---------|
| **MSH** | Message Header - metadata about the message |
| **PID** | Patient Identification - demographics |
| **PV1** | Patient Visit - encounter information |
| **OBR** | Observation Request - order information |
| **OBX** | Observation Result - individual results |
| **NK1** | Next of Kin - emergency contacts |
| **AL1** | Allergy Information |
| **DG1** | Diagnosis |
| **IN1** | Insurance |

### Level 3: Fields

Fields within a segment are separated by `|` (pipe). Field positions are numbered starting from 1:

```
PID|1||123456^^^MRN||DOE^JOHN||19800515|M
    ^  ^            ^        ^        ^
    1  2  3         4  5     6  7     8
```

Note: MSH is special—MSH-1 is the field separator itself (`|`), and MSH-2 contains the encoding characters (`^~\&`).

### Level 4: Repetitions

A field can contain multiple values, separated by `~`:

```
PID-3: 123456^^^MRN~9876543^^^SSN
       ├─ Rep 0 ─┤ ├─ Rep 1 ─┤
```

This patient has two identifiers: an MRN and an SSN.

### Level 5: Components

Complex fields are divided into components, separated by `^`:

```
PID-5: DOE^JOHN^MICHAEL^JR^DR
       │   │    │       │  │
       │   │    │       │  └── Prefix (Dr.)
       │   │    │       └── Suffix (Jr.)
       │   │    └── Middle name
       │   └── Given name
       └── Family name
```

### Level 6: Subcomponents

Components can be further divided using `&`:

```
CX data type: 123456^4^M11^HOSP&2.16.840.1.113883.3.123&ISO^MR
                          ├───────────────┬──────────────┤
                          Assigning Authority with subcomponents:
                          - Namespace ID: HOSP
                          - Universal ID: 2.16.840.1.113883.3.123
                          - Universal ID Type: ISO
```

## The Encoding Characters

The MSH-2 field (`^~\&`) defines the special characters:

| Character | Purpose | Name |
|-----------|---------|------|
| `^` | Component separator | Caret |
| `~` | Repetition separator | Tilde |
| `\` | Escape character | Backslash |
| `&` | Subcomponent separator | Ampersand |

These can technically be customized per message (though that's rare).

## VynaHL7's Data Model

VynaHL7 models this hierarchy with Rust types in the `vyna-hl7-core` crate:

### Message

```rust
pub struct Message {
    segments: Vec<Segment>,
    delimiters: Delimiters,
}

impl Message {
    // Access segments
    pub fn get_segment(&self, id: &str) -> Option<&Segment>;
    pub fn get_segments_by_id(&self, id: &str) -> Vec<&Segment>;
    pub fn segment_count(&self) -> usize;

    // MSH convenience methods
    pub fn get_sending_application(&self) -> Option<&str>;
    pub fn get_receiving_application(&self) -> Option<&str>;
    pub fn get_message_type(&self) -> Option<(&str, &str)>;
    pub fn get_control_id(&self) -> Option<&str>;
    pub fn get_version(&self) -> Option<Version>;

    // Encode back to HL7 string
    pub fn encode(&self) -> String;
}
```

### Segment

```rust
pub struct Segment {
    id: String,        // "MSH", "PID", "OBX", etc.
    fields: Vec<Field>,
}

impl Segment {
    pub fn get_field(&self, index: usize) -> Option<&Field>;
    pub fn get_field_value(&self, index: usize) -> Option<&str>;
    pub fn set_field_value(&mut self, index: usize, value: impl AsRef<str>) -> Result<()>;
}
```

### Field

```rust
pub struct Field {
    repetitions: Vec<Repetition>,
}

impl Field {
    pub fn get_repetition(&self, index: usize) -> Option<&Repetition>;
    pub fn repetition_count(&self) -> usize;
}
```

### Repetition, Component, Subcomponent

```rust
pub struct Repetition {
    components: Vec<Component>,
}

pub struct Component {
    subcomponents: Vec<Subcomponent>,
}

pub struct Subcomponent {
    value: Option<String>,
}
```

## Supported HL7 Versions

VynaHL7 supports HL7 versions 2.3 through 2.8.2:

```rust
pub enum Version {
    V2_3,
    V2_3_1,
    V2_4,
    V2_5,
    V2_5_1,
    V2_6,
    V2_7,
    V2_7_1,
    V2_8,
    V2_8_1,
    V2_8_2,
}
```

Each version has different:
- Required/optional fields
- Available segments
- Data type definitions
- Vocabulary tables

## Common Message Types

VynaHL7 supports all major HL7 v2 message types:

### ADT (Admit/Discharge/Transfer)

Patient movement events:

| Event | Description |
|-------|-------------|
| A01 | Admit a patient |
| A02 | Transfer a patient |
| A03 | Discharge a patient |
| A04 | Register an outpatient |
| A05 | Pre-admit a patient |
| A08 | Update patient information |
| A11 | Cancel admit |
| A28 | Add person information |
| A31 | Update person information |
| A40 | Merge patient records |

### ORU (Observation Result - Unsolicited)

Lab and diagnostic results:

| Event | Description |
|-------|-------------|
| R01 | Unsolicited observation result |

### ORM (Order Message)

Clinical orders:

| Event | Description |
|-------|-------------|
| O01 | Order message |

### Pharmacy Messages

| Message | Event | Description |
|---------|-------|-------------|
| RDE | O11 | Pharmacy/treatment encoded order |
| RDS | O13 | Pharmacy/treatment dispense |
| RGV | O15 | Pharmacy/treatment give |
| RAS | O17 | Pharmacy/treatment administration |
| RRA | O18 | Pharmacy/treatment administration acknowledgment |
| RRD | O14 | Pharmacy dispense information |

### Laboratory Messages

| Message | Event | Description |
|---------|-------|-------------|
| OUL | R21 | Unsolicited laboratory observation |
| OML | O21 | Laboratory order |

### Other Common Types

- **SIU** - Scheduling messages
- **MDM** - Medical document management
- **DFT** - Financial transactions
- **MFN** - Master file notifications
- **QRY** - Query messages
- **ACK** - Acknowledgment

## Working with the Structure

### Navigating Segments

```rust
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = parse_message(hl7_string)?;

    // Get first segment of a type
    if let Some(pid) = message.get_segments_by_id("PID").first() {
        println!("Found PID segment");
    }

    // Iterate all segments
    for segment in message.segments() {
        println!("Segment: {}", segment.id());
    }

    // Get all OBX segments (there may be many)
    let obx_segments = message.get_segments_by_id("OBX");
    println!("Found {} OBX segments", obx_segments.len());

    Ok(())
}
```

### Navigating Fields and Components

```rust
// Get raw field value (all components concatenated)
let mrn = pid.get_field_value(3);  // "123456^^^MRN"

// Get specific components
if let Some(field) = pid.get_field(5) {
    if let Some(rep) = field.get_repetition(0) {
        let family = rep.get_component(0).and_then(|c| c.value());
        let given = rep.get_component(1).and_then(|c| c.value());
    }
}
```

### Handling Repetitions

```rust
// PID-3 might have multiple patient IDs
if let Some(field) = pid.get_field(3) {
    for i in 0..field.repetition_count() {
        if let Some(rep) = field.get_repetition(i) {
            let id = rep.get_component(0).and_then(|c| c.value());
            let id_type = rep.get_component(4).and_then(|c| c.value());
            println!("ID: {:?} (type: {:?})", id, id_type);
        }
    }
}
```

## Key Data Types

HL7 v2 defines many data types. Here are the most common:

### Simple Types

| Type | Description | Example |
|------|-------------|---------|
| ST | String | `SMITH` |
| NM | Numeric | `95.5` |
| DT | Date | `20240315` |
| TM | Time | `143022` |
| TS | Timestamp | `20240315143022` |
| ID | Coded value | `M` (Male) |
| IS | Coded value (user-defined) | `ER` |

### Complex Types

| Type | Description | Example |
|------|-------------|---------|
| XPN | Extended Person Name | `DOE^JOHN^M^JR^DR` |
| XAD | Extended Address | `123 Main^Apt 4^City^ST^12345` |
| XTN | Extended Telephone | `^PRN^PH^^1^555^1234567` |
| CX | Extended Composite ID | `123456^^^MRN^MR` |
| XCN | Extended Composite ID + Name | `12345^SMITH^JOHN^M^^MD` |
| CE | Coded Element | `GLU^Glucose^LN` |
| CWE | Coded With Exceptions | Like CE, with more fields |
| HD | Hierarchic Designator | `HOSP^2.16.840.1...^ISO` |

## Building Complex Fields

VynaHL7 provides builders for complex field types:

```rust
use vyna_hl7_core::builders::fields::{XpnBuilder, XadBuilder, CxBuilder};

// Build a patient name (XPN)
let name = XpnBuilder::new()
    .family_name("SMITH")
    .given_name("JOHN")
    .middle_name("MICHAEL")
    .suffix("JR")
    .prefix("DR")
    .build();

// Build an address (XAD)
let address = XadBuilder::new()
    .street("123 Main Street")
    .other_designation("Suite 100")
    .city("Springfield")
    .state("IL")
    .zip("62701")
    .country("USA")
    .build();

// Build a patient identifier (CX)
let mrn = CxBuilder::new()
    .id("123456")
    .assigning_authority("HOSP")
    .identifier_type("MR")
    .build();
```

## Escape Sequences

Special characters in HL7 values must be escaped:

| Sequence | Character | Description |
|----------|-----------|-------------|
| `\F\` | `|` | Field separator |
| `\S\` | `^` | Component separator |
| `\T\` | `&` | Subcomponent separator |
| `\R\` | `~` | Repetition separator |
| `\E\` | `\` | Escape character |
| `\.br\` | newline | Line break |

VynaHL7 handles escaping and unescaping automatically:

```rust
// When encoding, special characters are escaped
// When parsing, escape sequences are decoded
```

## The MSH Segment Deep Dive

The MSH segment is the most important—it tells you everything about the message:

```
MSH|^~\&|SENDING_APP|SENDING_FAC|RECV_APP|RECV_FAC|20240315143000||ADT^A01^ADT_A01|MSG001|P|2.5|||AL|NE
    │    │           │           │        │        │               │    │            │      │ │   │  │
    1    3           4           5        6        7               9    10           11     12 13  17 18
```

| Field | Name | Description |
|-------|------|-------------|
| MSH-1 | Field Separator | `|` |
| MSH-2 | Encoding Characters | `^~\&` |
| MSH-3 | Sending Application | Source system |
| MSH-4 | Sending Facility | Source organization |
| MSH-5 | Receiving Application | Destination system |
| MSH-6 | Receiving Facility | Destination organization |
| MSH-7 | Date/Time of Message | When message was created |
| MSH-9 | Message Type | ADT^A01 (type^trigger) |
| MSH-10 | Message Control ID | Unique identifier |
| MSH-11 | Processing ID | P=Production, T=Training, D=Debugging |
| MSH-12 | Version ID | 2.5 |
| MSH-15 | Accept Acknowledgment Type | AL, NE, ER, SU |
| MSH-16 | Application Acknowledgment Type | AL, NE, ER, SU |

## Encoding and Decoding

VynaHL7's parser is built with `nom` for efficient, zero-copy parsing:

```rust
use vyna_hl7_parser::parse_message;

// Parsing (HL7 string → Message struct)
let message = parse_message(hl7_string)?;

// Encoding (Message struct → HL7 string)
let hl7_string = message.encode();
```

The parser:
- Handles HL7 versions 2.1 through 2.8.2
- Supports custom delimiters
- Processes escape sequences
- Performs zero-copy parsing where possible
- Achieves tens of thousands of messages/second (measured ~60,000/sec small ADT; see `docs/performance.md`)

## Summary

Understanding HL7 v2 message structure is essential for healthcare integration:

1. **Messages** contain **Segments** (MSH, PID, OBX...)
2. **Segments** contain **Fields** (separated by `|`)
3. **Fields** can have **Repetitions** (separated by `~`)
4. **Repetitions** contain **Components** (separated by `^`)
5. **Components** can have **Subcomponents** (separated by `&`)

VynaHL7 models this hierarchy faithfully in Rust, providing:
- Type-safe access at every level
- Builders for complex data types
- Automatic escape sequence handling
- High-performance parsing and encoding

In the next post, we'll explore the Terser API in depth—the elegant way to access fields without navigating this hierarchy manually.

---

*Next in series: [The Terser API: Elegant Field Access for HL7 Messages](./05-the-terser-api.md)*

*Previous: [Getting Started with VynaHL7](./03-getting-started-with-vyna-hl7.md)*
