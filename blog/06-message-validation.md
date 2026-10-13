# Message Validation: Ensuring HL7 Compliance with VynaHL7

*Part 6 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./05-the-terser-api.md), we explored the Terser API for elegant field access. But parsing a message is only half the battle—you also need to ensure it's valid according to HL7 standards.

VynaHL7 provides a comprehensive three-layer validation architecture:

1. **Schema Validation** - Message structure against HL7 standards
2. **Data Type Validation** - Format checking for dates, times, numerics, etc.
3. **Vocabulary Validation** - Code validation against HL7 tables

## Quick Start: Basic Validation

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_core::Version;
use vyna_hl7_validator::Validator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7 = r"MSH|^~\&|APP|FAC|RECV|DEST|20240315143000||ADT^A01|MSG001|P|2.5
PID|1||123456||DOE^JOHN||19800515|M
PV1|1|I|ICU^101^A";

    let message = parse_message(hl7)?;

    // Create validator for HL7 v2.5
    let validator = Validator::new(Version::V2_5);
    let result = validator.validate(&message);

    if result.is_valid() {
        println!("Message is valid!");
    } else {
        println!("Validation failed:");
        for error in &result.errors {
            println!("  Error at {}: {}", error.location, error.message);
        }
    }

    // Check warnings (non-fatal issues)
    for warning in &result.warnings {
        println!("  Warning at {}: {}", warning.location, warning.message);
    }

    Ok(())
}
```

## Layer 1: Schema Validation

Schema validation ensures the message structure follows HL7 specifications:
- Required segments are present
- Segments appear in the correct order
- Required fields have values
- Field lengths don't exceed limits
- Repeating vs. non-repeating fields

### Built-in Schemas

VynaHL7 includes schemas for all major message types across HL7 versions 2.3 through 2.7.1:

```
crates/vyna-hl7-validator/schemas/
├── v2_3/
│   ├── ADT_A01.json
│   ├── ORU_R01.json
│   └── ...
├── v2_4/
├── v2_5/
├── v2_6/
└── v2_7/
```

The validator automatically loads the appropriate schema based on the message type:

```rust
// Validator detects ADT^A01 and loads the correct schema
let validator = Validator::new(Version::V2_5);
let result = validator.validate(&message);
```

### Custom Schemas

You can define custom schemas for organization-specific validation:

```rust
use vyna_hl7_validator::{
    MessageSchema, SegmentDefinition, FieldDefinition, Validator
};
use std::collections::HashMap;

fn create_custom_schema() -> MessageSchema {
    let mut schema = MessageSchema {
        message_type: "ADT".to_string(),
        trigger_event: "A01".to_string(),
        version: "2.5".to_string(),
        segments: HashMap::new(),
    };

    // Define MSH segment requirements
    let mut msh_fields = HashMap::new();
    msh_fields.insert(11, FieldDefinition {
        name: "Processing ID".to_string(),
        data_type: "PT".to_string(),
        required: true,
        repeating: false,
        max_length: Some(3),
        table_id: Some("0103".to_string()),
    });

    schema.segments.insert("MSH".to_string(), SegmentDefinition {
        name: "Message Header".to_string(),
        required: true,
        repeating: false,
        fields: msh_fields,
    });

    // Define PID segment requirements
    let mut pid_fields = HashMap::new();
    pid_fields.insert(3, FieldDefinition {
        name: "Patient Identifier List".to_string(),
        data_type: "CX".to_string(),
        required: true,  // MRN is required in our system
        repeating: true,
        max_length: Some(250),
        table_id: None,
    });
    pid_fields.insert(5, FieldDefinition {
        name: "Patient Name".to_string(),
        data_type: "XPN".to_string(),
        required: true,  // Name is required
        repeating: true,
        max_length: Some(250),
        table_id: None,
    });
    pid_fields.insert(8, FieldDefinition {
        name: "Administrative Sex".to_string(),
        data_type: "IS".to_string(),
        required: false,
        repeating: false,
        max_length: Some(1),
        table_id: Some("0001".to_string()),  // Sex code table
    });

    schema.segments.insert("PID".to_string(), SegmentDefinition {
        name: "Patient Identification".to_string(),
        required: true,
        repeating: false,
        fields: pid_fields,
    });

    schema
}

