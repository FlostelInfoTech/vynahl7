//! Comprehensive real-world example: Patient Admission (ADT^A01)
//!
//! This example demonstrates how to use ALL field types together to model
//! a real HL7 ADT^A01 (Admit/Visit Notification) message using custom Z-segments.
//!
//! Field Types Demonstrated:
//! - String, u32, i32, i64, f64, bool (primitives)
//! - NaiveDateTime, NaiveDate, NaiveTime, DateTime<Utc> (date/time)
//! - Option<T> (optional fields)
//! - Vec<T> (repeating fields)
//! - Tuple types (components)
//! - Option<Tuple> (optional components)
//! - Vec<Tuple> (repeating components)
//!
//! Segments Modeled:
//! - ZPI - Patient Information (demographics with all field types)
//! - ZAD - Admission Details (visit information)
//! - ZIN - Insurance Information (coverage and billing)
//! - ZAL - Allergy Information (clinical data)
//!
//! Run with: cargo run --example real_world_adt

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use vyna_hl7_core::Delimiters;
use vyna_hl7_custom::{CustomSegment, CustomSegmentError, MessageExt, z_segment};
use vyna_hl7_parser::parse_message;

// ============================================================================
// ZPI - Patient Information Segment
// Demonstrates: All primitive types, DateTime types, components, optional components
// ============================================================================
z_segment! {
    ZPI,
    id = "ZPI",
    fields = {
        1 => patient_id: String,                                    // MRN
        2 => patient_name: (String, String, String, String, String), // Last^First^Middle^Suffix^Prefix
        3 => birth_date: NaiveDate,                                 // YYYYMMDD
        4 => birth_time: Option<NaiveTime>,                         // Optional: HHMMSS
        5 => gender: String,                                        // M/F/O/U
        6 => ssn: Option<String>,                                   // Optional SSN
        7 => marital_status: Option<String>,                        // Optional: S/M/D/W
        8 => race: Option<String>,                                  // Optional race code
        9 => primary_language: Option<String>,                      // Optional language code
        10 => is_deceased: bool,                                    // Y/N
        11 => death_datetime: Option<NaiveDateTime>,                // Optional: when died
        12 => maiden_name: Option<(String, String)>,                // Optional: Last^First
        13 => multiple_birth_indicator: Option<bool>,               // Optional: Y/N
        14 => birth_order: Option<u32>,                             // Optional: 1, 2, 3...
    },
    validate = |s: &ZPI| {
        // Business rule: If deceased, must have death datetime
        if s.is_deceased && s.death_datetime.is_none() {
            return Err(CustomSegmentError::validation_failed(
                "ZPI",
                "Deceased patients must have death datetime"
            ));
        }

        // Business rule: Birth order only valid if multiple birth indicator is true
        if s.birth_order.is_some() && !s.multiple_birth_indicator.unwrap_or(false) {
            return Err(CustomSegmentError::validation_failed(
                "ZPI",
                "Birth order requires multiple birth indicator to be true"
            ));
        }

        Ok(())
    }
}

// ============================================================================
// ZAD - Admission Details Segment
// Demonstrates: DateTime, components, Vec<Tuple>, f64
// ============================================================================
z_segment! {
    ZAD,
    id = "ZAD",
    fields = {
        1 => patient_id: String,
        2 => admission_datetime: NaiveDateTime,                     // When admitted
        3 => discharge_datetime: Option<NaiveDateTime>,             // When discharged (if applicable)
        4 => patient_class: String,                                 // I=Inpatient, O=Outpatient, E=Emergency
        5 => patient_type: String,                                  // Visit type code
        6 => admitting_doctor: (String, String, String),           // Last^First^Credentials
        7 => attending_doctors: Vec<(String, String, String)>,     // Multiple doctors
        8 => hospital_service: Option<String>,                      // Department code
        9 => room_bed: Option<(String, String, String)>,           // Building^Room^Bed
        10 => admission_type: String,                               // Elective/Emergency/etc
        11 => readmission_indicator: bool,                          // Y/N
        12 => days_since_last_visit: Option<u32>,                   // Days
        13 => estimated_length_of_stay: Option<u32>,                // Days
        14 => total_charges: Option<f64>,                           // Dollar amount
        15 => deposit_amount: Option<f64>,                          // Dollar amount
    }
}

