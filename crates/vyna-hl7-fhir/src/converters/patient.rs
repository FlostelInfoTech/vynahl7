//! Patient converter - PID segment to FHIR Patient resource
//!
//! Based on HL7 v2-to-FHIR mapping: <https://build.fhir.org/ig/HL7/v2-to-fhir/ConceptMap-segment-pid-to-patient.html>

use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::patient::Patient;
use chrono::{NaiveDate, NaiveTime};
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
        // PID-30: Patient Death Indicator -> Patient.deceased
        // FHIR deceased[x] is a choice: emit exactly one. The date wins
        // when present; the boolean applies only without a death date.
        if let Ok(Some(death_date)) = terser.get("PID-29")
            && !death_date.is_empty()
        {
            patient.deceased_date_time = Some(Self::convert_datetime(death_date)?);
        } else if let Ok(Some(death_ind)) = terser.get("PID-30")
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

        // Convert every repetition with no cap: repetitions are
        // parser-bounded (already materialized), truncating identifiers
        // loses patient identity data, and the segment converters (OBX,
        // AL1, ...) likewise convert every repetition.
        // Note: Terser uses 1-based indexing throughout
        let mut rep = 0usize;
        loop {
            let id_path = if rep == 0 {
                "PID-3".to_string() // first repetition; whole-field read returns component 1 (ID)
            } else {
                format!("PID-3({})", rep + 1)
            };

            // An absent repetition ends the scan; an empty one is a gap
            // to skip since valued repetitions may follow it.
            let id_value = match terser.get(&id_path) {
                Ok(Some(value)) => value,
                _ => break,
            };
            let cur = rep;
            rep += 1;
            let comp = |n: u8| {
                let path = if cur == 0 {
                    format!("PID-3-{n}")
                } else {
                    format!("PID-3({})-{n}", cur + 1)
                };
                terser.get(&path).ok().flatten().unwrap_or("")
            };

            // PID-3-4: Assigning Authority (component 4, 1-based indexing)
            let authority = comp(4);
            // PID-3-5: Identifier Type Code (component 5, 1-based indexing)
            let id_type = comp(5);
            if id_value.is_empty() && authority.is_empty() && id_type.is_empty() {
                continue;
            }

            let mut identifier = Identifier {
                use_: None,
                type_: None,
                system: None,
                value: if id_value.is_empty() {
                    None
                } else {
                    Some(id_value.to_string())
                },
                assigner: None,
            };

            if !authority.is_empty() {
                identifier.system = Some(format!("urn:oid:{}", authority));
            }

            if !id_type.is_empty() {
                identifier.type_ = Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v2-0203".to_string()),
                        version: None,
                        code: Some(id_type.to_string()),
                        display: None,
                    }]),
                    text: Some(id_type.to_string()),
                });
            }

            identifiers.push(identifier);
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

        // Convert every repetition with no cap (see convert_identifiers
        // for why truncation is worse than the parser-bounded cost).
        // Note: Terser uses 1-based component indexing
        // XPN structure: 1=Family, 2=Given, 3=Middle, 4=Suffix, 5=Prefix, 7=Type
        let mut rep = 0usize;
        loop {
            let family_path = if rep == 0 {
                "PID-5-1".to_string() // Component 1 (family name) - 1-based indexing
            } else {
                format!("PID-5({})-1", rep + 1) // Repetitions also 1-based
            };

            // An absent repetition ends the scan; an empty one is a gap
            // to skip since valued repetitions may follow it.
            let family = match terser.get(&family_path) {
                Ok(Some(value)) => value,
                _ => break,
            };
            let cur = rep;
            rep += 1;
            let comp = |n: u8| {
                let path = if cur == 0 {
                    format!("PID-5-{n}")
                } else {
                    format!("PID-5({})-{n}", cur + 1)
                };
                terser.get(&path).ok().flatten().unwrap_or("")
            };

            // PID-5-2: Given Name (component 2, 1-based indexing)
            let given = comp(2);
            // PID-5-3: Middle Name (component 3, 1-based indexing)
            let middle = comp(3);
            // PID-5-4: Suffix (component 4, 1-based indexing)
            let suffix = comp(4);
            // PID-5-5: Prefix (component 5, 1-based indexing)
            let prefix = comp(5);
            // PID-5-6: Degree (component 6, 1-based indexing)
            let degree = comp(6);
            // PID-5-7: Name Type Code (component 7, 1-based indexing).
            // Kept as an Option to preserve the match below, which maps
            // any unrecognized code to "usual".
            let name_type = if cur == 0 {
                terser.get("PID-5-7").ok().flatten()
            } else {
                terser.get(&format!("PID-5({})-7", cur + 1)).ok().flatten()
            };
            if family.is_empty()
                && given.is_empty()
                && middle.is_empty()
                && suffix.is_empty()
                && prefix.is_empty()
                && degree.is_empty()
                && name_type.unwrap_or("").is_empty()
            {
                continue;
            }

            let mut name = HumanName {
                use_: None,
                text: None,
                family: if family.is_empty() {
                    None
                } else {
                    Some(family.to_string())
                },
                given: None,
                prefix: None,
                suffix: None,
            };

            if !given.is_empty() {
                let mut given_names = vec![given.to_string()];
                if !middle.is_empty() {
                    given_names.push(middle.to_string());
                }
                name.given = Some(given_names);
            }

            // Suffix and degree share HumanName.suffix so XPN-6
            // is preserved instead of dropped.
            let mut suffixes = Vec::new();
            if !suffix.is_empty() {
                suffixes.push(suffix.to_string());
            }
            if !degree.is_empty() {
                suffixes.push(degree.to_string());
            }
            if !suffixes.is_empty() {
                name.suffix = Some(suffixes);
            }

            if !prefix.is_empty() {
                name.prefix = Some(vec![prefix.to_string()]);
            }

            if let Some(name_type) = name_type {
                name.use_ = Some(match name_type {
                    "L" => "official".to_string(),
                    "M" => "maiden".to_string(),
                    "N" => "nickname".to_string(),
                    "T" => "temp".to_string(),
                    _ => "usual".to_string(),
                });
            }

            names.push(name);
        }

        Ok(if names.is_empty() { None } else { Some(names) })
    }

    /// Convert PID-11 (Patient Address) to FHIR Address
    fn convert_addresses(terser: &Terser) -> ConversionResult<Option<Vec<Address>>> {
        let mut addresses = Vec::new();

        // Convert every repetition with no cap (see convert_identifiers
        // for why truncation is worse than the parser-bounded cost).
        // Note: Terser uses 1-based component indexing
        let mut rep = 0usize;
        loop {
            let street_path = if rep == 0 {
                "PID-11-1".to_string() // PID-11-1 (Street, component 1)
            } else {
                format!("PID-11({})-1", rep + 1)
            };

            // An absent repetition ends the scan; an empty one is a gap
            // to skip since valued repetitions may follow it.
            let street = match terser.get(&street_path) {
                Ok(Some(value)) => value,
                _ => break,
            };
            let cur = rep;
            rep += 1;
            let comp = |n: u8| {
                let path = if cur == 0 {
                    format!("PID-11-{n}")
                } else {
                    format!("PID-11({})-{n}", cur + 1)
                };
                terser.get(&path).ok().flatten().unwrap_or("")
            };

            // PID-11-2: Other Designation (component 2, 1-based indexing)
            let other = comp(2);
            // PID-11-3: City (component 3, 1-based indexing)
            let city = comp(3);
            // PID-11-4: State (component 4, 1-based indexing)
            let state = comp(4);
            // PID-11-5: Postal Code (component 5, 1-based indexing)
            let zip = comp(5);
            // PID-11-6: Country (component 6, 1-based indexing)
            let country = comp(6);
            // PID-11-7: Address Type (component 7, 1-based indexing).
            // Kept as an Option: a present-but-empty type still maps
            // through the match below (defaulting to home), matching the
            // long-standing conversion contract.
            let addr_type = if cur == 0 {
                terser.get("PID-11-7").ok().flatten()
            } else {
                terser.get(&format!("PID-11({})-7", cur + 1)).ok().flatten()
            };
            if street.is_empty()
                && other.is_empty()
                && city.is_empty()
                && state.is_empty()
                && zip.is_empty()
                && country.is_empty()
                && addr_type.unwrap_or("").is_empty()
            {
                continue;
            }

            let mut address = Address {
                use_: None,
                type_: None,
                text: None,
                line: if street.is_empty() {
                    None
                } else {
                    Some(vec![street.to_string()])
                },
                city: None,
                state: None,
                postal_code: None,
                country: None,
            };

            if !other.is_empty()
                && let Some(ref mut lines) = address.line
            {
                lines.push(other.to_string());
            }

            if !city.is_empty() {
                address.city = Some(city.to_string());
            }

            if !state.is_empty() {
                address.state = Some(state.to_string());
            }

            if !zip.is_empty() {
                address.postal_code = Some(zip.to_string());
            }

            if !country.is_empty() {
                address.country = Some(country.to_string());
            }

            if let Some(addr_type) = addr_type {
                address.use_ = Some(match addr_type {
                    "H" => "home".to_string(),
                    "O" => "work".to_string(),
                    "B" => "billing".to_string(),
                    _ => "home".to_string(),
                });
            }

            addresses.push(address);
        }

        Ok(if addresses.is_empty() {
            None
        } else {
            Some(addresses)
        })
    }

    /// Convert PID-13 and PID-14 (Phone Numbers) to FHIR ContactPoint
    ///
    /// PID-13/14 are XTN: component 1 is the telephone number, component 3
    /// the equipment type, and component 4 the email address. Every
    /// repetition converts; email forms map to `system: email` instead of
    /// being dropped or mislabeled as phone.
    fn convert_telecom(terser: &Terser) -> ConversionResult<Option<Vec<ContactPoint>>> {
        let mut telecoms = Vec::new();

        // (field, use): PID-13 is home, PID-14 is business.
        for (field, use_) in [("PID-13", "home"), ("PID-14", "work")] {
            let mut rep = 0usize;
            loop {
                let base = if rep == 0 {
                    field.to_string()
                } else {
                    format!("{field}({})", rep + 1)
                };
                // An absent repetition ends the scan; an empty one is a
                // gap to skip since valued repetitions may follow it.
                let number = match terser.get(&format!("{base}-1")) {
                    Ok(Some(value)) => value,
                    _ => break,
                };
                rep += 1;
                let email = terser
                    .get(&format!("{base}-4"))
                    .ok()
                    .flatten()
                    .unwrap_or("");
                let equipment = terser
                    .get(&format!("{base}-3"))
                    .ok()
                    .flatten()
                    .unwrap_or("");
                if number.is_empty() && email.is_empty() {
                    continue;
                }

                if !number.is_empty() {
                    let system = if Self::is_email_form(number, equipment) {
                        "email"
                    } else {
                        "phone"
                    };
                    telecoms.push(ContactPoint {
                        system: Some(system.to_string()),
                        value: Some(number.to_string()),
                        use_: Some(use_.to_string()),
                    });
                }
                if !email.is_empty() {
                    telecoms.push(ContactPoint {
                        system: Some("email".to_string()),
                        value: Some(email.to_string()),
                        use_: Some(use_.to_string()),
                    });
                }
            }
        }

        Ok(if telecoms.is_empty() {
            None
        } else {
            Some(telecoms)
        })
    }

    /// True when an XTN-1 value is an email address: either it looks like
    /// one or the XTN-3 equipment type says so (HL7 table 0202 Internet /
    /// X.400 devices carry email, not phone calls).
    fn is_email_form(number: &str, equipment: &str) -> bool {
        number.contains('@')
            || matches!(
                equipment.to_ascii_lowercase().as_str(),
                "internet" | "x.400"
            )
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
    ///
    /// Partial precision is preserved: `YYYYMM` becomes `YYYY-MM` and
    /// `YYYY` stays a year, since FHIR dates accept both.
    fn convert_date(date: &str) -> ConversionResult<String> {
        // A zone suffix cannot apply to a date (FHIR dates carry no
        // zone), so convert the stem alone.
        let (stem, _) = split_hl7_offset(date);
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
        let bytes = stem.as_bytes();
        if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            // Full-date precision requires a real calendar date; an
            // impossible month/day is an error, not a truncation to year.
            if valid_hl7_date(&stem[0..8]) {
                Ok(format!("{}-{}-{}", &stem[0..4], &stem[4..6], &stem[6..8]))
            } else {
                Err(Self::invalid_date_format())
            }
        } else if bytes.len() >= 6 && bytes[..6].iter().all(|b| b.is_ascii_digit()) {
            // Month precision, with the same impossible-date policy.
            if valid_hl7_year_month(&stem[0..6]) {
                Ok(format!("{}-{}", &stem[0..4], &stem[4..6]))
            } else {
                Err(Self::invalid_date_format())
            }
        } else if bytes.len() >= 4 && bytes[..4].iter().all(|b| b.is_ascii_digit()) {
            Ok(stem[0..4].to_string())
        } else {
            Err(Self::invalid_date_format())
        }
    }

    /// Convert HL7 v2 datetime format to FHIR datetime format
    ///
    /// `YYYYMMDDHHMM` keeps its minutes (`YYYY-MM-DDTHH:MM`) and a
    /// trailing HL7 zone (`±HHMM`) is preserved as `±HH:MM`.
    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        let (stem, offset) = split_hl7_offset(datetime);
        let zone = offset.as_deref().unwrap_or("");
        let bytes = stem.as_bytes();
        if bytes.len() >= 14 && bytes[..14].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) && valid_hl7_time(&stem[8..14]) {
                Ok(format!(
                    "{}-{}-{}T{}:{}:{}{}",
                    &stem[0..4],
                    &stem[4..6],
                    &stem[6..8],
                    &stem[8..10],
                    &stem[10..12],
                    &stem[12..14],
                    zone
                ))
            } else {
                Err(Self::invalid_datetime_format())
            }
        } else if bytes.len() >= 12 && bytes[..12].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) && valid_hl7_hour_min(&stem[8..12]) {
                Ok(format!(
                    "{}-{}-{}T{}:{}{}",
                    &stem[0..4],
                    &stem[4..6],
                    &stem[6..8],
                    &stem[8..10],
                    &stem[10..12],
                    zone
                ))
            } else {
                Err(Self::invalid_datetime_format())
            }
        } else if bytes.len() >= 8 {
            Self::convert_date(stem)
        } else {
            Err(Self::invalid_datetime_format())
        }
    }

    /// Redacted bad-date error: field location + reason only, never the value.
    fn invalid_date_format() -> ConversionError {
        ConversionError::InvalidFormat(
            "date".to_string(),
            "PID-7".to_string(),
            "expected YYYYMMDD calendar date, YYYYMM month, or YYYY year".to_string(),
        )
    }

    /// Redacted bad-datetime error: field location + reason only, never the value.
    fn invalid_datetime_format() -> ConversionError {
        ConversionError::InvalidFormat(
            "datetime".to_string(),
            "PID".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime".to_string(),
        )
    }
}

