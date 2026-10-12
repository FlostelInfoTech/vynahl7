//! Example demonstrating repeating field support (Vec<T>) in custom Z-segments
//!
//! This example shows how to use Vec<T> for fields that can have multiple values:
//! - Vec<String> for multiple text values (phone numbers, addresses)
//! - Vec<u32> for multiple numeric IDs
//! - Vec<i32> for multiple signed values
//! - Vec<f64> for multiple measurements
//! - Vec<bool> for multiple flags
//!
//! Run with: cargo run --example repeating_fields

use vyna_hl7_core::Delimiters;
use vyna_hl7_custom::{z_segment, CustomSegment, MessageExt};
use vyna_hl7_parser::parse_message;

// Define a Z-segment with repeating contact information
z_segment! {
    ZCT,
    id = "ZCT",
    fields = {
        1 => patient_id: String,
        2 => phone_numbers: Vec<String>,         // Multiple phone numbers
        3 => email_addresses: Vec<String>,       // Multiple emails
        4 => emergency_contact_ids: Vec<u32>,    // Multiple contact person IDs
    }
}

// Define a Z-segment with repeating lab results
z_segment! {
    ZLR,
    id = "ZLR",
    fields = {
        1 => patient_id: String,
        2 => test_ids: Vec<u32>,                 // Multiple test IDs
        3 => glucose_readings: Vec<f64>,         // Multiple glucose measurements
        4 => temperature_deltas: Vec<i32>,       // Temperature changes (can be negative)
        5 => abnormal_flags: Vec<bool>,          // Abnormal result flags
    }
}

