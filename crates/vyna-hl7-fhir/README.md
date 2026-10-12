# vyna-hl7-fhir - HL7 v2 to FHIR R4 Conversion

Converter library for transforming HL7 v2.x messages to FHIR R4 resources.

## Status

✅ **Core Functionality Complete** - Production-ready converters with comprehensive testing

### Completed ✅
- **12 FHIR R4 resource definitions:**
  - Patient, Observation, Practitioner, Encounter
  - DiagnosticReport, AllergyIntolerance, Medication/MedicationAdministration
  - Condition, Procedure, Immunization, ServiceRequest, Specimen
- Common FHIR data types (HumanName, Address, ContactPoint, Identifier, CodeableConcept, Period, etc.)
- **12 Production-ready converters:**
  - Patient (PID → Patient) - **100% tested**
  - Observation (OBX → Observation) - **100% tested**
  - Practitioner (PV1/ORC → Practitioner) - **100% tested**
  - Encounter (PV1 → Encounter) - **100% tested**
  - DiagnosticReport (OBR → DiagnosticReport) - **100% tested**
  - AllergyIntolerance (AL1 → AllergyIntolerance) - **100% tested**
  - MedicationAdministration (RXA → MedicationAdministration) - **100% tested**
  - Condition (PRB/DG1 → Condition) - **100% tested**
  - Procedure (PR1 → Procedure) - **100% tested**
  - Immunization (RXA → Immunization) - **100% tested** ✨ NEW
  - ServiceRequest (ORC → ServiceRequest) - **100% tested** ✨ NEW
  - Specimen (SPM → Specimen) - **100% tested** ✨ NEW
- Error handling and conversion result types
- Complete test suite - **All 33 tests passing ✅**
- Terser 0-based component indexing - **Fixed and documented** (see TERSER_INDEXING.md)
- **Working examples** - ADT, ORU, VXU, ORM, and OML message conversion demos (see EXAMPLES.md)

### Future Enhancements 📋
- Additional resources (CarePlan, Goal, CareTeam, etc.)
- Batch/Bundle processing for multiple resources
- Performance optimization for large message volumes
- Extended validation and conformance checking
- FHIR Questionnaire/QuestionnaireResponse support

## Architecture

```
vyna-hl7-fhir/
├── src/
│   ├── lib.rs                 # Main library entry point
│   ├── error.rs               # Conversion error types
│   ├── resources/
│   │   ├── mod.rs            # Resource module exports
│   │   ├── common.rs         # Common FHIR data types
│   │   ├── patient.rs        # FHIR Patient resource
│   │   ├── observation.rs    # FHIR Observation resource
│   │   └── practitioner.rs   # FHIR Practitioner resource
│   └── converters/
│       ├── mod.rs            # Converter module exports
│       ├── patient.rs        # PID → Patient converter
│       ├── observation.rs    # OBX → Observation converter
│       └── practitioner.rs   # PV1/ORC → Practitioner converter
├── examples/
│   ├── convert_adt.rs        # ADT^A01 patient admission example
│   ├── convert_oru.rs        # ORU^R01 laboratory results example
│   ├── convert_vxu.rs        # VXU^V04 immunization record example ✨ NEW
│   ├── convert_orm.rs        # ORM^O01 laboratory order example ✨ NEW
│   └── convert_oml.rs        # OML^O21 specimen collection example ✨ NEW
├── TERSER_INDEXING.md        # Component indexing documentation (0-based)
├── EXAMPLES.md               # Detailed examples documentation
└── README.md                 # This file
```

## Usage

```rust
use vyna_hl7_fhir::prelude::*;
use vyna_hl7_parser::parse_message;

// Parse HL7 v2 message
let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\n\
           PID|1||67890^^^MRN||DOE^JOHN^A||19800101|M";

let message = parse_message(hl7)?;

// Convert to FHIR Patient
let patient = PatientConverter::convert(&message)?;

// Serialize to JSON
let json = serde_json::to_string_pretty(&patient)?;
println!("{}", json);
```

## Converters

All converters use 0-based component indexing as documented in TERSER_INDEXING.md.

### PatientConverter (PID → Patient)
- PID-3 → identifier, PID-5 → name, PID-7 → birthDate
- PID-8 → gender, PID-11 → address, PID-13/14 → telecom
- PID-16 → maritalStatus, PID-24 → multipleBirth, PID-29/30 → deceased

### ObservationConverter (OBX → Observation)
- OBX-3 → code, OBX-5 → value[x], OBX-6 → valueQuantity.unit
- OBX-7 → referenceRange, OBX-8 → interpretation
- OBX-11 → status, OBX-14 → effectiveDateTime, OBX-16 → performer

### PractitionerConverter (PV1/ORC → Practitioner)
- PV1-7 → Attending Doctor, PV1-8 → Referring Doctor, PV1-9 → Consulting Doctor
- ORC-12 → Ordering Provider
- XCN components → name, identifier, qualification

### EncounterConverter (PV1 → Encounter)
- PV1-2 → class, PV1-3 → location, PV1-4 → type
- PV1-7/8/9 → participant (attending/referring/consulting)
- PV1-10 → serviceProvider, PV1-14/36 → hospitalization
- PV1-19 → identifier, PV1-44/45 → period

