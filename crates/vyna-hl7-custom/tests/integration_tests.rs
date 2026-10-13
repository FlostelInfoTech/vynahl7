//! Integration tests for vyna-hl7-custom Z-segment support
//!
//! These tests verify end-to-end functionality of custom Z-segments
//! including parsing, manipulation, and message integration.

use vyna_hl7_custom::{z_segment, CustomSegment, CustomSegmentRegistry, MessageExt};
use vyna_hl7_parser::parse_message;

// Test segments for integration testing

z_segment! {
    IntegrationZPV,
    id = "ZPV",
    fields = {
        1 => visit_type: String,
        2 => visit_number: String,
        3 => patient_class: Option<String>,
        4 => department_code: Option<String>,
    }
}

z_segment! {
    IntegrationZCU,
    id = "ZCU",
    fields = {
        1 => customer_id: String,
        2 => account_number: String,
        3 => balance: Option<f64>,
        4 => credit_limit: Option<f64>,
    },
    validate = |s: &IntegrationZCU| {
        if let Some(balance) = s.balance
            && balance < 0.0 {
                return Err(vyna_hl7_custom::CustomSegmentError::validation_failed(
                    "ZCU",
                    "Balance cannot be negative"
                ));
            }
        if let Some(credit_limit) = s.credit_limit
            && credit_limit < 0.0 {
                return Err(vyna_hl7_custom::CustomSegmentError::validation_failed(
                    "ZCU",
                    "Credit limit cannot be negative"
                ));
            }
        Ok(())
    }
}

z_segment! {
    IntegrationZLO,
    id = "ZLO",
    fields = {
        1 => building_code: String,
        2 => floor_number: u32,
        3 => wing: Option<String>,
        4 => room_type: Option<String>,
    }
}

#[test]
fn test_end_to_end_single_z_segment() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG001|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       PV1|1|O|CARDIO^101^1||||DR^SMITH^JOHN\r\
                       ZPV|OUTPATIENT|V202401-12345|O|CARDIO";

    let message = parse_message(hl7_message).expect("Failed to parse message");

    // Extract ZPV segment
    let zpv = message
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");

    assert_eq!(zpv.visit_type, "OUTPATIENT");
    assert_eq!(zpv.visit_number, "V202401-12345");
    assert_eq!(zpv.patient_class, Some("O".to_string()));
    assert_eq!(zpv.department_code, Some("CARDIO".to_string()));
}

#[test]
fn test_end_to_end_multiple_same_z_segments() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG002|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       ZPV|OUTPATIENT|V001|O|CARDIO\r\
                       ZPV|EMERGENCY|V002|E|ER\r\
                       ZPV|INPATIENT|V003|I|ICU";

    let message = parse_message(hl7_message).expect("Failed to parse message");

    // Get all ZPV segments
    let zpvs = message
        .get_custom_segments::<IntegrationZPV>()
        .expect("Failed to get ZPVs");

    assert_eq!(zpvs.len(), 3);
    assert_eq!(zpvs[0].visit_type, "OUTPATIENT");
    assert_eq!(zpvs[1].visit_type, "EMERGENCY");
    assert_eq!(zpvs[2].visit_type, "INPATIENT");
}

#[test]
fn test_end_to_end_different_z_segments() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG003|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       PV1|1|I|NORTH^3^301||||DR^JONES^MARY\r\
                       ZPV|INPATIENT|V12345|I|MED\r\
                       ZLO|NORTH|3|WEST|PRIVATE\r\
                       ZCU|CUST001|ACC-2024-001|1500.75|5000.00";

    let message = parse_message(hl7_message).expect("Failed to parse message");

    // Extract each Z-segment type
    let zpv = message
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");
    assert_eq!(zpv.visit_type, "INPATIENT");

    let zlo = message
        .get_custom_segment::<IntegrationZLO>()
        .expect("Failed to get ZLO")
        .expect("ZLO not found");
    assert_eq!(zlo.building_code, "NORTH");
    assert_eq!(zlo.floor_number, 3);

    let zcu = message
        .get_custom_segment::<IntegrationZCU>()
        .expect("Failed to get ZCU")
        .expect("ZCU not found");
    assert_eq!(zcu.customer_id, "CUST001");
    assert_eq!(zcu.balance, Some(1500.75));
}

#[test]
fn test_message_manipulation_add_segment() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG004|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M";

    let mut message = parse_message(hl7_message).expect("Failed to parse message");

    // Initially no ZPV
    assert!(!message.has_custom_segment::<IntegrationZPV>());

    // Add ZPV
    let zpv = IntegrationZPV::builder()
        .visit_type("EMERGENCY")
        .visit_number("V99999")
        .patient_class("E")
        .build()
        .expect("Failed to build ZPV");

    message.add_custom_segment(zpv);

    // Now has ZPV
    assert!(message.has_custom_segment::<IntegrationZPV>());
    let retrieved = message
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");
    assert_eq!(retrieved.visit_type, "EMERGENCY");
}

