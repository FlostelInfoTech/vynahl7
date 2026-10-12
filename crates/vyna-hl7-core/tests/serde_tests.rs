//! Serde round-trips for the core model (`serde` feature).
#![cfg(feature = "serde")]

use vyna_hl7_core::{Delimiters, Field, Message, Segment};
use vyna_hl7_parser::parse_message;

const ADT: &str = "MSH|^~\\&|SEND|SFAC|RECV|RFAC|20240101120000||ADT^A01|MSG001|P|2.5\rPID|1||PAT1^^^HOSP^MR~PAT2^^^CLIN^PI||Smith^John^A||19800101|M\rPV1|1|I\r";

#[test]
fn message_json_round_trip_preserves_everything() {
    let message = parse_message(ADT).unwrap();
    let json = serde_json::to_string(&message).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();

    assert_eq!(message, back);
    assert_eq!(message.encode(), back.encode());
}

#[test]
fn message_json_shape_is_structural() {
    let message = parse_message(ADT).unwrap();
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
    let message = parse_message(ADT).unwrap();
    let mut custom = message.clone();
    custom.delimiters = Delimiters::from_encoding_characters('#', "^~\\&").unwrap();

    let json = serde_json::to_string(&custom).unwrap();
    let back: Message = serde_json::from_str(&json).unwrap();

    assert_eq!(back.delimiters.field_separator, '#');
    assert_eq!(custom, back);
}

#[test]
fn segment_and_field_round_trip() {
    let message = parse_message(ADT).unwrap();
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
