//! ServiceRequest converter - ORC/OBR segments to FHIR ServiceRequest resource

use super::coding_systems::convert_coding_system;
use super::patient::{
    hl7_fraction_suffix, split_hl7_offset, valid_hl7_date, valid_hl7_hour_min, valid_hl7_time,
    valid_hl7_year_month,
};
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

        // ORC-2: Placer Order Number -> identifier. Empty is absent:
        // it must not become an identifier with an empty value.
        if let Ok(Some(placer_id)) = terser.get(&orc_path("2"))
            && !placer_id.is_empty()
        {
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

        // ORC-3: Filler Order Number -> identifier (same empty rule).
        if let Ok(Some(filler_id)) = terser.get(&orc_path("3"))
            && !filler_id.is_empty()
        {
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

        // ORC-4: Placer Group Number -> requisition (same empty rule).
        if let Ok(Some(group_id)) = terser.get(&orc_path("4"))
            && !group_id.is_empty()
        {
            service_request.requisition = Some(Identifier {
                system: Some("http://hospital.example.org/requisition".to_string()),
                value: Some(group_id.to_string()),
                type_: None,
                use_: None,
                assigner: None,
            });
        }

        // PV1-19: Visit Number -> encounter reference. Empty is
        // absent: it must not become an `Encounter/` reference.
        if let Ok(Some(visit_num)) = terser.get("PV1-19")
            && !visit_num.is_empty()
        {
            service_request.encounter = Some(Reference {
                reference: Some(format!("Encounter/{}", visit_num)),
                type_: Some("Encounter".to_string()),
                identifier: None,
                display: None,
            });
        }

        // ORC-15: Order Effective Date/Time -> authoredOn, with ORC-9 fallback.
        // Lenient by design (see crate docs "Invalid datetime handling"):
        // authored-on is ancillary, so a present-but-unparseable ORC-15
        // falls through to ORC-9 instead of dropping the timestamp.
        let authored_on = terser
            .get(&orc_path("15"))
            .ok()
            .flatten()
            .and_then(|dt| Self::convert_datetime(dt).ok())
            .or_else(|| {
                // ORC-9: Date/Time of Transaction (fallback)
                terser
                    .get(&orc_path("9"))
                    .ok()
                    .flatten()
                    .and_then(|dt| Self::convert_datetime(dt).ok())
            });
        if let Some(authored) = authored_on {
            service_request.authored_on = Some(authored);
        }

        // ORC-12: Ordering Provider -> requester. Empty is absent:
        // it must not become a `Practitioner/` reference. The display
        // joins the XCN name parts, like the encounter participant.
        let orc12 = orc_path("12");
        if let Ok(Some(provider_id)) = terser.get(&orc12)
            && !provider_id.is_empty()
        {
            service_request.requester = Some(Reference {
                reference: Some(format!("Practitioner/{}", provider_id)),
                type_: Some("Practitioner".to_string()),
                identifier: None,
                display: Self::xcn_display(&terser, &orc12),
            });
        }

        // The OBR for this order is the first one at/after its ORC in
        // message order, up to the next ORC (mirrors
        // DiagnosticReportConverter result linking). Index pairing
        // misattributes fields when an order lacks its OBR.
        let obr_paired = Self::paired_index(message, orc_index, "OBR");
        let obr_path = |field: &str| match obr_paired {
            Some(0) => format!("OBR-{field}"),
            Some(i) => format!("OBR({})-{field}", i + 1),
            // No OBR in this order group: index 0 is rejected by the
            // terser (1-based), so every OBR read below yields nothing.
            None => format!("OBR(0)-{field}"),
        };

        // OBR-4: Universal Service Identifier -> code. Empty is
        // absent: it must not become a coding with an empty code,
        // but a text-only CWE keeps its text like the allergy
        // converter.
        if let Ok(Some(service_code)) = terser.get(&obr_path("4"))
            && !service_code.is_empty()
        {
            let mut coding = Coding {
                system: None,
                code: Some(service_code.to_string()),
                display: None,
                version: None,
            };

            // Component 1: Service text/display (empty is absent).
            if let Ok(Some(service_text)) = terser.get(&format!("{}-2", obr_path("4")))
                && !service_text.is_empty()
            {
                coding.display = Some(service_text.to_string());
            }

            // OBR-4-3: Coding system selects the system URI.
            if let Ok(Some(system)) = terser.get(&format!("{}-3", obr_path("4")))
                && !system.is_empty()
            {
                coding.system = Some(convert_coding_system(system));
            }

            service_request.code = Some(CodeableConcept {
                coding: Some(vec![coding]),
                text: None,
            });
        } else if let Ok(Some(service_text)) = terser.get(&format!("{}-2", obr_path("4")))
            && !service_text.is_empty()
        {
            service_request.code = Some(CodeableConcept {
                coding: None,
                text: Some(service_text.to_string()),
            });
        }

        // OBR-5: Priority -> priority
        if let Ok(Some(priority_code)) = terser.get(&obr_path("5"))
            && !priority_code.is_empty()
        {
            service_request.priority = Some(Self::convert_priority(priority_code));
        } else {
            // TQ1-9: Priority (alternative location), paired with this
            // order's group like the OBR above.
            let tq1_path = match Self::paired_index(message, orc_index, "TQ1") {
                Some(0) => "TQ1-9".to_string(),
                Some(i) => format!("TQ1({})-9", i + 1),
                None => "TQ1(0)-9".to_string(),
            };
            if let Ok(Some(tq1_priority)) = terser.get(&tq1_path)
                && !tq1_priority.is_empty()
            {
                service_request.priority = Some(Self::convert_priority(tq1_priority));
            }
        }

        // OBR-13: Relevant Clinical Information -> note. Empty is
        // absent: it must not become a note with empty text.
        if let Ok(Some(note_text)) = terser.get(&obr_path("13"))
            && !note_text.is_empty()
        {
            service_request.note = Some(vec![Annotation {
                author_string: None,
                author_reference: None,
                time: None,
                text: note_text.to_string(),
            }]);
        }

        // OBR-15: Specimen Source -> specimen. Empty is absent:
        // it must not become a `Specimen/` reference.
        if let Ok(Some(specimen_id)) = terser.get(&obr_path("15"))
            && !specimen_id.is_empty()
        {
            service_request.specimen = Some(vec![Reference {
                reference: Some(format!("Specimen/{}", specimen_id)),
                type_: Some("Specimen".to_string()),
                identifier: None,
                display: None,
            }]);
        }

        // OBR-31: Reason for Study -> reasonCode. Empty is
        // absent: it must not become a coding with an empty code,
        // but a text-only CWE keeps its text like the allergy
        // converter. Every repetition converts: terser paths are
        // 1-based, so the bare base is the first repetition and
        // (n) is the nth (like the encounter participants).
        let reason_base = obr_path("31");
        let mut reasons = Vec::new();
        let mut rep = 0usize;
        loop {
            let base = if rep == 0 {
                reason_base.clone()
            } else {
                format!("{reason_base}({})", rep + 1)
            };
            // Only absence ends the scan; a present-but-empty
            // repetition is skipped inside.
            match terser.get(&base) {
                Ok(Some(_)) => {}
                _ => break,
            }
            rep += 1;
            if let Ok(Some(reason_code)) = terser.get(&base)
                && !reason_code.is_empty()
            {
                let mut coding = Coding {
                    system: None,
                    code: Some(reason_code.to_string()),
                    display: None,
                    version: None,
                };

                // Component 1: Reason text (empty is absent).
                if let Ok(Some(reason_text)) = terser.get(&format!("{base}-2"))
                    && !reason_text.is_empty()
                {
                    coding.display = Some(reason_text.to_string());
                }

                // OBR-31-3 selects the system URI, like OBR-4-3.
                if let Ok(Some(system)) = terser.get(&format!("{base}-3"))
                    && !system.is_empty()
                {
                    coding.system = Some(convert_coding_system(system));
                }

                reasons.push(CodeableConcept {
                    coding: Some(vec![coding]),
                    text: None,
                });
            } else if let Ok(Some(reason_text)) = terser.get(&format!("{base}-2"))
                && !reason_text.is_empty()
            {
                reasons.push(CodeableConcept {
                    coding: None,
                    text: Some(reason_text.to_string()),
                });
            }
        }
        if !reasons.is_empty() {
            service_request.reason_code = Some(reasons);
        }

        Ok(service_request)
    }

    /// 0-based terser index of the first `segment` at/after the
    /// `orc_index`-th ORC in message order, up to (excluding) the next
    /// ORC; `None` when the order group has none.
    fn paired_index(message: &Message, orc_index: usize, segment: &str) -> Option<usize> {
        let orc_positions: Vec<usize> = message
            .segments
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id == "ORC")
            .map(|(i, _)| i)
            .collect();
        let &orc_pos = orc_positions.get(orc_index)?;
        let end = message.segments[orc_pos + 1..]
            .iter()
            .position(|s| s.id == "ORC")
            .map(|p| orc_pos + 1 + p)
            .unwrap_or(message.segments.len());
        let rel = message.segments[orc_pos + 1..end]
            .iter()
            .position(|s| s.id == segment)?;
        let pos = orc_pos + 1 + rel;
        Some(
            message.segments[..pos]
                .iter()
                .filter(|s| s.id == segment)
                .count(),
        )
    }

    /// Build an XCN display name: prefix (6), given (3), middle
    /// (4), family (2), joined with spaces; `None` when all are empty.
    fn xcn_display(terser: &Terser, base_path: &str) -> Option<String> {
        let mut parts = Vec::new();
        for component in [6, 3, 4, 2] {
            if let Ok(Some(part)) = terser.get(&format!("{base_path}-{component}"))
                && !part.is_empty()
            {
                parts.push(part.to_string());
            }
        }
        if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        }
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

    /// `YYYYMMDD` -> `YYYY-MM-DD` (`YYYYMM` -> `YYYY-MM`,
    /// `YYYY` stays a year).
    fn convert_date(date: &str) -> ConversionResult<String> {
        let (stem, _) = split_hl7_offset(date);
        let bytes = stem.as_bytes();
        if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_date(&stem[0..8]) {
                Ok(format!("{}-{}-{}", &stem[0..4], &stem[4..6], &stem[6..8]))
            } else {
                Err(Self::invalid_datetime_format())
            }
        } else if bytes.len() >= 6 && bytes[..6].iter().all(|b| b.is_ascii_digit()) {
            if valid_hl7_year_month(&stem[0..6]) {
                Ok(format!("{}-{}", &stem[0..4], &stem[4..6]))
            } else {
                Err(Self::invalid_datetime_format())
            }
        } else if bytes.len() >= 4 && bytes[..4].iter().all(|b| b.is_ascii_digit()) {
            Ok(stem[0..4].to_string())
        } else {
            Err(Self::invalid_datetime_format())
        }
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
                Err(Self::invalid_datetime_format())
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
                Err(Self::invalid_datetime_format())
            }
        } else if bytes.len() >= 4 {
            // Date, month, or year precision, like the patient
            // converter.
            Self::convert_date(stem)
        } else {
            Err(Self::invalid_datetime_format())
        }
    }


    /// Redacted bad-datetime error: field location + reason only, never the value.
    fn invalid_datetime_format() -> ConversionError {
        ConversionError::InvalidFormat(
            "ORC-15/ORC-9".to_string(),
            "ORC".to_string(),
            "expected YYYYMMDDHHMMSS calendar datetime or YYYYMMDD date".to_string(),
        )
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
    fn test_empty_pid3_returns_missing_segment() {
        // PID-3 present but its first component empty: no patient id.
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW|ORD123456||GRP789|A";
        let message = parse_message(hl7).unwrap();
        let result = ServiceRequestConverter::convert_all(&message);
        assert!(matches!(result, Err(ConversionError::MissingSegment(_))));
    }

    #[test]
    fn test_empty_orc12_obr13_obr15_treated_as_absent() {
        // ORC-12 present-but-empty; ORC-15 forces its presence.
        let mut orc = vec![String::new(); 15];
        orc[0] = "NW".to_string();
        orc[1] = "ORD123".to_string();
        orc[4] = "A".to_string();
        orc[14] = "20240315100000".to_string();
        // OBR-13/OBR-15 present-but-empty; OBR-16 forces presence.
        let mut obr = vec![String::new(); 16];
        obr[0] = "1".to_string();
        obr[1] = "ORD123".to_string();
        obr[3] = "CBC^Complete Blood Count^LN".to_string();
        obr[4] = "R".to_string();
        obr[15] = "X".to_string();
        let hl7 = format!(
            "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rORC|{}\rOBR|{}",
            orc.join("|"),
            obr.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(requests[0].requester, None);
        assert_eq!(requests[0].note, None);
        assert_eq!(requests[0].specimen, None);
        // Valued siblings still convert.
        assert!(requests[0].code.is_some());
        assert_eq!(
            requests[0].authored_on.as_deref(),
            Some("2024-03-15T10:00:00")
        );
    }

    #[test]
    fn test_empty_obr4_obr31_pv19_treated_as_absent() {
        // OBR-4/OBR-31 present-but-empty; OBR-5/OBR-32 force presence.
        let mut obr = vec![String::new(); 32];
        obr[0] = "1".to_string();
        obr[1] = "ORD123".to_string();
        obr[4] = "R".to_string();
        obr[31] = "X".to_string();
        // PV1-19 present-but-empty; PV1-20 forces presence.
        let mut pv1 = vec![String::new(); 20];
        pv1[0] = "1".to_string();
        pv1[19] = "X".to_string();
        let hl7 = format!(
            "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rPV1|{}\rORC|NW|ORD123|||A\rOBR|{}",
            pv1.join("|"),
            obr.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(requests[0].code, None);
        assert_eq!(requests[0].reason_code, None);
        assert_eq!(requests[0].encounter, None);
        assert_eq!(requests[0].priority.as_deref(), Some("routine"));
    }

    #[test]
    fn test_empty_display_components_treated_as_absent() {
        // OBR-4 with empty text (component 2) leaves display unset.
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW|O1|||A\r\
                   OBR|1|O1||CBC^^LN";
        let message = parse_message(hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        let coding = &requests[0].code.as_ref().unwrap().coding.as_ref().unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("CBC"));
        assert_eq!(coding.display, None);

        // Same rule for OBR-31 (reason) component 2.
        let mut obr = vec![String::new(); 31];
        obr[0] = "1".to_string();
        obr[1] = "O1".to_string();
        obr[3] = "CBC^Complete Blood Count^LN".to_string();
        obr[30] = "R1^^LN".to_string();
        let hl7 = format!(
            "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rORC|NW|O1|||A\rOBR|{}",
            obr.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        let coding = &requests[0].reason_code.as_ref().unwrap()[0]
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("R1"));
        assert_eq!(coding.display, None);
    }

    #[test]
    fn test_requester_display_built_from_xcn_name() {
        // ORC-12 DOC001^SMITH^JANE: display joins prefix/given/middle/family.
        for (orc12, expected_ref, expected) in [
            ("DOC001^SMITH^JANE", "Practitioner/DOC001", "JANE SMITH"),
            (
                "DOC002^SMITH^JANE^M^^DR",
                "Practitioner/DOC002",
                "DR JANE M SMITH",
            ),
        ] {
            let mut orc = vec![String::new(); 12];
            orc[0] = "NW".to_string();
            orc[1] = "ORD123".to_string();
            orc[4] = "A".to_string();
            orc[11] = orc12.to_string();
            let hl7 = format!(
                "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                 PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rORC|{}",
                orc.join("|")
            );
            let message = parse_message(&hl7).unwrap();
            let requests = ServiceRequestConverter::convert_all(&message).unwrap();
            let requester = requests[0].requester.as_ref().unwrap();
            assert_eq!(requester.reference.as_deref(), Some(expected_ref));
            assert_eq!(
                requester.display.as_deref(),
                Some(expected),
                "ORC-12 {orc12}"
            );
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

    fn orc_message_with_dates(orc9: &str, orc15: &str) -> String {
        let mut fields = vec![String::new(); 15];
        fields[0] = "NW".to_string();
        fields[1] = "ORD123".to_string();
        fields[4] = "A".to_string();
        fields[8] = orc9.to_string();
        fields[14] = orc15.to_string();
        format!(
            "MSH|^~\\&|CPOE|H|LAB|H|20240315||ORM^O01|M|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
             ORC|{}",
            fields.join("|")
        )
    }

    #[test]
    fn test_authored_on_falls_back_to_orc9_when_orc15_invalid() {
        let hl7 = orc_message_with_dates("20240315100000", "not-a-datetime");
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(
            requests[0].authored_on.as_deref(),
            Some("2024-03-15T10:00:00")
        );
    }

    #[test]
    fn test_authored_on_prefers_valid_orc15() {
        let hl7 = orc_message_with_dates("20240101000000", "20240315100000");
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(
            requests[0].authored_on.as_deref(),
            Some("2024-03-15T10:00:00")
        );
    }

    #[test]
    fn test_impossible_calendar_datetimes_rejected() {
        assert!(ServiceRequestConverter::convert_datetime("20241345100000").is_err());
        assert!(ServiceRequestConverter::convert_datetime("20240230").is_err());
        assert!(ServiceRequestConverter::convert_datetime("20240315250000").is_err());
        assert_eq!(
            ServiceRequestConverter::convert_datetime("20240315100000").unwrap(),
            "2024-03-15T10:00:00"
        );
        assert_eq!(
            ServiceRequestConverter::convert_datetime("20240229").unwrap(),
            "2024-02-29"
        );
    }

    #[test]
    fn test_invalid_datetime_error_redacts_source_value() {
        let err = ServiceRequestConverter::convert_datetime("sensitive-timestamp").unwrap_err();
        let msg = err.to_string();
        assert!(
            !msg.contains("sensitive-timestamp"),
            "error leaks source: {msg}"
        );
        assert!(msg.contains("ORC-15/ORC-9"));
    }

    #[test]
    fn test_empty_orc_identifiers_treated_as_absent() {
        // ORC-2/3/4 present-but-empty leave identifier/requisition unset.
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW||||A";
        let message = parse_message(hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(requests[0].identifier, None);
        assert_eq!(requests[0].requisition, None);
    }

    #[test]
    fn test_tq1_priority_paired_with_orc_index() {
        // No OBR-5 anywhere, so TQ1-9 decides; each order must read its
        // own TQ1, and an empty TQ1-9 leaves priority unset.
        let tq1 = |priority: &str| {
            let mut fields = vec![String::new(); 9];
            fields[0] = "1".to_string();
            fields[8] = priority.to_string();
            format!("TQ1|{}", fields.join("|"))
        };
        let hl7 = format!(
            "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
             ORC|NW|O1|||A\rOBR|1|O1||CBC^CBC^LN\r{}\r\
             ORC|NW|O2|||A\rOBR|1|O2||BMP^BMP^LN\r{}\r\
             ORC|NW|O3|||A\rOBR|1|O3||GLU^GLU^LN\r{}",
            tq1("S"),
            tq1("R"),
            tq1("")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].priority.as_deref(), Some("stat"));
        assert_eq!(requests[1].priority.as_deref(), Some("routine"));
        assert_eq!(requests[2].priority, None);
    }

    #[test]
    fn test_datetime_preserves_minutes_and_zone() {
        assert_eq!(
            ServiceRequestConverter::convert_datetime("202403151000").unwrap(),
            "2024-03-15T10:00"
        );
        assert_eq!(
            ServiceRequestConverter::convert_datetime("20240315100000+0500").unwrap(),
            "2024-03-15T10:00:00+05:00"
        );

        let mut fields = vec![String::new(); 15];
        fields[0] = "NW".to_string();
        fields[1] = "ORD123".to_string();
        fields[4] = "A".to_string();
        fields[14] = "20240315100000+0500".to_string();
        let hl7 = format!(
            "MSH|^~\\&|CPOE|H|LAB|H|20240315||ORM^O01|M|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rORC|{}",
            fields.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(
            requests[0].authored_on.as_deref(),
            Some("2024-03-15T10:00:00+05:00")
        );
    }

    #[test]
    fn test_coding_system_maps_icd_via_shared_table() {
        // OBR-4-3 resolves via the shared table: ICD9/ICD10 must
        // not fall back to `urn:oid:`.
        let cases = [
            ("I10^HTN^ICD10", "http://hl7.org/fhir/sid/icd-10"),
            ("410^MI^ICD9", "http://hl7.org/fhir/sid/icd-9-cm"),
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
            let coding = &requests[0].code.as_ref().unwrap().coding.as_ref().unwrap()[0];
            assert_eq!(coding.system.as_deref(), Some(expected), "OBR-4 {obr4}");
        }
    }

    #[test]
    fn test_datetime_preserves_fractional_seconds() {
        assert_eq!(
            ServiceRequestConverter::convert_datetime("20240315100000.123").unwrap(),
            "2024-03-15T10:00:00.123"
        );
    }

    #[test]
    fn test_text_only_obr4_code_preserved() {
        let hl7 = "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
                   PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
                   ORC|NW|O1|||A\r\
                   OBR|1|O1||^Free text order";
        let message = parse_message(hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        let code = requests[0].code.as_ref().unwrap();
        assert_eq!(code.coding, None);
        assert_eq!(code.text.as_deref(), Some("Free text order"));
    }

    #[test]
    fn test_text_only_obr31_reason_preserved() {
        let mut obr = vec![String::new(); 31];
        obr[0] = "1".to_string();
        obr[1] = "O1".to_string();
        obr[3] = "CBC^Complete Blood Count^LN".to_string();
        obr[30] = "^Free text reason".to_string();
        let hl7 = format!(
            "MSH|^~\\&|CPOE|Hospital|LAB|Hospital|20240315||ORM^O01|MSG123|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rORC|NW|O1|||A\rOBR|{}",
            obr.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        let reason = &requests[0].reason_code.as_ref().unwrap()[0];
        assert_eq!(reason.coding, None);
        assert_eq!(reason.text.as_deref(), Some("Free text reason"));
    }

    #[test]
    fn test_datetime_accepts_year_month_precision() {
        assert_eq!(
            ServiceRequestConverter::convert_datetime("202401").unwrap(),
            "2024-01"
        );
        assert_eq!(
            ServiceRequestConverter::convert_datetime("1980").unwrap(),
            "1980"
        );
    }

    #[test]
    fn test_obr_and_tq1_pair_by_order_group_not_index() {
        // O2 has no OBR/TQ1: it must not inherit O3's GLU code and
        // priority, and O3 must keep its own.
        let tq1 = |priority: &str| {
            let mut fields = vec![String::new(); 9];
            fields[0] = "1".to_string();
            fields[8] = priority.to_string();
            format!("TQ1|{}", fields.join("|"))
        };
        let hl7 = format!(
            "MSH|^~\\&|CPOE|H|LAB|H|20240315||ORM^O01|M|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\r\
             ORC|NW|O1|||A\rOBR|1|O1||CBC^Complete Blood Count^LN\r{}\r\
             ORC|NW|O2|||A\r\
             ORC|NW|O3|||A\rOBR|1|O3||GLU^Glucose^LN\r{}",
            tq1("S"),
            tq1("R")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        assert_eq!(requests.len(), 3);
        let code = |i: usize| {
            requests[i].code.as_ref().unwrap().coding.as_ref().unwrap()[0]
                .code
                .clone()
        };
        assert_eq!(code(0).as_deref(), Some("CBC"));
        assert_eq!(requests[0].priority.as_deref(), Some("stat"));
        assert_eq!(requests[1].code, None);
        assert_eq!(requests[1].priority, None);
        assert_eq!(code(2).as_deref(), Some("GLU"));
        assert_eq!(requests[2].priority.as_deref(), Some("routine"));
    }

    #[test]
    fn test_reason_system_reads_obr31_3_via_shared_table() {
        // OBR-31-3 selects the reason system like OBR-4-3 does for code.
        let mut obr = vec![String::new(); 31];
        obr[0] = "1".to_string();
        obr[1] = "O1".to_string();
        obr[3] = "CBC^Complete Blood Count^LN".to_string();
        obr[30] = "I10^Hypertension^ICD10".to_string();
        let hl7 = format!(
            "MSH|^~\\&|CPOE|H|LAB|H|20240315||ORM^O01|M|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rORC|NW|O1|||A\rOBR|{}",
            obr.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        let coding = &requests[0].reason_code.as_ref().unwrap()[0]
            .coding
            .as_ref()
            .unwrap()[0];
        assert_eq!(coding.code.as_deref(), Some("I10"));
        assert_eq!(coding.display.as_deref(), Some("Hypertension"));
        assert_eq!(
            coding.system.as_deref(),
            Some("http://hl7.org/fhir/sid/icd-10")
        );
    }

    #[test]
    fn test_obr31_converts_every_repetition() {
        // OBR-31 repeats: every repetition becomes a
        // reasonCode, like the encounter participants loop.
        let mut obr = vec![String::new(); 31];
        obr[0] = "1".to_string();
        obr[1] = "O1".to_string();
        obr[3] = "CBC^Complete Blood Count^LN".to_string();
        obr[30] = "R1^Reason one^LN~R2^Reason two^LN".to_string();
        let hl7 = format!(
            "MSH|^~\\&|CPOE|H|LAB|H|20240315||ORM^O01|M|P|2.5\r\
             PID|1||12345^^^MRN||DOE^JOHN||19800101|M\rORC|NW|O1|||A\rOBR|{}",
            obr.join("|")
        );
        let message = parse_message(&hl7).unwrap();
        let requests = ServiceRequestConverter::convert_all(&message).unwrap();
        let reasons = requests[0].reason_code.as_ref().unwrap();
        assert_eq!(reasons.len(), 2);
        let codes: Vec<_> = reasons
            .iter()
            .map(|r| r.coding.as_ref().unwrap()[0].code.clone().unwrap())
            .collect();
        assert_eq!(codes, vec!["R1", "R2"]);
    }
}
