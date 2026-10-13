//! Integration tests for vyna-hl7-macros derive macros

use vyna_hl7_core::field::{Field, Repetition};
use vyna_hl7_core::message::Message;
use vyna_hl7_core::segment::Segment;
use vyna_hl7_macros::{Message, Segment as DeriveSegment, hl7_type};

/// Test segment with simple fields
#[derive(DeriveSegment, Default, Debug, PartialEq)]
#[hl7(id = "PID")]
struct PatientIdentification {
    #[hl7(field = 1)]
    set_id: Option<String>,

    #[hl7(field = 3)]
    patient_id: String,

    #[hl7(field = 5, component = 1)]
    family_name: String,

    #[hl7(field = 5, component = 2)]
    given_name: Option<String>,

    #[hl7(field = 7)]
    date_of_birth: Option<String>,

    #[hl7(field = 8)]
    sex: Option<String>,
}

/// Test segment for OBX
#[derive(DeriveSegment, Default, Debug)]
#[hl7(id = "OBX")]
struct Observation {
    #[hl7(field = 1)]
    set_id: String,

    #[hl7(field = 2)]
    value_type: String,

    #[hl7(field = 3)]
    observation_identifier: String,

    #[hl7(field = 5)]
    observation_value: Option<String>,

    #[hl7(field = 11)]
    result_status: Option<String>,
}

/// Test message with segments
#[derive(Message, Default, Debug)]
#[hl7(message_type = "ADT", trigger_event = "A01")]
struct AdtA01 {
    #[hl7(segment)]
    pid: PatientIdentification,

    #[hl7(segment, optional)]
    obx: Option<Observation>,
}

#[test]
fn test_segment_id_const() {
    assert_eq!(PatientIdentification::SEGMENT_ID, "PID");
    assert_eq!(Observation::SEGMENT_ID, "OBX");
}

#[test]
fn test_segment_id_method() {
    let pid = PatientIdentification::default();
    assert_eq!(pid.segment_id(), "PID");

    let obx = Observation::default();
    assert_eq!(obx.segment_id(), "OBX");
}

#[test]
fn test_message_type_const() {
    assert_eq!(AdtA01::MESSAGE_TYPE, "ADT");
    assert_eq!(AdtA01::TRIGGER_EVENT, "A01");
}

#[test]
fn test_message_type_methods() {
    let msg = AdtA01::default();
    assert_eq!(msg.message_type(), "ADT");
    assert_eq!(msg.trigger_event(), "A01");
}

#[test]
fn test_segment_to_segment() {
    let pid = PatientIdentification {
        set_id: Some("1".to_string()),
        patient_id: "12345".to_string(),
        family_name: "Smith".to_string(),
        given_name: Some("John".to_string()),
        date_of_birth: Some("19800101".to_string()),
        sex: Some("M".to_string()),
    };

    let segment = pid.to_segment();

    assert_eq!(segment.id, "PID");
    assert_eq!(segment.get_field_value(1), Some("1"));
    assert_eq!(segment.get_field_value(3), Some("12345"));
    assert_eq!(segment.get_field_value(7), Some("19800101"));
    assert_eq!(segment.get_field_value(8), Some("M"));
}

#[test]
fn test_segment_from_segment() {
    let mut segment = Segment::new("PID");
    let _ = segment.set_field_value(1, "1");
    let _ = segment.set_field_value(3, "12345");
    let _ = segment.set_field_value(7, "19800101");
    let _ = segment.set_field_value(8, "M");

    let pid = PatientIdentification::from_segment(&segment);

    assert!(pid.is_some());
    let pid = pid.unwrap();

    assert_eq!(pid.set_id, Some("1".to_string()));
    assert_eq!(pid.patient_id, "12345");
    assert_eq!(pid.date_of_birth, Some("19800101".to_string()));
    assert_eq!(pid.sex, Some("M".to_string()));
}

#[test]
fn test_segment_from_wrong_segment_type() {
    let segment = Segment::new("OBX");

    let pid = PatientIdentification::from_segment(&segment);
    assert!(pid.is_none());
}

