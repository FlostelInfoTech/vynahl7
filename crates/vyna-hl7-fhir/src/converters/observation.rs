//! Observation converter - OBX segment to FHIR Observation resource
//!
//! Based on HL7 v2-to-FHIR mapping: <https://build.fhir.org/ig/HL7/v2-to-fhir/ConceptMap-segment-obx-to-observation.html>

use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::observation::*;
use crate::resources::common::*;

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
        let code = if let Ok(Some(code_id)) = terser.get(&code_path) {
            let mut coding = Coding {
                system: None,
                version: None,
                code: Some(code_id.to_string()),
                display: None,
            };

            // OBX-3-2: Text (second component, 1-based indexing)
            if let Ok(Some(text)) = terser.get(&format!("{}-2", code_path))
                && !text.is_empty() {
                    coding.display = Some(text.to_string());
                }

            // OBX-3-3: Name of Coding System (third component, 1-based indexing)
            if let Ok(Some(system)) = terser.get(&format!("{}-3", code_path))
                && !system.is_empty() {
                    coding.system = Some(Self::convert_coding_system(system));
                }

            CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
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

        // OBX-1: Set ID
        let set_id_path = if obx_index == 0 {
            "OBX-1".to_string()
        } else {
            format!("OBX({})-1", obx_index + 1)
        };
        if let Ok(Some(set_id)) = terser.get(&set_id_path)
            && !set_id.is_empty() {
                observation.id = Some(format!("obs-{}", set_id));
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
        if let Ok(Some(value)) = terser.get(&value_path)
            && !value.is_empty() {
                Self::set_observation_value(&mut observation, value_type, value, &terser, &value_path)?;
            }

        // OBX-6: Units -> Observation.valueQuantity.unit (if applicable)
        let units_path = if obx_index == 0 {
            "OBX-6".to_string()
        } else {
            format!("OBX({})-6", obx_index + 1)
        };
        if let Ok(Some(units)) = terser.get(&units_path)
            && !units.is_empty()
                && let Some(ref mut quantity) = observation.value_quantity {
                    quantity.unit = Some(units.to_string());
                }

        // OBX-7: Reference Range -> Observation.referenceRange
        let ref_range_path = if obx_index == 0 {
            "OBX-7".to_string()
        } else {
            format!("OBX({})-7", obx_index + 1)
        };
        if let Ok(Some(ref_range)) = terser.get(&ref_range_path)
            && !ref_range.is_empty() {
                observation.reference_range = Some(vec![ObservationReferenceRange {
                    low: None,
                    high: None,
                    type_: None,
                    text: Some(ref_range.to_string()),
                }]);
            }

        // OBX-8: Interpretation Codes -> Observation.interpretation
        let interp_path = if obx_index == 0 {
            "OBX-8".to_string()
        } else {
            format!("OBX({})-8", obx_index + 1)
        };
        if let Ok(Some(interp)) = terser.get(&interp_path)
            && !interp.is_empty() {
                observation.interpretation = Some(vec![CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v3-ObservationInterpretation".to_string()),
                        version: None,
                        code: Some(interp.to_string()),
                        display: None,
                    }]),
                    text: Some(interp.to_string()),
                }]);
            }

        // OBX-14: Date/Time of Observation -> Observation.effectiveDateTime
        let datetime_path = if obx_index == 0 {
            "OBX-14".to_string()
        } else {
            format!("OBX({})-14", obx_index + 1)
        };
        if let Ok(Some(datetime)) = terser.get(&datetime_path)
            && !datetime.is_empty() {
                observation.effective_date_time = Some(Self::convert_datetime(datetime)?);
            }

        // OBX-16: Responsible Observer -> Observation.performer
        let observer_path = if obx_index == 0 {
            "OBX-16".to_string()
        } else {
            format!("OBX({})-16", obx_index + 1)
        };
        if let Ok(Some(observer_id)) = terser.get(&observer_path)
            && !observer_id.is_empty() {
                let mut reference = Reference {
                    reference: Some(format!("Practitioner/{}", observer_id)),
                    type_: Some("Practitioner".to_string()),
                    identifier: None,
                    display: None,
                };

                // Get observer name if available (component 2 = Family Name, 1-based indexing)
                if let Ok(Some(observer_name)) = terser.get(&format!("{}-2", observer_path))
                    && !observer_name.is_empty() {
                        reference.display = Some(observer_name.to_string());
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
                // Numeric
                if let Ok(numeric_value) = value.parse::<f64>() {
                    observation.value_quantity = Some(Quantity {
                        value: Some(numeric_value),
                        unit: None,
                        system: None,
                        code: None,
                    });
                }
            }
            "ST" | "TX" | "FT" => {
                // String, Text, Formatted Text
                observation.value_string = Some(value.to_string());
            }
            "CE" | "CWE" => {
                // Coded Element
                let mut coding = Coding {
                    system: None,
                    version: None,
                    code: Some(value.to_string()),
                    display: None,
                };

                // Component 2: Text (1-based indexing)
                if let Ok(Some(text)) = terser.get(&format!("{}-2", value_path))
                    && !text.is_empty() {
                        coding.display = Some(text.to_string());
                    }

                // Component 3: Coding System (1-based indexing)
                if let Ok(Some(system)) = terser.get(&format!("{}-3", value_path))
                    && !system.is_empty() {
                        coding.system = Some(Self::convert_coding_system(system));
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
                // Time
                observation.value_time = Some(value.to_string());
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
        match code {
            "R" => "registered",
            "P" => "preliminary",
            "F" => "final",
            "C" => "corrected",
            "X" => "cancelled",
            "D" => "entered-in-error",
            "I" => "preliminary",
            "S" => "preliminary",
            _ => "final",
        }.to_string()
    }

    /// Convert HL7 v2 coding system to FHIR system URI
    fn convert_coding_system(system: &str) -> String {
        match system {
            "LN" | "LNC" => "http://loinc.org".to_string(),
            "SNM" | "SCT" => "http://snomed.info/sct".to_string(),
            "ICD9" => "http://hl7.org/fhir/sid/icd-9-cm".to_string(),
            "ICD10" => "http://hl7.org/fhir/sid/icd-10".to_string(),
            "CPT" => "http://www.ama-assn.org/go/cpt".to_string(),
            _ => format!("urn:oid:{}", system),
        }
    }

    /// Convert HL7 v2 date format (YYYYMMDD) to FHIR date format (YYYY-MM-DD)
    fn convert_date(date: &str) -> ConversionResult<String> {
        if date.len() >= 8 {
            Ok(format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
        } else if date.len() >= 4 {
            Ok(date[0..4].to_string())
        } else {
            Err(ConversionError::InvalidFormat(
                "date".to_string(),
                "OBX".to_string(),
                format!("Invalid date format: {}", date),
            ))
        }
    }

    /// Convert HL7 v2 datetime format to FHIR datetime format
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
            Self::convert_date(datetime)
        } else {
            Err(ConversionError::InvalidFormat(
                "datetime".to_string(),
                "OBX".to_string(),
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
        assert_eq!(interp[0].coding.as_ref().unwrap()[0].code, Some("N".to_string()));

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
}
