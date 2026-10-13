//! Observation reverse converter - FHIR Observation resource to OBX segment
//!
//! Converts FHIR Observation resources back to HL7 v2.x OBX segments.

use crate::error::ConversionResult;
use crate::resources::common::*;
use crate::resources::observation::{Observation, ObservationReferenceRange};
use vyna_hl7_core::segment::Segment;

/// Reverse converter for transforming FHIR Observation resources to OBX segments
pub struct ObservationReverseConverter;

impl ObservationReverseConverter {
    /// Convert a FHIR Observation resource to an HL7 v2 OBX segment
    ///
    /// # Arguments
    ///
    /// * `observation` - The FHIR Observation resource
    /// * `set_id` - The set ID for OBX-1
    ///
    /// # Returns
    ///
    /// An HL7 v2 OBX segment
    pub fn convert(observation: &Observation, set_id: usize) -> ConversionResult<Segment> {
        let mut obx = Segment::new("OBX");

        // OBX-1: Set ID
        let _ = obx.set_field_value(1, set_id.to_string());

        // OBX-2: Value Type
        let value_type = Self::determine_value_type(observation);
        let _ = obx.set_field_value(2, &value_type);

        // OBX-3: Observation Identifier (CE/CWE)
        Self::set_coded_element(&mut obx, 3, &observation.code)?;

        // OBX-5: Observation Value
        Self::set_observation_value(&mut obx, observation, &value_type)?;

        // OBX-6: Units
        if let Some(ref value_quantity) = observation.value_quantity
            && let Some(ref unit) = value_quantity.unit
        {
            let _ = obx.set_field_value(6, unit);
        }

        // OBX-7: Reference Range
        if let Some(ref reference_range) = observation.reference_range
            && let Some(first_range) = reference_range.first()
        {
            let range_text = Self::format_reference_range(first_range);
            if !range_text.is_empty() {
                let _ = obx.set_field_value(7, &range_text);
            }
        }

        // OBX-8: Abnormal Flags
        if let Some(ref interpretation) = observation.interpretation
            && let Some(first_interp) = interpretation.first()
            && let Some(ref coding) = first_interp.coding
            && let Some(first_coding) = coding.first()
            && let Some(ref code) = first_coding.code
        {
            let _ = obx.set_field_value(8, code);
        }

        // OBX-11: Observation Result Status
        let hl7_status = Self::convert_status_to_hl7(&observation.status);
        let _ = obx.set_field_value(11, &hl7_status);

        // OBX-14: Date/Time of Observation
        if let Some(ref effective_dt) = observation.effective_date_time {
            let hl7_datetime = Self::convert_datetime_to_hl7(effective_dt);
            let _ = obx.set_field_value(14, &hl7_datetime);
        }

        Ok(obx)
    }

    /// Determine the HL7 v2 value type from the observation
    fn determine_value_type(observation: &Observation) -> String {
        if observation.value_quantity.is_some() {
            "NM".to_string() // Numeric
        } else if observation.value_string.is_some() {
            "ST".to_string() // String
        } else if observation.value_codeable_concept.is_some() {
            "CE".to_string() // Coded Element
        } else if observation.value_boolean.is_some() {
            "ST".to_string() // Boolean as string
        } else if observation.value_time.is_some() {
            "TM".to_string() // Time
        } else if observation.value_date_time.is_some() {
            // DTM, not DT: the forward converter reads DT as date-only
            // and would drop the time on re-forward.
            "DTM".to_string() // Date/Time
        } else {
            "ST".to_string() // Default to string
        }
    }

    /// Set a coded element (CE/CWE) in a field
    fn set_coded_element(
        obx: &mut Segment,
        field_num: usize,
        concept: &CodeableConcept,
    ) -> ConversionResult<()> {
        if let Some(ref coding) = concept.coding {
            if let Some(first_coding) = coding.first() {
                // CE structure: 1=Identifier, 2=Text, 3=Coding System
                if let Some(ref code) = first_coding.code {
                    let _ = obx.set_component(field_num, 0, 0, code);
                }
                if let Some(ref display) = first_coding.display {
                    let _ = obx.set_component(field_num, 0, 1, display);
                }
                if let Some(ref system) = first_coding.system {
                    // Extract coding system name from URL
                    let system_name = Self::extract_coding_system_name(system);
                    let _ = obx.set_component(field_num, 0, 2, &system_name);
                }
            }
        } else if let Some(ref text) = concept.text {
            let _ = obx.set_field_value(field_num, text);
        }
        Ok(())
    }

