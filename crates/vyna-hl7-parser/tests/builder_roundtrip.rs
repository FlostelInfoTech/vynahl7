//! Encode/parse round-trips for the QBP/RSP query builders.
//!
//! The builders' unit tests pin segment counts and ids; these tests prove
//! the built messages also survive the wire: encode, parse, and compare.

use vyna_hl7_core::Version;
use vyna_hl7_core::builders::qbp::{QbpQ11Builder, QbpQ21Builder, QbpQ22Builder};
use vyna_hl7_core::builders::rsp::{RspK11Builder, RspK21Builder, RspK22Builder};
use vyna_hl7_core::field::Field;
use vyna_hl7_core::segment::Segment;
use vyna_hl7_parser::parse_message;

fn segment_ids(message: &vyna_hl7_core::Message) -> Vec<&str> {
    message.segments.iter().map(|s| s.id.as_str()).collect()
}

#[test]
fn qbp_q11_roundtrip() {
    let built = QbpQ11Builder::new(Version::V2_5_1)
        .sending_application("MyEHR")
        .sending_facility("FAC001")
        .receiving_application("CAIR2")
        .receiving_facility("CAIR2")
        .query_tag("Q123456789")
        .patient_id("234567^^^MYEHR^MR")
        .patient_name("DOE^JANE")
        .build()
        .unwrap();

    let wire = built.encode();
    let parsed = parse_message(&wire).unwrap();

    assert_eq!(segment_ids(&parsed), vec!["MSH", "QPD", "RCP"]);
    assert_eq!(
        parsed.get_message_type(),
        Some(("QBP".to_string(), "Q11".to_string()))
    );
    let qpd = parsed.get_segment_by_id("QPD").unwrap();
    assert_eq!(qpd.get_field_value(2), Some("Q123456789"));
    assert_eq!(parsed.encode(), wire);
}

#[test]
fn qbp_q22_roundtrip() {
    let built = QbpQ22Builder::new(Version::V2_5_1)
        .sending_application("CLINREG")
        .receiving_application("HOSPMPI")
        .query_tag("987654321")
        .parameter("@PID.5.1^SMITH")
        .parameter("@PID.5.2^JOHN")
        .build()
        .unwrap();

    let wire = built.encode();
    let parsed = parse_message(&wire).unwrap();

    assert_eq!(segment_ids(&parsed), vec!["MSH", "QPD", "RCP"]);
    let qpd = parsed.get_segment_by_id("QPD").unwrap();
    assert_eq!(qpd.get_field_value(2), Some("987654321"));
    assert!(qpd.fields.len() >= 4);
    assert_eq!(parsed.encode(), wire);
}

#[test]
fn qbp_q21_roundtrip() {
    let built = QbpQ21Builder::new(Version::V2_5)
        .sending_application("APP")
        .sending_facility("FAC")
        .receiving_application("MPI")
        .receiving_facility("HOSP")
        .patient_id("12345^^^MPI^MR")
        .build()
        .unwrap();

    let wire = built.encode();
    let parsed = parse_message(&wire).unwrap();

    assert_eq!(segment_ids(&parsed), vec!["MSH", "QPD", "RCP"]);
    assert_eq!(
        parsed.get_message_type(),
        Some(("QBP".to_string(), "Q21".to_string()))
    );
    assert_eq!(parsed.encode(), wire);
}

#[test]
fn rsp_k11_roundtrip_with_data() {
    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_delimited("234567^^^MYEHR^MR"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_delimited("DOE^JANE"));

    let built = RspK11Builder::new(Version::V2_5_1)
        .sending_application("CAIR2")
        .sending_facility("CAIR2")
        .receiving_application("MyEHR")
        .receiving_facility("FAC001")
        .in_response_to("MSG-20231115-001")
        .query_tag("Q123456789")
        .query_response_status("OK")
        .hit_count(1)
        .add_segment(pid)
        .build()
        .unwrap();

    let wire = built.encode();
    let parsed = parse_message(&wire).unwrap();

    assert_eq!(
        segment_ids(&parsed),
        vec!["MSH", "MSA", "QAK", "QPD", "PID"]
    );
    assert_eq!(
        parsed.get_message_type(),
        Some(("RSP".to_string(), "K11".to_string()))
    );
    let pid = parsed.get_segment_by_id("PID").unwrap();
    assert_eq!(pid.get_field_value(1), Some("1"));
    // Composite PID-3/5 survive as structure, not escaped text.
    assert_eq!(
        pid.get_field(3)
            .unwrap()
            .get_component(0, 0)
            .unwrap()
            .value(),
        Some("234567")
    );
    assert_eq!(
        pid.get_field(5)
            .unwrap()
            .get_component(0, 1)
            .unwrap()
            .value(),
        Some("JANE")
    );
    assert_eq!(parsed.encode(), wire);
}

#[test]
fn rsp_k22_roundtrip_with_pagination() {
    let built = RspK22Builder::new(Version::V2_5_1)
        .sending_application("HOSPMPI")
        .receiving_application("CLINREG")
        .in_response_to("Q-20231115-045")
        .query_tag("987654321")
        .query_response_status("OK")
        .hit_counts(247, 2, 245)
        .build()
        .unwrap();

    let wire = built.encode();
    let parsed = parse_message(&wire).unwrap();

    assert_eq!(segment_ids(&parsed), vec!["MSH", "MSA", "QAK", "QPD"]);
    let qak = parsed.get_segment_by_id("QAK").unwrap();
    assert_eq!(qak.get_field_value(2), Some("OK"));
    assert_eq!(parsed.encode(), wire);
}

#[test]
fn rsp_k21_roundtrip_with_data() {
    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));

    let built = RspK21Builder::new(Version::V2_5)
        .sending_application("MPI")
        .receiving_application("APP")
        .in_response_to("MSG-002")
        .query_tag("TAG-001")
        .hit_count(1)
        .add_segment(pid)
        .build()
        .unwrap();

    let wire = built.encode();
    let parsed = parse_message(&wire).unwrap();

    assert_eq!(
        segment_ids(&parsed),
        vec!["MSH", "MSA", "QAK", "QPD", "PID"]
    );
    assert_eq!(parsed.encode(), wire);
}
