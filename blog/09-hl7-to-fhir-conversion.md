# Bridging to Modern Healthcare: HL7 v2 to FHIR Conversion

*Part 9 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./08-network-transport.md), we explored network transport protocols. Now let's look at one of the most requested features in healthcare integration: converting HL7 v2 messages to FHIR R4 resources.

## Why Convert HL7 v2 to FHIR?

While HL7 v2 dominates internal hospital systems, FHIR (Fast Healthcare Interoperability Resources) is becoming the standard for:

- Patient-facing APIs and portals
- Health information exchanges (HIEs)
- Government mandates (21st Century Cures Act, ONC rules)
- Mobile health applications
- Analytics and research platforms

VynaHL7's `vyna-hl7-fhir` crate provides converters that transform HL7 v2 segments into FHIR R4 resources.

## Available Converters

VynaHL7 includes 12 converters covering the most common clinical data:

| Converter | Source | Target | Description |
|-----------|--------|--------|-------------|
| **PatientConverter** | PID | Patient | Demographics, identifiers, contact info |
| **ObservationConverter** | OBX | Observation | Lab results, vital signs |
| **EncounterConverter** | PV1 | Encounter | Patient visits |
| **DiagnosticReportConverter** | OBR | DiagnosticReport | Orders and panels |
| **PractitionerConverter** | PV1/ORC | Practitioner | Healthcare providers |
| **AllergyIntoleranceConverter** | AL1 | AllergyIntolerance | Allergies |
| **MedicationConverter** | RXA | MedicationAdministration | Medications given |
| **ConditionConverter** | PRB/DG1 | Condition | Diagnoses |
| **ProcedureConverter** | PR1 | Procedure | Clinical procedures |
| **ImmunizationConverter** | RXA | Immunization | Vaccinations administered |
| **ServiceRequestConverter** | ORC | ServiceRequest | Orders and referrals |
| **SpecimenConverter** | SPM | Specimen | Specimen collection details |

## Converting Patient Demographics

The most common conversion is patient demographics from PID to FHIR Patient:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_fhir::converters::PatientConverter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7 = r"MSH|^~\&|ADT|Hospital|EMR|Hospital|20240315||ADT^A01|MSG001|P|2.5
PID|1||MRN123456^^^MRN^MR~SSN987654321^^^SSN^SS||SMITH^JANE^ELIZABETH^III^MS||19850220|F||2106-3^White^HL70005|456 Oak Ave^Apt 3B^Chicago^IL^60601^USA^^H~789 Work Blvd^^Chicago^IL^60602^USA^^W||(312)555-1234^H^PH~(312)555-5678^W^PH||S|CAT|ACC987654321|123-45-6789|||H^Hispanic or Latino^HL70189
NK1|1|SMITH^JOHN^M|SPO^Spouse^HL70063|(312)555-9876^H^PH";

    let message = parse_message(hl7)?;

    // Convert to FHIR Patient
    let patient = PatientConverter::convert(&message)?;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&patient)?;
    println!("{}", json);

    Ok(())
}
```

Output:
```json
{
  "resourceType": "Patient",
  "id": "MRN123456",
  "identifier": [
    {
      "type": {
        "coding": [
          {
            "system": "http://terminology.hl7.org/CodeSystem/v2-0203",
            "code": "MR"
          }
        ],
        "text": "Medical Record Number"
      },
      "value": "MRN123456"
    },
    {
      "type": {
        "coding": [
          {
            "system": "http://terminology.hl7.org/CodeSystem/v2-0203",
            "code": "SS"
          }
        ],
        "text": "Social Security Number"
      },
      "value": "SSN987654321"
    }
  ],
  "name": [
    {
      "use": "official",
      "family": "SMITH",
      "given": ["JANE", "ELIZABETH"],
      "prefix": ["MS"],
      "suffix": ["III"]
    }
  ],
  "gender": "female",
  "birthDate": "1985-02-20",
  "address": [
    {
      "use": "home",
      "line": ["456 Oak Ave", "Apt 3B"],
      "city": "Chicago",
      "state": "IL",
      "postalCode": "60601",
      "country": "USA"
    },
    {
      "use": "work",
      "line": ["789 Work Blvd"],
      "city": "Chicago",
      "state": "IL",
      "postalCode": "60602",
      "country": "USA"
    }
  ],
  "telecom": [
    {
      "system": "phone",
      "value": "(312)555-1234",
      "use": "home"
    },
    {
      "system": "phone",
      "value": "(312)555-5678",
      "use": "work"
    }
  ],
  "maritalStatus": {
    "coding": [
      {
        "system": "http://terminology.hl7.org/CodeSystem/v3-MaritalStatus",
        "code": "S",
        "display": "Never Married"
      }
    ]
  }
}
```

## Converting Lab Results

Lab results are converted from OBX segments to FHIR Observations:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_fhir::converters::{PatientConverter, ObservationConverter};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7 = r"MSH|^~\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|MSG001|P|2.5
PID|1||12345678^^^MRN||DOE^JOHN||19800115|M
OBR|1|LAB2024-001|LAB2024-001-01|24331-1^Lipid Panel^LN|||20240315080000
OBX|1|NM|2093-3^Total Cholesterol^LN||195|mg/dL|<200|N|||F|||20240315090000
OBX|2|NM|2085-9^HDL Cholesterol^LN||55|mg/dL|>40|N|||F|||20240315090000
OBX|3|NM|2089-1^LDL Cholesterol^LN||110|mg/dL|<100|H|||F|||20240315090000
OBX|4|NM|2571-8^Triglycerides^LN||150|mg/dL|<150|N|||F|||20240315090000";

    let message = parse_message(hl7)?;

    // Convert patient
    let patient = PatientConverter::convert(&message)?;
    println!("Patient: {:?}", patient.id);

    // Convert ALL observations
    let observations = ObservationConverter::convert_all(&message)?;
    println!("\nFound {} observations:", observations.len());

    for obs in &observations {
        println!("\n--- {} ---", obs.code.as_ref()
            .and_then(|c| c.text.as_ref())
            .unwrap_or(&"Unknown".to_string()));
        println!("{}", serde_json::to_string_pretty(&obs)?);
    }

    Ok(())
}
```