/// Validate an HL7 date (`YYYYMMDD`) as a real calendar date.
///
/// Shared by the converters in this module so digit-prefix checks alone
/// never accept impossible dates such as month 13 or February 30.
pub(crate) fn valid_hl7_date(ymd: &str) -> bool {
    let digits = ymd.as_bytes();
    digits.len() == 8
        && digits.iter().all(|b| b.is_ascii_digit())
        && NaiveDate::from_ymd_opt(
            digit_value(&digits[0..4]) as i32,
            digit_value(&digits[4..6]),
            digit_value(&digits[6..8]),
        )
        .is_some()
}

/// Validate an HL7 time (`HHMMSS`) as a real time of day.
pub(crate) fn valid_hl7_time(hms: &str) -> bool {
    let digits = hms.as_bytes();
    digits.len() == 6
        && digits.iter().all(|b| b.is_ascii_digit())
        && NaiveTime::from_hms_opt(
            digit_value(&digits[0..2]),
            digit_value(&digits[2..4]),
            digit_value(&digits[4..6]),
        )
        .is_some()
}

/// Validate an HL7 year-month (`YYYYMM`): six digits with month 01-12.
pub(crate) fn valid_hl7_year_month(yyyymm: &str) -> bool {
    let digits = yyyymm.as_bytes();
    digits.len() == 6
        && digits.iter().all(|b| b.is_ascii_digit())
        && (1..=12).contains(&digit_value(&digits[4..6]))
}

