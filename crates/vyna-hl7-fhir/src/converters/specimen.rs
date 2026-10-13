//! Specimen converter - SPM segment to FHIR Specimen resource

use super::patient::{
    hl7_fraction_suffix, split_hl7_offset, valid_hl7_date, valid_hl7_hour_min,
    valid_hl7_time,
};
use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::specimen::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming SPM segments to FHIR Specimen resources
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

        // SPM-2: Specimen ID -> identifier. Empty is absent: it
        // must not become an identifier with an empty value.
        if let Ok(Some(specimen_id)) = terser.get(&spm_path("2"))
            && !specimen_id.is_empty()
        {
            specimen.identifier = Some(vec![Identifier {
                system: Some("http://hospital.example.org/specimen-id".to_string()),
                value: Some(specimen_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            }]);
        }

        // SPM-30: Accession ID -> accessionIdentifier
        if let Ok(Some(accession_id)) = terser.get(&spm_path("30"))
            && !accession_id.is_empty()
        {
            specimen.accession_identifier = Some(Identifier {
                system: Some("http://hospital.example.org/accession".to_string()),
                value: Some(accession_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            });
        }

        // SPM-20: Specimen Availability -> status. Empty is absent:
        // it leaves status unset instead of defaulting to available.
        if let Ok(Some(availability)) = terser.get(&spm_path("20"))
            && !availability.is_empty()
        {
            specimen.status = Some(Self::convert_status(availability));
        }

        // SPM-18: Specimen Received Date/Time -> receivedTime
        // Lenient by design (see crate docs "Invalid datetime handling").
        if let Ok(Some(received_dt)) = terser.get(&spm_path("18"))
            && let Ok(received_time) = Self::convert_datetime(received_dt)
        {
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
        // Lenient by design (see crate docs "Invalid datetime handling").
        if let Ok(Some(collected_dt)) = terser.get(&spm_path("17"))
            && let Ok(collected_time) = Self::convert_datetime(collected_dt)
        {
            collection.collected_date_time = Some(collected_time);
            has_collection = true;
        }

        // SPM-7: Specimen Collection Method -> collection.method
        if let Ok(Some(method_code)) = terser.get(&spm_path("7"))
            && !method_code.is_empty()
        {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0488".to_string()),
                code: Some(method_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Method text
            if let Ok(Some(method_text)) = terser.get(&format!("{}-2", spm_path("7")))
                && !method_text.is_empty()
            {
                coding.display = Some(method_text.to_string());
            }

            collection.method = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
            has_collection = true;
        }

        // SPM-8: Specimen Source Site -> collection.bodySite
        if let Ok(Some(site_code)) = terser.get(&spm_path("8"))
            && !site_code.is_empty()
        {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0163".to_string()),
                code: Some(site_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Site text
            if let Ok(Some(site_text)) = terser.get(&format!("{}-2", spm_path("8")))
                && !site_text.is_empty()
            {
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
            && !amount_str.is_empty()
        {
            // A present-but-unparseable amount is an error, not a
            // silent omission of the quantity.
            let amount = amount_str.parse::<f64>().map_err(|_| {
                ConversionError::InvalidDataType(
                    "SPM-12".to_string(),
                    "numeric".to_string(),
                    // Redacted: never embed the source value (may be PHI).
                    "unparseable numeric value".to_string(),
                )
            })?;
            let mut quantity = Quantity {
                value: Some(amount),
                unit: None,
                system: Some("http://unitsofmeasure.org".to_string()),
                code: None,
            };

            // Component 2: Units (SPM-12-2)
            if let Ok(Some(units)) = terser.get(&format!("{}-2", spm_path("12")))
                && !units.is_empty()
            {
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
        if let Ok(Some(condition_code)) = terser.get(&spm_path("24"))
            && !condition_code.is_empty()
        {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0493".to_string()),
                code: Some(condition_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Condition text
            if let Ok(Some(condition_text)) = terser.get(&format!("{}-2", spm_path("24")))
                && !condition_text.is_empty()
            {
                coding.display = Some(condition_text.to_string());
            }

            specimen.condition = Some(vec![CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            }]);
        }

        // SPM-14: Specimen Description -> note
        if let Ok(Some(description)) = terser.get(&spm_path("14"))
            && !description.is_empty()
        {
            specimen.note = Some(vec![Annotation {
                author_string: None,
                author_reference: None,
                time: None,
                text: description.to_string(),
            }]);
        }

        // SPM-15: Specimen Handling Code -> container
        if let Ok(Some(container_type)) = terser.get(&spm_path("15"))
            && !container_type.is_empty()
        {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0376".to_string()),
                code: Some(container_type.to_string()),
                display: None,
                version: None,
            };

            if let Ok(Some(container_text)) = terser.get(&format!("{}-2", spm_path("15")))
                && !container_text.is_empty()
            {
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
        }
        .to_string()
    }

    /// Convert HL7 datetime (YYYYMMDDHHMMSS) to FHIR ISO 8601 format
    ///
    /// `YYYYMMDDHHMM` keeps its minutes and a trailing HL7 zone is
    /// preserved as `±HH:MM`. Validated fractional seconds are kept
    /// as `.digits`.
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
            "SPM-17/SPM-18".to_string(),
            "SPM".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
    }

    /// Extract specimen type from SPM-4 field
    fn extract_specimen_type(
        terser: &Terser,
        base_path: &str,
    ) -> ConversionResult<CodeableConcept> {
        // Empty is missing: it must not become a type with an empty code.
        let code = terser
            .get(base_path)
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                ConversionError::MissingField(base_path.to_string(), "SPM".to_string())
            })?;

        let mut coding = Coding {
            system: Some("http://terminology.hl7.org/CodeSystem/v2-0487".to_string()),
            code: Some(code.to_string()),
            display: None,
            version: None,
        };

        // Component 1: Specimen type text/display
        if let Ok(Some(display)) = terser.get(&format!("{}-2", base_path))
            && !display.is_empty()
        {
            coding.display = Some(display.to_string());
        }

        Ok(CodeableConcept {
            coding: Some(vec![coding]),
            text: None,
        })
    }

    /// Extract patient reference from PID segment
    fn extract_patient_reference(terser: &Terser) -> ConversionResult<Reference> {
        // Empty is missing: it must not become a `Patient/` reference.
        let patient_id = terser
            .get("PID-3")
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
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
        assert_eq!(
            collection.collected_date_time,
            Some("2024-03-15T10:00:00".to_string())
        );
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
        assert_eq!(
            spec.accession_identifier.as_ref().unwrap().value,
            Some("ACC123".to_string())
        );
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
        assert_eq!(
            specimens[0].identifier.as_ref().unwrap()[0].value,
            Some("SPEC001".to_string())
        );
        assert_eq!(
            specimens[1].identifier.as_ref().unwrap()[0].value,
            Some("SPEC002".to_string())
        );
    }

    #[test]
    fn test_convert_specimen_with_condition() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487||||||||||||||||||||HEM^Hemolyzed^HL70493";

        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();

        assert_eq!(specimens.len(), 1);
        let spec = &specimens[0];
        assert!(spec.condition.is_some());
        let condition = spec.condition.as_ref().unwrap()[0].coding.as_ref().unwrap();
        assert_eq!(condition[0].code.as_deref(), Some("HEM"));
    }

    #[test]
    fn test_unparseable_amount_returns_error() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487|||ANP^Venipuncture^HL70488\
                   |LA^Left Arm^HL70163||||lots^mL|||||20240315100000";

        let message = parse_message(hl7).unwrap();
        let result = SpecimenConverter::convert_all(&message);
        assert!(matches!(
            result,
            Err(ConversionError::InvalidDataType(_, _, _))
        ));
    }

    #[test]
    fn test_non_ascii_datetime_returns_error_without_panic() {
        assert!(SpecimenConverter::convert_datetime("123\u{e9}56789012").is_err());
        assert_eq!(
            SpecimenConverter::convert_datetime("20240315100000").unwrap(),
            "2024-03-15T10:00:00"
        );
    }

    #[test]
    fn test_impossible_calendar_datetime_dropped() {
        assert!(SpecimenConverter::convert_datetime("20240230100000").is_err());

        // SPM-17/SPM-18 are ancillary: an impossible date is dropped,
        // never emitted, while the resource still converts.
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487|||ANP^Venipuncture^HL70488\
                   |LA^Left Arm^HL70163||||5.0^mL|||||20240230100000";

        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();

        assert_eq!(specimens.len(), 1);
        let collection = specimens[0].collection.as_ref().unwrap();
        assert_eq!(collection.collected_date_time, None);
        assert!(collection.quantity.is_some());
    }

    #[test]
    fn test_empty_spm20_leaves_status_unset() {
        let mut fields = vec![String::new(); 30];
        fields[0] = "1".to_string();
        fields[1] = "SPEC123".to_string();
        fields[3] = "BLD^Blood^HL70487".to_string();
        fields[19] = String::new(); // SPM-20 present-but-empty
        let hl7 = format!(
            "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rSPM|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();
        assert_eq!(specimens[0].status, None);
    }

    #[test]
    fn test_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            SpecimenConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            SpecimenConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );

        let mut fields = vec![String::new(); 30];
        fields[0] = "1".to_string();
        fields[1] = "SPEC123".to_string();
        fields[3] = "BLD^Blood^HL70487".to_string();
        fields[16] = "20240315100000+0500".to_string(); // SPM-17
        let hl7 = format!(
            "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rSPM|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();
        let collection = specimens[0].collection.as_ref().unwrap();
        assert_eq!(
            collection.collected_date_time.as_deref(),
            Some("2024-03-15T10:00:00+05:00")
        );
    }

    #[test]
    fn test_empty_pid3_returns_missing_segment() {
        // PID-3 present but its first component empty: no patient id.
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487";
        let message = parse_message(hl7).unwrap();
        let result = SpecimenConverter::convert_all(&message);
        assert!(matches!(result, Err(ConversionError::MissingSegment(_))));
    }

    #[test]
    fn test_empty_spm2_leaves_identifier_unset() {
        // SPM-2 present-but-empty; SPM-4 forces its presence.
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|||BLD^Blood^HL70487";
        let message = parse_message(hl7).unwrap();
        let specimens = SpecimenConverter::convert_all(&message).unwrap();
        assert_eq!(specimens[0].identifier, None);
    }

    #[test]
    fn test_unparseable_amount_error_redacts_source_value() {
        let hl7 = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   SPM|1|SPEC123||BLD^Blood^HL70487|||ANP^Venipuncture^HL70488\
                   |LA^Left Arm^HL70163||||SECRET-DOSE^mL|||||20240315100000";
        let message = parse_message(hl7).unwrap();
        let err = SpecimenConverter::convert_all(&message).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("SPM-12"), "keeps field location: {msg}");
        assert!(!msg.contains("SECRET-DOSE"), "error leaks source: {msg}");
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

    #[test]
    fn test_empty_spm_fields_omitted() {
        // Present-but-empty SPM-7/8/12-units/14/15/24/30 must not
        // emit empty codes, units, text, or values.
        let mut fields = vec![String::new(); 30];
        fields[0] = "1".to_string();
        fields[1] = "SPEC123".to_string();
        fields[3] = "BLD^Blood^HL70487".to_string();
        fields[11] = "5.0^".to_string();
        let hl7 = format!(
            "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rSPM|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let spec = &SpecimenConverter::convert_all(&message).unwrap()[0];
        let collection = spec.collection.as_ref().unwrap();
        assert!(collection.method.is_none());
        assert!(collection.body_site.is_none());
        let quantity = collection.quantity.as_ref().unwrap();
        assert_eq!(quantity.value, Some(5.0));
        assert!(quantity.unit.is_none());
        assert!(quantity.code.is_none());
        assert!(spec.note.is_none());
        assert!(spec.container.is_none());
        assert!(spec.condition.is_none());
        assert!(spec.accession_identifier.is_none());
    }

    #[test]
    fn test_empty_spm_component_text_omitted() {
        // Fields with a code but empty text keep the code only.
        let mut fields = vec![String::new(); 30];
        fields[0] = "1".to_string();
        fields[1] = "SPEC123".to_string();
        fields[3] = "BLD^^HL70487".to_string();
        fields[6] = "ANP^".to_string();
        fields[7] = "LA^".to_string();
        fields[14] = "X^".to_string();
        fields[23] = "Y^".to_string();
        let hl7 = format!(
            "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rSPM|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let spec = &SpecimenConverter::convert_all(&message).unwrap()[0];
        let type_coding = spec.type_.coding.as_ref().unwrap();
        assert_eq!(type_coding[0].code.as_deref(), Some("BLD"));
        assert!(type_coding[0].display.is_none());
        let collection = spec.collection.as_ref().unwrap();
        let method = collection.method.as_ref().unwrap().coding.as_ref().unwrap();
        assert!(method[0].display.is_none());
        let site = collection
            .body_site
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap();
        assert!(site[0].display.is_none());
        let container = spec.container.as_ref().unwrap()[0]
            .type_
            .as_ref()
            .unwrap()
            .coding
            .as_ref()
            .unwrap();
        assert!(container[0].display.is_none());
        let condition = spec.condition.as_ref().unwrap()[0].coding.as_ref().unwrap();
        assert!(condition[0].display.is_none());
    }

    #[test]
    fn test_empty_spm4_returns_missing_field() {
        // SPM-4 present-but-empty is missing, like an absent SPM-4.
        let mut fields = vec![String::new(); 30];
        fields[0] = "1".to_string();
        fields[1] = "SPEC123".to_string();
        fields[29] = "ACC123".to_string();
        let hl7 = format!(
            "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315||OML^O21|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rSPM|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let result = SpecimenConverter::convert_all(&message);
        assert!(matches!(result, Err(ConversionError::MissingField(_, _))));
    }

    #[test]
    fn test_datetime_preserves_fractional_seconds() {
        assert_eq!(
            SpecimenConverter::convert_datetime("20240315100000.123").unwrap(),
            "2024-03-15T10:00:00.123"
        );
    }
}
