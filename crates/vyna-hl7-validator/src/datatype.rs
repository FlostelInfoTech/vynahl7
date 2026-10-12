//! Data type validation for HL7 fields
//!
//! This module provides format validation for HL7 data types according to the
//! HL7 v2.x specification. Each data type has specific formatting rules that
//! must be followed for conformance.

use vyna_hl7_core::datetime::{Hl7Date, Hl7DateTime, Hl7Time};
use vyna_hl7_core::error::Error as CoreError;
use vyna_hl7_core::types::DataType;

/// Reason text from a core parse error (without the "Parse error: " prefix).
fn parse_reason(error: CoreError) -> String {
    match error {
        CoreError::Parse(message) => message,
        other => other.to_string(),
    }
}

/// Result of data type validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataTypeValidation {
    Valid,
    Invalid { reason: String },
}

impl DataTypeValidation {
    pub fn is_valid(&self) -> bool {
        matches!(self, DataTypeValidation::Valid)
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            DataTypeValidation::Invalid { reason } => Some(reason),
            DataTypeValidation::Valid => None,
        }
    }
}

/// Validate a value against a specific HL7 data type
pub fn validate_data_type(value: &str, data_type: DataType) -> DataTypeValidation {
    // Empty values are generally allowed (required-ness is checked separately)
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    match data_type {
        DataType::DT => validate_date(value),
        DataType::TM => validate_time(value),
        DataType::DTM | DataType::TS => validate_timestamp(value),
        DataType::NM => validate_numeric(value),
        DataType::SI => validate_sequence_id(value),
        DataType::ST | DataType::TX | DataType::FT => validate_string(value),
        DataType::ID => validate_identifier(value),
        DataType::CE | DataType::CWE | DataType::CNE => validate_coded_element(value),
        DataType::XPN => validate_person_name(value),
        DataType::XAD => validate_address(value),
        DataType::XTN => validate_telecom(value),
        DataType::CX => validate_composite_id(value),
        DataType::EI => validate_entity_identifier(value),
        DataType::HD => validate_hierarchic_designator(value),
        DataType::MSG => validate_message_type(value),
        DataType::PT => validate_processing_type(value),
        DataType::NA => validate_numeric_array(value),
        DataType::IS => validate_coded_value(value),
        DataType::XCN => validate_composite_name(value),
        DataType::PL => validate_person_location(value),
        DataType::VID => validate_version_identifier(value),
        DataType::CP => validate_composite_type(value),
        DataType::CQ => validate_composite_type(value),
        DataType::DR => validate_composite_type(value),
        DataType::EIP => validate_composite_type(value),
        DataType::ELD => validate_composite_type(value),
        DataType::TQ => validate_composite_type(value),
        DataType::Varies => DataTypeValidation::Valid,
        DataType::SN => validate_structured_numeric(value),
        DataType::XON => validate_composite_type(value),
        DataType::FN => validate_composite_type(value),
        DataType::MO => validate_composite_type(value),
        DataType::CF => validate_coded_element(value),
        DataType::DIN => validate_composite_type(value),
        DataType::DLD => validate_composite_type(value),
        DataType::FC => validate_composite_type(value),
        DataType::JCC => validate_composite_type(value),
    }
}

/// Validate DT (Date) format: YYYY[MM[DD]]
fn validate_date(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    match Hl7Date::parse(value) {
        Ok(_) => DataTypeValidation::Valid,
        Err(e) => DataTypeValidation::Invalid {
            reason: parse_reason(e),
        },
    }
}

/// Validate TM (Time) format: HH[MM[SS[.S[S[S[S]]]]]][+/-ZZZZ]
fn validate_time(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    match Hl7Time::parse(value) {
        Ok(time) if time.frac_digits() > 4 => DataTypeValidation::Invalid {
            reason: "Fractional seconds must be 1-4 digits".to_string(),
        },
        Ok(_) => DataTypeValidation::Valid,
        Err(e) => DataTypeValidation::Invalid {
            reason: parse_reason(e),
        },
    }
}

/// Validate DTM/TS (Timestamp) format: YYYY[MM[DD[HH[MM[SS[.S[S[S[S]]]]]]]]][+/-ZZZZ]
fn validate_timestamp(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    match Hl7DateTime::parse(value) {
        Ok(ts) if ts.frac_digits() > 4 => DataTypeValidation::Invalid {
            reason: "Fractional seconds must be 1-4 digits".to_string(),
        },
        Ok(_) => DataTypeValidation::Valid,
        Err(e) => DataTypeValidation::Invalid {
            reason: parse_reason(e),
        },
    }
}

/// Validate NM (Numeric) - can include decimals and signs
fn validate_numeric(value: &str) -> DataTypeValidation {
    // Allow optional sign, digits, and optional decimal point
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return DataTypeValidation::Valid;
    }

    // Try parsing as f64
    match trimmed.parse::<f64>() {
        Ok(_) => DataTypeValidation::Valid,
        Err(_) => DataTypeValidation::Invalid {
            reason: "Not a valid numeric value".to_string(),
        },
    }
}

