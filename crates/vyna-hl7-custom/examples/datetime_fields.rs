//! Example demonstrating DateTime field types in custom Z-segments
//!
//! This example shows how to use chrono date/time types:
//! - NaiveDateTime (timestamp without timezone)
//! - NaiveDate (date only)
//! - NaiveTime (time only)
//! - DateTime<Utc> (timezone-aware timestamp)
//!
//! Run with: cargo run --example datetime_fields

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use vyna_hl7_core::Delimiters;
use vyna_hl7_custom::{CustomSegment, MessageExt, z_segment};
use vyna_hl7_parser::parse_message;

// Define a Z-segment with various date/time field types
z_segment! {
    ZDT,
    id = "ZDT",
    fields = {
        1 => patient_id: String,
        2 => admission_datetime: NaiveDateTime,        // Full timestamp
        3 => birth_date: NaiveDate,                    // Date only
        4 => appointment_time: NaiveTime,              // Time only
        5 => last_updated_utc: DateTime<Utc>,          // UTC timestamp
        6 => discharge_datetime: Option<NaiveDateTime>, // Optional timestamp
        7 => followup_date: Option<NaiveDate>,         // Optional date
        8 => surgery_time: Option<NaiveTime>,          // Optional time
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DateTime Field Types Example ===\n");

    // Example 1: Create a Z-segment with date/time fields
    println!("1. Creating ZDT segment with date/time fields:");
    println!("   ------------------------------------------");

    let admission = NaiveDate::from_ymd_opt(2025, 1, 19)
        .unwrap()
        .and_hms_opt(14, 30, 0)
        .unwrap();

    let birth_date = NaiveDate::from_ymd_opt(1980, 6, 15).unwrap();
    let appt_time = NaiveTime::from_hms_opt(10, 30, 0).unwrap();
    let last_updated = Utc.with_ymd_and_hms(2025, 1, 19, 14, 30, 0).unwrap();

    let zdt = ZDT::builder()
        .patient_id("PAT-12345")
        .admission_datetime(admission)
        .birth_date(birth_date)
        .appointment_time(appt_time)
        .last_updated_utc(last_updated)
        // Optional fields can be omitted or set
        .discharge_datetime(
            NaiveDate::from_ymd_opt(2025, 1, 22)
                .unwrap()
                .and_hms_opt(11, 0, 0)
                .unwrap(),
        )
        .followup_date(NaiveDate::from_ymd_opt(2025, 2, 1).unwrap())
        .surgery_time(NaiveTime::from_hms_opt(8, 0, 0).unwrap())
        .build()?;

    println!("   Patient ID: {}", zdt.patient_id);
    println!(
        "   Admission: {}",
        zdt.admission_datetime.format("%Y-%m-%d %H:%M:%S")
    );
    println!("   Birth Date: {}", zdt.birth_date.format("%Y-%m-%d"));
    println!(
        "   Appointment Time: {}",
        zdt.appointment_time.format("%H:%M:%S")
    );
    println!(
        "   Last Updated (UTC): {}",
        zdt.last_updated_utc.format("%Y-%m-%d %H:%M:%S %Z")
    );
    println!(
        "   Discharge: {:?}",
        zdt.discharge_datetime
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
    );
    println!(
        "   Follow-up: {:?}",
        zdt.followup_date.map(|d| d.format("%Y-%m-%d").to_string())
    );
    println!(
        "   Surgery Time: {:?}",
        zdt.surgery_time.map(|t| t.format("%H:%M:%S").to_string())
    );

    // Example 2: HL7 encoding of date/time fields
    println!("\n2. HL7 encoding of date/time fields:");
    println!("   ------------------------------------------");

    let delimiters = Delimiters::default();
    let segment = zdt.to_segment();
    let encoded = segment.encode(&delimiters);
    println!("   {}", encoded);

    println!("\n   HL7 Date/Time Formats:");
    println!("   - DateTime → YYYYMMDDHHMMSS (e.g., 20250119143000)");
    println!("   - Date     → YYYYMMDD (e.g., 20250119)");
    println!("   - Time     → HHMMSS (e.g., 143000)");

    // Example 3: Parsing HL7 messages with date/time fields
    println!("\n3. Parsing HL7 messages with date/time fields:");
    println!("   ------------------------------------------");

    let hl7_message = format!(
        "MSH|^~\\&|SendApp|SendFac|RecvApp|RecvFac|20250119120000||ADT^A01|MSG001|P|2.5\r\
         PID|1||PAT-67890||Smith^Jane||19900515|F\r\
         {}\r",
        "ZDT|PAT-67890|20250120093000|19900515|133000|20250120093500|20250125120000|20250210|073000"
    );

    let message = parse_message(&hl7_message)?;

    if let Some(parsed_zdt) = message.get_custom_segment::<ZDT>()? {
        println!("   Successfully parsed ZDT segment:");
        println!("   - Patient: {}", parsed_zdt.patient_id);
        println!(
            "   - Admission: {}",
            parsed_zdt.admission_datetime.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "   - Birth Date: {}",
            parsed_zdt.birth_date.format("%B %d, %Y")
        );
        println!(
            "   - Appointment: {}",
            parsed_zdt.appointment_time.format("%I:%M %p")
        );
        println!(
            "   - Discharge: {}",
            parsed_zdt
                .discharge_datetime
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or("Not set".to_string())
        );
    }

    // Example 4: Different date formats supported by HL7
    println!("\n4. HL7 date format flexibility:");
    println!("   ------------------------------------------");

    let date_examples = vec![
        ("20250119", "YYYYMMDD - Full date"),
        ("202501", "YYYYMM - Month only (defaults to 1st)"),
        ("2025", "YYYY - Year only (defaults to Jan 1st)"),
    ];

    for (value, desc) in date_examples {
        let test_msg = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
             ZDT|PAT001|20250119120000|{}|120000|20250119120000|||",
            value
        );

        if let Ok(msg) = parse_message(&test_msg)
            && let Ok(Some(seg)) = msg.get_custom_segment::<ZDT>()
        {
            println!(
                "   '{}' - {} → {}",
                value,
                desc,
                seg.birth_date.format("%Y-%m-%d")
            );
        }
    }

    // Example 5: Time format flexibility
    println!("\n5. HL7 time format flexibility:");
    println!("   ------------------------------------------");

    let time_examples = vec![
        ("143000", "HHMMSS - Full time with seconds"),
        ("1430", "HHMM - Hours and minutes only"),
    ];

    for (value, desc) in time_examples {
        let test_msg = format!(
            "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
             ZDT|PAT001|20250119120000|20250119|{}|20250119120000|||",
            value
        );

        if let Ok(msg) = parse_message(&test_msg)
            && let Ok(Some(seg)) = msg.get_custom_segment::<ZDT>()
        {
            println!(
                "   '{}' - {} → {}",
                value,
                desc,
                seg.appointment_time.format("%H:%M:%S")
            );
        }
    }

    // Example 6: Working with optional date/time fields
    println!("\n6. Optional date/time fields:");
    println!("   ------------------------------------------");

    let minimal_zdt = ZDT::builder()
        .patient_id("PAT-99999")
        .admission_datetime(
            NaiveDate::from_ymd_opt(2025, 1, 20)
                .unwrap()
                .and_hms_opt(9, 0, 0)
                .unwrap(),
        )
        .birth_date(NaiveDate::from_ymd_opt(1975, 3, 10).unwrap())
        .appointment_time(NaiveTime::from_hms_opt(14, 0, 0).unwrap())
        .last_updated_utc(Utc::now())
        // Optional fields omitted
        .build()?;

    println!("   Created minimal segment (required fields only):");
    println!("   - Patient ID: {}", minimal_zdt.patient_id);
    println!(
        "   - Admission: {}",
        minimal_zdt.admission_datetime.format("%Y-%m-%d %H:%M:%S")
    );
    println!("   - Discharge: {:?}", minimal_zdt.discharge_datetime);
    println!("   - Follow-up: {:?}", minimal_zdt.followup_date);
    println!("   - Surgery Time: {:?}", minimal_zdt.surgery_time);

    let minimal_encoded = minimal_zdt.to_segment().encode(&delimiters);
    println!("\n   Encoded: {}", minimal_encoded);
    println!("   (Note: Optional fields appear as empty in HL7 output)");

    // Example 7: Date arithmetic and manipulation
    println!("\n7. Date/time manipulation:");
    println!("   ------------------------------------------");

    let mut msg = parse_message(
        "MSH|^~\\&|App|Fac|App|Fac|20250119120000||ADT^A01|MSG|P|2.5\r\
         PID|1||PAT-11111||Doe^John\r\
         ZDT|PAT-11111|20250120100000|19850615|140000|20250120100000|||",
    )?;

    println!("   Original admission time:");
    if let Some(original) = msg.get_custom_segment::<ZDT>()? {
        println!(
            "   {}",
            original.admission_datetime.format("%Y-%m-%d %H:%M:%S")
        );
    }

    // Modify the segment - add 3 days to admission for discharge
    let mut modified_zdt = msg.get_custom_segment::<ZDT>()?.unwrap();
    let discharge = modified_zdt.admission_datetime + chrono::Duration::days(3);
    modified_zdt.discharge_datetime = Some(discharge);

    // Set follow-up 2 weeks after discharge
    modified_zdt.followup_date = Some(discharge.date() + chrono::Duration::weeks(2));

    msg.set_custom_segment(modified_zdt)?;

    println!("\n   After updating with calculated dates:");
    if let Some(updated) = msg.get_custom_segment::<ZDT>()? {
        println!(
            "   - Admission: {}",
            updated.admission_datetime.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "   - Discharge: {}",
            updated
                .discharge_datetime
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or("Not set".to_string())
        );
        println!(
            "   - Follow-up: {}",
            updated
                .followup_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or("Not set".to_string())
        );
    }

    // Example 8: UTC timestamps for distributed systems
    println!("\n8. UTC timestamps for distributed systems:");
    println!("   ------------------------------------------");

    let utc_now = Utc::now();
    println!(
        "   Current UTC time: {}",
        utc_now.format("%Y-%m-%d %H:%M:%S %Z")
    );
    println!("   HL7 format: {}", utc_now.format("%Y%m%d%H%M%S"));

    println!("\n   Benefits of DateTime<Utc>:");
    println!("   - Consistent across timezones");
    println!("   - No ambiguity during DST transitions");
    println!("   - Ideal for logging and audit trails");
    println!("   - Easy to convert to local time when needed");

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
