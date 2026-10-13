//! Patient converter - PID segment to FHIR Patient resource
//!
//! Based on HL7 v2-to-FHIR mapping: <https://build.fhir.org/ig/HL7/v2-to-fhir/ConceptMap-segment-pid-to-patient.html>

use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::patient::Patient;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming PID segments to FHIR Patient resources
pub struct PatientConverter;

impl PatientConverter {
    /// Convert an HL7 v2 message containing a PID segment to a FHIR Patient resource
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 v2 message containing the PID segment
    ///
    /// # Returns
    ///
    /// A FHIR Patient resource
    ///
    /// # Errors
    ///
    /// Returns an error if the PID segment is not found or required fields are missing
    pub fn convert(message: &Message) -> ConversionResult<Patient> {
        let terser = Terser::new(message);

        // Check if PID segment exists
        if !message.segments.iter().any(|s| s.id == "PID") {
            return Err(ConversionError::MissingSegment("PID".to_string()));
        }

        let mut patient = Patient::new();

        // PID-3: Patient Identifier List -> Patient.identifier
        patient.identifier = Self::convert_identifiers(&terser)?;

        // PID-5: Patient Name -> Patient.name
        patient.name = Self::convert_names(&terser)?;

        // PID-7: Date of Birth -> Patient.birthDate
        if let Ok(Some(dob)) = terser.get("PID-7")
            && !dob.is_empty()
        {
            patient.birth_date = Some(Self::convert_date(dob)?);
        }

        // PID-8: Administrative Sex -> Patient.gender
        if let Ok(Some(sex)) = terser.get("PID-8")
            && !sex.is_empty()
        {
            patient.gender = Some(Self::convert_gender(sex)?);
        }

        // PID-11: Patient Address -> Patient.address
        patient.address = Self::convert_addresses(&terser)?;

        // PID-13: Phone Number - Home -> Patient.telecom
        // PID-14: Phone Number - Business -> Patient.telecom
        patient.telecom = Self::convert_telecom(&terser)?;

        // PID-16: Marital Status -> Patient.maritalStatus
        if let Ok(Some(marital)) = terser.get("PID-16")
            && !marital.is_empty()
        {
            patient.marital_status = Some(Self::convert_marital_status(marital));
        }

        // PID-24: Multiple Birth Indicator -> Patient.multipleBirth
        if let Ok(Some(multiple_birth)) = terser.get("PID-24") {
            if multiple_birth == "Y" {
                patient.multiple_birth_boolean = Some(true);
            } else if multiple_birth == "N" {
                patient.multiple_birth_boolean = Some(false);
            }
        }

        // PID-29: Patient Death Date and Time -> Patient.deceased
        if let Ok(Some(death_date)) = terser.get("PID-29")
            && !death_date.is_empty()
        {
            patient.deceased_date_time = Some(Self::convert_datetime(death_date)?);
        }

        // PID-30: Patient Death Indicator -> Patient.deceased
        if let Ok(Some(death_ind)) = terser.get("PID-30")
            && death_ind == "Y"
        {
            patient.deceased_boolean = Some(true);
        }

        patient.active = Some(true);

        Ok(patient)
    }

    /// Convert PID-3 (Patient Identifier List) to FHIR Identifier
    fn convert_identifiers(terser: &Terser) -> ConversionResult<Option<Vec<Identifier>>> {
        let mut identifiers = Vec::new();

        // Try to get all repetitions of PID-3
        // Note: Terser uses 1-based indexing throughout
        for rep in 0..10 {
            let id_path = if rep == 0 {
                "PID-3".to_string() // first repetition; whole-field read returns component 1 (ID)
            } else {
                format!("PID-3({})", rep + 1)
            };

            if let Ok(Some(id_value)) = terser.get(&id_path) {
                if id_value.is_empty() {
                    break;
                }

                let mut identifier = Identifier {
                    use_: None,
                    type_: None,
                    system: None,
                    value: Some(id_value.to_string()),
                    assigner: None,
                };

                // PID-3-4: Assigning Authority (component 4, 1-based indexing)
                let authority_path = if rep == 0 {
                    "PID-3-4".to_string()
                } else {
                    format!("PID-3({})-4", rep + 1)
                };
                if let Ok(Some(authority)) = terser.get(&authority_path)
                    && !authority.is_empty()
                {
                    identifier.system = Some(format!("urn:oid:{}", authority));
                }

                // PID-3-5: Identifier Type Code (component 5, 1-based indexing)
                let type_path = if rep == 0 {
                    "PID-3-5".to_string()
                } else {
                    format!("PID-3({})-5", rep + 1)
                };
                if let Ok(Some(id_type)) = terser.get(&type_path)
                    && !id_type.is_empty()
                {
                    identifier.type_ = Some(CodeableConcept {
                        coding: Some(vec![Coding {
                            system: Some(
                                "http://terminology.hl7.org/CodeSystem/v2-0203".to_string(),
                            ),
                            version: None,
                            code: Some(id_type.to_string()),
                            display: None,
                        }]),
                        text: Some(id_type.to_string()),
                    });
                }

                identifiers.push(identifier);
            } else {
                break;
            }
        }

        Ok(if identifiers.is_empty() {
            None
        } else {
            Some(identifiers)
        })
    }

