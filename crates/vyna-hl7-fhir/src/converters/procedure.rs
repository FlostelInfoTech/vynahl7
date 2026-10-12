//! Procedure converter - PR1 segment to FHIR Procedure resource

use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::procedure::*;
use crate::resources::common::*;

pub struct ProcedureConverter;

impl ProcedureConverter {
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<Procedure>> {
        let pr1_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "PR1")
            .collect();

        if pr1_segments.is_empty() {
            return Err(ConversionError::MissingSegment("PR1".to_string()));
        }

        let mut procedures = Vec::new();
        for (pr1_count, _) in pr1_segments.iter().enumerate() {
            let procedure = Self::convert_single(message, pr1_count)?;
            procedures.push(procedure);
        }

        Ok(procedures)
    }

    pub fn convert_single(message: &Message, pr1_index: usize) -> ConversionResult<Procedure> {
        let terser = Terser::new(message);
        let mut procedure = Procedure::new("completed".to_string());

        // PR1-3: Procedure Code
        let code_path = if pr1_index == 0 {
            "PR1-3".to_string()
        } else {
            format!("PR1({})-3", pr1_index)
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

            // Component 2: Coding System (0-based)
            if let Ok(Some(system)) = terser.get(&format!("{}-2", code_path)) {
                coding.system = Some(Self::convert_coding_system(system));
            }

            procedure.code = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        }

        // PR1-5: Procedure Date/Time
        let datetime_path = if pr1_index == 0 {
            "PR1-5".to_string()
        } else {
            format!("PR1({})-5", pr1_index)
        };
        if let Ok(Some(datetime)) = terser.get(&datetime_path) {
            procedure.performed_date_time = Some(Self::convert_datetime(datetime)?);
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3") {
            procedure.subject = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        Ok(procedure)
    }

    fn convert_coding_system(system: &str) -> String {
        match system {
            "I9C" | "I9" => "http://hl7.org/fhir/sid/icd-9-cm".to_string(),
            "I10" | "I10P" => "http://hl7.org/fhir/sid/icd-10-pcs".to_string(),
            "C4" | "CPT" => "http://www.ama-assn.org/go/cpt".to_string(),
            _ => format!("urn:oid:{}", system),
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
        } else if datetime.len() >= 8 {
            Ok(format!(
                "{}-{}-{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8]
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
    fn test_convert_procedure() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PR1|1||0016^Appendectomy^I9C||20240315100000";

        let message = parse_message(hl7).unwrap();
        let procedures = ProcedureConverter::convert_all(&message).unwrap();

        assert_eq!(procedures.len(), 1);
        assert_eq!(procedures[0].resource_type, "Procedure");
        assert_eq!(procedures[0].status, "completed");
    }
}
