//! Encounter reverse converter - FHIR Encounter resource to PV1 segment
//!
//! Converts FHIR Encounter resources back to HL7 v2.x PV1 segments.

use super::split_trailing_zone;
use crate::error::ConversionResult;
use crate::resources::common::*;
use crate::resources::encounter::Encounter;
use vyna_hl7_core::segment::Segment;

/// Reverse converter for transforming FHIR Encounter resources to PV1 segments
pub struct EncounterReverseConverter;

impl EncounterReverseConverter {
    /// Convert a FHIR Encounter resource to an HL7 v2 PV1 segment
    ///
    /// # Arguments
    ///
    /// * `encounter` - The FHIR Encounter resource
    ///
    /// # Returns
    ///
    /// An HL7 v2 PV1 segment
    pub fn convert(encounter: &Encounter) -> ConversionResult<Segment> {
        let mut pv1 = Segment::new("PV1");

        // PV1-1: Set ID
        let _ = pv1.set_field_value(1, "1");

        // PV1-2: Patient Class
        if let Some(ref class) = encounter.class {
            let patient_class = Self::convert_class_to_hl7(class);
            let _ = pv1.set_field_value(2, &patient_class);
        }

        // PV1-3: Assigned Patient Location (PL: point of care ^
        // room ^ bed ^ facility). The forward converter joins the
        // PL parts into the display ("ER, 101, 1") with the point of
        // care as `Location/{id}`; splitting the display back into
        // components (with component 1 from the reference id) keeps
        // the reference stable across a re-forward. Writing the
        // display as a single PL component would corrupt the
        // reference to `Location/ER, 101, 1`.
        if let Some(ref locations) = encounter.location
            && let Some(first_loc) = locations.first()
        {
            // location is a required Reference field (not Optional)
            let location = &first_loc.location;
            let mut parts: Vec<&str> = location
                .display
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|display| display.split(", ").collect())
                .unwrap_or_default();
            if let Some(id) = location
                .reference
                .as_deref()
                .and_then(|r| r.strip_prefix("Location/"))
                .filter(|s| !s.is_empty())
            {
                if parts.is_empty() {
                    parts.push(id);
                } else {
                    parts[0] = id;
                }
            }
            for (index, part) in parts.iter().take(4).enumerate() {
                let _ = pv1.set_component(3, 0, index, *part);
            }
            // A non-Location reference with no display keeps the old
            // whole-field behavior so nothing is silently dropped.
            if parts.is_empty()
                && let Some(ref reference) = location.reference
                && !reference.is_empty()
            {
                let _ = pv1.set_field_value(3, reference);
            }
        }

        // PV1-4: Admission Type (round-trips Encounter.type_, which
        // the forward converter populates from PV1-4)
        if let Some(ref types) = encounter.type_
            && let Some(first_type) = types.first()
            && let Some(ref coding) = first_type.coding
            && let Some(first_coding) = coding.first()
            && let Some(ref code) = first_coding.code
        {
            let _ = pv1.set_field_value(4, code);
        }

