//! AllergyIntolerance converter - AL1 segment to FHIR AllergyIntolerance resource

use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::allergy_intolerance::*;
use crate::resources::common::*;

pub struct AllergyIntoleranceConverter;

impl AllergyIntoleranceConverter {
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<AllergyIntolerance>> {
        let al1_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "AL1")
            .collect();

        if al1_segments.is_empty() {
            return Err(ConversionError::MissingSegment("AL1".to_string()));
        }

        let mut allergies = Vec::new();
        for (al1_count, _) in al1_segments.iter().enumerate() {
            let allergy = Self::convert_single(message, al1_count)?;
            allergies.push(allergy);
        }

        Ok(allergies)
    }

    pub fn convert_single(message: &Message, al1_index: usize) -> ConversionResult<AllergyIntolerance> {
        let terser = Terser::new(message);
        let mut allergy = AllergyIntolerance::new();

        // AL1-2: Allergen Type Code
        let type_path = if al1_index == 0 {
            "AL1-2".to_string()
        } else {
            format!("AL1({})-2", al1_index)
        };
        if let Ok(Some(allergen_type)) = terser.get(&type_path) {
            allergy.category = Some(vec![Self::convert_allergen_type(allergen_type)]);
        }

        // AL1-3: Allergen Code/Mnemonic/Description
        let code_path = if al1_index == 0 {
            "AL1-3".to_string()
        } else {
            format!("AL1({})-3", al1_index)
        };
        if let Ok(Some(code)) = terser.get(&code_path) {
            allergy.code = Some(CodeableConcept {
                coding: Some(vec![Coding {
                    system: None,
                    version: None,
                    code: Some(code.to_string()),
                    display: None,
                }]),
                text: Some(code.to_string()),
            });
        }

        // AL1-4: Allergy Severity Code
        let severity_path = if al1_index == 0 {
            "AL1-4".to_string()
        } else {
            format!("AL1({})-4", al1_index)
        };
        if let Ok(Some(severity)) = terser.get(&severity_path) {
            allergy.criticality = Some(Self::convert_severity(severity));
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3") {
            allergy.patient = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        allergy.clinical_status = Some(CodeableConcept {
            coding: Some(vec![Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/allergyintolerance-clinical".to_string()),
                version: None,
                code: Some("active".to_string()),
                display: Some("Active".to_string()),
            }]),
            text: None,
        });

        Ok(allergy)
    }

    fn convert_allergen_type(code: &str) -> String {
        match code {
            "DA" => "medication".to_string(),
            "FA" => "food".to_string(),
            "MA" | "EA" => "environment".to_string(),
            _ => "environment".to_string(),
        }
    }

    fn convert_severity(code: &str) -> String {
        match code {
            "MI" => "low".to_string(),
            "MO" => "low".to_string(),
            "SV" => "high".to_string(),
            _ => "unable-to-assess".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_allergy() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|1191^ASPIRIN|SV";

        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();

        assert_eq!(allergies.len(), 1);
        assert_eq!(allergies[0].resource_type, "AllergyIntolerance");
        assert_eq!(allergies[0].category, Some(vec!["medication".to_string()]));
        assert_eq!(allergies[0].criticality, Some("high".to_string()));
    }
}
