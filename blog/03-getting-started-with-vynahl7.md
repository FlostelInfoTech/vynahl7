# Getting Started with VynaHL7: Your First HL7 Integration in 10 Minutes

*Part 3 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./02-vyna-hl7-test-panel-your-hl7-workbench.md), we introduced the VynaHL7 Test Panel for interactive HL7 development. Now let's get hands-on and build your first HL7 integration.

By the end of this tutorial, you'll be able to:
- Parse any HL7 v2 message
- Extract patient demographics and clinical data
- Create new HL7 messages programmatically
- Validate messages against HL7 standards

## Prerequisites

- Rust 1.91.0 or later (VynaHL7 uses Rust edition 2024)
- Basic familiarity with Rust

## Step 1: Create a New Project

```bash
cargo new hl7-demo
cd hl7-demo
```

Add VynaHL7 crates to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-core = "1.2"
vyna-hl7-parser = "1.2"
vyna-hl7-terser = "1.2"
vyna-hl7-validator = "1.2"
```

## Step 2: Parse Your First HL7 Message

Let's start with a real ADT^A01 (Admit/Visit Notification) message:

```rust
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A sample ADT^A01 message
    let hl7_message = r"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20240315143000||ADT^A01|MSG0001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||(555)555-5555|||S||67890|123-45-6789
PV1|1|I|WARD^ROOM^BED|||ATTEND^DOCTOR^A|||MED||||1|||ATTEND^DOCTOR^A||VN12345|||||||||||||||||||||||||20240315143000";

    // Parse the message
    let message = parse_message(hl7_message)?;

    // Display basic info
    println!("Segments: {}", message.segment_count());
    println!("Sending App: {:?}", message.get_sending_application());
    println!("Message Type: {:?}", message.get_message_type());
    println!("Control ID: {:?}", message.get_control_id());
    println!("HL7 Version: {:?}", message.get_version());

    Ok(())
}
```

Run it:
```bash
cargo run
```

Output:
```
Segments: 3
Sending App: Some("SendingApp")
Message Type: Some(("ADT", "A01"))
Control ID: Some("MSG0001")
HL7 Version: Some(V2_5)
```

That's it! You've parsed your first HL7 message.

## Step 3: Extract Data with Direct Field Access

For quick data extraction, you can access segments and fields directly:

```rust
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7_message = r"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20240315143000||ADT^A01|MSG0001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||(555)555-5555|||S||67890|123-45-6789
PV1|1|I|WARD^ROOM^BED|||ATTEND^DOCTOR^A|||MED||||1|||ATTEND^DOCTOR^A||VN12345|||||||||||||||||||||||||20240315143000";

    let message = parse_message(hl7_message)?;

    // Get the PID segment
    if let Some(pid) = message.get_segments_by_id("PID").first() {
        println!("Patient ID: {:?}", pid.get_field_value(2));
        println!("MRN: {:?}", pid.get_field_value(3));

        // Access name components
        if let Some(name_field) = pid.get_field(5) {
            if let Some(rep) = name_field.get_repetition(0) {
                let family = rep.get_component(0).and_then(|c| c.value());
                let given = rep.get_component(1).and_then(|c| c.value());
                let middle = rep.get_component(2).and_then(|c| c.value());
                println!("Name: {:?} {:?} {:?}", family, given, middle);
            }
        }

        println!("DOB: {:?}", pid.get_field_value(7));
        println!("Gender: {:?}", pid.get_field_value(8));
    }

    Ok(())
}
```

This works, but accessing nested components gets verbose. That's where Terser comes in.

## Step 4: Extract Data with Terser (The Better Way)

Terser provides path-based field access using intuitive notation:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7_message = r"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20240315143000||ADT^A01|MSG0001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||(555)555-5555|||S||67890|123-45-6789
PV1|1|I|WARD^ROOM^BED|||ATTEND^DOCTOR^A|||MED||||1|||ATTEND^DOCTOR^A||VN12345|||||||||||||||||||||||||20240315143000";

    let message = parse_message(hl7_message)?;
    let terser = Terser::new(&message);

    // Simple, intuitive path notation
    println!("Patient Demographics:");
    println!("  Family Name: {:?}", terser.get("PID-5-1")?);
    println!("  Given Name: {:?}", terser.get("PID-5-2")?);
    println!("  Middle Name: {:?}", terser.get("PID-5-3")?);
    println!("  DOB: {:?}", terser.get("PID-7")?);
    println!("  Gender: {:?}", terser.get("PID-8")?);
    println!("  SSN: {:?}", terser.get("PID-19")?);

    println!("\nVisit Information:");
    println!("  Patient Class: {:?}", terser.get("PV1-2")?);
    println!("  Ward: {:?}", terser.get("PV1-3-1")?);
    println!("  Room: {:?}", terser.get("PV1-3-2")?);
    println!("  Bed: {:?}", terser.get("PV1-3-3")?);
    println!("  Attending Doctor: {:?}", terser.get("PV1-7-2")?);
    println!("  Visit Number: {:?}", terser.get("PV1-19")?);

    Ok(())
}
```

