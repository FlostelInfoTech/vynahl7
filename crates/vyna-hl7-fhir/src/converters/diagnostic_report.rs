//! DiagnosticReport converter - OBR segment to FHIR DiagnosticReport resource

use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::diagnostic_report::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming OBR segments to FHIR DiagnosticReport resources
pub struct DiagnosticReportConverter;

impl DiagnosticReportConverter {
    /// Convert an HL7 v2 message containing OBR segments to FHIR DiagnosticReport resources
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<DiagnosticReport>> {
        let obr_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "OBR")
            .collect();

        if obr_segments.is_empty() {
            return Err(ConversionError::MissingSegment("OBR".to_string()));
        }

        let mut reports = Vec::new();
        for (obr_count, _) in obr_segments.iter().enumerate() {
            let report = Self::convert_single(message, obr_count)?;
            reports.push(report);
        }

        Ok(reports)
    }

    /// Convert a single OBR segment to a FHIR DiagnosticReport resource
    pub fn convert_single(
        message: &Message,
        obr_index: usize,
    ) -> ConversionResult<DiagnosticReport> {
        let terser = Terser::new(message);

        // OBR-4: Universal Service Identifier -> DiagnosticReport.code
        let code_path = if obr_index == 0 {
            "OBR-4".to_string()
        } else {
            format!("OBR({})-4", obr_index + 1)
        };

        let code = if let Ok(Some(code_id)) = terser.get(&code_path) {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code_id.to_string()),
                display: None,
            };

            // OBR-4-2: Text (component 2, 1-based indexing)
            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
                && !text.is_empty()
            {
                coding.display = Some(text.to_string());
            }

            // OBR-4-3: Coding System (component 3, 1-based indexing)
            if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
                && !system.is_empty()
            {
                coding.system = Some(Self::convert_coding_system(system));
            }

            CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            }
        } else {
            return Err(ConversionError::MissingField(
                "OBR-4".to_string(),
                "OBR".to_string(),
            ));
        };

        // OBR-25: Result Status -> DiagnosticReport.status
        let status_path = if obr_index == 0 {
            "OBR-25".to_string()
        } else {
            format!("OBR({})-25", obr_index + 1)
        };
        let status = if let Ok(Some(status_code)) = terser.get(&status_path) {
            Self::convert_status(status_code)
        } else {
            "final".to_string()
        };

        let mut report = DiagnosticReport::new(status, code);

        // OBR-2: Placer Order Number -> DiagnosticReport.identifier
        let placer_path = if obr_index == 0 {
            "OBR-2".to_string()
        } else {
            format!("OBR({})-2", obr_index + 1)
        };
        if let Ok(Some(placer)) = terser.get(&placer_path)
            && !placer.is_empty()
        {
            report.identifier = Some(vec![Identifier {
                use_: Some("official".to_string()),
                type_: None,
                system: None,
                value: Some(placer.to_string()),
                assigner: None,
            }]);
            report.id = Some(placer.to_string());
        }

        // OBR-7: Observation Date/Time -> DiagnosticReport.effectiveDateTime
        let obs_dt_path = if obr_index == 0 {
            "OBR-7".to_string()
        } else {
            format!("OBR({})-7", obr_index + 1)
        };
        if let Ok(Some(obs_dt)) = terser.get(&obs_dt_path)
            && !obs_dt.is_empty()
        {
            report.effective_date_time = Some(Self::convert_datetime(obs_dt)?);
        }

        // OBR-22: Results Report Date/Time -> DiagnosticReport.issued
        let issued_path = if obr_index == 0 {
            "OBR-22".to_string()
        } else {
            format!("OBR({})-22", obr_index + 1)
        };
        if let Ok(Some(issued)) = terser.get(&issued_path)
            && !issued.is_empty()
        {
            report.issued = Some(Self::convert_datetime(issued)?);
        }

        // Link to patient from PID segment
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty()
        {
            report.subject = Some(Reference {
                reference: Some(format!("Patient/{}", patient_id)),
                type_: Some("Patient".to_string()),
                identifier: None,
                display: None,
            });
        }

        // Link to observations (simple approach: link all OBX in
        // message). IDs use the same `obs-{OBX-1}` scheme as
        // ObservationConverter so the references resolve.
        let obx_count = message.segments.iter().filter(|s| s.id == "OBX").count();
        let mut result_refs = Vec::new();
        for obx_index in 0..obx_count {
            let set_id_path = if obx_index == 0 {
                "OBX-1".to_string()
            } else {
                format!("OBX({})-1", obx_index + 1)
            };
            if let Ok(Some(set_id)) = terser.get(&set_id_path)
                && !set_id.is_empty()
            {
                result_refs.push(Reference {
                    reference: Some(format!("Observation/obs-{}", set_id)),
                    type_: Some("Observation".to_string()),
                    identifier: None,
                    display: None,
                });
            }
        }
        if !result_refs.is_empty() {
            report.result = Some(result_refs);
        }

        Ok(report)
    }

    fn convert_status(code: &str) -> String {
        match code {
            "O" => "registered".to_string(),
            "P" => "preliminary".to_string(),
            "F" => "final".to_string(),
            "C" => "corrected".to_string(),
            "X" => "cancelled".to_string(),
            _ => "final".to_string(),
        }
    }

    fn convert_coding_system(system: &str) -> String {
        match system {
            "LN" | "LNC" => "http://loinc.org".to_string(),
            "SNM" | "SCT" => "http://snomed.info/sct".to_string(),
            _ => format!("urn:oid:{}", system),
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
                "OBR".to_string(),
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
    fn test_convert_diagnostic_report() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBR|1|LAB123|LAB123-01|24331-1^Lipid Panel^LN|||20240315080000|||||||||||||||20240315100000||||F\r\
                   OBX|1|NM|2093-3^Cholesterol^LN||195|mg/dL||||F";

        let message = parse_message(hl7).unwrap();
        let reports = DiagnosticReportConverter::convert_all(&message).unwrap();

        assert_eq!(reports.len(), 1);
        let report = &reports[0];

        assert_eq!(report.resource_type, "DiagnosticReport");
        assert_eq!(report.status, "final");
        assert_eq!(
            report.code.coding.as_ref().unwrap()[0].code,
            Some("24331-1".to_string())
        );
    }

    #[test]
    fn test_result_refs_match_observation_ids() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBR|1|LAB123|LAB123-01|24331-1^Lipid Panel^LN\r\
                   OBX|1|NM|2093-3^Cholesterol^LN||195|mg/dL||||F\r\
                   OBX|2|NM|2085-9^HDL^LN||55|mg/dL||||F";

        let message = parse_message(hl7).unwrap();
        let observations =
            crate::converters::observation::ObservationConverter::convert_all(&message).unwrap();
        let reports = DiagnosticReportConverter::convert_all(&message).unwrap();

        let refs: Vec<_> = reports[0]
            .result
            .as_ref()
            .unwrap()
            .iter()
            .map(|r| r.reference.clone().unwrap())
            .collect();
        let expected: Vec<_> = observations
            .iter()
            .map(|o| format!("Observation/{}", o.id.clone().unwrap()))
            .collect();
        assert_eq!(refs, expected);
        assert_eq!(refs, vec!["Observation/obs-1", "Observation/obs-2"]);
    }

    #[test]
    fn test_non_ascii_datetime_returns_error_without_panic() {
        assert!(DiagnosticReportConverter::convert_datetime("123\u{e9}56789012").is_err());
        assert_eq!(
            DiagnosticReportConverter::convert_datetime("20240315100000").unwrap(),
            "2024-03-15T10:00:00"
        );
    }

    #[test]
    fn test_convert_report_second_repetition_reads_second_segment() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ORU^R01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBR|1|||24331-1^Lipid Panel^LN\r\
                   OBR|2|||24342-8^Metabolic Panel^LN";

        let message = parse_message(hl7).unwrap();
        let reports = DiagnosticReportConverter::convert_all(&message).unwrap();

        assert_eq!(reports.len(), 2);
        assert_eq!(
            reports[0].code.coding.as_ref().unwrap()[0].code,
            Some("24331-1".to_string())
        );
        assert_eq!(
            reports[1].code.coding.as_ref().unwrap()[0].code,
            Some("24342-8".to_string())
        );
    }
}