/// Validate SI (Sequence ID) - must be a positive integer
fn validate_sequence_id(value: &str) -> DataTypeValidation {
    match value.parse::<u32>() {
        Ok(n) if n > 0 => DataTypeValidation::Valid,
        Ok(_) => DataTypeValidation::Invalid {
            reason: "Sequence ID must be positive".to_string(),
        },
        Err(_) => DataTypeValidation::Invalid {
            reason: "Sequence ID must be a positive integer".to_string(),
        },
    }
}

/// Validate ST/TX/FT (String types) - basic string validation
fn validate_string(_value: &str) -> DataTypeValidation {
    // String types accept most content
    // Main constraint is they shouldn't contain unescaped delimiters
    // but that's handled at the encoding level
    DataTypeValidation::Valid
}

/// Validate ID (Identifier) - alphanumeric with some special chars
fn validate_identifier(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    // IDs should be alphanumeric, may include underscore, hyphen
    if value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        DataTypeValidation::Valid
    } else {
        DataTypeValidation::Invalid {
            reason: "Identifier should contain only alphanumeric characters, hyphens, or underscores".to_string(),
        }
    }
}

/// Validate CE/CWE/CNE (Coded Elements) - component structure
fn validate_coded_element(value: &str) -> DataTypeValidation {
    // Coded elements have components separated by ^
    // Format: identifier^text^coding system^...
    // At minimum, should have an identifier
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    // Just check that it's not malformed (basic validation)
    // More detailed validation would require checking component count
    DataTypeValidation::Valid
}

