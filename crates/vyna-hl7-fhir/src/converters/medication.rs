//! Medication converter - RXA segment to FHIR MedicationAdministration resource

use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::medication::*;
use crate::resources::common::*;

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

    pub fn convert_single(message: &Message, rxa_index: usize) -> ConversionResult<MedicationAdministration> {
        let terser = Terser::new(message);

        // RXA-20: Completion Status -> MedicationAdministration.status
        let status_path = if rxa_index == 0 {
            "RXA-20".to_string()
        } else {
            format!("RXA({})-20", rxa_index)
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
            format!("RXA({})-5", rxa_index)
        };
        if let Ok(Some(code)) = terser.get(&code_path) {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code.to_string()),
                display: None,
            };

            // Component 1: Text (0-based)
            if let Ok(Some(text)) = terser.get(&format!("{}-1", code_path)) {
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
            format!("RXA({})-3", rxa_index)
        };
        if let Ok(Some(datetime)) = terser.get(&datetime_path) {
            admin.effective_date_time = Some(Self::convert_datetime(datetime)?);
        }

        // RXA-6: Administered Amount
        let amount_path = if rxa_index == 0 {
            "RXA-6".to_string()
        } else {
            format!("RXA({})-6", rxa_index)
        };
        if let Ok(Some(amount)) = terser.get(&amount_path)
            && let Ok(value) = amount.parse::<f64>() {
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
                    format!("RXA({})-7", rxa_index)
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

    fn convert_status(code: &str) -> String {
        match code {
            "CP" => "completed".to_string(),
            "PA" => "stopped".to_string(),
            "NA" => "entered-in-error".to_string(),
            _ => "completed".to_string(),
        }
    }

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
        } else {
            Ok(datetime.to_string())
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
}
