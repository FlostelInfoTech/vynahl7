//! Example: Convert HL7 v2 VXU^V04 (Immunization) message to FHIR Immunization resources
//!
//! This example demonstrates converting an HL7 v2 VXU^V04 message (unsolicited vaccination record update)
//! to FHIR R4 Immunization resources.
//!
//! Run with: `cargo run --example convert_vxu -p vyna-hl7-fhir`

use vyna_hl7_fhir::converters::ImmunizationConverter;
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample VXU^V04 message with multiple immunizations
    let hl7_message = "\
MSH|^~\\&|VAERS|CDC|CDCINFO|CDC|20240315120000||VXU^V04^VXU_V04|MSG20240315001|P|2.5.1
PID|1||12345678^^^MRN||DOE^JOHN^MICHAEL||19800115|M||2106-3^White^HL70005|123 MAIN ST^^ANYTOWN^CA^12345^USA^H||555-1234^PRN^PH|||M^Married^HL70002||123-45-6789|||N^Not Hispanic or Latino^HL70189
PV1|1|O|||||G12345^SMITH^MARY^A^^^MD^L^^^NPI||||||||||||V1234567|||||||||||||||||||||||||20240315
ORC|RE||IZ-783274^NDA|||||||57422^RADON^NICHOLAS^^^^^^NPI^L
RXA|0|1|20240315100000|20240315100000|08^HepB adult^CVX|1.0|mL^milliliters^UCUM||01^Historical^NDA|57422^RADON^NICHOLAS^^^^^^NPI^L|^^^Main Clinic||||LOT123456|20251231|MFR123^Merck^MVX|||CP|20240315120000|RNC^Clinical Nurse^HL70443
RXR|IM^Intramuscular^HL70162|LD^Left Deltoid^HL70163
ORC|RE||IZ-783275^NDA|||||||57422^RADON^NICHOLAS^^^^^^NPI^L
RXA|0|1|20240315101500|20240315101500|20^DTaP^CVX|0.5|mL^milliliters^UCUM||01^Historical^NDA|57422^RADON^NICHOLAS^^^^^^NPI^L|^^^Main Clinic||||LOT789012|20260630|MFR456^GlaxoSmithKline^MVX|||CP|20240315120000|RNC^Clinical Nurse^HL70443
RXR|IM^Intramuscular^HL70162|RD^Right Deltoid^HL70163\r";

    // Parse the HL7 message
    println!("📥 Parsing HL7 VXU^V04 message...");
    let message = parse_message(hl7_message)?;
    println!("✓ Message parsed successfully\n");

    // Convert all RXA segments to FHIR Immunization resources
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("💉 Converting immunization records to FHIR Immunization resources...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let immunizations = ImmunizationConverter::convert_all(&message)?;

    println!("Found {} immunizations:\n", immunizations.len());

    // Display each immunization as JSON
    for (i, imm) in immunizations.iter().enumerate() {
        println!("=== Immunization {} ===", i + 1);
        println!("Resource Type: {}", imm.resource_type);
        println!("Status: {}", imm.status);
        println!("Patient: {:?}", imm.patient.reference);
        println!(
            "Vaccine Code: {:?}",
            imm.vaccine_code
                .coding
                .as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.code.as_ref())
        );
        println!(
            "Vaccine Name: {:?}",
            imm.vaccine_code
                .coding
                .as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.display.as_ref())
        );
        println!("Occurrence: {}", imm.occurrence_date_time);
        println!("Lot Number: {:?}", imm.lot_number);
        println!("Expiration Date: {:?}", imm.expiration_date);
        println!(
            "Manufacturer: {:?}",
            imm.manufacturer.as_ref().and_then(|m| m.reference.as_ref())
        );
        println!(
            "Dose Quantity: {:?}",
            imm.dose_quantity
                .as_ref()
                .map(|q| format!("{:?} {:?}", q.value, q.unit))
        );
        println!(
            "Site: {:?}",
            imm.site
                .as_ref()
                .and_then(|s| s.coding.as_ref())
                .and_then(|c| c.first())
                .and_then(|c| c.display.as_ref())
        );

        // Serialize to JSON for full details
        let json = serde_json::to_string_pretty(&imm)?;
        println!("\nFull FHIR JSON:\n{}\n", json);
    }

    // Example: Access specific immunization details
    if let Some(first_imm) = immunizations.first() {
        println!("=== Detailed View of First Immunization ===");
        println!("Primary Source: {:?}", first_imm.primary_source);
        println!("Recorded: {:?}", first_imm.recorded);

        if let Some(performer) = &first_imm.performer {
            println!("Performer(s):");
            for p in performer {
                println!("  - Actor: {:?}", p.actor.reference);
                println!(
                    "    Function: {:?}",
                    p.function
                        .as_ref()
                        .and_then(|f| f.coding.as_ref())
                        .and_then(|c| c.first())
                        .and_then(|c| c.display.as_ref())
                );
            }
        }

        if let Some(encounter) = &first_imm.encounter {
            println!("Encounter Reference: {:?}", encounter.reference);
        }
    }

    println!("\n✓ Conversion complete!");

    Ok(())
}
