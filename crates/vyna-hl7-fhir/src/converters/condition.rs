//! Condition converter - PRB/DG1 segments to FHIR Condition resource

use super::coding_systems::convert_coding_system;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::condition::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming PRB/DG1 segments to FHIR Condition resources
pub struct ConditionConverter;

impl ConditionConverter {
    /// Convert all PRB and DG1 segments in message to Condition resources
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<Condition>> {
        let prb_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "PRB" || s.id == "DG1")
            .collect();

        if prb_segments.is_empty() {
            return Err(ConversionError::MissingSegment("PRB or DG1".to_string()));
        }

        // Each segment type has its own Terser index space, so the
        // combined PRB|DG1 position must not be reused as the
        // type-specific index in mixed messages.
        let mut conditions = Vec::new();
        let mut prb_index = 0;
        let mut dg1_index = 0;
        for (_, seg) in prb_segments.iter() {
            let condition = if seg.id == "PRB" {
                let condition = Self::convert_prb(message, prb_index)?;
                prb_index += 1;
                condition
            } else {
                let condition = Self::convert_dg1(message, dg1_index)?;
                dg1_index += 1;
                condition
            };
            conditions.push(condition);
        }

        Ok(conditions)
    }

    fn convert_prb(message: &Message, prb_index: usize) -> ConversionResult<Condition> {
        let terser = Terser::new(message);
        let mut condition = Condition::new();

        // PRB-3: Problem ID (CWE: code^text^coding system)
        let code_path = if prb_index == 0 {
            "PRB-3".to_string()
        } else {
            format!("PRB({})-3", prb_index + 1)
        };
        // Empty is absent: no code concept for a blank Problem ID,
        // but a text-only CWE keeps its text like the allergy
        // converter.
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

            // Component 3: Coding System (1-based indexing)
            if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
                && !system.is_empty()
            {
                coding.system = Some(convert_coding_system(system));
            }

            condition.code = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        } else if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
            && !text.is_empty()
        {
            condition.code = Some(CodeableConcept {
                coding: None,
                text: Some(text.to_string()),
            });
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty()
        {
            condition.subject = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        condition.clinical_status = Some(CodeableConcept {
            coding: Some(vec![Coding {
                system: Some(
                    "http://terminology.hl7.org/CodeSystem/condition-clinical".to_string(),
                ),
                version: None,
                code: Some("active".to_string()),
                display: None,
            }]),
            text: None,
        });

        Ok(condition)
    }

    fn convert_dg1(message: &Message, dg1_index: usize) -> ConversionResult<Condition> {
        let terser = Terser::new(message);
        let mut condition = Condition::new();

        // DG1-3: Diagnosis Code
        let code_path = if dg1_index == 0 {
            "DG1-3".to_string()
        } else {
            format!("DG1({})-3", dg1_index + 1)
        };
        // Empty is absent: no code concept for a blank Diagnosis
        // Code, but a text-only CWE keeps its text like the
        // allergy converter.
        if let Ok(Some(code)) = terser.get(&code_path)
            && !code.is_empty()
        {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code.to_string()),
                display: None,
            };

            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
                && !text.is_empty()
            {
                coding.display = Some(text.to_string());
            }

            // Component 3: Coding System (1-based indexing)
            if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
                && !system.is_empty()
            {
                coding.system = Some(convert_coding_system(system));
            }

            condition.code = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        } else if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
            && !text.is_empty()
        {
            condition.code = Some(CodeableConcept {
                coding: None,
                text: Some(text.to_string()),
            });
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty()
        {
            condition.subject = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        condition.clinical_status = Some(CodeableConcept {
            coding: Some(vec![Coding {
                system: Some(
                    "http://terminology.hl7.org/CodeSystem/condition-clinical".to_string(),
                ),
                version: None,
                code: Some("active".to_string()),
                display: None,
            }]),
            text: None,
        });

        Ok(condition)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_condition_dg1() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   DG1|1||I10^Essential Hypertension^ICD10";

        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();

        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions[0].resource_type, "Condition");
    }

    #[test]
    fn test_mixed_prb_dg1_message_uses_type_specific_indexes() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||PPR^PC1|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PRB|AD|20240315|AAA^A^SCT|P1\r\
                   DG1|1||BBB^B^ICD10\r\
                   PRB|AD|20240315|CCC^C^SCT|P2\r\
                   DG1|2||DDD^D^ICD10";

        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();

        assert_eq!(conditions.len(), 4);
        let code = |c: &Condition| {
            c.code.as_ref().unwrap().coding.as_ref().unwrap()[0]
                .code
                .clone()
        };
        assert_eq!(code(&conditions[0]).as_deref(), Some("AAA"));
        assert_eq!(code(&conditions[1]).as_deref(), Some("BBB"));
        assert_eq!(code(&conditions[2]).as_deref(), Some("CCC"));
        assert_eq!(code(&conditions[3]).as_deref(), Some("DDD"));
    }

    #[test]
    fn test_convert_condition_second_repetition_reads_second_segment() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||PPR^PC1|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PRB|AD|20240315|38341003^HTN^SCT|P1\r\
                   PRB|AD|20240315|44054006^DIAB^SCT|P2";

        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();

        assert_eq!(conditions.len(), 2);
        let code = |c: &Condition| {
            c.code.as_ref().unwrap().coding.as_ref().unwrap()[0]
                .code
                .clone()
        };
        assert_eq!(code(&conditions[0]).as_deref(), Some("38341003"));
        assert_eq!(code(&conditions[1]).as_deref(), Some("44054006"));
    }

    #[test]
    fn test_prb_reads_display_and_mapped_system() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||PPR^PC1|1|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PRB|AD|20240315|38341003^HTN^SCT|P1";
        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();
        let coding = &conditions[0]
            .code
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("38341003"));
        assert_eq!(coding.display.as_deref(), Some("HTN"));
        assert_eq!(coding.system.as_deref(), Some("http://snomed.info/sct"));
    }

    #[test]
    fn test_empty_prb3_and_pid3_omitted() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||PPR^PC1|1|P|2.5\r\
                   PID|1||||DOE^JOHN||19800101|M\r\
                   PRB|AD|20240315||P1";
        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();
        assert!(conditions[0].code.is_none());
        assert!(conditions[0].subject.is_none());
    }

    #[test]
    fn test_dg1_empty_display_omitted() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   DG1|1||I10^^ICD10";
        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();
        let coding = &conditions[0]
            .code
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("I10"));
        assert!(coding.display.is_none());
    }

    #[test]
    fn test_dg1_system_mapped_from_component_3() {
        // DG1-3-3 selects the system, like PRB-3-3 does.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   DG1|1||I10^Essential Hypertension^ICD10";
        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();
        let coding = &conditions[0]
            .code
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("I10"));
        assert_eq!(coding.display.as_deref(), Some("Essential Hypertension"));
        assert_eq!(
            coding.system.as_deref(),
            Some("http://hl7.org/fhir/sid/icd-10")
        );
    }

    #[test]
    fn test_dg1_sets_active_clinical_status() {
        // The DG1 path sets the same active clinicalStatus as PRB.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   DG1|1||I10^Essential Hypertension^ICD10";
        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();
        let coding = &conditions[0]
            .clinical_status
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("active"));
        assert_eq!(
            coding.system.as_deref(),
            Some("http://terminology.hl7.org/CodeSystem/condition-clinical")
        );
    }

    #[test]
    fn test_prb_text_only_code_preserved() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||PPR^PC1|1|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PRB|AD|20240315|^Chest pain|P1";
        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();
        let code = conditions[0].code.as_ref().unwrap();
        assert_eq!(code.coding, None);
        assert_eq!(code.text.as_deref(), Some("Chest pain"));
    }

    #[test]
    fn test_dg1_text_only_code_preserved() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   DG1|1||^Free text dx";
        let message = parse_message(hl7).unwrap();
        let conditions = ConditionConverter::convert_all(&message).unwrap();
        let code = conditions[0].code.as_ref().unwrap();
        assert_eq!(code.coding, None);
        assert_eq!(code.text.as_deref(), Some("Free text dx"));
    }
}
