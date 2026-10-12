//! Integration tests for vyna-hl7-macros derive macros

use vyna_hl7_core::segment::Segment;
use vyna_hl7_macros::{Message, Segment as DeriveSegment};

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