#[test]
fn test_message_manipulation_replace_segment() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG005|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       ZPV|OUTPATIENT|V001|O|CARDIO";

    let mut message = parse_message(hl7_message).expect("Failed to parse message");

    // Verify original
    let original = message
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");
    assert_eq!(original.visit_type, "OUTPATIENT");

    // Replace with new ZPV
    let new_zpv = IntegrationZPV::builder()
        .visit_type("EMERGENCY")
        .visit_number("V002")
        .patient_class("E")
        .build()
        .expect("Failed to build ZPV");

    message
        .set_custom_segment(new_zpv)
        .expect("Failed to set ZPV");

    // Verify replacement
    let replaced = message
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");
    assert_eq!(replaced.visit_type, "EMERGENCY");
    assert_eq!(replaced.visit_number, "V002");

    // Should still be only one ZPV
    let all = message
        .get_custom_segments::<IntegrationZPV>()
        .expect("Failed to get ZPVs");
    assert_eq!(all.len(), 1);
}

#[test]
fn test_message_manipulation_remove_segments() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG006|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       ZPV|OUTPATIENT|V001|O|CARDIO\r\
                       ZPV|EMERGENCY|V002|E|ER\r\
                       ZLO|NORTH|3|WEST|PRIVATE";

    let mut message = parse_message(hl7_message).expect("Failed to parse message");

    // Verify initial state
    assert_eq!(message.get_custom_segments::<IntegrationZPV>().unwrap().len(), 2);
    assert!(message.has_custom_segment::<IntegrationZLO>());

    // Remove ZPV segments
    let removed = message.remove_custom_segments::<IntegrationZPV>();
    assert_eq!(removed, 2);

    // Verify removal
    assert!(!message.has_custom_segment::<IntegrationZPV>());
    assert_eq!(message.get_custom_segments::<IntegrationZPV>().unwrap().len(), 0);

    // ZLO should still be there
    assert!(message.has_custom_segment::<IntegrationZLO>());
}

#[test]
fn test_validation_on_parse() {
    // Invalid ZCU with negative balance
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||DFT^P03|MSG007|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       ZCU|CUST001|ACC-001|-100.50|5000.00";

    let message = parse_message(hl7_message).expect("Failed to parse message");

    // Should fail validation when trying to extract
    let result = message.get_custom_segment::<IntegrationZCU>();
    assert!(result.is_err());
}

#[test]
fn test_validation_on_build() {
    // Try to build with invalid data
    let result = IntegrationZCU::builder()
        .customer_id("CUST002")
        .account_number("ACC-002")
        .balance(-50.0)
        .build();

    assert!(result.is_err());
}

#[test]
fn test_set_custom_segment_rejects_invalid() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||DFT^P03|MSG008|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M";

    let mut message = parse_message(hl7_message).expect("Failed to parse message");

    // The builder refuses invalid data, so construct it directly.
    let invalid = IntegrationZCU {
        customer_id: "CUST001".to_string(),
        account_number: "ACC-001".to_string(),
        balance: Some(-100.50),
        credit_limit: Some(5000.00),
    };

    // set_custom_segment validates: rejected, message unchanged.
    assert!(message.set_custom_segment(invalid).is_err());
    assert!(!message.has_custom_segment::<IntegrationZCU>());
}

#[test]
fn test_builder_missing_required_field() {
    // Try to build without required field
    let result = IntegrationZPV::builder()
        .visit_type("OUTPATIENT")
        // missing visit_number
        .build();

    assert!(result.is_err());
}

#[test]
fn test_builder_with_all_optional_fields() {
    let zpv = IntegrationZPV::builder()
        .visit_type("OUTPATIENT")
        .visit_number("V12345")
        .patient_class("O")
        .department_code("CARDIO")
        .build()
        .expect("Failed to build ZPV");

    assert_eq!(zpv.patient_class, Some("O".to_string()));
    assert_eq!(zpv.department_code, Some("CARDIO".to_string()));
}

#[test]
fn test_builder_without_optional_fields() {
    let zpv = IntegrationZPV::builder()
        .visit_type("EMERGENCY")
        .visit_number("V99999")
        .build()
        .expect("Failed to build ZPV");

    assert_eq!(zpv.patient_class, None);
    assert_eq!(zpv.department_code, None);
}

#[test]
fn test_numeric_type_u32() {
    let zlo = IntegrationZLO::builder()
        .building_code("NORTH")
        .floor_number(15u32)
        .build()
        .expect("Failed to build ZLO");

    assert_eq!(zlo.floor_number, 15);

    // Convert to segment and back
    let segment = zlo.to_segment();
    let parsed = IntegrationZLO::from_segment(&segment).expect("Failed to parse ZLO");
    assert_eq!(parsed.floor_number, 15);
}

