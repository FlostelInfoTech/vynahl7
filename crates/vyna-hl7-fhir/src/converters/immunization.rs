//! Immunization converter - RXA segment to FHIR Immunization resource

use super::coding_systems::convert_coding_system;
use super::patient::{
    split_hl7_offset, valid_hl7_date, valid_hl7_hour_min, valid_hl7_time, valid_hl7_year_month,
};
use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::immunization::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming RXA segments to FHIR Immunization resources
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
        let status = terser
            .get(&path("20"))
            .ok()
            .flatten()
            .map(Self::convert_status)
            .unwrap_or_else(|| "completed".to_string());

        // RXA-5: Administered Code -> vaccineCode
        let vaccine_code = Self::extract_vaccine_code(&terser, &path("5"))?;

        // PID-3: Patient ID -> patient reference
        let patient = Self::extract_patient_reference(&terser)?;

        // RXA-3: Date/Time Start of Administration -> occurrenceDateTime
        let occurrence_date_time = terser
            .get(&path("3"))
            .ok()
            .flatten()
            .map(Self::convert_datetime)
            .transpose()?
            .ok_or_else(|| ConversionError::MissingField("RXA-3".to_string(), "RXA".to_string()))?;

        let mut immunization =
            Immunization::new(status, vaccine_code, patient, occurrence_date_time);

        // PV1-19: Visit Number -> encounter reference
        if let Ok(Some(visit_num)) = terser.get("PV1-19")
            && !visit_num.is_empty()
        {
            immunization.encounter = Some(Reference {
                reference: Some(format!("Encounter/{}", visit_num)),
                type_: Some("Encounter".to_string()),
                identifier: None,
                display: None,
            });
        }

        // RXA-22: System Entry Date/Time -> recorded
        // Lenient by design (see crate docs "Invalid datetime handling").
        if let Ok(Some(recorded_dt)) = terser.get(&path("22"))
            && let Ok(recorded) = Self::convert_datetime(recorded_dt)
        {
            immunization.recorded = Some(recorded);
        }

        // RXA-9: Information Authority -> primarySource / reportOrigin.
        // Empty is absent: it takes the default, not a false flag with an
        // empty report origin.
        if let Ok(Some(auth_code)) = terser.get(&path("9"))
            && !auth_code.is_empty()
        {
            match auth_code {
                "00" => immunization.primary_source = Some(true),
                _ => {
                    immunization.primary_source = Some(false);
                    immunization.report_origin = Some(CodeableConcept {
                        coding: Some(vec![Coding {
                            system: Some(
                                "http://terminology.hl7.org/CodeSystem/v2-0356".to_string(),
                            ),
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
        if let Ok(Some(location_id)) = terser.get(&path("27"))
            && !location_id.is_empty()
        {
            immunization.location = Some(Reference {
                reference: Some(format!("Location/{}", location_id)),
                type_: Some("Location".to_string()),
                identifier: None,
                display: None,
            });
        }

        // RXA-17: Substance Manufacturer Name -> manufacturer
        if let Ok(Some(mfr_id)) = terser.get(&path("17"))
            && !mfr_id.is_empty()
        {
            immunization.manufacturer = Some(Reference {
                reference: Some(format!("Organization/{}", mfr_id)),
                type_: Some("Organization".to_string()),
                identifier: None,
                display: None,
            });
        }

        // RXA-15: Substance Lot Number -> lotNumber. Empty is
        // absent: it must not become an empty lot number.
        if let Ok(Some(lot)) = terser.get(&path("15"))
            && !lot.is_empty()
        {
            immunization.lot_number = Some(lot.to_string());
        }

        // RXA-16: Substance Expiration Date -> expirationDate
        if let Ok(Some(exp_date)) = terser.get(&path("16"))
            && let Ok(date) = Self::convert_date(exp_date)
        {
            immunization.expiration_date = Some(date);
        }

        // RXA-18: Substance Refusal Reason (when status is not-done)
        if immunization.status == "not-done"
            && let Ok(Some(refusal_reason)) = terser.get(&path("18"))
            && !refusal_reason.is_empty()
        {
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
            && !amount_str.is_empty()
        {
            // A present-but-unparseable amount is an error, not a
            // silent omission of the dose quantity.
            let amount = amount_str.parse::<f64>().map_err(|_| {
                ConversionError::InvalidDataType(
                    "RXA-6".to_string(),
                    "numeric".to_string(),
                    // Redacted: never embed the source value (may be PHI).
                    "unparseable numeric value".to_string(),
                )
            })?;
            let mut dose = Quantity {
                value: Some(amount),
                unit: None,
                system: Some("http://unitsofmeasure.org".to_string()),
                code: None,
            };

            if let Ok(Some(units)) = terser.get(&path("7"))
                && !units.is_empty()
            {
                dose.unit = Some(units.to_string());
                dose.code = Some(units.to_string());
            }

            immunization.dose_quantity = Some(dose);
        }

        // RXA-10: Administering Provider -> performer
        if let Ok(Some(performer_id)) = terser.get(&path("10"))
            && !performer_id.is_empty()
        {
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
        if let Ok(Some(site_code)) = terser.get(&path("11"))
            && !site_code.is_empty()
        {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0163".to_string()),
                code: Some(site_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Site text
            if let Ok(Some(site_text)) = terser.get(&format!("{}-2", path("11")))
                && !site_text.is_empty()
            {
                coding.display = Some(site_text.to_string());
            }

            immunization.site = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        }

        // RXA-21 is the HL7 Action Code, not Administration Notes
        // (RXA-9, already mapped above); it has no Immunization mapping.
        Ok(immunization)
    }

    /// Convert HL7 v2 RXA-20 completion status to FHIR Immunization status
    fn convert_status(code: &str) -> String {
        match code {
            "CP" => "completed",
            "PA" => "completed", // Partially administered - still completed in FHIR
            "NA" => "not-done",
            "RE" => "not-done", // Refused
            _ => "completed",   // Default to completed
        }
        .to_string()
    }

    /// Convert HL7 datetime (YYYYMMDDHHMMSS) to FHIR ISO 8601 format
    ///
    /// `YYYYMMDDHHMM` keeps its minutes and a trailing HL7 zone is
    /// preserved as `±HH:MM`.
    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        let (stem, offset) = split_hl7_offset(datetime);
        let zone = offset.as_deref().unwrap_or("");
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
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
        } else if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            // Date only; still requires a real calendar date.
            if valid_hl7_date(&stem[0..8]) {
                Ok(format!("{}-{}-{}", &stem[0..4], &stem[4..6], &stem[6..8]))
            } else {
                Err(Self::invalid_datetime(datetime))
            }
        } else {
            Err(Self::invalid_datetime(datetime))
        }
    }

    /// Redacted bad-datetime error: field location + reason only, never the value.
    fn invalid_datetime(_datetime: &str) -> ConversionError {
        ConversionError::InvalidFormat(
            "RXA-3".to_string(),
            "RXA".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
    }

    /// Convert HL7 date (YYYYMMDD) to FHIR date format
    ///
    /// `YYYYMM` month precision is preserved as `YYYY-MM`.
    fn convert_date(date: &str) -> ConversionResult<String> {
        let (stem, _) = split_hl7_offset(date);
        let bytes = stem.as_bytes();
        if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) {
                Ok(format!("{}-{}-{}", &stem[0..4], &stem[4..6], &stem[6..8]))
            } else {
                Err(Self::invalid_date())
            }
        } else if bytes.len() >= 6 && bytes[..6].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_year_month(&stem[0..6]) {
                Ok(format!("{}-{}", &stem[0..4], &stem[4..6]))
            } else {
                Err(Self::invalid_date())
            }
        } else {
            Err(Self::invalid_date())
        }
    }

    /// Redacted bad-date error: field location + reason only, never the value.
    fn invalid_date() -> ConversionError {
        ConversionError::InvalidFormat(
            "RXA-16".to_string(),
            "RXA".to_string(),
            // Redacted: never embed the source value (may be PHI).
            "expected YYYYMMDD calendar date or YYYYMM month".to_string(),
        )
    }

    /// Extract vaccine code from RXA-5 field
    fn extract_vaccine_code(terser: &Terser, base_path: &str) -> ConversionResult<CodeableConcept> {
        let code = terser.get(base_path).ok().flatten().ok_or_else(|| {
            ConversionError::MissingField(base_path.to_string(), "RXA".to_string())
        })?;

        // RXA-5-3 selects the system; only an absent one defaults to CVX.
        let system = terser
            .get(&format!("{}-3", base_path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .map(convert_coding_system)
            .unwrap_or_else(|| "http://hl7.org/fhir/sid/cvx".to_string());

        let mut coding = Coding {
            system: Some(system),
            code: Some(code.to_string()),
            display: None,
            version: None,
        };

        // Component 1: Vaccine text/display (1-based component indexing)
        if let Ok(Some(display)) = terser.get(&format!("{}-2", base_path))
            && !display.is_empty()
        {
            coding.display = Some(display.to_string());
        }

        Ok(CodeableConcept {
            coding: Some(vec![coding]),
            text: None,
        })
    }

    /// Extract patient reference from PID segment
    fn extract_patient_reference(terser: &Terser) -> ConversionResult<Reference> {
        // Empty is missing: it must not become a `Patient/` reference.
        let patient_id = terser
            .get("PID-3")
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
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
    fn test_unparseable_amount_returns_error() {
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|lots|mL||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let result = ImmunizationConverter::convert_all(&message);
        assert!(matches!(
            result,
            Err(ConversionError::InvalidDataType(_, _, _))
        ));
    }

    #[test]
    fn test_non_ascii_datetime_returns_error_without_panic() {
        assert!(ImmunizationConverter::convert_datetime("123\u{e9}56789012").is_err());
        assert!(ImmunizationConverter::convert_date("12\u{e9}\u{e9}5678").is_err());
    }

    #[test]
    fn test_impossible_calendar_datetime_rejected() {
        assert!(ImmunizationConverter::convert_datetime("20240230100000").is_err());
        assert!(ImmunizationConverter::convert_date("20240230").is_err());

        // RXA-3 is a primary timestamp: an impossible date must fail.
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240230100000||08^HepB adult^CVX|1.0|mL||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        assert!(ImmunizationConverter::convert_all(&message).is_err());
    }

    #[test]
    fn test_empty_rxa9_treated_as_absent() {
        // RXA-9 present-but-empty takes the default instead of emitting
        // primarySource=false with an empty report origin.
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|1.0|mL||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let immunizations = ImmunizationConverter::convert_all(&message).unwrap();
        assert_eq!(immunizations[0].primary_source, Some(true));
        assert_eq!(immunizations[0].report_origin, None);
    }

    #[test]
    fn test_datetime_preserves_minutes_zone_and_month() {
        assert_eq!(
            ImmunizationConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            ImmunizationConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );
        assert_eq!(
            ImmunizationConverter::convert_date("202512").unwrap(),
            "2025-12"
        );

        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000+0500||08^HepB adult^CVX|1.0|mL||||||||||||CP";
        let message = parse_message(hl7).unwrap();
        let immunizations = ImmunizationConverter::convert_all(&message).unwrap();
        assert_eq!(
            immunizations[0].occurrence_date_time,
            "2024-03-15T10:00:00+05:00"
        );
    }

    #[test]
    fn test_vaccine_code_system_mapped_from_rxa5_3() {
        let message_for = |rxa5: &str| {
            let hl7 = format!(
                "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                 PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                 RXA|0|1|20240315100000||{rxa5}|1.0|mL||||||||||||CP"
            );
            parse_message(&hl7).unwrap()
        };

        // RXA-5-3 selects the system: NDC must not be labeled CVX.
        let message = message_for("1191^ASPIRIN^NDC");
        let imm = &ImmunizationConverter::convert_all(&message).unwrap()[0];
        let coding = imm.vaccine_code.coding.as_ref().unwrap();
        assert_eq!(coding[0].code.as_deref(), Some("1191"));
        assert_eq!(coding[0].display.as_deref(), Some("ASPIRIN"));
        assert_eq!(
            coding[0].system.as_deref(),
            Some("http://hl7.org/fhir/sid/ndc")
        );

        // Absent RXA-5-3 still defaults to CVX.
        let message = message_for("08^HepB adult");
        let imm = &ImmunizationConverter::convert_all(&message).unwrap()[0];
        let coding = imm.vaccine_code.coding.as_ref().unwrap();
        assert_eq!(
            coding[0].system.as_deref(),
            Some("http://hl7.org/fhir/sid/cvx")
        );
    }

    #[test]
    fn test_empty_pid3_returns_missing_segment() {
        // PID-3 present but its first component empty: no patient id.
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|1.0|mL||||||||||||CP";
        let message = parse_message(hl7).unwrap();
        let result = ImmunizationConverter::convert_all(&message);
        assert!(matches!(result, Err(ConversionError::MissingSegment(_))));
    }

    #[test]
    fn test_empty_rxa15_leaves_lot_number_unset() {
        // RXA-15 present-but-empty (RXA-20 forces its presence).
        let mut fields = vec![String::new(); 20];
        fields[0] = "0".to_string();
        fields[1] = "1".to_string();
        fields[2] = "20240315100000".to_string();
        fields[4] = "08^HepB adult^CVX".to_string();
        fields[5] = "1.0".to_string();
        fields[6] = "mL".to_string();
        fields[19] = "CP".to_string();
        let hl7 = format!(
            "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rRXA|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let immunizations = ImmunizationConverter::convert_all(&message).unwrap();
        assert_eq!(immunizations[0].lot_number, None);
    }

    #[test]
    fn test_unparseable_amount_error_redacts_source_value() {
        let hl7 = "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||08^HepB adult^CVX|SECRET-DOSE|mL||||||||||||CP";
        let message = parse_message(hl7).unwrap();
        let err = ImmunizationConverter::convert_all(&message).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("RXA-6"), "keeps field location: {msg}");
        assert!(!msg.contains("SECRET-DOSE"), "error leaks source: {msg}");
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

    #[test]
    fn test_empty_rxa_fields_omit_references_and_codes() {
        // Present-but-empty RXA-7/10/11/17/27, RXA-5-2, and PV1-19
        // must not emit empty references, codes, or quantities.
        let mut fields = vec![String::new(); 27];
        fields[0] = "0".to_string();
        fields[1] = "1".to_string();
        fields[2] = "20240315100000".to_string();
        fields[4] = "08^^CVX".to_string();
        fields[5] = "1.0".to_string();
        fields[19] = "CP".to_string();
        let mut pv1 = vec![String::new(); 19];
        pv1[0] = "1".to_string();
        let hl7 = format!(
            "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rPV1|{}\rRXA|{}",
            pv1.join("|"),
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let imm = &ImmunizationConverter::convert_all(&message).unwrap()[0];
        assert!(imm.performer.is_none());
        assert!(imm.site.is_none());
        assert!(imm.manufacturer.is_none());
        assert!(imm.encounter.is_none());
        assert!(imm.location.is_none());
        let coding = imm.vaccine_code.coding.as_ref().unwrap();
        assert!(coding[0].display.is_none());
        let dose = imm.dose_quantity.as_ref().unwrap();
        assert_eq!(dose.value, Some(1.0));
        assert!(dose.unit.is_none());
        assert!(dose.code.is_none());
    }

    #[test]
    fn test_empty_rxa11_text_omits_site_display() {
        // RXA-11 with a code but empty text keeps the code only.
        let mut fields = vec![String::new(); 20];
        fields[0] = "0".to_string();
        fields[1] = "1".to_string();
        fields[2] = "20240315100000".to_string();
        fields[4] = "08^HepB adult^CVX".to_string();
        fields[5] = "1.0".to_string();
        fields[6] = "mL".to_string();
        fields[10] = "LA^".to_string();
        fields[19] = "CP".to_string();
        let hl7 = format!(
            "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rRXA|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let imm = &ImmunizationConverter::convert_all(&message).unwrap()[0];
        let site = imm.site.as_ref().unwrap().coding.as_ref().unwrap();
        assert_eq!(site[0].code.as_deref(), Some("LA"));
        assert!(site[0].display.is_none());
    }

    #[test]
    fn test_empty_rxa18_omits_status_reason() {
        // A refused dose with empty RXA-18 has no status reason.
        let mut fields = vec![String::new(); 20];
        fields[0] = "0".to_string();
        fields[1] = "1".to_string();
        fields[2] = "20240315100000".to_string();
        fields[4] = "08^HepB adult^CVX".to_string();
        fields[19] = "RE".to_string();
        let hl7 = format!(
            "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rRXA|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let imm = &ImmunizationConverter::convert_all(&message).unwrap()[0];
        assert_eq!(imm.status, "not-done");
        assert!(imm.status_reason.is_none());
    }

    #[test]
    fn test_rxa21_action_code_not_mapped_to_note() {
        // RXA-21 is the HL7 Action Code, not Administration Notes
        // (RXA-9); it must not become note text.
        let mut fields = vec![String::new(); 21];
        fields[0] = "0".to_string();
        fields[1] = "1".to_string();
        fields[2] = "20240315100000".to_string();
        fields[4] = "08^HepB adult^CVX".to_string();
        fields[19] = "CP".to_string();
        fields[20] = "A".to_string();
        let hl7 = format!(
            "MSH|^~\\&|Clinic|Hospital|EMR|Hospital|20240315||VXU^V04|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rRXA|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let imm = &ImmunizationConverter::convert_all(&message).unwrap()[0];
        assert!(imm.note.is_none());
    }
}
