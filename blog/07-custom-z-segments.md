# Custom Z-Segments: Extending HL7 for Your Organization

*Part 7 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./06-message-validation.md), we covered message validation. Now let's explore one of HL7's most powerful features: **custom Z-segments** for organization-specific data.

## What Are Z-Segments?

HL7 v2 reserves segment names starting with "Z" for local, site-specific use. These **Z-segments** allow organizations to extend HL7 messages with custom data that isn't covered by standard segments.

Common examples:

| Segment | Purpose |
|---------|---------|
| `ZPV` | Custom visit information |
| `ZPD` | Additional patient demographics |
| `ZIN` | Custom insurance data |
| `ZLB` | Lab-specific extensions |
| `ZRX` | Pharmacy-specific data |

## The Challenge

While Z-segments are flexible, they create challenges:

1. **No standard schema** - Each organization defines their own structure
2. **Documentation often lacking** - Field meanings may be tribal knowledge
3. **Type safety** - Treating everything as strings leads to bugs
4. **Maintenance** - Changes to Z-segment structure propagate poorly

VynaHL7's `vyna-hl7-custom` crate solves these problems with a **type-safe framework** for defining and parsing custom Z-segments.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-custom = "1.2"
```

## Defining a Custom Z-Segment

Use the `z_segment!` macro to define your segment with typed fields:

```rust
use vyna_hl7_custom::z_segment;

z_segment! {
    /// Custom patient visit segment
    ZPV {
        /// Visit sequence number
        visit_sequence: u32,

        /// Custom visit type code
        visit_type: String,

        /// Expected length of stay (days)
        expected_los: Option<u32>,

        /// VIP indicator
        vip_flag: bool,

        /// Custom notes
        notes: Option<String>,
    }
}
```

This generates:
- A `ZPV` struct with typed fields
- Parsing from HL7 field data
- Serialization back to HL7 format
- Validation for required fields

## Using Custom Segments

### Parsing from a Message

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_custom::MessageExt;

let hl7 = r"MSH|^~\&|App|Fac|||202403150800||ADT^A01|123|P|2.5
PID|1||12345||Doe^John
ZPV|1|OUTPATIENT|5|Y|VIP patient - expedite";

let message = parse_message(hl7)?;

// Extract the custom segment
if let Some(zpv) = message.get_custom_segment::<ZPV>()? {
    println!("Visit Sequence: {}", zpv.visit_sequence);  // 1
    println!("Visit Type: {}", zpv.visit_type);          // "OUTPATIENT"
    println!("Expected LOS: {:?}", zpv.expected_los);    // Some(5)
    println!("VIP: {}", zpv.vip_flag);                   // true
    println!("Notes: {:?}", zpv.notes);                  // Some("VIP patient...")
}
```

### Building Messages with Z-Segments

```rust
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_custom::MessageExt;

let zpv = ZPV {
    visit_sequence: 1,
    visit_type: "INPATIENT".to_string(),
    expected_los: Some(7),
    vip_flag: false,
    notes: None,
};

let message = AdtBuilder::a01(Version::V2_5)
    .sending_application("MyApp")
    .patient_id("12345")
    .patient_name("Doe", "John", None)
    .build()?
    .with_custom_segment(zpv)?;
```

## Field Types

The `z_segment!` macro supports these field types:

| Rust Type | HL7 Representation | Example |
|-----------|-------------------|---------|
| `String` | Text | `"ABC123"` |
| `u32` | Numeric | `123` |
| `i32` | Signed numeric | `-5` |
| `f64` | Decimal | `98.6` |
| `bool` | Y/N flag | `"Y"` or `"N"` |
| `Option<T>` | Optional field | Empty = `None` |

## Components and Subcomponents

For complex fields with components:

```rust
z_segment! {
    ZCU {
        /// Customer ID
        customer_id: String,

        /// Customer name (family^given^middle)
        #[components(3)]
        customer_name: (String, String, Option<String>),

        /// Address (street^city^state^zip)
        #[components(4)]
        address: (String, String, String, String),
    }
}
```

Usage:

```rust
let zcu = message.get_custom_segment::<ZCU>()?;
let (family, given, middle) = &zcu.customer_name;
let (street, city, state, zip) = &zcu.address;
```

