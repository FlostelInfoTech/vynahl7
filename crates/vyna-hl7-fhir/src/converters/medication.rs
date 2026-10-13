//! Medication converter - RXA segment to FHIR MedicationAdministration resource

use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::medication::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

pub struct MedicationConverter;

impl MedicationConverter {
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
        if let Ok(Some(code)) = terser.get(&code_path) {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code.to_string()),
                display: None,
            };

            // Component 2: Text (1-based indexing)
            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path)) {
                coding.display = Some(text.to_string());
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
                ConversionError::InvalidDataType(
                    "RXA-6".to_string(),
                    "numeric".to_string(),
                    amount.to_string(),
                )
            })?;
            let mut dosage = MedicationAdministrationDosage {
                route: None,
                dose: Some(Quantity {
                    value: Some(value),
                    unit: None,
                    system: None,
                    code: None,
                }),
            };

            // RXA-7: Administered Units
            let units_path = if rxa_index == 0 {
                "RXA-7".to_string()
            } else {
                format!("RXA({})-7", rxa_index + 1)
            };
            if let Ok(Some(units)) = terser.get(&units_path) {
                dosage.dose.as_mut().unwrap().unit = Some(units.to_string());
            }

            admin.dosage = Some(dosage);
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3") {
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
            _ => "completed".to_string(),
        }
    }

    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
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
        } else if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            Ok(format!(
                "{}-{}-{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8]
            ))
        } else {
            Err(ConversionError::InvalidFormat(
                "datetime".to_string(),
                "RXA".to_string(),
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
}