    /// Convert PID-5 (Patient Name) to FHIR HumanName
    fn convert_names(terser: &Terser) -> ConversionResult<Option<Vec<HumanName>>> {
        let mut names = Vec::new();

        // Try to get all repetitions of PID-5
        // Note: Terser uses 1-based component indexing
        // XPN structure: 1=Family, 2=Given, 3=Middle, 4=Suffix, 5=Prefix, 7=Type
        for rep in 0..5 {
            let family_path = if rep == 0 {
                "PID-5-1".to_string() // Component 1 (family name) - 1-based indexing
            } else {
                format!("PID-5({})-1", rep + 1) // Repetitions also 1-based
            };

            if let Ok(Some(family)) = terser.get(&family_path) {
                if family.is_empty() {
                    break;
                }

                let mut name = HumanName {
                    use_: None,
                    text: None,
                    family: Some(family.to_string()),
                    given: None,
                    prefix: None,
                    suffix: None,
                };

                // PID-5-2: Given Name (component 2, 1-based indexing)
                let given_path = if rep == 0 {
                    "PID-5-2".to_string()
                } else {
                    format!("PID-5({})-2", rep + 1)
                };
                if let Ok(Some(given)) = terser.get(&given_path)
                    && !given.is_empty()
                {
                    let mut given_names = vec![given.to_string()];

                    // PID-5-3: Middle Name (component 3, 1-based indexing)
                    let middle_path = if rep == 0 {
                        "PID-5-3".to_string()
                    } else {
                        format!("PID-5({})-3", rep + 1)
                    };
                    if let Ok(Some(middle)) = terser.get(&middle_path)
                        && !middle.is_empty()
                    {
                        given_names.push(middle.to_string());
                    }

                    name.given = Some(given_names);
                }

                // PID-5-4: Suffix (component 4, 1-based indexing)
                let suffix_path = if rep == 0 {
                    "PID-5-4".to_string()
                } else {
                    format!("PID-5({})-4", rep + 1)
                };
                if let Ok(Some(suffix)) = terser.get(&suffix_path)
                    && !suffix.is_empty()
                {
                    name.suffix = Some(vec![suffix.to_string()]);
                }

                // PID-5-5: Prefix (component 5, 1-based indexing)
                let prefix_path = if rep == 0 {
                    "PID-5-5".to_string()
                } else {
                    format!("PID-5({})-5", rep + 1)
                };
                if let Ok(Some(prefix)) = terser.get(&prefix_path)
                    && !prefix.is_empty()
                {
                    name.prefix = Some(vec![prefix.to_string()]);
                }

                // PID-5-7: Name Type Code (component 7, 1-based indexing)
                let type_path = if rep == 0 {
                    "PID-5-7".to_string()
                } else {
                    format!("PID-5({})-7", rep + 1)
                };
                if let Ok(Some(name_type)) = terser.get(&type_path) {
                    name.use_ = Some(match name_type {
                        "L" => "official".to_string(),
                        "M" => "maiden".to_string(),
                        "N" => "nickname".to_string(),
                        "T" => "temp".to_string(),
                        _ => "usual".to_string(),
                    });
                }

                names.push(name);
            } else {
                break;
            }
        }

        Ok(if names.is_empty() { None } else { Some(names) })
    }