// ============================================================================
// ZIN - Insurance Information Segment
// Demonstrates: Vec<Tuple>, NaiveDate, i64, bool
// ============================================================================
z_segment! {
    ZIN,
    id = "ZIN",
    fields = {
        1 => patient_id: String,
        2 => insurance_plans: Vec<(String, String, String, String)>, // PlanID^PlanName^GroupNumber^Priority
        3 => policy_numbers: Vec<String>,                           // Multiple policy numbers
        4 => coverage_effective_date: NaiveDate,                    // When coverage starts
        5 => coverage_expiration_date: Option<NaiveDate>,           // When coverage ends
        6 => policy_holder_name: Option<(String, String)>,          // Last^First (if different from patient)
        7 => relationship_to_patient: Option<String>,               // Self/Spouse/Child/etc
        8 => annual_deductible: i64,                                // Dollar amount (can be 0)
        9 => deductible_met: i64,                                   // Dollar amount met
        10 => out_of_pocket_max: i64,                               // Maximum OOP
        11 => out_of_pocket_met: i64,                               // OOP met
        12 => is_pre_authorized: bool,                              // Y/N
        13 => authorization_numbers: Vec<String>,                   // Multiple auth numbers
        14 => copay_amount: Option<f64>,                            // Copay per visit
    }
}