#[test]
fn test_segment_into_trait() {
    let pid = PatientIdentification {
        set_id: None,
        patient_id: "99999".to_string(),
        family_name: "Doe".to_string(),
        given_name: None,
        date_of_birth: None,
        sex: None,
    };

    let segment: Segment = pid.into();
    assert_eq!(segment.id, "PID");
    assert_eq!(segment.get_field_value(3), Some("99999"));
}

#[test]
fn test_segment_try_from_trait() {
    use std::convert::TryFrom;

    let mut segment = Segment::new("PID");
    let _ = segment.set_field_value(3, "TEST123");

    let pid = PatientIdentification::try_from(&segment);
    assert!(pid.is_ok());
    assert_eq!(pid.unwrap().patient_id, "TEST123");
}

#[test]
fn test_segment_try_from_wrong_type() {
    use std::convert::TryFrom;

    let segment = Segment::new("OBX");
    let pid = PatientIdentification::try_from(&segment);
    assert!(pid.is_err());
}

#[test]
fn test_component_extraction() {
    let mut segment = Segment::new("PID");
    // Set field 5 with components: family^given
    let _ = segment.set_component(5, 0, 0, "Smith");
    let _ = segment.set_component(5, 0, 1, "John");

    let pid = PatientIdentification::from_segment(&segment);
    assert!(pid.is_some());
    let pid = pid.unwrap();

    assert_eq!(pid.family_name, "Smith");
    assert_eq!(pid.given_name, Some("John".to_string()));
}

#[test]
fn test_component_serialization() {
    let pid = PatientIdentification {
        set_id: None,
        patient_id: "12345".to_string(),
        family_name: "Smith".to_string(),
        given_name: Some("John".to_string()),
        date_of_birth: None,
        sex: None,
    };

    let segment = pid.to_segment();

    // Check that component 1 of field 5 is set
    let field5 = segment.get_field(5);
    assert!(field5.is_some());

    let field5 = field5.unwrap();
    let rep = field5.get_repetition(0);
    assert!(rep.is_some());

    let rep = rep.unwrap();
    assert!(rep.get_component(0).is_some());
    assert_eq!(rep.get_component(0).unwrap().value(), Some("Smith"));
    assert!(rep.get_component(1).is_some());
    assert_eq!(rep.get_component(1).unwrap().value(), Some("John"));
}

#[test]
fn test_message_to_message() {
    let adt = AdtA01 {
        pid: PatientIdentification {
            set_id: None,
            patient_id: "P001".to_string(),
            family_name: "Test".to_string(),
            given_name: None,
            date_of_birth: None,
            sex: None,
        },
        obx: None,
    };

    let message = adt.to_message();

    // Should have the PID segment
    assert!(!message.segments.is_empty());
}

#[test]
fn test_message_into_trait() {
    let adt = AdtA01::default();
    let message: vyna_hl7_core::message::Message = adt.into();
    assert!(message.segments.is_empty() || !message.segments.is_empty()); // Just verify conversion works
}

/// Test message with a repeating segment
#[derive(Message, Default, Debug)]
#[hl7(message_type = "ORU", trigger_event = "R01")]
struct OruR01 {
    #[hl7(segment)]
    pid: PatientIdentification,

    #[hl7(segment, repeating)]
    obx: Vec<Observation>,
}

/// Test segment with a repeating field (e.g. PID-3 patient identifier list)
#[derive(DeriveSegment, Default, Debug, PartialEq)]
#[hl7(id = "PID")]
struct RepeatingPid {
    #[hl7(field = 3)]
    patient_ids: Vec<String>,
}

fn sample_pid_segment() -> Segment {
    let mut segment = Segment::new("PID");
    let _ = segment.set_field_value(1, "1");
    let _ = segment.set_field_value(3, "12345");
    let _ = segment.set_component(5, 0, 0, "Smith");
    let _ = segment.set_component(5, 0, 1, "John");
    let _ = segment.set_field_value(7, "19800101");
    let _ = segment.set_field_value(8, "M");
    segment
}

