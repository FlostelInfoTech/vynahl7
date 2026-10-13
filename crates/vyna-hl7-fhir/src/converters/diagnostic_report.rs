//! DiagnosticReport converter - OBR segment to FHIR DiagnosticReport resource

use super::patient::{split_hl7_offset, valid_hl7_date, valid_hl7_hour_min, valid_hl7_time};
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

        // Link this report's own observations: the OBX segments
        // following its OBR in message order, up to the next OBR. IDs use
        // the same `obs-{OBX-1}` scheme as ObservationConverter so the
        // references resolve.
        let obr_positions: Vec<usize> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "OBR")
            .map(|(i, _)| i)
            .collect();
        if let Some(&obr_pos) = obr_positions.get(obr_index) {
            let end = message.segments[obr_pos + 1..]
                .iter()
                .position(|s| s.id == "OBR")
                .map(|p| obr_pos + 1 + p)
                .unwrap_or(message.segments.len());
            let mut result_refs = Vec::new();
            for segment in &message.segments[obr_pos + 1..end] {
                if segment.id != "OBX" {
                    continue;
                }
                if let Some(set_id) = segment.get_field_value(1)
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

    /// `YYYYMMDDHHMM` keeps its minutes and a trailing HL7 zone is
    /// preserved as `±HH:MM`.
    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        let (stem, offset) = split_hl7_offset(datetime);
        let zone = offset.as_deref().unwrap_or("");
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
        let bytes = stem.as_bytes();
        if bytes.len() >= 14 && bytes[..14].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) && valid_hl7_time(&stem[8..14]) {
                Ok(format!(
                    "{}-{}-{}T{}:{}:{}{}",
                    &stem[0..4],
                    &stem[4..6],
                    &stem[6..8],
                    &stem[8..10],
                    &stem[10..12],
                    &stem[12..14],
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
            "OBR".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
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
    fn test_impossible_calendar_datetime_rejected() {
        assert!(DiagnosticReportConverter::convert_datetime("20240230100000").is_err());

        // OBR-7 is a primary timestamp: an impossible date must fail.
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBR|1|LAB123|LAB123-01|24331-1^Lipid Panel^LN|||20240230100000";

        let message = parse_message(hl7).unwrap();
        assert!(DiagnosticReportConverter::convert_all(&message).is_err());
    }

    #[test]
    fn test_each_report_links_only_its_own_obx() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBR|1|||24331-1^Lipid Panel^LN\r\
                   OBX|1|NM|2093-3^Cholesterol^LN||195|mg/dL||||F\r\
                   OBR|2|||24342-8^Metabolic Panel^LN\r\
                   OBX|2|NM|2085-9^HDL^LN||55|mg/dL||||F";

        let message = parse_message(hl7).unwrap();
        let reports = DiagnosticReportConverter::convert_all(&message).unwrap();
        assert_eq!(reports.len(), 2);
        let refs = |report: &DiagnosticReport| {
            report
                .result
                .as_ref()
                .unwrap()
                .iter()
                .map(|r| r.reference.clone().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(refs(&reports[0]), vec!["Observation/obs-1"]);
        assert_eq!(refs(&reports[1]), vec!["Observation/obs-2"]);
    }

    #[test]
    fn test_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            DiagnosticReportConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            DiagnosticReportConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
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
