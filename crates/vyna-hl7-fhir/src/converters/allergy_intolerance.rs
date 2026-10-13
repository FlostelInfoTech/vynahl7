//! AllergyIntolerance converter - AL1 segment to FHIR AllergyIntolerance resource

use super::coding_systems::convert_coding_system;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::allergy_intolerance::*;
use crate::resources::common::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming AL1 segments to FHIR AllergyIntolerance resources
pub struct AllergyIntoleranceConverter;

impl AllergyIntoleranceConverter {
    /// Convert all AL1 segments in message to AllergyIntolerance resources
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

    /// Convert a specific AL1 segment to an AllergyIntolerance resource
    ///
    /// # Arguments
    /// * `message` - HL7 message containing AL1 segment
    /// * `al1_index` - 0-based index (0 = first AL1, 1 = second AL1, etc.)
    pub fn convert_single(
        message: &Message,
        al1_index: usize,
    ) -> ConversionResult<AllergyIntolerance> {
        let terser = Terser::new(message);
        let mut allergy = AllergyIntolerance::new();

        // AL1-2: Allergen Type Code
        let type_path = if al1_index == 0 {
            "AL1-2".to_string()
        } else {
            format!("AL1({})-2", al1_index + 1)
        };
        // Empty is absent: it leaves the category unset instead of
        // defaulting to environment.
        if let Ok(Some(allergen_type)) = terser.get(&type_path)
            && !allergen_type.is_empty()
        {
            allergy.category = Some(vec![Self::convert_allergen_type(allergen_type)]);
        }

        // AL1-3: Allergen Code/Mnemonic/Description
        let code_path = if al1_index == 0 {
            "AL1-3".to_string()
        } else {
            format!("AL1({})-3", al1_index + 1)
        };
        // Empty is absent (same rule as the allergen type above),
        // but a text-only CWE (empty component 1, valued component 2)
        // keeps its text instead of dropping the code entirely.
        let code = terser
            .get(&code_path)
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        // AL1-3-2: description text, AL1-3-3: coding system (1-based indexing)
        let text = terser
            .get(&format!("{code_path}-2"))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let system = terser
            .get(&format!("{code_path}-3"))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .map(convert_coding_system);
        match (code, text) {
            (Some(code), text) => {
                allergy.code = Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system,
                        version: None,
                        code: Some(code.clone()),
                        display: text.clone(),
                    }]),
                    text: text.or(Some(code)),
                });
            }
            (None, Some(text)) => {
                allergy.code = Some(CodeableConcept {
                    coding: None,
                    text: Some(text),
                });
            }
            (None, None) => {}
        }

        // AL1-4: Allergy Severity Code
        let severity_path = if al1_index == 0 {
            "AL1-4".to_string()
        } else {
            format!("AL1({})-4", al1_index + 1)
        };
        // Empty is absent (same rule as the allergen type above).
        if let Ok(Some(severity)) = terser.get(&severity_path)
            && !severity.is_empty()
        {
            allergy.criticality = Some(Self::convert_severity(severity));
        }

        // Link to patient. Empty is absent: it must not become a
        // `Patient/` reference.
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty()
        {
            allergy.patient = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        allergy.clinical_status = Some(CodeableConcept {
            coding: Some(vec![Coding {
                system: Some(
                    "http://terminology.hl7.org/CodeSystem/allergyintolerance-clinical".to_string(),
                ),
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

    #[test]
    fn test_convert_allergy_second_repetition_reads_second_segment() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|1191^ASPIRIN|SV\r\
                   AL1|2|FA|716^PEANUT|MO";

        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();

        assert_eq!(allergies.len(), 2);
        assert_eq!(allergies[0].category, Some(vec!["medication".to_string()]));
        assert_eq!(allergies[1].category, Some(vec!["food".to_string()]));
        assert_eq!(allergies[1].criticality, Some("low".to_string()));
        let code = |a: &AllergyIntolerance| {
            a.code.as_ref().unwrap().coding.as_ref().unwrap()[0]
                .code
                .clone()
        };
        assert_eq!(code(&allergies[0]).as_deref(), Some("1191"));
        assert_eq!(code(&allergies[1]).as_deref(), Some("716"));
    }

    #[test]
    fn test_allergy_reads_description_and_system() {
        // AL1-3 is CWE: component 1 = code, 2 = text, 3 = coding system.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|1191^ASPIRIN^RXNORM|SV";

        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        let concept = allergies[0].code.as_ref().unwrap();
        let coding = &concept.coding.as_ref().unwrap()[0];

        assert_eq!(coding.code.as_deref(), Some("1191"));
        assert_eq!(coding.display.as_deref(), Some("ASPIRIN"));
        assert_eq!(
            coding.system.as_deref(),
            Some("http://www.nlm.nih.gov/research/umls/rxnorm")
        );
        assert_eq!(concept.text.as_deref(), Some("ASPIRIN"));
    }

    #[test]
    fn test_empty_al1_fields_treated_as_absent() {
        // AL1-2 present-but-empty leaves the category unset instead of
        // defaulting to environment.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1||1191^ASPIRIN|SV";
        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        assert_eq!(allergies[0].category, None);
        // Valued siblings still convert.
        assert!(allergies[0].code.is_some());
        assert_eq!(allergies[0].criticality.as_deref(), Some("high"));

        // Empty code and severity are likewise absent.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA||";
        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        assert_eq!(allergies[0].category, Some(vec!["medication".to_string()]));
        assert_eq!(allergies[0].code, None);
        assert_eq!(allergies[0].criticality, None);
    }

    #[test]
    fn test_text_only_allergen_code_preserved() {
        // AL1-3 with empty component 1 but valued text: keep the text.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|^ASPIRIN|SV";
        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        let concept = allergies[0].code.as_ref().unwrap();
        assert_eq!(concept.coding, None);
        assert_eq!(concept.text.as_deref(), Some("ASPIRIN"));
    }

    #[test]
    fn test_empty_pid3_omits_patient() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|1191^ASPIRIN|SV";
        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        assert_eq!(allergies[0].patient, None);
    }

    #[test]
    fn test_allergy_system_uses_shared_table() {
        // AL1-3-3 resolves through the shared coding-system table:
        // known systems become FHIR URIs, unknown ones keep the
        // `urn:oid:` fallback.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|1191^ASPIRIN^RXNORM|SV";
        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        let coding = &allergies[0].code.as_ref().unwrap().coding.as_ref().unwrap()[0];
        assert_eq!(
            coding.system.as_deref(),
            Some("http://www.nlm.nih.gov/research/umls/rxnorm")
        );

        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|1191^ASPIRIN^LOCAL|SV";
        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        let coding = &allergies[0].code.as_ref().unwrap().coding.as_ref().unwrap()[0];
        assert_eq!(coding.system.as_deref(), Some("urn:oid:LOCAL"));
    }

    #[test]
    fn test_allergy_text_falls_back_to_code() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   AL1|1|DA|PENICILLIN|SV";

        let message = parse_message(hl7).unwrap();
        let allergies = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        let concept = allergies[0].code.as_ref().unwrap();

        assert_eq!(concept.text.as_deref(), Some("PENICILLIN"));
        assert_eq!(concept.coding.as_ref().unwrap()[0].display.as_deref(), None);
    }
}
