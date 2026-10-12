# VynaHL7-FHIR Examples

This directory contains practical examples demonstrating how to use vyna-hl7-fhir to convert HL7 v2 messages to FHIR resources.

## Available Examples

### 1. `convert_adt.rs` - Patient Admission (ADT^A01)

Demonstrates converting an HL7 v2 ADT (Admit/Discharge/Transfer) message to a FHIR Patient resource with complete demographic information.

**Features:**
- Multiple patient identifiers (MRN, SSN)
- Name with prefix, given names, family name, and suffix
- Multiple addresses (home, work)
- Multiple contact points (phone numbers)
- Gender and birth date
- Marital status

**Run:**
```bash
cargo run --example convert_adt -p vyna-hl7-fhir
```

### 2. `convert_oru.rs` - Laboratory Results (ORU^R01)

Demonstrates converting a complete HL7 v2 laboratory result message to FHIR Patient, Observation, and Practitioner resources.

**Features:**
- Patient demographics from PID segment
- Multiple observations from OBX segments (numeric and text values)
- Observation codes with LOINC system mapping
- Reference ranges and interpretations
- Practitioner information from PV1 segment
- Observation performers from OBX-16

**Run:**
```bash
cargo run --example convert_oru -p vyna-hl7-fhir
```

## Understanding the Output

### Patient Resource

The FHIR Patient resource includes:
- **identifier**: Patient identifiers (MRN, SSN, etc.) with proper typing
- **name**: Human names with components (prefix, given, family, suffix)
- **telecom**: Contact points (phone, email) with use codes (home, work)
- **gender**: Administrative gender (male, female, other, unknown)
- **birthDate**: Date of birth in YYYY-MM-DD format
- **address**: Addresses with use codes and full components
- **maritalStatus**: Marital status code

### Observation Resource

The FHIR Observation resource includes:
- **code**: Observation identifier (typically LOINC) with text
- **status**: Observation status (final, preliminary, etc.)
- **value[x]**: Observation value (can be numeric, string, coded, etc.)
- **interpretation**: Abnormal flags (normal, high, low, etc.)
- **referenceRange**: Normal reference ranges
- **effectiveDateTime**: When the observation was made
- **performer**: Who performed or verified the observation

### Practitioner Resource

The FHIR Practitioner resource includes:
- **identifier**: Practitioner identifiers (NPI, local IDs)
- **name**: Practitioner name with components
- **qualification**: Degrees and certifications (MD, DO, etc.)

## Code Structure

Each example follows this pattern:

```rust
use vyna_hl7_fhir::converters::{PatientConverter, ObservationConverter, PractitionerConverter};
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Define HL7 v2 message
    let hl7_message = "MSH|^~\\&|...";

    // 2. Parse the message
    let message = parse_message(hl7_message)?;

    // 3. Convert to FHIR resources
    let patient = PatientConverter::convert(&message)?;
    let observations = ObservationConverter::convert_all(&message)?;
    let practitioner = PractitionerConverter::convert_attending_doctor(&message)?;

    // 4. Serialize to JSON
    let json = serde_json::to_string_pretty(&patient)?;
    println!("{}", json);

    Ok(())
}
```

## Creating Your Own Converters

To convert your own HL7 v2 messages:

1. **Parse the message:**
   ```rust
   use vyna_hl7_parser::parse_message;
   let message = parse_message(your_hl7_string)?;
   ```

2. **Choose the appropriate converter:**
   - `PatientConverter::convert()` for PID segments
   - `ObservationConverter::convert_all()` for all OBX segments
   - `ObservationConverter::convert_single()` for a specific OBX
   - `PractitionerConverter::convert_attending_doctor()` for PV1-7
   - `PractitionerConverter::convert_referring_doctor()` for PV1-8
   - `PractitionerConverter::extract_all_practitioners()` for all practitioners

3. **Serialize to JSON:**
   ```rust
   let json = serde_json::to_string_pretty(&resource)?;
   ```

## Supported Message Types

Currently supported HL7 v2 message types:

| Message Type | Segments | FHIR Resources |
|--------------|----------|----------------|
| ADT^A01..A47 | PID, PV1 | Patient, Practitioner |
| ORU^R01 | PID, PV1, OBR, OBX | Patient, Observation, Practitioner |
| MDM^T01..T11 | PID, PV1 | Patient, Practitioner |

## Data Mapping

### Key HL7 v2 to FHIR Mappings

#### Patient (PID → Patient)
- PID-3 → Patient.identifier
- PID-5 → Patient.name
- PID-7 → Patient.birthDate
- PID-8 → Patient.gender
- PID-11 → Patient.address
- PID-13/14 → Patient.telecom
- PID-16 → Patient.maritalStatus

#### Observation (OBX → Observation)
- OBX-3 → Observation.code
- OBX-5 → Observation.value[x]
- OBX-6 → Observation.valueQuantity.unit
- OBX-7 → Observation.referenceRange
- OBX-8 → Observation.interpretation
- OBX-11 → Observation.status
- OBX-14 → Observation.effectiveDateTime
- OBX-16 → Observation.performer

#### Practitioner (PV1/OBX → Practitioner)
- PV1-7/8/9 → Practitioner (attending/referring/consulting)
- XCN-1 → Practitioner.identifier
- XCN-2..6 → Practitioner.name
- XCN-7 → Practitioner.qualification

## Error Handling

All converters return `Result` types and will return errors for:
- Missing required segments (e.g., PID for Patient)
- Missing required fields (e.g., OBX-3 for Observation)
- Invalid date formats
- Parse errors

Handle errors appropriately:

```rust
match PatientConverter::convert(&message) {
    Ok(patient) => {
        // Use the patient resource
    }
    Err(e) => {
        eprintln!("Conversion failed: {}", e);
    }
}
```

## Further Reading

- [FHIR R4 Specification](https://hl7.org/fhir/R4/)
- [HL7 v2 to FHIR Mapping](https://build.fhir.org/ig/HL7/v2-to-fhir/)
- [VynaHL7 Documentation](../../README.md)
