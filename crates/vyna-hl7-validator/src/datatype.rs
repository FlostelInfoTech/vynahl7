//! Data type validation for HL7 fields
//!
//! This module provides format validation for HL7 data types according to the
//! HL7 v2.x specification. Each data type has specific formatting rules that
//! must be followed for conformance.
//!
//! Composite data types (CE, XPN, XAD, XTN, CX, and the other
//! multi-component types) intentionally accept any scalar here:
//! component structure is enforced by schema component definitions
//! from the parsed field (see `Validator`), because these functions
//! only ever see one decoded scalar, in which separator characters
//! are data rather than structure.

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
    /// The value matches the data type format
    Valid,
    /// The value does not match the data type format
    Invalid {
        /// Why the value failed
        reason: String,
    },
}

impl DataTypeValidation {
    /// True when the value matches the data type format
    pub fn is_valid(&self) -> bool {
        matches!(self, DataTypeValidation::Valid)
    }

    /// Why the value failed, or `None` when it passed
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
        // Legacy pre-2.3 types share their successors' shapes
        DataType::PN => validate_person_name(value),
        DataType::AD => validate_address(value),
        DataType::TN => validate_telecom(value),
        DataType::CN => validate_composite_name(value),
        DataType::CK => validate_composite_id(value),
        DataType::RI | DataType::CM => validate_composite_type(value),
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

/// Validate NM (Numeric): `[+-]?` digits with at most one decimal point
///
/// This is a grammar check, deliberately not `f64` parsing: `f64`
/// accepts `NaN`, infinities, and exponents (`1e5`), none of which
/// are valid HL7 NM values.
fn validate_numeric(value: &str) -> DataTypeValidation {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return DataTypeValidation::Valid;
    }

    let digits = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    let mut seen_dot = false;
    let mut seen_digit = false;
    let mut valid = !digits.is_empty();
    for c in digits.chars() {
        if c == '.' {
            if seen_dot {
                valid = false;
                break;
            }
            seen_dot = true;
        } else if c.is_ascii_digit() {
            seen_digit = true;
        } else {
            valid = false;
            break;
        }
    }

    if valid && seen_digit {
        DataTypeValidation::Valid
    } else {
        DataTypeValidation::Invalid {
            reason: "Not a valid numeric value".to_string(),
        }
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

    // IDs should be alphanumeric, may include underscore, hyphen, dot
    // (dotted values are legitimate: HL7 table 0104 holds "2.1".."2.8.2").
    if value
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        DataTypeValidation::Valid
    } else {
        DataTypeValidation::Invalid {
            reason: "Identifier should contain only alphanumeric characters, hyphens, underscores, or dots"
                .to_string(),
        }
    }
}

/// Accept any scalar for CE/CWE/CNE/CF (Coded Elements)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_coded_element(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Accept any scalar for XPN/PN (Person Name)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_person_name(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Accept any scalar for XAD/AD (Address)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_address(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Accept any scalar for XTN/TN (Telecom)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_telecom(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Accept any scalar for CX/CK (Composite ID)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_composite_id(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Accept any scalar for EI (Entity Identifier)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_entity_identifier(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }

    DataTypeValidation::Valid
}

/// Accept any scalar for HD (Hierarchic Designator)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_hierarchic_designator(value: &str) -> DataTypeValidation {
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

/// Accept any scalar for IS (Coded Value for HL7 Defined Tables)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_coded_value(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Accept any scalar for XCN/CN (Extended Composite ID Number and Name)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_composite_name(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Accept any scalar for PL (Person Location)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_person_location(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Accept any scalar for VID (Version Identifier)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_version_identifier(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Accept any scalar for SN (Structured Numeric)
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
fn validate_structured_numeric(value: &str) -> DataTypeValidation {
    if value.is_empty() {
        return DataTypeValidation::Valid;
    }
    DataTypeValidation::Valid
}

/// Accept any scalar for the remaining composite types
///
/// Component structure is enforced by schema component definitions
/// from the parsed field (see `Validator`); this layer only ever
/// sees one decoded scalar, in which separators are data.
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
    fn test_validate_numeric_rejects_non_nm_float_forms() {
        // f64 parses these, but the HL7 NM grammar (optional sign,
        // digits, one decimal point) does not allow them.
        for bad in ["NaN", "inf", "-inf", "+Infinity", "1e5", "1E-3"] {
            assert!(!validate_numeric(bad).is_valid(), "{bad}");
        }
        // Grammar edge cases: a sign or point alone is not a number.
        for bad in ["+", "-", ".", "+.", "1.2.3", "12a", ""] {
            if bad.is_empty() {
                assert!(validate_numeric(bad).is_valid());
            } else {
                assert!(!validate_numeric(bad).is_valid(), "{bad}");
            }
        }
        // Still valid: leading/trailing-point decimals and signed values.
        for good in ["0", "-0", "+0.5", ".5", "5.", "007", "123456789.987654321"] {
            assert!(validate_numeric(good).is_valid(), "{good}");
        }
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
        assert!(validate_identifier("2.8.2").is_valid()); // Dotted IDs (table 0104)
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

    #[test]
    fn test_composite_validators_accept_any_scalar_by_design() {
        // Composite checks intentionally accept any scalar: component
        // structure is enforced by schema component definitions from the
        // parsed field, while these functions only ever see one decoded
        // scalar, in which separators are data rather than structure.
        for value in ["DOE^JOHN", "^^^^", "anything at all"] {
            assert!(validate_coded_element(value).is_valid());
            assert!(validate_person_name(value).is_valid());
            assert!(validate_address(value).is_valid());
            assert!(validate_telecom(value).is_valid());
            assert!(validate_composite_id(value).is_valid());
            assert!(validate_composite_type(value).is_valid());
        }
    }
}
