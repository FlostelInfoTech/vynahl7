# VynaHL7-FHIR Converters Reference

Complete guide to all available HL7 v2 to FHIR converters.

## Overview

VynaHL7-FHIR provides 9 production-ready converters for transforming HL7 v2.x messages to FHIR R4 resources.

**Test Coverage:** 16/16 tests passing ✅

## Available Converters

| Converter | HL7 Segment(s) | FHIR Resource | Tests |
|-----------|----------------|---------------|-------|
| PatientConverter | PID | Patient | 2 |
| ObservationConverter | OBX | Observation | 3 |
| PractitionerConverter | PV1, ORC | Practitioner | 3 |
| EncounterConverter | PV1 | Encounter | 3 |
| DiagnosticReportConverter | OBR | DiagnosticReport | 1 |
| AllergyIntoleranceConverter | AL1 | AllergyIntolerance | 1 |
| MedicationConverter | RXA | MedicationAdministration | 1 |
| ConditionConverter | PRB, DG1 | Condition | 1 |
| ProcedureConverter | PR1 | Procedure | 1 |

## Detailed Mappings

### 1. PatientConverter

**Input:** PID (Patient Identification) segment
**Output:** FHIR Patient resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| PID-3 | Patient.identifier | Multiple identifiers supported with repetitions |
| PID-5 | Patient.name | Supports multiple names with all components |
| PID-7 | Patient.birthDate | Converted from YYYYMMDD to YYYY-MM-DD |
| PID-8 | Patient.gender | M→male, F→female, O→other, U/A→unknown, anything else→unknown (lossy default) |
| PID-11 | Patient.address | Multiple addresses with full components |
| PID-13 | Patient.telecom | Home phone |
| PID-14 | Patient.telecom | Work phone |
| PID-16 | Patient.maritalStatus | HL7 v3 MaritalStatus code system |
| PID-24 | Patient.multipleBirthBoolean | Y/N indicator |
| PID-29 | Patient.deceasedDateTime | Death date/time |
| PID-30 | Patient.deceasedBoolean | Death indicator |

#### Usage Example

```rust
use vyna_hl7_fhir::converters::PatientConverter;
use vyna_hl7_parser::parse_message;

let message = parse_message(hl7_string)?;
let patient = PatientConverter::convert(&message)?;
```

---

### 2. ObservationConverter

**Input:** OBX (Observation/Result) segment
**Output:** FHIR Observation resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| OBX-1 | Observation.id | Set ID for observation |
| OBX-2 | — | Determines which value[x] field to populate |
| OBX-3 | Observation.code | LOINC/SNOMED codes |
| OBX-5 | Observation.value[x] | Type varies based on OBX-2 |
| OBX-6 | Observation.valueQuantity.unit | For numeric values |
| OBX-7 | Observation.referenceRange | Normal range |
| OBX-8 | Observation.interpretation | N, H, L, etc. |
| OBX-11 | Observation.status | F→final, P→preliminary, etc.; unmapped codes→final (lossy default) |
| OBX-14 | Observation.effectiveDateTime | When observed |
| OBX-16 | Observation.performer | Responsible observer |

#### Supported Value Types (OBX-2)

- **NM** (Numeric) → valueQuantity
- **ST/TX/FT** (String/Text) → valueString
- **CE/CWE** (Coded) → valueCodeableConcept
- **DT** (Date) → valueDateTime
- **TM** (Time) → valueTime
- **TS/DTM** (Timestamp) → valueDateTime
- **Any other OBX-2 type** → valueString (lossy fallback)

#### Usage Example

```rust
// Convert all OBX segments in message
let observations = ObservationConverter::convert_all(&message)?;

// Convert specific OBX segment
let observation = ObservationConverter::convert_single(&message, obx_index)?;
```

---

### 3. PractitionerConverter

**Input:** PV1 (Patient Visit), ORC (Common Order) segments
**Output:** FHIR Practitioner resource

