//! Encounter converter - PV1 segment to FHIR Encounter resource
//!
//! Based on HL7 v2-to-FHIR mapping: <https://build.fhir.org/ig/HL7/v2-to-fhir/ConceptMap-segment-pv1-to-encounter.html>

use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::encounter::*;
use crate::resources::common::*;

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

        // Determine status based on context (default to "finished" for completed encounters)
        let status = Self::determine_status(patient_class);

        let mut encounter = Encounter::new(status);

        // PV1-19: Visit Number -> Encounter.identifier
        if let Ok(Some(visit_number)) = terser.get("PV1-19")
            && !visit_number.is_empty() {
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
            && !admission_type.is_empty() {
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

        // PV1-3: Assigned Patient Location -> Encounter.location
        if let Ok(Some(location)) = terser.get("PV1-3")
            && !location.is_empty() {
                let mut location_display = location.to_string();

                // Try to get more detailed location info
                if let Ok(Some(room)) = terser.get("PV1-3-1")
                    && !room.is_empty() {
                        location_display = room.to_string();
                    }

                encounter.location = Some(vec![EncounterLocation {
                    location: Reference {
                        reference: Some(format!("Location/{}", location)),
                        type_: Some("Location".to_string()),
                        identifier: None,
                        display: Some(location_display),
                    },
                    status: Some("active".to_string()),
                    period: None,
                }]);
            }

        // PV1-44: Admit Date/Time -> Encounter.period.start
        // PV1-45: Discharge Date/Time -> Encounter.period.end
        let admit_dt = terser.get("PV1-44").ok().flatten();
        let discharge_dt = terser.get("PV1-45").ok().flatten();

        if admit_dt.is_some() || discharge_dt.is_some() {
            encounter.period = Some(Period {
                start: admit_dt.and_then(|dt| {
                    if dt.is_empty() { None } else { Self::convert_datetime(dt).ok() }
                }),
                end: discharge_dt.and_then(|dt| {
                    if dt.is_empty() { None } else { Self::convert_datetime(dt).ok() }
                }),
            });
        }

        // PV1-7: Attending Doctor -> Encounter.participant
        // PV1-8: Referring Doctor -> Encounter.participant
        // PV1-9: Consulting Doctor -> Encounter.participant
        let mut participants = Vec::new();

        if let Ok(Some(attending)) = terser.get("PV1-7")
            && !attending.is_empty()
                && let Some(participant) = Self::create_participant(&terser, "PV1-7", "ATND") {
                    participants.push(participant);
                }

        if let Ok(Some(referring)) = terser.get("PV1-8")
            && !referring.is_empty()
                && let Some(participant) = Self::create_participant(&terser, "PV1-8", "REF") {
                    participants.push(participant);
                }

        if let Ok(Some(consulting)) = terser.get("PV1-9")
            && !consulting.is_empty()
                && let Some(participant) = Self::create_participant(&terser, "PV1-9", "CON") {
                    participants.push(participant);
                }

        if !participants.is_empty() {
            encounter.participant = Some(participants);
        }

        // PV1-10: Hospital Service -> Encounter.serviceProvider
        if let Ok(Some(service)) = terser.get("PV1-10")
            && !service.is_empty() {
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

        if admit_source.is_some() || discharge_disp.is_some() {
            let mut hospitalization = EncounterHospitalization {
                pre_admission_identifier: None,
                origin: None,
                admit_source: None,
                re_admission: None,
                diet_preference: None,
                special_courtesy: None,
                special_arrangement: None,
                destination: None,
                discharge_disposition: None,
            };

            if let Some(source) = admit_source
                && !source.is_empty() {
                    hospitalization.admit_source = Some(CodeableConcept {
                        coding: Some(vec![Coding {
                            system: Some("http://terminology.hl7.org/CodeSystem/v2-0023".to_string()),
                            version: None,
                            code: Some(source.to_string()),
                            display: None,
                        }]),
                        text: Some(source.to_string()),
                    });
                }

            if let Some(disp) = discharge_disp
                && !disp.is_empty() {
                    hospitalization.discharge_disposition = Some(CodeableConcept {
                        coding: Some(vec![Coding {
                            system: Some("http://terminology.hl7.org/CodeSystem/v2-0112".to_string()),
                            version: None,
                            code: Some(disp.to_string()),
                            display: None,
                        }]),
                        text: Some(disp.to_string()),
                    });
                }

            encounter.hospitalization = Some(hospitalization);
        }

        // Set patient reference if we have patient ID from PID segment
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty() {
                encounter.subject = Some(Reference {
                    reference: Some(format!("Patient/{}", patient_id)),
                    type_: Some("Patient".to_string()),
                    identifier: None,
                    display: None,
                });
            }

        Ok(encounter)
    }

    /// Determine encounter status from patient class and other context
    fn determine_status(patient_class: &str) -> String {
        // In a real implementation, you might determine this from:
        // - Discharge date (if present, status is "finished")
        // - Cancelled flag
        // - Current state flags
        // For now, we use a simple mapping
        match patient_class {
            "P" => "planned".to_string(),      // Preadmit
            "I" | "O" | "E" => "in-progress".to_string(), // Inpatient, Outpatient, Emergency
            _ => "finished".to_string(),        // Default to finished
        }
    }

    /// Convert HL7 v2 patient class to FHIR encounter class
    fn convert_patient_class(class: &str) -> Coding {
        let (code, display, system) = match class {
            "I" => ("IMP", "inpatient encounter", "http://terminology.hl7.org/CodeSystem/v3-ActCode"),
            "O" => ("AMB", "ambulatory", "http://terminology.hl7.org/CodeSystem/v3-ActCode"),
            "E" => ("EMER", "emergency", "http://terminology.hl7.org/CodeSystem/v3-ActCode"),
            "P" => ("PRENC", "pre-admission", "http://terminology.hl7.org/CodeSystem/v3-ActCode"),
            "R" => ("AMB", "ambulatory", "http://terminology.hl7.org/CodeSystem/v3-ActCode"), // Recurring
            "B" => ("IMP", "inpatient encounter", "http://terminology.hl7.org/CodeSystem/v3-ActCode"), // Obstetrics
            "C" => ("AMB", "ambulatory", "http://terminology.hl7.org/CodeSystem/v3-ActCode"), // Commercial Account
            "N" => ("IMP", "inpatient encounter", "http://terminology.hl7.org/CodeSystem/v3-ActCode"), // Not Applicable
            "U" => ("AMB", "ambulatory", "http://terminology.hl7.org/CodeSystem/v3-ActCode"), // Unknown
            _ => ("AMB", "ambulatory", "http://terminology.hl7.org/CodeSystem/v3-ActCode"),
        };

        Coding {
            system: Some(system.to_string()),
            version: None,
            code: Some(code.to_string()),
            display: Some(display.to_string()),
        }
    }

    /// Create an encounter participant from practitioner info
    fn create_participant(terser: &Terser, path: &str, role_code: &str) -> Option<EncounterParticipant> {
        // Get practitioner ID (component 0 in 0-based indexing)
        let id = terser.get(path).ok().flatten()?;
        if id.is_empty() {
            return None;
        }

        // Get practitioner name (component 1 in 0-based indexing = family name)
        let name = terser.get(&format!("{}-1", path)).ok().flatten();

        let role_display = match role_code {
            "ATND" => "Attending",
            "REF" => "Referrer",
            "CON" => "Consultant",
            _ => "Participant",
        };

        Some(EncounterParticipant {
            type_: Some(vec![CodeableConcept {
                coding: Some(vec![Coding {
                    system: Some("http://terminology.hl7.org/CodeSystem/v3-ParticipationType".to_string()),
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
                display: name.map(|s| s.to_string()),
            }),
        })
    }

    /// Convert HL7 v2 datetime format to FHIR datetime format
    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        if datetime.len() >= 14 {
            Ok(format!(
                "{}-{}-{}T{}:{}:{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8],
                &datetime[8..10],
                &datetime[10..12],
                &datetime[12..14]
            ))
        } else if datetime.len() >= 8 {
            Ok(format!(
                "{}-{}-{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8]
            ))
        } else {
            Err(ConversionError::InvalidFormat(
                "datetime".to_string(),
                "PV1".to_string(),
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
        assert!(individual.reference.as_ref().unwrap().contains("Practitioner/"));
    }
}