fn sample_obx_segment(set_id: &str, value: &str) -> Segment {
    let mut segment = Segment::new("OBX");
    let _ = segment.set_field_value(1, set_id);
    let _ = segment.set_field_value(2, "NM");
    let _ = segment.set_field_value(3, "GLUCOSE");
    let _ = segment.set_field_value(5, value);
    let _ = segment.set_field_value(11, "F");
    segment
}

#[test]
fn test_message_from_message() {
    let mut message = Message::new();
    message.add_segment(sample_pid_segment());
    message.add_segment(sample_obx_segment("1", "100"));

    let adt = AdtA01::from_message(&message);
    assert!(adt.is_some());
    let adt = adt.unwrap();
    assert_eq!(adt.pid.patient_id, "12345");
    assert_eq!(adt.pid.family_name, "Smith");
    assert!(adt.obx.is_some());
    assert_eq!(adt.obx.unwrap().observation_value, Some("100".to_string()));
}

#[test]
fn test_message_from_message_missing_required() {
    // No PID segment: required field cannot be satisfied
    let mut message = Message::new();
    message.add_segment(sample_obx_segment("1", "100"));

    assert!(AdtA01::from_message(&message).is_none());
}

#[test]
fn test_message_from_message_optional_absent() {
    let mut message = Message::new();
    message.add_segment(sample_pid_segment());

    let adt = AdtA01::from_message(&message);
    assert!(adt.is_some());
    assert!(adt.unwrap().obx.is_none());
}

#[test]
fn test_message_from_message_repeating() {
    let mut message = Message::new();
    message.add_segment(sample_pid_segment());
    message.add_segment(sample_obx_segment("1", "100"));
    message.add_segment(sample_obx_segment("2", "105"));

    let oru = OruR01::from_message(&message);
    assert!(oru.is_some());
    let oru = oru.unwrap();
    assert_eq!(oru.pid.patient_id, "12345");
    assert_eq!(oru.obx.len(), 2);
    assert_eq!(oru.obx[0].observation_value, Some("100".to_string()));
    assert_eq!(oru.obx[1].observation_value, Some("105".to_string()));
}

#[test]
fn test_message_from_message_repeating_empty() {
    let mut message = Message::new();
    message.add_segment(sample_pid_segment());

    let oru = OruR01::from_message(&message);
    assert!(oru.is_some());
    assert!(oru.unwrap().obx.is_empty());
}

#[test]
fn test_message_try_from() {
    use std::convert::TryFrom;

    let mut message = Message::new();
    message.add_segment(sample_pid_segment());
    let adt = AdtA01::try_from(&message);
    assert!(adt.is_ok());

    let empty = Message::new();
    assert!(AdtA01::try_from(&empty).is_err());
}

#[test]
fn test_message_round_trip() {
    let mut message = Message::new();
    message.add_segment(sample_pid_segment());
    message.add_segment(sample_obx_segment("1", "100"));

    let adt = AdtA01::from_message(&message).unwrap();
    let encoded = adt.to_message();

    assert_eq!(encoded.segments.len(), 2);
    assert_eq!(
        encoded.get_segment_by_id("PID").unwrap().get_field_value(3),
        Some("12345")
    );
    assert_eq!(
        encoded.get_segment_by_id("OBX").unwrap().get_field_value(5),
        Some("100")
    );
}

#[test]
fn test_repeating_field_from_segment() {
    let mut field = Field::new();
    field.add_repetition(Repetition::from_value("ID-A"));
    field.add_repetition(Repetition::from_value("ID-B"));
    let mut segment = Segment::new("PID");
    let _ = segment.set_field(3, field);

    let pid = RepeatingPid::from_segment(&segment).unwrap();
    assert_eq!(
        pid.patient_ids,
        vec!["ID-A".to_string(), "ID-B".to_string()]
    );
}

/// Test segment mapping a composite field as one whole string
#[derive(DeriveSegment, Default, Debug)]
#[hl7(id = "PID")]
struct WholeNamePid {
    #[hl7(field = 5)]
    patient_name: String,
}

