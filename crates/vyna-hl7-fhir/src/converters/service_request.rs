//! ServiceRequest converter - ORC/OBR segments to FHIR ServiceRequest resource

use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::service_request::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming ORC/OBR segments to FHIR ServiceRequest resources
pub struct ServiceRequestConverter;

impl ServiceRequestConverter {
    /// Convert all ORC segments in message to ServiceRequest resources
    pub fn convert_all(message: &Message) -> ConversionResult<Vec<ServiceRequest>> {
        let orc_segments: Vec<_> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "ORC")
            .collect();

        if orc_segments.is_empty() {
            return Err(ConversionError::MissingSegment("ORC".to_string()));
        }

        let mut service_requests = Vec::new();
        for (orc_count, _) in orc_segments.iter().enumerate() {
            let sr = Self::convert_single(message, orc_count)?;
            service_requests.push(sr);
        }

        Ok(service_requests)
    }

    /// Convert a specific ORC segment to ServiceRequest resource
    ///
    /// # Arguments
    /// * `message` - HL7 message containing ORC segment
    /// * `orc_index` - 0-based index (0 = first ORC, 1 = second ORC, etc.)
    pub fn convert_single(message: &Message, orc_index: usize) -> ConversionResult<ServiceRequest> {
        let terser = Terser::new(message);

        // Build base path for ORC fields
        // Note: Terser uses 1-based segment indexing, so ORC(1) is the first ORC segment
        let orc_path = |field: &str| {
            if orc_index == 0 {
                format!("ORC-{}", field)
            } else {
                format!("ORC({})-{}", orc_index + 1, field)
            }
        };

        // ORC-5: Order Status -> status
        let status = terser
            .get(&orc_path("5"))
            .ok()
            .flatten()
            .map(Self::convert_status)
            .unwrap_or_else(|| "active".to_string());

        // Intent is typically "order" for ORC segments
        let intent = "order".to_string();

        // PID-3: Patient ID -> subject reference
        let subject = Self::extract_patient_reference(&terser)?;

        let mut service_request = ServiceRequest::new(status, intent, subject);

        // ORC-2: Placer Order Number -> identifier
        if let Ok(Some(placer_id)) = terser.get(&orc_path("2")) {
            let mut identifiers = service_request.identifier.unwrap_or_default();
            identifiers.push(Identifier {
                system: Some("http://hospital.example.org/placer-order-number".to_string()),
                value: Some(placer_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            });
            service_request.identifier = Some(identifiers);
        }

        // ORC-3: Filler Order Number -> identifier
        if let Ok(Some(filler_id)) = terser.get(&orc_path("3")) {
            let mut identifiers = service_request.identifier.unwrap_or_default();
            identifiers.push(Identifier {
                system: Some("http://hospital.example.org/filler-order-number".to_string()),
                value: Some(filler_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            });
            service_request.identifier = Some(identifiers);
        }

        // ORC-4: Placer Group Number -> requisition
        if let Ok(Some(group_id)) = terser.get(&orc_path("4")) {
            service_request.requisition = Some(Identifier {
                system: Some("http://hospital.example.org/requisition".to_string()),
                value: Some(group_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            });
        }

        // PV1-19: Visit Number -> encounter reference
        if let Ok(Some(visit_num)) = terser.get("PV1-19") {
            service_request.encounter = Some(Reference {
                reference: Some(format!("Encounter/{}", visit_num)),
                type_: Some("Encounter".to_string()),
                identifier: None,
                display: None,
            });
        }

        // ORC-15: Order Effective Date/Time -> authoredOn
        // Lenient by design (see crate docs "Invalid datetime handling"):
        // authored-on is ancillary, with ORC-9 as fallback.
        if let Ok(Some(effective_dt)) = terser.get(&orc_path("15")) {
            if let Ok(authored) = Self::convert_datetime(effective_dt) {
                service_request.authored_on = Some(authored);
            }
        } else if let Ok(Some(transaction_dt)) = terser.get(&orc_path("9")) {
            // ORC-9: Date/Time of Transaction (fallback)
            if let Ok(authored) = Self::convert_datetime(transaction_dt) {
                service_request.authored_on = Some(authored);
            }
        }

        // ORC-12: Ordering Provider -> requester
        if let Ok(Some(provider_id)) = terser.get(&orc_path("12")) {
            service_request.requester = Some(Reference {
                reference: Some(format!("Practitioner/{}", provider_id)),
                type_: Some("Practitioner".to_string()),
                identifier: None,
                display: None,
            });
        }

        // Try to find corresponding OBR segment for additional fields
        // OBR segments typically follow ORC segments in order messages
        let obr_path = |field: &str| {
            if orc_index == 0 {
                format!("OBR-{}", field)
            } else {
                format!("OBR({})-{}", orc_index + 1, field)
            }
        };

        // OBR-4: Universal Service Identifier -> code
        if let Ok(Some(service_code)) = terser.get(&obr_path("4")) {
            let mut coding = Coding {
                system: None,
                code: Some(service_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Service text/display
            if let Ok(Some(service_text)) = terser.get(&format!("{}-2", obr_path("4"))) {
                coding.display = Some(service_text.to_string());
            }

            // OBR-4-3: Coding system selects the system URI.
            if let Ok(Some(system)) = terser.get(&format!("{}-3", obr_path("4")))
                && !system.is_empty()
            {
                coding.system = Some(Self::convert_coding_system(system));
            }

            service_request.code = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        }

        // OBR-5: Priority -> priority
        if let Ok(Some(priority_code)) = terser.get(&obr_path("5")) {
            service_request.priority = Some(Self::convert_priority(priority_code));
        } else if let Ok(Some(tq1_priority)) = terser.get("TQ1-9") {
            // TQ1-9: Priority (alternative location)
            service_request.priority = Some(Self::convert_priority(tq1_priority));
        }

        // OBR-13: Relevant Clinical Information -> note
        if let Ok(Some(note_text)) = terser.get(&obr_path("13")) {
            service_request.note = Some(vec![Annotation {
                author_string: None,
                author_reference: None,
                time: None,
                text: note_text.to_string(),
            }]);
        }

        // OBR-15: Specimen Source -> specimen
        if let Ok(Some(specimen_id)) = terser.get(&obr_path("15")) {
            service_request.specimen = Some(vec![Reference {
                reference: Some(format!("Specimen/{}", specimen_id)),
                type_: Some("Specimen".to_string()),
                identifier: None,
                display: None,
            }]);
        }

        // OBR-31: Reason for Study -> reasonCode
        if let Ok(Some(reason_code)) = terser.get(&obr_path("31")) {
            let mut coding = Coding {
                system: Some("http://terminology.hl7.org/CodeSystem/v2-0340".to_string()),
                code: Some(reason_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Reason text
            if let Ok(Some(reason_text)) = terser.get(&format!("{}-2", obr_path("31"))) {
                coding.display = Some(reason_text.to_string());
            }

            service_request.reason_code = Some(vec![CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            }]);
        }

        Ok(service_request)
    }

    /// Convert HL7 v2 ORC-5 order status to FHIR ServiceRequest status
    fn convert_status(code: &str) -> String {
        match code {
            "A" => "active",     // Some, but not all, of the services have been performed
            "CA" => "revoked",   // Canceled
            "CM" => "completed", // Completed
            "DC" => "revoked",   // Discontinued
            "ER" => "entered-in-error",
            "HD" => "on-hold", // On hold
            "IP" => "active",  // In process, unspecified
            "RP" => "active",  // Order has been replaced
            "SC" => "active",  // In process, scheduled
            _ => "active",     // Default to active
        }
        .to_string()
    }

    /// Convert HL7 v2 priority codes to FHIR ServiceRequest priority
    fn convert_priority(code: &str) -> String {
        match code {
            "S" => "stat",
            "A" => "asap",
            "R" => "routine",
            "P" => "routine", // Preoperative (treat as routine)
            "T" => "routine", // Timing critical (treat as routine)
            _ => "routine",
        }
        .to_string()
    }

    /// Convert HL7 coding system (OBR-4-3) to a FHIR system URI.
    fn convert_coding_system(system: &str) -> String {
        match system {
            "LN" | "LNC" => "http://loinc.org".to_string(),
            "SNM" | "SCT" => "http://snomed.info/sct".to_string(),
            "C4" | "CPT" => "http://www.ama-assn.org/go/cpt".to_string(),
            _ => format!("urn:oid:{system}"),
        }
    }

    /// Convert HL7 datetime (YYYYMMDDHHMMSS) to FHIR ISO 8601 format
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
            // Date only
            Ok(format!(
                "{}-{}-{}",
                &datetime[0..4],
                &datetime[4..6],
                &datetime[6..8]
            ))
        } else {
            Err(ConversionError::InvalidFormat(
                "ORC-15/ORC-9".to_string(),
                "ORC".to_string(),
                format!("Invalid datetime format: {}", datetime),
            ))
        }
    }

    /// Extract patient reference from PID segment
    fn extract_patient_reference(terser: &Terser) -> ConversionResult<Reference> {
        let patient_id = terser
            .get("PID-3")
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
    fn test_convert_service_request_basic() {
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW|ORD123456||GRP789|A||||||DOC001|||20240315100000\r\
                   OBR|1|ORD123456||CBC^Complete Blood Count^LN|R||||||||||||||||||||||||||||";

        let message = parse_message(hl7).unwrap();
        let service_requests = ServiceRequestConverter::convert_all(&message).unwrap();

        assert_eq!(service_requests.len(), 1);
        let sr = &service_requests[0];
        assert_eq!(sr.resource_type, "ServiceRequest");
        assert_eq!(sr.status, "active");
        assert_eq!(sr.intent, "order");
        assert_eq!(sr.subject.reference, Some("Patient/12345".to_string()));
    }

    #[test]
    fn test_convert_service_request_with_identifiers() {
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW|PLACER123|FILLER456|GRP789|A\r\
                   OBR|1|PLACER123|FILLER456|CBC^Complete Blood Count^LN";

        let message = parse_message(hl7).unwrap();
        let service_requests = ServiceRequestConverter::convert_all(&message).unwrap();

        assert_eq!(service_requests.len(), 1);
        let sr = &service_requests[0];
        assert!(sr.identifier.is_some());
        let identifiers = sr.identifier.as_ref().unwrap();
        assert_eq!(identifiers.len(), 2);
        assert_eq!(identifiers[0].value, Some("PLACER123".to_string()));
        assert_eq!(identifiers[1].value, Some("FILLER456".to_string()));
    }

    #[test]
    fn test_convert_service_request_with_code_and_priority() {
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW|ORD123|||A||||||DOC001\r\
                   OBR|1|ORD123||85025^Complete Blood Count^LN|S";

        let message = parse_message(hl7).unwrap();
        let service_requests = ServiceRequestConverter::convert_all(&message).unwrap();

        assert_eq!(service_requests.len(), 1);
        let sr = &service_requests[0];
        assert!(sr.code.is_some());
        assert_eq!(sr.priority, Some("stat".to_string()));
    }

    #[test]
    fn test_convert_multiple_service_requests() {
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW|ORD001|||A\r\
                   OBR|1|ORD001||CBC^Complete Blood Count^LN\r\
                   ORC|NW|ORD002|||A\r\
                   OBR|1|ORD002||BMP^Basic Metabolic Panel^LN";

        let message = parse_message(hl7).unwrap();
        let service_requests = ServiceRequestConverter::convert_all(&message).unwrap();

        assert_eq!(service_requests.len(), 2);
        assert_eq!(
            service_requests[0].identifier.as_ref().unwrap()[0].value,
            Some("ORD001".to_string())
        );
        assert_eq!(
            service_requests[1].identifier.as_ref().unwrap()[0].value,
            Some("ORD002".to_string())
        );
    }

    #[test]
    fn test_convert_service_request_status_mapping() {
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|CM|ORD123|||CM";

        let message = parse_message(hl7).unwrap();
        let service_requests = ServiceRequestConverter::convert_all(&message).unwrap();

        assert_eq!(service_requests.len(), 1);
        assert_eq!(service_requests[0].status, "completed");
    }

    #[test]
    fn test_non_ascii_datetime_returns_error_without_panic() {
        assert!(ServiceRequestConverter::convert_datetime("123\u{e9}56789012").is_err());
        assert_eq!(
            ServiceRequestConverter::convert_datetime("20240315100000").unwrap(),
            "2024-03-15T10:00:00"
        );
    }

    #[test]
    fn test_coding_system_mapped_from_obr4_3() {
        // OBR-4-3 selects the coding system; CPT/SNOMED must
        // not be mislabeled with the LOINC system URI.
        let cases = [
            ("85025^CBC^LN", "http://loinc.org"),
            ("123456^Proc^SNM", "http://snomed.info/sct"),
            ("99213^Visit^CPT", "http://www.ama-assn.org/go/cpt"),
        ];
        for (obr4, expected) in cases {
            let hl7 = format!(
                "MSH|^~\\&|CPOE|H|LAB|H|20240315||ORM^O01|M|P|2.5\r\
                 PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                 ORC|NW|ORD123|||A\r\
                 OBR|1|ORD123||{obr4}"
            );
            let message = parse_message(&hl7).unwrap();
            let requests = ServiceRequestConverter::convert_all(&message).unwrap();
            let coding = requests[0].code.as_ref().unwrap().coding.as_ref().unwrap();
            assert_eq!(coding[0].system.as_deref(), Some(expected), "OBR-4 {obr4}");
        }
    }

    #[test]
    fn test_missing_orc_segment() {
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ADT^A01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let result = ServiceRequestConverter::convert_all(&message);

        assert!(result.is_err());
        match result {
            Err(ConversionError::MissingSegment(seg)) => assert_eq!(seg, "ORC"),
            _ => panic!("Expected MissingSegment error"),
        }
    }
}
