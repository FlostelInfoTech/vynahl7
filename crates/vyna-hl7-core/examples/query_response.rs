//! Example of creating QBP queries and RSP responses
//!
//! This example demonstrates:
//! - Building QBP^Q11 (Immunization History Query)
//! - Building QBP^Q22 (Find Candidates Query)
//! - Building RSP^K11 (Immunization Response)
//! - Building RSP^K22 (Find Candidates Response)
//! - Parsing RSP messages to extract query results

use vyna_hl7_core::{
    builders::{
        qbp::{QbpQ11Builder, QbpQ22Builder},
        rsp::{RspK11Builder, RspK22Builder},
    },
    field::Field,
    segment::Segment,
    Version,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("═══════════════════════════════════════════════════════════════");
    println!("  VynaHL7 Query/Response Examples - QBP/RSP Messages");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Example 1: QBP^Q11 - Immunization History Query
    example_1_immunization_query()?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Example 2: RSP^K11 - Immunization Response
    example_2_immunization_response()?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Example 3: QBP^Q22 - Patient Search Query
    example_3_patient_search()?;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Example 4: RSP^K22 - Patient Search Response (with pagination)
    example_4_patient_search_response()?;

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  Examples Complete");
    println!("═══════════════════════════════════════════════════════════════");

    Ok(())
}

fn example_1_immunization_query() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 1: QBP^Q11 - Immunization History Query (Z44 Profile)");
    println!("──────────────────────────────────────────────────────────────\n");

    let query = QbpQ11Builder::new(Version::V2_5_1)
        .sending_application("MyEHR")
        .sending_facility("FAC001")
        .receiving_application("CAIR2")
        .receiving_facility("CAIR2")
        .query_name("Z44^Request Evaluated History and Forecast^CDCPHINVS")
        .query_tag("Q123456789")
        .patient_id("234567^^^MYEHR^MR")
        .patient_name("DOE^JANE^MARIE^^^^L")
        .mothers_name("SMITH^MARY^^^^^M")
        .date_of_birth("20180315")
        .sex("F")
        .address("123 MAIN ST^^SACRAMENTO^CA^95814^USA^H^^CA067")
        .quantity_limit("100^RD")
        .build()?;

    let encoded = query.encode_with_separator("\n");
    println!("Encoded QBP^Q11 Message:\n");
    for line in encoded.lines() {
        println!("  {}", line);
    }

    println!("\n✓ Query created successfully");
    println!("  - Query Tag: Q123456789");
    println!("  - Query Type: Z44 (Immunization History + Forecast)");
    println!("  - Max Records: 100");

    Ok(())
}

fn example_2_immunization_response() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 2: RSP^K11 - Immunization Response with Data");
    println!("──────────────────────────────────────────────────────────────\n");

    // Build PID segment
    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1")); // PID-1: Set ID
    pid.add_field(Field::from_value("")); // PID-2: External ID (deprecated)
    pid.add_field(Field::from_value("234567^^^MYEHR^MR~987654^^^CAIR2^SR")); // PID-3: Patient ID
    pid.add_field(Field::from_value("")); // PID-4: Alternate Patient ID (deprecated)
    pid.add_field(Field::from_value("DOE^JANE^MARIE^^^^L")); // PID-5: Patient Name
    pid.add_field(Field::from_value("SMITH^MARY^^^^^M")); // PID-6: Mother's Maiden Name
    pid.add_field(Field::from_value("20180315")); // PID-7: Date of Birth
    pid.add_field(Field::from_value("F")); // PID-8: Sex

    // Build ORC segment (order control)
    let mut orc = Segment::new("ORC");
    orc.add_field(Field::from_value("RE")); // ORC-1: Order Control (RE = Observations to follow)
    orc.add_field(Field::from_value("")); // ORC-2: Placer Order Number
    orc.add_field(Field::from_value("65432^CAIR2")); // ORC-3: Filler Order Number

    // Build RXA segment (pharmacy/treatment administration)
    let mut rxa = Segment::new("RXA");
    rxa.add_field(Field::from_value("0")); // RXA-1: Give Sub-ID Counter
    rxa.add_field(Field::from_value("1")); // RXA-2: Administration Sub-ID Counter
    rxa.add_field(Field::from_value("20180515")); // RXA-3: Date/Time Start of Administration
    rxa.add_field(Field::from_value("")); // RXA-4: Date/Time End of Administration
    rxa.add_field(Field::from_value("83^Hep A, ped/adol, 2 dose^CVX")); // RXA-5: Administered Code

    // Build the RSP message
    let response = RspK11Builder::new(Version::V2_5_1)
        .sending_application("CAIR2")
        .sending_facility("CAIR2")
        .receiving_application("MyEHR")
        .receiving_facility("FAC001")
        .in_response_to("MSG-20231115-001")
        .query_tag("Q123456789")
        .query_name("Z44^Request Evaluated History and Forecast^CDCPHINVS")
        .query_response_status("OK")
        .hit_count(1)
        .qpd_parameter("234567^^^MYEHR^MR")
        .qpd_parameter("DOE^JANE^MARIE^^^^L")
        .add_segment(pid)
        .add_segment(orc)
        .add_segment(rxa)
        .build()?;

    let encoded = response.encode_with_separator("\n");
    println!("Encoded RSP^K11 Message:\n");
    for line in encoded.lines() {
        println!("  {}", line);
    }

    println!("\n✓ Response created successfully");
    println!("  - Status: OK (Data found)");
    println!("  - Hit Count: 1 immunization record");
    println!("  - Includes: PID, ORC, RXA segments");

    Ok(())
}

