//! Encounter converter - PV1 segment to FHIR Encounter resource
//!
//! Based on HL7 v2-to-FHIR mapping: <https://build.fhir.org/ig/HL7/v2-to-fhir/ConceptMap-segment-pv1-to-encounter.html>

use super::patient::{
    hl7_fraction_suffix, split_hl7_offset, valid_hl7_date, valid_hl7_hour_min, valid_hl7_time,
    valid_hl7_year_month,
};
use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::encounter::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming PV1 segments to FHIR Encounter resources
pub struct EncounterConverter;

impl EncounterConverter {
    /// Convert an HL7 v2 message containing a PV1 segment to a FHIR Encounter resource
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 v2 message containing the PV1 segment
    ///
    /// # Returns
    ///
    /// A FHIR Encounter resource
    ///
    /// # Errors
    ///
    /// Returns an error if the PV1 segment is not found or required fields are missing
    pub fn convert(message: &Message) -> ConversionResult<Encounter> {
        let terser = Terser::new(message);

        // Check if PV1 segment exists
        if !message.segments.iter().any(|s| s.id == "PV1") {
            return Err(ConversionError::MissingSegment("PV1".to_string()));
        }

        // PV1-2: Patient Class -> Encounter.class (required for status determination)
        let patient_class = terser.get("PV1-2").ok().flatten().unwrap_or_default();

        // A valued discharge date means the encounter finished.
        let has_discharge = terser
            .get("PV1-45")
            .ok()
            .flatten()
            .is_some_and(|s| !s.is_empty());
        let status = Self::determine_status(patient_class, has_discharge);

        let mut encounter = Encounter::new(status);

        // PV1-19: Visit Number -> Encounter.identifier
        if let Ok(Some(visit_number)) = terser.get("PV1-19")
            && !visit_number.is_empty()
        {
            encounter.identifier = Some(vec![Identifier {
                use_: Some("official".to_string()),
                type_: Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v2-0203".to_string()),
                        version: None,
                        code: Some("VN".to_string()),
                        display: Some("Visit Number".to_string()),
                    }]),
                    text: Some("Visit Number".to_string()),
                }),
                system: None,
                value: Some(visit_number.to_string()),
                assigner: None,
            }]);
            encounter.id = Some(visit_number.to_string());
        }

        // PV1-2: Patient Class -> Encounter.class
        if !patient_class.is_empty() {
            encounter.class = Some(Self::convert_patient_class(patient_class));
        }

        // PV1-4: Admission Type -> Encounter.type
        if let Ok(Some(admission_type)) = terser.get("PV1-4")
            && !admission_type.is_empty()
        {
            encounter.type_ = Some(vec![CodeableConcept {
                coding: Some(vec![Coding {
                    system: Some("http://terminology.hl7.org/CodeSystem/v2-0007".to_string()),
                    version: None,
                    code: Some(admission_type.to_string()),
                    display: None,
                }]),
                text: Some(admission_type.to_string()),
            }]);
        }

        // PV1-3: Assigned Patient Location (PL) -> Encounter.location.
        // PL components: 1 = point of care, 2 = room, 3 = bed, 4 = facility.
        // A whole-field read returns component 1, so the detail parts
        // must be read as explicit components 2-4.
        if let Ok(Some(point_of_care)) = terser.get("PV1-3")
            && !point_of_care.is_empty()
        {
            let mut parts = vec![point_of_care.to_string()];
            for index in 2..=4 {
                if let Ok(Some(part)) = terser.get(&format!("PV1-3-{index}"))
                    && !part.is_empty()
                {
                    parts.push(part.to_string());
                }
            }

            encounter.location = Some(vec![EncounterLocation {
                location: Reference {
                    reference: Some(format!("Location/{}", point_of_care)),
                    type_: Some("Location".to_string()),
                    identifier: None,
                    display: Some(parts.join(", ")),
                },
                status: Some("active".to_string()),
                period: None,
            }]);
        }

        // PV1-44: Admit Date/Time -> Encounter.period.start
        // PV1-45: Discharge Date/Time -> Encounter.period.end
        // Lenient by design (see crate docs "Invalid datetime handling"):
        // period bounds are ancillary, so an invalid value drops the
        // bound while the encounter still converts.
        let admit_dt = terser.get("PV1-44").ok().flatten();
        let discharge_dt = terser.get("PV1-45").ok().flatten();

        // The wrapper exists only when at least one bound is valued:
        // empty or invalid bounds must not yield an all-None period.
        let start = admit_dt.and_then(|dt| {
            if dt.is_empty() {
                None
            } else {
                Self::convert_datetime(dt).ok()
            }
        });
        let end = discharge_dt.and_then(|dt| {
            if dt.is_empty() {
                None
            } else {
                Self::convert_datetime(dt).ok()
            }
        });
        if start.is_some() || end.is_some() {
            encounter.period = Some(Period { start, end });
        }

        // PV1-7: Attending Doctor -> Encounter.participant
        // PV1-8: Referring Doctor -> Encounter.participant
        // PV1-9: Consulting Doctor -> Encounter.participant
        let mut participants = Vec::new();

        if let Ok(Some(attending)) = terser.get("PV1-7")
            && !attending.is_empty()
            && let Some(participant) = Self::create_participant(&terser, "PV1-7", "ATND")
        {
            participants.push(participant);
        }

        if let Ok(Some(referring)) = terser.get("PV1-8")
            && !referring.is_empty()
            && let Some(participant) = Self::create_participant(&terser, "PV1-8", "REF")
        {
            participants.push(participant);
        }

        if let Ok(Some(consulting)) = terser.get("PV1-9")
            && !consulting.is_empty()
            && let Some(participant) = Self::create_participant(&terser, "PV1-9", "CON")
        {
            participants.push(participant);
        }

        if !participants.is_empty() {
            encounter.participant = Some(participants);
        }

        // PV1-10: Hospital Service -> Encounter.serviceProvider
        if let Ok(Some(service)) = terser.get("PV1-10")
            && !service.is_empty()
        {
            encounter.service_provider = Some(Reference {
                reference: Some(format!("Organization/{}", service)),
                type_: Some("Organization".to_string()),
                identifier: None,
                display: Some(service.to_string()),
            });
        }

        // PV1-14: Admit Source -> Encounter.hospitalization.admitSource
        // PV1-36: Discharge Disposition -> Encounter.hospitalization.dischargeDisposition
        let admit_source = terser.get("PV1-14").ok().flatten();
        let discharge_disp = terser.get("PV1-36").ok().flatten();

        // The wrapper exists only when at least one side is valued:
        // empty fields must not yield an all-None hospitalization.
        let admit_source = admit_source.and_then(|source| {
            if source.is_empty() {
                None
            } else {
                Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v2-0023".to_string()),
                        version: None,
                        code: Some(source.to_string()),
                        display: None,
                    }]),
                    text: Some(source.to_string()),
                })
            }
        });
        let discharge_disposition = discharge_disp.and_then(|disp| {
            if disp.is_empty() {
                None
            } else {
                Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v2-0112".to_string()),
                        version: None,
                        code: Some(disp.to_string()),
                        display: None,
                    }]),
                    text: Some(disp.to_string()),
                })
            }
        });
        if admit_source.is_some() || discharge_disposition.is_some() {
            encounter.hospitalization = Some(EncounterHospitalization {
                pre_admission_identifier: None,
                origin: None,
                admit_source,
                re_admission: None,
                diet_preference: None,
                special_courtesy: None,
                special_arrangement: None,
                destination: None,
                discharge_disposition,
            });
        }

        // Set patient reference if we have patient ID from PID segment
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty()
        {
            encounter.subject = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        Ok(encounter)
    }

    /// Determine encounter status from patient class and discharge.
    fn determine_status(patient_class: &str, has_discharge: bool) -> String {
        if has_discharge {
            return "finished".to_string();
        }
        match patient_class {
            "P" => "planned".to_string(),                 // Preadmit
            "I" | "O" | "E" => "in-progress".to_string(), // Inpatient, Outpatient, Emergency
            _ => "finished".to_string(),                  // Default to finished
        }
    }

    /// Convert HL7 v2 patient class to FHIR encounter class
    fn convert_patient_class(class: &str) -> Coding {
        let (code, display, system) = match class {
            "I" => (
                "IMP",
                "inpatient encounter",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ),
            "O" => (
                "AMB",
                "ambulatory",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ),
            "E" => (
                "EMER",
                "emergency",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ),
            "P" => (
                "PRENC",
                "pre-admission",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ),
            "R" => (
                "AMB",
                "ambulatory",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ), // Recurring
            "B" => (
                "IMP",
                "inpatient encounter",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ), // Obstetrics
            "C" => (
                "AMB",
                "ambulatory",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ), // Commercial Account
            "N" => (
                "IMP",
                "inpatient encounter",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ), // Not Applicable
            "U" => (
                "AMB",
                "ambulatory",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ), // Unknown
            _ => (
                "AMB",
                "ambulatory",
                "http://terminology.hl7.org/CodeSystem/v3-ActCode",
            ),
        };

        Coding {
            system: Some(system.to_string()),
            version: None,
            code: Some(code.to_string()),
            display: Some(display.to_string()),
        }
    }

    /// Create an encounter participant from practitioner info
    fn create_participant(
        terser: &Terser,
        path: &str,
        role_code: &str,
    ) -> Option<EncounterParticipant> {
        // Get practitioner ID (whole-field read returns component 1)
        let id = terser.get(path).ok().flatten()?;
        if id.is_empty() {
            return None;
        }

        // XCN name parts for display, like PractitionerConverter
        // text: prefix (6), given (3) + middle (4), family (2).
        let family = terser
            .get(&format!("{}-2", path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let given = terser
            .get(&format!("{}-3", path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let middle = terser
            .get(&format!("{}-4", path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let prefix = terser
            .get(&format!("{}-6", path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let mut display_parts = Vec::new();
        if let Some(prefix) = prefix {
            display_parts.push(prefix.to_string());
        }
        if let Some(given) = given {
            display_parts.push(given.to_string());
        }
        if let Some(middle) = middle {
            display_parts.push(middle.to_string());
        }
        if let Some(family) = family {
            display_parts.push(family.to_string());
        }
        let display = if display_parts.is_empty() {
            None
        } else {
            Some(display_parts.join(" "))
        };

        let role_display = match role_code {
            "ATND" => "Attending",
            "REF" => "Referrer",
            "CON" => "Consultant",
            _ => "Participant",
        };

        Some(EncounterParticipant {
            type_: Some(vec![CodeableConcept {
                coding: Some(vec![Coding {
                    system: Some(
                        "http://terminology.hl7.org/CodeSystem/v3-ParticipationType".to_string(),
                    ),
                    version: None,
                    code: Some(role_code.to_string()),
                    display: Some(role_display.to_string()),
                }]),
                text: Some(role_display.to_string()),
            }]),
            period: None,
            individual: Some(Reference {
                reference: Some(format!("Practitioner/{}", id)),
                type_: Some("Practitioner".to_string()),
                identifier: None,
                display,
            }),
        })
    }

    /// `YYYYMMDD` -> `YYYY-MM-DD` (`YYYYMM` -> `YYYY-MM`,
    /// `YYYY` stays a year).
    fn convert_date(date: &str) -> ConversionResult<String> {
        let (stem, _) = split_hl7_offset(date);
        let bytes = stem.as_bytes();
        if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) {
                Ok(format!("{}-{}-{}", &stem[0..4], &stem[4..6], &stem[6..8]))
            } else {
                Err(Self::invalid_datetime(date))
            }
        } else if bytes.len() >= 6 && bytes[..6].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_year_month(&stem[0..6]) {
                Ok(format!("{}-{}", &stem[0..4], &stem[4..6]))
            } else {
                Err(Self::invalid_datetime(date))
            }
        } else if bytes.len() >= 4 && bytes[..4].iter().all(|b| b.is_ascii_digit()) {
            Ok(stem[0..4].to_string())
        } else {
            Err(Self::invalid_datetime(date))
        }
    }

    /// Convert HL7 v2 datetime format to FHIR datetime format
    ///
    /// `YYYYMMDDHHMM` keeps its minutes (`YYYY-MM-DDTHH:MM`) and a
    /// trailing HL7 zone (`±HHMM`) is preserved as `±HH:MM`.
    /// Validated fractional seconds are kept as `.digits`.
    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        let (stem, offset) = split_hl7_offset(datetime);
        let zone = offset.as_deref().unwrap_or("");
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
        let bytes = stem.as_bytes();
        if bytes.len() >= 14 && bytes[..14].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) && valid_hl7_time(&stem[8..14]) {
                Ok(format!(
                    "{}-{}-{}T{}:{}:{}{}{}",
                    &stem[0..4],
                    &stem[4..6],
                    &stem[6..8],
                    &stem[8..10],
                    &stem[10..12],
                    &stem[12..14],
                    hl7_fraction_suffix(stem, 14),
                    zone
                ))
            } else {
                Err(Self::invalid_datetime(datetime))
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
                Err(Self::invalid_datetime(datetime))
            }
        } else if bytes.len() >= 4 {
            // Date, month, or year precision, like the patient
            // converter.
            Self::convert_date(stem)
        } else {
            Err(Self::invalid_datetime(datetime))
        }
    }


    /// Redacted bad-datetime error: field location + reason only, never the value.
    fn invalid_datetime(_datetime: &str) -> ConversionError {
        ConversionError::InvalidFormat(
            "datetime".to_string(),
            "PV1".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_inpatient_encounter() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\r\
                   PV1|1|I|4N^401^01||||1234567^SMITH^JANE^^^DR^MD|||CARDIO";

        let message = parse_message(hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();

        assert_eq!(encounter.resource_type, "Encounter");
        assert_eq!(encounter.status, "in-progress");

        // Check class
        let class = encounter.class.unwrap();
        assert_eq!(class.code, Some("IMP".to_string()));
        assert_eq!(class.display, Some("inpatient encounter".to_string()));

        // Check patient reference
        let subject = encounter.subject.unwrap();
        assert_eq!(subject.reference, Some("Patient/MRN123".to_string()));
    }

    #[test]
    fn test_convert_outpatient_encounter() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A04|12345|P|2.5\r\
                   PID|1||MRN456^^^MRN||SMITH^JANE||19900220|F\r\
                   PV1|1|O|CLINIC||||9876543^JONES^ROBERT^^^DR^DO";

        let message = parse_message(hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();

        assert_eq!(encounter.resource_type, "Encounter");
        assert_eq!(encounter.status, "in-progress");

        // Check class
        let class = encounter.class.unwrap();
        assert_eq!(class.code, Some("AMB".to_string()));
        assert_eq!(class.display, Some("ambulatory".to_string()));
    }

    #[test]
    fn test_non_ascii_admit_datetime_is_dropped_without_panic() {
        // PV1-44/PV1-45 are lenient (.ok()), but must never panic.
        let mut pv1 = String::from("PV1|1|I");
        for _ in 3..44 {
            pv1.push('|');
        }
        pv1.push_str("|123\u{e9}56789012");
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\r\
             {pv1}"
        );

        let message = parse_message(&hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        assert!(encounter.period.is_none());
        assert!(EncounterConverter::convert_datetime("123\u{e9}56789012").is_err());
    }

    #[test]
    fn test_impossible_admit_datetime_dropped() {
        assert!(EncounterConverter::convert_datetime("20240230100000").is_err());

        // PV1-44/PV1-45 are ancillary: an impossible date is dropped,
        // never emitted, while the encounter still converts.
        let mut pv1 = String::from("PV1|1|I");
        for _ in 3..44 {
            pv1.push('|');
        }
        pv1.push_str("|20240230100000");
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\r\
             {pv1}"
        );

        let message = parse_message(&hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        assert!(encounter.period.is_none());
    }

    #[test]
    fn test_empty_period_and_hospitalization_omitted() {
        // PV1-44/45 present-but-empty (PV1-46 forces presence):
        // no all-None period.
        let mut pv1 = vec![String::new(); 46];
        pv1[0] = "1".to_string();
        pv1[1] = "I".to_string();
        pv1[45] = "X".to_string();
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\r\
             PV1|{}",
            pv1.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        assert!(encounter.period.is_none());

        // PV1-14/36 present-but-empty (PV1-37 forces presence): no
        // all-None hospitalization.
        let mut pv1 = vec![String::new(); 37];
        pv1[0] = "1".to_string();
        pv1[1] = "I".to_string();
        pv1[36] = "X".to_string();
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\r\
             PV1|{}",
            pv1.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        assert!(encounter.hospitalization.is_none());
    }

    #[test]
    fn test_encounter_with_participants() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN789|||TEST^PATIENT||19700101|M\r\
                   PV1|1|I||||111^ATT^DOCTOR|222^REF^DOCTOR|333^CON^DOCTOR";

        let message = parse_message(hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();

        assert_eq!(encounter.resource_type, "Encounter");

        // Check that participants were created
        let participants = encounter.participant.unwrap();
        assert!(!participants.is_empty()); // At least attending doctor should be present

        // Check that we have practitioner references
        assert!(participants[0].individual.is_some());
        let individual = participants[0].individual.as_ref().unwrap();
        assert!(individual.reference.is_some());
        assert!(
            individual
                .reference
                .as_ref()
                .unwrap()
                .contains("Practitioner/")
        );
    }

    #[test]
    fn test_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            EncounterConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            EncounterConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );
    }

    #[test]
    fn test_location_uses_pl_room_bed_facility() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
                   PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\r\
                   PV1|1|I|4N^401^01^HOSP";
        let message = parse_message(hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        let location = &encounter.location.unwrap()[0].location;
        assert_eq!(location.reference, Some("Location/4N".to_string()));
        assert_eq!(location.display, Some("4N, 401, 01, HOSP".to_string()));
    }

    #[test]
    fn test_datetime_preserves_fractional_seconds() {
        assert_eq!(
            EncounterConverter::convert_datetime("20240315100000.123").unwrap(),
            "2024-03-15T10:00:00.123"
        );
    }

    #[test]
    fn test_discharge_present_marks_finished() {
        let mut pv1 = vec![String::new(); 45];
        pv1[0] = "1".to_string();
        pv1[1] = "I".to_string();
        pv1[44] = "20240315120000".to_string();
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\rPV1|{}",
            pv1.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        assert_eq!(encounter.status, "finished");
    }

    #[test]
    fn test_datetime_accepts_year_month_precision() {
        assert_eq!(
            EncounterConverter::convert_datetime("202401").unwrap(),
            "2024-01"
        );
        assert_eq!(
            EncounterConverter::convert_datetime("1980").unwrap(),
            "1980"
        );
    }

    #[test]
    fn test_participant_display_includes_given_name() {
        // PV1-7 family + given both appear in display.
        let mut pv1 = vec![String::new(); 10];
        pv1[0] = "1".to_string();
        pv1[1] = "I".to_string();
        pv1[6] = "123^SMITH^JANE".to_string();
        let hl7 = format!(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\rPV1|{}",
            pv1.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        let participant = &encounter.participant.as_ref().unwrap()[0];
        let display = participant
            .individual
            .as_ref()
            .unwrap()
            .display
            .as_deref()
            .unwrap();
        assert!(display.contains("SMITH"), "family missing: {display}");
        assert!(display.contains("JANE"), "given missing: {display}");
    }
}
