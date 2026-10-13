//! Builder conformance suite: every message builder that can produce a
//! schema-valid message must do so, at HL7 v2.5, both as built and after a
//! strict parse round-trip.
//!
//! Out of scope (no message schema to conform to): QBP/RSP builders (no QBP,
//! RSP, or batch schemas exist), batch/file builders (not messages), and the
//! field builders in `builders::fields` (not messages).

use vyna_hl7_core::{Message, Version};
use vyna_hl7_validator::Validator;

/// A builder conforms when its output validates clean as built and after a
/// strict parse round-trip.
fn assert_conformant(message: &Message, message_type: &str, trigger_event: &str) {
    let validator =
        Validator::for_message_type(Version::V2_5, message_type, trigger_event).unwrap();
    let built = validator.validate(message);
    assert!(
        built.errors.is_empty(),
        "{}^{} built message errors: {:?}",
        message_type,
        trigger_event,
        built.errors
    );

    let parsed = vyna_hl7_parser::parse_message(&message.encode()).unwrap();
    let reparsed = validator.validate(&parsed);
    assert!(
        reparsed.errors.is_empty(),
        "{}^{} parsed message errors: {:?}",
        message_type,
        trigger_event,
        reparsed.errors
    );
}

#[test]
fn adt_a01_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a01(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .date_of_birth("19800101")
        .sex("M")
        .patient_class("I")
        .assigned_location("3N^301^B")
        .attending_doctor("W10001^Smith^John")
        .admit_datetime("20240115120000")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A01");
}

#[test]
fn adt_a02_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a02(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .assigned_location("3N^301^B")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A02");
}

#[test]
fn adt_a03_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a03(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A03");
}

#[test]
fn adt_a05_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a05(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A05");
}

#[test]
fn adt_a06_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a06(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("O")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A06");
}

#[test]
fn adt_a07_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a07(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("O")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A07");
}

#[test]
fn adt_a09_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a09(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A09");
}

#[test]
fn adt_a10_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a10(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A10");
}

#[test]
fn adt_a34_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a34(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("NEW123")
        .patient_name("Smith", "John")
        .prior_patient_id("OLD456")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A34");
}

#[test]
fn adt_a40_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a40(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("NEW123")
        .patient_name("Smith", "John")
        .patient_class("I")
        .add_prior_patient_id("OLD456")
        .add_prior_patient_id("OLD789")
        .add_prior_patient_name("Smyth", "Jon")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A40");
}

#[test]
fn orm_o01_conforms() {
    use vyna_hl7_core::builders::orm::OrmO01Builder;

    // Short service id: the builder stores it as one unsplit component, so
    // the whole string counts against the 20-char component limit.
    let message = OrmO01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("ORD1")
        .order_control("NW")
        .universal_service_id("CHEM^Chem^L")
        .build()
        .unwrap();

    assert_conformant(&message, "ORM", "O01");
}

#[test]
fn oml_o21_conforms() {
    use vyna_hl7_core::builders::laboratory::OmlO21Builder;

    let message = OmlO21Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("ORD1")
        .order_control("NW")
        .universal_service_id("CHEM^Chem^L")
        .build()
        .unwrap();

    assert_conformant(&message, "OML", "O21");
}

#[test]
fn omp_o09_conforms() {
    use vyna_hl7_core::builders::omp::{MedicationOrder, OmpO09Builder};

    let message = OmpO09Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("O")
        .add_order(MedicationOrder::new("00093-0058^Loraz^NDC", "PO").amount_minimum("1"))
        .build()
        .unwrap();

    assert_conformant(&message, "OMP", "O09");
}

#[test]
fn ppr_pc1_conforms() {
    use vyna_hl7_core::builders::ppr::{PprPc1Builder, Problem};

    let message = PprPc1Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("O")
        .add_problem(
            Problem::new("38341003^HTN^SCT", "PROB-1", "20240101100000").note("Stage 1, monitor"),
        )
        .build()
        .unwrap();

    assert_conformant(&message, "PPR", "PC1");
}

#[test]
fn vxu_v04_conforms() {
    use vyna_hl7_core::builders::vxu::{ImmunizationDose, VxuV04Builder};

    let message = VxuV04Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .add_immunization(
            ImmunizationDose::new("08^HEPB^CVX", "20240115130000")
                .amount("0.5")
                .units("mL"),
        )
        .build()
        .unwrap();

    assert_conformant(&message, "VXU", "V04");
}

#[test]
fn oul_r21_conforms() {
    use vyna_hl7_core::builders::laboratory::OulR21Builder;

    let message = OulR21Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .observation_id("845-9^MCV^LN")
        .observation_value("92")
        .result_status("F")
        .build()
        .unwrap();

    assert_conformant(&message, "OUL", "R21");
}

#[test]
fn adt_a04_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a04(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("O")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A04");
}