Each Observation includes:

```json
{
  "resourceType": "Observation",
  "status": "final",
  "code": {
    "coding": [
      {
        "system": "http://loinc.org",
        "code": "2093-3",
        "display": "Total Cholesterol"
      }
    ],
    "text": "Total Cholesterol"
  },
  "valueQuantity": {
    "value": 195,
    "unit": "mg/dL",
    "system": "http://unitsofmeasure.org"
  },
  "referenceRange": [
    {
      "text": "<200"
    }
  ],
  "interpretation": [
    {
      "coding": [
        {
          "system": "http://terminology.hl7.org/CodeSystem/v3-ObservationInterpretation",
          "code": "N",
          "display": "Normal"
        }
      ]
    }
  ],
  "effectiveDateTime": "2024-03-15T09:00:00Z"
}
```

## Converting Practitioners

Healthcare provider information from PV1 becomes FHIR Practitioner resources:

```rust
use vyna_hl7_fhir::converters::PractitionerConverter;

// Convert attending doctor (PV1-7)
let attending = PractitionerConverter::convert_attending_doctor(&message)?;

// Convert referring doctor (PV1-8)
let referring = PractitionerConverter::convert_referring_doctor(&message)?;
```

Output:
```json
{
  "resourceType": "Practitioner",
  "id": "1234567",
  "identifier": [
    {
      "system": "http://hl7.org/fhir/sid/us-npi",
      "value": "1234567"
    }
  ],
  "name": [
    {
      "use": "official",
      "family": "SMITH",
      "given": ["JANE"],
      "prefix": ["DR"],
      "suffix": ["MD"]
    }
  ]
}
```

## Converting Encounters

Patient visit information from PV1 becomes FHIR Encounter:

```rust
use vyna_hl7_fhir::converters::EncounterConverter;

let encounter = EncounterConverter::convert(&message)?;
```

The converter maps:
- PV1-2 (Patient Class) → Encounter.class (inpatient, outpatient, emergency)
- PV1-3 (Assigned Location) → Encounter.location
- PV1-7 (Attending Doctor) → Encounter.participant
- PV1-44/45 (Admit/Discharge time) → Encounter.period

## Converting Allergies

AL1 segments become AllergyIntolerance resources:

```rust
use vyna_hl7_fhir::converters::AllergyIntoleranceConverter;

let hl7 = r"MSH|^~\&|EMR|Hospital|EMR|Hospital|20240315||ADT^A01|MSG001|P|2.5
PID|1||12345||DOE^JOHN||19800115|M
AL1|1|DA|PCN^Penicillin^|SV|Hives and rash||20200315
AL1|2|FA|EGG^Eggs^|MI|Nausea||20180101";

let message = parse_message(hl7)?;
let allergies = AllergyIntoleranceConverter::convert_all(&message)?;

for allergy in allergies {
    println!("{}", serde_json::to_string_pretty(&allergy)?);
}
```

## Converting Medications

RXA segments become MedicationAdministration resources:

```rust
use vyna_hl7_fhir::converters::MedicationAdministrationConverter;

let med_admin = MedicationAdministrationConverter::convert(&message)?;
```

## Complete Conversion Example

Here's a complete example converting an ORU message to a FHIR Bundle:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_fhir::converters::*;
use serde_json::json;