/// Validate an HL7 hour-minute (`HHMM`) as a real time of day.
pub(crate) fn valid_hl7_hour_min(hhmm: &str) -> bool {
    let digits = hhmm.as_bytes();
    digits.len() == 4
        && digits.iter().all(|b| b.is_ascii_digit())
        && NaiveTime::from_hms_opt(digit_value(&digits[0..2]), digit_value(&digits[2..4]), 0)
            .is_some()
}

/// Split an HL7 datetime into its digit stem and an optional zone.
///
/// HL7 DTM carries the zone as a trailing `±HHMM` (e.g.
/// `20240315100000+0500`); FHIR spells it `±HH:MM`. A missing or
/// out-of-range zone yields `None`, in which case callers convert the
/// stem alone (and their digit-prefix checks ignore any trailing junk,
/// as before).
pub(crate) fn split_hl7_offset(value: &str) -> (&str, Option<String>) {
    if value.len() > 5 && value.is_char_boundary(value.len() - 5) {
        let (stem, suffix) = value.split_at(value.len() - 5);
        let bytes = suffix.as_bytes();
        // ASCII sign + digits make the `&suffix` slices below safe.
        if (bytes[0] == b'+' || bytes[0] == b'-')
            && bytes[1..].iter().all(|b| b.is_ascii_digit())
            && digit_value(&bytes[1..3]) <= 23
            && digit_value(&bytes[3..5]) <= 59
        {
            let zone = format!("{}{}:{}", &suffix[..1], &suffix[1..3], &suffix[3..5]);
            return (stem, Some(zone));
        }
    }
    (value, None)
}