    /// Convert PID-11 (Patient Address) to FHIR Address
    fn convert_addresses(terser: &Terser) -> ConversionResult<Option<Vec<Address>>> {
        let mut addresses = Vec::new();

        // Try to get all repetitions of PID-11
        // Note: Terser uses 1-based component indexing
        for rep in 0..5 {
            let street_path = if rep == 0 {
                "PID-11-1".to_string() // PID-11-1 (Street, component 1)
            } else {
                format!("PID-11({})-1", rep + 1)
            };

            if let Ok(Some(street)) = terser.get(&street_path) {
                if street.is_empty() {
                    break;
                }

                let mut address = Address {
                    use_: None,
                    type_: None,
                    text: None,
                    line: Some(vec![street.to_string()]),
                    city: None,
                    state: None,
                    postal_code: None,
                    country: None,
                };

                // PID-11-2: Other Designation (component 2, 1-based indexing)
                let other_path = if rep == 0 {
                    "PID-11-2".to_string()
                } else {
                    format!("PID-11({})-2", rep + 1)
                };
                if let Ok(Some(other)) = terser.get(&other_path)
                    && !other.is_empty()
                    && let Some(ref mut lines) = address.line
                {
                    lines.push(other.to_string());
                }

                // PID-11-3: City (component 3, 1-based indexing)
                let city_path = if rep == 0 {
                    "PID-11-3".to_string()
                } else {
                    format!("PID-11({})-3", rep + 1)
                };
                if let Ok(Some(city)) = terser.get(&city_path)
                    && !city.is_empty()
                {
                    address.city = Some(city.to_string());
                }

                // PID-11-4: State (component 4, 1-based indexing)
                let state_path = if rep == 0 {
                    "PID-11-4".to_string()
                } else {
                    format!("PID-11({})-4", rep + 1)
                };
                if let Ok(Some(state)) = terser.get(&state_path)
                    && !state.is_empty()
                {
                    address.state = Some(state.to_string());
                }

                // PID-11-5: Postal Code (component 5, 1-based indexing)
                let zip_path = if rep == 0 {
                    "PID-11-5".to_string()
                } else {
                    format!("PID-11({})-5", rep + 1)
                };
                if let Ok(Some(zip)) = terser.get(&zip_path)
                    && !zip.is_empty()
                {
                    address.postal_code = Some(zip.to_string());
                }

                // PID-11-6: Country (component 6, 1-based indexing)
                let country_path = if rep == 0 {
                    "PID-11-6".to_string()
                } else {
                    format!("PID-11({})-6", rep + 1)
                };
                if let Ok(Some(country)) = terser.get(&country_path)
                    && !country.is_empty()
                {
                    address.country = Some(country.to_string());
                }

                // PID-11-7: Address Type (component 7, 1-based indexing)
                let type_path = if rep == 0 {
                    "PID-11-7".to_string()
                } else {
                    format!("PID-11({})-7", rep + 1)
                };
                if let Ok(Some(addr_type)) = terser.get(&type_path) {
                    address.use_ = Some(match addr_type {
                        "H" => "home".to_string(),
                        "O" => "work".to_string(),
                        "B" => "billing".to_string(),
                        _ => "home".to_string(),
                    });
                }

                addresses.push(address);
            } else {
                break;
            }
        }

        Ok(if addresses.is_empty() {
            None
        } else {
            Some(addresses)
        })
    }

    /// Convert PID-13 and PID-14 (Phone Numbers) to FHIR ContactPoint
    fn convert_telecom(terser: &Terser) -> ConversionResult<Option<Vec<ContactPoint>>> {
        let mut telecoms = Vec::new();

        // PID-13: Home Phone
        if let Ok(Some(home_phone)) = terser.get("PID-13")
            && !home_phone.is_empty()
        {
            telecoms.push(ContactPoint {
                system: Some("phone".to_string()),
                value: Some(home_phone.to_string()),
                use_: Some("home".to_string()),
            });
        }

        // PID-14: Business Phone
        if let Ok(Some(work_phone)) = terser.get("PID-14")
            && !work_phone.is_empty()
        {
            telecoms.push(ContactPoint {
                system: Some("phone".to_string()),
                value: Some(work_phone.to_string()),
                use_: Some("work".to_string()),
            });
        }

        Ok(if telecoms.is_empty() {
            None
        } else {
            Some(telecoms)
        })
    }

    /// Convert HL7 v2 gender code to FHIR gender code
    fn convert_gender(sex: &str) -> ConversionResult<String> {
        Ok(match sex {
            "M" => "male",
            "F" => "female",
            "O" => "other",
            "U" | "A" => "unknown",
            _ => "unknown",
        }
        .to_string())
    }