// Define a Z-segment with repeating diagnosis codes
z_segment! {
    ZDX,
    id = "ZDX",
    fields = {
        1 => patient_id: String,
        2 => diagnosis_codes: Vec<String>,       // ICD-10 codes
        3 => severity_scores: Vec<u32>,          // Severity ratings (1-10)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Repeating Fields (Vec<T>) Example ===\n");

    // Example 1: Create a segment with repeating phone numbers
    println!("1. Creating ZCT segment with multiple phone numbers:");
    println!("   ------------------------------------------");

    let zct = ZCT::builder()
        .patient_id("PAT-12345")
        .phone_numbers(vec![
            "555-1234".to_string(),
            "555-5678".to_string(),
            "555-9999".to_string(),
        ])
        .email_addresses(vec![
            "patient@email.com".to_string(),
            "patient.work@company.com".to_string(),
        ])
        .emergency_contact_ids(vec![101, 102, 103])
        .build()?;

    println!("   Patient ID: {}", zct.patient_id);
    println!("   Phone Numbers: {:?}", zct.phone_numbers);
    println!("   Email Addresses: {:?}", zct.email_addresses);
    println!("   Emergency Contact IDs: {:?}", zct.emergency_contact_ids);

    // Example 2: HL7 encoding with ~ separator
    println!("\n2. HL7 encoding of repeating fields:");
    println!("   ------------------------------------------");

    let delimiters = Delimiters::default();
    let segment = zct.to_segment();
    let encoded = segment.encode(&delimiters);
    println!("   {}", encoded);
    println!("\n   Note: The ~ (tilde) character separates repeated values");
    println!("   Example: 555-1234~555-5678~555-9999");

    // Example 3: Create lab results with multiple readings
    println!("\n3. Creating ZLR segment with multiple lab readings:");
    println!("   ------------------------------------------");

    let zlr = ZLR::builder()
        .patient_id("PAT-67890")
        .test_ids(vec![1001, 1002, 1003, 1004])
        .glucose_readings(vec![95.5, 98.2, 102.3, 99.8])
        .temperature_deltas(vec![-2, 0, 1, -1])  // Temperature changes from baseline
        .abnormal_flags(vec![false, false, true, false])
        .build()?;

    println!("   Patient ID: {}", zlr.patient_id);
    println!("   Test IDs: {:?}", zlr.test_ids);
    println!("   Glucose Readings: {:?}", zlr.glucose_readings);
    println!("   Temperature Deltas: {:?}", zlr.temperature_deltas);
    println!("   Abnormal Flags: {:?}", zlr.abnormal_flags);

    let zlr_encoded = zlr.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", zlr_encoded);

    // Example 4: Parsing HL7 messages with repeating fields
    println!("\n4. Parsing HL7 messages with repeating fields:");
    println!("   ------------------------------------------");

    let hl7_message = format!(
        "MSH|^~\\&|LabSystem|Hospital|EMR|Clinic|20250119120000||ORU^R01|MSG001|P|2.5\r\
         PID|1||PAT-99999||Smith^John||19800101|M\r\
         {}\r",
        "ZDX|PAT-99999|I10~E11.9~I25.10~J44.0|3~5~7~4"
    );

    let message = parse_message(&hl7_message)?;

    if let Some(zdx) = message.get_custom_segment::<ZDX>()? {
        println!("   Successfully parsed ZDX segment:");
        println!("   - Patient: {}", zdx.patient_id);
        println!("   - Diagnosis Codes: {:?}", zdx.diagnosis_codes);
        println!("   - Severity Scores: {:?}", zdx.severity_scores);
        println!("\n   Diagnosis Details:");
        for (code, severity) in zdx.diagnosis_codes.iter().zip(zdx.severity_scores.iter()) {
            println!("     * {} (severity: {})", code, severity);
        }
    }

    // Example 5: Empty repeating fields
    println!("\n5. Empty repeating fields:");
    println!("   ------------------------------------------");

    let minimal_zct = ZCT::builder()
        .patient_id("PAT-00001")
        .phone_numbers(vec![])  // Empty vector - no phone numbers
        .email_addresses(vec![])
        .emergency_contact_ids(vec![])
        .build()?;

    println!("   Created segment with empty repeating fields:");
    println!("   - Patient ID: {}", minimal_zct.patient_id);
    println!("   - Phone Numbers: {:?}", minimal_zct.phone_numbers);
    println!("   - Email Addresses: {:?}", minimal_zct.email_addresses);

    let minimal_encoded = minimal_zct.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", minimal_encoded);
    println!("   (Empty fields appear as consecutive delimiters)");

    // Example 6: Single vs. multiple values
    println!("\n6. Single vs. multiple values:");
    println!("   ------------------------------------------");

    let single_phone = ZCT::builder()
        .patient_id("PAT-00002")
        .phone_numbers(vec!["555-1111".to_string()])  // Single value in Vec
        .email_addresses(vec![])
        .emergency_contact_ids(vec![])
        .build()?;

    println!("   Single phone number in Vec:");
    println!("   - Phone Numbers: {:?}", single_phone.phone_numbers);

    let multiple_phones = ZCT::builder()
        .patient_id("PAT-00003")
        .phone_numbers(vec![
            "555-2222".to_string(),
            "555-3333".to_string(),
            "555-4444".to_string(),
            "555-5555".to_string(),
        ])
        .email_addresses(vec![])
        .emergency_contact_ids(vec![])
        .build()?;

    println!("\n   Multiple phone numbers in Vec:");
    println!("   - Phone Numbers: {:?}", multiple_phones.phone_numbers);
    println!("   - Count: {}", multiple_phones.phone_numbers.len());

    // Example 7: Modifying repeating fields
    println!("\n7. Modifying repeating fields:");
    println!("   ------------------------------------------");

    let mut msg = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         PID|1||PAT-55555||Doe^Jane\r\
         ZCT|PAT-55555|555-0001|jane@email.com|201\r"
    )?;

    println!("   Original contact info:");
    if let Some(original) = msg.get_custom_segment::<ZCT>()? {
        println!("   - Phone Numbers: {:?}", original.phone_numbers);
        println!("   - Email Addresses: {:?}", original.email_addresses);
    }

    // Add more phone numbers and emails
    let mut updated_zct = msg.get_custom_segment::<ZCT>()?.unwrap();
    updated_zct.phone_numbers.push("555-0002".to_string());
    updated_zct.phone_numbers.push("555-0003".to_string());
    updated_zct.email_addresses.push("jane.work@company.com".to_string());
    updated_zct.emergency_contact_ids.push(202);

    msg.set_custom_segment(updated_zct)?;

    println!("\n   After adding more values:");
    if let Some(updated) = msg.get_custom_segment::<ZCT>()? {
        println!("   - Phone Numbers: {:?}", updated.phone_numbers);
        println!("   - Email Addresses: {:?}", updated.email_addresses);
        println!("   - Emergency Contact IDs: {:?}", updated.emergency_contact_ids);
    }

    // Example 8: Real-world use case - tracking allergies
    println!("\n8. Real-world use case: Patient allergies:");
    println!("   ------------------------------------------");

    z_segment! {
        ZAL,
        id = "ZAL",
        fields = {
            1 => patient_id: String,
            2 => allergen_codes: Vec<String>,     // SNOMED-CT codes
            3 => severity_levels: Vec<u32>,       // 1=Mild, 2=Moderate, 3=Severe
            4 => verified_flags: Vec<bool>,       // Verified by physician
        }
    }

    let allergies = ZAL::builder()
        .patient_id("PAT-11111")
        .allergen_codes(vec![
            "91935009".to_string(),   // Allergy to peanuts
            "300916003".to_string(),  // Allergy to latex
            "419199007".to_string(),  // Allergy to penicillin
        ])
        .severity_levels(vec![3, 2, 3])  // Severe, Moderate, Severe
        .verified_flags(vec![true, true, false])  // First two verified
        .build()?;

    println!("   Patient ID: {}", allergies.patient_id);
    println!("\n   Allergy Profile:");
    for i in 0..allergies.allergen_codes.len() {
        let severity = match allergies.severity_levels[i] {
            1 => "Mild",
            2 => "Moderate",
            3 => "Severe",
            _ => "Unknown",
        };
        let verified = if allergies.verified_flags[i] { "Verified" } else { "Unverified" };
        println!("     * Code: {} - {} ({})",
            allergies.allergen_codes[i], severity, verified);
    }

    let allergy_encoded = allergies.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", allergy_encoded);

    // Example 9: Combining different Vec types
    println!("\n9. Combining different Vec<T> types:");
    println!("   ------------------------------------------");

    z_segment! {
        ZMX,
        id = "ZMX",
        fields = {
            1 => record_id: String,
            2 => tags: Vec<String>,              // Text tags
            3 => counts: Vec<u32>,               // Counters
            4 => adjustments: Vec<i32>,          // Can be positive or negative
            5 => measurements: Vec<f64>,         // Decimal values
            6 => flags: Vec<bool>,               // Boolean indicators
        }
    }

    let mixed = ZMX::builder()
        .record_id("REC-001")
        .tags(vec!["urgent".to_string(), "reviewed".to_string(), "flagged".to_string()])
        .counts(vec![5, 12, 8])
        .adjustments(vec![-3, 7, -1, 4])
        .measurements(vec![98.6, 120.5, 80.2])
        .flags(vec![true, false, true, true, false])
        .build()?;

    println!("   Record ID: {}", mixed.record_id);
    println!("   Tags (Vec<String>): {:?}", mixed.tags);
    println!("   Counts (Vec<u32>): {:?}", mixed.counts);
    println!("   Adjustments (Vec<i32>): {:?}", mixed.adjustments);
    println!("   Measurements (Vec<f64>): {:?}", mixed.measurements);
    println!("   Flags (Vec<bool>): {:?}", mixed.flags);

    let mixed_encoded = mixed.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", mixed_encoded);
    println!("   (Each Vec is encoded with ~ separators)");

    // Example 10: Parsing with varying repetition counts
    println!("\n10. Parsing fields with different repetition counts:");
    println!("    ------------------------------------------");

    let varied_msg = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         ZMX|REC-002|tag1~tag2|100|5~-2~3|12.5~13.8~14.2~15.0|Y~N\r"
    )?;

    if let Some(parsed) = varied_msg.get_custom_segment::<ZMX>()? {
        println!("    Successfully parsed segment with varying repetitions:");
        println!("    - Tags: {} values = {:?}", parsed.tags.len(), parsed.tags);
        println!("    - Counts: {} values = {:?}", parsed.counts.len(), parsed.counts);
        println!("    - Adjustments: {} values = {:?}", parsed.adjustments.len(), parsed.adjustments);
        println!("    - Measurements: {} values = {:?}", parsed.measurements.len(), parsed.measurements);
        println!("    - Flags: {} values = {:?}", parsed.flags.len(), parsed.flags);
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