## Repeating Fields

For fields that can repeat (separated by `~`):

```rust
z_segment! {
    ZAL {
        /// Allergy code (can repeat)
        #[repeating]
        allergy_codes: Vec<String>,

        /// Severity for each allergy
        #[repeating]
        severities: Vec<String>,
    }
}
```

HL7 example:
```
ZAL|PENICILLIN~SULFA~LATEX|HIGH~MEDIUM~LOW
```

## Validation

Add custom validation with the `#[validate]` attribute:

```rust
z_segment! {
    ZLB {
        /// Lab accession number (must be 10 digits)
        #[validate(regex = r"^\d{10}$")]
        accession: String,

        /// Result value (must be positive)
        #[validate(range = "0..")]
        result_value: f64,

        /// Status (must be one of: P, F, C)
        #[validate(values = ["P", "F", "C"])]
        status: String,
    }
}
```

## Multiple Z-Segments

Handle multiple occurrences of a Z-segment:

```rust
// Get all ZPV segments (for messages with multiple visits)
let all_zpv: Vec<ZPV> = message.get_all_custom_segments()?;

for (i, zpv) in all_zpv.iter().enumerate() {
    println!("Visit {}: Type = {}", i + 1, zpv.visit_type);
}
```

## Registration for Dynamic Parsing

Register segments globally for dynamic parsing:

```rust
use vyna_hl7_custom::registry::register_segment;

// At startup
register_segment::<ZPV>("ZPV");
register_segment::<ZCU>("ZCU");
register_segment::<ZLB>("ZLB");

// Later: parse any registered segment by name
let segment = message.get_registered_segment("ZPV")?;
```

## Real-World Example: Healthcare Extension

A hospital's custom segments for their ADT workflow:

```rust
z_segment! {
    /// Extended patient demographics
    ZPD {
        /// Primary language code
        primary_language: String,

        /// Interpreter needed flag
        interpreter_needed: bool,

        /// Preferred contact method (PHONE, EMAIL, MAIL)
        preferred_contact: String,

        /// Emergency contact relationship
        emergency_relationship: Option<String>,

        /// Advance directive on file
        advance_directive: bool,
    }
}

z_segment! {
    /// Insurance authorization
    ZIN {
        /// Authorization number
        auth_number: String,

        /// Authorized days
        authorized_days: u32,

        /// Authorization start date (YYYYMMDD)
        auth_start: String,

        /// Authorization end date (YYYYMMDD)
        auth_end: String,

        /// Pre-certification status
        precert_status: String,
    }
}

// Usage in an ADT message handler
fn process_admission(message: &Message) -> Result<()> {
    let pid = Terser::new(&message).get("PID-5-1")?;

    // Get custom demographics
    if let Some(zpd) = message.get_custom_segment::<ZPD>()? {
        if zpd.interpreter_needed {
            request_interpreter(&zpd.primary_language)?;
        }
    }

    // Get insurance authorization
    if let Some(zin) = message.get_custom_segment::<ZIN>()? {
        verify_authorization(&zin.auth_number, zin.authorized_days)?;
    }

    Ok(())
}
```

## Interoperability Tips

When working with Z-segments across organizations:

1. **Document thoroughly** - Create a data dictionary for each Z-segment
2. **Version your segments** - Consider `ZPV1`, `ZPV2` for breaking changes
3. **Use optional fields** - New fields should be `Option<T>` for backward compatibility
4. **Validate early** - Check Z-segment data at the interface boundary
5. **Map to standard segments when possible** - Use Z-segments only for truly custom data

## Performance

Z-segment parsing adds minimal overhead:

| Operation | Time |
|-----------|------|
| Parse standard segment | ~2 µs |
| Parse Z-segment (5 fields) | ~3 µs |
| Parse Z-segment (10 fields) | ~5 µs |

The type-safe approach catches errors at parse time rather than runtime, improving overall system reliability.

## What's Next?

In the [next post](./08-network-transport.md), we'll explore MLLP and HTTP transport for sending HL7 messages over the network.

---

## Resources

- **Source Code:** `crates/vyna-hl7-custom/`
- **API Documentation:** Available after crates.io publication
- **Examples:** `crates/vyna-hl7-custom/examples/`
