//! Specimen converter - SPM segment to FHIR Specimen resource

use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::specimen::*;
use crate::resources::common::*;

pub struct SpecimenConverter;

impl SpecimenConverter {
    /// Convert all SPM segments in message to Specimen resources
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<Specimen>> {
        let spm_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "SPM")
            .collect();

        if spm_segments.is_empty() {
            return Err(ConversionError::MissingSegment("SPM".to_string()));
        }

        let mut specimens = Vec::new();
        for (spm_count, _) in spm_segments.iter().enumerate() {
            let specimen = Self::convert_single(message, spm_count)?;
            specimens.push(specimen);
        }

        Ok(specimens)
    }

    /// Convert a specific SPM segment to Specimen resource
    ///
    /// # Arguments
    /// * `message` - HL7 message containing SPM segment
    /// * `spm_index` - 0-based index (0 = first SPM, 1 = second SPM, etc.)
    pub fn convert_single(message: &Message, spm_index: usize) -> ConversionResult<Specimen> {
        let terser = Terser::new(message);

        // Build base path for SPM fields
        // Note: Terser uses 1-based segment indexing, so SPM(1) is the first SPM segment
        let spm_path = |field: &str| {
            if spm_index == 0 {
                format!("SPM-{}", field)
            } else {
                format!("SPM({})-{}", spm_index + 1, field)
            }
        };

        // SPM-4: Specimen Type -> type
        let specimen_type = Self::extract_specimen_type(&terser, &spm_path("4"))?;

        // PID-3: Patient ID -> subject reference
        let subject = Self::extract_patient_reference(&terser)?;

        let mut specimen = Specimen::new(specimen_type, subject);

        // SPM-2: Specimen ID -> identifier
        if let Ok(Some(specimen_id)) = terser.get(&spm_path("2")) {
            specimen.identifier = Some(vec![Identifier {
                system: Some("http://hospital.example.org/specimen-id".to_string()),
                value: Some(specimen_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            }]);
        }

        // SPM-30: Accession ID -> accessionIdentifier
        if let Ok(Some(accession_id)) = terser.get(&spm_path("30")) {
            specimen.accession_identifier = Some(Identifier {
                system: Some("http://hospital.example.org/accession".to_string()),
                value: Some(accession_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            });
        }

        // SPM-20: Specimen Availability -> status
        if let Ok(Some(availability)) = terser.get(&spm_path("20")) {
            specimen.status = Some(Self::convert_status(availability));
        }

        // SPM-18: Specimen Received Date/Time -> receivedTime
        if let Ok(Some(received_dt)) = terser.get(&spm_path("18"))
            && let Ok(received_time) = Self::convert_datetime(received_dt) {
                specimen.received_time = Some(received_time);
            }

        // Build collection information
        let mut collection = SpecimenCollection {
            collector: None,
            collected_date_time: None,
            collected_period: None,
            quantity: None,
            method: None,
            body_site: None,
        };
        let mut has_collection = false;

        // SPM-17: Specimen Collection Date/Time -> collection.collectedDateTime
        if let Ok(Some(collected_dt)) = terser.get(&spm_path("17"))
            && let Ok(collected_time) = Self::convert_datetime(collected_dt) {
                collection.collected_date_time = Some(collected_time);
                has_collection = true;
            }

        // SPM-7: Specimen Collection Method -> collection.method
        if let Ok(Some(method_code)) = terser.get(&spm_path("7")) {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0488".to_string()),
                code: Some(method_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Method text
            if let Ok(Some(method_text)) = terser.get(&format!("{}-1", spm_path("7"))) {
                coding.display = Some(method_text.to_string());
            }

            collection.method = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
            has_collection = true;
        }

        // SPM-8: Specimen Source Site -> collection.bodySite
        if let Ok(Some(site_code)) = terser.get(&spm_path("8")) {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0163".to_string()),
                code: Some(site_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Site text
            if let Ok(Some(site_text)) = terser.get(&format!("{}-1", spm_path("8"))) {
                coding.display = Some(site_text.to_string());
            }

            collection.body_site = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
            has_collection = true;
        }

        // SPM-12: Specimen Collection Amount -> collection.quantity
        if let Ok(Some(amount_str)) = terser.get(&spm_path("12"))
            && let Ok(amount) = amount_str.parse::<f64>() {
                let mut quantity = Quantity {
                    value: Some(amount),
                    unit: None,
                    system: Some("http://unitsofmeasure.org".to_string()),
                    code: None,
                };

                // Component 2: Units (SPM-12-2)
                if let Ok(Some(units)) = terser.get(&format!("{}-2", spm_path("12"))) {
                    quantity.unit = Some(units.to_string());
                    quantity.code = Some(units.to_string());
                }

                collection.quantity = Some(quantity);
                has_collection = true;
            }

        if has_collection {
            specimen.collection = Some(collection);
        }

        // SPM-24: Specimen Condition -> condition
        if let Ok(Some(condition_code)) = terser.get(&spm_path("24")) {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0493".to_string()),
                code: Some(condition_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Condition text
            if let Ok(Some(condition_text)) = terser.get(&format!("{}-1", spm_path("24"))) {
                coding.display = Some(condition_text.to_string());
            }

            specimen.condition = Some(vec![CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            }]);
        }

        // SPM-14: Specimen Description -> note
        if let Ok(Some(description)) = terser.get(&spm_path("14")) {
            specimen.note = Some(vec![Annotation {
                author_string: None,
                author_reference: None,
                time: None,
                text: description.to_string(),
            }]);
        }

        // SPM-15: Specimen Handling Code -> container
        if let Ok(Some(container_type)) = terser.get(&spm_path("15")) {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0376".to_string()),
                code: Some(container_type.to_string()),
                display: None,
                version: None,
            };

            if let Ok(Some(container_text)) = terser.get(&format!("{}-1", spm_path("15"))) {
                coding.display = Some(container_text.to_string());
            }

            specimen.container = Some(vec![SpecimenContainer {
                identifier: None,
                description: None,
                type_: Some(CodeableConcept {
                    coding: Some(vec![coding]),
                    text: None,
                }),
                capacity: None,
                specimen_quantity: None,
                additive_codeable_concept: None,
                additive_reference: None,
            }]);
        }

        Ok(specimen)
    }

    /// Convert HL7 v2 SPM-20 availability to FHIR Specimen status
    fn convert_status(code: &str) -> String {
        match code {
            "Y" => "available",
            "N" => "unavailable",
            "U" => "unsatisfactory",
            _ => "available", // Default
        }.to_string()
    }

    /// Convert HL7 datetime (YYYYMMDDHHMMSS) to FHIR ISO 8601 format
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
            // Date only
            Ok(format!(
                "{}-{}-{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8]
            ))
        } else {
            Err(ConversionError::InvalidFormat(
                "SPM-17/SPM-18".to_string(),
                "SPM".to_string(),
                format!("Invalid datetime format: {}", datetime)
            ))
        }
    }

    /// Extract specimen type from SPM-4 field
    fn extract_specimen_type(terser: &Terser, base_path: &str) -> ConversionResult<CodeableConcept> {
        let code = terser.get(base_path)
            .ok()
            .flatten()
            .ok_or_else(|| ConversionError::MissingField(base_path.to_string(), "SPM".to_string()))?;

        let mut coding = Coding {
            system: Some("http://terminology.hl7.org/CodeSystem/v2-0487".to_string()),
            code: Some(code.to_string()),
            display: None,
            version: None,
        };

        // Component 1: Specimen type text/display
        if let Ok(Some(display)) = terser.get(&format!("{}-1", base_path)) {
            coding.display = Some(display.to_string());
        }

        Ok(CodeableConcept {
            coding: Some(vec![coding]),
            text: None,
        })
    }

    /// Extract patient reference from PID segment
    fn extract_patient_reference(terser: &Terser) -> ConversionResult<Reference> {
        let patient_id = terser.get("PID-3")
            .ok()
            .flatten()
            .ok_or_else(|| ConversionError::MissingSegment("PID".to_string()))?;

        Ok(Reference {
            reference: Some(format!("Patient/{}", patient_id)),
            type_: Some("Patient".to_string()),
            identifier: None,
            display: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_specimen_basic() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487";

        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();

        assert_eq!(specimens.len(), 1);
        let spec = &specimens[0];
        assert_eq!(spec.resource_type, "Specimen");
        assert_eq!(spec.subject.reference, Some("Patient/12345".to_string()));
    }

    #[test]
    fn test_convert_specimen_with_collection() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487|||ANP^Venipuncture^HL70488|LA^Left Arm^HL70163||||5.0^mL|||||20240315100000";

        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();

        assert_eq!(specimens.len(), 1);
        let spec = &specimens[0];
        assert!(spec.collection.is_some());
        let collection = spec.collection.as_ref().unwrap();
        assert_eq!(collection.collected_date_time, Some("2024-03-15T10:00:00".to_string()));
        assert!(collection.method.is_some());
        assert!(collection.body_site.is_some());
        assert!(collection.quantity.is_some());
    }

    #[test]
    fn test_convert_specimen_with_status() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487||||||||||||||||Y||||||||||ACC123";

        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();

        assert_eq!(specimens.len(), 1);
        let spec = &specimens[0];
        assert_eq!(spec.status, Some("available".to_string()));
        assert_eq!(spec.accession_identifier.as_ref().unwrap().value, Some("ACC123".to_string()));
    }

    #[test]
    fn test_convert_multiple_specimens() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC001||BLD^Blood^HL70487\r\
                   SPM|2|SPEC002||URN^Urine^HL70487";

        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();

        assert_eq!(specimens.len(), 2);
        assert_eq!(specimens[0].identifier.as_ref().unwrap()[0].value, Some("SPEC001".to_string()));
        assert_eq!(specimens[1].identifier.as_ref().unwrap()[0].value, Some("SPEC002".to_string()));
    }

    #[test]
    fn test_convert_specimen_with_condition() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487|||||||||||||||||||||||||HEM^Hemolyzed^HL70493";

        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();

        assert_eq!(specimens.len(), 1);
        let spec = &specimens[0];
        assert!(spec.condition.is_some());
    }

    #[test]
    fn test_missing_spm_segment() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let result = SpecimenConverter::convert_all(&message);

        assert!(result.is_err());
        match result {
            Err(ConversionError::MissingSegment(seg)) => assert_eq!(seg, "SPM"),
            _ => panic!("Expected MissingSegment error"),
        }
    }
}
