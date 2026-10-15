//! Builder conformance suite: every message builder that can produce a
//! schema-valid message must do so, at HL7 v2.5, both as built and after a
//! strict parse round-trip.
//!
//! Out of scope (no message schema to conform to): QBP/RSP builders (no QBP,
//! RSP, or batch schemas exist), batch/file builders (not messages), the
//! field builders in `builders::fields` (not messages), and the rich
//! OMG/OMI/REF builders (no OMG, OMI, or REF schemas exist; REF wire shape
//! is covered by the core `rich_smoke` suite instead).

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

    // Composite builder inputs must encode with literal delimiters: a composite
    // stored as one component would escape to \S\, \T\, or \R\ on the wire,
    // which validates cleanly (short merged values) while corrupting data.
    let wire = message.encode();
    for escape in ["\\S\\", "\\T\\", "\\R\\"] {
        assert!(
            !wire.contains(escape),
            "{}^{} wire contains {} (composite stored as one component): {}",
            message_type,
            trigger_event,
            escape,
            wire.lines().find(|l| l.contains(escape)).unwrap_or("")
        );
    }

    let parsed = vyna_hl7_parser::parse_message(&wire).unwrap();
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

    // Composite service id: the builder splits it into components, so
    // each part counts against its own limit ("CHEM" vs the 20-char
    // component-1 cap, "L" against table 0396).
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
        .transaction_code("99213^Visit^C4")
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
        .transaction_code("99213^Visit^C4")
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

/// Assert one built component value (0-based component index).
fn assert_component(
    message: &Message,
    segment: &str,
    field: usize,
    component: usize,
    expected: &str,
) {
    let seg = message
        .get_segments_by_id(segment)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("missing {segment} segment"));
    let actual = seg
        .get_field(field)
        .unwrap_or_else(|| panic!("missing {segment}-{field}"))
        .get_component(0, component)
        .and_then(|c| c.value())
        .unwrap_or("");
    assert_eq!(
        actual,
        expected,
        "{segment}-{field} component {}",
        component + 1
    );
}

#[test]
fn composites_become_structure_not_escapes() {
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
        .transaction_code("99213^Visit^C4")
        .transaction_amount("150.00")
        .build()
        .unwrap();

    assert_component(&message, "PID", 5, 0, "Smith");
    assert_component(&message, "PID", 5, 1, "John");
    assert_component(&message, "FT1", 7, 0, "99213");
    assert_component(&message, "FT1", 7, 1, "Visit");
    assert_component(&message, "FT1", 7, 2, "C4");
    assert!(message.encode().contains("99213^Visit^C4"));
}

#[test]
fn pharmacy_composites_become_structure() {
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

    assert_component(&message, "PID", 5, 0, "Smith");
    assert_component(&message, "RXE", 2, 0, "08");
    assert_component(&message, "RXE", 2, 2, "CVX");
    assert!(message.encode().contains("08^HEPB^CVX"));
}

#[test]
fn siu_composites_become_structure() {
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

    assert_component(&message, "PID", 5, 1, "John");
    assert_component(&message, "SCH", 6, 0, "APT");
    assert_component(&message, "SCH", 16, 0, "W1");
    assert_component(&message, "SCH", 16, 2, "John");
}

#[test]
fn ack_msh9_becomes_structure() {
    use vyna_hl7_core::builders::ack::AckBuilder;

    let message = AckBuilder::new(Version::V2_5, "M1", "SND", "SFAC")
        .trigger_event("A01")
        .accept()
        .build()
        .unwrap();

    assert_component(&message, "MSH", 9, 0, "ACK");
    assert_component(&message, "MSH", 9, 1, "A01");
}

#[test]
fn oru_composites_become_structure() {
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

    assert_component(&message, "PID", 5, 0, "Smith");
    assert_component(&message, "OBR", 4, 0, "CHEM");
    assert_component(&message, "OBX", 3, 0, "8867-4");
    assert_component(&message, "OBX", 3, 2, "LN");
}

