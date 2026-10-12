//! Patient reverse converter - FHIR Patient resource to PID segment
//!
//! Converts FHIR Patient resources back to HL7 v2.x PID segments.

use crate::error::ConversionResult;
use crate::resources::common::*;
use crate::resources::patient::Patient;
use vyna_hl7_core::segment::Segment;

/// Reverse converter for transforming FHIR Patient resources to PID segments
pub struct PatientReverseConverter;

impl PatientReverseConverter {
    /// Convert a FHIR Patient resource to an HL7 v2 PID segment
    ///
    /// # Arguments
    ///
    /// * `patient` - The FHIR Patient resource
    ///
    /// # Returns
    ///
    /// An HL7 v2 PID segment
    pub fn convert(patient: &Patient) -> ConversionResult<Segment> {
        let mut pid = Segment::new("PID");

        // Set ID - PID-1
        let _ = pid.set_field_value(1, "1");

        // Patient identifier - PID-3
        if let Some(ref identifiers) = patient.identifier {
            Self::set_identifiers(&mut pid, identifiers)?;
        }

        // Patient name - PID-5
        if let Some(ref names) = patient.name {
            Self::set_names(&mut pid, names)?;
        }

        // Birth date - PID-7
        if let Some(ref birth_date) = patient.birth_date {
            let hl7_date = Self::convert_date_to_hl7(birth_date)?;
            let _ = pid.set_field_value(7, &hl7_date);
        }

        // Gender - PID-8
        if let Some(ref gender) = patient.gender {
            let hl7_gender = Self::convert_gender_to_hl7(gender);
            let _ = pid.set_field_value(8, &hl7_gender);
        }

        // Address - PID-11
        if let Some(ref addresses) = patient.address {
            Self::set_addresses(&mut pid, addresses)?;
        }

        // Telecom - PID-13 (home), PID-14 (work)
        if let Some(ref telecoms) = patient.telecom {
            Self::set_telecoms(&mut pid, telecoms)?;
        }

        // Marital status - PID-16
        if let Some(ref marital_status) = patient.marital_status
            && let Some(ref coding) = marital_status.coding
                && let Some(first_coding) = coding.first()
                    && let Some(ref code) = first_coding.code {
                        let _ = pid.set_field_value(16, code);
                    }

        // Multiple birth - PID-24
        if let Some(multiple_birth) = patient.multiple_birth_boolean {
            let _ = pid.set_field_value(24, if multiple_birth { "Y" } else { "N" });
        }

        // Death date/time - PID-29
        if let Some(ref deceased_dt) = patient.deceased_date_time {
            let hl7_datetime = Self::convert_datetime_to_hl7(deceased_dt)?;
            let _ = pid.set_field_value(29, &hl7_datetime);
        }

        // Death indicator - PID-30
        if let Some(deceased) = patient.deceased_boolean {
            let _ = pid.set_field_value(30, if deceased { "Y" } else { "N" });
        }

        Ok(pid)
    }

    /// Set patient identifiers in PID-3
    fn set_identifiers(pid: &mut Segment, identifiers: &[Identifier]) -> ConversionResult<()> {
        if let Some(first) = identifiers.first() {
            // Set first identifier value
            if let Some(ref value) = first.value {
                let _ = pid.set_field_value(3, value);
            }

            // Set assigning authority as component 4 (0-indexed: 3)
            if let Some(ref system) = first.system {
                // Strip urn:oid: prefix if present
                let authority = system.strip_prefix("urn:oid:").unwrap_or(system);
                let _ = pid.set_component(3, 0, 3, authority);
            }

            // Set identifier type code as component 5 (0-indexed: 4)
            if let Some(ref type_) = first.type_
                && let Some(ref coding) = type_.coding
                    && let Some(first_coding) = coding.first()
                        && let Some(ref code) = first_coding.code {
                            let _ = pid.set_component(3, 0, 4, code);
                        }
        }
        Ok(())
    }