fn convert_oru_to_bundle(hl7: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let message = parse_message(hl7)?;

    let mut entries = Vec::new();

    // Convert Patient
    let patient = PatientConverter::convert(&message)?;
    let patient_ref = format!("Patient/{}", patient.id.as_ref().unwrap_or(&"unknown".to_string()));
    entries.push(json!({
        "resource": patient,
        "request": {
            "method": "PUT",
            "url": patient_ref
        }
    }));

    // Convert Observations
    let observations = ObservationConverter::convert_all(&message)?;
    for (i, obs) in observations.into_iter().enumerate() {
        entries.push(json!({
            "resource": obs,
            "request": {
                "method": "POST",
                "url": "Observation"
            }
        }));
    }

    // Convert Practitioners
    if let Ok(attending) = PractitionerConverter::convert_attending_doctor(&message) {
        entries.push(json!({
            "resource": attending,
            "request": {
                "method": "PUT",
                "url": format!("Practitioner/{}", attending.id.unwrap_or_default())
            }
        }));
    }

    // Create Bundle
    let bundle = json!({
        "resourceType": "Bundle",
        "type": "transaction",
        "entry": entries
    });

    Ok(bundle)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7 = r"MSH|^~\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|MSG001|P|2.5
PID|1||12345678^^^MRN||DOE^JOHN||19800115|M
OBR|1|LAB2024-001|LAB2024-001-01|24331-1^Lipid Panel^LN|||20240315080000
OBX|1|NM|2093-3^Total Cholesterol^LN||195|mg/dL|<200|N|||F
OBX|2|NM|2085-9^HDL Cholesterol^LN||55|mg/dL|>40|N|||F
PV1|1|O|OUTPATIENT||||1234567^SMITH^JANE^^^DR^MD";

    let bundle = convert_oru_to_bundle(hl7)?;
    println!("{}", serde_json::to_string_pretty(&bundle)?);

    Ok(())
}
```

## Mapping Reference

### PID to Patient Mapping

| PID Field | FHIR Patient Element |
|-----------|---------------------|
| PID-3 | identifier |
| PID-5 | name |
| PID-7 | birthDate |
| PID-8 | gender |
| PID-10 | extension (race) |
| PID-11 | address |
| PID-13 | telecom (home) |
| PID-14 | telecom (work) |
| PID-16 | maritalStatus |
| PID-22 | extension (ethnicity) |

### OBX to Observation Mapping

| OBX Field | FHIR Observation Element |
|-----------|-------------------------|
| OBX-2 | valueType determines value[x] |
| OBX-3 | code |
| OBX-5 | value[x] (valueQuantity, valueString, etc.) |
| OBX-6 | valueQuantity.unit |
| OBX-7 | referenceRange |
| OBX-8 | interpretation |
| OBX-11 | status |
| OBX-14 | effectiveDateTime |
| OBX-16 | performer |

### PV1 to Encounter Mapping

| PV1 Field | FHIR Encounter Element |
|-----------|------------------------|
| PV1-2 | class |
| PV1-3 | location |
| PV1-7 | participant (attending) |
| PV1-8 | participant (referring) |
| PV1-17 | participant (admitting) |
| PV1-19 | identifier (visit number) |
| PV1-44 | period.start |
| PV1-45 | period.end |

## Best Practices

### 1. Handle Missing Data Gracefully

Not all HL7 messages have complete data:

```rust
let patient = PatientConverter::convert(&message)?;

// Check for missing data
if patient.name.is_none() {
    log::warn!("Patient {} has no name", patient.id.as_ref().unwrap_or(&"?".to_string()));
}
```

### 2. Validate Before Converting

Ensure messages are valid before converting:

```rust
use vyna_hl7_validator::Validator;

let validator = Validator::new(Version::V2_5);
let result = validator.validate(&message);

if result.is_valid() {
    let patient = PatientConverter::convert(&message)?;
} else {
    return Err("Invalid message".into());
}
```

### 3. Add Patient References

Link observations to patients:

```rust
let patient = PatientConverter::convert(&message)?;
let patient_ref = format!("Patient/{}", patient.id.as_ref().unwrap());

for mut obs in ObservationConverter::convert_all(&message)? {
    obs.subject = Some(Reference {
        reference: Some(patient_ref.clone()),
        ..Default::default()
    });
}
```

### 4. Handle Code Systems

VynaHL7 maps to standard code systems:
- LOINC for lab codes
- SNOMED CT for clinical concepts
- HL7 FHIR code systems for administrative codes

```rust
// The converter automatically maps HL7 table codes to FHIR ValueSets
// PID-8 "M" → Patient.gender "male"
// OBX-8 "H" → Observation.interpretation "High"
```

## Integration with FHIR Servers

Send converted resources to a FHIR server:

```rust
use reqwest::Client;

async fn send_to_fhir_server(bundle: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client
        .post("https://fhir-server.example.org/fhir")
        .header("Content-Type", "application/fhir+json")
        .json(bundle)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Resources created successfully");
    } else {
        println!("Error: {}", response.text().await?);
    }

    Ok(())
}
```

## Summary

VynaHL7's FHIR converters bridge the gap between legacy HL7 v2 systems and modern FHIR infrastructure:

- **12 converters** covering patients, observations, encounters, and more
- **Standards-compliant** mapping to FHIR R4
- **Automatic code system mapping** for terminology
- **Flexible JSON output** for integration with any FHIR server

This enables organizations to maintain their existing HL7 v2 infrastructure while participating in FHIR-based interoperability initiatives.

---

*Next in series: [Building Production-Ready HL7 Integrations](./10-production-ready-integrations.md)*

*Previous: [Network Transport](./08-network-transport.md)*
