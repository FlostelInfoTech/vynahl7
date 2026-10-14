//! Medication converter - RXA segment to FHIR MedicationAdministration resource

use super::coding_systems::{convert_coding_system, convert_coding_system_opt};
use super::patient::{
    hl7_fraction_suffix, split_hl7_offset, valid_hl7_date, valid_hl7_hour_min, valid_hl7_time,
    valid_hl7_year_month,
};
use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::medication::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming RXA segments to FHIR MedicationAdministration resources
pub struct MedicationConverter;

impl MedicationConverter {
    /// Convert all RXA segments in message to MedicationAdministration resources
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<MedicationAdministration>> {
        let rxa_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "RXA")
            .collect();

        if rxa_segments.is_empty() {
            return Err(ConversionError::MissingSegment("RXA".to_string()));
        }

        let mut administrations = Vec::new();
        for (rxa_count, _) in rxa_segments.iter().enumerate() {
            let admin = Self::convert_single(message, rxa_count)?;
            administrations.push(admin);
        }

        Ok(administrations)
    }

    /// Convert a specific RXA segment to a MedicationAdministration resource
    ///
    /// # Arguments
    /// * `message` - HL7 message containing RXA segment
    /// * `rxa_index` - 0-based index (0 = first RXA, 1 = second RXA, etc.)
    pub fn convert_single(
        message: &Message,
        rxa_index: usize,
    ) -> ConversionResult<MedicationAdministration> {
        let terser = Terser::new(message);

        // RXA-20: Completion Status -> MedicationAdministration.status
        let status_path = if rxa_index == 0 {
            "RXA-20".to_string()
        } else {
            format!("RXA({})-20", rxa_index + 1)
        };
        let status = if let Ok(Some(status_code)) = terser.get(&status_path) {
            Self::convert_status(status_code)
        } else {
            "completed".to_string()
        };

        let mut admin = MedicationAdministration::new(status);

        // RXA-5: Administered Code -> MedicationAdministration.medicationCodeableConcept.
        // A text-only CWE keeps its text like the allergy converter.
        let code_path = if rxa_index == 0 {
            "RXA-5".to_string()
        } else {
            format!("RXA({})-5", rxa_index + 1)
        };
        if let Ok(Some(code)) = terser.get(&code_path)
            && !code.is_empty()
        {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code.to_string()),
                display: None,
            };

            // Component 2: Text (1-based indexing)
            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
                && !text.is_empty()
            {
                coding.display = Some(text.to_string());
            }

            // Component 3: Coding system (1-based indexing)
            if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
                && !system.is_empty()
            {
                coding.system = Some(convert_coding_system(system));
            }

            admin.medication_codeable_concept = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        } else if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
            && !text.is_empty()
        {
            admin.medication_codeable_concept = Some(CodeableConcept {
                coding: None,
                text: Some(text.to_string()),
            });
        }

        // RXA-3: Date/Time Start of Administration
        let datetime_path = if rxa_index == 0 {
            "RXA-3".to_string()
        } else {
            format!("RXA({})-3", rxa_index + 1)
        };
        if let Ok(Some(datetime)) = terser.get(&datetime_path)
            && !datetime.is_empty()
        {
            admin.effective_date_time = Some(Self::convert_datetime(datetime)?);
        }

        // RXA-6: Administered Amount
        let amount_path = if rxa_index == 0 {
            "RXA-6".to_string()
        } else {
            format!("RXA({})-6", rxa_index + 1)
        };
        if let Ok(Some(amount)) = terser.get(&amount_path)
            && !amount.is_empty()
        {
            // A present-but-unparseable amount is an error, not a
            // silent omission of the dosage.
            let value = amount.parse::<f64>().map_err(|_| {
                // Redacted: never embed the source value (may be PHI).
                ConversionError::InvalidDataType(
                    "RXA-6".to_string(),
                    "numeric".to_string(),
                    "non-numeric value".to_string(),
                )
            })?;
            let mut dose = Quantity {
                value: Some(value),
                unit: None,
                system: Some("http://unitsofmeasure.org".to_string()),
                code: None,
            };

            // RXA-7: Administered Units
            let units_path = if rxa_index == 0 {
                "RXA-7".to_string()
            } else {
                format!("RXA({})-7", rxa_index + 1)
            };
            if let Ok(Some(units)) = terser.get(&units_path)
                && !units.is_empty()
            {
                dose.unit = Some(units.to_string());
                dose.code = Some(units.to_string());
            }

            admin.dosage = Some(MedicationAdministrationDosage {
                route: None,
                dose: Some(dose),
            });
        }

        // RXR-1: Route -> dosage.route, paired with this RXA by
        // order like ImmunizationConverter pairs RXR-2.
        if let Some(route_path) = Self::rxr_route_path(message, rxa_index)
            && let Ok(Some(route_code)) = terser.get(&route_path)
            && !route_code.is_empty()
        {
            // RXR-1-3 selects the system when known; otherwise the
            // v2-0162 bound-table default stands.
            let route_system = terser
                .get(&format!("{route_path}-3"))
                .ok()
                .flatten()
                .filter(|s| !s.is_empty())
                .and_then(convert_coding_system_opt)
                .unwrap_or_else(|| "http://terminology.hl7.org/CodeSystem/v2-0162".to_string());
            let mut coding = Coding {
                system: Some(route_system),
                code: Some(route_code.to_string()),
                display: None,
                version: None,
            };
            if let Ok(Some(route_text)) = terser.get(&format!("{route_path}-2"))
                && !route_text.is_empty()
            {
                coding.display = Some(route_text.to_string());
            }
            let route = CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            };
            match admin.dosage.as_mut() {
                Some(dosage) => dosage.route = Some(route),
                None => {
                    admin.dosage = Some(MedicationAdministrationDosage {
                        route: Some(route),
                        dose: None,
                    });
                }
            }
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty()
        {
            admin.subject = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        Ok(admin)
    }

    /// Terser path of the RXR-1 paired with the Nth RXA, if any.
    ///
    /// Pairing is by segment order: the first RXR at/after the RXA,
    /// up to the next RXA. When the RXA index is absent, pairing is
    /// impossible and the first RXR is used instead; with no RXR at
    /// all there is no route.
    fn rxr_route_path(message: &Message, rxa_index: usize) -> Option<String> {
        let mut rxr_ordinals: Vec<(usize, usize)> = Vec::new();
        let mut rxa_positions: Vec<usize> = Vec::new();
        for (pos, segment) in message.segments.iter().enumerate() {
            if segment.id == "RXR" {
                rxr_ordinals.push((pos, rxr_ordinals.len() + 1));
            } else if segment.id == "RXA" {
                rxa_positions.push(pos);
            }
        }
        if rxr_ordinals.is_empty() {
            return None;
        }
        let path_for = |ordinal: usize| {
            if ordinal == 1 {
                "RXR-1".to_string()
            } else {
                format!("RXR({ordinal})-1")
            }
        };
        let Some(rxa_pos) = rxa_positions.get(rxa_index).copied() else {
            return Some(path_for(1));
        };
        let end = rxa_positions
            .get(rxa_index + 1)
            .copied()
            .unwrap_or(usize::MAX);
        rxr_ordinals
            .iter()
            .find(|(pos, _)| *pos >= rxa_pos && *pos < end)
            .map(|(_, ordinal)| path_for(*ordinal))
    }

    /// Convert RXA-5 (Administered Code) to a standalone FHIR
    /// Medication resource.
    pub fn convert_medication(message: &Message, rxa_index: usize) -> ConversionResult<Medication> {
        let terser = Terser::new(message);

        let code_path = if rxa_index == 0 {
            "RXA-5".to_string()
        } else {
            format!("RXA({})-5", rxa_index + 1)
        };
        // A text-only CWE keeps its text like convert_single;
        // only a fully-empty field errors.
        let Some(code) = terser
            .get(&code_path)
            .ok()
            .flatten()
            .filter(|code| !code.is_empty())
        else {
            if let Ok(Some(text)) = terser.get(&format!("{code_path}-2"))
                && !text.is_empty()
            {
                let mut medication = Medication::new();
                medication.code = Some(CodeableConcept {
                    coding: None,
                    text: Some(text.to_string()),
                });
                return Ok(medication);
            }
            return Err(ConversionError::MissingField(
                code_path.clone(),
                "RXA".to_string(),
            ));
        };

        let mut coding = Coding {
            system: None,
            version: None,
            code: Some(code.to_string()),
            display: None,
        };

        // Component 2: Text (1-based indexing)
        if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
            && !text.is_empty()
        {
            coding.display = Some(text.to_string());
        }

        // Component 3: Coding system (1-based indexing)
        if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
            && !system.is_empty()
        {
            coding.system = Some(convert_coding_system(system));
        }

        let mut medication = Medication::new();
        medication.code = Some(CodeableConcept {
            coding: Some(vec![coding]),
            text: None,
        });
        Ok(medication)
    }

    fn convert_status(code: &str) -> String {
        match code {
            "CP" => "completed".to_string(),
            "PA" => "stopped".to_string(),
            "NA" => "not-done".to_string(),
            "RE" => "not-done".to_string(),
            _ => "completed".to_string(),
        }
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
            "RXA".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_medication_administration() {
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let administrations = MedicationConverter::convert_all(&message).unwrap();

        assert_eq!(administrations.len(), 1);
        assert_eq!(administrations[0].resource_type, "MedicationAdministration");
        assert_eq!(administrations[0].status, "completed");
    }

    #[test]
    fn test_convert_medication_second_repetition_reads_second_segment() {
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP\r\
                   RXA|0|1|20240315110000||104375^TYLENOL^NDC|200|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let administrations = MedicationConverter::convert_all(&message).unwrap();

        assert_eq!(administrations.len(), 2);
        let code = |a: &MedicationAdministration| {
            a.medication_codeable_concept
                .as_ref()
                .unwrap()
                .coding
                .as_ref()
                .unwrap()[0]
                .code
                .clone()
        };
        assert_eq!(code(&administrations[0]).as_deref(), Some("1191"));
        assert_eq!(code(&administrations[1]).as_deref(), Some("104375"));
    }

    #[test]
    fn test_unparseable_amount_returns_error() {
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|lots|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let result = MedicationConverter::convert_all(&message);
        assert!(matches!(
            result,
            Err(ConversionError::InvalidDataType(_, _, _))
        ));
    }

    #[test]
    fn test_non_ascii_datetime_returns_error_without_panic() {
        assert!(MedicationConverter::convert_datetime("123\u{e9}56789012").is_err());
        assert!(MedicationConverter::convert_datetime("short").is_err());
        assert_eq!(
            MedicationConverter::convert_datetime("20240315").unwrap(),
            "2024-03-15"
        );
    }

    #[test]
    fn test_convert_medication_builds_standalone_resource() {
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let medication = MedicationConverter::convert_medication(&message, 0).unwrap();

        assert_eq!(medication.resource_type, "Medication");
        let coding = &medication.code.unwrap().coding.unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("1191"));
        assert_eq!(coding.display.as_deref(), Some("ASPIRIN"));
    }

    #[test]
    fn test_refused_status_maps_to_not_done() {
        // RXA-20 "RE" (refused) must match ImmunizationConverter: not-done.
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG|||||||||||||RE";

        let message = parse_message(hl7).unwrap();
        let administrations = MedicationConverter::convert_all(&message).unwrap();

        assert_eq!(administrations.len(), 1);
        assert_eq!(administrations[0].status, "not-done");
    }

    #[test]
    fn test_impossible_calendar_datetime_rejected() {
        assert!(MedicationConverter::convert_datetime("20240230100000").is_err());
        assert!(MedicationConverter::convert_datetime("20240315250000").is_err());

        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240230100000||1191^ASPIRIN^NDC|100|MG|||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        assert!(MedicationConverter::convert_all(&message).is_err());
    }

    #[test]
    fn test_medication_display_reads_component_2() {
        // RXA-5 is CWE: component 1 = code, component 2 = text (1-based).
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let administrations = MedicationConverter::convert_all(&message).unwrap();
        let coding = &administrations[0]
            .medication_codeable_concept
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap()[0];

        assert_eq!(coding.code.as_deref(), Some("1191"));
        assert_eq!(coding.display.as_deref(), Some("ASPIRIN"));
    }

    #[test]
    fn test_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            MedicationConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            MedicationConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );
    }

    #[test]
    fn test_empty_pid3_and_rxa7_omitted() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||RAS^O17|1|P|2.5\r\
                   PID|1||||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|||||||||||||CP";
        let message = parse_message(hl7).unwrap();
        let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
        assert!(admin.subject.is_none());
        let dose = admin.dosage.as_ref().unwrap().dose.as_ref().unwrap();
        assert_eq!(dose.value, Some(100.0));
        assert!(dose.unit.is_none());
    }

    #[test]
    fn test_dose_error_redacts_source_value() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||RAS^O17|1|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|SECRET-DOSE|MG";
        let message = parse_message(hl7).unwrap();
        let err = MedicationConverter::convert_all(&message).unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("SECRET-DOSE"), "error leaks source: {msg}");
    }

    #[test]
    fn test_rxa5_system_mapped_from_component_3() {
        // RXA-5-3 selects the system in both converters.
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
        let coding = &admin
            .medication_codeable_concept
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("1191"));
        assert_eq!(
            coding.system.as_deref(),
            Some("http://hl7.org/fhir/sid/ndc")
        );

        let medication = MedicationConverter::convert_medication(&message, 0).unwrap();
        let coding = &medication.code.unwrap().coding.unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("1191"));
        assert_eq!(
            coding.system.as_deref(),
            Some("http://hl7.org/fhir/sid/ndc")
        );
    }

    #[test]
    fn test_empty_display_omitted_in_convert_single() {
        // RXA-5 with empty text leaves the display unset.
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^^NDC|100|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
        let coding = &admin
            .medication_codeable_concept
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("1191"));
        assert!(coding.display.is_none());
    }

    #[test]
    fn test_empty_rxa5_omits_medication_concept() {
        // RXA-5 present-but-empty (RXA-6 forces presence) leaves the
        // whole concept unset instead of coding code "".
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000|||100|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
        assert!(admin.medication_codeable_concept.is_none());
        // Valued sibling still converts.
        let dose = admin.dosage.as_ref().unwrap().dose.as_ref().unwrap();
        assert_eq!(dose.value, Some(100.0));
    }

    #[test]
    fn test_datetime_preserves_fractional_seconds() {
        assert_eq!(
            MedicationConverter::convert_datetime("20240315100000.123").unwrap(),
            "2024-03-15T10:00:00.123"
        );
    }

    #[test]
    fn test_text_only_rxa5_concept_preserved() {
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||^Aspirin free text|100|MG||||||||||||CP";
        let message = parse_message(hl7).unwrap();
        let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
        let concept = admin.medication_codeable_concept.as_ref().unwrap();
        assert_eq!(concept.coding, None);
        assert_eq!(concept.text.as_deref(), Some("Aspirin free text"));
    }

    #[test]
    fn test_empty_rxa5_convert_medication_returns_missing_field() {
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000|||100|MG||||||||||||CP";
        let message = parse_message(hl7).unwrap();
        let result = MedicationConverter::convert_medication(&message, 0);
        assert!(matches!(result, Err(ConversionError::MissingField(_, _))));
    }

    #[test]
    fn test_datetime_accepts_year_month_precision() {
        assert_eq!(
            MedicationConverter::convert_datetime("202401").unwrap(),
            "2024-01"
        );
        assert_eq!(
            MedicationConverter::convert_datetime("1980").unwrap(),
            "1980"
        );
    }

    #[test]
    fn test_dosage_route_from_paired_rxr1() {
        // RXR-1 route pairs with RXA by order.
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP\r\
                   RXR|IM^Intramuscular";
        let message = parse_message(hl7).unwrap();
        let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
        let route = admin.dosage.as_ref().unwrap().route.as_ref().unwrap();
        let coding = route.coding.as_ref().unwrap();
        assert_eq!(coding[0].code.as_deref(), Some("IM"));
        assert_eq!(coding[0].display.as_deref(), Some("Intramuscular"));
    }

    #[test]
    fn test_not_administered_status_maps_to_not_done() {
        // RXA-20 "NA" (not administered) matches ImmunizationConverter:
        // not-done, not entered-in-error (a nullified record).
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG|||||||||||||NA";

        let message = parse_message(hl7).unwrap();
        let administrations = MedicationConverter::convert_all(&message).unwrap();

        assert_eq!(administrations.len(), 1);
        assert_eq!(administrations[0].status, "not-done");
    }

    #[test]
    fn test_dose_carries_ucum_system_and_code() {
        // RXA-6/RXA-7 carry the UCUM system and code, like the
        // immunization dose quantity.
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP";

        let message = parse_message(hl7).unwrap();
        let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
        let dose = admin.dosage.as_ref().unwrap().dose.as_ref().unwrap();
        assert_eq!(dose.value, Some(100.0));
        assert_eq!(dose.unit.as_deref(), Some("MG"));
        assert_eq!(dose.system.as_deref(), Some("http://unitsofmeasure.org"));
        assert_eq!(dose.code.as_deref(), Some("MG"));
    }

    #[test]
    fn test_route_honors_explicit_coding_system() {
        // An explicitly different RXR-1-3 wins over the v2-0162
        // bound-table default; absent or unknown keeps the default.
        let message_for = |rxr1: &str| {
            let hl7 = format!(
                "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                 PID|1||12345|||DOE^JOHN||19800101|M\r\
                 RXA|0|1|20240315100000||1191^ASPIRIN^NDC|100|MG||||||||||||CP\r\
                 RXR|{rxr1}"
            );
            parse_message(&hl7).unwrap()
        };
        let system_for = |rxr1: &str| {
            let message = message_for(rxr1);
            let admin = &MedicationConverter::convert_all(&message).unwrap()[0];
            admin
                .dosage
                .as_ref()
                .unwrap()
                .route
                .as_ref()
                .unwrap()
                .coding
                .as_ref()
                .unwrap()[0]
                .system
                .clone()
        };
        assert_eq!(
            system_for("IM^Intramuscular^SCT").as_deref(),
            Some("http://snomed.info/sct")
        );
        assert_eq!(
            system_for("IM^Intramuscular").as_deref(),
            Some("http://terminology.hl7.org/CodeSystem/v2-0162")
        );
        assert_eq!(
            system_for("IM^Intramuscular^XYZ").as_deref(),
            Some("http://terminology.hl7.org/CodeSystem/v2-0162")
        );
    }

    #[test]
    fn test_convert_medication_preserves_text_only_rxa5() {
        // Text-only RXA-5 keeps its text like convert_single
        // (Medication.code is optional); only a fully-empty
        // field errors (see above).
        let hl7 = "MSH|^~\\&|Pharmacy|Hospital|EMR|Hospital|20240315||RAS^O17|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   RXA|0|1|20240315100000||^Aspirin free text|100|MG||||||||||||CP";
        let message = parse_message(hl7).unwrap();
        let medication = MedicationConverter::convert_medication(&message, 0).unwrap();
        let code = medication.code.as_ref().unwrap();
        assert_eq!(code.coding, None);
        assert_eq!(code.text.as_deref(), Some("Aspirin free text"));
    }
}