#[test]
fn vxu_and_qry_composites_become_structure() {
    use vyna_hl7_core::builders::qry::QryA19Builder;
    use vyna_hl7_core::builders::vxu::{ImmunizationDose, VxuV04Builder};

    let vxu = VxuV04Builder::new(Version::V2_5)
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
    assert_component(&vxu, "RXA", 5, 0, "08");
    assert_component(&vxu, "RXA", 5, 2, "CVX");

    let qry = QryA19Builder::new(Version::V2_5)
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
    assert_component(&qry, "QRD", 8, 0, "Smith");
    assert_component(&qry, "QRD", 8, 1, "John");
}

#[test]
fn field_builders_emit_structure() {
    use vyna_hl7_core::Delimiters;
    use vyna_hl7_core::builders::fields::{XadBuilder, XpnBuilder};

    let delims = Delimiters::default();
    let name = XpnBuilder::new()
        .family_name("Smith")
        .given_name("John")
        .build_field();
    assert_eq!(name.get_component(0, 0).unwrap().value(), Some("Smith"));
    assert_eq!(name.get_component(0, 1).unwrap().value(), Some("John"));
    assert_eq!(name.encode(&delims), "Smith^John");

    let addr = XadBuilder::new()
        .city("Springfield")
        .state("IL")
        .build_field();
    assert_eq!(
        addr.get_component(0, 2).unwrap().value(),
        Some("Springfield")
    );
    assert!(addr.encode(&delims).contains("Springfield^IL"));
}

#[test]
fn bar_p01_conforms() {
    use vyna_hl7_core::builders::bar::{BarP01Builder, Diagnosis, Insurance, Procedure};

    let message = BarP01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_account("ACCT1")
        .patient_class("I")
        .assigned_location("3N^301^B")
        .visit_number("VN1")
        .add_diagnosis(Diagnosis::new("I10^Essential hypertension^I10", "F"))
        .add_procedure(Procedure::new(
            "C4^Coronary angiography^C4",
            "20240115130000",
        ))
        .guarantor_name("Smith", "John")
        .insurance(Insurance::new("PPO^Preferred^L", "BCBS001"))
        .build()
        .unwrap();

    assert_conformant(&message, "BAR", "P01");
}

#[test]
fn bar_p02_conforms() {
    use vyna_hl7_core::builders::bar::{BarP02Builder, Diagnosis};

    let message = BarP02Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .add_diagnosis(Diagnosis::new("I10^Essential hypertension^I10", "F"))
        .build()
        .unwrap();

    assert_conformant(&message, "BAR", "P02");
}

#[test]
fn mfn_m01_conforms() {
    use vyna_hl7_core::builders::mfn::MfnM01Builder;
    use vyna_hl7_core::{Field, Segment};

    let mut loc = Segment::new("LOC");
    loc.add_field(Field::from_value("LOC1"));
    loc.add_field(Field::from_delimited("Ward 3N^3N^L"));
    let message = MfnM01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .master_file_id("LOC")
        .master_file_text("Location")
        .file_event("UPD")
        .response_level("NE")
        .record_event("MUP")
        .mf_control_id("MF100")
        .primary_key("LOC1")
        .primary_key_type("CE")
        .add_segment(loc)
        .build()
        .unwrap();

    assert_conformant(&message, "MFN", "M01");
}

// ---------------------------------------------------------------------------
// Rich clinical builders (`builders::rich`).
// ---------------------------------------------------------------------------

/// Rich-layer MSH assertions: the expected MSH-9 trigger plus the UTF-8
/// declaration every rich message carries in MSH-18.
fn assert_rich_msh(message: &Message, expected_msh9: &str) {
    let msh = message
        .get_segments_by_id("MSH")
        .into_iter()
        .next()
        .expect("rich message carries MSH");
    let msh9 = msh
        .get_field(9)
        .and_then(|f| f.get_repetition(0))
        .and_then(|r| r.full_value())
        .unwrap_or_default();
    assert!(
        msh9.starts_with(expected_msh9),
        "MSH-9 {msh9} starts with {expected_msh9}"
    );
    assert_eq!(
        msh.get_field(18).and_then(|f| f.value()),
        Some("UNICODE UTF-8"),
        "rich MSH-18 declares UTF-8"
    );
}