fn example_3_patient_search() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 3: QBP^Q22 - Patient Search (Find Candidates)");
    println!("──────────────────────────────────────────────────────────────\n");

    let query = QbpQ22Builder::new(Version::V2_5_1)
        .sending_application("CLINREG")
        .sending_facility("WESTCLIN")
        .receiving_application("HOSPMPI")
        .receiving_facility("HOSP")
        .query_tag("987654321")
        .parameter("@PID.5.1^SMITH") // Family name
        .parameter("@PID.5.2^JOHN") // Given name
        .parameter("@PID.7^19850610") // Date of birth
        .parameter("@PID.8^M") // Sex
        .quantity_limit("50^RD")
        .build()?;

    let encoded = query.encode_with_separator("\n");
    println!("Encoded QBP^Q22 Message:\n");
    for line in encoded.lines() {
        println!("  {}", line);
    }

    println!("\n✓ Query created successfully");
    println!("  - Query Tag: 987654321");
    println!("  - Search Criteria:");
    println!("    • Family Name: SMITH");
    println!("    • Given Name: JOHN");
    println!("    • DOB: 1985-06-10");
    println!("    • Sex: M");
    println!("  - Max Records: 50");

    Ok(())
}

fn example_4_patient_search_response() -> Result<(), Box<dyn std::error::Error>> {
    println!("Example 4: RSP^K22 - Patient Search Response (with Pagination)");
    println!("──────────────────────────────────────────────────────────────\n");

    // Build first matching patient
    let mut pid1 = Segment::new("PID");
    pid1.add_field(Field::from_value("1")); // Set ID
    pid1.add_field(Field::from_value("")); // External ID
    pid1.add_field(Field::from_value("1001^^^MPI^MR")); // Patient ID
    pid1.add_field(Field::from_value("")); // Alternate ID
    pid1.add_field(Field::from_value("SMITH^JOHN^A")); // Name
    pid1.add_field(Field::from_value("")); // Mother's Maiden
    pid1.add_field(Field::from_value("19850610")); // DOB
    pid1.add_field(Field::from_value("M")); // Sex

    // Build second matching patient
    let mut pid2 = Segment::new("PID");
    pid2.add_field(Field::from_value("2"));
    pid2.add_field(Field::from_value(""));
    pid2.add_field(Field::from_value("1002^^^MPI^MR"));
    pid2.add_field(Field::from_value(""));
    pid2.add_field(Field::from_value("SMITH^JOHN^B"));
    pid2.add_field(Field::from_value(""));
    pid2.add_field(Field::from_value("19850610"));
    pid2.add_field(Field::from_value("M"));

    let response = RspK22Builder::new(Version::V2_5_1)
        .sending_application("HOSPMPI")
        .sending_facility("HOSP")
        .receiving_application("CLINREG")
        .receiving_facility("WESTCLIN")
        .in_response_to("Q-20231115-045")
        .query_tag("987654321")
        .query_name("Q22^Find Candidates^HL7")
        .query_response_status("OK")
        .hit_counts(247, 2, 245) // 247 total, 2 in this message, 245 remaining
        .qpd_parameter("@PID.5.1^SMITH")
        .qpd_parameter("@PID.5.2^JOHN")
        .add_segment(pid1)
        .add_segment(pid2)
        .build()?;

    let encoded = response.encode_with_separator("\n");
    println!("Encoded RSP^K22 Message:\n");
    for line in encoded.lines() {
        println!("  {}", line);
    }

    println!("\n✓ Response created successfully");
    println!("  - Status: OK (Data found)");
    println!("  - Total Matches: 247 patients");
    println!("  - In This Response: 2 patients");
    println!("  - Remaining: 245 patients");
    println!("  - ⚠️  More data available (pagination required)");
    println!("\nTo retrieve next page:");
    println!("  - Include DSC segment with continuation pointer in next query");

    Ok(())
}