#[test]
fn test_numeric_type_f64() {
    let zcu = IntegrationZCU::builder()
        .customer_id("CUST001")
        .account_number("ACC-001")
        .balance(1234.56)
        .credit_limit(9999.99)
        .build()
        .expect("Failed to build ZCU");

    assert_eq!(zcu.balance, Some(1234.56));
    assert_eq!(zcu.credit_limit, Some(9999.99));

    // Convert to segment and back
    let segment = zcu.to_segment();
    let parsed = IntegrationZCU::from_segment(&segment).expect("Failed to parse ZCU");
    assert_eq!(parsed.balance, Some(1234.56));
    assert_eq!(parsed.credit_limit, Some(9999.99));
}

#[test]
fn test_segment_round_trip() {
    let original = IntegrationZPV::builder()
        .visit_type("INPATIENT")
        .visit_number("V777")
        .patient_class("I")
        .department_code("SURG")
        .build()
        .expect("Failed to build ZPV");

    // Convert to segment
    let segment = original.to_segment();
    assert_eq!(segment.id, "ZPV");

    // Convert back
    let parsed = IntegrationZPV::from_segment(&segment).expect("Failed to parse ZPV");

    // Should be identical
    assert_eq!(parsed.visit_type, original.visit_type);
    assert_eq!(parsed.visit_number, original.visit_number);
    assert_eq!(parsed.patient_class, original.patient_class);
    assert_eq!(parsed.department_code, original.department_code);
}

#[test]
fn test_message_encoding_with_z_segments() {
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG008|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       ZPV|OUTPATIENT|V12345|O|CARDIO";

    let message = parse_message(hl7_message).expect("Failed to parse message");

    // Encode the message
    let encoded = message.encode();

    // Should contain the ZPV segment
    assert!(encoded.contains("ZPV|OUTPATIENT|V12345|O|CARDIO"));

    // Re-parse and verify
    let reparsed = parse_message(&encoded).expect("Failed to re-parse message");
    let zpv = reparsed
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");
    assert_eq!(zpv.visit_type, "OUTPATIENT");
}

#[test]
fn test_registry_is_registered() {
    // Note: Using a new registry instance to avoid conflicts with global state
    let registry = CustomSegmentRegistry::new();

    assert!(!registry.is_registered("ZPV"));

    registry.register::<IntegrationZPV>().expect("Failed to register ZPV");

    assert!(registry.is_registered("ZPV"));
}

#[test]
fn test_registry_registered_ids() {
    let registry = CustomSegmentRegistry::new();

    registry.register::<IntegrationZPV>().expect("Failed to register ZPV");
    registry.register::<IntegrationZCU>().expect("Failed to register ZCU");

    let ids = registry.registered_ids();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"ZPV".to_string()));
    assert!(ids.contains(&"ZCU".to_string()));
}

#[test]
fn test_complex_workflow() {
    // Parse message with Z-segments
    let hl7_message = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG009|P|2.5\r\
                       PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                       PV1|1|O|CARDIO^101^1||||DR^SMITH^JOHN\r\
                       ZPV|OUTPATIENT|V001|O|CARDIO";

    let mut message = parse_message(hl7_message).expect("Failed to parse message");

    // Extract and verify ZPV
    let zpv = message
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");
    assert_eq!(zpv.visit_type, "OUTPATIENT");

    // Add location information
    let zlo = IntegrationZLO::builder()
        .building_code("NORTH")
        .floor_number(5u32)
        .wing("WEST")
        .room_type("PRIVATE")
        .build()
        .expect("Failed to build ZLO");
    message.add_custom_segment(zlo);

    // Add financial information
    let zcu = IntegrationZCU::builder()
        .customer_id("CUST12345")
        .account_number("ACC-2024-001")
        .balance(500.00)
        .credit_limit(2000.00)
        .build()
        .expect("Failed to build ZCU");
    message.add_custom_segment(zcu);

    // Verify all segments present
    assert!(message.has_custom_segment::<IntegrationZPV>());
    assert!(message.has_custom_segment::<IntegrationZLO>());
    assert!(message.has_custom_segment::<IntegrationZCU>());

    // Update the visit information
    let new_zpv = IntegrationZPV::builder()
        .visit_type("EMERGENCY")
        .visit_number("V002")
        .patient_class("E")
        .department_code("ER")
        .build()
        .expect("Failed to build ZPV");
    message.set_custom_segment(new_zpv).expect("Failed to set ZPV");

    // Verify update
    let updated_zpv = message
        .get_custom_segment::<IntegrationZPV>()
        .expect("Failed to get ZPV")
        .expect("ZPV not found");
    assert_eq!(updated_zpv.visit_type, "EMERGENCY");
    assert_eq!(updated_zpv.visit_number, "V002");

    // Encode and verify structure
    let encoded = message.encode();
    assert!(encoded.contains("ZPV|EMERGENCY|V002|E|ER"));
    assert!(encoded.contains("ZLO|NORTH|5|WEST|PRIVATE"));
    assert!(encoded.contains("ZCU|CUST12345|ACC-2024-001|500|2000"));
}
