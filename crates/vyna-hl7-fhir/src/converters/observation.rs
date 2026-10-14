//! Observation converter - OBX segment to FHIR Observation resource
//!
//! Based on HL7 v2-to-FHIR mapping: <https://build.fhir.org/ig/HL7/v2-to-fhir/ConceptMap-segment-obx-to-observation.html>

use super::coding_systems::convert_coding_system;
use super::patient::{
    hl7_fraction_suffix, split_hl7_offset, valid_hl7_date, valid_hl7_hour_min, valid_hl7_time,
    valid_hl7_year_month,
};
use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::observation::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming OBX segments to FHIR Observation resources
pub struct ObservationConverter;

impl ObservationConverter {
    /// Convert all OBX segments in an HL7 v2 message to FHIR Observation resources
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 v2 message containing OBX segments
    ///
    /// # Returns
    ///
    /// A vector of FHIR Observation resources, one for each OBX segment
    ///
    /// # Errors
    ///
    /// Returns an error if no OBX segments are found or required fields are missing
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<Observation>> {
        let obx_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "OBX")
            .collect();

        if obx_segments.is_empty() {
            return Err(ConversionError::MissingSegment("OBX".to_string()));
        }

        let mut observations = Vec::new();

        for (obx_count, (_index, _)) in obx_segments.iter().enumerate() {
            let observation = Self::convert_single(message, obx_count)?;
            observations.push(observation);
        }