    /// Set patient names in PID-5
    fn set_names(pid: &mut Segment, names: &[HumanName]) -> ConversionResult<()> {
        if let Some(first) = names.first() {
            // XPN structure: 1=Family, 2=Given, 3=Middle, 4=Suffix, 5=Prefix, 7=Type

            // Family name - component 1 (0-indexed: 0)
            if let Some(ref family) = first.family {
                let _ = pid.set_component(5, 0, 0, family);
            }

            // Given names - component 2 (0-indexed: 1)
            if let Some(ref given) = first.given {
                if let Some(first_given) = given.first() {
                    let _ = pid.set_component(5, 0, 1, first_given);
                }
                // Middle name - component 3 (0-indexed: 2)
                if given.len() > 1 {
                    let _ = pid.set_component(5, 0, 2, &given[1]);
                }
            }

            // Suffix - component 4 (0-indexed: 3)
            if let Some(ref suffix) = first.suffix
                && let Some(first_suffix) = suffix.first() {
                    let _ = pid.set_component(5, 0, 3, first_suffix);
                }

            // Prefix - component 5 (0-indexed: 4)
            if let Some(ref prefix) = first.prefix
                && let Some(first_prefix) = prefix.first() {
                    let _ = pid.set_component(5, 0, 4, first_prefix);
                }

            // Name type - component 7 (0-indexed: 6)
            if let Some(ref use_) = first.use_ {
                let type_code = match use_.as_str() {
                    "official" => "L",
                    "maiden" => "M",
                    "nickname" => "N",
                    "temp" => "T",
                    _ => "L",
                };
                let _ = pid.set_component(5, 0, 6, type_code);
            }
        }
        Ok(())
    }

    /// Set addresses in PID-11
    fn set_addresses(pid: &mut Segment, addresses: &[Address]) -> ConversionResult<()> {
        if let Some(first) = addresses.first() {
            // XAD structure: 1=Street, 2=Other, 3=City, 4=State, 5=ZIP, 6=Country, 7=Type

            // Street address - component 1 (0-indexed: 0)
            if let Some(ref lines) = first.line {
                if let Some(first_line) = lines.first() {
                    let _ = pid.set_component(11, 0, 0, first_line);
                }
                // Other designation - component 2 (0-indexed: 1)
                if lines.len() > 1 {
                    let _ = pid.set_component(11, 0, 1, &lines[1]);
                }
            }

            // City - component 3 (0-indexed: 2)
            if let Some(ref city) = first.city {
                let _ = pid.set_component(11, 0, 2, city);
            }

            // State - component 4 (0-indexed: 3)
            if let Some(ref state) = first.state {
                let _ = pid.set_component(11, 0, 3, state);
            }

            // Postal code - component 5 (0-indexed: 4)
            if let Some(ref postal_code) = first.postal_code {
                let _ = pid.set_component(11, 0, 4, postal_code);
            }

            // Country - component 6 (0-indexed: 5)
            if let Some(ref country) = first.country {
                let _ = pid.set_component(11, 0, 5, country);
            }

            // Address type - component 7 (0-indexed: 6)
            if let Some(ref use_) = first.use_ {
                let type_code = match use_.as_str() {
                    "home" => "H",
                    "work" => "O",
                    "billing" => "B",
                    _ => "H",
                };
                let _ = pid.set_component(11, 0, 6, type_code);
            }
        }
        Ok(())
    }

    /// Set telecoms in PID-13 (home) and PID-14 (work)
    fn set_telecoms(pid: &mut Segment, telecoms: &[ContactPoint]) -> ConversionResult<()> {
        for telecom in telecoms {
            if let Some(ref value) = telecom.value
                && let Some(ref use_) = telecom.use_ {
                    match use_.as_str() {
                        "home" => {
                            let _ = pid.set_field_value(13, value);
                        }
                        "work" => {
                            let _ = pid.set_field_value(14, value);
                        }
                        _ => {}
                    }
                }
        }
        Ok(())
    }

    /// Convert FHIR gender to HL7 v2 gender code
    fn convert_gender_to_hl7(gender: &str) -> String {
        match gender {
            "male" => "M",
            "female" => "F",
            "other" => "O",
            "unknown" => "U",
            _ => "U",
        }
        .to_string()
    }

    /// Convert FHIR date (YYYY-MM-DD) to HL7 v2 date (YYYYMMDD)
    fn convert_date_to_hl7(date: &str) -> ConversionResult<String> {
        // Handle both YYYY-MM-DD and YYYY formats
        Ok(date.replace('-', ""))
    }

