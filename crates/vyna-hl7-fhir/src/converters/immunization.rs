//! Immunization converter - RXA segment to FHIR Immunization resource

use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::immunization::*;
use crate::resources::common::*;

pub struct ImmunizationConverter;

impl ImmunizationConverter {
    /// Convert all RXA segments in message to Immunization resources
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<Immunization>> {
        let rxa_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "RXA")
            .collect();

        if rxa_segments.is_empty() {
            return Err(ConversionError::MissingSegment("RXA".to_string()));
        }

        let mut immunizations = Vec::new();
        for (rxa_count, _) in rxa_segments.iter().enumerate() {
            let imm = Self::convert_single(message, rxa_count)?;
            immunizations.push(imm);
        }

        Ok(immunizations)
    }

    /// Convert a specific RXA segment to Immunization resource
    ///
    /// # Arguments
    /// * `message` - HL7 message containing RXA segment
    /// * `rxa_index` - 0-based index (0 = first RXA, 1 = second RXA, etc.)
    pub fn convert_single(message: &Message, rxa_index: usize) -> ConversionResult<Immunization> {
        let terser = Terser::new(message);

        // Build base path for RXA fields
        // Note: Terser uses 1-based segment indexing, so RXA(1) is the first RXA segment
        let path = |field: &str| {
            if rxa_index == 0 {
                format!("RXA-{}", field)
            } else {
                format!("RXA({})-{}", rxa_index + 1, field)
            }
        };

        // RXA-20: Completion Status -> status
        let status = terser.get(&path("20"))
            .ok()
            .flatten()
            .map(Self::convert_status)
            .unwrap_or_else(|| "completed".to_string());

        // RXA-5: Administered Code -> vaccineCode
        let vaccine_code = Self::extract_vaccine_code(&terser, &path("5"))?;

        // PID-3: Patient ID -> patient reference
        let patient = Self::extract_patient_reference(&terser)?;

        // RXA-3: Date/Time Start of Administration -> occurrenceDateTime
        let occurrence_date_time = terser.get(&path("3"))
            .ok()
            .flatten()
            .map(Self::convert_datetime)
            .transpose()?
            .ok_or_else(|| ConversionError::MissingField("RXA-3".to_string(), "RXA".to_string()))?;

        let mut immunization = Immunization::new(
            status,
            vaccine_code,
            patient,
            occurrence_date_time,
        );

        // PV1-19: Visit Number -> encounter reference
        if let Ok(Some(visit_num)) = terser.get("PV1-19") {
            immunization.encounter = Some(Reference {
                reference: Some(format!("Encounter/{}", visit_num)),
                type_: Some("Encounter".to_string()),
                identifier: None,
                display: None,
            });
        }

        // RXA-22: System Entry Date/Time -> recorded
        if let Ok(Some(recorded_dt)) = terser.get(&path("22"))
            && let Ok(recorded) = Self::convert_datetime(recorded_dt) {
                immunization.recorded = Some(recorded);
            }

        // RXA-9: Information Authority -> primarySource / reportOrigin
        if let Ok(Some(auth_code)) = terser.get(&path("9")) {
            match auth_code {
                "00" => immunization.primary_source = Some(true),
                _ => {
                    immunization.primary_source = Some(false);
                    immunization.report_origin = Some(CodeableConcept {
                        coding: Some(vec![Coding {
                            system: Some("http://terminology.hl7.org/CodeSystem/v2-0356".to_string()),
                            code: Some(auth_code.to_string()),
                            display: None,
                            version: None,
                        }]),
                        text: None,
                    });
                }
            }
        } else {
            immunization.primary_source = Some(true); // Default
        }

        // RXA-27: Administration Address -> location
        if let Ok(Some(location_id)) = terser.get(&path("27")) {
            immunization.location = Some(Reference {
                reference: Some(format!("Location/{}", location_id)),
                type_: Some("Location".to_string()),
                identifier: None,
                display: None,
            });
        }

        // RXA-17: Substance Manufacturer Name -> manufacturer
        if let Ok(Some(mfr_id)) = terser.get(&path("17")) {
            immunization.manufacturer = Some(Reference {
                reference: Some(format!("Organization/{}", mfr_id)),
                type_: Some("Organization".to_string()),
                identifier: None,
                display: None,
            });
        }

        // RXA-15: Substance Lot Number -> lotNumber
        if let Ok(Some(lot)) = terser.get(&path("15")) {
            immunization.lot_number = Some(lot.to_string());
        }

        // RXA-16: Substance Expiration Date -> expirationDate
        if let Ok(Some(exp_date)) = terser.get(&path("16"))
            && let Ok(date) = Self::convert_date(exp_date) {
                immunization.expiration_date = Some(date);
            }

        // RXA-20: Substance Refusal Reason (when status is not-done)
        if immunization.status == "not-done"
            && let Ok(Some(refusal_reason)) = terser.get(&path("18")) {
                immunization.status_reason = Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v2-0443".to_string()),
                        code: Some(refusal_reason.to_string()),
                        display: None,
                        version: None,
                    }]),
                    text: None,
                });
            }

        // RXA-7: Administered Units + RXA-6: Administered Amount -> doseQuantity
        if let Ok(Some(amount_str)) = terser.get(&path("6"))
            && let Ok(amount) = amount_str.parse::<f64>() {
                let mut dose = Quantity {
                    value: Some(amount),
                    unit: None,
                    system: Some("http://unitsofmeasure.org".to_string()),
                    code: None,
                };

                if let Ok(Some(units)) = terser.get(&path("7")) {
                    dose.unit = Some(units.to_string());
                    dose.code = Some(units.to_string());
                }

                immunization.dose_quantity = Some(dose);
            }

        // RXA-10: Administering Provider -> performer
        if let Ok(Some(performer_id)) = terser.get(&path("10")) {
            immunization.performer = Some(vec![ImmunizationPerformer {
                function: Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v2-0443".to_string()),
                        code: Some("AP".to_string()),
                        display: Some("Administering Provider".to_string()),
                        version: None,
                    }]),
                    text: None,
                }),
                actor: Reference {
                    reference: Some(format!("Practitioner/{}", performer_id)),
                    type_: Some("Practitioner".to_string()),
                    identifier: None,
                    display: None,
                },
            }]);
        }

        // RXA-11: Administered-at Location -> site
        if let Ok(Some(site_code)) = terser.get(&path("11")) {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0163".to_string()),
                code: Some(site_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Site text
            if let Ok(Some(site_text)) = terser.get(&format!("{}-1", path("11"))) {
                coding.display = Some(site_text.to_string());
            }

            immunization.site = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        }

        // RXA-21: Administration Notes -> note
        if let Ok(Some(note_text)) = terser.get(&path("21")) {
            immunization.note = Some(vec![Annotation {
                author_string: None,
                author_reference: None,
                time: None,
                text: note_text.to_string(),
            }]);
        }

        Ok(immunization)
    }

    /// Convert HL7 v2 RXA-20 completion status to FHIR Immunization status
    fn convert_status(code: &str) -> String {
        match code {
            "CP" => "completed",
            "PA" => "completed", // Partially administered - still completed in FHIR
            "NA" => "not-done",
            "RE" => "not-done",  // Refused
            _ => "completed",    // Default to completed
        }.to_string()
    }

    /// Convert HL7 datetime (YYYYMMDDHHMMSS) to FHIR ISO 8601 format
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
            // Date only
            Ok(format!(
                "{}-{}-{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8]
            ))
        } else {
            Err(ConversionError::InvalidFormat(
                "RXA-3".to_string(),
                "RXA".to_string(),
                format!("Invalid datetime format: {}", datetime)
            ))
        }
    }

    /// Convert HL7 date (YYYYMMDD) to FHIR date format
    fn convert_date(date: &str) -> ConversionResult<String> {
        if date.len() >= 8 {
            Ok(format!(
                "{}-{}-{}",
                &date[0..4],
                &date[4..6],
                &date[6..8]
            ))
        } else {
            Err(ConversionError::InvalidFormat(
                "RXA-16".to_string(),
                "RXA".to_string(),
                format!("Invalid date format: {}", date)
            ))
        }
    }

    /// Extract vaccine code from RXA-5 field
    fn extract_vaccine_code(terser: &Terser, base_path: &str) -> ConversionResult<CodeableConcept> {
        let code = terser.get(base_path)
            .ok()
            .flatten()
            .ok_or_else(|| ConversionError::MissingField(base_path.to_string(), "RXA".to_string()))?;

        let mut coding = Coding {
            system: Some("http://hl7.org/fhir/sid/cvx".to_string()), // CVX vaccine codes
            code: Some(code.to_string()),
            display: None,
            version: None,
        };

        // Component 1: Vaccine text/display (1-based component indexing)
        if let Ok(Some(display)) = terser.get(&format!("{}-1", base_path)) {
            coding.display = Some(display.to_string());
        }

        Ok(CodeableConcept {
            coding: Some(vec![coding]),
            text: None,
        })
    }

    /// Extract patient reference from PID segment
    fn extract_patient_reference(terser: &Terser) -> ConversionResult<Reference> {
        let patient_id = terser.get("PID-3")
            .ok()
            .flatten()
            .ok_or_else(|| ConversionError::MissingSegment("PID".to_string()))?;

        Ok(Reference {
            reference: Some(format!("Patient/{}", patient_id)),
            type_: Some("Patient".to_string()),
            identifier: None,
            display: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_immunization_basic() {
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|1.0|mL||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let immunizations = ImmunizationConverter::convert_all(&message).unwrap();

        assert_eq!(immunizations.len(), 1);
        let imm = &immunizations[0];
        assert_eq!(imm.resource_type, "Immunization");
        assert_eq!(imm.status, "completed");
        assert_eq!(imm.patient.reference, Some("Patient/12345".to_string()));
        assert_eq!(imm.occurrence_date_time, "2024-03-15T10:00:00");
    }

    #[test]
    fn test_convert_immunization_with_lot_and_expiration() {
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|1.0|mL^milliliters|||DOC123|||||LOT123456|20251231|MFR123|||CP";

        let message = parse_message(hl7).unwrap();
        let immunizations = ImmunizationConverter::convert_all(&message).unwrap();

        assert_eq!(immunizations.len(), 1);
        let imm = &immunizations[0];
        assert_eq!(imm.lot_number, Some("LOT123456".to_string()));
        assert_eq!(imm.expiration_date, Some("2025-12-31".to_string()));
        assert!(imm.performer.is_some());
    }

    #[test]
    fn test_convert_multiple_immunizations() {
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|1.0|mL|||DOC001|||||LOT001|||||CP\r\
                   RXA|0|1|20240315101500||20^DTaP^CVX|0.5|mL|||DOC002|||||LOT002|||||CP";

        let message = parse_message(hl7).unwrap();
        let immunizations = ImmunizationConverter::convert_all(&message).unwrap();

        assert_eq!(immunizations.len(), 2);
        assert_eq!(immunizations[0].lot_number, Some("LOT001".to_string()));
        assert_eq!(immunizations[1].lot_number, Some("LOT002".to_string()));
    }

    #[test]
    fn test_convert_refused_immunization() {
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|0||||||||||||||RE";

        let message = parse_message(hl7).unwrap();
        let immunizations = ImmunizationConverter::convert_all(&message).unwrap();

        assert_eq!(immunizations.len(), 1);
        assert_eq!(immunizations[0].status, "not-done");
    }

    #[test]
    fn test_missing_rxa_segment() {
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let result = ImmunizationConverter::convert_all(&message);

        assert!(result.is_err());
        match result {
            Err(ConversionError::MissingSegment(seg)) => assert_eq!(seg, "RXA"),
            _ => panic!("Expected MissingSegment error"),
        }
    }
}
