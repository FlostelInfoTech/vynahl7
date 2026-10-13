//! Condition converter - PRB/DG1 segments to FHIR Condition resource

use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::condition::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

pub struct ConditionConverter;

impl ConditionConverter {
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

        // PRB-3: Problem ID
        let code_path = if prb_index == 0 {
            "PRB-3".to_string()
        } else {
            format!("PRB({})-3", prb_index + 1)
        };
        if let Ok(Some(code)) = terser.get(&code_path) {
            condition.code = Some(CodeableConcept {
                coding: Some(vec![Coding {
                    system: None,
                    version: None,
                    code: Some(code.to_string()),
                    display: None,
                }]),
                text: None,
            });
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3") {
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
        if let Ok(Some(code)) = terser.get(&code_path) {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code.to_string()),
                display: None,
            };

            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path)) {
                coding.display = Some(text.to_string());
            }

            condition.code = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3") {
            condition.subject = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

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
}