#[test]
fn adt_a08_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a08(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A08");
}

#[test]
fn adt_a11_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a11(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A11");
}

#[test]
fn adt_a12_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a12(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A12");
}

#[test]
fn adt_a13_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a13(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A13");
}

#[test]
fn adt_a17_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a17(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A17");
}

#[test]
fn adt_a28_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a28(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A28");
}

#[test]
fn adt_a31_conforms() {
    use vyna_hl7_core::builders::adt::AdtBuilder;

    let message = AdtBuilder::a31(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A31");
}

#[test]
fn dft_p03_conforms() {
    use vyna_hl7_core::builders::dft::DftP03Builder;

    let message = DftP03Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .transaction_date("20240115")
        .transaction_type("CG")
        .transaction_code("99213^Visit^CPT")
        .transaction_amount("150.00")
        .build()
        .unwrap();

    assert_conformant(&message, "DFT", "P03");
}

#[test]
fn dft_p11_conforms() {
    use vyna_hl7_core::builders::dft::DftP11Builder;

    let message = DftP11Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .transaction_date("20240115")
        .transaction_type("CG")
        .transaction_code("99213^Visit^CPT")
        .transaction_amount("150.00")
        .build()
        .unwrap();

    assert_conformant(&message, "DFT", "P11");
}

#[test]
fn qry_a19_conforms() {
    use vyna_hl7_core::builders::qry::QryA19Builder;

    let message = QryA19Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .query_id("Q1")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .quantity_limited_request("10^RD")
        .what_department_data_code("LAB")
        .build()
        .unwrap();

    assert_conformant(&message, "QRY", "A19");
}

#[test]
fn qry_q01_conforms() {
    use vyna_hl7_core::builders::qry::QryQ01Builder;

    let message = QryQ01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .query_id("Q1")
        .who_subject_filter("12345")
        .what_subject_filter("DEM")
        .quantity_limited_request("10^RD")
        .what_department_data_code("LAB")
        .build()
        .unwrap();

    assert_conformant(&message, "QRY", "Q01");
}

#[test]
fn qry_q02_conforms() {
    use vyna_hl7_core::builders::qry::QryQ02Builder;

    let message = QryQ02Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .query_id("Q1")
        .who_subject_filter("12345")
        .what_subject_filter("DEM")
        .quantity_limited_request("10^RD")
        .what_department_data_code("LAB")
        .build()
        .unwrap();

    assert_conformant(&message, "QRY", "Q02");
}

#[test]
fn mdm_t01_conforms() {
    use vyna_hl7_core::builders::mdm::MdmT01Builder;

    let message = MdmT01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .document_type("DS")
        .unique_document_number("DOC1")
        .document_completion_status("AU")
        .build()
        .unwrap();

    assert_conformant(&message, "MDM", "T01");
}

#[test]
fn mdm_t02_conforms() {
    use vyna_hl7_core::builders::mdm::MdmT02Builder;

    let message = MdmT02Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .document_type("DS")
        .unique_document_number("DOC1")
        .document_completion_status("AU")
        .observation_identifier("DS^Summary^L")
        .observation_value("Discharge summary text")
        .result_status("F")
        .build()
        .unwrap();

    assert_conformant(&message, "MDM", "T02");
}

#[test]
fn mdm_t04_conforms() {
    use vyna_hl7_core::builders::mdm::MdmT04Builder;

    let message = MdmT04Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .document_type("DS")
        .unique_document_number("DOC1")
        .document_completion_status("AU")
        .observation_identifier("DS^Summary^L")
        .observation_value("Status correction")
        .result_status("F")
        .build()
        .unwrap();

    assert_conformant(&message, "MDM", "T04");
}

#[test]
fn siu_s12_conforms() {
    use vyna_hl7_core::builders::siu::SiuS12Builder;

    let message = SiuS12Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .placer_appointment_id("APT1")
        .filler_appointment_id("FAPT1")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .event_reason("APT^Routine^L")
        .filler_contact("W1^Smith^John")
        .entered_by("W2^Jones^Amy")
        .resource_action_code("A")
        .build()
        .unwrap();

    assert_conformant(&message, "SIU", "S12");
}

#[test]
fn siu_s13_conforms() {
    use vyna_hl7_core::builders::siu::SiuS13Builder;

    let message = SiuS13Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .placer_appointment_id("APT1")
        .filler_appointment_id("FAPT1")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .event_reason("APT^Routine^L")
        .filler_contact("W1^Smith^John")
        .entered_by("W2^Jones^Amy")
        .resource_action_code("A")
        .build()
        .unwrap();

    assert_conformant(&message, "SIU", "S13");
}

#[test]
fn siu_s14_conforms() {
    use vyna_hl7_core::builders::siu::SiuS14Builder;

    let message = SiuS14Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .placer_appointment_id("APT1")
        .filler_appointment_id("FAPT1")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .event_reason("APT^Routine^L")
        .filler_contact("W1^Smith^John")
        .entered_by("W2^Jones^Amy")
        .resource_action_code("A")
        .build()
        .unwrap();

    assert_conformant(&message, "SIU", "S14");
}

#[test]
fn siu_s15_conforms() {
    use vyna_hl7_core::builders::siu::SiuS15Builder;

    let message = SiuS15Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .placer_appointment_id("APT1")
        .filler_appointment_id("FAPT1")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .event_reason("APT^Routine^L")
        .filler_contact("W1^Smith^John")
        .entered_by("W2^Jones^Amy")
        .resource_action_code("A")
        .build()
        .unwrap();

    assert_conformant(&message, "SIU", "S15");
}

#[test]
fn ack_new_conforms() {
    use vyna_hl7_core::builders::ack::AckBuilder;

    let message = AckBuilder::new(Version::V2_5, "M1", "SND", "SFAC")
        .trigger_event("A01")
        .accept()
        .build()
        .unwrap();

    assert_conformant(&message, "ACK", "");
}

#[test]
fn ack_for_message_conforms() {
    use vyna_hl7_core::builders::{ack::AckBuilder, adt::AdtBuilder};

    let incoming = AdtBuilder::a01(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();
    let message = AckBuilder::for_message(&incoming).accept().build().unwrap();

    assert_conformant(&message, "ACK", "");
}

#[test]
fn commit_ack_conforms() {
    use vyna_hl7_core::builders::{ack::CommitAckBuilder, adt::AdtBuilder};

    let incoming = AdtBuilder::a01(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();
    let message = CommitAckBuilder::for_message(&incoming)
        .commit_accept()
        .build()
        .unwrap();

    assert_conformant(&message, "ACK", "");
}

#[test]
fn rde_o11_conforms() {
    use vyna_hl7_core::builders::pharmacy::RdeO11Builder;

    let message = RdeO11Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("RX1")
        .order_control("NW")
        .give_code("08^HEPB^CVX")
        .give_amount_minimum("1")
        .give_units("TAB^Tablet^L")
        .route("PO")
        .build()
        .unwrap();

    assert_conformant(&message, "RDE", "O11");
}

#[test]
fn ras_o17_conforms() {
    use vyna_hl7_core::builders::pharmacy::RasO17Builder;

    let message = RasO17Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("RX1")
        .order_control("RE")
        .administered_code("08^HEPB^CVX")
        .amount("0.5")
        .datetime_start("20240115140000")
        .datetime_end("20240115140000")
        .route("IM")
        .build()
        .unwrap();

    assert_conformant(&message, "RAS", "O17");
}

#[test]
fn rds_o13_conforms() {
    use vyna_hl7_core::builders::pharmacy::RdsO13Builder;

    let message = RdsO13Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("RX1")
        .order_control("RE")
        .dispense_code("08^HEPB^CVX")
        .datetime_dispensed("20240115140000")
        .dispense_amount("1")
        .prescription_number("RX12345")
        .route("PO")
        .build()
        .unwrap();

    assert_conformant(&message, "RDS", "O13");
}

#[test]
fn rgv_o15_conforms() {
    use vyna_hl7_core::builders::pharmacy::RgvO15Builder;

    let message = RgvO15Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("RX1")
        .order_control("RE")
        .give_code("08^HEPB^CVX")
        .give_amount_minimum("1")
        .give_units("TAB^Tablet^L")
        .route("PO")
        .build()
        .unwrap();

    assert_conformant(&message, "RGV", "O15");
}

#[test]
fn rra_o18_conforms() {
    use vyna_hl7_core::builders::pharmacy::RraO18Builder;

    let message = RraO18Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .ack_code("AA")
        .acknowledged_control_id("M1")
        .build()
        .unwrap();

    assert_conformant(&message, "RRA", "O18");
}

#[test]
fn rrd_o14_conforms() {
    use vyna_hl7_core::builders::pharmacy::RrdO14Builder;

    let message = RrdO14Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .ack_code("AA")
        .acknowledged_control_id("M1")
        .build()
        .unwrap();

    assert_conformant(&message, "RRD", "O14");
}

#[test]
fn oru_r01_conforms() {
    use vyna_hl7_core::builders::oru::{Observation, OruR01Builder};

    let message = OruR01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .placer_order_number("ORD1")
        .filler_order_number("FILL1")
        .universal_service_id("CHEM^Chem^L")
        .result_status("F")
        .add_observation(Observation {
            set_id: 1,
            value_type: "NM".to_string(),
            identifier: "8867-4^Heart rate^LN".to_string(),
            value: "72".to_string(),
            units: Some("/min".to_string()),
            status: "F".to_string(),
        })
        .build()
        .unwrap();

    assert_conformant(&message, "ORU", "R01");
}