    /// Convert HL7 v2 marital status to FHIR CodeableConcept
    fn convert_marital_status(code: &str) -> CodeableConcept {
        CodeableConcept {
            coding: Some(vec![Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v3-MaritalStatus".to_string()),
                version: None,
                code: Some(code.to_string()),
                display: None,
            }]),
            text: None,
        }
    }

    /// Convert HL7 v2 date format (YYYYMMDD) to FHIR date format (YYYY-MM-DD)
    fn convert_date(date: &str) -> ConversionResult<String> {
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
        let bytes = date.as_bytes();
        if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
        } else if bytes.len() >= 4 && bytes[..4].iter().all(|b| b.is_ascii_digit()) {
            Ok(date[0..4].to_string())
        } else {
            Err(ConversionError::InvalidFormat(
                "date".to_string(),
                "PID-7".to_string(),
                format!("Invalid date format: {}", date),
            ))
        }
    }

    /// Convert HL7 v2 datetime format to FHIR datetime format
    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        let bytes = datetime.as_bytes();
        if bytes.len() >= 14 && bytes[..14].iter().all(|b| b.is_ascii_digit()) {
            Ok(format!(
                "{}-{}-{}T{}:{}:{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8],
                &datetime[8..10],
                &datetime[10..12],
                &datetime[12..14]
            ))
        } else if bytes.len() >= 8 {
            Self::convert_date(datetime)
        } else {
            Err(ConversionError::InvalidFormat(
                "datetime".to_string(),
                "PID".to_string(),
                format!("Invalid datetime format: {}", datetime),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_simple_patient() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||67890^^^MRN||DOE^JOHN^A||19800101|M";

        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();

        assert_eq!(patient.resource_type, "Patient");
        assert_eq!(patient.gender, Some("male".to_string()));
        assert_eq!(patient.birth_date, Some("1980-01-01".to_string()));

        let names = patient.name.as_ref().unwrap();
        assert_eq!(names[0].family, Some("DOE".to_string()));
        assert_eq!(
            names[0].given,
            Some(vec!["JOHN".to_string(), "A".to_string()])
        );
    }

    #[test]
    fn test_convert_patient_with_address() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123^^^MRN||SMITH^JANE||19900215|F|||123 Main St^^Springfield^IL^62701^USA^^H";

        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();

        let addresses = patient.address.unwrap();
        assert_eq!(addresses[0].line, Some(vec!["123 Main St".to_string()]));
        assert_eq!(addresses[0].city, Some("Springfield".to_string()));
        assert_eq!(addresses[0].state, Some("IL".to_string()));
        assert_eq!(addresses[0].postal_code, Some("62701".to_string()));
        assert_eq!(addresses[0].country, Some("USA".to_string()));
        assert_eq!(addresses[0].use_, Some("home".to_string()));
    }

    #[test]
    fn test_identifiers_read_every_repetition_with_authority_and_type() {
        // PID-3 is CX: component 1 = ID, 4 = assigning authority, 5 = type.
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||12345^^^HOSP^MR~67890^^^CLINIC^SS|||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let ids = patient.identifier.as_ref().unwrap();

        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0].value.as_deref(), Some("12345"));
        assert_eq!(ids[0].system.as_deref(), Some("urn:oid:HOSP"));
        assert_eq!(type_code(&ids[0]).as_deref(), Some("MR"));
        assert_eq!(ids[1].value.as_deref(), Some("67890"));
        assert_eq!(ids[1].system.as_deref(), Some("urn:oid:CLINIC"));
        assert_eq!(type_code(&ids[1]).as_deref(), Some("SS"));

        fn type_code(id: &Identifier) -> Option<String> {
            id.type_
                .as_ref()
                .unwrap()
                .coding
                .as_ref()
                .unwrap()
                .first()
                .unwrap()
                .code
                .clone()
        }
    }

    #[test]
    fn test_non_ascii_dob_returns_error_without_panic() {
        // Misaligned multi-byte chars used to panic on byte slicing.
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||67890^^^MRN||DOE^JOHN^A||123\u{e9}5678|M";

        let message = parse_message(hl7).unwrap();
        let result = PatientConverter::convert(&message);
        assert!(matches!(
            result,
            Err(ConversionError::InvalidFormat(_, _, _))
        ));
    }

    #[test]
    fn test_non_digit_dob_returns_error() {
        assert!(PatientConverter::convert_date("abcdefgh").is_err());
        assert!(PatientConverter::convert_date("1980abcd").is_ok());
        assert_eq!(
            PatientConverter::convert_date("19800101").unwrap(),
            "1980-01-01"
        );
        assert!(PatientConverter::convert_datetime("not-a-datetime").is_err());
        assert_eq!(
            PatientConverter::convert_datetime("20240315100000").unwrap(),
            "2024-03-15T10:00:00"
        );
    }

    #[test]
    fn test_address_reads_second_repetition() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123^^^MRN||SMITH^JANE||19900215|F|||123 Main St^^Springfield^IL^62701^USA^^H~456 Oak Ave^^Chicago^IL^60601^USA^^W";

        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let addresses = patient.address.as_ref().unwrap();

        assert_eq!(addresses.len(), 2);
        assert_eq!(addresses[1].city, Some("Chicago".to_string()));
        assert_eq!(addresses[1].postal_code, Some("60601".to_string()));
    }
}