    /// Set the observation value in OBX-5
    fn set_observation_value(
        obx: &mut Segment,
        observation: &Observation,
        _value_type: &str,
    ) -> ConversionResult<()> {
        if let Some(ref quantity) = observation.value_quantity {
            if let Some(value) = quantity.value {
                let _ = obx.set_field_value(5, value.to_string());
            }
        } else if let Some(ref string_value) = observation.value_string {
            let _ = obx.set_field_value(5, string_value);
        } else if let Some(ref concept) = observation.value_codeable_concept {
            Self::set_coded_element(obx, 5, concept)?;
        } else if let Some(boolean_value) = observation.value_boolean {
            let _ = obx.set_field_value(5, if boolean_value { "Y" } else { "N" });
        } else if let Some(ref time) = observation.value_time {
            // FHIR `time` carries colons; HL7 TM does not.
            let _ = obx.set_field_value(5, &time.replace(':', ""));
        } else if let Some(ref datetime) = observation.value_date_time {
            let hl7_datetime = Self::convert_datetime_to_hl7(datetime);
            let _ = obx.set_field_value(5, &hl7_datetime);
        }
        Ok(())
    }

    /// Convert FHIR datetime to HL7 v2 datetime.
    ///
    /// A trailing `Z` becomes `+0000`: a bare `Z` is not valid HL7 DTM.
    fn convert_datetime_to_hl7(datetime: &str) -> String {
        let zoned = match datetime.strip_suffix('Z') {
            Some(stem) => format!("{stem}+0000"),
            None => datetime.to_string(),
        };
        zoned.replace(['-', ':', 'T'], "")
    }

    /// Format a reference range as an HL7 string
    fn format_reference_range(range: &ObservationReferenceRange) -> String {
        let mut parts = Vec::new();

        if let Some(ref low) = range.low
            && let Some(value) = low.value
        {
            parts.push(value.to_string());
        }

        if let Some(ref high) = range.high
            && let Some(value) = high.value
        {
            if parts.is_empty() {
                parts.push(String::new());
            }
            parts.push(value.to_string());
        }

        if parts.len() == 2 {
            format!("{}-{}", parts[0], parts[1])
        } else if parts.len() == 1 {
            parts[0].clone()
        } else if let Some(text) = range.text.as_ref().filter(|t| !t.is_empty()) {
            // Text-only ranges round-trip: the forward converter reads
            // OBX-7 into range text.
            text.clone()
        } else {
            String::new()
        }
    }

    /// Convert FHIR observation status to HL7 v2 result status
    fn convert_status_to_hl7(status: &str) -> String {
        match status {
            "registered" => "O",  // Order received
            "preliminary" => "P", // Preliminary
            "final" => "F",       // Final
            "amended" => "C",     // Corrected
            "corrected" => "C",   // Corrected
            "cancelled" => "X",   // Cancelled
            // D, not W: the forward converter reads D as
            // entered-in-error and W as final.
            "entered-in-error" => "D", // Deleted (entered in error)
            _ => "F",                  // Default to final
        }
        .to_string()
    }

