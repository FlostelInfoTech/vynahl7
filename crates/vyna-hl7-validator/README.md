# vyna-hl7-validator

Comprehensive HL7 v2.x message validation for Rust.

## Overview

`vyna-hl7-validator` provides multi-layer validation for HL7 messages including schema validation, data type validation, vocabulary validation, and business rules validation.

## Features

- **Schema Validation**: Validate message structure against HL7 standards
- **Data Type Validation**: Verify field formats (dates, times, numbers, coded values)
- **Vocabulary Validation**: Check codes against HL7 tables
- **Business Rules Engine**: Custom validation rules with severity levels
- **43 Message Schemas**: Support for ADT, ORU, ORM, SIU, MDM, DFT, QRY, BAR, RDE, RAS, RDS, RGV, RRA, RRD, OUL, OML, MFN, ACK message types
- **Complete Field Definitions**: All segment fields with component definitions for composite data types
- **HL7 Versions**: 2.3, 2.3.1, 2.4, 2.5, 2.5.1, 2.6, 2.7, 2.7.1, 2.8, 2.8.1, 2.8.2

## Installation

```toml
[dependencies]
vyna-hl7-validator = "0.9"
```

## Quick Start

```rust
use vyna_hl7_validator::Validator;
use vyna_hl7_parser::parse_message;

let hl7 = "MSH|^~\\&|APP|FAC|EMR|HOSP|20231010120000||ADT^A01|MSG001|P|2.5\r\
           PID|1||12345^^^MR||Doe^John||19800115|M";

let message = parse_message(hl7)?;

// Create validator for ADT^A01 message in HL7 v2.5
let validator = Validator::for_message("ADT_A01", "2.5")?;

// Validate message
let result = validator.validate(&message)?;

if result.is_valid() {
    println!("Message is valid!");
} else {
    for error in &result.errors {
        eprintln!("Error: {}", error.message);
    }
}
```

## Schema Validation

Validates message structure against HL7 specifications:

```rust
use vyna_hl7_validator::Validator;

let validator = Validator::for_message("ADT_A01", "2.5")?;
let result = validator.validate(&message)?;

// Check for required segments
// Verify segment order
// Validate field presence
```

## Data Type Validation

Validates field formats according to HL7 data types:

```rust
use vyna_hl7_validator::datatype::DataTypeValidator;

let dt_validator = DataTypeValidator::new();

// Validate date (DT type)
assert!(dt_validator.validate_dt("20231010").is_ok());
assert!(dt_validator.validate_dt("2023-10-10").is_err());  // Invalid format

// Validate time (TM type)
assert!(dt_validator.validate_tm("120000").is_ok());
assert!(dt_validator.validate_tm("12:00:00").is_err());  // Invalid format

// Validate datetime (DTM/TS type)
assert!(dt_validator.validate_dtm("20231010120000").is_ok());

// Validate numeric (NM type)
assert!(dt_validator.validate_nm("123.45").is_ok());
assert!(dt_validator.validate_nm("ABC").is_err());
```

## Vocabulary Validation

Validate codes against HL7 tables:

```rust
use vyna_hl7_validator::vocabulary::TableRegistry;

let registry = TableRegistry::new();

// Validate against Table 0001 (Administrative Sex)
assert!(registry.is_valid_code("0001", "M").unwrap());  // Male
assert!(registry.is_valid_code("0001", "F").unwrap());  // Female
assert!(!registry.is_valid_code("0001", "X").unwrap()); // Invalid

// Validate against Table 0004 (Patient Class)
assert!(registry.is_valid_code("0004", "I").unwrap());  // Inpatient
assert!(registry.is_valid_code("0004", "O").unwrap());  // Outpatient
assert!(registry.is_valid_code("0004", "E").unwrap());  // Emergency
```

## Business Rules Validation

Create custom validation rules:

```rust
use vyna_hl7_validator::rules::{RulesEngine, ValidationRule, RuleSeverity};
use vyna_hl7_terser::Terser;

let mut engine = RulesEngine::new();

// Add rule: If patient class is 'I' (Inpatient), assigned location is required
let rule = ValidationRule::new(
    "inpatient_requires_location",
    "Inpatient admission must have assigned location",
    RuleSeverity::Error
).with_condition(|msg| {
    let terser = Terser::new(msg);
    let patient_class = terser.get("PV1-2").unwrap_or_default();
    let location = terser.get("PV1-3").unwrap_or_default();

    if patient_class == "I" && location.is_empty() {
        return false;  // Rule violated
    }
    true
});

engine.add_rule(rule);

// Validate message against rules
let result = engine.validate(&message);
```

## Complete Validation

Combine all validation layers:

```rust
use vyna_hl7_validator::Validator;

let mut validator = Validator::for_message("ADT_A01", "2.5")?
    .with_datatype_validation(true)
    .with_vocabulary_validation(true);

// Add custom rules
validator.add_rule(custom_rule);

// Perform complete validation
let result = validator.validate(&message)?;

// Check results
println!("Valid: {}", result.is_valid());
println!("Errors: {}", result.errors.len());
println!("Warnings: {}", result.warnings.len());
```

## Built-in HL7 Tables

13 built-in HL7 tables are included:

- Table 0001: Administrative Sex
- Table 0002: Marital Status
- Table 0004: Patient Class
- Table 0007: Admission Type
- Table 0061: Check Digit Scheme
- Table 0063: Relationship
- Table 0078: Interpretation Codes
- Table 0085: Observation Result Status
- Table 0103: Processing ID
- Table 0119: Order Control Codes
- Table 0201: Telecommunication Use Code
- Table 0203: Identifier Type
- Table 0301: Universal ID Type

## Cross-Field Validation

Validate field dependencies:

```rust
use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;

// If PID-8 (sex) is valued, then it must be M, F, O, U, or A
let rule = CrossFieldValidator::if_then("PID-8", "M", "PV1-2")
    .build();

// At least one of PID-3, PID-4, or PID-18 must be valued
let rule = CrossFieldValidator::at_least_one(vec!["PID-3", "PID-4", "PID-18"])
    .build();

// All or none: If any of PID-11 components are valued, all must be valued
let rule = CrossFieldValidator::all_or_none(vec![
    "PID-11-1",  // Street
    "PID-11-3",  // City
    "PID-11-4",  // State
    "PID-11-5"   // Zip
]).build();
```

## Declarative Rules (YAML/JSON)

Load validation rules from configuration files:

```yaml
rules:
  - name: "pid_patient_id_required"
    description: "Patient ID (PID-3) is required"
    severity: "Error"
    condition:
      type: "FieldValued"
      field: "PID-3"

  - name: "inpatient_requires_location"
    description: "Inpatient must have location"
    severity: "Error"
    condition:
      type: "IfThen"
      if_field: "PV1-2"
      if_value: "I"
      then_field: "PV1-3"
```

```rust
use vyna_hl7_validator::rules::declarative::RuleConfig;

let config = RuleConfig::from_yaml_file("rules.yaml")?;
let rules = config.into_validation_rules();

for rule in rules {
    engine.add_rule(rule);
}
```

## Related Crates

- **vyna-hl7-parser**: Parse HL7 messages before validation
- **vyna-hl7-terser**: Access fields for custom validation rules
- **vyna-hl7-conformance**: Advanced conformance profile validation

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