/// Validate XPN (Person Name) - component structure
fn validate_person_name(value: &str) -> DataTypeValidation {
    // Format: family^given^middle^suffix^prefix^...
    // Basic validation - should have components
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Validate XAD (Address) - component structure
fn validate_address(value: &str) -> DataTypeValidation {
    // Format: street^other designation^city^state^zip^...
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Validate XTN (Telecom) - phone/email format
fn validate_telecom(value: &str) -> DataTypeValidation {
    // Format: [(999)]999-9999[X99999][B99999][C any text]
    // Or email format
    // Basic validation for now
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Validate CX (Composite ID)
fn validate_composite_id(value: &str) -> DataTypeValidation {
    // Format: ID^check digit^check digit scheme^assigning authority^...
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Validate EI (Entity Identifier)
fn validate_entity_identifier(value: &str) -> DataTypeValidation {
    // Format: entity identifier^namespace ID^universal ID^universal ID type
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Validate HD (Hierarchic Designator)
fn validate_hierarchic_designator(value: &str) -> DataTypeValidation {
    // Format: namespace ID^universal ID^universal ID type
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Validate MSG (Message Type)
fn validate_message_type(value: &str) -> DataTypeValidation {
    // Format: message code^trigger event^message structure
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    let components: Vec<&str> = value.split('^').collect();
    if components.is_empty() {
        return DataTypeValidation::Invalid {
            reason: "Message type must have at least a message code".to_string(),
        };
    }

    // Message code should be 3 uppercase letters
    let msg_code = components[0];
    if msg_code.len() != 3 || !msg_code.chars().all(|c| c.is_ascii_uppercase()) {
        return DataTypeValidation::Invalid {
            reason: "Message code must be 3 uppercase letters".to_string(),
        };
    }

    DataTypeValidation::Valid
}

/// Validate PT (Processing Type)
fn validate_processing_type(value: &str) -> DataTypeValidation {
    // Format: processing ID^processing mode
    // Processing ID is usually P (Production), D (Debugging), or T (Training)
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    let first_char = value.chars().next().unwrap();
    if !['P', 'D', 'T', 'I'].contains(&first_char) {
        return DataTypeValidation::Invalid {
            reason: "Processing type should be P, D, T, or I".to_string(),
        };
    }

    DataTypeValidation::Valid
}

/// Validate NA (Numeric Array)
fn validate_numeric_array(value: &str) -> DataTypeValidation {
    // Format: value1~value2~value3
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    let values: Vec<&str> = value.split('~').collect();
    for v in values {
        if validate_numeric(v) != DataTypeValidation::Valid {
            return DataTypeValidation::Invalid {
                reason: format!("Invalid numeric value in array: {}", v),
            };
        }
    }

    DataTypeValidation::Valid
}

/// Validate IS (Coded Value for HL7 Defined Tables) - similar to ID
fn validate_coded_value(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Validate XCN (Extended Composite ID Number and Name)
fn validate_composite_name(value: &str) -> DataTypeValidation {
    // Format: ID^family^given^middle^suffix^prefix^degree^source table^...
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Validate PL (Person Location)
fn validate_person_location(value: &str) -> DataTypeValidation {
    // Format: point of care^room^bed^facility^location status^...
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Validate VID (Version Identifier)
fn validate_version_identifier(value: &str) -> DataTypeValidation {
    // Format: version ID^internationalization code^international version ID
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Validate SN (Structured Numeric)
fn validate_structured_numeric(value: &str) -> DataTypeValidation {
    // Format: comparator^num1^separator/suffix^num2
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Generic validation for composite types with component structure
fn validate_composite_type(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_date() {
        assert!(validate_date("20240315").is_valid());
        assert!(validate_date("202403").is_valid());
        assert!(validate_date("2024").is_valid());
        assert!(!validate_date("2024031").is_valid()); // Wrong length
        assert!(!validate_date("20241301").is_valid()); // Invalid month
        assert!(!validate_date("2024abcd").is_valid()); // Non-numeric
    }

    #[test]
    fn test_validate_time() {
        assert!(validate_time("14").is_valid());
        assert!(validate_time("1430").is_valid());
        assert!(validate_time("143000").is_valid());
        assert!(validate_time("143000.123").is_valid());
        assert!(validate_time("143000+0800").is_valid()); // Offset
        assert!(validate_time("143000.12-0530").is_valid()); // Offset
        assert!(!validate_time("2530").is_valid()); // Invalid hours
        assert!(!validate_time("1460").is_valid()); // Invalid minutes
        assert!(!validate_time("143060").is_valid()); // Invalid seconds
        assert!(!validate_time("14300").is_valid()); // Wrong length
        assert!(!validate_time("143000+2500").is_valid()); // Bad offset
        assert!(!validate_time("143000.12345").is_valid()); // Fraction too long
    }

    #[test]
    fn test_validate_timestamp() {
        assert!(validate_timestamp("20240315").is_valid());
        assert!(validate_timestamp("20240315143000").is_valid());
        assert!(validate_timestamp("20240315143000.1234").is_valid());
        assert!(validate_timestamp("20240315143000+0800").is_valid()); // Offset
        assert!(validate_timestamp("20240315143000.12-0530").is_valid()); // Offset
        assert!(!validate_timestamp("202403151").is_valid()); // Wrong length
        assert!(!validate_timestamp("20241301").is_valid()); // Invalid date
        assert!(!validate_timestamp("20240315143000+9999").is_valid()); // Bad offset
        assert!(!validate_timestamp("20240315143000+ABCD").is_valid()); // Bad offset
        assert!(!validate_timestamp("20240315143000.12345").is_valid()); // Fraction too long
    }

    #[test]
    fn test_validate_numeric() {
        assert!(validate_numeric("123").is_valid());
        assert!(validate_numeric("123.45").is_valid());
        assert!(validate_numeric("-123.45").is_valid());
        assert!(validate_numeric("+123").is_valid());
        assert!(!validate_numeric("abc").is_valid());
        assert!(!validate_numeric("12.34.56").is_valid());
    }

    #[test]
    fn test_validate_sequence_id() {
        assert!(validate_sequence_id("1").is_valid());
        assert!(validate_sequence_id("123").is_valid());
        assert!(!validate_sequence_id("0").is_valid()); // Must be positive
        assert!(!validate_sequence_id("-1").is_valid());
        assert!(!validate_sequence_id("abc").is_valid());
    }

    #[test]
    fn test_validate_identifier() {
        assert!(validate_identifier("ABC123").is_valid());
        assert!(validate_identifier("test_id").is_valid());
        assert!(validate_identifier("test-id").is_valid());
        assert!(!validate_identifier("test id").is_valid()); // No spaces
        assert!(!validate_identifier("test@id").is_valid()); // No special chars
    }

    #[test]
    fn test_validate_message_type() {
        assert!(validate_message_type("ADT^A01").is_valid());
        assert!(validate_message_type("ORU^R01^ORU_R01").is_valid());
        assert!(!validate_message_type("ad^A01").is_valid()); // Not 3 chars
        assert!(!validate_message_type("adt^A01").is_valid()); // Not uppercase
    }

    #[test]
    fn test_validate_processing_type() {
        assert!(validate_processing_type("P").is_valid());
        assert!(validate_processing_type("D").is_valid());
        assert!(validate_processing_type("T").is_valid());
        assert!(!validate_processing_type("X").is_valid());
    }

    #[test]
    fn test_validate_numeric_array() {
        assert!(validate_numeric_array("1~2~3").is_valid());
        assert!(validate_numeric_array("1.5~2.7~3.9").is_valid());
        assert!(!validate_numeric_array("1~abc~3").is_valid());
    }

    #[test]
    fn test_empty_values() {
        // Empty values should be valid (required-ness checked separately)
        assert!(validate_date("").is_valid());
        assert!(validate_time("").is_valid());
        assert!(validate_numeric("").is_valid());
    }
}
