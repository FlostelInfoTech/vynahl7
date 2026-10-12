//! Example: Converting HL7 v2 ADT^A01 (Patient Admission) message to FHIR Patient resource
//!
//! This example demonstrates how to convert an HL7 v2 ADT (Admit/Discharge/Transfer)
//! message into a FHIR Patient resource with complete demographic information.
//!
//! Run with: cargo run --example convert_adt

use vyna_hl7_fhir::converters::PatientConverter;
use vyna_hl7_parser::parse_message;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample HL7 v2 ADT^A01 message (Patient Admission)
    let hl7_message = "\
MSH|^~\\&|ADT|Hospital|EMR|Hospital|20240315140000||ADT^A01|MSG789012|P|2.5
EVN|A01|20240315140000
PID|1||MRN987654^^^MRN^MR~SSN123456789^^^SSN^SS||SMITH^JANE^ELIZABETH^III^MS||19850220|F||2106-3^White^HL70005|456 Oak Ave^Apt 3B^Chicago^IL^60601^USA^^H~789 Work Blvd^^Chicago^IL^60602^USA^^W||(312)555-1234^H^PH~(312)555-5678^W^PH||S|CAT|ACC987654321|123-45-6789|||H^Hispanic or Latino^HL70189||||Y
NK1|1|SMITH^JOHN^M|SPO^Spouse^HL70063|(312)555-9876^H^PH
PV1|1|I|4N^401^01^Hospital^^^N||||5678901^JONES^MICHAEL^^^DR^MD^^^^^^NPI
";

    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║         HL7 v2 to FHIR Conversion Example - Patient Admission       ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝\n");

    // Parse the HL7 message
    println!("📥 Parsing HL7 v2 ADT^A01 message...");
    let message = parse_message(hl7_message)?;
    println!("✓ Message parsed successfully\n");

    // Convert to FHIR Patient
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧑 Converting Patient with full demographics...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let patient = PatientConverter::convert(&message)?;

    // Display the FHIR Patient resource
    let patient_json = serde_json::to_string_pretty(&patient)?;
    println!("{}\n", patient_json);

    // Display key information
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Key Patient Information:");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if let Some(ref names) = patient.name
        && let Some(name) = names.first() {
            print!("Name: ");
            if let Some(ref prefix) = name.prefix {
                print!("{} ", prefix.join(" "));
            }
            if let Some(ref given) = name.given {
                print!("{} ", given.join(" "));
            }
            if let Some(ref family) = name.family {
                print!("{}", family);
            }
            if let Some(ref suffix) = name.suffix {
                print!(" {}", suffix.join(" "));
            }
            println!();
        }

    if let Some(ref gender) = patient.gender {
        println!("Gender: {}", gender);
    }

    if let Some(ref birth_date) = patient.birth_date {
        println!("Birth Date: {}", birth_date);
    }

    if let Some(ref identifiers) = patient.identifier {
        println!("\nIdentifiers:");
        for id in identifiers {
            if let Some(ref value) = id.value {
                print!("  - {}", value);
                if let Some(ref type_) = id.type_
                    && let Some(ref text) = type_.text {
                        print!(" ({})", text);
                    }
                println!();
            }
        }
    }

    if let Some(ref addresses) = patient.address {
        println!("\nAddresses:");
        for addr in addresses {
            print!("  - ");
            if let Some(ref lines) = addr.line {
                print!("{}, ", lines.join(", "));
            }
            if let Some(ref city) = addr.city {
                print!("{}, ", city);
            }
            if let Some(ref state) = addr.state {
                print!("{} ", state);
            }
            if let Some(ref postal) = addr.postal_code {
                print!("{}", postal);
            }
            if let Some(ref use_) = addr.use_ {
                print!(" ({})", use_);
            }
            println!();
        }
    }

    if let Some(ref telecom) = patient.telecom {
        println!("\nContact:");
        for contact in telecom {
            if let Some(ref value) = contact.value {
                print!("  - {}", value);
                if let Some(ref use_) = contact.use_ {
                    print!(" ({})", use_);
                }
                if let Some(ref system) = contact.system {
                    print!(" [{}]", system);
                }
                println!();
            }
        }
    }

    println!("\n╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                    Conversion Complete! ✓                            ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");

    Ok(())
}
