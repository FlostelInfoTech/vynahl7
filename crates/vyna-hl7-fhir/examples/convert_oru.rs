//! Example: Converting HL7 v2 ORU^R01 (Observation Result) message to FHIR resources
//!
//! This example demonstrates how to convert a complete HL7 v2 laboratory result message
//! into FHIR Patient, Observation, and Practitioner resources.
//!
//! Run with: cargo run --example convert_oru

use vyna_hl7_fhir::converters::{
    DiagnosticReportConverter, ObservationConverter, PatientConverter, PractitionerConverter,
};
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample HL7 v2 ORU^R01 message (Observation Result - Laboratory)
    let hl7_message = "\
MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315120000||ORU^R01|MSG123456|P|2.5
PID|1||12345678^^^MRN||DOE^JOHN^MICHAEL^JR^DR||19800115|M|||123 Main St^^Springfield^IL^62701^USA^^H||(555)123-4567^H^PH|(555)987-6543^W^PH||S||987654321|||SSN|123-45-6789
PV1|1|O|OUTPATIENT||||1234567^SMITH^JANE^^^DR^MD^^^^^^NPI|9876543^JOHNSON^ROBERT^^^DR^DO^^^^^^NPI
OBR|1|LAB2024-001|LAB2024-001-01|24331-1^Lipid Panel^LN|||20240315080000|20240315083000
OBX|1|NM|2093-3^Total Cholesterol^LN||195|mg/dL|<200|N|||F|||20240315090000||1234567^SMITH^JANE^^^DR^MD
OBX|2|NM|2085-9^HDL Cholesterol^LN||55|mg/dL|>40|N|||F|||20240315090000||1234567^SMITH^JANE^^^DR^MD
OBX|3|NM|2089-1^LDL Cholesterol^LN||110|mg/dL|<100|H|||F|||20240315090000||1234567^SMITH^JANE^^^DR^MD
OBX|4|NM|2571-8^Triglycerides^LN||150|mg/dL|<150|N|||F|||20240315090000||1234567^SMITH^JANE^^^DR^MD
OBX|5|ST|NOTE^Clinical Note^LN||Patient is advised to continue current diet and exercise regimen.|||||||F
";

    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║        HL7 v2 to FHIR Conversion Example - Laboratory Results       ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\n");

    // Parse the HL7 message
    println!("📥 Parsing HL7 v2 message...");
    let message = parse_message(hl7_message)?;
    println!("✓ Message parsed successfully\n");

    // Convert to FHIR Patient
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧑 Converting Patient (PID segment)...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let patient = PatientConverter::convert(&message)?;
    println!("{}\n", serde_json::to_string_pretty(&patient)?);

    // Convert to FHIR Observations
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔬 Converting Observations (OBX segments)...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let observations = ObservationConverter::convert_all(&message)?;
    println!("Found {} observations:\n", observations.len());

    for (i, observation) in observations.iter().enumerate() {
        println!("--- Observation {} ---", i + 1);
        println!("{}\n", serde_json::to_string_pretty(observation)?);
    }

    // Convert to FHIR DiagnosticReports
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 Converting DiagnosticReports (OBR segments)...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    let reports = DiagnosticReportConverter::convert_all(&message)?;
    println!("Found {} diagnostic reports:\n", reports.len());

    for (i, report) in reports.iter().enumerate() {
        println!("--- DiagnosticReport {} ---", i + 1);
        println!("{}\n", serde_json::to_string_pretty(report)?);
    }

    // Convert to FHIR Practitioners
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("👨‍⚕️ Converting Practitioners (PV1 segment)...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Attending Doctor (PV1-7)
    if let Ok(attending) = PractitionerConverter::convert_attending_doctor(&message) {
        println!("Attending Doctor:");
        println!("{}\n", serde_json::to_string_pretty(&attending)?);
    }

    // Referring Doctor (PV1-8)
    if let Ok(referring) = PractitionerConverter::convert_referring_doctor(&message) {
        println!("Referring Doctor:");
        println!("{}\n", serde_json::to_string_pretty(&referring)?);
    }

    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                    Conversion Complete! ✓                            ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");

    Ok(())
}
