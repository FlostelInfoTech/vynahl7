//! End-to-end proof for schema-driven typed models.
//!
//! The checked-in module under `tests/generated/` is real generator output
//! (see `examples/generate.rs`). These tests prove it compiles, parses a real
//! wire message, and stays in sync with the generator.

use vyna_hl7_codegen::generate_for;
use vyna_hl7_core::Version;

#[path = "generated/v2_5_adt_a01.rs"]
mod v2_5_adt_a01;

use v2_5_adt_a01::AdtA01;

const SAMPLE_ADT_A01: &str = "MSH|^~\\&|SEND|SFAC|RECV|RFAC|20240101120000||ADT^A01|MSG001|P|2.5\rEVN||20240101120000\rPID|1||12345^^^HOSP^MR||Smith^John^A||19800101|M\rPV1|1|I|WARD^101^A\r";

#[test]
fn generated_fixture_matches_generator_output() {
    let source = generate_for(Version::V2_5, "ADT", "A01").unwrap();
    let fixture = include_str!("generated/v2_5_adt_a01.rs");
    assert_eq!(
        source, fixture,
        "fixture is stale - run `cargo run -p vyna-hl7-codegen --example generate`"
    );
}

#[test]
fn generated_model_parses_a_real_message() {
    let message = vyna_hl7_parser::parse_message(SAMPLE_ADT_A01).unwrap();
    let adt = AdtA01::from_message(&message).unwrap();

    // MSH-9 maps per component.
    assert_eq!(adt.msh.message_type_message_code, "ADT");
    assert_eq!(adt.msh.message_type_trigger_event, "A01");

    // Repeating composite fields stay whole, one Vec element per repetition.
    assert_eq!(adt.pid.patient_identifier_list, vec!["12345^^^HOSP^MR"]);
    assert_eq!(adt.pid.patient_name, vec!["Smith^John^A"]);
    assert_eq!(
        adt.pid.date_time_of_birth_time_of_an_event.as_deref(),
        Some("19800101")
    );
    assert_eq!(adt.pid.administrative_sex.as_deref(), Some("M"));

    // PV1-3 maps per component.
    assert_eq!(adt.pv1.patient_class, "I");
    assert_eq!(
        adt.pv1.assigned_patient_location_point_of_care.as_deref(),
        Some("WARD")
    );
    assert_eq!(
        adt.pv1.assigned_patient_location_room.as_deref(),
        Some("101")
    );
    assert_eq!(adt.pv1.assigned_patient_location_bed.as_deref(), Some("A"));

    // Absent optional / repeating segments stay empty.
    assert!(adt.pd1.is_none());
    assert!(adt.rol.is_empty());
    assert!(adt.obx.is_empty());
}

#[test]
fn generated_model_round_trips_back_to_wire_format() {
    let message = vyna_hl7_parser::parse_message(SAMPLE_ADT_A01).unwrap();
    let adt = AdtA01::from_message(&message).unwrap();

    let encoded = adt.to_message().encode();
    assert!(encoded.starts_with("MSH|^~\\&|SEND|SFAC|"), "{encoded}");
    assert!(encoded.contains("ADT^A01"), "{encoded}");
    assert!(encoded.contains("12345^^^HOSP^MR"), "{encoded}");
    assert!(encoded.contains("Smith^John^A"), "{encoded}");
    assert!(encoded.contains("WARD^101^A"), "{encoded}");
}
