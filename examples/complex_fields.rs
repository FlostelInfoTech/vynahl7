//! Example demonstrating complex field builders for HL7 composite data types

use vyna_hl7_core::builders::fields::{CxBuilder, XadBuilder, XcnBuilder, XpnBuilder, XtnBuilder};

fn main() {
    println!("=== HL7 Complex Field Builders Example ===\n");

    // XPN - Extended Person Name
    println!("--- XPN (Extended Person Name) Examples ---");

    let simple_name = XpnBuilder::new()
        .family_name("DOE")
        .given_name("JOHN")
        .build();
    println!("Simple name: {}", simple_name);

    let full_name = XpnBuilder::new()
        .family_name("SMITH")
        .given_name("ROBERT")
        .middle_name("JAMES")
        .suffix("JR")
        .prefix("DR")
        .degree("MD")
        .name_type_code("L")
        .build();
    println!("Full name with credentials: {}", full_name);

    let married_name = XpnBuilder::new()
        .family_name("JOHNSON")
        .given_name("MARY")
        .middle_name("ELIZABETH")
        .name_type_code("M")
        .build();
    println!("Married name: {}\n", married_name);

    // XAD - Extended Address
    println!("--- XAD (Extended Address) Examples ---");

    let home_address = XadBuilder::new()
        .street_address("123 Main Street")
        .city("Springfield")
        .state("IL")
        .postal_code("62701")
        .country("USA")
        .address_type("H")
        .build();
    println!("Home address: {}", home_address);

    let work_address = XadBuilder::new()
        .street_address("456 Corporate Blvd")
        .other_designation("Suite 100")
        .city("Chicago")
        .state("IL")
        .postal_code("60601")
        .country("USA")
        .address_type("O")
        .build();
    println!("Work address: {}", work_address);

    let international_address = XadBuilder::new()
        .street_address("10 Downing Street")
        .city("London")
        .postal_code("SW1A 2AA")
        .country("GBR")
        .address_type("H")
        .build();
    println!("International address: {}\n", international_address);

    // XTN - Extended Telecommunication Number
    println!("--- XTN (Extended Telecommunication Number) Examples ---");

    let home_phone = XtnBuilder::new()
        .phone_number("(555) 123-4567")
        .use_code("PRN")
        .equipment_type("PH")
        .build();
    println!("Home phone: {}", home_phone);

    let work_phone = XtnBuilder::new()
        .phone_number("(555) 987-6543")
        .use_code("WPN")
        .equipment_type("PH")
        .extension("1234")
        .build();
    println!("Work phone with extension: {}", work_phone);

    let cell_phone = XtnBuilder::new()
        .phone_number("(555) 555-5555")
        .use_code("PRN")
        .equipment_type("CP")
        .build();
    println!("Cell phone: {}", cell_phone);

    let fax = XtnBuilder::new()
        .phone_number("(555) 999-8888")
        .use_code("WPN")
        .equipment_type("FX")
        .build();
    println!("Fax: {}", fax);

    let email = XtnBuilder::new()
        .email("john.doe@example.com")
        .use_code("NET")
        .equipment_type("Internet")
        .build();
    println!("Email: {}\n", email);

    // CX - Extended Composite ID with Check Digit
    println!("--- CX (Extended Composite ID) Examples ---");

    let medical_record = CxBuilder::new("MRN123456")
        .identifier_type_code("MR")
        .assigning_authority("HospitalA")
        .build();
    println!("Medical Record Number: {}", medical_record);

    let patient_id = CxBuilder::new("PID789012")
        .identifier_type_code("PI")
        .assigning_authority("ClinicB")
        .assigning_facility("Building2")
        .build();
    println!("Patient ID: {}", patient_id);

    let ssn = CxBuilder::new("123-45-6789")
        .identifier_type_code("SS")
        .assigning_authority("SSA")
        .build();
    println!("Social Security Number: {}", ssn);

    let drivers_license = CxBuilder::new("D1234567")
        .check_digit("8")
        .check_digit_scheme("M10")
        .identifier_type_code("DL")
        .assigning_authority("IL DMV")
        .build();
    println!("Driver's License: {}\n", drivers_license);

    // XCN - Extended Composite ID Number and Name for Persons
    println!("--- XCN (Extended Composite Name) Examples ---");

    let attending_doctor = XcnBuilder::new()
        .id_number("1234567890")
        .family_name("SMITH")
        .given_name("JAMES")
        .prefix("DR")
        .degree("MD")
        .identifier_type_code("NPI")
        .build();
    println!("Attending Doctor: {}", attending_doctor);

    let referring_physician = XcnBuilder::new()
        .id_number("0987654321")
        .family_name("JOHNSON")
        .given_name("EMILY")
        .middle_name("ROSE")
        .prefix("DR")
        .degree("DO")
        .identifier_type_code("NPI")
        .assigning_authority("AMA")
        .build();
    println!("Referring Physician: {}", referring_physician);

    let nurse = XcnBuilder::new()
        .id_number("NRS5555")
        .family_name("WILLIAMS")
        .given_name("SARAH")
        .degree("RN")
        .identifier_type_code("EI")
        .build();
    println!("Nurse: {}\n", nurse);

    println!("✓ All complex field builders demonstrated successfully!");
}
