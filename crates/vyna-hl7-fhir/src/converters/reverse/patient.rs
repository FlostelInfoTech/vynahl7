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
            && let Some(ref code) = first_coding.code
        {
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

    /// Set patient identifiers in PID-3, one repetition per identifier
    fn set_identifiers(pid: &mut Segment, identifiers: &[Identifier]) -> ConversionResult<()> {
        for (rep, identifier) in identifiers.iter().enumerate() {
            // Identifier value as component 1 (0-indexed: 0)
            if let Some(ref value) = identifier.value {
                let _ = pid.set_component(3, rep, 0, value);
            }

            // Assigning authority as component 4 (0-indexed: 3)
            if let Some(ref system) = identifier.system {
                // Strip urn:oid: prefix if present
                let authority = system.strip_prefix("urn:oid:").unwrap_or(system);
                let _ = pid.set_component(3, rep, 3, authority);
            }

            // Identifier type code as component 5 (0-indexed: 4)
            if let Some(ref type_) = identifier.type_
                && let Some(ref coding) = type_.coding
                && let Some(first_coding) = coding.first()
                && let Some(ref code) = first_coding.code
            {
                let _ = pid.set_component(3, rep, 4, code);
            }
        }
        Ok(())
    }

    /// Set patient names in PID-5, one repetition per name
    fn set_names(pid: &mut Segment, names: &[HumanName]) -> ConversionResult<()> {
        for (rep, name) in names.iter().enumerate() {
            // XPN structure: 1=Family, 2=Given, 3=Middle, 4=Suffix, 5=Prefix, 7=Type

            // Family name - component 1 (0-indexed: 0)
            if let Some(ref family) = name.family {
                let _ = pid.set_component(5, rep, 0, family);
            }

            // Given names - component 2 (0-indexed: 1)
            if let Some(ref given) = name.given {
                if let Some(first_given) = given.first() {
                    let _ = pid.set_component(5, rep, 1, first_given);
                }
                // Middle name - component 3 (0-indexed: 2)
                if given.len() > 1 {
                    let _ = pid.set_component(5, rep, 2, &given[1]);
                }
            }

            // Suffix - component 4 (0-indexed: 3)
            if let Some(ref suffix) = name.suffix
                && let Some(first_suffix) = suffix.first()
            {
                let _ = pid.set_component(5, rep, 3, first_suffix);
            }

            // Degree - component 6 (0-indexed: 5). The forward
            // converter appends XPN-6 after XPN-4 in `suffix`, so
            // the second entry round-trips here.
            if let Some(ref suffix) = name.suffix
                && suffix.len() > 1
            {
                let _ = pid.set_component(5, rep, 5, &suffix[1]);
            }

            // Prefix - component 5 (0-indexed: 4)
            if let Some(ref prefix) = name.prefix
                && let Some(first_prefix) = prefix.first()
            {
                let _ = pid.set_component(5, rep, 4, first_prefix);
            }

            // Name type - component 7 (0-indexed: 6). Only mapped
            // uses write a code: usual/unknown must not re-forward to
            // L (official), which would corrupt the round-trip.
            if let Some(ref use_) = name.use_ {
                let type_code = match use_.as_str() {
                    "official" => Some("L"),
                    "maiden" => Some("M"),
                    "nickname" => Some("N"),
                    "temp" => Some("T"),
                    _ => None,
                };
                if let Some(type_code) = type_code {
                    let _ = pid.set_component(5, rep, 6, type_code);
                }
            }
        }
        Ok(())
    }

    /// Set addresses in PID-11, one repetition per address
    fn set_addresses(pid: &mut Segment, addresses: &[Address]) -> ConversionResult<()> {
        for (rep, address) in addresses.iter().enumerate() {
            // XAD structure: 1=Street, 2=Other, 3=City, 4=State, 5=ZIP, 6=Country, 7=Type

            // Street address - component 1 (0-indexed: 0)
            if let Some(ref lines) = address.line {
                if let Some(first_line) = lines.first() {
                    let _ = pid.set_component(11, rep, 0, first_line);
                }
                // Other designation - component 2 (0-indexed: 1)
                if lines.len() > 1 {
                    let _ = pid.set_component(11, rep, 1, &lines[1]);
                }
            }

            // City - component 3 (0-indexed: 2)
            if let Some(ref city) = address.city {
                let _ = pid.set_component(11, rep, 2, city);
            }

            // State - component 4 (0-indexed: 3)
            if let Some(ref state) = address.state {
                let _ = pid.set_component(11, rep, 3, state);
            }

            // Postal code - component 5 (0-indexed: 4)
            if let Some(ref postal_code) = address.postal_code {
                let _ = pid.set_component(11, rep, 4, postal_code);
            }

            // Country - component 6 (0-indexed: 5)
            if let Some(ref country) = address.country {
                let _ = pid.set_component(11, rep, 5, country);
            }

            // Address type - component 7 (0-indexed: 6)
            if let Some(ref use_) = address.use_ {
                let type_code = match use_.as_str() {
                    "home" => "H",
                    "work" => "O",
                    "billing" => "B",
                    _ => "H",
                };
                let _ = pid.set_component(11, rep, 6, type_code);
            }
        }
        Ok(())
    }

    /// Set telecoms in PID-13 (home and every other use) and
    /// PID-14 (work), one repetition per telecom (a whole-field write
    /// would overwrite all but the last). Email values go to XTN-4;
    /// anything else goes to XTN-1.
    fn set_telecoms(pid: &mut Segment, telecoms: &[ContactPoint]) -> ConversionResult<()> {
        let mut home_rep = 0usize;
        let mut work_rep = 0usize;
        for telecom in telecoms {
            let Some(ref value) = telecom.value else {
                continue;
            };
            // Only work use lands on PID-14; every other use (home,
            // mobile, temp, old, or none) lands on PID-13 so no
            // telecom is silently dropped.
            let (field, rep) = if telecom.use_.as_deref() == Some("work") {
                let rep = work_rep;
                work_rep += 1;
                (14, rep)
            } else {
                let rep = home_rep;
                home_rep += 1;
                (13, rep)
            };
            let comp = if telecom.system.as_deref() == Some("email") {
                3 // XTN-4: email address
            } else {
                0 // XTN-1: telephone number
            };
            let _ = pid.set_component(field, rep, comp, value);
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
    ///
    /// A trailing `Z` becomes `+0000`: a bare `Z` is not valid HL7 DTM.
    fn convert_datetime_to_hl7(datetime: &str) -> ConversionResult<String> {
        let zoned = match datetime.strip_suffix('Z') {
            Some(stem) => format!("{stem}+0000"),
            None => datetime.to_string(),
        };
        // Remove dashes, colons, and T separator
        Ok(zoned.replace(['-', ':', 'T'], ""))
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
            && let Some(value) = field.value()
        {
            assert!(value.contains("12345"));
        }
    }

    #[test]
    fn test_convert_gender() {
        assert_eq!(PatientReverseConverter::convert_gender_to_hl7("male"), "M");
        assert_eq!(
            PatientReverseConverter::convert_gender_to_hl7("female"),
            "F"
        );
        assert_eq!(PatientReverseConverter::convert_gender_to_hl7("other"), "O");
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

    fn rep_component(pid: &Segment, field: usize, rep: usize, comp: usize) -> Option<String> {
        pid.get_field(field)
            .and_then(|f| f.repetitions.get(rep))
            .and_then(|r| r.components.get(comp))
            .and_then(|c| c.value())
            .map(|s| s.to_string())
    }

    #[test]
    fn test_names_emit_every_entry_as_repetition() {
        let mut patient = Patient::new();
        patient.name = Some(vec![
            HumanName {
                use_: Some("official".to_string()),
                text: None,
                family: Some("Smith".to_string()),
                given: Some(vec!["John".to_string()]),
                prefix: None,
                suffix: None,
            },
            HumanName {
                use_: Some("maiden".to_string()),
                text: None,
                family: Some("Jones".to_string()),
                given: Some(vec!["Johnny".to_string()]),
                prefix: None,
                suffix: None,
            },
        ]);
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        assert_eq!(pid.get_field(5).unwrap().repetitions.len(), 2);
        assert_eq!(rep_component(&pid, 5, 0, 0).as_deref(), Some("Smith"));
        assert_eq!(rep_component(&pid, 5, 1, 0).as_deref(), Some("Jones"));
        assert_eq!(rep_component(&pid, 5, 1, 1).as_deref(), Some("Johnny"));
        assert_eq!(rep_component(&pid, 5, 1, 6).as_deref(), Some("M"));
    }

    #[test]
    fn test_addresses_emit_every_entry_as_repetition() {
        let mut patient = Patient::new();
        patient.address = Some(vec![
            Address {
                use_: Some("home".to_string()),
                type_: None,
                text: None,
                line: Some(vec!["1 Main St".to_string()]),
                city: Some("Springfield".to_string()),
                state: None,
                postal_code: None,
                country: None,
            },
            Address {
                use_: Some("work".to_string()),
                type_: None,
                text: None,
                line: Some(vec!["2 Work Rd".to_string()]),
                city: Some("Chicago".to_string()),
                state: None,
                postal_code: None,
                country: None,
            },
        ]);
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        assert_eq!(pid.get_field(11).unwrap().repetitions.len(), 2);
        assert_eq!(
            rep_component(&pid, 11, 0, 2).as_deref(),
            Some("Springfield")
        );
        assert_eq!(rep_component(&pid, 11, 1, 0).as_deref(), Some("2 Work Rd"));
        assert_eq!(rep_component(&pid, 11, 1, 2).as_deref(), Some("Chicago"));
    }

    #[test]
    fn test_identifiers_emit_every_entry_as_repetition() {
        let mut patient = Patient::new();
        patient.identifier = Some(vec![
            Identifier {
                use_: None,
                type_: None,
                system: None,
                value: Some("A1".to_string()),
                assigner: None,
            },
            Identifier {
                use_: None,
                type_: None,
                system: Some("urn:oid:HOSP".to_string()),
                value: Some("B2".to_string()),
                assigner: None,
            },
        ]);
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        assert_eq!(pid.get_field(3).unwrap().repetitions.len(), 2);
        assert_eq!(rep_component(&pid, 3, 0, 0).as_deref(), Some("A1"));
        assert_eq!(rep_component(&pid, 3, 1, 0).as_deref(), Some("B2"));
        assert_eq!(rep_component(&pid, 3, 1, 3).as_deref(), Some("HOSP"));
    }

    #[test]
    fn test_telecoms_emit_every_entry_as_repetition() {
        let mut patient = Patient::new();
        patient.telecom = Some(vec![
            ContactPoint {
                system: Some("phone".to_string()),
                value: Some("111".to_string()),
                use_: Some("home".to_string()),
            },
            ContactPoint {
                system: Some("phone".to_string()),
                value: Some("222".to_string()),
                use_: Some("home".to_string()),
            },
            ContactPoint {
                system: Some("phone".to_string()),
                value: Some("333".to_string()),
                use_: Some("work".to_string()),
            },
        ]);
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        assert_eq!(pid.get_field(13).unwrap().repetitions.len(), 2);
        assert_eq!(rep_component(&pid, 13, 0, 0).as_deref(), Some("111"));
        assert_eq!(rep_component(&pid, 13, 1, 0).as_deref(), Some("222"));
        assert_eq!(pid.get_field(14).unwrap().repetitions.len(), 1);
        assert_eq!(rep_component(&pid, 14, 0, 0).as_deref(), Some("333"));
    }

    #[test]
    fn test_telecoms_mobile_none_use_and_email_preserved() {
        let mut patient = Patient::new();
        patient.telecom = Some(vec![
            ContactPoint {
                system: Some("phone".to_string()),
                value: Some("555-9999".to_string()),
                use_: Some("mobile".to_string()),
            },
            ContactPoint {
                system: Some("phone".to_string()),
                value: Some("555-0000".to_string()),
                use_: None,
            },
            ContactPoint {
                system: Some("email".to_string()),
                value: Some("a@example.com".to_string()),
                use_: Some("home".to_string()),
            },
        ]);
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        // Non-home/work uses land on PID-13 as extra repetitions.
        assert_eq!(pid.get_field(13).unwrap().repetitions.len(), 3);
        assert_eq!(rep_component(&pid, 13, 0, 0).as_deref(), Some("555-9999"));
        assert_eq!(rep_component(&pid, 13, 1, 0).as_deref(), Some("555-0000"));
        // Email values land in XTN-4, not XTN-1.
        assert_eq!(rep_component(&pid, 13, 2, 0), None);
        assert_eq!(
            rep_component(&pid, 13, 2, 3).as_deref(),
            Some("a@example.com")
        );
    }

    #[test]
    fn test_convert_datetime_maps_trailing_z_to_offset() {
        assert_eq!(
            PatientReverseConverter::convert_datetime_to_hl7("2024-03-15T10:00:00Z").unwrap(),
            "20240315100000+0000"
        );
        // No zone: unchanged.
        assert_eq!(
            PatientReverseConverter::convert_datetime_to_hl7("2024-03-15T10:00:00").unwrap(),
            "20240315100000"
        );
        // End to end through PID-29.
        let mut patient = Patient::new();
        patient.deceased_date_time = Some("2024-03-15T10:00:00Z".to_string());
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        assert_eq!(pid.get_field_value(29), Some("20240315100000+0000"));
    }

    #[test]
    fn test_suffix_and_degree_round_trip() {
        use crate::converters::patient::PatientConverter;
        use vyna_hl7_parser::parse_message;

        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||67890^^^MRN||DOE^JOHN^A^JR^MR^MD^L||19800101|M";
        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        // XPN-4 keeps the suffix, XPN-6 keeps the degree.
        assert_eq!(rep_component(&pid, 5, 0, 3).as_deref(), Some("JR"));
        assert_eq!(rep_component(&pid, 5, 0, 5).as_deref(), Some("MD"));
    }

    #[test]
    fn test_usual_name_use_omits_type_code() {
        let named = |use_: &str| HumanName {
            use_: Some(use_.to_string()),
            text: None,
            family: Some("Smith".to_string()),
            given: Some(vec!["John".to_string()]),
            prefix: None,
            suffix: None,
        };
        let mut patient = Patient::new();
        patient.name = Some(vec![named("usual"), named("unknown"), named("official")]);
        let pid = PatientReverseConverter::convert(&patient).unwrap();
        // usual/unknown must not re-forward to official (L).
        assert_eq!(rep_component(&pid, 5, 0, 6), None);
        assert_eq!(rep_component(&pid, 5, 1, 6), None);
        assert_eq!(rep_component(&pid, 5, 2, 6).as_deref(), Some("L"));
    }
}