fn rich_demographics() -> vyna_hl7_core::builders::rich::PatientDemographics {
    use vyna_hl7_core::builders::rich::{CodedValue, PatientDemographics};
    PatientDemographics {
        patient_id_flat: Some("12345".to_string()),
        name: Some(("Smith".to_string(), "John".to_string())),
        date_of_birth: Some("19800101".to_string()),
        sex: Some("M".to_string()),
        // Coded: strict validation holds PID-10 to table 0005. The
        // free-text race shape stays covered by the core smoke suite.
        race: vec![CodedValue {
            code: Some("2028-9".to_string()),
            text: "Asian".to_string(),
            system: None,
        }],
        ..Default::default()
    }
}

fn rich_visit() -> vyna_hl7_core::builders::rich::VisitInfo {
    use vyna_hl7_core::builders::rich::VisitInfo;
    VisitInfo {
        patient_class: Some("I".to_string()),
        assigned_location: Some("3N^301^B".to_string()),
        attending_doctor: Some("W10001^Smith^John".to_string()),
        admit_datetime: Some("20240115120000".to_string()),
        ..Default::default()
    }
}

#[test]
fn rich_adt_a01_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA01Builder;
    use vyna_hl7_core::builders::rich::{NextOfKin, PatientAllergy};

    let message = RichAdtA01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .message_structure("ADT_A01")
        .demographics(rich_demographics())
        .visit(rich_visit())
        .allergies(vec![PatientAllergy {
            allergen_type: Some("DA".to_string()),
            allergen_name: "Penicillin".to_string(),
            severity: Some("SV".to_string()),
            reaction: Some("Rash".to_string()),
            ..Default::default()
        }])
        .next_of_kin(vec![NextOfKin {
            name: "Jane Smith".to_string(),
            relationship: Some("SPO".to_string()),
            contact_role: Some("C".to_string()),
            ..Default::default()
        }])
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A01");
    assert_rich_msh(&message, "ADT^A01^ADT_A01");
    assert_component(&message, "PID", 10, 0, "2028-9");
    assert_component(&message, "PID", 10, 1, "Asian");
    assert_component(&message, "AL1", 3, 0, "Penicillin");
    assert_component(&message, "NK1", 2, 0, "Jane Smith");
    assert_component(&message, "PV1", 44, 0, "20240115120000");
}

#[test]
fn rich_adt_a02_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA02Builder;

    let message = RichAdtA02Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A02");
    assert_rich_msh(&message, "ADT^A02");
}

#[test]
fn rich_adt_a03_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA03Builder;

    let message = RichAdtA03Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A03");
}

#[test]
fn rich_adt_a04_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA04Builder;

    let message = RichAdtA04Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A04");
    assert_rich_msh(&message, "ADT^A04");
}

#[test]
fn rich_adt_a05_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA05Builder;

    let message = RichAdtA05Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A05");
}

#[test]
fn rich_adt_a06_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA06Builder;

    let message = RichAdtA06Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A06");
    assert_rich_msh(&message, "ADT^A06");
}

#[test]
fn rich_adt_a07_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA07Builder;

    let message = RichAdtA07Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A07");
    assert_rich_msh(&message, "ADT^A07");
}

#[test]
fn rich_adt_a08_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA08Builder;

    let message = RichAdtA08Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A08");
}

#[test]
fn rich_adt_a09_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA09Builder;

    let message = RichAdtA09Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A09");
}

#[test]
fn rich_adt_a10_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA10Builder;

    let message = RichAdtA10Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A10");
}