// Use custom schema
let schema = create_custom_schema();
let validator = Validator::with_schema(Version::V2_5, schema);
let result = validator.validate(&message);
```

## Layer 2: Data Type Validation

VynaHL7 validates field values against HL7 data type specifications:

```rust
use vyna_hl7_core::types::DataType;
use vyna_hl7_validator::validate_data_type;

fn main() {
    // Date validation (DT)
    let result = validate_data_type("20240315", DataType::DT);
    assert!(result.is_valid());

    let result = validate_data_type("20241301", DataType::DT);  // Invalid month
    assert!(!result.is_valid());
    println!("Error: {}", result.error_message().unwrap());

    // Time validation (TM)
    let result = validate_data_type("143000", DataType::TM);
    assert!(result.is_valid());

    let result = validate_data_type("2530", DataType::TM);  // Invalid hour
    assert!(!result.is_valid());

    // Numeric validation (NM)
    let result = validate_data_type("123.45", DataType::NM);
    assert!(result.is_valid());

    let result = validate_data_type("abc", DataType::NM);
    assert!(!result.is_valid());
}
```

### Supported Data Types

| Data Type | Description | Valid Examples | Invalid Examples |
|-----------|-------------|----------------|------------------|
| **DT** | Date | `20240315`, `202403`, `2024` | `20241301`, `2024031` |
| **TM** | Time | `14`, `1430`, `143000`, `143000.123` | `2530`, `1460` |
| **DTM/TS** | Timestamp | `20240315143000`, `20240315143000.1234` | `202403151` |
| **NM** | Numeric | `123`, `123.45`, `-123.45`, `+123` | `abc`, `12.34.56` |
| **SI** | Sequence ID | `1`, `123` | `0`, `-1`, `abc` |
| **ID** | Identifier | `ABC123`, `test_id`, `test-id` | `test id`, `test@id` |
| **MSG** | Message Type | `ADT^A01`, `ORU^R01^ORU_R01` | `adt^A01`, `ad^A01` |
| **PT** | Processing Type | `P`, `D`, `T` | `X`, `Production` |
| **NA** | Numeric Array | `1~2~3`, `1.5~2.7~3.9` | `1~abc~3` |

### Date/Time Validation Examples

```rust
use vyna_hl7_core::types::DataType;
use vyna_hl7_validator::validate_data_type;

// Dates support multiple precisions
validate_data_type("2024", DataType::DT);        // Year only - valid
validate_data_type("202403", DataType::DT);      // Year-month - valid
validate_data_type("20240315", DataType::DT);    // Full date - valid

// Times support multiple precisions
validate_data_type("14", DataType::TM);          // Hours only - valid
validate_data_type("1430", DataType::TM);        // Hours:minutes - valid
validate_data_type("143022", DataType::TM);      // Full time - valid
validate_data_type("143022.1234", DataType::TM); // With fractions - valid

// Timestamps combine date and time
validate_data_type("20240315143022", DataType::DTM);       // Full timestamp
validate_data_type("20240315143022.1234", DataType::DTM);  // With fractions
validate_data_type("20240315143022-0500", DataType::DTM);  // With timezone
```

## Layer 3: Vocabulary Validation

HL7 defines standard code tables for many fields. VynaHL7 validates values against these tables:

### Common HL7 Tables

| Table | Field | Description | Valid Values |
|-------|-------|-------------|--------------|
| 0001 | PID-8 | Administrative Sex | M, F, O, U, A, N |
| 0002 | PID-17 | Marital Status | A, D, M, S, W, ... |
| 0004 | PV1-2 | Patient Class | I, O, E, P, R, B |
| 0063 | OBR-25 | Result Status | O, I, S, A, P, C, R, F, X |
| 0078 | OBX-8 | Abnormal Flags | L, H, LL, HH, N, A, ... |
| 0085 | OBX-11 | Observation Result Status | D, F, I, N, O, P, R, S, U, W, X |
| 0103 | MSH-11 | Processing ID | P, D, T |

### Vocabulary Validation Example

```rust
use vyna_hl7_validator::{Validator, MessageSchema, SegmentDefinition, FieldDefinition};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create schema with table references
    let mut schema = create_schema_with_tables();

    // Valid message - gender is "M" (valid for table 0001)
    let valid_msg = create_message_with_gender("M");
    let validator = Validator::with_schema(Version::V2_5, schema.clone());
    let result = validator.validate(&valid_msg);
    assert!(result.is_valid());

    // Invalid message - gender is "X" (not in table 0001)
    let invalid_msg = create_message_with_gender("X");
    let result = validator.validate(&invalid_msg);
    assert!(!result.is_valid());

    for error in &result.errors {
        println!("Error: {} at {}", error.message, error.location);
        // Output: "Value 'X' is not valid for table 0001 at PID-8"
    }

    Ok(())
}

