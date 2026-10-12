//! Example: Convert HL7 v2 ORM^O01 (Order) message to FHIR ServiceRequest resources
//!
//! This example demonstrates converting an HL7 v2 ORM^O01 message (general laboratory order)
//! to FHIR R4 ServiceRequest resources.
//!
//! Run with: `cargo run --example convert_orm -p vyna-hl7-fhir`

use vyna_hl7_parser::parse_message;
use vyna_hl7_fhir::converters::ServiceRequestConverter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample ORM^O01 message with multiple orders
    let hl7_message = "\
MSH|^~\\&|CPOE|HOSPITAL|LAB|HOSPITAL|20240315143000||ORM^O01^ORM_O01|MSG20240315002|P|2.5.1
PID|1||87654321^^^MRN||SMITH^JANE^ELIZABETH||19750622|F||2106-3^White^HL70005|456 ELM ST^^METROPOLIS^NY^54321^USA^H||555-5678^PRN^PH|||M^Married^HL70002
PV1|1|I|4N^401^01^MAIN||||DOC123^JOHNSON^ROBERT^L^^^MD^L|||SUR||||ADM|A0|||||||||||||||||||||||||20240315
ORC|NW|ORD2024031501|FIL2024031501|GRP789|IP||^^^^^R||20240315143000|||DOC123^JOHNSON^ROBERT^L^^^MD^L|||20240315143000|||||HOSPITAL^Main Hospital Laboratory
OBR|1|ORD2024031501|FIL2024031501|85025^Complete Blood Count^LN|S|20240315140000|||||||20240315140000|BLD^Blood^HL70070|DOC123^JOHNSON^ROBERT^L^^^MD^L||||||||20240315140000|||F|||||||||||20240315143000
ORC|NW|ORD2024031502|FIL2024031502|GRP789|IP||^^^^^R||20240315143000|||DOC123^JOHNSON^ROBERT^L^^^MD^L|||20240315143000|||||HOSPITAL^Main Hospital Laboratory
OBR|1|ORD2024031502|FIL2024031502|2345-7^Glucose Ser/Plas^LN|R|20240315140000|||||||20240315140000|SER^Serum^HL70070|DOC123^JOHNSON^ROBERT^L^^^MD^L||||||||20240315140000|||F|||||||||||20240315143000
ORC|NW|ORD2024031503|FIL2024031503|GRP789|IP||^^^^^A||20240315143000|||DOC123^JOHNSON^ROBERT^L^^^MD^L|||20240315143000|||||HOSPITAL^Main Hospital Laboratory
OBR|1|ORD2024031503|FIL2024031503|24326-1^Electrolytes Panel^LN|A|20240315140000|||||||20240315140000|SER^Serum^HL70070|DOC123^JOHNSON^ROBERT^L^^^MD^L||||||||20240315140000|||F|||||||||||20240315143000\r";

    // Parse the HL7 message
    println!("📥 Parsing HL7 ORM^O01 message...");
    let message = parse_message(hl7_message)?;
    println!("✓ Message parsed successfully\n");

    // Convert all ORC segments to FHIR ServiceRequest resources
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📋 Converting laboratory orders to FHIR ServiceRequest resources...");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let service_requests = ServiceRequestConverter::convert_all(&message)?;

    println!("Found {} service requests:\n", service_requests.len());

    // Display each service request
    for (i, sr) in service_requests.iter().enumerate() {
        println!("=== Service Request {} ===", i + 1);
        println!("Resource Type: {}", sr.resource_type);
        println!("Status: {}", sr.status);
        println!("Intent: {}", sr.intent);
        println!("Subject: {:?}", sr.subject.reference);

        if let Some(identifiers) = &sr.identifier {
            println!("Identifiers:");
            for id in identifiers {
                println!("  - System: {:?}", id.system);
                println!("    Value: {:?}", id.value);
            }
        }

        if let Some(code) = &sr.code {
            println!("Service Code: {:?}", code.coding.as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.code.as_ref()));
            println!("Service Name: {:?}", code.coding.as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.display.as_ref()));
        }

        println!("Priority: {:?}", sr.priority);
        println!("Authored On: {:?}", sr.authored_on);
        println!("Requester: {:?}", sr.requester.as_ref()
            .and_then(|r| r.reference.as_ref()));
        println!("Encounter: {:?}", sr.encounter.as_ref()
            .and_then(|e| e.reference.as_ref()));

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&sr)?;
        println!("\nFull FHIR JSON:\n{}\n", json);
    }

    // Example: Filter by priority
    let stat_orders: Vec<_> = service_requests.iter()
        .filter(|sr| sr.priority.as_deref() == Some("stat"))
        .collect();

    println!("=== STAT Priority Orders ===");
    println!("Found {} STAT orders", stat_orders.len());
    for sr in stat_orders {
        if let Some(code) = &sr.code
            && let Some(display) = code.coding.as_ref()
                .and_then(|c| c.first())
                .and_then(|c| c.display.as_ref()) {
                println!("  - {}", display);
            }
    }

    // Example: Group by requisition
    if let Some(first_sr) = service_requests.first()
        && let Some(requisition) = &first_sr.requisition {
            println!("\n=== Orders in Requisition {:?} ===", requisition.value);
            let same_requisition: Vec<_> = service_requests.iter()
                .filter(|sr| sr.requisition.as_ref().and_then(|r| r.value.as_ref())
                    == requisition.value.as_ref())
                .collect();
            println!("Count: {}", same_requisition.len());
        }

    println!("\n✓ Conversion complete!");

    Ok(())
}
