//! Example demonstrating repeating component fields (Vec<Tuple>) in custom Z-segments
//!
//! This example shows how to use Vec<Tuple> for fields with multiple structured values.
//! This combines both repetitions (~ separator) and components (^ separator).
//!
//! Supported types:
//! - Vec<(String, String)> - Multiple 2-component values
//! - Vec<(String, String, String)> - Multiple 3-component values
//! - Vec<(String, String, String, String)> - Multiple 4-component values
//! - Vec<(String, String, String, String, String)> - Multiple 5-component values
//!
//! HL7 encoding: "value1^type1~value2^type2~value3^type3"
//! Components separated by ^ (caret), repetitions separated by ~ (tilde)
//!
//! Run with: cargo run --example repeating_components

use vyna_hl7_core::Delimiters;
use vyna_hl7_custom::{CustomSegment, MessageExt, z_segment};
use vyna_hl7_parser::parse_message;

// Define a Z-segment with multiple phone numbers (each with number and type)
z_segment! {
    ZPH,
    id = "ZPH",
    fields = {
        1 => patient_id: String,
        2 => phone_numbers: Vec<(String, String)>,  // Number^Type (Home/Work/Mobile)
    }
}

// Define a Z-segment with multiple addresses
z_segment! {
    ZAD,
    id = "ZAD",
    fields = {
        1 => patient_id: String,
        2 => addresses: Vec<(String, String, String, String, String)>, // Street^City^State^Zip^Country
    }
}

// Define a Z-segment with multiple identifiers
z_segment! {
    ZID,
    id = "ZID",
    fields = {
        1 => patient_id: String,
        2 => identifiers: Vec<(String, String, String, String)>, // ID^Type^Authority^Facility
    }
}