#[test]
fn rich_adt_a11_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA11Builder;

    let message = RichAdtA11Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A11");
}

#[test]
fn rich_adt_a12_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA12Builder;

    let message = RichAdtA12Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A12");
}

#[test]
fn rich_adt_a13_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA13Builder;

    let message = RichAdtA13Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A13");
}

#[test]
fn rich_adt_a17_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA17Builder;

    let message = RichAdtA17Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A17");
}

#[test]
fn rich_adt_a28_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA28Builder;

    let message = RichAdtA28Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A28");
}

#[test]
fn rich_adt_a31_conforms() {
    use vyna_hl7_core::builders::rich::adt::RichAdtA31Builder;

    let message = RichAdtA31Builder::new(Version::V2_5)
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
    assert_rich_msh(&message, "ADT^A31");
}

#[test]
fn rich_adt_a40_conforms() {
    use vyna_hl7_core::builders::rich::PatientIdentifier;
    use vyna_hl7_core::builders::rich::adt::RichAdtA40Builder;

    let message = RichAdtA40Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("NEW123")
        .patient_name("Smith", "John")
        .patient_class("I")
        .prior_patient_identifier(PatientIdentifier {
            id: "OLD456".to_string(),
            identifier_type_code: Some("MR".to_string()),
            ..Default::default()
        })
        .prior_patient_name("Smyth", "Jon")
        .build()
        .unwrap();

    assert_conformant(&message, "ADT", "A40");
    assert_rich_msh(&message, "ADT^A40");
    assert_component(&message, "MRG", 1, 0, "OLD456");
    assert_component(&message, "MRG", 1, 4, "MR");
    assert_component(&message, "MRG", 7, 0, "Smyth");
}

#[test]
fn rich_siu_s12_conforms() {
    use vyna_hl7_core::builders::rich::siu::{AppointmentResource, RichSiuS12Builder};

    let message = RichSiuS12Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .placer_appointment_id("APT1")
        .filler_appointment_id("FAPT1")
        .event_reason("APT^Routine^L")
        .appointment_datetime("20240115130000")
        .appointment_end_datetime("20240115133000")
        .filler_contact("W1^Smith^John")
        .entered_by("W2^Jones^Amy")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .add_resource(AppointmentResource {
            resource_group_id: "RG1".to_string(),
            provider_id: "W10001".to_string(),
            provider_name: "Smith".to_string(),
            start_datetime: "20240115130000".to_string(),
            duration_minutes: 30,
            location: None,
        })
        .build()
        .unwrap();

    assert_conformant(&message, "SIU", "S12");
    assert_rich_msh(&message, "SIU^S12^SIU_S12");
    assert_component(&message, "SCH", 16, 0, "W1");
    assert_component(&message, "SCH", 20, 2, "Amy");
}

#[test]
fn rich_dft_p03_conforms() {
    use vyna_hl7_core::builders::rich::dft::RichDftP03Builder;

    let message = RichDftP03Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .transaction_date("20240115")
        .transaction_type("CG")
        .transaction_code("99213", "Office visit")
        .transaction_quantity("1")
        .transaction_amount("150.00")
        .build()
        .unwrap();

    assert_conformant(&message, "DFT", "P03");
    assert_rich_msh(&message, "DFT^P03^DFT_P03");
    assert_component(&message, "FT1", 7, 0, "99213");
}