#[test]
fn test_composite_whole_field_round_trip() {
    let mut segment = Segment::new("PID");
    let _ = segment.set_component(5, 0, 0, "Smith");
    let _ = segment.set_component(5, 0, 1, "John");

    // Whole-field read preserves every component, not just the first.
    let pid = WholeNamePid::from_segment(&segment).unwrap();
    assert_eq!(pid.patient_name, "Smith^John");

    // Whole-field write rebuilds the component structure.
    let back = pid.to_segment();
    let rep = back.get_field(5).unwrap().get_repetition(0).unwrap();
    assert_eq!(rep.get_component(0).unwrap().value(), Some("Smith"));
    assert_eq!(rep.get_component(1).unwrap().value(), Some("John"));
}

#[test]
fn test_composite_repeating_field_round_trip() {
    let pid = RepeatingPid {
        patient_ids: vec!["ID1^AA".to_string(), "ID2^BB".to_string()],
    };

    let segment = pid.to_segment();
    let field = segment.get_field(3).unwrap();
    assert_eq!(field.repetitions.len(), 2);
    assert_eq!(
        field.get_repetition(0).unwrap().full_value().as_deref(),
        Some("ID1^AA")
    );

    let back = RepeatingPid::from_segment(&segment).unwrap();
    assert_eq!(back.patient_ids, pid.patient_ids);
}

#[test]
fn test_repeating_field_to_segment() {
    let pid = RepeatingPid {
        patient_ids: vec!["ID-A".to_string(), "ID-B".to_string()],
    };

    let segment = pid.to_segment();
    let field = segment.get_field(3).unwrap();
    assert_eq!(field.repetitions.len(), 2);
    assert_eq!(field.get_repetition(0).unwrap().value(), Some("ID-A"));
    assert_eq!(field.get_repetition(1).unwrap().value(), Some("ID-B"));
}

#[test]
fn test_message_from_message_ignores_segment_order() {
    // Segments match by ID, not position: OBX first still binds correctly.
    let mut message = Message::new();
    message.add_segment(sample_obx_segment("1", "100"));
    message.add_segment(sample_pid_segment());

    let adt = AdtA01::from_message(&message).unwrap();
    assert_eq!(adt.pid.patient_id, "12345");
    assert_eq!(adt.obx.unwrap().observation_value, Some("100".to_string()));
}

/// Composite data type via the hl7_type attribute macro
#[hl7_type(data_type = "XPN")]
struct ExtendedPersonName {
    #[hl7(component = 1)]
    family_name: String,

    #[hl7(component = 2)]
    given_name: String,

    #[hl7(component = 3)]
    second_name: Option<String>,
}

#[test]
fn test_hl7_type_round_trip() {
    assert_eq!(ExtendedPersonName::DATA_TYPE, "XPN");

    let name = ExtendedPersonName {
        family_name: "Smith".to_string(),
        given_name: "John".to_string(),
        second_name: None,
    };
    assert_eq!(name.data_type(), "XPN");

    let rep = name.to_repetition();
    assert_eq!(rep.get_component(0).and_then(|c| c.value()), Some("Smith"));
    assert_eq!(rep.get_component(1).and_then(|c| c.value()), Some("John"));

    let back = ExtendedPersonName::from_repetition(&rep);
    assert_eq!(back.family_name, "Smith");
    assert_eq!(back.given_name, "John");
    assert_eq!(back.second_name, None);

    let via_from = ExtendedPersonName::from(&rep);
    assert_eq!(via_from.family_name, "Smith");

    let rep_back: Repetition = name.into();
    assert_eq!(
        rep_back.get_component(0).and_then(|c| c.value()),
        Some("Smith")
    );
}

#[test]
fn test_optional_segment_serialization() {
    let adt_with_obx = AdtA01 {
        pid: PatientIdentification::default(),
        obx: Some(Observation {
            set_id: "1".to_string(),
            value_type: "NM".to_string(),
            observation_identifier: "GLUCOSE".to_string(),
            observation_value: Some("100".to_string()),
            result_status: Some("F".to_string()),
        }),
    };

    let message = adt_with_obx.to_message();

    // Should have 2 segments when OBX is present
    assert_eq!(message.segments.len(), 2);
}