Output:
```
Patient Demographics:
  Family Name: Some("DOE")
  Given Name: Some("JOHN")
  Middle Name: Some("A")
  DOB: Some("19800101")
  Gender: Some("M")
  SSN: Some("123-45-6789")

Visit Information:
  Patient Class: Some("I")
  Ward: Some("WARD")
  Room: Some("ROOM")
  Bed: Some("BED")
  Attending Doctor: Some("DOCTOR")
  Visit Number: Some("VN12345")
```

Much cleaner! The Terser path notation is:
- `PID-5` - Field 5 of PID segment
- `PID-5-1` - First component of field 5
- `PID-5-1-2` - Second subcomponent of first component
- `OBX(2)-5` - Field 5 of the second OBX segment (1-based indexing)
- `PID-11(2)-1` - Second repetition of field 11, first component

## Step 5: Create Messages Programmatically

VynaHL7 provides type-safe builders for common message types:

```rust
use vyna_hl7_core::builders::adt::AdtBuilder;
use vyna_hl7_core::Version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an ADT^A01 (Admit) message
    let message = AdtBuilder::a01(Version::V2_5)
        .sending_application("MyHospitalApp")
        .sending_facility("General Hospital")
        .receiving_application("CentralRegistry")
        .receiving_facility("HealthNetwork")
        .patient_id("PAT-12345")
        .patient_name("SMITH", "JANE")
        .date_of_birth("19850315")
        .sex("F")
        .patient_class("I")  // Inpatient
        .assigned_location("ICU^101^A")
        .attending_doctor("JOHNSON^ROBERT^MD")
        .build()?;

    // Encode to HL7 string
    println!("Generated ADT^A01:");
    println!("{}", message.encode().replace('\r', "\n"));

    Ok(())
}
```

VynaHL7 includes builders for many message types:

| Builder | Message Type | Description |
|---------|--------------|-------------|
| `AdtBuilder::a01()` | ADT^A01 | Admit/Visit Notification |
| `AdtBuilder::a02()` | ADT^A02 | Transfer Patient |
| `AdtBuilder::a03()` | ADT^A03 | Discharge Patient |
| `AdtBuilder::a04()` | ADT^A04 | Register Patient |
| `AdtBuilder::a08()` | ADT^A08 | Update Patient Info |
| `OruR01Builder` | ORU^R01 | Lab Results |
| `OrmO01Builder` | ORM^O01 | Orders |
| `RdeO11Builder` | RDE^O11 | Pharmacy Order |

### Creating Lab Results (ORU^R01)

```rust
use vyna_hl7_core::builders::oru::{OruR01Builder, Observation};
use vyna_hl7_core::Version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = OruR01Builder::new(Version::V2_5)
        .sending_application("LabSystem")
        .sending_facility("CentralLab")
        .receiving_application("EMR")
        .receiving_facility("Hospital")
        .patient_id("PAT-12345")
        .patient_name("SMITH", "JANE")
        .filler_order_number("LAB-2024-001")
        .add_observation(Observation {
            set_id: 1,
            value_type: "NM".to_string(),
            identifier: "GLUCOSE^Glucose^LN".to_string(),
            value: "95".to_string(),
            units: Some("mg/dL".to_string()),
            status: "F".to_string(),  // Final result
        })
        .add_observation(Observation {
            set_id: 2,
            value_type: "NM".to_string(),
            identifier: "HBA1C^Hemoglobin A1c^LN".to_string(),
            value: "5.4".to_string(),
            units: Some("%".to_string()),
            status: "F".to_string(),
        })
        .build()?;

    println!("Generated ORU^R01:");
    println!("{}", message.encode().replace('\r', "\n"));

    Ok(())
}
```

## Step 6: Validate Messages

VynaHL7 provides comprehensive validation against HL7 standards:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_core::Version;
use vyna_hl7_validator::Validator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7_message = r"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20240315143000||ADT^A01|MSG0001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 MAIN ST^^CITY^ST^12345||(555)555-5555
PV1|1|I|WARD^ROOM^BED";

    let message = parse_message(hl7_message)?;

    // Create validator for HL7 v2.5
    let validator = Validator::new(Version::V2_5);
    let result = validator.validate(&message);

    if result.is_valid() {
        println!("Message is valid!");
    } else {
        println!("Validation errors:");
        for error in &result.errors {
            println!("  {} - {}", error.location, error.message);
        }
    }

    if !result.warnings.is_empty() {
        println!("\nWarnings:");
        for warning in &result.warnings {
            println!("  {} - {}", warning.location, warning.message);
        }
    }

    Ok(())
}
```

## Step 7: Modify Messages with TerserMut

You can also modify existing messages:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::TerserMut;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7_message = r"MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20240315143000||ADT^A01|MSG0001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M";

    let mut message = parse_message(hl7_message)?;

    // Use TerserMut to modify values
    let mut terser = TerserMut::new(&mut message);

    terser.set("PID-5-1", "SMITH")?;        // Change family name
    terser.set("PID-5-2", "JANE")?;          // Change given name
    terser.set("PID-8", "F")?;               // Change gender
    terser.set("PID-7", "19900101")?;        // Change DOB

    println!("Modified message:");
    println!("{}", message.encode().replace('\r', "\n"));

    Ok(())
}
```

