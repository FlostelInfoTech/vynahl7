//! Example demonstrating component field support (tuple types) in custom Z-segments
//!
//! This example shows how to use tuple types for fields with components:
//! - (String, String) for 2-component fields
//! - (String, String, String) for 3-component fields
//! - (String, String, String, String) for 4-component fields
//! - (String, String, String, String, String) for 5-component fields
//!
//! Components are separated by ^ (caret) in HL7 encoding.
//! Common use cases: patient names, addresses, identifiers
//!
//! Run with: cargo run --example component_fields

use vyna_hl7_core::Delimiters;
use vyna_hl7_custom::{CustomSegment, MessageExt, z_segment};
use vyna_hl7_parser::parse_message;

// Define a Z-segment with patient name as components
z_segment! {
    ZPN,
    id = "ZPN",
    fields = {
        1 => patient_id: String,
        2 => patient_name: (String, String, String),  // Last^First^Middle
        3 => emergency_contact: (String, String),     // Last^First
    }
}

// Define a Z-segment with address components
z_segment! {
    ZAD,
    id = "ZAD",
    fields = {
        1 => patient_id: String,
        2 => home_address: (String, String, String, String, String), // Street^City^State^Zip^Country
        3 => work_phone: (String, String),            // Number^Extension
    }
}

