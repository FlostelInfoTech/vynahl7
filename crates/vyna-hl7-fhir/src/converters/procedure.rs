//! Procedure converter - PR1 segment to FHIR Procedure resource

use super::coding_systems::convert_coding_system;
use super::patient::{
    hl7_fraction_suffix, split_hl7_offset, valid_hl7_date, valid_hl7_hour_min, valid_hl7_time,
    valid_hl7_year_month,
};
use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::procedure::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming PR1 segments to FHIR Procedure resources
pub struct ProcedureConverter;

impl ProcedureConverter {
    /// Convert all PR1 segments in message to Procedure resources
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

    /// Convert a specific PR1 segment to a Procedure resource
    ///
    /// # Arguments
    /// * `message` - HL7 message containing PR1 segment
    /// * `pr1_index` - 0-based index (0 = first PR1, 1 = second PR1, etc.)
    pub fn convert_single(message: &Message, pr1_index: usize) -> ConversionResult<Procedure> {
        let terser = Terser::new(message);
        let mut procedure = Procedure::new("completed".to_string());

        // PR1-3: Procedure Code
        let code_path = if pr1_index == 0 {
            "PR1-3".to_string()
        } else {
            format!("PR1({})-3", pr1_index + 1)
        };
        // Empty is absent on every part: an empty code leaves the
        // whole code unset, and empty text/system stay None instead
        // of `""` / a fabricated `urn:oid:`. A text-only CWE keeps
        // its text like the allergy converter.
        if let Ok(Some(code)) = terser.get(&code_path)
            && !code.is_empty()
        {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code.to_string()),
                display: None,
            };

            // Component 1: Text (1-based indexing)
            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
                && !text.is_empty()
            {
                coding.display = Some(text.to_string());
            }

            // Component 2: Coding System (1-based indexing)
            if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
                && !system.is_empty()
            {
                coding.system = Some(Self::convert_coding_system(system));
            }