/// Numeric value of an all-ASCII-digit slice (callers guarantee the digits).
fn digit_value(digits: &[u8]) -> u32 {
    digits.iter().fold(0, |acc, b| acc * 10 + (b - b'0') as u32)
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
    fn test_convert_names_preserves_degree_in_suffix() {
        // XPN-6 (degree) must survive alongside XPN-4 (suffix).
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||67890^^^MRN||DOE^JOHN^A^JR^MR^MD^L||19800101|M";
        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let names = patient.name.as_ref().unwrap();
        assert_eq!(
            names[0].suffix,
            Some(vec!["JR".to_string(), "MD".to_string()])
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

    fn pid_message_with_death_fields(death_date: &str, death_ind: &str) -> String {
        let mut fields = vec![String::new(); 30];
        fields[0] = "1".to_string();
        fields[28] = death_date.to_string();
        fields[29] = death_ind.to_string();
        format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\rPID|{}",
            fields.join("|")
        )
    }

    #[test]
    fn test_deceased_date_wins_over_indicator() {
        // FHIR deceased[x] is a choice: date and boolean must not both serialize.
        let hl7 = pid_message_with_death_fields("20240315100000", "Y");
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        assert_eq!(
            patient.deceased_date_time.as_deref(),
            Some("2024-03-15T10:00:00")
        );
        assert_eq!(patient.deceased_boolean, None);
    }

    #[test]
    fn test_deceased_indicator_without_date_sets_boolean() {
        let hl7 = pid_message_with_death_fields("", "Y");
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        assert_eq!(patient.deceased_date_time, None);
        assert_eq!(patient.deceased_boolean, Some(true));
    }

    #[test]
    fn test_impossible_calendar_dates_rejected() {
        assert!(PatientConverter::convert_date("20241345").is_err());
        assert!(PatientConverter::convert_date("20240230").is_err());
        assert!(PatientConverter::convert_date("20240015").is_err());
        assert_eq!(
            PatientConverter::convert_date("20240229").unwrap(),
            "2024-02-29"
        );
        assert_eq!(
            PatientConverter::convert_date("19800101").unwrap(),
            "1980-01-01"
        );
        assert!(PatientConverter::convert_datetime("20240315250000").is_err());
        assert!(PatientConverter::convert_datetime("20240230100000").is_err());
        assert_eq!(
            PatientConverter::convert_datetime("20240315100000").unwrap(),
            "2024-03-15T10:00:00"
        );
    }

    #[test]
    fn test_invalid_date_error_redacts_source_value() {
        let err = PatientConverter::convert_date("nonsense").unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("nonsense"), "error leaks source: {msg}");
        assert!(msg.contains("PID-7"));

        let err = PatientConverter::convert_datetime("bad-dt!!").unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("bad-dt!!"), "error leaks source: {msg}");
    }

    #[test]
    fn test_names_convert_all_repetitions_past_old_cap() {
        let names: Vec<String> = (1..=6).map(|i| format!("FAM{i}^GIV{i}")).collect();
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||ID||{}||19800101|M",
            names.join("~")
        );
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let names = patient.name.as_ref().unwrap();
        assert_eq!(names.len(), 6);
        assert_eq!(names[5].family.as_deref(), Some("FAM6"));
    }

    #[test]
    fn test_names_skip_empty_gaps_and_keep_valued() {
        for (pid5, expected) in [
            ("~DOE^JOHN", vec!["DOE"]),
            ("AAA^A~~BBB^B", vec!["AAA", "BBB"]),
        ] {
            let hl7 = format!(
                "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                 PID|1||ID||{pid5}||19800101|M"
            );
            let message = parse_message(&hl7).unwrap();
            let patient = PatientConverter::convert(&message).unwrap();
            let names = patient.name.as_ref().unwrap();
            let families: Vec<_> = names.iter().filter_map(|n| n.family.as_deref()).collect();
            assert_eq!(families, expected, "PID-5 {pid5}");
        }
        // Given-only repetition is still valued.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
             PID|1||ID||^JOHN||19800101|M";
        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let names = patient.name.as_ref().unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].family, None);
        assert_eq!(names[0].given, Some(vec!["JOHN".to_string()]));
    }

    #[test]
    fn test_identifiers_convert_all_repetitions_past_old_cap() {
        let ids: Vec<String> = (1..=11).map(|i| format!("ID{i}")).collect();
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||{}||DOE^JOHN||19800101|M",
            ids.join("~")
        );
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let ids = patient.identifier.as_ref().unwrap();
        assert_eq!(ids.len(), 11);
        assert_eq!(ids[10].value.as_deref(), Some("ID11"));
    }

    #[test]
    fn test_addresses_convert_all_repetitions_past_old_cap() {
        let addrs: Vec<String> = (1..=6).map(|i| format!("{i} Main St^^City{i}")).collect();
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||ID||DOE^JOHN||19800101|M|||{}",
            addrs.join("~")
        );
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let addresses = patient.address.as_ref().unwrap();
        assert_eq!(addresses.len(), 6);
        assert_eq!(addresses[5].city.as_deref(), Some("City6"));
    }

    #[test]
    fn test_convert_date_preserves_year_month_precision() {
        assert_eq!(PatientConverter::convert_date("198001").unwrap(), "1980-01");
        assert_eq!(PatientConverter::convert_date("202402").unwrap(), "2024-02");
        // Impossible month is an error, like an impossible day.
        assert!(PatientConverter::convert_date("198013").is_err());
        assert!(PatientConverter::convert_date("198000").is_err());
        // Year fallback still applies below month precision.
        assert_eq!(PatientConverter::convert_date("1980").unwrap(), "1980");
        assert_eq!(PatientConverter::convert_date("1980abcd").unwrap(), "1980");

        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
             PID|1||ID||DOE^JOHN||198001|M";
        let message = parse_message(hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        assert_eq!(patient.birth_date.as_deref(), Some("1980-01"));
    }

    #[test]
    fn test_convert_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            PatientConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            PatientConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );
        assert_eq!(
            PatientConverter::convert_datetime("202403151000-0500").unwrap(),
            "2024-03-15T10:00-05:00"
        );
        // A date carries no zone: the stem converts without it.
        assert_eq!(
            PatientConverter::convert_datetime("20240315+0500").unwrap(),
            "2024-03-15"
        );
        // An out-of-range zone is ignored; the stem still converts.
        assert_eq!(
            PatientConverter::convert_datetime("20240315100000+9999").unwrap(),
            "2024-03-15T10:00:00"
        );
        assert!(PatientConverter::convert_datetime("202403152500").is_err());

        let mut fields = vec![String::new(); 30];
        fields[0] = "1".to_string();
        fields[28] = "20240315100000+0500".to_string();
        let hl7 = format!(
            "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\rPID|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        assert_eq!(
            patient.deceased_date_time.as_deref(),
            Some("2024-03-15T10:00:00+05:00")
        );
    }

    fn pid_with_telecom(p13: &str, p14: &str) -> String {
        let mut fields = vec![String::new(); 14];
        fields[0] = "1".to_string();
        fields[2] = "ID".to_string();
        fields[4] = "DOE^JOHN".to_string();
        fields[6] = "19800101".to_string();
        fields[7] = "M".to_string();
        fields[12] = p13.to_string();
        fields[13] = p14.to_string();
        format!(
            "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\rPID|{}",
            fields.join("|")
        )
    }

    #[test]
    fn test_telecom_maps_xtn_email_forms() {
        // XTN-4 email with empty component 1 is an email telecom.
        let hl7 = pid_with_telecom("^^^user@example.com", "");
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let telecoms = patient.telecom.as_ref().unwrap();
        assert_eq!(telecoms.len(), 1);
        assert_eq!(telecoms[0].system.as_deref(), Some("email"));
        assert_eq!(telecoms[0].value.as_deref(), Some("user@example.com"));
        assert_eq!(telecoms[0].use_.as_deref(), Some("home"));

        // Phone and email side by side both convert.
        let hl7 = pid_with_telecom("555-1234^^^boss@example.com", "");
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let telecoms = patient.telecom.as_ref().unwrap();
        assert_eq!(telecoms.len(), 2);
        assert_eq!(telecoms[0].system.as_deref(), Some("phone"));
        assert_eq!(telecoms[1].system.as_deref(), Some("email"));

        // An email address in XTN-1 (or an Internet device) is email.
        for xtn in ["user@example.com", "user^WPN^Internet"] {
            let hl7 = pid_with_telecom(xtn, "");
            let message = parse_message(&hl7).unwrap();
            let patient = PatientConverter::convert(&message).unwrap();
            let telecoms = patient.telecom.as_ref().unwrap();
            assert_eq!(telecoms[0].system.as_deref(), Some("email"), "XTN {xtn}");
        }

        // A plain phone number still maps to phone.
        let hl7 = pid_with_telecom("555-1234", "555-5678");
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let telecoms = patient.telecom.as_ref().unwrap();
        assert_eq!(telecoms.len(), 2);
        assert_eq!(telecoms[0].system.as_deref(), Some("phone"));
        assert_eq!(telecoms[0].use_.as_deref(), Some("home"));
        assert_eq!(telecoms[1].use_.as_deref(), Some("work"));
    }

    #[test]
    fn test_telecom_converts_every_repetition() {
        let hl7 = pid_with_telecom("111~222", "");
        let message = parse_message(&hl7).unwrap();
        let patient = PatientConverter::convert(&message).unwrap();
        let telecoms = patient.telecom.as_ref().unwrap();
        assert_eq!(telecoms.len(), 2);
        assert_eq!(telecoms[0].value.as_deref(), Some("111"));
        assert_eq!(telecoms[1].value.as_deref(), Some("222"));
    }
}