// Define a Z-segment with extended identifier
z_segment! {
    ZID,
    id = "ZID",
    fields = {
        1 => patient_identifier: (String, String, String, String), // ID^Type^Authority^Facility
        2 => referring_physician: (String, String, String),        // Last^First^Credentials
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Component Fields (Tuple Types) Example ===\n");

    // Example 1: Patient name with 3 components
    println!("1. Creating ZPN segment with patient name components:");
    println!("   ------------------------------------------");

    let zpn = ZPN::builder()
        .patient_id("PAT-12345")
        .patient_name((
            "Smith".to_string(),
            "John".to_string(),
            "Alexander".to_string(),
        ))
        .emergency_contact(("Doe".to_string(), "Jane".to_string()))
        .build()?;

    println!("   Patient ID: {}", zpn.patient_id);
    println!(
        "   Patient Name: {} {} {}",
        zpn.patient_name.0, zpn.patient_name.1, zpn.patient_name.2
    );
    println!(
        "   Emergency Contact: {} {}",
        zpn.emergency_contact.0, zpn.emergency_contact.1
    );

    // Example 2: HL7 encoding with ^ separator
    println!("\n2. HL7 encoding of component fields:");
    println!("   ------------------------------------------");

    let delimiters = Delimiters::default();
    let segment = zpn.to_segment();
    let encoded = segment.encode(&delimiters);
    println!("   {}", encoded);
    println!("\n   Note: The ^ (caret) character separates components");
    println!("   Example: Smith^John^Alexander");

    // Example 3: Address with 5 components
    println!("\n3. Creating ZAD segment with address components:");
    println!("   ------------------------------------------");

    let zad = ZAD::builder()
        .patient_id("PAT-67890")
        .home_address((
            "123 Main Street".to_string(),
            "Springfield".to_string(),
            "IL".to_string(),
            "62701".to_string(),
            "USA".to_string(),
        ))
        .work_phone(("555-1234".to_string(), "5678".to_string()))
        .build()?;

    println!("   Patient ID: {}", zad.patient_id);
    println!("   Home Address:");
    println!("     Street: {}", zad.home_address.0);
    println!("     City: {}", zad.home_address.1);
    println!("     State: {}", zad.home_address.2);
    println!("     Zip: {}", zad.home_address.3);
    println!("     Country: {}", zad.home_address.4);
    println!(
        "   Work Phone: {} ext. {}",
        zad.work_phone.0, zad.work_phone.1
    );

    let zad_encoded = zad.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", zad_encoded);

    // Example 4: Parsing HL7 messages with components
    println!("\n4. Parsing HL7 messages with component fields:");
    println!("   ------------------------------------------");

    let hl7_message = format!(
        "MSH|^~\\&|SendApp|SendFac|RecvApp|RecvFac|20250119120000||ADT^A01|MSG001|P|2.5\r\
         PID|1||PAT-99999||Williams^Sarah^Marie||19850615|F\r\
         {}\r",
        "ZID|MRN123^MRN^Hospital^MainCampus|Johnson^Robert^MD"
    );

    let message = parse_message(&hl7_message)?;

    if let Some(zid) = message.get_custom_segment::<ZID>()? {
        println!("   Successfully parsed ZID segment:");
        println!("\n   Patient Identifier:");
        println!("     ID: {}", zid.patient_identifier.0);
        println!("     Type: {}", zid.patient_identifier.1);
        println!("     Authority: {}", zid.patient_identifier.2);
        println!("     Facility: {}", zid.patient_identifier.3);
        println!("\n   Referring Physician:");
        println!(
            "     Name: {} {}, {}",
            zid.referring_physician.0, zid.referring_physician.1, zid.referring_physician.2
        );
    }

    // Example 5: Destructuring components
    println!("\n5. Destructuring component fields:");
    println!("   ------------------------------------------");

    let patient_name = zpn.patient_name;
    let (last, first, middle) = patient_name;
    println!("   Destructured patient name:");
    println!("   - Last: {}", last);
    println!("   - First: {}", first);
    println!("   - Middle: {}", middle);

    // Example 6: Modifying component fields
    println!("\n6. Modifying component fields:");
    println!("   ------------------------------------------");

    let mut msg = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         PID|1||PAT-00001\r\
         ZPN|PAT-00001|Brown^Alice^Marie|White^Bob\r",
    )?;

    println!("   Original patient name:");
    if let Some(original) = msg.get_custom_segment::<ZPN>()? {
        println!(
            "   {} {} {}",
            original.patient_name.0, original.patient_name.1, original.patient_name.2
        );
    }

    // Update the patient name
    let mut updated_zpn = msg.get_custom_segment::<ZPN>()?.unwrap();
    updated_zpn.patient_name = (
        "Brown-Smith".to_string(), // Married name
        "Alice".to_string(),
        "Marie".to_string(),
    );

    msg.set_custom_segment(updated_zpn)?;

    println!("\n   After name change:");
    if let Some(updated) = msg.get_custom_segment::<ZPN>()? {
        println!(
            "   {} {} {}",
            updated.patient_name.0, updated.patient_name.1, updated.patient_name.2
        );
    }

    // Example 7: Real-world use case - healthcare provider directory
    println!("\n7. Real-world use case: Healthcare provider directory:");
    println!("   ------------------------------------------");

    z_segment! {
        ZPR,
        id = "ZPR",
        fields = {
            1 => provider_id: String,
            2 => provider_name: (String, String, String, String, String), // Last^First^Middle^Suffix^Prefix
            3 => specialty: (String, String),                              // Code^Description
            4 => license: (String, String, String),                        // Number^State^Expiration
        }
    }

    let provider = ZPR::builder()
        .provider_id("PROV-001")
        .provider_name((
            "Williams".to_string(),
            "Sarah".to_string(),
            "Jane".to_string(),
            "MD".to_string(),
            "Dr".to_string(),
        ))
        .specialty((
            "207R00000X".to_string(), // NPI taxonomy code
            "Internal Medicine".to_string(),
        ))
        .license((
            "IL123456".to_string(),
            "IL".to_string(),
            "2026-12-31".to_string(),
        ))
        .build()?;

    println!("   Provider ID: {}", provider.provider_id);
    println!(
        "\n   Provider Name: {} {} {} {}, {}",
        provider.provider_name.4, // Prefix (Dr)
        provider.provider_name.1, // First
        provider.provider_name.2, // Middle
        provider.provider_name.0, // Last
        provider.provider_name.3
    ); // Suffix (MD)
    println!("\n   Specialty:");
    println!("     Code: {}", provider.specialty.0);
    println!("     Description: {}", provider.specialty.1);
    println!("\n   License:");
    println!("     Number: {}", provider.license.0);
    println!("     State: {}", provider.license.1);
    println!("     Expiration: {}", provider.license.2);

    let provider_encoded = provider.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", provider_encoded);

    // Example 8: Comparing simple fields vs component fields
    println!("\n8. Simple fields vs. component fields:");
    println!("   ------------------------------------------");

    z_segment! {
        ZCM,
        id = "ZCM",
        fields = {
            1 => simple_name: String,                           // Single value
            2 => component_name: (String, String),              // Last^First
            3 => full_name: (String, String, String),           // Last^First^Middle
        }
    }

    let comparison = ZCM::builder()
        .simple_name("John Smith") // Less structured
        .component_name(("Smith".to_string(), "John".to_string())) // Structured, easier to query
        .full_name((
            "Smith".to_string(),
            "John".to_string(),
            "Alexander".to_string(),
        )) // Most structured
        .build()?;

    println!("   Simple field (String):");
    println!(
        "     \"{}\" - Hard to separate first/last programmatically",
        comparison.simple_name
    );
    println!("\n   Component field (2 components):");
    println!(
        "     Last: {}, First: {} - Easy to access individually",
        comparison.component_name.0, comparison.component_name.1
    );
    println!("\n   Component field (3 components):");
    println!(
        "     Last: {}, First: {}, Middle: {} - Full structure",
        comparison.full_name.0, comparison.full_name.1, comparison.full_name.2
    );

    // Example 9: Working with message batches
    println!("\n9. Multiple segments with components:");
    println!("   ------------------------------------------");

    let batch_msg = parse_message(
        "MSH|^~\\&|Lab|Hospital|EMR|Clinic|20250119120000||ORU^R01|BATCH001|P|2.5\r\
         ZPN|PAT-001|Smith^John^A|Doe^Jane\r\
         ZPN|PAT-002|Johnson^Mary^B|Johnson^Bob\r\
         ZPN|PAT-003|Williams^David^C|White^Alice\r",
    )?;

    let all_zpns = batch_msg.get_custom_segments::<ZPN>()?;
    println!("   Found {} patient name records:", all_zpns.len());
    for (i, zpn) in all_zpns.iter().enumerate() {
        println!(
            "   {}. Patient {}: {} {}, Emergency: {} {}",
            i + 1,
            zpn.patient_id,
            zpn.patient_name.1,      // First
            zpn.patient_name.0,      // Last
            zpn.emergency_contact.1, // Emergency first
            zpn.emergency_contact.0
        ); // Emergency last
    }

    // Example 10: Optional component fields
    println!("\n10. Optional component fields:");
    println!("    ------------------------------------------");

    z_segment! {
        ZOC,
        id = "ZOC",
        fields = {
            1 => patient_id: String,
            2 => primary_physician: (String, String, String),       // Required: Last^First^Credentials
            3 => secondary_physician: Option<(String, String, String)>, // Optional
            4 => maiden_name: Option<(String, String)>,             // Optional: Last^First
        }
    }

    // Create segment with optional fields present
    let with_optional = ZOC::builder()
        .patient_id("PAT-001")
        .primary_physician(("Smith".to_string(), "John".to_string(), "MD".to_string()))
        .secondary_physician(Some((
            "Doe".to_string(),
            "Jane".to_string(),
            "RN".to_string(),
        )))
        .maiden_name(Some(("Johnson".to_string(), "Mary".to_string())))
        .build()?;

    println!("    Patient with optional fields:");
    println!("      Patient ID: {}", with_optional.patient_id);
    println!(
        "      Primary: Dr. {} {}",
        with_optional.primary_physician.1, with_optional.primary_physician.0
    );

    if let Some((last, first, cred)) = &with_optional.secondary_physician {
        println!("      Secondary: {} {} {}", cred, first, last);
    }

    if let Some((last, first)) = &with_optional.maiden_name {
        println!("      Maiden name: {} {}", first, last);
    }

    // Create segment without optional fields
    let without_optional = ZOC::builder()
        .patient_id("PAT-002")
        .primary_physician((
            "Williams".to_string(),
            "Sarah".to_string(),
            "DO".to_string(),
        ))
        .build()?; // No secondary_physician or maiden_name

    println!("\n    Patient without optional fields:");
    println!("      Patient ID: {}", without_optional.patient_id);
    println!(
        "      Primary: Dr. {} {}",
        without_optional.primary_physician.1, without_optional.primary_physician.0
    );
    println!(
        "      Secondary: {:?}",
        without_optional.secondary_physician
    );
    println!("      Maiden name: {:?}", without_optional.maiden_name);

    // HL7 encoding
    let encoded_with = with_optional.to_segment().encode(&delimiters);
    let encoded_without = without_optional.to_segment().encode(&delimiters);

    println!("\n    HL7 encoding:");
    println!("      With optional: {}", encoded_with);
    println!("      Without optional: {}", encoded_without);
    println!("      (Note: Optional fields appear as empty when None)");

    // Parsing messages with optional components
    let msg_with_optional = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         ZOC|PAT-003|Brown^Robert^MD|White^Alice^NP|Anderson^Beth\r",
    )?;

    let msg_without_optional = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         ZOC|PAT-004|Taylor^David^MD||\r",
    )?;

    println!("\n    Parsing HL7 messages:");
    if let Some(zoc) = msg_with_optional.get_custom_segment::<ZOC>()? {
        println!(
            "      Patient {}: Primary={} {}, Secondary={:?}, Maiden={:?}",
            zoc.patient_id,
            zoc.primary_physician.1,
            zoc.primary_physician.0,
            zoc.secondary_physician
                .as_ref()
                .map(|(l, f, c)| format!("{} {} {}", c, f, l)),
            zoc.maiden_name
                .as_ref()
                .map(|(l, f)| format!("{} {}", f, l))
        );
    }

    if let Some(zoc) = msg_without_optional.get_custom_segment::<ZOC>()? {
        println!(
            "      Patient {}: Primary={} {}, Secondary={:?}, Maiden={:?}",
            zoc.patient_id,
            zoc.primary_physician.1,
            zoc.primary_physician.0,
            zoc.secondary_physician,
            zoc.maiden_name
        );
    }

    // Example 11: Component field benefits
    println!("\n11. Benefits of component fields:");
    println!("    ------------------------------------------");

    println!("    ✓ Type-safe access to structured data");
    println!("    ✓ No string parsing required");
    println!("    ✓ Compile-time field count validation");
    println!("    ✓ Self-documenting code with tuple destructuring");
    println!("    ✓ Follows HL7 v2.x component structure exactly");
    println!("    ✓ Works seamlessly with HL7 ^ (caret) separator");
    println!("    ✓ Optional components with Option<Tuple> support");

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
