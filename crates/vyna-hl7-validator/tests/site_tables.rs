//! Site-local code tables: strict validation stays strict while feeds that
//! extend the standard tables (e.g. NDC drug codes where HL7 v2.5 tables
//! RXE-2 to 0292/CVX) register their codes explicitly.

use vyna_hl7_core::builders::pharmacy::RdeO11Builder;
use vyna_hl7_core::Version;
use vyna_hl7_validator::Validator;

fn ndc_coded_order() -> vyna_hl7_core::Message {
    RdeO11Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("RX1")
        .order_control("NW")
        .give_code("00093-0058^Lorazepam^NDC")
        .give_amount_minimum("1")
        .give_units("TAB^Tablet^L")
        .route("PO")
        .build()
        .unwrap()
}

#[test]
fn strict_rejects_uncoded_site_values() {
    let message = ndc_coded_order();
    let validator = Validator::for_message_type(Version::V2_5, "RDE", "O11").unwrap();
    let result = validator.validate(&message);
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.location.starts_with("RXE") && e.message.contains("0292")),
        "expected an RXE-2 table 0292 error, got: {:?}",
        result.errors
    );
}

#[test]
fn site_extension_accepts_site_values_and_keeps_standard_ones() {
    let message = ndc_coded_order();
    let mut validator = Validator::for_message_type(Version::V2_5, "RDE", "O11").unwrap();
    validator
        .table_registry_mut()
        .get_table_mut("0292")
        .unwrap()
        .add_value("00093-0058", "Lorazepam 1mg tablet (site NDC)", false);

    let result = validator.validate(&message);
    assert!(
        result.errors.is_empty(),
        "extended table should accept the NDC code, got: {:?}",
        result.errors
    );

    // The standard CVX set is untouched by the extension.
    let registry = validator.table_registry();
    assert!(registry.validate("0292", "08").is_valid());
    assert!(registry.validate("0292", "00093-0058").is_valid());
    assert!(!registry.validate("0292", "NOT-A-CODE").is_valid());
}
