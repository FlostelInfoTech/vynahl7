//! Example: Convert HL7 v2 OML^O21 (Laboratory Order) message to FHIR Specimen resources
//!
//! This example demonstrates converting an HL7 v2 OML^O21 message (laboratory order for specimens)
//! to FHIR R4 Specimen resources.
//!
//! Run with: `cargo run --example convert_oml -p vyna-hl7-fhir`

use vyna_hl7_parser::parse_message;
use vyna_hl7_fhir::converters::SpecimenConverter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample OML^O21 message with multiple specimens
    let hl7_message = "\
MSH|^~\\&|LIS|HOSPITAL|CPOE|HOSPITAL|20240315150000||OML^O21^OML_O21|MSG20240315003|P|2.5.1
PID|1||11223344^^^MRN||WILSON^ROBERT^JAMES||19680910|M||2106-3^White^HL70005|789 OAK AVE^^SPRINGFIELD^IL^62701^USA^H||555-9012^PRN^PH|||M^Married^HL70002
PV1|1|O|||||DOC789^WILLIAMS^SARAH^M^^^MD^L|||LAB||||||||V9876543|||||||||||||||||||||||||20240315
ORC|NW|LAB20240315-001||||||20240315150000|||DOC789^WILLIAMS^SARAH^M^^^MD^L
OBR|1|LAB20240315-001||85025^Complete Blood Count^LN|||||||||20240315145000
SPM|1|SPEC-BLD-001||BLD^Blood^HL70487|||ANP^Venipuncture^HL70488|LA^Left Arm^HL70163||||5.0^mL^UCUM|||||20240315145000|20240315150000|||Y||||HEM^Hemolyzed^HL70493|||||||ACC-20240315-001
ORC|NW|LAB20240315-002||||||20240315150000|||DOC789^WILLIAMS^SARAH^M^^^MD^L
OBR|1|LAB20240315-002||2345-7^Glucose Ser/Plas^LN|||||||||20240315145500
SPM|1|SPEC-SER-001||SER^Serum^HL70487|||CAP^Capillary Specimen^HL70488|RF^Right Finger^HL70163||||3.0^mL^UCUM|||||20240315145500|20240315150000|||Y||||||||||ACC-20240315-002
ORC|NW|LAB20240315-003||||||20240315150000|||DOC789^WILLIAMS^SARAH^M^^^MD^L
OBR|1|LAB20240315-003||24326-1^Electrolytes Panel^LN|||||||||20240315145500
SPM|1|SPEC-SER-002||SER^Serum^HL70487|||ANP^Venipuncture^HL70488|RA^Right Arm^HL70163||||4.0^mL^UCUM|Special Handling Required|||GRN^Green Top Tube^HL70370||20240315145500|20240315150000|||Y||||||||||ACC-20240315-003\r";

    // Parse the HL7 message
    println!("📥 Parsing HL7 OML^O21 message...");
    let message = parse_message(hl7_message)?;
    println!("✓ Message parsed successfully\n");

    // Convert all SPM segments to FHIR Specimen resources
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 Converting specimen records to FHIR Specimen resources...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let specimens = SpecimenConverter::convert_all(&message)?;

    println!("Found {} specimens:\n", specimens.len());

    // Display each specimen
    for (i, spec) in specimens.iter().enumerate() {
        println!("=== Specimen {} ===", i + 1);
        println!("Resource Type: {}", spec.resource_type);
        println!("Subject: {:?}", spec.subject.reference);

        if let Some(identifiers) = &spec.identifier {
            println!("Specimen ID:");
            for id in identifiers {
                println!("  - System: {:?}", id.system);
                println!("    Value: {:?}", id.value);
            }
        }

        if let Some(accession) = &spec.accession_identifier {
            println!("Accession ID: {:?}", accession.value);
        }

        println!("Specimen Type: {:?}", spec.type_.coding.as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.code.as_ref()));
        println!("Specimen Type Name: {:?}", spec.type_.coding.as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.display.as_ref()));

        println!("Status: {:?}", spec.status);
        println!("Received Time: {:?}", spec.received_time);

        if let Some(collection) = &spec.collection {
            println!("Collection Details:");
            println!("  - Collected: {:?}", collection.collected_date_time);
            if let Some(method) = &collection.method {
                println!("  - Method: {:?}", method.coding.as_ref()
                    .and_then(|c| c.first())
                    .and_then(|c| c.display.as_ref()));
            }
            if let Some(site) = &collection.body_site {
                println!("  - Body Site: {:?}", site.coding.as_ref()
                    .and_then(|c| c.first())
                    .and_then(|c| c.display.as_ref()));
            }
            if let Some(quantity) = &collection.quantity {
                println!("  - Quantity: {:?} {:?}", quantity.value, quantity.unit);
            }
        }

        if let Some(condition) = &spec.condition {
            println!("Condition:");
            for cond in condition {
                println!("  - {:?}", cond.coding.as_ref()
                    .and_then(|c| c.first())
                    .and_then(|c| c.display.as_ref()));
            }
        }

        if let Some(container) = &spec.container {
            println!("Container:");
            for cont in container {
                if let Some(type_) = &cont.type_ {
                    println!("  - Type: {:?}", type_.coding.as_ref()
                        .and_then(|c| c.first())
                        .and_then(|c| c.code.as_ref()));
                }
            }
        }

        if let Some(note) = &spec.note {
            println!("Notes:");
            for n in note {
                println!("  - {}", n.text);
            }
        }

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&spec)?;
        println!("\nFull FHIR JSON:\n{}\n", json);
    }

    // Example: Group specimens by type
    println!("=== Specimens by Type ===");
    let blood_specimens: Vec<_> = specimens.iter()
        .filter(|s| s.type_.coding.as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.code.as_ref())
            .map(|code| code == "BLD")
            .unwrap_or(false))
        .collect();

    println!("Blood specimens: {}", blood_specimens.len());

    let serum_specimens: Vec<_> = specimens.iter()
        .filter(|s| s.type_.coding.as_ref()
            .and_then(|c| c.first())
            .and_then(|c| c.code.as_ref())
            .map(|code| code == "SER")
            .unwrap_or(false))
        .collect();

    println!("Serum specimens: {}", serum_specimens.len());

    // Example: Check specimen status
    println!("\n=== Specimen Availability ===");
    let available = specimens.iter()
        .filter(|s| s.status.as_deref() == Some("available"))
        .count();
    println!("Available specimens: {} / {}", available, specimens.len());

    // Example: Specimens with special conditions
    let conditioned_specimens: Vec<_> = specimens.iter()
        .filter(|s| s.condition.is_some())
        .collect();

    if !conditioned_specimens.is_empty() {
        println!("\n=== Specimens with Conditions ===");
        for spec in conditioned_specimens {
            if let (Some(id), Some(condition)) = (&spec.identifier, &spec.condition) {
                println!("Specimen {:?}:", id.first().and_then(|i| i.value.as_ref()));
                for cond in condition {
                    if let Some(display) = cond.coding.as_ref()
                        .and_then(|c| c.first())
                        .and_then(|c| c.display.as_ref()) {
                        println!("  - {}", display);
                    }
                }
            }
        }
    }

    println!("\n✓ Conversion complete!");

    Ok(())
}