#### Field Mappings (XCN Data Type)

| HL7 Component | FHIR Element | Notes |
|---------------|--------------|-------|
| XCN-1 | Practitioner.identifier.value | ID number |
| XCN-2 | Practitioner.name.family | Family name |
| XCN-3 | Practitioner.name.given | Given name |
| XCN-4 | Practitioner.name.given | Middle name |
| XCN-5 | Practitioner.name.suffix | Suffix |
| XCN-6 | Practitioner.name.prefix | Prefix/title |
| XCN-7 | Practitioner.qualification | Degree (MD, DO, etc.) |
| XCN-9 | Practitioner.identifier.system | Assigning authority |
| XCN-13 | Practitioner.identifier.type | ID type code |

#### Conversion Methods

```rust
// PV1-7: Attending Doctor
let attending = PractitionerConverter::convert_attending_doctor(&message)?;

// PV1-8: Referring Doctor
let referring = PractitionerConverter::convert_referring_doctor(&message)?;

// PV1-9: Consulting Doctor
let consulting = PractitionerConverter::convert_consulting_doctor(&message)?;

// ORC-12: Ordering Provider
let ordering = PractitionerConverter::convert_ordering_provider(&message)?;

// All practitioners in message
let practitioners = PractitionerConverter::extract_all_practitioners(&message)?;
```

---

### 4. EncounterConverter

**Input:** PV1 (Patient Visit) segment
**Output:** FHIR Encounter resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| PV1-2 | Encounter.class | I→IMP, O→AMB, E→EMER; anything else→AMB (lossy default) |
| PV1-3 | Encounter.location | Assigned patient location |
| PV1-4 | Encounter.type | Admission type |
| PV1-7 | Encounter.participant | Attending doctor |
| PV1-8 | Encounter.participant | Referring doctor |
| PV1-9 | Encounter.participant | Consulting doctor |
| PV1-10 | Encounter.serviceProvider | Hospital service |
| PV1-14 | Encounter.hospitalization.admitSource | Admit source |
| PV1-19 | Encounter.identifier | Visit number |
| PV1-36 | Encounter.hospitalization.dischargeDisposition | Discharge disposition |
| PV1-44 | Encounter.period.start | Admit date/time |
| PV1-45 | Encounter.period.end | Discharge date/time |

#### Patient Class Mapping

| HL7 Code | FHIR Code | Display |
|----------|-----------|---------|
| I | IMP | inpatient encounter |
| O | AMB | ambulatory |
| E | EMER | emergency |
| P | PRENC | pre-admission |
| (other) | AMB | ambulatory (lossy default) |

---

### 5. DiagnosticReportConverter

**Input:** OBR (Observation Request) segment
**Output:** FHIR DiagnosticReport resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| OBR-2 | DiagnosticReport.identifier | Placer order number |
| OBR-4 | DiagnosticReport.code | Universal service ID |
| OBR-7 | DiagnosticReport.effectiveDateTime | Observation date/time |
| OBR-22 | DiagnosticReport.issued | Results report date/time |
| OBR-25 | DiagnosticReport.status | O→registered, P→preliminary, F→final; unmapped codes→final (lossy default) |

#### Automatic Linking

- Links to Patient from PID segment
- Links to all OBX segments as results

---

### 6. AllergyIntoleranceConverter

**Input:** AL1 (Patient Allergy Information) segment
**Output:** FHIR AllergyIntolerance resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| AL1-2 | AllergyIntolerance.category | DA→medication, FA→food, MA/EA→environment |
| AL1-3 | AllergyIntolerance.code | Allergen code/description |
| AL1-4 | AllergyIntolerance.criticality | MI/MO→low, SV→high |

#### Allergen Type Mapping

| HL7 Code | FHIR Category |
|----------|---------------|
| DA | medication |
| FA | food |
| MA | environment |
| EA | environment |

---

### 7. MedicationConverter