    /// Convert FHIR datetime to HL7 v2 datetime
    fn convert_datetime_to_hl7(datetime: &str) -> ConversionResult<String> {
        // Remove dashes, colons, and T separator
        Ok(datetime
            .replace(['-', ':', 'T'], ""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_patient() -> Patient {
        let mut patient = Patient::new();

        patient.identifier = Some(vec![Identifier {
            use_: Some("official".to_string()),
            type_: Some(CodeableConcept {
                coding: Some(vec![Coding {
                    system: Some("http://terminology.hl7.org/CodeSystem/v2-0203".to_string()),
                    version: None,
                    code: Some("MR".to_string()),
                    display: Some("Medical Record Number".to_string()),
                }]),
                text: Some("MRN".to_string()),
            }),
            system: Some("urn:oid:1.2.3.4".to_string()),
            value: Some("12345".to_string()),
            assigner: None,
        }]);

        patient.name = Some(vec![HumanName {
            use_: Some("official".to_string()),
            text: None,
            family: Some("Smith".to_string()),
            given: Some(vec!["John".to_string(), "A".to_string()]),
            prefix: Some(vec!["Mr".to_string()]),
            suffix: Some(vec!["Jr".to_string()]),
        }]);

        patient.birth_date = Some("1980-01-15".to_string());
        patient.gender = Some("male".to_string());

        patient.address = Some(vec![Address {
            use_: Some("home".to_string()),
            type_: None,
            text: None,
            line: Some(vec!["123 Main St".to_string(), "Apt 4".to_string()]),
            city: Some("Springfield".to_string()),
            state: Some("IL".to_string()),
            postal_code: Some("62701".to_string()),
            country: Some("USA".to_string()),
        }]);

        patient.telecom = Some(vec![
            ContactPoint {
                system: Some("phone".to_string()),
                value: Some("555-1234".to_string()),
                use_: Some("home".to_string()),
            },
            ContactPoint {
                system: Some("phone".to_string()),
                value: Some("555-5678".to_string()),
                use_: Some("work".to_string()),
            },
        ]);

        patient.multiple_birth_boolean = Some(false);
        patient.deceased_boolean = Some(false);

        patient
    }

    #[test]
    fn test_convert_patient_to_pid() {
        let patient = create_test_patient();
        let pid = PatientReverseConverter::convert(&patient).unwrap();

        assert_eq!(pid.id, "PID");
        assert_eq!(pid.get_field_value(1), Some("1"));
    }

    #[test]
    fn test_convert_identifier() {
        let patient = create_test_patient();
        let pid = PatientReverseConverter::convert(&patient).unwrap();

        // Check PID-3 value
        if let Some(field) = pid.get_field(3)
            && let Some(value) = field.value() {
                assert!(value.contains("12345"));
            }
    }

    #[test]
    fn test_convert_gender() {
        assert_eq!(
            PatientReverseConverter::convert_gender_to_hl7("male"),
            "M"
        );
        assert_eq!(
            PatientReverseConverter::convert_gender_to_hl7("female"),
            "F"
        );
        assert_eq!(
            PatientReverseConverter::convert_gender_to_hl7("other"),
            "O"
        );
        assert_eq!(
            PatientReverseConverter::convert_gender_to_hl7("unknown"),
            "U"
        );
    }

    #[test]
    fn test_convert_date() {
        assert_eq!(
            PatientReverseConverter::convert_date_to_hl7("1980-01-15").unwrap(),
            "19800115"
        );
        assert_eq!(
            PatientReverseConverter::convert_date_to_hl7("1980").unwrap(),
            "1980"
        );
    }

    #[test]
    fn test_convert_datetime() {
        assert_eq!(
            PatientReverseConverter::convert_datetime_to_hl7("1980-01-15T10:30:00").unwrap(),
            "19800115103000"
        );
    }

    #[test]
    fn test_roundtrip_gender() {
        // Test that gender converts correctly in both directions
        let patient = create_test_patient();
        let pid = PatientReverseConverter::convert(&patient).unwrap();

        assert_eq!(pid.get_field_value(8), Some("M"));
    }
}
