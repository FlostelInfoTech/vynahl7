//! Serde round-trips for the core model (`serde` feature).
#![cfg(feature = "serde")]

use vyna_hl7_core::{Delimiters, Field, Message, Segment};

/// MSH/PID/PV1 message with repeating PID-3 and a three-part XPN name.
///
/// Built by hand so core's tests don't depend on the parser crate (core
/// is the lower layer; the dependency would also block publishing).
fn fixture() -> Message {
    let mut message = Message::default();

    let mut msh = Segment::new("MSH");
    msh.set_field(1, Field::from_value("|")).unwrap();
    msh.set_field(2, Field::from_value("^~\\&")).unwrap();
    msh.set_field(3, Field::from_value("SEND")).unwrap();
    msh.set_field(4, Field::from_value("SFAC")).unwrap();
    msh.set_field(5, Field::from_value("RECV")).unwrap();
    msh.set_field(6, Field::from_value("RFAC")).unwrap();
    msh.set_field(7, Field::from_value("20240101120000"))
        .unwrap();
    msh.set_field(9, Field::from_delimited("ADT^A01")).unwrap();
    msh.set_field(10, Field::from_value("MSG001")).unwrap();
    msh.set_field(11, Field::from_value("P")).unwrap();
    msh.set_field(12, Field::from_value("2.5")).unwrap();
    message.add_segment(msh);

    let mut pid = Segment::new("PID");
    pid.set_field(1, Field::from_value("1")).unwrap();
    pid.set_field(3, Field::from_delimited("PAT1^^^HOSP^MR~PAT2^^^CLIN^PI"))
        .unwrap();
    pid.set_field(5, Field::from_delimited("Smith^John^A"))
        .unwrap();
    pid.set_field(7, Field::from_value("19800101")).unwrap();
    pid.set_field(8, Field::from_value("M")).unwrap();
    message.add_segment(pid);

    let mut pv1 = Segment::new("PV1");
    pv1.set_field(1, Field::from_value("1")).unwrap();
    pv1.set_field(2, Field::from_value("I")).unwrap();
    message.add_segment(pv1);

    message
}

#[test]
fn message_json_round_trip_preserves_everything() {
    let message = fixture();
    let json = serde_json::to_string(&message).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();

    assert_eq!(message, back);
    assert_eq!(message.encode(), back.encode());
}

#[test]
fn message_json_shape_is_structural() {
    let message = fixture();
    let json: serde_json::Value = serde_json::to_value(&message).unwrap();

    assert_eq!(json["segments"][0]["id"], "MSH");
    assert_eq!(json["segments"][1]["id"], "PID");
    assert_eq!(json["delimiters"]["field_separator"], "|");
    // Repeating PID-3 survived as structured data.
    assert_eq!(
        json["segments"][1]["fields"][2]["repetitions"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn custom_delimiters_survive_the_round_trip() {
    let message = fixture();
    let mut custom = message.clone();
    custom.delimiters = Delimiters::from_encoding_characters('#', "^~\\&").unwrap();

    let json = serde_json::to_string(&custom).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();

    assert_eq!(back.delimiters.field_separator, '#');
    assert_eq!(custom, back);
}

#[test]
fn segment_and_field_round_trip() {
    let message = fixture();
    let pid = message.get_segment_by_id("PID").unwrap();

    let json = serde_json::to_string(pid).unwrap();
    let segment_back: Segment = serde_json::from_str(&json).unwrap();
    assert_eq!(*pid, segment_back);

    let name = pid.get_field(5).unwrap();
    let json = serde_json::to_string(name).unwrap();
    let field_back: Field = serde_json::from_str(&json).unwrap();
    assert_eq!(*name, field_back);
    // Components and subcomponents are structural, not flattened.
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        value["repetitions"][0]["components"][0]["subcomponents"][0]["value"],
        "Smith"
    );
}

#[test]
fn invalid_json_is_an_error() {
    let result: Result<Message, _> = serde_json::from_str(r#"{"segments": []}"#);
    assert!(result.is_err()); // delimiters missing
}