## Complete Example: Parse, Validate, Extract, and Respond

Here's a complete example that ties everything together:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_core::{Version, Message, Segment, Field};
use vyna_hl7_core::delimiters::Delimiters;
use vyna_hl7_terser::Terser;
use vyna_hl7_validator::Validator;
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Incoming ADT message
    let incoming = r"MSH|^~\&|ADT|HOSPITAL|LAB|LAB|20240315143000||ADT^A01|MSG001|P|2.5
PID|1||12345^^^MRN||SMITH^JOHN^M||19800515|M|||123 Main St^^Springfield^IL^62701
PV1|1|I|ICU^101^A|||12345^JOHNSON^ROBERT^MD";

    println!("=== Processing Incoming ADT^A01 ===\n");

    // 1. Parse
    let message = parse_message(incoming)?;
    println!("Parsed message with {} segments", message.segment_count());

    // 2. Validate
    let validator = Validator::new(Version::V2_5);
    let validation = validator.validate(&message);

    if !validation.is_valid() {
        println!("Validation failed!");
        for error in &validation.errors {
            println!("  Error: {}", error.message);
        }
        return Ok(());
    }
    println!("Message validated successfully");

    // 3. Extract key data
    let terser = Terser::new(&message);

    let patient_id = terser.get("PID-3-1")?.unwrap_or_default();
    let patient_name = format!(
        "{} {}",
        terser.get("PID-5-2")?.unwrap_or_default(),
        terser.get("PID-5-1")?.unwrap_or_default()
    );
    let location = terser.get("PV1-3-1")?.unwrap_or_default();

    println!("\nPatient: {} (MRN: {})", patient_name, patient_id);
    println!("Admitted to: {}", location);

    // 4. Generate ACK response
    let ack = create_ack(&message, "AA")?;
    println!("\n=== ACK Response ===");
    println!("{}", ack.encode().replace('\r', "\n"));

    Ok(())
}

fn create_ack(original: &Message, ack_code: &str) -> Result<Message, Box<dyn std::error::Error>> {
    let mut ack = Message::new();
    let delims = Delimiters::default();

    // Build MSH
    let mut msh = Segment::new("MSH");
    msh.add_field(Field::from_value(delims.field_separator.to_string()));
    msh.add_field(Field::from_value(delims.encoding_characters()));

    // Swap sender and receiver
    if let Some(v) = original.get_receiving_application() {
        msh.set_field_value(3, v)?;
    }
    if let Some(v) = original.get_receiving_facility() {
        msh.set_field_value(4, v)?;
    }
    if let Some(v) = original.get_sending_application() {
        msh.set_field_value(5, v)?;
    }
    if let Some(v) = original.get_sending_facility() {
        msh.set_field_value(6, v)?;
    }

    msh.set_field_value(7, Utc::now().format("%Y%m%d%H%M%S").to_string())?;
    msh.set_field_value(9, "ACK")?;
    msh.set_field_value(10, format!("ACK{}", Utc::now().timestamp()))?;
    msh.set_field_value(11, "P")?;

    if let Some(v) = original.get_version() {
        msh.set_field_value(12, v.as_str())?;
    }
    ack.add_segment(msh);

    // Build MSA
    let mut msa = Segment::new("MSA");
    msa.set_field_value(1, ack_code)?;
    if let Some(id) = original.get_control_id() {
        msa.set_field_value(2, id)?;
    }
    ack.add_segment(msa);

    Ok(ack)
}
```

## What's Next?

You've now learned the fundamentals of VynaHL7:
- **Parsing** - Converting HL7 strings to structured data
- **Terser** - Path-based field access and modification
- **Builders** - Creating messages programmatically
- **Validation** - Ensuring message compliance

In the next post, we'll dive deeper into HL7 v2 message structure and VynaHL7's core architecture, giving you a solid foundation for understanding any HL7 message you encounter.

## Quick Reference

```rust
// Parse a message
let message = parse_message(hl7_string)?;

// Read with Terser
let terser = Terser::new(&message);
let value = terser.get("PID-5-1")?;

// Modify with TerserMut
let mut terser = TerserMut::new(&mut message);
terser.set("PID-5-1", "SMITH")?;

// Build a message
let message = AdtBuilder::a01(Version::V2_5)
    .patient_id("12345")
    .patient_name("DOE", "JOHN")
    .build()?;

// Validate
let validator = Validator::new(Version::V2_5);
let result = validator.validate(&message);

// Encode back to HL7
let hl7_string = message.encode();
```

---

*Next in series: [Understanding HL7 v2 Messages: VynaHL7's Core Architecture](./04-understanding-hl7-v2-messages.md)*

*Previous: [VynaHL7 Test Panel](./02-vyna-hl7-test-panel-your-hl7-workbench.md)*