fn create_schema_with_tables() -> MessageSchema {
    let mut schema = MessageSchema {
        message_type: "ADT".to_string(),
        trigger_event: "A01".to_string(),
        version: "2.5".to_string(),
        segments: HashMap::new(),
    };

    let mut pid_fields = HashMap::new();
    pid_fields.insert(8, FieldDefinition {
        name: "Administrative Sex".to_string(),
        data_type: "IS".to_string(),
        required: false,
        repeating: false,
        max_length: Some(1),
        table_id: Some("0001".to_string()),  // References Table 0001
    });

    schema.segments.insert("PID".to_string(), SegmentDefinition {
        name: "Patient Identification".to_string(),
        required: true,
        repeating: false,
        fields: pid_fields,
    });

    schema
}
```

## Validation Results

The `ValidationResult` structure provides detailed information:

```rust
pub struct ValidationResult {
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

pub struct ValidationIssue {
    pub location: String,    // e.g., "PID-8", "OBX(2)-5"
    pub message: String,     // Human-readable description
    pub severity: Severity,  // Error or Warning
    pub code: String,        // Machine-readable code
}
```

### Working with Results

```rust
let result = validator.validate(&message);

// Quick check
if result.is_valid() {
    process_message(&message);
} else {
    reject_message(&message, &result.errors);
}

// Detailed inspection
for error in &result.errors {
    log::error!(
        "[{}] {} at {} (code: {})",
        error.severity,
        error.message,
        error.location,
        error.code
    );
}

// Count by severity
let error_count = result.errors.len();
let warning_count = result.warnings.len();
println!("Validation: {} errors, {} warnings", error_count, warning_count);

// Filter specific errors
let missing_required: Vec<_> = result.errors
    .iter()
    .filter(|e| e.code == "REQUIRED_FIELD_MISSING")
    .collect();
```

## Real-World Example: Lab Result Validator

Here's a complete example validating incoming lab results:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_core::Version;
use vyna_hl7_validator::Validator;
use vyna_hl7_terser::Terser;

#[derive(Debug)]
enum ValidationDecision {
    Accept,
    AcceptWithWarnings(Vec<String>),
    Reject(Vec<String>),
}

fn validate_lab_result(hl7: &str) -> ValidationDecision {
    // Parse message
    let message = match parse_message(hl7) {
        Ok(msg) => msg,
        Err(e) => return ValidationDecision::Reject(vec![format!("Parse error: {}", e)]),
    };

    // Standard HL7 validation
    let validator = Validator::new(Version::V2_5);
    let result = validator.validate(&message);

    // Business rule validation
    let mut business_errors = Vec::new();
    let terser = Terser::new(&message);

    // Rule 1: Must be ORU^R01
    if let Some((msg_type, trigger)) = message.get_message_type() {
        if msg_type != "ORU" || trigger != "R01" {
            business_errors.push(format!(
                "Expected ORU^R01, got {}^{}", msg_type, trigger
            ));
        }
    }

    // Rule 2: Patient MRN required
    if terser.get("PID-3-1").ok().flatten().is_none() {
        business_errors.push("Patient MRN (PID-3-1) is required".to_string());
    }

    // Rule 3: At least one OBX segment required
    if message.get_segments_by_id("OBX").is_empty() {
        business_errors.push("At least one OBX segment required".to_string());
    }

    // Rule 4: All OBX results must have status
    for (i, obx) in message.get_segments_by_id("OBX").iter().enumerate() {
        if obx.get_field_value(11).is_none() {
            business_errors.push(format!(
                "OBX({}) missing result status (OBX-11)", i + 1
            ));
        }
    }

    // Combine results
    let all_errors: Vec<String> = result.errors
        .iter()
        .map(|e| format!("{}: {}", e.location, e.message))
        .chain(business_errors)
        .collect();

    let warnings: Vec<String> = result.warnings
        .iter()
        .map(|w| format!("{}: {}", w.location, w.message))
        .collect();

    if !all_errors.is_empty() {
        ValidationDecision::Reject(all_errors)
    } else if !warnings.is_empty() {
        ValidationDecision::AcceptWithWarnings(warnings)
    } else {
        ValidationDecision::Accept
    }
}

fn main() {
    let valid_oru = r"MSH|^~\&|LAB|HOSP|EMR|CLINIC|20240315||ORU^R01|MSG001|P|2.5
PID|1||123456^^^MRN||DOE^JOHN||19800515|M
OBR|1||RES001|CBC^Complete Blood Count|||20240315090000
OBX|1|NM|WBC^White Blood Count||7.5|10*3/uL|4.0-11.0|N|||F";

    match validate_lab_result(valid_oru) {
        ValidationDecision::Accept => println!("Message accepted"),
        ValidationDecision::AcceptWithWarnings(w) => {
            println!("Accepted with warnings:");
            for warning in w {
                println!("  - {}", warning);
            }
        }
        ValidationDecision::Reject(errors) => {
            println!("Message rejected:");
            for error in errors {
                println!("  - {}", error);
            }
        }
    }
}
```

## Best Practices

### 1. Validate Early

Validate messages as soon as they're received, before any processing:

```rust
async fn receive_message(hl7: &str) -> Result<(), Error> {
    let message = parse_message(hl7)?;

    let validator = Validator::new(Version::V2_5);
    let result = validator.validate(&message);

    if !result.is_valid() {
        // Send NAK (negative acknowledgment)
        return Err(Error::ValidationFailed(result.errors));
    }

    // Process valid message
    process_message(message).await
}
```

### 2. Log Validation Issues

Always log validation failures for debugging:

```rust
if !result.is_valid() {
    log::warn!(
        "Validation failed for message {} from {}: {:?}",
        message.get_control_id().unwrap_or("unknown"),
        message.get_sending_application().unwrap_or("unknown"),
        result.errors
    );
}
```

### 3. Use Appropriate Strictness

Different scenarios need different validation levels:

```rust
// Strict (the default): production systems receiving from known senders
let strict_validator = Validator::new(Version::V2_5);

// Standard: pinned-version feeds whose senders drift in MSH-12
let standard_validator = Validator::new(Version::V2_5)
    .with_context(ValidationContext::standard());

// Lenient: Testing or receiving from legacy systems
let lenient_validator = Validator::new(Version::V2_5)
    .with_context(ValidationContext::lenient());
```

### 4. Combine with Business Rules

HL7 validation ensures technical compliance; add your own business rules:

```rust
fn validate_for_our_system(message: &Message) -> ValidationResult {
    let mut result = Validator::new(Version::V2_5).validate(message);

    // Add organization-specific rules
    let terser = Terser::new(message);

    // Our system requires attending physician
    if terser.get("PV1-7").ok().flatten().is_none() {
        result.errors.push(ValidationIssue {
            location: "PV1-7".to_string(),
            message: "Attending physician required".to_string(),
            severity: Severity::Error,
            code: "BUSINESS_RULE_001".to_string(),
        });
    }

    result
}
```

## Summary

VynaHL7's validation architecture ensures your HL7 messages are compliant:

- **Schema validation** checks message structure
- **Data type validation** ensures proper formatting
- **Vocabulary validation** verifies code values

Combined with custom business rules, you can build robust validation pipelines that catch errors early and maintain data quality.

---

*Next in series: [Custom Z-Segments: Extending HL7 for Your Organization](./07-custom-z-segments.md)*

*Previous: [The Terser API](./05-the-terser-api.md)*