#[test]
fn rich_oru_r01_conforms() {
    use vyna_hl7_core::builders::rich::laboratory::SpecimenInfo;
    use vyna_hl7_core::builders::rich::oru::{Observation, RichOruR01Builder};

    let message = RichOruR01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .message_structure("ORU_R01")
        .demographics(rich_demographics())
        .visit(rich_visit())
        .placer_order_number("ORD1")
        .filler_order_number("FILL1")
        .universal_service_coded("CHEM", "Chem", "L")
        .add_observation(Observation {
            set_id: 1,
            value_type: "NM".to_string(),
            identifier: "Heart rate".to_string(),
            identifier_code: Some("8867-4".to_string()),
            identifier_system: Some("LN".to_string()),
            value: "72".to_string(),
            units: Some("/min".to_string()),
            abnormal_flag: vec!["H".to_string(), "U".to_string()],
            status: "F".to_string(),
            ..Default::default()
        })
        .add_specimen(SpecimenInfo {
            specimen_type: "BLD".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "ORU", "R01");
    assert_rich_msh(&message, "ORU^R01^ORU_R01");
    assert_component(&message, "OBR", 4, 0, "CHEM");
    assert_component(&message, "OBX", 3, 0, "8867-4");
    assert_component(&message, "SPM", 4, 0, "BLD");
    // OBX-8 renders one repetition per flag, never one value with a tilde.
    let wire = message.encode();
    let obx = wire.split('\r').find(|l| l.starts_with("OBX")).unwrap();
    assert!(obx.contains("|H~U|"), "{obx}");
}

#[test]
fn rich_mdm_t02_conforms() {
    use vyna_hl7_core::builders::rich::mdm::RichMdmT02Builder;

    let message = RichMdmT02Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .document_type("DS")
        .unique_document_number("DOC1")
        .completion_status("AU")
        .document_content("Discharge summary text")
        .build()
        .unwrap();

    assert_conformant(&message, "MDM", "T02");
    assert_rich_msh(&message, "MDM^T02^MDM_T02");
    assert_component(&message, "TXA", 2, 0, "DS");
    assert_component(&message, "TXA", 12, 0, "DOC1");
    assert_component(&message, "OBX", 5, 0, "Discharge summary text");
}

#[test]
fn rich_omp_o09_conforms() {
    use vyna_hl7_core::builders::rich::pharmacy::{PharmacyPrescription, RichOmpO09Builder};

    let message = RichOmpO09Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .demographics(rich_demographics())
        .visit(rich_visit())
        .order_control("NW")
        .prescription(PharmacyPrescription {
            order_id: "RX1".to_string(),
            medication_code: Some("00093-0058".to_string()),
            medication_name: "Loraz".to_string(),
            coding_system: "NDC".to_string(),
            dose: "1".to_string(),
            dose_unit: "TAB".to_string(),
            route: "PO".to_string(),
            frequency: "BID".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "OMP", "O09");
    assert_rich_msh(&message, "OMP^O09^OMP_O09");
    assert_component(&message, "ORC", 1, 0, "NW");
    assert_component(&message, "RXR", 1, 0, "PO");
}

#[test]
fn rich_oml_o21_conforms() {
    use vyna_hl7_core::builders::rich::laboratory::{LabOrder, RichOmlO21Builder, SpecimenInfo};

    let message = RichOmlO21Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .order_control("NW")
        .order(LabOrder {
            order_number: "ORD1".to_string(),
            service_code: "CHEM".to_string(),
            service_name: "Chem".to_string(),
            service_coding_system: Some("L".to_string()),
            priority: "R".to_string(),
            ordering_provider: Some((
                "W10001".to_string(),
                "Smith".to_string(),
                "John".to_string(),
            )),
            ..Default::default()
        })
        .specimen(SpecimenInfo {
            specimen_type: "BLD".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "OML", "O21");
    assert_rich_msh(&message, "OML^O21^OML_O21");
    assert_component(&message, "OBR", 4, 0, "CHEM");
    assert_component(&message, "SPM", 4, 0, "BLD");
}

#[test]
fn rich_bar_p01_conforms() {
    use vyna_hl7_core::builders::rich::billing::{InsuranceInfo, RichBarP01Builder};
    use vyna_hl7_core::builders::rich::referral::Diagnosis;

    let message = RichBarP01Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .diagnosis(Diagnosis {
            code: Some("I10".to_string()),
            description: Some("Essential hypertension".to_string()),
            ..Default::default()
        })
        .insurance(InsuranceInfo {
            guarantor_name: Some("John Smith".to_string()),
            company_id: Some("BCBS001".to_string()),
            plan_name: Some("Gold PPO 2025".to_string()),
            member_id: Some("M123".to_string()),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "BAR", "P01");
    assert_rich_msh(&message, "BAR^P01^BAR_P01");
    assert_component(&message, "GT1", 3, 0, "John Smith");
    assert_component(&message, "IN1", 3, 0, "BCBS001");
}

#[test]
fn rich_rde_o11_conforms() {
    use vyna_hl7_core::builders::rich::pharmacy::{PharmacyPrescription, RichRdeO11Builder};

    let message = RichRdeO11Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .order_control("NW")
        .prescription(PharmacyPrescription {
            order_id: "RX1".to_string(),
            medication_code: Some("08".to_string()),
            medication_name: "HEPB".to_string(),
            coding_system: "CVX".to_string(),
            dose: "1".to_string(),
            dose_unit: "TAB".to_string(),
            route: "PO".to_string(),
            frequency: "BID".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "RDE", "O11");
    assert_rich_msh(&message, "RDE^O11^RDE_O11");
    assert_component(&message, "RXE", 2, 0, "08");
    assert!(
        !message.get_segments_by_id("TQ1").is_empty(),
        "RDE carries schema-required TQ1"
    );
}

#[test]
fn rich_rds_o13_conforms() {
    use vyna_hl7_core::builders::rich::pharmacy::{DispenseEvent, RichRdsO13Builder};

    let message = RichRdsO13Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .patient_class("I")
        .dispense(DispenseEvent {
            order_id: "RX1".to_string(),
            dispense_code: "08".to_string(),
            dispense_code_name: "HEPB".to_string(),
            coding_system: "CVX".to_string(),
            dispense_date: "20240115140000".to_string(),
            actual_dose: "1".to_string(),
            actual_dose_unit: "TAB".to_string(),
            prescription_number: Some("RX12345".to_string()),
            actual_route: "PO".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "RDS", "O13");
    assert_rich_msh(&message, "RDS^O13^RDS_O13");
    assert_component(&message, "RXD", 7, 0, "RX12345");
}

#[test]
fn rich_ras_o17_conforms() {
    use vyna_hl7_core::builders::rich::pharmacy::{AdministrationEvent, RichRasO17Builder};

    let message = RichRasO17Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .demographics(rich_demographics())
        .visit(rich_visit())
        .administration(AdministrationEvent {
            order_id: "RX1".to_string(),
            medication_code: Some("08".to_string()),
            medication_name: "HEPB".to_string(),
            coding_system: "CVX".to_string(),
            start_datetime: "20240115140000".to_string(),
            end_datetime: "20240115140000".to_string(),
            administered_amount: "0.5".to_string(),
            completion_status: "CP".to_string(),
            route: Some("IM".to_string()),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "RAS", "O17");
    assert_rich_msh(&message, "RAS^O17^RAS_O17");
    assert_component(&message, "RXA", 6, 0, "0.5");
}

#[test]
fn rich_vxu_v04_conforms() {
    use vyna_hl7_core::builders::rich::pharmacy::{ImmunizationEvent, RichVxuV04Builder};

    let message = RichVxuV04Builder::new(Version::V2_5)
        .sending_application("SEND")
        .sending_facility("SFAC")
        .receiving_application("RECV")
        .receiving_facility("RFAC")
        .patient_id("12345")
        .patient_name("Smith", "John")
        .immunization(ImmunizationEvent {
            vaccine_code: Some("08".to_string()),
            vaccine_name: "HEPB".to_string(),
            administered_date: "20240115".to_string(),
            route: Some("IM".to_string()),
            dose_number: Some(1),
            ..Default::default()
        })
        .build()
        .unwrap();

    assert_conformant(&message, "VXU", "V04");
    assert_rich_msh(&message, "VXU^V04^VXU_V04");
    assert_component(&message, "RXA", 5, 0, "08");
}
