//! Medication converter - RXA segment to FHIR MedicationAdministration resource

use super::coding_systems::convert_coding_system;
use super::patient::{
    hl7_fraction_suffix, split_hl7_offset, valid_hl7_date, valid_hl7_hour_min,
    valid_hl7_time,
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

        // RXA-5: Administered Code -> MedicationAdministration.medicationCodeableConcept
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
                system: None,
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
            }

            admin.dosage = Some(MedicationAdministrationDosage {
                route: None,
                dose: Some(dose),
            });
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

    /// Convert RXA-5 (Administered Code) to a standalone FHIR
    /// Medication resource.
    pub fn convert_medication(message: &Message, rxa_index: usize) -> ConversionResult<Medication> {
        let terser = Terser::new(message);

        let code_path = if rxa_index == 0 {
            "RXA-5".to_string()
        } else {
            format!("RXA({})-5", rxa_index + 1)
        };
        let code =
            terser.get(&code_path).ok().flatten().ok_or_else(|| {
                ConversionError::MissingField(code_path.clone(), "RXA".to_string())
            })?;

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
            "NA" => "entered-in-error".to_string(),
            "RE" => "not-done".to_string(),
            _ => "completed".to_string(),
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
}