        Ok(observations)
    }

    /// Convert a single OBX segment to a FHIR Observation resource
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 v2 message
    /// * `obx_index` - The index of the OBX segment in the message (0-based)
    ///
    /// # Returns
    ///
    /// A FHIR Observation resource
    pub fn convert_single(message: &Message, obx_index: usize) -> ConversionResult<Observation> {
        let terser = Terser::new(message);

        // OBX-3: Observation Identifier -> Observation.code
        // Note: obx_index is 0-based internally, but Terser uses 1-based segment indexing
        let code_path = if obx_index == 0 {
            "OBX-3".to_string()
        } else {
            format!("OBX({})-3", obx_index + 1)
        };
        let code = if let Ok(Some(code_id)) = terser.get(&code_path)
            && !code_id.is_empty()
        {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code_id.to_string()),
                display: None,
            };

            // OBX-3-2: Text (second component, 1-based indexing)
            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
                && !text.is_empty()
            {
                coding.display = Some(text.to_string());
            }

            // OBX-3-3: Name of Coding System (third component, 1-based indexing)
            if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
                && !system.is_empty()
            {
                coding.system = Some(convert_coding_system(system));
            }

            CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            }
        } else if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
            && !text.is_empty()
        {
            // Text-only: empty code with valued text keeps the
            // observation, like OBR-4 and the OBX-5 coded arms.
            CodeableConcept {
                coding: None,
                text: Some(text.to_string()),
            }
        } else {
            return Err(ConversionError::MissingField(
                "OBX-3".to_string(),
                "OBX".to_string(),
            ));
        };

        // OBX-11: Observation Result Status -> Observation.status
        let status_path = if obx_index == 0 {
            "OBX-11".to_string()
        } else {
            format!("OBX({})-11", obx_index + 1)
        };
        let status = if let Ok(Some(status_code)) = terser.get(&status_path) {
            Self::convert_status(status_code)
        } else {
            "final".to_string() // Default status
        };

        let mut observation = Observation::new(status, code);

        // OBX-1: Set ID -> Observation.id via the shared scheme,
        // which disambiguates restarted set ids with the ordinal.
        if let Some(id) = super::observation_id(message, obx_index) {
            observation.id = Some(id);
        }

        // OBX-2: Value Type -> determines which value[x] field to populate
        let value_type_path = if obx_index == 0 {
            "OBX-2".to_string()
        } else {
            format!("OBX({})-2", obx_index + 1)
        };
        let value_type = terser.get(&value_type_path).ok().flatten().unwrap_or("");

        // OBX-5: Observation Value -> Observation.value[x]
        let value_path = if obx_index == 0 {
            "OBX-5".to_string()
        } else {
            format!("OBX({})-5", obx_index + 1)
        };
        if let Ok(Some(value)) = terser.get(&value_path) {
            // A text-only CE/CWE (empty component 1, valued component
            // 2) still converts, like the allergy converter; other
            // types keep the whole-field empty check, and a coded
            // value with both components empty is still absent.
            let text_only_coded = value.is_empty()
                && matches!(value_type, "CE" | "CWE")
                && terser
                    .get(&format!("{value_path}-2"))
                    .ok()
                    .flatten()
                    .is_some_and(|text| !text.is_empty());
            if !value.is_empty() || text_only_coded {
                Self::set_observation_value(
                    &mut observation,
                    value_type,
                    value,
                    &terser,
                    &value_path,
                )?;
            }
        }

        // OBX-6: Units -> Observation.valueQuantity.unit (if applicable)
        let units_path = if obx_index == 0 {
            "OBX-6".to_string()
        } else {
            format!("OBX({})-6", obx_index + 1)
        };
        if let Ok(Some(units)) = terser.get(&units_path)
            && !units.is_empty()
            && let Some(ref mut quantity) = observation.value_quantity
        {
            quantity.unit = Some(units.to_string());
            quantity.code = Some(units.to_string());
        }

        // OBX-7: Reference Range -> Observation.referenceRange
        let ref_range_path = if obx_index == 0 {
            "OBX-7".to_string()
        } else {
            format!("OBX({})-7", obx_index + 1)
        };
        if let Ok(Some(ref_range)) = terser.get(&ref_range_path)
            && !ref_range.is_empty()
        {
            observation.reference_range = Some(vec![ObservationReferenceRange {
                low: None,
                high: None,
                type_: None,
                text: Some(ref_range.to_string()),
            }]);
        }

        // OBX-8: Interpretation Codes -> Observation.interpretation.
        // Every repetition converts: terser paths are 1-based, so
        // the bare base is the first repetition and (n) is the
        // nth (like the encounter participants).
        let interp_base = if obx_index == 0 {
            "OBX-8".to_string()
        } else {
            format!("OBX({})-8", obx_index + 1)
        };
        let mut interpretations = Vec::new();
        let mut rep = 0usize;
        loop {
            let base = if rep == 0 {
                interp_base.clone()
            } else {
                format!("{interp_base}({})", rep + 1)
            };
            // Only absence ends the scan; a present-but-empty
            // repetition is skipped inside.
            match terser.get(&base) {
                Ok(Some(_)) => {}
                _ => break,
            }
            rep += 1;
            if let Ok(Some(interp)) = terser.get(&base)
                && !interp.is_empty()
            {
                // OBX-8-2 feeds display and text; without it the text
                // falls back to the code.
                let interp_text = terser
                    .get(&format!("{base}-2"))
                    .ok()
                    .flatten()
                    .filter(|s| !s.is_empty());
                interpretations.push(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some(
                            "http://terminology.hl7.org/CodeSystem/v3-ObservationInterpretation"
                                .to_string(),
                        ),
                        version: None,
                        code: Some(interp.to_string()),
                        display: interp_text.map(|text| text.to_string()),
                    }]),
                    text: Some(interp_text.unwrap_or(interp).to_string()),
                });
            }
        }
        if !interpretations.is_empty() {
            observation.interpretation = Some(interpretations);
        }

        // OBX-14: Date/Time of Observation -> Observation.effectiveDateTime
        let datetime_path = if obx_index == 0 {
            "OBX-14".to_string()
        } else {
            format!("OBX({})-14", obx_index + 1)
        };
        if let Ok(Some(datetime)) = terser.get(&datetime_path)
            && !datetime.is_empty()
        {
            observation.effective_date_time = Some(Self::convert_datetime(datetime)?);
        }

        // OBX-16: Responsible Observer -> Observation.performer
        let observer_path = if obx_index == 0 {
            "OBX-16".to_string()
        } else {
            format!("OBX({})-16", obx_index + 1)
        };
        if let Ok(Some(observer_id)) = terser.get(&observer_path)
            && !observer_id.is_empty()
        {
            let mut reference = Reference {
                reference: Some(format!("Practitioner/{}", observer_id)),
                type_: Some("Practitioner".to_string()),
                identifier: None,
                display: None,
            };

            // XCN name parts for display, like the encounter
            // participant: prefix (6), given (3) + middle (4),
            // family (2).
            let mut display_parts = Vec::new();
            for component in ["6", "3", "4", "2"] {
                if let Ok(Some(part)) = terser.get(&format!("{observer_path}-{component}"))
                    && !part.is_empty()
                {
                    display_parts.push(part);
                }
            }
            if !display_parts.is_empty() {
                reference.display = Some(display_parts.join(" "));
            }

            observation.performer = Some(vec![reference]);
        }

        Ok(observation)
    }

    /// Set the appropriate value[x] field based on OBX-2 (Value Type)
    fn set_observation_value(
        observation: &mut Observation,
        value_type: &str,
        value: &str,
        terser: &Terser,
        value_path: &str,
    ) -> ConversionResult<()> {
        match value_type {
            "NM" => {
                // Numeric: a present-but-unparseable value is an error,
                // not a silent omission of value[x].
                let numeric_value = value.parse::<f64>().map_err(|_| {
                    // Redacted: never embed the source value (may be PHI).
                    ConversionError::InvalidDataType(
                        "OBX-5".to_string(),
                        "NM numeric".to_string(),
                        "non-numeric value".to_string(),
                    )
                })?;
                observation.value_quantity = Some(Quantity {
                    value: Some(numeric_value),
                    unit: None,
                    system: Some("http://unitsofmeasure.org".to_string()),
                    code: None,
                });
            }
            "ST" | "TX" | "FT" => {
                // String, Text, Formatted Text
                observation.value_string = Some(value.to_string());
            }
            "CE" | "CWE" => {
                // Component 2: Text (1-based indexing)
                let text = terser
                    .get(&format!("{value_path}-2"))
                    .ok()
                    .flatten()
                    .filter(|text| !text.is_empty());
                if value.is_empty() {
                    // Text-only: the gate guarantees text is present.
                    observation.value_codeable_concept = Some(CodeableConcept {
                        coding: None,
                        text: text.map(|text| text.to_string()),
                    });
                    return Ok(());
                }
                // Coded Element
                let mut coding = Coding {
                    system: None,
                    version: None,
                    code: Some(value.to_string()),
                    display: None,
                };

                if let Some(text) = text {
                    coding.display = Some(text.to_string());
                }

                // Component 3: Coding System (1-based indexing)
                if let Ok(Some(system)) = terser.get(&format!("{}-3", value_path))
                    && !system.is_empty()
                {
                    coding.system = Some(convert_coding_system(system));
                }

                observation.value_codeable_concept = Some(CodeableConcept {
                    coding: Some(vec![coding]),
                    text: None,
                });
            }
            "DT" => {
                // Date
                observation.value_date_time = Some(Self::convert_date(value)?);
            }
            "TM" => {
                // Time: a present-but-malformed value is an error,
                // not a silent pass-through into valueTime.
                if let Some(formatted) = Self::convert_time(value) {
                    observation.value_time = Some(formatted);
                } else {
                    return Err(Self::invalid_time());
                }
            }
            "TS" | "DTM" => {
                // Timestamp, DateTime
                observation.value_date_time = Some(Self::convert_datetime(value)?);
            }
            _ => {
                // Default to string for unknown types
                observation.value_string = Some(value.to_string());
            }
        }

        Ok(())
    }

    /// Convert OBX-11 status code to FHIR Observation status
    fn convert_status(code: &str) -> String {
        // IG ConceptMap table-hl70085-to-observation-status adds
        // A -> amended and W -> entered-in-error; the R/I/S
        // arms are existing extensions, left as they are.
        match code {
            "R" => "registered",
            "P" => "preliminary",
            "F" => "final",
            "C" => "corrected",
            "A" => "amended",
            "X" => "cancelled",
            "D" => "entered-in-error",
            "W" => "entered-in-error",
            "I" => "preliminary",
            "S" => "preliminary",
            _ => "final",
        }
        .to_string()
    }

    /// Convert HL7 v2 date format (YYYYMMDD) to FHIR date format (YYYY-MM-DD)
    ///
    /// `YYYYMM` month precision is preserved as `YYYY-MM`.
    fn convert_date(date: &str) -> ConversionResult<String> {
        // A zone suffix cannot apply to a date (FHIR dates carry no
        // zone), so convert the stem alone.
        let (stem, _) = split_hl7_offset(date);
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
        let bytes = stem.as_bytes();
        if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            // Full-date precision requires a real calendar date.
            if valid_hl7_date(&stem[0..8]) {
                Ok(format!("{}-{}-{}", &stem[0..4], &stem[4..6], &stem[6..8]))
            } else {
                Err(Self::invalid_date(date))
            }
        } else if bytes.len() >= 6 && bytes[..6].iter().all(|b| b.is_ascii_digit()) {
            // Month precision, with the same impossible-date policy.
            if valid_hl7_year_month(&stem[0..6]) {
                Ok(format!("{}-{}", &stem[0..4], &stem[4..6]))
            } else {
                Err(Self::invalid_date(date))
            }
        } else if bytes.len() >= 4 && bytes[..4].iter().all(|b| b.is_ascii_digit()) {
            Ok(stem[0..4].to_string())
        } else {
            Err(Self::invalid_date(date))
        }
    }

    /// Convert HL7 v2 datetime format to FHIR datetime format
    ///
    /// `YYYYMMDDHHMM` keeps its minutes (`YYYY-MM-DDTHH:MM`) and a
    /// trailing HL7 zone (`±HHMM`) is preserved as `±HH:MM`.
    fn convert_datetime(datetime: &str) -> ConversionResult<String> {
        let (stem, offset) = split_hl7_offset(datetime);
        let zone = offset.as_deref().unwrap_or("");
        // Byte-slice checks on `[u8]` never split a char boundary, and an
        // all-ASCII-digit prefix makes the `&str` slices below safe.
        let bytes = stem.as_bytes();
        if bytes.len() >= 14 && bytes[..14].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) && valid_hl7_time(&stem[8..14]) {
                let fraction = hl7_fraction_suffix(stem, 14);
                Ok(format!(
                    "{}-{}-{}T{}:{}:{}{}{}",
                    &stem[0..4],
                    &stem[4..6],
                    &stem[6..8],
                    &stem[8..10],
                    &stem[10..12],
                    &stem[12..14],
                    fraction,
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

    /// Redacted bad-date error: field location + reason only, never the value.
    fn invalid_date(_date: &str) -> ConversionError {
        ConversionError::InvalidFormat(
            "date".to_string(),
            "OBX".to_string(),
            "expected YYYYMMDD calendar date, YYYYMM month, or YYYY year".to_string(),
        )
    }

    /// Format an HL7 TM (`HHMMSS[.S...]`) as FHIR `HH:MM:SS[.S...]`.
    ///
    /// `None` when the stem is not a real clock time; a validated
    /// fractional second is preserved, and anything after it is
    /// trailing junk the digit-prefix checks ignore as before.
    fn convert_time(value: &str) -> Option<String> {
        let bytes = value.as_bytes();
        if bytes.len() < 6
            || !bytes[..6].iter().all(|b| b.is_ascii_digit())
            || !valid_hl7_time(&value[0..6])
        {
            return None;
        }
        Some(format!(
            "{}:{}:{}{}",
            &value[0..2],
            &value[2..4],
            &value[4..6],
            hl7_fraction_suffix(value, 6)
        ))
    }

    /// Redacted bad-time error: field location + reason only, never the value.
    fn invalid_time() -> ConversionError {
        ConversionError::InvalidFormat(
            "OBX-5".to_string(),
            "OBX".to_string(),
            "expected HHMMSS time".to_string(),
        )
    }

    /// Redacted bad-datetime error: field location + reason only, never the value.
    fn invalid_datetime(_datetime: &str) -> ConversionError {
        ConversionError::InvalidFormat(
            "datetime".to_string(),
            "OBX".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_numeric_observation() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
                   PID|1||MRN123|||DOE^JOHN||19800101|M\r\
                   OBR|1|12345|67890|PANEL^Complete Blood Count^LN\r\
                   OBX|1|NM|WBC^White Blood Count^LN||7.5|10*3/uL|4.0-11.0|N|||F";

        let message = parse_message(hl7).unwrap();
        let observations = ObservationConverter::convert_all(&message).unwrap();

        assert_eq!(observations.len(), 1);
        let obs = &observations[0];

        assert_eq!(obs.resource_type, "Observation");
        assert_eq!(obs.status, "final");

        // Check code
        let code = obs.code.coding.as_ref().unwrap();
        assert_eq!(code[0].code, Some("WBC".to_string()));
        assert_eq!(code[0].display, Some("White Blood Count".to_string()));

        // Check numeric value
        let quantity = obs.value_quantity.as_ref().unwrap();
        assert_eq!(quantity.value, Some(7.5));
        assert_eq!(quantity.unit, Some("10*3/uL".to_string()));

        // Check interpretation
        let interp = obs.interpretation.as_ref().unwrap();
        assert_eq!(
            interp[0].coding.as_ref().unwrap()[0].code,
            Some("N".to_string())
        );

        // Check reference range
        let ref_range = obs.reference_range.as_ref().unwrap();
        assert_eq!(ref_range[0].text, Some("4.0-11.0".to_string()));
    }

    #[test]
    fn test_convert_string_observation() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
                   OBX|1|ST|NOTE^Clinical Note^LN||Patient is stable|||||||F";

        let message = parse_message(hl7).unwrap();
        let observations = ObservationConverter::convert_all(&message).unwrap();

        assert_eq!(observations.len(), 1);
        let obs = &observations[0];

        assert_eq!(obs.value_string, Some("Patient is stable".to_string()));
    }

    #[test]
    fn test_unparseable_nm_value_returns_error() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
                   OBX|1|NM|WBC^White Blood Count^LN||not-a-number||||||F";

        let message = parse_message(hl7).unwrap();
        let result = ObservationConverter::convert_all(&message);
        assert!(matches!(
            result,
            Err(ConversionError::InvalidDataType(_, _, _))
        ));
    }

    #[test]
    fn test_non_ascii_datetime_returns_error_without_panic() {
        assert!(ObservationConverter::convert_datetime("123\u{e9}56789012").is_err());
        assert!(ObservationConverter::convert_date("12\u{e9}\u{e9}5678").is_err());
        assert_eq!(
            ObservationConverter::convert_datetime("20240315100000").unwrap(),
            "2024-03-15T10:00:00"
        );
    }

    #[test]
    fn test_invalid_datetime_error_redacts_source_value() {
        let err = ObservationConverter::convert_datetime("junkjunk").unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("junkjunk"), "error leaks source: {msg}");
        assert!(msg.contains("OBX"));
    }

    #[test]
    fn test_impossible_calendar_datetime_rejected() {
        assert!(ObservationConverter::convert_date("20240230").is_err());
        assert!(ObservationConverter::convert_datetime("20240230100000").is_err());
        assert!(ObservationConverter::convert_datetime("20240315250000").is_err());

        // OBX-14 is a primary timestamp: an impossible date must fail.
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
                   OBX|1|NM|WBC^White Blood Count^LN||7.5|mg/dL|||||F|||20240230100000";

        let message = parse_message(hl7).unwrap();
        assert!(ObservationConverter::convert_all(&message).is_err());
    }

    #[test]
    fn test_convert_coded_observation() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
                   OBX|1|CE|BLOOD_TYPE^Blood Type^LN||A+^A Positive^LN|||||||F";

        let message = parse_message(hl7).unwrap();
        let observations = ObservationConverter::convert_all(&message).unwrap();

        assert_eq!(observations.len(), 1);
        let obs = &observations[0];

        let value_concept = obs.value_codeable_concept.as_ref().unwrap();
        let coding = value_concept.coding.as_ref().unwrap();
        assert_eq!(coding[0].code, Some("A+".to_string()));
        assert_eq!(coding[0].display, Some("A Positive".to_string()));
        assert_eq!(coding[0].system, Some("http://loinc.org".to_string()));
    }

    #[test]
    fn test_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            ObservationConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            ObservationConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );
    }

    #[test]
    fn test_date_preserves_year_month() {
        assert_eq!(
            ObservationConverter::convert_date("198001").unwrap(),
            "1980-01"
        );
        assert!(ObservationConverter::convert_date("198013").is_err());
    }

    #[test]
    fn test_empty_obx3_returns_missing_field() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\rOBX|1|NM|||7.5";
        let message = parse_message(hl7).unwrap();
        let result = ObservationConverter::convert_single(&message, 0);
        assert!(matches!(result, Err(ConversionError::MissingField(_, _))));
    }

    #[test]
    fn test_tm_garbage_returns_invalid_format() {
        let base = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\rOBX|1|TM|T^Time^LN||";
        let bad = format!("{base}not-a-time|||||||F");
        let message = parse_message(&bad).unwrap();
        let result = ObservationConverter::convert_all(&message);
        assert!(matches!(
            result,
            Err(ConversionError::InvalidFormat(_, _, _))
        ));

        let good = format!("{base}103000|||||||F");
        let message = parse_message(&good).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        assert_eq!(obs.value_time, Some("10:30:00".to_string()));
    }

    #[test]
    fn test_nm_error_redacts_source_value() {
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|NM|WBC^White Blood Count^LN||SECRET-DOSE||||||F";
        let message = parse_message(hl7).unwrap();
        let err = ObservationConverter::convert_all(&message).unwrap_err();
        let msg = err.to_string();
        assert!(!msg.contains("SECRET-DOSE"), "error leaks source: {msg}");
    }

    #[test]
    fn test_tm_formats_hhmmss_with_colons() {
        assert_eq!(
            ObservationConverter::convert_time("103000").as_deref(),
            Some("10:30:00")
        );
        assert_eq!(
            ObservationConverter::convert_time("103000.5").as_deref(),
            Some("10:30:00.5")
        );
        // An impossible clock time is still an error, end to end.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|TM|T^Time^LN||256000|||||||F";
        let message = parse_message(hl7).unwrap();
        assert!(matches!(
            ObservationConverter::convert_all(&message),
            Err(ConversionError::InvalidFormat(_, _, _))
        ));
    }

    #[test]
    fn test_ce_text_only_value_preserved() {
        // Empty component 1 with valued text: text survives, like the
        // allergy converter.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|CE|T^Text^LN||^Positive|||||||F";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        let concept = obs.value_codeable_concept.as_ref().unwrap();
        assert_eq!(concept.coding, None);
        assert_eq!(concept.text.as_deref(), Some("Positive"));

        // Both components empty: still no value[x].
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|CE|T^Text^LN||||||||F";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        assert_eq!(obs.value_codeable_concept, None);
        assert_eq!(obs.value_string, None);
    }

    #[test]
    fn test_convert_datetime_preserves_fractional_seconds() {
        assert_eq!(
            ObservationConverter::convert_datetime("20240315100000.123").unwrap(),
            "2024-03-15T10:00:00.123"
        );
        // End to end through OBX-14.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|NM|WBC^W^LN||7.5|mg/dL|||||F|||20240315100000.123";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        assert_eq!(
            obs.effective_date_time.as_deref(),
            Some("2024-03-15T10:00:00.123")
        );
    }

    #[test]
    fn test_datetime_accepts_year_month_precision() {
        assert_eq!(
            ObservationConverter::convert_datetime("202401").unwrap(),
            "2024-01"
        );
        assert_eq!(
            ObservationConverter::convert_datetime("1980").unwrap(),
            "1980"
        );
    }

    #[test]
    fn test_obx16_performer_display_builds_full_name() {
        // OBX-16 is XCN: display is prefix/given/middle/family,
        // like the encounter participant display.
        for (obx16, expected) in [
            ("123^SMITH^JANE", "JANE SMITH"),
            ("123^SMITH^JANE^A^^DR", "DR JANE A SMITH"),
        ] {
            let hl7 = format!(
                "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                 OBX|1|NM|WBC^W^LN||7.5|mg/dL||N|||F|||20240315100000||{obx16}"
            );
            let message = parse_message(&hl7).unwrap();
            let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
            let performer = &obs.performer.as_ref().unwrap()[0];
            assert_eq!(performer.display.as_deref(), Some(expected), "{obx16}");
        }
    }

    #[test]
    fn test_obx8_interpretation_uses_component_text() {
        // OBX-8-2 feeds display and text; without it the text
        // falls back to the code.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|NM|WBC^W^LN||7.5|mg/dL|4.0-11.0|H^High^HL70078|||F";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        let interp = &obs.interpretation.as_ref().unwrap()[0];
        let coding = &interp.coding.as_ref().unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("H"));
        assert_eq!(coding.display.as_deref(), Some("High"));
        assert_eq!(interp.text.as_deref(), Some("High"));

        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|NM|WBC^W^LN||7.5|mg/dL|4.0-11.0|H|||F";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        let interp = &obs.interpretation.as_ref().unwrap()[0];
        let coding = &interp.coding.as_ref().unwrap()[0];
        assert_eq!(coding.display, None);
        assert_eq!(interp.text.as_deref(), Some("H"));
    }

    #[test]
    fn test_coding_system_uses_shared_table_for_drug_systems() {
        // CVX/NDC/MVX/RXNORM must resolve via the shared table, not
        // the `urn:oid:` fallback, on both code and value.
        for (system, expected) in [
            ("CVX", "http://hl7.org/fhir/sid/cvx"),
            ("NDC", "http://hl7.org/fhir/sid/ndc"),
            ("MVX", "http://hl7.org/fhir/sid/mvx"),
            ("RXNORM", "http://www.nlm.nih.gov/research/umls/rxnorm"),
        ] {
            let hl7 = format!(
                "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                 OBX|1|CE|MED^Med^{system}||MED^Med^{system}|||||||F"
            );
            let message = parse_message(&hl7).unwrap();
            let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
            let code = &obs.code.coding.as_ref().unwrap()[0];
            assert_eq!(code.system.as_deref(), Some(expected), "code {system}");
            let value = obs
                .value_codeable_concept
                .as_ref()
                .unwrap()
                .coding
                .as_ref()
                .unwrap();
            assert_eq!(value[0].system.as_deref(), Some(expected), "value {system}");
        }
    }

    #[test]
    fn test_restarted_obx_set_ids_stay_unique() {
        // OBX-1 restarts per OBR group: the second OBX|1 must not
        // collide with the first (each report links its own in the
        // diagnostic-report tests).
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|M|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBR|1|||24331-1^Lipid Panel^LN\r\
                   OBX|1|NM|2093-3^Cholesterol^LN||195|mg/dL||||F\r\
                   OBR|2|||24342-8^Metabolic Panel^LN\r\
                   OBX|1|NM|2085-9^HDL^LN||55|mg/dL||||F";
        let message = parse_message(hl7).unwrap();
        let observations = ObservationConverter::convert_all(&message).unwrap();
        let ids: Vec<_> = observations.iter().map(|o| o.id.clone().unwrap()).collect();
        assert_eq!(ids, vec!["obs-1-1", "obs-1-2"]);
    }

    #[test]
    fn test_text_only_obx3_preserved_as_text() {
        // Empty code with valued text keeps the observation, like
        // OBR-4 and the OBX-5 coded arms (fully-empty still errors).
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|ST|^Free text code||some value||||||F";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        assert_eq!(obs.code.coding, None);
        assert_eq!(obs.code.text.as_deref(), Some("Free text code"));
        assert_eq!(obs.value_string.as_deref(), Some("some value"));
    }

    #[test]
    fn test_value_quantity_carries_ucum_system_and_code() {
        // Units carry the UCUM system and code, like the
        // immunization dose quantity and the specimen quantity.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|NM|WBC^W^LN||7.5|mg/dL|||||F";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        let quantity = obs.value_quantity.as_ref().unwrap();
        assert_eq!(quantity.value, Some(7.5));
        assert_eq!(quantity.unit.as_deref(), Some("mg/dL"));
        assert_eq!(
            quantity.system.as_deref(),
            Some("http://unitsofmeasure.org")
        );
        assert_eq!(quantity.code.as_deref(), Some("mg/dL"));
    }

    #[test]
    fn test_obx11_maps_aw_per_ig_concept_map() {
        // IG ConceptMap table-hl70085-to-observation-status: A ->
        // amended, W -> entered-in-error. The R/I/S arms are
        // existing extensions and stay as they are.
        let message_for = |status: &str| {
            parse_message(&format!(
                "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                 OBX|1|NM|WBC^W^LN||7.5|mg/dL|||||{status}"
            ))
            .unwrap()
        };
        for (status, expected) in [
            ("A", "amended"),
            ("W", "entered-in-error"),
            ("R", "registered"),
            ("I", "preliminary"),
            ("S", "preliminary"),
            ("F", "final"),
        ] {
            let message = message_for(status);
            let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
            assert_eq!(obs.status, expected, "OBX-11 {status}");
        }
    }

    #[test]
    fn test_obx8_converts_every_repetition() {
        // OBX-8 repeats: every repetition becomes an
        // interpretation, like the encounter participants loop.
        let hl7 = "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
                   OBX|1|NM|WBC^W^LN||7.5|mg/dL||H^High~L^Low|||F";
        let message = parse_message(hl7).unwrap();
        let obs = &ObservationConverter::convert_all(&message).unwrap()[0];
        let interp = obs.interpretation.as_ref().unwrap();
        assert_eq!(interp.len(), 2);
        let codes: Vec<_> = interp
            .iter()
            .map(|c| c.coding.as_ref().unwrap()[0].code.clone().unwrap())
            .collect();
        assert_eq!(codes, vec!["H", "L"]);
        assert_eq!(interp[0].text.as_deref(), Some("High"));
        assert_eq!(interp[1].text.as_deref(), Some("Low"));
    }

    #[test]
    fn test_literal_suffixed_set_id_cannot_collide() {
        // A literal OBX-1 "1-2" would collide with the
        // disambiguated obs-1-2, so when ANY set id repeats
        // every valued id carries its ordinal.
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|M|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBX|1|NM|2093-3^Cholesterol^LN||195|mg/dL|||||F\r\
                   OBX|1|NM|2085-9^HDL^LN||55|mg/dL|||||F\r\
                   OBX|1-2|NM|2089-1^LDL^LN||100|mg/dL|||||F";
        let message = parse_message(hl7).unwrap();
        let observations = ObservationConverter::convert_all(&message).unwrap();
        let ids: Vec<_> = observations.iter().map(|o| o.id.clone().unwrap()).collect();
        assert_eq!(ids, vec!["obs-1-1", "obs-1-2", "obs-1-2-3"]);

        // Unique-only messages keep the bare obs-{set} form.
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ORU^R01|M|P|2.5\r\
                   PID|1||12345|||DOE^JOHN||19800101|M\r\
                   OBX|1|NM|2093-3^Cholesterol^LN||195|mg/dL|||||F\r\
                   OBX|2|NM|2085-9^HDL^LN||55|mg/dL|||||F\r\
                   OBX|3|NM|2089-1^LDL^LN||100|mg/dL|||||F";
        let message = parse_message(hl7).unwrap();
        let observations = ObservationConverter::convert_all(&message).unwrap();
        let ids: Vec<_> = observations.iter().map(|o| o.id.clone().unwrap()).collect();
        assert_eq!(ids, vec!["obs-1", "obs-2", "obs-3"]);
    }
}
