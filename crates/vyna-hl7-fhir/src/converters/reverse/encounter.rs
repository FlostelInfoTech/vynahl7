//! Encounter reverse converter - FHIR Encounter resource to PV1 segment
//!
//! Converts FHIR Encounter resources back to HL7 v2.x PV1 segments.

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

        // PV1-3: Assigned Patient Location
        if let Some(ref locations) = encounter.location
            && let Some(first_loc) = locations.first() {
                // location is a required Reference field (not Optional)
                let location = &first_loc.location;
                if let Some(ref display) = location.display {
                    let _ = pv1.set_field_value(3, display);
                } else if let Some(ref reference) = location.reference {
                    let _ = pv1.set_field_value(3, reference);
                }
            }

        // PV1-4: Admission Type
        if let Some(ref priority) = encounter.priority
            && let Some(ref coding) = priority.coding
                && let Some(first_coding) = coding.first()
                    && let Some(ref code) = first_coding.code {
                        let _ = pv1.set_field_value(4, code);
                    }

        // PV1-7: Attending Doctor (from participant with type ATND)
        // PV1-8: Referring Doctor (from participant with type REF)
        // PV1-9: Consulting Doctor (from participant with type CON)
        if let Some(ref participants) = encounter.participant {
            for participant in participants {
                if let Some(ref types) = participant.type_ {
                    for type_ in types {
                        if let Some(ref coding) = type_.coding {
                            for code in coding {
                                if let Some(ref code_val) = code.code
                                    && let Some(ref individual) = participant.individual {
                                        let doctor_ref = individual
                                            .display
                                            .as_ref()
                                            .or(individual.reference.as_ref())
                                            .cloned()
                                            .unwrap_or_default();

                                        match code_val.as_str() {
                                            "ATND" => {
                                                let _ = pv1.set_field_value(7, &doctor_ref);
                                            }
                                            "REF" => {
                                                let _ = pv1.set_field_value(8, &doctor_ref);
                                            }
                                            "CON" => {
                                                let _ = pv1.set_field_value(9, &doctor_ref);
                                            }
                                            _ => {}
                                        }
                                    }
                            }
                        }
                    }
                }
            }
        }

        // PV1-10: Hospital Service (from Encounter.type_)
        if let Some(ref types) = encounter.type_
            && let Some(first_type) = types.first()
                && let Some(ref coding) = first_type.coding
                    && let Some(first_coding) = coding.first()
                        && let Some(ref code) = first_coding.code {
                            let _ = pv1.set_field_value(10, code);
                        }

        // PV1-14: Admit Source
        if let Some(ref hospitalization) = encounter.hospitalization {
            if let Some(ref admit_source) = hospitalization.admit_source
                && let Some(ref coding) = admit_source.coding
                    && let Some(first_coding) = coding.first()
                        && let Some(ref code) = first_coding.code {
                            let _ = pv1.set_field_value(14, code);
                        }

            // PV1-36: Discharge Disposition
            if let Some(ref discharge_disposition) = hospitalization.discharge_disposition
                && let Some(ref coding) = discharge_disposition.coding
                    && let Some(first_coding) = coding.first()
                        && let Some(ref code) = first_coding.code {
                            let _ = pv1.set_field_value(36, code);
                        }
        }

        // PV1-19: Visit Number
        if let Some(ref identifiers) = encounter.identifier
            && let Some(first_id) = identifiers.first()
                && let Some(ref value) = first_id.value {
                    let _ = pv1.set_field_value(19, value);
                }

        // PV1-44: Admit Date/Time
        if let Some(ref period) = encounter.period {
            if let Some(ref start) = period.start {
                let hl7_datetime = start.replace(['-', ':', 'T'], "");
                let _ = pv1.set_field_value(44, &hl7_datetime);
            }

            // PV1-45: Discharge Date/Time
            if let Some(ref end) = period.end {
                let hl7_datetime = end.replace(['-', ':', 'T'], "");
                let _ = pv1.set_field_value(45, &hl7_datetime);
            }
        }

        Ok(pv1)
    }

    /// Convert FHIR encounter class to HL7 v2 patient class
    fn convert_class_to_hl7(class: &Coding) -> String {
        if let Some(ref code) = class.code {
            match code.as_str() {
                "IMP" | "ACUTE" | "inpatient" => "I", // Inpatient
                "AMB" | "ambulatory" | "outpatient" => "O", // Outpatient
                "EMER" | "emergency" => "E",         // Emergency
                "OBSENC" | "observation" => "B",     // Observation
                "PRENC" | "preadmit" => "P",         // Preadmit
                "SS" | "short-stay" => "R",          // Recurring patient
                _ => "O",                            // Default to outpatient
            }
            .to_string()
        } else {
            "O".to_string()
        }
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
        assert_eq!(EncounterReverseConverter::convert_class_to_hl7(&coding), "I");

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
    fn test_visit_number() {
        let encounter = create_test_encounter();
        let pv1 = EncounterReverseConverter::convert(&encounter).unwrap();

        assert_eq!(pv1.get_field_value(19), Some("V12345"));
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