    /// Extract a coding system name from a URL
    fn extract_coding_system_name(system: &str) -> String {
        // Common FHIR coding system URL patterns
        if system.contains("loinc") {
            "LN".to_string()
        } else if system.contains("snomed") {
            "SCT".to_string()
        } else if system.contains("icd-10") {
            "I10".to_string()
        } else if system.contains("icd-9") {
            "I9".to_string()
        } else if system.contains("cpt") {
            "C4".to_string()
        } else if system.contains("rxnorm") {
            "RXNORM".to_string()
        } else if system.contains("ucum") {
            "UCUM".to_string()
        } else {
            // Return the last path segment as the system name
            system.split('/').next_back().unwrap_or("L").to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_code() -> CodeableConcept {
        CodeableConcept {
            coding: Some(vec![Coding {
                system: Some("http://loinc.org".to_string()),
                version: None,
                code: Some("2339-0".to_string()),
                display: Some("Glucose".to_string()),
            }]),
            text: Some("Glucose".to_string()),
        }
    }

    fn create_test_observation() -> Observation {
        let mut obs = Observation::new("final".to_string(), create_code());

        obs.value_quantity = Some(Quantity {
            value: Some(100.0),
            unit: Some("mg/dL".to_string()),
            system: Some("http://unitsofmeasure.org".to_string()),
            code: Some("mg/dL".to_string()),
        });

        obs.reference_range = Some(vec![ObservationReferenceRange {
            low: Some(Quantity {
                value: Some(70.0),
                unit: Some("mg/dL".to_string()),
                system: None,
                code: None,
            }),
            high: Some(Quantity {
                value: Some(100.0),
                unit: Some("mg/dL".to_string()),
                system: None,
                code: None,
            }),
            type_: None,
            text: None,
        }]);

        obs
    }

    #[test]
    fn test_convert_observation_to_obx() {
        let observation = create_test_observation();
        let obx = ObservationReverseConverter::convert(&observation, 1).unwrap();

        assert_eq!(obx.id, "OBX");
        assert_eq!(obx.get_field_value(1), Some("1"));
        assert_eq!(obx.get_field_value(2), Some("NM"));
        assert_eq!(obx.get_field_value(11), Some("F"));
    }

    #[test]
    fn test_determine_value_type() {
        let mut obs = Observation::new("final".to_string(), create_code());
        obs.value_quantity = Some(Quantity {
            value: Some(100.0),
            unit: None,
            system: None,
            code: None,
        });
        assert_eq!(
            ObservationReverseConverter::determine_value_type(&obs),
            "NM"
        );

        let mut obs2 = Observation::new("final".to_string(), create_code());
        obs2.value_string = Some("Test".to_string());
        assert_eq!(
            ObservationReverseConverter::determine_value_type(&obs2),
            "ST"
        );
    }

    #[test]
    fn test_convert_status() {
        assert_eq!(
            ObservationReverseConverter::convert_status_to_hl7("final"),
            "F"
        );
        assert_eq!(
            ObservationReverseConverter::convert_status_to_hl7("preliminary"),
            "P"
        );
        assert_eq!(
            ObservationReverseConverter::convert_status_to_hl7("corrected"),
            "C"
        );
    }

    #[test]
    fn test_extract_coding_system() {
        assert_eq!(
            ObservationReverseConverter::extract_coding_system_name("http://loinc.org"),
            "LN"
        );
        assert_eq!(
            ObservationReverseConverter::extract_coding_system_name("http://snomed.info/sct"),
            "SCT"
        );
    }

    #[test]
    fn test_format_reference_range() {
        let range = ObservationReferenceRange {
            low: Some(Quantity {
                value: Some(70.0),
                unit: None,
                system: None,
                code: None,
            }),
            high: Some(Quantity {
                value: Some(100.0),
                unit: None,
                system: None,
                code: None,
            }),
            type_: None,
            text: None,
        };
        assert_eq!(
            ObservationReverseConverter::format_reference_range(&range),
            "70-100"
        );
    }

    fn round_trip(obx: &Segment) -> Observation {
        use crate::converters::observation::ObservationConverter;
        use vyna_hl7_core::delimiters::Delimiters;
        use vyna_hl7_parser::parse_message;

        let hl7 = format!(
            "MSH|^~\\&|A|B|C|D|20240315||ORU^R01|1|P|2.5\r\
             PID|1||1||A^B||19800101|M\r{}",
            obx.encode(&Delimiters::default())
        );
        let message = parse_message(&hl7).unwrap();
        let observations = ObservationConverter::convert_all(&message).unwrap();
        assert_eq!(observations.len(), 1);
        observations.into_iter().next().unwrap()
    }

    #[test]
    fn test_entered_in_error_round_trips() {
        let mut observation = Observation::new("entered-in-error".to_string(), create_code());
        observation.value_string = Some("x".to_string());
        let obx = ObservationReverseConverter::convert(&observation, 1).unwrap();
        // D re-forwards to entered-in-error; W would become final.
        assert_eq!(obx.get_field_value(11), Some("D"));
        assert_eq!(round_trip(&obx).status, "entered-in-error");
    }

    #[test]
    fn test_value_datetime_with_time_round_trips() {
        let mut observation = Observation::new("final".to_string(), create_code());
        observation.value_date_time = Some("2024-03-15T10:00:00".to_string());
        let obx = ObservationReverseConverter::convert(&observation, 1).unwrap();
        assert_eq!(obx.get_field_value(2), Some("DTM"));
        let back = round_trip(&obx);
        assert_eq!(back.value_date_time.as_deref(), Some("2024-03-15T10:00:00"));
    }

    #[test]
    fn test_value_time_writes_tm_type_and_value() {
        let mut observation = Observation::new("final".to_string(), create_code());
        observation.value_time = Some("10:00:00".to_string());
        let obx = ObservationReverseConverter::convert(&observation, 1).unwrap();
        assert_eq!(obx.get_field_value(2), Some("TM"));
        // FHIR colons are stripped: HL7 TM is HHMMSS.
        assert_eq!(obx.get_field_value(5), Some("100000"));
        let back = round_trip(&obx);
        assert_eq!(back.value_time.as_deref(), Some("100000"));
    }

    #[test]
    fn test_datetimes_map_trailing_z_to_offset() {
        let mut observation = Observation::new("final".to_string(), create_code());
        observation.effective_date_time = Some("2024-03-15T10:00:00Z".to_string());
        observation.value_date_time = Some("2024-03-15T10:00:00Z".to_string());
        let obx = ObservationReverseConverter::convert(&observation, 1).unwrap();
        assert_eq!(obx.get_field_value(14), Some("20240315100000+0000"));
        assert_eq!(obx.get_field_value(5), Some("20240315100000+0000"));
    }

    #[test]
    fn test_text_only_reference_range_round_trips() {
        let mut observation = Observation::new("final".to_string(), create_code());
        observation.value_string = Some("x".to_string());
        observation.reference_range = Some(vec![ObservationReferenceRange {
            low: None,
            high: None,
            type_: None,
            text: Some("normal range".to_string()),
        }]);
        let obx = ObservationReverseConverter::convert(&observation, 1).unwrap();
        assert_eq!(obx.get_field_value(7), Some("normal range"));
        let back = round_trip(&obx);
        let ranges = back.reference_range.as_ref().unwrap();
        assert_eq!(ranges[0].text.as_deref(), Some("normal range"));
    }
}