// Define a Z-segment with multiple provider names
z_segment! {
    ZPR,
    id = "ZPR",
    fields = {
        1 => patient_id: String,
        2 => providers: Vec<(String, String, String)>, // Last^First^Credentials
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Repeating Component Fields (Vec<Tuple>) Example ===\n");

    // Example 1: Multiple phone numbers with type
    println!("1. Multiple phone numbers (2 components each):");
    println!("   ------------------------------------------");

    let zph = ZPH::builder()
        .patient_id("PAT-12345")
        .phone_numbers(vec![
            ("555-1234".to_string(), "Home".to_string()),
            ("555-5678".to_string(), "Work".to_string()),
            ("555-9999".to_string(), "Mobile".to_string()),
        ])
        .build()?;

    println!("   Patient ID: {}", zph.patient_id);
    println!("   Phone Numbers:");
    for (i, (number, phone_type)) in zph.phone_numbers.iter().enumerate() {
        println!("     {}. {} ({})", i + 1, number, phone_type);
    }

    // HL7 encoding
    let delimiters = Delimiters::default();
    let segment = zph.to_segment();
    let encoded = segment.encode(&delimiters);
    println!("\n   HL7 Encoded: {}", encoded);
    println!("   (Note: ^ separates components, ~ separates repetitions)");

    // Example 2: Multiple full addresses
    println!("\n2. Multiple addresses (5 components each):");
    println!("   ------------------------------------------");

    let zad = ZAD::builder()
        .patient_id("PAT-67890")
        .addresses(vec![
            (
                "123 Main Street".to_string(),
                "Springfield".to_string(),
                "IL".to_string(),
                "62701".to_string(),
                "USA".to_string(),
            ),
            (
                "456 Oak Avenue".to_string(),
                "Chicago".to_string(),
                "IL".to_string(),
                "60601".to_string(),
                "USA".to_string(),
            ),
        ])
        .build()?;

    println!("   Patient ID: {}", zad.patient_id);
    println!("   Addresses:");
    for (i, (street, city, state, zip, country)) in zad.addresses.iter().enumerate() {
        println!(
            "     {}. {} {}, {} {} {}",
            i + 1,
            street,
            city,
            state,
            zip,
            country
        );
    }

    let zad_encoded = zad.to_segment().encode(&delimiters);
    println!("\n   HL7 Encoded: {}", zad_encoded);

    // Example 3: Multiple identifiers
    println!("\n3. Multiple identifiers (4 components each):");
    println!("   ------------------------------------------");

    let zid = ZID::builder()
        .patient_id("PAT-99999")
        .identifiers(vec![
            (
                "MRN123".to_string(),
                "MRN".to_string(),
                "Hospital".to_string(),
                "MainCampus".to_string(),
            ),
            (
                "ACC456".to_string(),
                "Account".to_string(),
                "Billing".to_string(),
                "West".to_string(),
            ),
            (
                "SSN789".to_string(),
                "SSN".to_string(),
                "Federal".to_string(),
                "National".to_string(),
            ),
        ])
        .build()?;

    println!("   Patient ID: {}", zid.patient_id);
    println!("   Identifiers:");
    for (i, (id, id_type, authority, facility)) in zid.identifiers.iter().enumerate() {
        println!(
            "     {}. ID: {}, Type: {}, Authority: {}, Facility: {}",
            i + 1,
            id,
            id_type,
            authority,
            facility
        );
    }

    let zid_encoded = zid.to_segment().encode(&delimiters);
    println!("\n   HL7 Encoded: {}", zid_encoded);

    // Example 4: Parsing HL7 messages with repeating components
    println!("\n4. Parsing HL7 messages with repeating components:");
    println!("   ------------------------------------------");

    let hl7_message = format!(
        "MSH|^~\\&|SendApp|SendFac|RecvApp|RecvFac|20250119120000||ADT^A01|MSG001|P|2.5\r\
         PID|1||PAT-11111||Smith^John||19800101|M\r\
         {}\r",
        "ZPR|PAT-11111|Williams^Sarah^MD~Brown^Robert^DO~Johnson^Alice^NP"
    );

    let message = parse_message(&hl7_message)?;

    if let Some(zpr) = message.get_custom_segment::<ZPR>()? {
        println!("   Successfully parsed ZPR segment:");
        println!("   Patient ID: {}", zpr.patient_id);
        println!("   Providers:");
        for (i, (last, first, cred)) in zpr.providers.iter().enumerate() {
            println!("     {}. {} {}, {}", i + 1, cred, first, last);
        }
    }

    // Example 5: Empty repeating component fields
    println!("\n5. Empty repeating component fields:");
    println!("   ------------------------------------------");

    let minimal_zph = ZPH::builder()
        .patient_id("PAT-00001")
        .phone_numbers(vec![]) // No phone numbers
        .build()?;

    println!("   Created segment with empty repeating components:");
    println!("   - Patient ID: {}", minimal_zph.patient_id);
    println!("   - Phone Numbers: {:?}", minimal_zph.phone_numbers);

    let minimal_encoded = minimal_zph.to_segment().encode(&delimiters);
    println!("\n   HL7 Encoded: {}", minimal_encoded);
    println!("   (Empty fields appear as consecutive delimiters)");

    // Example 6: Single vs. multiple repetitions
    println!("\n6. Single vs. multiple repetitions:");
    println!("   ------------------------------------------");

    let single_phone = ZPH::builder()
        .patient_id("PAT-00002")
        .phone_numbers(vec![("555-1111".to_string(), "Home".to_string())])
        .build()?;

    println!("   Single phone number:");
    println!("   {:?}", single_phone.phone_numbers);

    let multiple_phones = ZPH::builder()
        .patient_id("PAT-00003")
        .phone_numbers(vec![
            ("555-2222".to_string(), "Home".to_string()),
            ("555-3333".to_string(), "Work".to_string()),
            ("555-4444".to_string(), "Mobile".to_string()),
            ("555-5555".to_string(), "Fax".to_string()),
        ])
        .build()?;

    println!("\n   Multiple phone numbers:");
    for phone in &multiple_phones.phone_numbers {
        println!("   - {}: {}", phone.0, phone.1);
    }
    println!("   Count: {}", multiple_phones.phone_numbers.len());

    // Example 7: Modifying repeating component fields
    println!("\n7. Modifying repeating component fields:");
    println!("   ------------------------------------------");

    let mut msg = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         PID|1||PAT-55555||Doe^Jane\r\
         ZPH|PAT-55555|555-0001^Home\r",
    )?;

    println!("   Original phone numbers:");
    if let Some(original) = msg.get_custom_segment::<ZPH>()? {
        for phone in &original.phone_numbers {
            println!("   - {}: {}", phone.0, phone.1);
        }
    }

    // Add more phone numbers
    let mut updated_zph = msg.get_custom_segment::<ZPH>()?.unwrap();
    updated_zph
        .phone_numbers
        .push(("555-0002".to_string(), "Work".to_string()));
    updated_zph
        .phone_numbers
        .push(("555-0003".to_string(), "Mobile".to_string()));

    msg.set_custom_segment(updated_zph)?;

    println!("\n   After adding more phone numbers:");
    if let Some(updated) = msg.get_custom_segment::<ZPH>()? {
        for phone in &updated.phone_numbers {
            println!("   - {}: {}", phone.0, phone.1);
        }
    }

    // Example 8: Real-world use case - Emergency contacts
    println!("\n8. Real-world use case: Emergency contacts:");
    println!("   ------------------------------------------");

    z_segment! {
        ZEC,
        id = "ZEC",
        fields = {
            1 => patient_id: String,
            2 => emergency_contacts: Vec<(String, String, String)>, // Name^Relationship^Phone
        }
    }

    let emergency = ZEC::builder()
        .patient_id("PAT-22222")
        .emergency_contacts(vec![
            (
                "Jane Smith".to_string(),
                "Spouse".to_string(),
                "555-1111".to_string(),
            ),
            (
                "Robert Johnson".to_string(),
                "Father".to_string(),
                "555-2222".to_string(),
            ),
            (
                "Alice Williams".to_string(),
                "Sister".to_string(),
                "555-3333".to_string(),
            ),
        ])
        .build()?;

    println!("   Patient ID: {}", emergency.patient_id);
    println!("\n   Emergency Contacts:");
    for (i, (name, relationship, phone)) in emergency.emergency_contacts.iter().enumerate() {
        println!("     {}. {} ({}) - {}", i + 1, name, relationship, phone);
    }

    let emergency_encoded = emergency.to_segment().encode(&delimiters);
    println!("\n   HL7 Encoded: {}", emergency_encoded);

    // Example 9: Combining with other field types
    println!("\n9. Combining repeating components with other field types:");
    println!("   ------------------------------------------");

    z_segment! {
        ZMX,
        id = "ZMX",
        fields = {
            1 => patient_id: String,                           // Simple field
            2 => primary_phone: (String, String),              // Single component field
            3 => alternate_phones: Vec<(String, String)>,      // Repeating component field
            4 => verified: bool,                                // Boolean field
        }
    }

    let mixed = ZMX::builder()
        .patient_id("PAT-33333")
        .primary_phone(("555-9999".to_string(), "Primary".to_string()))
        .alternate_phones(vec![
            ("555-8888".to_string(), "Alt1".to_string()),
            ("555-7777".to_string(), "Alt2".to_string()),
        ])
        .verified(true)
        .build()?;

    println!("   Patient ID: {}", mixed.patient_id);
    println!(
        "   Primary Phone: {} ({})",
        mixed.primary_phone.0, mixed.primary_phone.1
    );
    println!("   Alternate Phones:");
    for phone in &mixed.alternate_phones {
        println!("     - {}: {}", phone.0, phone.1);
    }
    println!("   Verified: {}", mixed.verified);

    let mixed_encoded = mixed.to_segment().encode(&delimiters);
    println!("\n   HL7 Encoded: {}", mixed_encoded);

    // Example 10: Parsing messages with varying repetition counts
    println!("\n10. Parsing with varying repetition counts:");
    println!("    ------------------------------------------");

    let varied_msg = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         ZPH|PAT-44444|555-1111^Type1~555-2222^Type2\r\
         ZPH|PAT-55555|555-3333^Type3~555-4444^Type4~555-5555^Type5~555-6666^Type6\r",
    )?;

    let all_zphs = varied_msg.get_custom_segments::<ZPH>()?;
    println!("    Found {} ZPH segments:", all_zphs.len());
    for (i, zph) in all_zphs.iter().enumerate() {
        println!(
            "    {}. Patient {}: {} phone numbers",
            i + 1,
            zph.patient_id,
            zph.phone_numbers.len()
        );
        for phone in &zph.phone_numbers {
            println!("       - {}: {}", phone.0, phone.1);
        }
    }

    // Example 11: Benefits of repeating component fields
    println!("\n11. Benefits of Vec<Tuple> repeating component fields:");
    println!("    ------------------------------------------");

    println!("    ✓ Type-safe access to structured repeating data");
    println!("    ✓ No manual parsing of HL7 separators required");
    println!("    ✓ Compile-time validation of component count");
    println!("    ✓ Iterator support for easy processing");
    println!("    ✓ Follows HL7 v2.x specification exactly");
    println!("    ✓ Combines ~ (repetition) and ^ (component) separators");
    println!("    ✓ Mutable access for adding/removing repetitions");
    println!("    ✓ Works seamlessly with other field types");

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