// ============================================================================
// ZAL - Allergy Information Segment
// Demonstrates: Vec<Tuple>, NaiveDateTime, u32
// ============================================================================
z_segment! {
    ZAL,
    id = "ZAL",
    fields = {
        1 => patient_id: String,
        2 => allergies: Vec<(String, String, String)>,              // AllergenCode^AllergenName^Severity
        3 => reactions: Vec<String>,                                // Reaction descriptions
        4 => onset_datetime: Option<NaiveDateTime>,                 // When allergy discovered
        5 => verified_by: Option<(String, String, String)>,        // Doctor: Last^First^Credentials
        6 => verification_datetime: Option<NaiveDateTime>,          // When verified
        7 => severity_scores: Vec<u32>,                             // 1-10 severity (matches allergies)
        8 => is_life_threatening: bool,                             // Y/N
        9 => requires_epipen: bool,                                 // Y/N
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Real-World ADT^A01 Example: Patient Admission ===\n");
    println!("Demonstrating ALL field types in a practical scenario\n");

    // ========================================================================
    // Example 1: Create a complete patient admission record
    // ========================================================================
    println!("1. Creating Patient Information (ZPI):");
    println!("   -------------------------------------");

    let birth_date = NaiveDate::from_ymd_opt(1985, 6, 15).unwrap();
    let birth_time = NaiveTime::from_hms_opt(14, 30, 0).unwrap();

    let zpi = ZPI::builder()
        .patient_id("MRN-123456")
        .patient_name((
            "Smith".to_string(),
            "John".to_string(),
            "Alexander".to_string(),
            "Jr".to_string(),
            "Dr".to_string(),
        ))
        .birth_date(birth_date)
        .birth_time(birth_time)
        .gender("M")
        .ssn("123-45-6789")
        .marital_status("M") // Married
        .race("2106-3") // White
        .primary_language("en")
        .is_deceased(false)
        // Optional fields not set will default to None
        .multiple_birth_indicator(false)
        .build()?;

    println!(
        "   Patient: {} {} {} {}, {}",
        zpi.patient_name.4, // Prefix: Dr
        zpi.patient_name.1, // First: John
        zpi.patient_name.2, // Middle: Alexander
        zpi.patient_name.0, // Last: Smith
        zpi.patient_name.3
    ); // Suffix: Jr

    println!("   MRN: {}", zpi.patient_id);
    println!(
        "   DOB: {} at {}",
        zpi.birth_date,
        zpi.birth_time
            .map(|t| t.to_string())
            .unwrap_or("Unknown".to_string())
    );
    println!("   Gender: {}", zpi.gender);
    println!("   SSN: {}", zpi.ssn.as_ref().unwrap());
    println!(
        "   Marital Status: {}",
        zpi.marital_status.as_ref().unwrap()
    );
    println!("   Deceased: {}", zpi.is_deceased);

    // ========================================================================
    // Example 2: Create admission details
    // ========================================================================
    println!("\n2. Creating Admission Details (ZAD):");
    println!("   ----------------------------------");

    let admission_dt = NaiveDate::from_ymd_opt(2025, 1, 19)
        .unwrap()
        .and_hms_opt(10, 30, 0)
        .unwrap();

    let zad = ZAD::builder()
        .patient_id("MRN-123456")
        .admission_datetime(admission_dt)
        // discharge_datetime omitted (still admitted - will be None)
        .patient_class("I") // Inpatient
        .patient_type("SURGICAL")
        .admitting_doctor(("Johnson".to_string(), "Sarah".to_string(), "MD".to_string()))
        .attending_doctors(vec![
            ("Johnson".to_string(), "Sarah".to_string(), "MD".to_string()),
            (
                "Williams".to_string(),
                "Robert".to_string(),
                "DO".to_string(),
            ),
            ("Brown".to_string(), "Alice".to_string(), "NP".to_string()),
        ])
        .hospital_service("CARDIO")
        .room_bed(("North".to_string(), "302".to_string(), "A".to_string()))
        .admission_type("Elective")
        .readmission_indicator(false)
        .days_since_last_visit(180u32)
        .estimated_length_of_stay(3u32)
        .total_charges(45000.50)
        .deposit_amount(2000.00)
        .build()?;

    println!("   Admission: {}", zad.admission_datetime);
    println!(
        "   Patient Class: {} ({})",
        zad.patient_class,
        if zad.patient_class == "I" {
            "Inpatient"
        } else {
            "Other"
        }
    );
    println!(
        "   Admitting Doctor: {} {}, {}",
        zad.admitting_doctor.2, zad.admitting_doctor.1, zad.admitting_doctor.0
    );

    println!("   Attending Doctors ({}):", zad.attending_doctors.len());
    for (i, doctor) in zad.attending_doctors.iter().enumerate() {
        println!("     {}. {} {} {}", i + 1, doctor.2, doctor.1, doctor.0);
    }

    if let Some((building, room, bed)) = &zad.room_bed {
        println!(
            "   Location: Building {}, Room {}, Bed {}",
            building, room, bed
        );
    }

    println!("   Readmission: {}", zad.readmission_indicator);
    println!(
        "   Est. LOS: {} days",
        zad.estimated_length_of_stay.unwrap()
    );
    println!("   Total Charges: ${:.2}", zad.total_charges.unwrap());
    println!("   Deposit: ${:.2}", zad.deposit_amount.unwrap());

    // ========================================================================
    // Example 3: Create insurance information
    // ========================================================================
    println!("\n3. Creating Insurance Information (ZIN):");
    println!("   -------------------------------------");

    let coverage_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let coverage_end = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();

    let zin = ZIN::builder()
        .patient_id("MRN-123456")
        .insurance_plans(vec![
            (
                "BC-12345".to_string(),
                "Blue Cross PPO".to_string(),
                "GRP-789".to_string(),
                "1".to_string(), // Primary
            ),
            (
                "UHC-67890".to_string(),
                "United Healthcare".to_string(),
                "GRP-456".to_string(),
                "2".to_string(), // Secondary
            ),
        ])
        .policy_numbers(vec!["POL-ABC123".to_string(), "POL-XYZ789".to_string()])
        .coverage_effective_date(coverage_start)
        .coverage_expiration_date(coverage_end)
        // policy_holder_name omitted (patient is policyholder - will be None)
        .relationship_to_patient("Self")
        .annual_deductible(2000)
        .deductible_met(500)
        .out_of_pocket_max(6000)
        .out_of_pocket_met(1200)
        .is_pre_authorized(true)
        .authorization_numbers(vec!["AUTH-001".to_string(), "AUTH-002".to_string()])
        .copay_amount(35.00)
        .build()?;

    println!("   Insurance Plans ({}):", zin.insurance_plans.len());
    for (i, plan) in zin.insurance_plans.iter().enumerate() {
        println!(
            "     {}. {} (ID: {}, Group: {}, Priority: {})",
            i + 1,
            plan.1,
            plan.0,
            plan.2,
            plan.3
        );
    }

    println!(
        "\n   Coverage Period: {} to {}",
        zin.coverage_effective_date,
        zin.coverage_expiration_date
            .map(|d| d.to_string())
            .unwrap_or("Ongoing".to_string())
    );

    println!(
        "   Deductible: ${} / ${} met",
        zin.annual_deductible, zin.deductible_met
    );
    println!(
        "   Out-of-Pocket Max: ${} / ${} met",
        zin.out_of_pocket_max, zin.out_of_pocket_met
    );
    println!("   Pre-authorized: {}", zin.is_pre_authorized);
    println!("   Authorization Numbers: {:?}", zin.authorization_numbers);
    println!("   Copay: ${:.2}", zin.copay_amount.unwrap());

    // ========================================================================
    // Example 4: Create allergy information
    // ========================================================================
    println!("\n4. Creating Allergy Information (ZAL):");
    println!("   -----------------------------------");

    let onset_dt = NaiveDate::from_ymd_opt(2020, 3, 15)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let verified_dt = NaiveDate::from_ymd_opt(2020, 3, 16)
        .unwrap()
        .and_hms_opt(9, 0, 0)
        .unwrap();

    let zal = ZAL::builder()
        .patient_id("MRN-123456")
        .allergies(vec![
            (
                "91935009".to_string(), // SNOMED-CT code
                "Peanuts".to_string(),
                "Severe".to_string(),
            ),
            (
                "300916003".to_string(),
                "Latex".to_string(),
                "Moderate".to_string(),
            ),
            (
                "419199007".to_string(),
                "Penicillin".to_string(),
                "Severe".to_string(),
            ),
        ])
        .reactions(vec![
            "Anaphylaxis".to_string(),
            "Skin rash".to_string(),
            "Hives".to_string(),
        ])
        .onset_datetime(onset_dt)
        .verified_by(("Johnson".to_string(), "Sarah".to_string(), "MD".to_string()))
        .verification_datetime(verified_dt)
        .severity_scores(vec![9, 5, 8]) // Matches allergies count
        .is_life_threatening(true)
        .requires_epipen(true)
        .build()?;

    println!("   Allergies ({}):", zal.allergies.len());
    for (i, allergy) in zal.allergies.iter().enumerate() {
        println!(
            "     {}. {} (Code: {}, Severity: {}, Score: {})",
            i + 1,
            allergy.1, // Name
            allergy.0, // Code
            allergy.2, // Severity text
            zal.severity_scores[i]
        );
    }

    println!("\n   Reactions: {:?}", zal.reactions);

    if let Some(verified_by) = &zal.verified_by {
        println!(
            "   Verified By: {} {}, {}",
            verified_by.2, verified_by.1, verified_by.0
        );
    }

    if let Some(verified_dt) = zal.verification_datetime {
        println!("   Verified: {}", verified_dt);
    }

    println!("   Life Threatening: {}", zal.is_life_threatening);
    println!("   Requires EpiPen: {}", zal.requires_epipen);

    // ========================================================================
    // Example 5: Encode all segments to HL7
    // ========================================================================
    println!("\n5. HL7 Encoding:");
    println!("   -------------");

    let delimiters = Delimiters::default();

    let zpi_encoded = zpi.to_segment().encode(&delimiters);
    let zad_encoded = zad.to_segment().encode(&delimiters);
    let zin_encoded = zin.to_segment().encode(&delimiters);
    let zal_encoded = zal.to_segment().encode(&delimiters);

    println!("   ZPI: {}", zpi_encoded);
    println!("   ZAD: {}", zad_encoded);
    println!("   ZIN: {}", zin_encoded);
    println!("   ZAL: {}", zal_encoded);

    // ========================================================================
    // Example 6: Parse complete ADT^A01 message (roundtrip test)
    // ========================================================================
    println!("\n6. Parsing Complete ADT^A01 Message (Roundtrip Test):");
    println!("   --------------------------------------------------");

    // Create a complete message using the segments we already created
    let complete_message = format!(
        "MSH|^~\\&|HIS|MainHospital|EMR|Clinic|20250119103000||ADT^A01|MSG-12345|P|2.5\r\
         EVN|A01|20250119103000\r\
         PID|1||{}||{}^{}^{}^{}^{}||{}|{}\r\
         {}\r\
         {}\r\
         {}\r\
         {}\r",
        zpi.patient_id,
        zpi.patient_name.0,
        zpi.patient_name.1,
        zpi.patient_name.2,
        zpi.patient_name.3,
        zpi.patient_name.4,
        zpi.birth_date.format("%Y%m%d"),
        zpi.gender,
        zpi_encoded,
        zad_encoded,
        zin_encoded,
        zal_encoded
    );

    let message = parse_message(&complete_message)?;

    println!("   Successfully parsed ADT^A01 message");

    if let Some(parsed_zpi) = message.get_custom_segment::<ZPI>()? {
        println!(
            "   Patient: {} {} {} {}, {}",
            parsed_zpi.patient_name.4,
            parsed_zpi.patient_name.1,
            parsed_zpi.patient_name.2,
            parsed_zpi.patient_name.0,
            parsed_zpi.patient_name.3
        );
        println!("   Deceased: {}", parsed_zpi.is_deceased);
    }

    if let Some(parsed_zad) = message.get_custom_segment::<ZAD>()? {
        println!("   Patient Class: {}", parsed_zad.patient_class);
        println!(
            "   Attending Doctors: {}",
            parsed_zad.attending_doctors.len()
        );
        println!("   Readmission: {}", parsed_zad.readmission_indicator);
    }

    if let Some(parsed_zin) = message.get_custom_segment::<ZIN>()? {
        println!("   Insurance Plans: {}", parsed_zin.insurance_plans.len());
        println!("   Pre-authorized: {}", parsed_zin.is_pre_authorized);
        println!("   Deductible Met: ${}", parsed_zin.deductible_met);
    }

    if let Some(parsed_zal) = message.get_custom_segment::<ZAL>()? {
        println!("   Allergies: {}", parsed_zal.allergies.len());
        println!("   Life Threatening: {}", parsed_zal.is_life_threatening);
        println!("   Requires EpiPen: {}", parsed_zal.requires_epipen);
    }

    // ========================================================================
    // Example 7: Validation
    // ========================================================================
    println!("\n7. Field Validation:");
    println!("   -----------------");

    // Try to create invalid patient (deceased but no death datetime)
    let invalid_patient = ZPI::builder()
        .patient_id("MRN-999")
        .patient_name((
            "Test".to_string(),
            "Patient".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ))
        .birth_date(birth_date)
        // birth_time omitted - defaults to None
        .gender("M")
        // Optional fields omitted - all default to None
        .is_deceased(true) // Deceased = true
        // death_datetime omitted - will be None, should fail validation
        .build();

    match invalid_patient {
        Err(e) => println!("   ✓ Validation correctly failed: {}", e),
        Ok(_) => println!("   ✗ Validation should have failed!"),
    }

    // ========================================================================
    // Example 8: Field Type Summary
    // ========================================================================
    println!("\n8. Field Types Used in This Example:");
    println!("   ----------------------------------");
    println!("   ✓ String - Patient IDs, names, codes");
    println!("   ✓ u32 - Days, scores, counts");
    println!("   ✓ i32 - (Available for negative values)");
    println!("   ✓ i64 - Large dollar amounts (deductible, OOP)");
    println!("   ✓ f64 - Precise dollar amounts (charges, copay)");
    println!("   ✓ bool - Flags (deceased, readmission, pre-auth)");
    println!("   ✓ NaiveDate - Birth dates, coverage dates");
    println!("   ✓ NaiveTime - Birth time");
    println!("   ✓ NaiveDateTime - Admission, onset, verification times");
    println!("   ✓ DateTime<Utc> - (Available for UTC timestamps)");
    println!("   ✓ Option<T> - All optional fields");
    println!("   ✓ Vec<T> - Policy numbers, reactions, auth numbers");
    println!("   ✓ (String, String, ...) - Names, locations");
    println!("   ✓ Option<Tuple> - Optional name components, verifier");
    println!("   ✓ Vec<Tuple> - Insurance plans, doctors, allergies");

    println!("\n=== Example completed successfully! ===");
    println!("\nThis example demonstrates how vyna-hl7-custom provides:");
    println!("  • Complete type safety for all HL7 field patterns");
    println!("  • Compile-time validation of field structure");
    println!("  • Runtime business rule validation");
    println!("  • Zero-cost abstractions over raw HL7 encoding");
    println!("  • Ergonomic API for building complex medical records");

    Ok(())
}