        // PV1-7: Attending Doctor (from participant with type ATND)
        // PV1-8: Referring Doctor (from participant with type REF)
        // PV1-9: Consulting Doctor (from participant with type CON)
        // Each role repeats (~) for multiple participants. The XCN-1 id
        // comes from the reference ("Practitioner/{id}"); the display is
        // the family name (XCN-2), falling back to XCN-1 when no
        // reference exists.
        let mut reps = [0usize; 3];
        if let Some(ref participants) = encounter.participant {
            for participant in participants {
                if let Some(ref types) = participant.type_ {
                    for type_ in types {
                        if let Some(ref coding) = type_.coding {
                            for code in coding {
                                if let Some(ref code_val) = code.code
                                    && let Some(ref individual) = participant.individual
                                    && let Some((field, slot)) = match code_val.as_str() {
                                        "ATND" => Some((7, 0)),
                                        "REF" => Some((8, 1)),
                                        "CON" => Some((9, 2)),
                                        _ => None,
                                    }
                                {
                                    let id = individual
                                        .reference
                                        .as_deref()
                                        .and_then(|r| r.rsplit('/').next())
                                        .filter(|s| !s.is_empty());
                                    let display =
                                        individual.display.as_deref().filter(|s| !s.is_empty());
                                    // Skip participants carrying neither.
                                    let Some(value) = id.or(display) else {
                                        continue;
                                    };
                                    let rep = reps[slot];
                                    reps[slot] += 1;
                                    let _ = pv1.set_component(field, rep, 0, value);
                                    if id.is_some()
                                        && let Some(display) = display
                                    {
                                        let _ = pv1.set_component(field, rep, 1, display);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // PV1-10: Hospital Service (round-trips
        // Encounter.service_provider, which the forward converter
        // populates from PV1-10 as `Organization/{service}` with the
        // service code as display)
        if let Some(ref provider) = encounter.service_provider {
            let service = provider.display.as_deref().or_else(|| {
                provider
                    .reference
                    .as_deref()
                    .and_then(|r| r.strip_prefix("Organization/"))
            });
            if let Some(service) = service
                && !service.is_empty()
            {
                let _ = pv1.set_field_value(10, service);
            }
        }

        // PV1-14: Admit Source
        if let Some(ref hospitalization) = encounter.hospitalization {
            if let Some(ref admit_source) = hospitalization.admit_source
                && let Some(ref coding) = admit_source.coding
                && let Some(first_coding) = coding.first()
                && let Some(ref code) = first_coding.code
            {
                let _ = pv1.set_field_value(14, code);
            }

            // PV1-36: Discharge Disposition
            if let Some(ref discharge_disposition) = hospitalization.discharge_disposition
                && let Some(ref coding) = discharge_disposition.coding
                && let Some(first_coding) = coding.first()
                && let Some(ref code) = first_coding.code
            {
                let _ = pv1.set_field_value(36, code);
            }
        }

        // PV1-19: Visit Number
        if let Some(ref identifiers) = encounter.identifier
            && let Some(first_id) = identifiers.first()
            && let Some(ref value) = first_id.value
        {
            let _ = pv1.set_field_value(19, value);
        }

        // PV1-44: Admit Date/Time
        if let Some(ref period) = encounter.period {
            if let Some(ref start) = period.start {
                let hl7_datetime = Self::convert_datetime_to_hl7(start);
                let _ = pv1.set_field_value(44, &hl7_datetime);
            }

            // PV1-45: Discharge Date/Time
            if let Some(ref end) = period.end {
                let hl7_datetime = Self::convert_datetime_to_hl7(end);
                let _ = pv1.set_field_value(45, &hl7_datetime);
            }
        }

        Ok(pv1)
    }

    /// Convert FHIR encounter class to HL7 v2 patient class.
    ///
    /// Observation (`OBSENC`) and short-stay (`SS`) have no distinct
    /// HL7 Table 0004 letter: `B` is obstetrics and re-forwards to
    /// `IMP`, so both fold to the outpatient default `O` (which
    /// re-forwards to `AMB` and stays stable).
    fn convert_class_to_hl7(class: &Coding) -> String {
        if let Some(ref code) = class.code {
            match code.as_str() {
                "IMP" | "ACUTE" | "inpatient" => "I",       // Inpatient
                "AMB" | "ambulatory" | "outpatient" => "O", // Outpatient
                "EMER" | "emergency" => "E",                // Emergency
                "PRENC" | "preadmit" => "P",                // Preadmit
                // No distinct Table 0004 letter (see doc comment).
                "OBSENC" | "observation" | "SS" | "short-stay" => "O",
                _ => "O", // Default to outpatient
            }
            .to_string()
        } else {
            "O".to_string()
        }
    }

    /// Convert FHIR datetime to HL7 v2 datetime.
    ///
    /// A trailing `Z` becomes `+0000`: a bare `Z` is not valid HL7 DTM.
    /// A trailing numeric zone is split off before separators are
    /// stripped so a negative offset keeps its sign.
    fn convert_datetime_to_hl7(datetime: &str) -> String {
        let zoned = match datetime.strip_suffix('Z') {
            Some(stem) => format!("{stem}+0000"),
            None => datetime.to_string(),
        };
        let (stem, zone) = split_trailing_zone(&zoned);
        format!("{}{}", stem.replace(['-', ':', 'T'], ""), zone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::encounter::EncounterHospitalization;

    fn create_test_encounter() -> Encounter {
        let mut encounter = Encounter::new("finished".to_string());

        encounter.identifier = Some(vec![Identifier {
            use_: None,
            type_: None,
            system: Some("http://hospital.org/visit".to_string()),
            value: Some("V12345".to_string()),
            assigner: None,
        }]);

        encounter.class = Some(Coding {
            system: Some("http://terminology.hl7.org/CodeSystem/v3-ActCode".to_string()),
            version: None,
            code: Some("IMP".to_string()),
            display: Some("inpatient".to_string()),
        });

        encounter.period = Some(Period {
            start: Some("2024-01-15T10:00:00".to_string()),
            end: Some("2024-01-20T14:30:00".to_string()),
        });

        encounter.hospitalization = Some(EncounterHospitalization {
            pre_admission_identifier: None,
            origin: None,
            admit_source: Some(CodeableConcept {
                coding: Some(vec![Coding {
                    system: None,
                    version: None,
                    code: Some("7".to_string()),
                    display: Some("Emergency Room".to_string()),
                }]),
                text: None,
            }),
            re_admission: None,
            diet_preference: None,
            special_courtesy: None,
            special_arrangement: None,
            destination: None,
            discharge_disposition: Some(CodeableConcept {
                coding: Some(vec![Coding {
                    system: None,
                    version: None,
                    code: Some("01".to_string()),
                    display: Some("Discharged to home".to_string()),
                }]),
                text: None,
            }),
        });

        encounter
    }

    #[test]
    fn test_convert_encounter_to_pv1() {
        let encounter = create_test_encounter();
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();

        assert_eq!(pv1.id, "PV1");
        assert_eq!(pv1.get_field_value(1), Some("1"));
        assert_eq!(pv1.get_field_value(2), Some("I")); // Inpatient
    }

    #[test]
    fn test_convert_class() {
        let coding = Coding {
            system: None,
            version: None,
            code: Some("IMP".to_string()),
            display: None,
        };
        assert_eq!(
            EncounterReverseConverter::convert_class_to_hl7(&coding),
            "I"
        );

        let coding2 = Coding {
            system: None,
            version: None,
            code: Some("AMB".to_string()),
            display: None,
        };
        assert_eq!(
            EncounterReverseConverter::convert_class_to_hl7(&coding2),
            "O"
        );

        let coding3 = Coding {
            system: None,
            version: None,
            code: Some("EMER".to_string()),
            display: None,
        };
        assert_eq!(
            EncounterReverseConverter::convert_class_to_hl7(&coding3),
            "E"
        );
    }

    #[test]
    fn test_location_pl_structure_round_trips() {
        use crate::converters::encounter::EncounterConverter;
        use vyna_hl7_core::delimiters::Delimiters;
        use vyna_hl7_parser::parse_message;

        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
                   PID|1||1||A^B||19800101|M\r\
                   PV1|1|I|ER^101^1";
        let message = parse_message(hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        // PL components are preserved, not flattened to the display.
        let field = pv1.get_field(3).unwrap();
        let comp = |i: usize| field.repetitions[0].components[i].value();
        assert_eq!(
            (comp(0), comp(1), comp(2)),
            (Some("ER"), Some("101"), Some("1"))
        );
        // Re-forward keeps the Location reference intact.
        let hl7 = format!(
            "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
             PID|1||1||A^B||19800101|M\r{}",
            pv1.encode(&Delimiters::default())
        );
        let back = EncounterConverter::convert(&parse_message(&hl7).unwrap()).unwrap();
        let location = &back.location.as_ref().unwrap()[0].location;
        assert_eq!(location.reference.as_deref(), Some("Location/ER"));
        assert_eq!(location.display.as_deref(), Some("ER, 101, 1"));
    }

    #[test]
    fn test_location_reference_without_display() {
        let mut encounter = Encounter::new("finished".to_string());
        encounter.location = Some(vec![crate::resources::encounter::EncounterLocation {
            location: Reference {
                reference: Some("Location/ER".to_string()),
                type_: Some("Location".to_string()),
                identifier: None,
                display: None,
            },
            status: Some("active".to_string()),
            period: None,
        }]);
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        // The reference id becomes PL component 1, not the whole reference.
        assert_eq!(pv1.get_field_value(3), Some("ER"));
    }

    #[test]
    fn test_observation_and_short_stay_fold_to_outpatient() {
        for code in ["OBSENC", "observation", "SS", "short-stay"] {
            let coding = Coding {
                system: None,
                version: None,
                code: Some(code.to_string()),
                display: None,
            };
            // Never B: B is obstetrics and re-forwards to IMP.
            assert_eq!(
                EncounterReverseConverter::convert_class_to_hl7(&coding),
                "O",
                "class {code}"
            );
        }
    }

    #[test]
    fn test_admission_type_and_service_round_trip() {
        use crate::converters::encounter::EncounterConverter;
        use vyna_hl7_parser::parse_message;

        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123^^^MRN||DOE^JOHN||19800101|M\r\
                   PV1|1|I|4N^401^01|EL||||||CARDIO";

        let message = parse_message(hl7).unwrap();
        let encounter = EncounterConverter::convert(&message).unwrap();
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();

        // PV1-4 round-trips through Encounter.type_, PV1-10 through
        // Encounter.service_provider.
        assert_eq!(pv1.get_field_value(4), Some("EL"));
        assert_eq!(pv1.get_field_value(10), Some("CARDIO"));
    }

    #[test]
    fn test_service_provider_reference_without_display() {
        let mut encounter = Encounter::new("finished".to_string());
        encounter.service_provider = Some(Reference {
            reference: Some("Organization/NEURO".to_string()),
            type_: Some("Organization".to_string()),
            identifier: None,
            display: None,
        });

        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        assert_eq!(pv1.get_field_value(10), Some("NEURO"));
    }

    fn participant(
        role: &str,
        reference: Option<&str>,
        display: Option<&str>,
    ) -> crate::resources::encounter::EncounterParticipant {
        use crate::resources::encounter::EncounterParticipant;
        EncounterParticipant {
            type_: Some(vec![CodeableConcept {
                coding: Some(vec![Coding {
                    system: None,
                    version: None,
                    code: Some(role.to_string()),
                    display: None,
                }]),
                text: None,
            }]),
            period: None,
            individual: Some(Reference {
                reference: reference.map(|s| s.to_string()),
                type_: Some("Practitioner".to_string()),
                identifier: None,
                display: display.map(|s| s.to_string()),
            }),
        }
    }

    #[test]
    fn test_participant_writes_practitioner_id() {
        let mut encounter = Encounter::new("finished".to_string());
        encounter.participant = Some(vec![participant(
            "ATND",
            Some("Practitioner/DOC123"),
            Some("Smith"),
        )]);
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        // The id lands in XCN-1; the display stays in the name component.
        assert_eq!(pv1.get_field_value(7), Some("DOC123"));
        let family = pv1
            .get_field(7)
            .and_then(|f| f.repetitions.first())
            .and_then(|r| r.components.get(1))
            .and_then(|c| c.value());
        assert_eq!(family, Some("Smith"));

        // Without a reference the display still fills the field.
        let mut encounter = Encounter::new("finished".to_string());
        encounter.participant = Some(vec![participant("REF", None, Some("Jones"))]);
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        assert_eq!(pv1.get_field_value(8), Some("Jones"));
    }

    #[test]
    fn test_participant_round_trips_through_pv1() {
        use crate::converters::encounter::EncounterConverter;
        use vyna_hl7_core::delimiters::Delimiters;
        use vyna_hl7_parser::parse_message;

        let mut encounter = Encounter::new("finished".to_string());
        encounter.participant = Some(vec![participant(
            "ATND",
            Some("Practitioner/DOC123"),
            Some("Smith"),
        )]);
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        let hl7 = format!(
            "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
             PID|1||1||A^B||19800101|M\r{}",
            pv1.encode(&Delimiters::default())
        );
        let back = EncounterConverter::convert(&parse_message(&hl7).unwrap()).unwrap();
        let individual = back.participant.as_ref().unwrap()[0]
            .individual
            .as_ref()
            .unwrap();
        assert_eq!(individual.reference.as_deref(), Some("Practitioner/DOC123"));
        assert_eq!(individual.display.as_deref(), Some("Smith"));
    }

    #[test]
    fn test_multiple_participants_emit_repetitions() {
        let mut encounter = Encounter::new("finished".to_string());
        encounter.participant = Some(vec![
            participant("ATND", Some("Practitioner/D1"), Some("Doc1")),
            participant("ATND", Some("Practitioner/D2"), Some("Doc2")),
        ]);
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        let field = pv1.get_field(7).unwrap();
        assert_eq!(field.repetitions.len(), 2);
        assert_eq!(field.repetitions[0].components[0].value(), Some("D1"));
        assert_eq!(field.repetitions[1].components[0].value(), Some("D2"));
    }

    #[test]
    fn test_visit_number() {
        let encounter = create_test_encounter();
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();

        assert_eq!(pv1.get_field_value(19), Some("V12345"));
    }

    #[test]
    fn test_period_datetimes_map_trailing_z_to_offset() {
        let mut encounter = Encounter::new("finished".to_string());
        encounter.period = Some(Period {
            start: Some("2024-01-15T10:00:00Z".to_string()),
            end: Some("2024-01-20T14:30:00Z".to_string()),
        });
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        assert_eq!(pv1.get_field_value(44), Some("20240115100000+0000"));
        assert_eq!(pv1.get_field_value(45), Some("20240120143000+0000"));
        // A negative offset keeps its sign: the '-' is a zone sign,
        // not a date separator.
        let mut encounter = Encounter::new("finished".to_string());
        encounter.period = Some(Period {
            start: Some("2024-01-15T10:00:00-05:00".to_string()),
            end: Some("2024-01-20T14:30:00-05:00".to_string()),
        });
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();
        assert_eq!(pv1.get_field_value(44), Some("20240115100000-0500"));
        assert_eq!(pv1.get_field_value(45), Some("20240120143000-0500"));
    }

    #[test]
    fn test_admit_discharge_dates() {
        let encounter = create_test_encounter();
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();

        // Dates should be converted to HL7 format (without separators)
        assert!(pv1.get_field_value(44).is_some());
        assert!(pv1.get_field_value(45).is_some());
    }
}
