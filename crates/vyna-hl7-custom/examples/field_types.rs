//! Example demonstrating all supported field types in custom Z-segments
//!
//! This example shows how to use various field types including:
//! - String (text fields)
//! - u32 (unsigned integers)
//! - f64 (floating point numbers)
//! - i32 (signed integers)
//! - i64 (large signed integers)
//! - bool (boolean flags)
//!
//! Run with: cargo run --example field_types

use vyna_hl7_core::Delimiters;
use vyna_hl7_custom::{CustomSegment, MessageExt, z_segment};
use vyna_hl7_parser::parse_message;

// Define a comprehensive Z-segment demonstrating all field types
z_segment! {
    ZFT,
    id = "ZFT",
    fields = {
        1 => patient_id: String,                    // Text field
        2 => age: u32,                              // Unsigned integer (0 and up)
        3 => weight_kg: f64,                        // Floating point number
        4 => temperature_delta: i32,                // Signed integer (can be negative)
        5 => account_balance: i64,                  // Large signed integer
        6 => is_active: bool,                       // Boolean flag
        7 => insurance_verified: bool,              // Boolean flag
        8 => bed_number: Option<u32>,               // Optional unsigned integer
        9 => systolic_bp: Option<i32>,              // Optional signed integer
        10 => account_id: Option<i64>,              // Optional large integer
        11 => consent_given: Option<bool>,          // Optional boolean
        12 => notes: Option<String>,                // Optional text
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Custom Z-Segment Field Types Example ===\n");

    // Example 1: Create a Z-segment using the builder with all field types
    println!("1. Creating ZFT segment with builder API:");
    println!("   ------------------------------------------");

    let zft = ZFT::builder()
        .patient_id("PAT-12345")
        .age(45u32)
        .weight_kg(72.5)
        .temperature_delta(-2) // Negative temperature change
        .account_balance(150000i64) // Large account balance
        .is_active(true) // Boolean: patient is active
        .insurance_verified(false) // Boolean: insurance not yet verified
        .bed_number(305u32) // Optional: has bed assignment
        .systolic_bp(120) // Optional: blood pressure recorded
        .account_id(9876543210i64) // Optional: large account ID
        .consent_given(true) // Optional: consent was given
        .notes("Regular checkup") // Optional: notes present
        .build()?;

    println!("   Patient ID: {}", zft.patient_id);
    println!("   Age: {} years", zft.age);
    println!("   Weight: {} kg", zft.weight_kg);
    println!("   Temperature Delta: {}°C", zft.temperature_delta);
    println!("   Account Balance: ${}", zft.account_balance);
    println!("   Is Active: {}", zft.is_active);
    println!("   Insurance Verified: {}", zft.insurance_verified);
    println!("   Bed Number: {:?}", zft.bed_number);
    println!("   Systolic BP: {:?}", zft.systolic_bp);
    println!("   Account ID: {:?}", zft.account_id);
    println!("   Consent Given: {:?}", zft.consent_given);
    println!("   Notes: {:?}", zft.notes);

    // Example 2: Convert to HL7 segment
    println!("\n2. Converting to HL7 segment:");
    println!("   ------------------------------------------");

    let delimiters = Delimiters::default();
    let segment = zft.to_segment();
    let encoded = segment.encode(&delimiters);
    println!("   {}", encoded);

    // Show how booleans are encoded
    println!("\n   Note: Booleans are encoded as:");
    println!("   - true  → 'Y'");
    println!("   - false → 'N'");

    // Example 3: Parse from HL7 message
    println!("\n3. Parsing from HL7 message:");
    println!("   ------------------------------------------");

    let hl7_message = format!(
        "MSH|^~\\&|SendApp|SendFac|RecvApp|RecvFac|20250119120000||ADT^A01|MSG001|P|2.5\r\
         PID|1||PAT-67890||Doe^John||19800115|M\r\
         {}\r",
        "ZFT|PAT-67890|38|68.2|-5|250000|Y|Y|210|115|1234567890123|N|Post-operative care"
    );

    let message = parse_message(&hl7_message)?;

    if let Some(parsed_zft) = message.get_custom_segment::<ZFT>()? {
        println!("   Successfully parsed ZFT segment:");
        println!("   - Patient: {}", parsed_zft.patient_id);
        println!("   - Age: {}", parsed_zft.age);
        println!("   - Weight: {} kg", parsed_zft.weight_kg);
        println!("   - Temp Delta: {}°C", parsed_zft.temperature_delta);
        println!("   - Balance: ${}", parsed_zft.account_balance);
        println!("   - Active: {}", parsed_zft.is_active);
        println!("   - Verified: {}", parsed_zft.insurance_verified);
        println!("   - Bed: {:?}", parsed_zft.bed_number);
        println!("   - BP: {:?}", parsed_zft.systolic_bp);
        println!("   - Account: {:?}", parsed_zft.account_id);
        println!("   - Consent: {:?}", parsed_zft.consent_given);
        println!("   - Notes: {:?}", parsed_zft.notes);
    }

    // Example 4: Boolean parsing variations
    println!("\n4. Boolean field parsing supports multiple formats:");
    println!("   ------------------------------------------");

    let bool_examples = vec![
        ("Y", "Y"),
        ("N", "N"),
        ("YES", "YES"),
        ("NO", "NO"),
        ("T", "T"),
        ("F", "F"),
        ("TRUE", "TRUE"),
        ("FALSE", "FALSE"),
        ("1", "1"),
        ("0", "0"),
        ("yes", "yes (case-insensitive)"),
        ("No", "No (case-insensitive)"),
    ];

    for (value, desc) in bool_examples {
        let test_msg = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
             ZFT|PAT001|30|70.0|0|0|{}|Y|||||||",
            value
        );

        if let Ok(msg) = parse_message(&test_msg)
            && let Ok(Some(seg)) = msg.get_custom_segment::<ZFT>()
        {
            println!("   '{}' → {} (parsed as {})", value, desc, seg.is_active);
        }
    }

    // Example 5: Using optional fields
    println!("\n5. Handling optional fields:");
    println!("   ------------------------------------------");

    let minimal_zft = ZFT::builder()
        .patient_id("PAT-99999")
        .age(55u32)
        .weight_kg(80.0)
        .temperature_delta(0)
        .account_balance(0i64)
        .is_active(true)
        .insurance_verified(true)
        // All optional fields omitted
        .build()?;

    println!("   Created minimal segment (required fields only):");
    println!("   - Patient ID: {}", minimal_zft.patient_id);
    println!("   - Age: {}", minimal_zft.age);
    println!("   - Bed Number: {:?}", minimal_zft.bed_number);
    println!("   - Notes: {:?}", minimal_zft.notes);

    let minimal_encoded = minimal_zft.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", minimal_encoded);
    println!("   (Note: Optional fields are empty in output)");

    // Example 6: Numeric edge cases
    println!("\n6. Numeric type edge cases:");
    println!("   ------------------------------------------");

    let edge_case_zft = ZFT::builder()
        .patient_id("PAT-EDGE")
        .age(u32::MAX) // Maximum u32
        .weight_kg(f64::MAX) // Maximum f64
        .temperature_delta(i32::MIN) // Minimum i32 (most negative)
        .account_balance(i64::MIN) // Minimum i64 (most negative)
        .is_active(true)
        .insurance_verified(false)
        .build()?;

    println!("   u32 max: {}", edge_case_zft.age);
    println!("   f64 max: {}", edge_case_zft.weight_kg);
    println!("   i32 min: {}", edge_case_zft.temperature_delta);
    println!("   i64 min: {}", edge_case_zft.account_balance);

    // Example 7: Message manipulation with typed fields
    println!("\n7. Message manipulation with different types:");
    println!("   ------------------------------------------");

    let mut msg = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         PID|1||PAT-11111||Smith^Jane\r\
         ZFT|PAT-11111|25|65.5|2|100000|Y|N|||||Initial visit",
    )?;

    println!("   Original segment:");
    if let Some(original) = msg.get_custom_segment::<ZFT>()? {
        println!("   - Active: {}", original.is_active);
        println!("   - Verified: {}", original.insurance_verified);
        println!("   - Balance: ${}", original.account_balance);
    }

    // Modify the segment
    let mut modified_zft = msg.get_custom_segment::<ZFT>()?.unwrap();
    modified_zft.insurance_verified = true; // Update boolean
    modified_zft.account_balance += 50000; // Update large integer
    modified_zft.temperature_delta = -1; // Update signed integer
    modified_zft.bed_number = Some(410); // Add optional field

    msg.set_custom_segment(modified_zft)?;

    println!("\n   Modified segment:");
    if let Some(updated) = msg.get_custom_segment::<ZFT>()? {
        println!("   - Active: {}", updated.is_active);
        println!("   - Verified: {}", updated.insurance_verified);
        println!("   - Balance: ${}", updated.account_balance);
        println!("   - Bed: {:?}", updated.bed_number);
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
