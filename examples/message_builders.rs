//! Example demonstrating HL7 message builders

use vyna_hl7_core::Version;
use vyna_hl7_core::builders::adt::AdtBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== HL7 Message Builders Example ===\n");

    // Create ADT^A01 message
    println!("Creating ADT^A01 (Admit/Visit Notification)...");
    let message = AdtBuilder::a01(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800101")
        .sex("M")
        .patient_class("I")
        .assigned_location("ER^101^1")
        .attending_doctor("SMITH^JAMES")
        .build()?;

    println!("{}\n", message.encode());

    // Create ADT^A08 message
    println!("Creating ADT^A08 (Update Patient Information)...");
    let update_message = AdtBuilder::a08(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800101")
        .sex("M")
        .build()?;

    println!("{}\n", update_message.encode());

    // Create ADT^A05 message
    println!("Creating ADT^A05 (Pre-admit a Patient)...");
    let a05_message = AdtBuilder::a05(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800101")
        .sex("M")
        .patient_class("P")
        .build()?;

    println!("{}\n", a05_message.encode());

    // Create ADT^A06 message
    println!("Creating ADT^A06 (Change Outpatient to Inpatient)...");
    let a06_message = AdtBuilder::a06(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .patient_class("I")
        .assigned_location("Ward^201^2")
        .build()?;

    println!("{}\n", a06_message.encode());

    // Create ADT^A07 message
    println!("Creating ADT^A07 (Change Inpatient to Outpatient)...");
    let a07_message = AdtBuilder::a07(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .patient_class("O")
        .build()?;

    println!("{}\n", a07_message.encode());

    // Create ADT^A11 message
    println!("Creating ADT^A11 (Cancel Admit/Visit Notification)...");
    let a11_message = AdtBuilder::a11(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    println!("{}\n", a11_message.encode());

    // Create ADT^A12 message
    println!("Creating ADT^A12 (Cancel Transfer)...");
    let a12_message = AdtBuilder::a12(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    println!("{}\n", a12_message.encode());

    // Create ADT^A13 message
    println!("Creating ADT^A13 (Cancel Discharge/End Visit)...");
    let a13_message = AdtBuilder::a13(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    println!("{}\n", a13_message.encode());

    // Create ADT^A02 message
    println!("Creating ADT^A02 (Transfer a Patient)...");
    let a02_message = AdtBuilder::a02(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .patient_class("I")
        .assigned_location("ICU^301^3")
        .build()?;

    println!("{}\n", a02_message.encode());

    // Create ADT^A03 message
    println!("Creating ADT^A03 (Discharge/End Visit)...");
    let a03_message = AdtBuilder::a03(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .patient_class("I")
        .build()?;

    println!("{}\n", a03_message.encode());

    // Create ADT^A04 message
    println!("Creating ADT^A04 (Register a Patient)...");
    let a04_message = AdtBuilder::a04(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800101")
        .sex("M")
        .build()?;

    println!("{}\n", a04_message.encode());

    // Create ADT^A09 message
    println!("Creating ADT^A09 (Patient Departing - Tracking)...");
    let a09_message = AdtBuilder::a09(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .patient_class("I")
        .assigned_location("Ward^201^2")
        .build()?;

    println!("{}\n", a09_message.encode());

    // Create ADT^A10 message
    println!("Creating ADT^A10 (Patient Arriving - Tracking)...");
    let a10_message = AdtBuilder::a10(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .patient_class("I")
        .assigned_location("Ward^201^2")
        .build()?;

    println!("{}\n", a10_message.encode());

    // Create ADT^A17 message
    println!("Creating ADT^A17 (Swap Patients)...");
    let a17_message = AdtBuilder::a17(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    println!("{}\n", a17_message.encode());

    // Create ADT^A28 message
    println!("Creating ADT^A28 (Add Person Information)...");
    let a28_message = AdtBuilder::a28(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800101")
        .sex("M")
        .build()?;

    println!("{}\n", a28_message.encode());

    // Create ADT^A31 message
    println!("Creating ADT^A31 (Update Person Information)...");
    let a31_message = AdtBuilder::a31(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .date_of_birth("19800101")
        .sex("M")
        .build()?;

    println!("{}\n", a31_message.encode());

    // Create ADT^A40 message
    println!("Creating ADT^A40 (Merge Patient - Patient Identifier List)...");
    let a40_message = AdtBuilder::a40(Version::V2_5)
        .sending_application("MyApp")
        .sending_facility("MyFacility")
        .receiving_application("RecApp")
        .receiving_facility("RecFacility")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .build()?;

    println!("{}\n", a40_message.encode());

    // Create ORU^R01 message
    println!("Creating ORU^R01 (Observation Result)...");
    use vyna_hl7_core::builders::oru::{Observation, OruR01Builder};

    let oru_message = OruR01Builder::new(Version::V2_5)
        .sending_application("LabApp")
        .sending_facility("LabFacility")
        .receiving_application("EMR")
        .receiving_facility("Hospital")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .filler_order_number("LAB123456")
        .add_observation(Observation {
            set_id: 1,
            value_type: "NM".to_string(),
            identifier: "GLUCOSE".to_string(),
            value: "95".to_string(),
            units: Some("mg/dL".to_string()),
            status: "F".to_string(),
        })
        .add_observation(Observation {
            set_id: 2,
            value_type: "NM".to_string(),
            identifier: "CHOL".to_string(),
            value: "180".to_string(),
            units: Some("mg/dL".to_string()),
            status: "F".to_string(),
        })
        .build()?;

    println!("{}\n", oru_message.encode());

    // Create ORM^O01 message
    println!("Creating ORM^O01 (Order Message)...");
    use vyna_hl7_core::builders::orm::OrmO01Builder;

    let orm_message = OrmO01Builder::new(Version::V2_5)
        .sending_application("OrderApp")
        .sending_facility("Clinic")
        .receiving_application("LabApp")
        .receiving_facility("Lab")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .placer_order_number("ORD123456")
        .order_control("NW")
        .universal_service_id("CBC")
        .build()?;

    println!("{}\n", orm_message.encode());

    // Create QRY^A19 message
    println!("Creating QRY^A19 (Patient Query)...");
    use vyna_hl7_core::builders::qry::QryA19Builder;

    let qry_message = QryA19Builder::new(Version::V2_5)
        .sending_application("QueryApp")
        .sending_facility("Hospital")
        .receiving_application("MPI")
        .receiving_facility("Registry")
        .patient_id("12345")
        .build()?;

    println!("{}\n", qry_message.encode());

    // Create OUL^R21 message
    println!("Creating OUL^R21 (Unsolicited Laboratory Observation)...");
    use vyna_hl7_core::builders::laboratory::OulR21Builder;

    let oul_message = OulR21Builder::new(Version::V2_5)
        .sending_application("LabApp")
        .sending_facility("Laboratory")
        .receiving_application("EMR")
        .receiving_facility("Hospital")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .observation_id("GLU^Glucose^LN")
        .observation_value("95")
        .build()?;

    println!("{}\n", oul_message.encode());

    // Create OML^O21 message
    println!("Creating OML^O21 (Laboratory Order)...");
    use vyna_hl7_core::builders::laboratory::OmlO21Builder;

    let oml_message = OmlO21Builder::new(Version::V2_5)
        .sending_application("OrderApp")
        .sending_facility("Clinic")
        .receiving_application("LabApp")
        .receiving_facility("Laboratory")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .order_control("NW")
        .placer_order_number("LAB-ORD-001")
        .universal_service_id("CBC^Complete Blood Count^LN")
        .build()?;

    println!("{}\n", oml_message.encode());

    // Create RDE^O11 message
    println!("Creating RDE^O11 (Pharmacy/Treatment Encoded Order)...");
    use vyna_hl7_core::builders::pharmacy::RdeO11Builder;

    let rde_message = RdeO11Builder::new(Version::V2_5)
        .sending_application("PharmacyApp")
        .sending_facility("Hospital")
        .receiving_application("PharmacySystem")
        .receiving_facility("Pharmacy")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .order_control("NW")
        .placer_order_number("RX-001")
        .give_code("08^HEPB^CVX")
        .give_amount_minimum("1")
        .give_units("TAB^Tablet^L")
        .route("PO")
        .build()?;

    println!("{}\n", rde_message.encode());

    // Create RAS^O17 message
    println!("Creating RAS^O17 (Pharmacy/Treatment Administration)...");
    use vyna_hl7_core::builders::pharmacy::RasO17Builder;

    let ras_message = RasO17Builder::new(Version::V2_5)
        .sending_application("PharmacyApp")
        .sending_facility("Hospital")
        .receiving_application("EMR")
        .receiving_facility("Hospital")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .order_control("RE")
        .placer_order_number("RX-001")
        .administered_code("08^HEPB^CVX")
        .amount("0.5")
        .route("IM")
        .build()?;

    println!("{}\n", ras_message.encode());

    // Create RDS^O13 message
    println!("Creating RDS^O13 (Pharmacy/Treatment Dispense)...");
    use vyna_hl7_core::builders::pharmacy::RdsO13Builder;

    let rds_message = RdsO13Builder::new(Version::V2_5)
        .sending_application("PharmacySystem")
        .sending_facility("Pharmacy")
        .receiving_application("EMR")
        .receiving_facility("Hospital")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .order_control("RE")
        .placer_order_number("RX-001")
        .dispense_code("08^HEPB^CVX")
        .dispense_amount("1")
        .prescription_number("RX12345")
        .route("PO")
        .build()?;

    println!("{}\n", rds_message.encode());

    // Create RGV^O15 message
    println!("Creating RGV^O15 (Pharmacy/Treatment Give)...");
    use vyna_hl7_core::builders::pharmacy::RgvO15Builder;

    let rgv_message = RgvO15Builder::new(Version::V2_5)
        .sending_application("PharmacyApp")
        .sending_facility("Hospital")
        .receiving_application("EMR")
        .receiving_facility("Hospital")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .order_control("RE")
        .placer_order_number("RX-001")
        .give_code("08^HEPB^CVX")
        .give_amount_minimum("1")
        .give_units("TAB^Tablet^L")
        .route("PO")
        .build()?;

    println!("{}\n", rgv_message.encode());

    // Create RRA^O18 message
    println!("Creating RRA^O18 (Pharmacy/Treatment Administration Acknowledgment)...");
    use vyna_hl7_core::builders::pharmacy::RraO18Builder;

    let rra_message = RraO18Builder::new(Version::V2_5)
        .sending_application("PharmacyApp")
        .sending_facility("Hospital")
        .receiving_application("OrderingSystem")
        .receiving_facility("Hospital")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .ack_code("AA")
        .acknowledged_control_id("RX-001")
        .build()?;

    println!("{}\n", rra_message.encode());

    // Create RRD^O14 message
    println!("Creating RRD^O14 (Pharmacy/Treatment Dispense Information)...");
    use vyna_hl7_core::builders::pharmacy::RrdO14Builder;

    let rrd_message = RrdO14Builder::new(Version::V2_5)
        .sending_application("PharmacySystem")
        .sending_facility("Pharmacy")
        .receiving_application("EMR")
        .receiving_facility("Hospital")
        .patient_id("12345")
        .patient_name("DOE", "JOHN")
        .ack_code("AA")
        .acknowledged_control_id("RX-001")
        .build()?;

    println!("{}\n", rrd_message.encode());

    println!("✓ All messages built successfully!");

    Ok(())
}