### DiagnosticReportConverter (OBR → DiagnosticReport)
- OBR-2 → identifier, OBR-4 → code, OBR-7 → effectiveDateTime
- OBR-22 → issued, OBR-25 → status
- Links to Observations (result), Patient (subject)

### AllergyIntoleranceConverter (AL1 → AllergyIntolerance)
- AL1-2 → category (allergen type), AL1-3 → code
- AL1-4 → criticality (severity), Links to Patient

### MedicationConverter (RXA → MedicationAdministration)
- RXA-3 → effectiveDateTime, RXA-5 → medicationCodeableConcept
- RXA-6/7 → dosage.dose, RXA-20 → status
- Links to Patient (subject)

### ConditionConverter (PRB/DG1 → Condition)
- PRB-3 or DG1-3 → code, Links to Patient (subject)
- Sets clinicalStatus to active

### ProcedureConverter (PR1 → Procedure)
- PR1-3 → code, PR1-5 → performedDateTime
- Links to Patient (subject), Default status: completed

### ImmunizationConverter (RXA → Immunization) ✨ NEW
- RXA-3 → occurrenceDateTime, RXA-5 → vaccineCode (CVX system)
- RXA-6/7 → doseQuantity, RXA-9 → primarySource/reportOrigin
- RXA-10 → performer, RXA-11 → site, RXA-15 → lotNumber
- RXA-16 → expirationDate, RXA-17 → manufacturer
- RXA-20 → status (CP→completed, RE→not-done), RXA-27 → location
- Links to Patient (subject), Encounter (PV1-19)

### ServiceRequestConverter (ORC/OBR → ServiceRequest) ✨ NEW
- ORC-2 → identifier (placer order), ORC-3 → identifier (filler order)
- ORC-4 → requisition, ORC-5 → status (A→active, CA→revoked, CM→completed)
- ORC-12 → requester, OBR-4 → code (service identifier)
- OBR-5 or TQ1-9 → priority (S→stat, A→asap, R→routine)
- OBR-13 → note, OBR-15 → specimen, OBR-31 → reasonCode
- Links to Patient (subject), Encounter (PV1-19)

### SpecimenConverter (SPM → Specimen) ✨ NEW
- SPM-2 → identifier, SPM-4 → type (specimen type code)
- SPM-7 → collection.method, SPM-8 → collection.bodySite
- SPM-12 → collection.quantity, SPM-17 → collection.collectedDateTime
- SPM-18 → receivedTime, SPM-20 → status (Y→available, N→unavailable)
- SPM-24 → condition, SPM-30 → accessionIdentifier
- Links to Patient (subject)

## Data Type Conversions

### Date/Time
- HL7 YYYYMMDD → FHIR YYYY-MM-DD
- HL7 YYYYMMDDHHMMSS → FHIR YYYY-MM-DDTHH:MM:SS

### Gender Codes
- M → male
- F → female
- O → other
- U/A → unknown

### Coding Systems
- LN/LNC → http://loinc.org
- SNM/SCT → http://snomed.info/sct
- ICD9 → http://hl7.org/fhir/sid/icd-9-cm
- ICD10 → http://hl7.org/fhir/sid/icd-10
- CPT → http://www.ama-assn.org/go/cpt

## Examples

See `EXAMPLES.md` for detailed examples and usage patterns.

### Quick Start

Run the included examples to see the converters in action:

```bash
# Convert an ADT^A01 patient admission message
cargo run --example convert_adt -p vyna-hl7-fhir

# Convert an ORU^R01 laboratory results message
cargo run --example convert_oru -p vyna-hl7-fhir

# Convert a VXU^V04 immunization record message
cargo run --example convert_vxu -p vyna-hl7-fhir

# Convert an ORM^O01 laboratory order message
cargo run --example convert_orm -p vyna-hl7-fhir

# Convert an OML^O21 specimen collection message
cargo run --example convert_oml -p vyna-hl7-fhir
```

## Component Indexing

⚠️ **Important**: The Terser API uses 0-based component indexing, not 1-based HL7 standard numbering.

For example:
- HL7 PID-5-1 (Family Name) → Terser path `PID-5` or `PID-5-0`
- HL7 PID-5-2 (Given Name) → Terser path `PID-5-1`

See `TERSER_INDEXING.md` for complete details. All converters have been updated and tested with correct 0-based indexing.

## Testing

```bash
# Run all tests
cargo test -p vyna-hl7-fhir

# Run specific converter tests
cargo test -p vyna-hl7-fhir patient
cargo test -p vyna-hl7-fhir observation
cargo test -p vyna-hl7-fhir practitioner

# Run with output
cargo test -p vyna-hl7-fhir -- --nocapture
```

## Dependencies

- vyna-hl7-core: Core HL7 data structures
- vyna-hl7-parser: HL7 message parser
- vyna-hl7-terser: Path-based field access
- serde/serde_json: JSON serialization
- chrono: Date/time handling
- thiserror: Error handling

## References

- [HL7 v2-to-FHIR Implementation Guide](https://build.fhir.org/ig/HL7/v2-to-fhir/)
- [FHIR R4 Specification](https://www.hl7.org/fhir/R4/)
- [HL7 v2.x Standard](https://www.hl7.org/implement/standards/product_brief.cfm?product_id=185)