            procedure.code = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        } else if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
            && !text.is_empty()
        {
            procedure.code = Some(CodeableConcept {
                coding: None,
                text: Some(text.to_string()),
            });
        }

        // PR1-5: Procedure Date/Time
        let datetime_path = if pr1_index == 0 {
            "PR1-5".to_string()
        } else {
            format!("PR1({})-5", pr1_index + 1)
        };
        if let Ok(Some(datetime)) = terser.get(&datetime_path)
            && !datetime.is_empty()
        {
            procedure.performed_date_time = Some(Self::convert_datetime(datetime)?);
        }

        // Link to patient
        if let Ok(Some(patient_id)) = terser.get("PID-3")
            && !patient_id.is_empty()
        {
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
            // Table 0396: only I10P is the procedure system
            // (icd-10-pcs); I10 (diagnosis) falls through to
            // the shared table (icd-10).
            "I10P" => "http://hl7.org/fhir/sid/icd-10-pcs".to_string(),
            "C4" | "CPT" => "http://www.ama-assn.org/go/cpt".to_string(),
            _ => convert_coding_system(system),
        }
    }

    /// `YYYYMMDD` -> `YYYY-MM-DD` (`YYYYMM` -> `YYYY-MM`,
    /// `YYYY` stays a year).
    fn convert_date(date: &str) -> ConversionResult<String> {
        let (stem, _) = split_hl7_offset(date);
        let bytes = stem.as_bytes();
        if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) {
                Ok(format!("{}-{}-{}", &stem[0..4], &stem[4..6], &stem[6..8]))
            } else {
                Err(Self::invalid_datetime(date))
            }
        } else if bytes.len() >= 6 && bytes[..6].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_year_month(&stem[0..6]) {
                Ok(format!("{}-{}", &stem[0..4], &stem[4..6]))
            } else {
                Err(Self::invalid_datetime(date))
            }
        } else if bytes.len() >= 4 && bytes[..4].iter().all(|b| b.is_ascii_digit()) {
            Ok(stem[0..4].to_string())
        } else {
            Err(Self::invalid_datetime(date))
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
        } else if bytes.len() >= 4 {
            // Date, month, or year precision, like the patient
            // converter.
            Self::convert_date(stem)
        } else {
            Err(Self::invalid_datetime(datetime))
        }
    }


    /// Redacted bad-datetime error: field location + reason only, never the value.
    fn invalid_datetime(_datetime: &str) -> ConversionError {
        ConversionError::InvalidFormat(
            "datetime".to_string(),
            "PR1".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
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

    #[test]
    fn test_non_ascii_datetime_returns_error_without_panic() {
        assert!(ProcedureConverter::convert_datetime("123\u{e9}56789012").is_err());
        assert!(ProcedureConverter::convert_datetime("junk").is_err());
        assert_eq!(
            ProcedureConverter::convert_datetime("20240315").unwrap(),
            "2024-03-15"
        );
    }

    #[test]
    fn test_impossible_calendar_datetime_rejected() {
        assert!(ProcedureConverter::convert_datetime("20240230100000").is_err());

        // PR1-5 is a primary timestamp: an impossible date must fail.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PR1|1||0016^Appendectomy^I9C||20240230100000";

        let message = parse_message(hl7).unwrap();
        assert!(ProcedureConverter::convert_all(&message).is_err());
    }

    #[test]
    fn test_convert_procedure_reads_each_segment_with_code_and_display() {
        // PR1-3 is CWE: component 1 = code, 2 = text, 3 = coding system.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PR1|1||0016^Appendectomy^I9C||20240315100000\r\
                   PR1|2||0017^Cholecystectomy^I9C||20240315110000";

        let message = parse_message(hl7).unwrap();
        let procedures = ProcedureConverter::convert_all(&message).unwrap();

        assert_eq!(procedures.len(), 2);
        let coding = |p: &Procedure| {
            p.code
                .as_ref()
                .unwrap()
                .coding
                .as_ref()
                .unwrap()
                .first()
                .cloned()
                .unwrap()
        };
        let first = coding(&procedures[0]);
        assert_eq!(first.code.as_deref(), Some("0016"));
        assert_eq!(first.display.as_deref(), Some("Appendectomy"));
        // Coding systems are mapped to FHIR URIs.
        assert_eq!(
            first.system.as_deref(),
            Some("http://hl7.org/fhir/sid/icd-9-cm")
        );
        let second = coding(&procedures[1]);
        assert_eq!(second.code.as_deref(), Some("0017"));
        assert_eq!(second.display.as_deref(), Some("Cholecystectomy"));
    }

    #[test]
    fn test_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            ProcedureConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            ProcedureConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );
    }

    #[test]
    fn test_empty_code_parts_treated_as_absent() {
        // Empty system stays None instead of a fabricated `urn:oid:`.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PR1|1||0016^Appendectomy^||20240315100000";
        let message = parse_message(hl7).unwrap();
        let procedure = &ProcedureConverter::convert_all(&message).unwrap()[0];
        let coding = &procedure.code.as_ref().unwrap().coding.as_ref().unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("0016"));
        assert_eq!(coding.display.as_deref(), Some("Appendectomy"));
        assert_eq!(coding.system, None);

        // Empty code leaves the whole code unset.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PR1|1|||X";
        let message = parse_message(hl7).unwrap();
        let procedure = &ProcedureConverter::convert_all(&message).unwrap()[0];
        assert_eq!(procedure.code, None);
    }

    #[test]
    fn test_empty_pid3_omits_subject() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ADT^A01|1|P|2.5\r\
                   PID|1||||DOE^JOHN||19800101|M\r\
                   PR1|1||0016^Appendectomy^I9C||20240315100000";
        let message = parse_message(hl7).unwrap();
        let procedure = &ProcedureConverter::convert_all(&message).unwrap()[0];
        assert!(procedure.subject.is_none());
    }

    #[test]
    fn test_datetime_preserves_fractional_seconds() {
        assert_eq!(
            ProcedureConverter::convert_datetime("20240315100000.123").unwrap(),
            "2024-03-15T10:00:00.123"
        );
    }

    #[test]
    fn test_text_only_procedure_code_preserved() {
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PR1|1||^Appendectomy free text||20240315100000";
        let message = parse_message(hl7).unwrap();
        let procedure = &ProcedureConverter::convert_all(&message).unwrap()[0];
        let code = procedure.code.as_ref().unwrap();
        assert_eq!(code.coding, None);
        assert_eq!(code.text.as_deref(), Some("Appendectomy free text"));
    }

    #[test]
    fn test_datetime_accepts_year_month_precision() {
        assert_eq!(
            ProcedureConverter::convert_datetime("202401").unwrap(),
            "2024-01"
        );
        assert_eq!(
            ProcedureConverter::convert_datetime("1980").unwrap(),
            "1980"
        );
    }

    #[test]
    fn test_coding_system_uses_shared_table_fallback() {
        // SCT/ICD10/LN resolve via the shared table, not `urn:oid:`.
        for (system, expected) in [
            ("SCT", "http://snomed.info/sct"),
            ("ICD10", "http://hl7.org/fhir/sid/icd-10"),
            ("LN", "http://loinc.org"),
        ] {
            let hl7 = format!(
                "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|M|P|2.5\r\
                 PID|1||12345|||DOE^JOHN||19800101|M\r\
                 PR1|1||0016^Appendectomy^{system}||20240315100000"
            );
            let message = parse_message(&hl7).unwrap();
            let proc = &ProcedureConverter::convert_all(&message).unwrap()[0];
            let coding = &proc.code.as_ref().unwrap().coding.as_ref().unwrap()[0];
            assert_eq!(coding.system.as_deref(), Some(expected), "PR1-3 {system}");
        }
        // Procedure-specific arms are unchanged.
        for (system, expected) in [
            ("I9C", "http://hl7.org/fhir/sid/icd-9-cm"),
            ("C4", "http://www.ama-assn.org/go/cpt"),
            ("CPT", "http://www.ama-assn.org/go/cpt"),
        ] {
            assert_eq!(
                ProcedureConverter::convert_coding_system(system),
                expected,
                "PR1-3 {system}"
            );
        }
    }

    #[test]
    fn test_i10_maps_to_icd10_diagnosis_table_0396() {
        // HL7 table 0396: I10 is the diagnosis system
        // (http://hl7.org/fhir/sid/icd-10); only I10P is the
        // procedure system (icd-10-pcs). I10 falls through to
        // the shared table like everywhere else.
        let hl7 = "MSH|^~\\&|System|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   PR1|1||0016^Appendectomy^I10||20240315100000";
        let message = parse_message(hl7).unwrap();
        let proc = &ProcedureConverter::convert_all(&message).unwrap()[0];
        let coding = &proc.code.as_ref().unwrap().coding.as_ref().unwrap()[0];
        assert_eq!(
            coding.system.as_deref(),
            Some("http://hl7.org/fhir/sid/icd-10")
        );
        assert_eq!(
            ProcedureConverter::convert_coding_system("I10P"),
            "http://hl7.org/fhir/sid/icd-10-pcs"
        );
    }
}