**Input:** RXA (Pharmacy/Treatment Administration) segment
**Output:** FHIR MedicationAdministration resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| RXA-3 | MedicationAdministration.effectiveDateTime | Date/time of administration |
| RXA-5 | MedicationAdministration.medicationCodeableConcept | Administered code |
| RXA-6 | MedicationAdministration.dosage.dose.value | Administered amount |
| RXA-7 | MedicationAdministration.dosage.dose.unit | Administered units |
| RXA-20 | MedicationAdministration.status | CP→completed, PA→stopped |

---

### 8. ConditionConverter

**Input:** PRB (Problem Details) or DG1 (Diagnosis) segments
**Output:** FHIR Condition resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| PRB-3 | Condition.code | Problem ID |
| DG1-3 | Condition.code | Diagnosis code (ICD-9/10) |

Both converters automatically:
- Link to Patient from PID segment
- Set clinicalStatus to "active"

---

### 9. ProcedureConverter

**Input:** PR1 (Procedures) segment
**Output:** FHIR Procedure resource

#### Field Mappings

| HL7 Field | FHIR Element | Notes |
|-----------|--------------|-------|
| PR1-3 | Procedure.code | Procedure code (ICD-9/10, CPT) |
| PR1-5 | Procedure.performedDateTime | Procedure date/time |

#### Coding System Mapping

| HL7 Code | FHIR System |
|----------|-------------|
| I9C, I9 | http://hl7.org/fhir/sid/icd-9-cm |
| I10, I10P | http://hl7.org/fhir/sid/icd-10-pcs |
| C4, CPT | http://www.ama-assn.org/go/cpt |

---

## Common Patterns

### Converting Complete Messages

Most converters support batch conversion:

```rust
// Convert all observations in a message
let observations = ObservationConverter::convert_all(&message)?;

// Convert all allergies
let allergies = AllergyIntoleranceConverter::convert_all(&message)?;

// Convert all medications
let medications = MedicationConverter::convert_all(&message)?;

// Convert all conditions
let conditions = ConditionConverter::convert_all(&message)?;

// Convert all procedures
let procedures = ProcedureConverter::convert_all(&message)?;

// Convert all diagnostic reports
let reports = DiagnosticReportConverter::convert_all(&message)?;
```

### Error Handling

All converters return `ConversionResult<T>`:

```rust
match PatientConverter::convert(&message) {
    Ok(patient) => println!("{}", serde_json::to_string_pretty(&patient)?),
    Err(ConversionError::MissingSegment(seg)) => {
        eprintln!("Required segment {} not found", seg);
    }
    Err(ConversionError::MissingField(field, segment)) => {
        eprintln!("Required field {} in segment {} missing", field, segment);
    }
    Err(e) => eprintln!("Conversion error: {}", e),
}
```

### Common Coding Systems

The converters automatically map HL7 v2 coding systems to FHIR URIs:

| HL7 Code | FHIR System URI |
|----------|-----------------|
| LN, LNC | http://loinc.org |
| SNM, SCT | http://snomed.info/sct |
| ICD9 | http://hl7.org/fhir/sid/icd-9-cm |
| ICD10 | http://hl7.org/fhir/sid/icd-10 |
| CPT | http://www.ama-assn.org/go/cpt |

## Component Indexing

Terser paths are 1-based throughout, matching the HL7 standard.

Example for PID-5 (Patient Name: "DOE^JOHN^A"):
- PID-5 or PID-5-1 → "DOE" (component 1)
- PID-5-2 → "JOHN" (component 2)
- PID-5-3 → "A" (component 3)

See `TERSER_INDEXING.md` for complete details.

## Testing

All converters include unit tests:

```bash
# Run all converter tests
cargo test -p vyna-hl7-fhir --lib

# Run specific converter tests
cargo test -p vyna-hl7-fhir patient
cargo test -p vyna-hl7-fhir observation
cargo test -p vyna-hl7-fhir encounter
```

Current test coverage: **16 tests, 100% passing** ✅
