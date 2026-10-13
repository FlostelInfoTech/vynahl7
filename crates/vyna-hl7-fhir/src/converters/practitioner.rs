//! Practitioner converter - PV1 and other segments to FHIR Practitioner resource
//!
//! Based on HL7 v2-to-FHIR mapping for practitioner information

use crate::error::{ConversionError, ConversionResult};
use crate::resources::common::*;
use crate::resources::practitioner::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Converter for transforming HL7 v2 practitioner data to FHIR Practitioner resources
pub struct PractitionerConverter;

impl PractitionerConverter {
    /// Convert practitioner information from PV1-7 (Attending Doctor) to a FHIR Practitioner resource
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 v2 message containing PV1 segment
    ///
    /// # Returns
    ///
    /// A FHIR Practitioner resource representing the attending doctor
    ///
    /// # Errors
    ///
    /// Returns an error if the PV1 segment or required fields are not found
    pub fn convert_attending_doctor(message: &Message) -> ConversionResult<Practitioner> {
        let terser = Terser::new(message);

        // Check if PV1 segment exists
        if !message.segments.iter().any(|s| s.id == "PV1") {
            return Err(ConversionError::MissingSegment("PV1".to_string()));
        }

        Self::convert_xcn_to_practitioner(&terser, "PV1-7")
    }

    /// Convert practitioner information from PV1-8 (Referring Doctor) to a FHIR Practitioner resource
    pub fn convert_referring_doctor(message: &Message) -> ConversionResult<Practitioner> {
        let terser = Terser::new(message);

        if !message.segments.iter().any(|s| s.id == "PV1") {
            return Err(ConversionError::MissingSegment("PV1".to_string()));
        }

        Self::convert_xcn_to_practitioner(&terser, "PV1-8")
    }

    /// Convert practitioner information from PV1-9 (Consulting Doctor) to a FHIR Practitioner resource
    pub fn convert_consulting_doctor(message: &Message) -> ConversionResult<Practitioner> {
        let terser = Terser::new(message);

        if !message.segments.iter().any(|s| s.id == "PV1") {
            return Err(ConversionError::MissingSegment("PV1".to_string()));
        }

        Self::convert_xcn_to_practitioner(&terser, "PV1-9")
    }

    /// Convert practitioner information from OBX-16 (Responsible Observer) to a FHIR Practitioner resource
    /// Note: obx_index is 0-based internally, but Terser uses 1-based segment indexing
    pub fn convert_responsible_observer(
        message: &Message,
        obx_index: usize,
    ) -> ConversionResult<Practitioner> {
        let terser = Terser::new(message);
        let path = if obx_index == 0 {
            "OBX-16".to_string()
        } else {
            format!("OBX({})-16", obx_index + 1)
        };
        Self::convert_xcn_to_practitioner(&terser, &path)
    }

    /// Convert an XCN (Extended Composite ID Number and Name) field to a FHIR Practitioner
    ///
    /// XCN components (HL7 standard):
    /// 1. ID Number
    /// 2. Family Name
    /// 3. Given Name
    /// 4. Second/Middle Name
    /// 5. Suffix
    /// 6. Prefix
    /// 7. Degree
    /// 8. Source Table
    /// 9. Assigning Authority
    /// 10. Name Type Code
    /// 11. Identifier Check Digit
    /// 12. Check Digit Scheme
    /// 13. Identifier Type Code
    ///
    /// Note: Terser uses 1-based component indexing
    fn convert_xcn_to_practitioner(
        terser: &Terser,
        base_path: &str,
    ) -> ConversionResult<Practitioner> {
        let mut practitioner = Practitioner::new();

        // XCN-1: ID Number -> Practitioner.identifier (component 1, 1-based indexing)
        if let Ok(Some(id_number)) = terser.get(base_path)
            && !id_number.is_empty()
        {
            let mut identifier = Identifier {
                use_: Some("official".to_string()),
                type_: None,
                system: None,
                value: Some(id_number.to_string()),
                assigner: None,
            };

            // XCN-9: Assigning Authority (component 9, 1-based indexing)
            if let Ok(Some(authority)) = terser.get(&format!("{}-9", base_path))
                && !authority.is_empty()
            {
                identifier.system = Some(format!("urn:oid:{}", authority));
            }

            // XCN-13: Identifier Type Code (component 13, 1-based indexing)
            if let Ok(Some(id_type)) = terser.get(&format!("{}-13", base_path))
                && !id_type.is_empty()
            {
                identifier.type_ = Some(CodeableConcept {
                    coding: Some(vec![Coding {
                        system: Some("http://terminology.hl7.org/CodeSystem/v2-0203".to_string()),
                        version: None,
                        code: Some(id_type.to_string()),
                        display: None,
                    }]),
                    text: Some(id_type.to_string()),
                });
            }

            practitioner.identifier = Some(vec![identifier]);
            practitioner.id = Some(id_number.to_string());
        }

        // XCN-2 through XCN-6: Name components -> Practitioner.name.
        // Any valued part keeps the name, like the patient converter
        // keeps a given-only XPN.
        let family = terser
            .get(&format!("{}-2", base_path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let given = terser
            .get(&format!("{}-3", base_path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let middle = terser
            .get(&format!("{}-4", base_path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let suffix = terser
            .get(&format!("{}-5", base_path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        let prefix = terser
            .get(&format!("{}-6", base_path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        // XCN-10: Name Type Code (HL7 table 0200) mapped to
        // FHIR HumanName.use; absent keeps the official default.
        let name_type = terser
            .get(&format!("{}-10", base_path))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty());
        if family.is_some()
            || given.is_some()
            || middle.is_some()
            || suffix.is_some()
            || prefix.is_some()
        {
            let mut name = HumanName {
                use_: Some(match name_type {
                    Some("L") => "official".to_string(),
                    Some("M") => "maiden".to_string(),
                    Some("N") => "nickname".to_string(),
                    Some("T") => "temp".to_string(),
                    Some(_) => "usual".to_string(),
                    None => "official".to_string(),
                }),
                text: None,
                family: family.map(|s| s.to_string()),
                given: None,
                prefix: None,
                suffix: None,
            };

            match (given, middle) {
                (Some(given), middle) => {
                    let mut given_names = vec![given.to_string()];
                    if let Some(middle) = middle {
                        given_names.push(middle.to_string());
                    }
                    name.given = Some(given_names);
                }
                (None, Some(middle)) => {
                    name.given = Some(vec![middle.to_string()]);
                }
                (None, None) => {}
            }

            if let Some(suffix) = suffix {
                name.suffix = Some(vec![suffix.to_string()]);
            }

            if let Some(prefix) = prefix {
                name.prefix = Some(vec![prefix.to_string()]);
            }

            // Build display text
            let mut display_parts = Vec::new();
            if let Some(ref prefix_vec) = name.prefix {
                display_parts.push(prefix_vec.join(" "));
            }
            if let Some(ref given_vec) = name.given {
                display_parts.push(given_vec.join(" "));
            }
            if let Some(ref fam) = name.family {
                display_parts.push(fam.clone());
            }
            if let Some(ref suffix_vec) = name.suffix {
                display_parts.push(suffix_vec.join(" "));
            }
            name.text = Some(display_parts.join(" "));

            practitioner.name = Some(vec![name]);
        }

        // XCN-7: Degree -> Practitioner.qualification (component 7, 1-based indexing)
        if let Ok(Some(degree)) = terser.get(&format!("{}-7", base_path))
            && !degree.is_empty()
        {
            let qualification = PractitionerQualification {
                identifier: None,
                code: CodeableConcept {
                    coding: None,
                    text: Some(degree.to_string()),
                },
                period: None,
                issuer: None,
            };

            practitioner.qualification = Some(vec![qualification]);
        }

        practitioner.active = Some(true);

        Ok(practitioner)
    }

    /// Convert practitioner information from ORC-12 (Ordering Provider) to a FHIR Practitioner resource
    pub fn convert_ordering_provider(message: &Message) -> ConversionResult<Practitioner> {
        let terser = Terser::new(message);

        if !message.segments.iter().any(|s| s.id == "ORC") {
            return Err(ConversionError::MissingSegment("ORC".to_string()));
        }

        Self::convert_xcn_to_practitioner(&terser, "ORC-12")
    }

    /// Extract all practitioners from a message (attending, referring, consulting doctors)
    ///
    /// Every repetition of each PV1 doctor field converts, as does
    /// every ORC-12 repetition of every ORC segment, in source order.
    pub fn extract_all_practitioners(message: &Message) -> ConversionResult<Vec<Practitioner>> {
        let terser = Terser::new(message);
        let mut practitioners = Vec::new();

        // Attending, referring, and consulting doctors (PV1-7/8/9).
        for base in ["PV1-7", "PV1-8", "PV1-9"] {
            Self::convert_all_xcn_reps(&terser, base, &mut practitioners);
        }

        // Ordering providers: every ORC segment's ORC-12. The segment
        // count bounds the loop (terser segment indexing is 1-based,
        // so an out-of-range index errors instead of ending a scan).
        let orc_count = message.segments.iter().filter(|s| s.id == "ORC").count();
        for orc in 0..orc_count {
            let base = if orc == 0 {
                "ORC-12".to_string()
            } else {
                format!("ORC({})-12", orc + 1)
            };
            Self::convert_all_xcn_reps(&terser, &base, &mut practitioners);
        }

        if practitioners.is_empty() {
            return Err(ConversionError::MissingSegment("PV1 or ORC".to_string()));
        }

        Ok(practitioners)
    }

    /// Convert every repetition of one XCN field into `out`, skipping
    /// empty ones. Terser paths are 1-based: the bare base is the
    /// first repetition and (n) is the nth; an absent repetition ends
    /// the scan.
    fn convert_all_xcn_reps(terser: &Terser, base: &str, out: &mut Vec<Practitioner>) {
        let mut rep = 0usize;
        loop {
            let path = if rep == 0 {
                base.to_string()
            } else {
                format!("{base}({})", rep + 1)
            };
            match terser.get(&path) {
                Ok(Some(_)) => {}
                _ => break,
            }
            rep += 1;
            if let Ok(practitioner) = Self::convert_xcn_to_practitioner(terser, &path)
                && !Self::is_empty_practitioner(&practitioner)
            {
                out.push(practitioner);
            }
        }
    }

    /// True when a conversion carried no identifying data (the source
    /// field was absent): `active` alone is not a practitioner.
    fn is_empty_practitioner(practitioner: &Practitioner) -> bool {
        practitioner.id.is_none()
            && practitioner.identifier.is_none()
            && practitioner.name.is_none()
            && practitioner.qualification.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_convert_attending_doctor() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123|||DOE^JOHN||19800101|M\r\
                   PV1|1|I|ER^101^1||||1234567890^SMITH^JAMES^^^DR^MD^^^^^^NPI";

        let message = parse_message(hl7).unwrap();
        let practitioner = PractitionerConverter::convert_attending_doctor(&message).unwrap();

        assert_eq!(practitioner.resource_type, "Practitioner");
        assert_eq!(practitioner.id, Some("1234567890".to_string()));
        assert_eq!(practitioner.active, Some(true));

        let names = practitioner.name.unwrap();
        assert_eq!(names[0].family, Some("SMITH".to_string()));
        assert_eq!(names[0].given, Some(vec!["JAMES".to_string()]));
        assert_eq!(names[0].prefix, Some(vec!["DR".to_string()]));

        let quals = practitioner.qualification.unwrap();
        assert_eq!(quals[0].code.text, Some("MD".to_string()));

        let identifiers = practitioner.identifier.unwrap();
        assert_eq!(identifiers[0].value, Some("1234567890".to_string()));
    }

    #[test]
    fn test_convert_referring_doctor() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PV1|1|O||||||0987654321^JOHNSON^EMILY^ROSE^^DR^DO";

        let message = parse_message(hl7).unwrap();
        let practitioner = PractitionerConverter::convert_referring_doctor(&message).unwrap();

        assert_eq!(practitioner.resource_type, "Practitioner");

        let names = practitioner.name.unwrap();
        assert_eq!(names[0].family, Some("JOHNSON".to_string()));
        assert_eq!(
            names[0].given,
            Some(vec!["EMILY".to_string(), "ROSE".to_string()])
        );
        assert_eq!(names[0].prefix, Some(vec!["DR".to_string()]));

        let quals = practitioner.qualification.unwrap();
        assert_eq!(quals[0].code.text, Some("DO".to_string()));
    }

    #[test]
    fn test_extract_all_practitioners() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PV1|1|I|||||||111^DOC1^FIRST|||222^DOC2^SECOND||333^DOC3^THIRD";

        let message = parse_message(hl7).unwrap();
        let practitioners = PractitionerConverter::extract_all_practitioners(&message).unwrap();

        // Should extract attending (PV1-7), referring (PV1-8), and consulting (PV1-9)
        assert!(!practitioners.is_empty());
    }

    #[test]
    fn test_extract_all_skips_absent_doctors() {
        // A PV1 without doctors yields no practitioners at all, not three
        // empty ones.
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123|||DOE^JOHN||19800101|M\r\
                   PV1|1|I";
        let message = parse_message(hl7).unwrap();
        assert!(PractitionerConverter::extract_all_practitioners(&message).is_err());

        // Only the valued field converts.
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PV1|1|I|||||123^SMITH^JAMES";
        let message = parse_message(hl7).unwrap();
        let practitioners = PractitionerConverter::extract_all_practitioners(&message).unwrap();
        assert_eq!(practitioners.len(), 1);
        assert_eq!(practitioners[0].id.as_deref(), Some("123"));
    }

    #[test]
    fn test_given_only_xcn_preserves_name() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123|||DOE^JOHN||19800101|M\r\
                   PV1|1|I|||||123^^JOHN";
        let message = parse_message(hl7).unwrap();
        let practitioner = PractitionerConverter::convert_attending_doctor(&message).unwrap();
        let names = practitioner.name.as_ref().unwrap();
        assert_eq!(names[0].family, None);
        assert_eq!(names[0].given, Some(vec!["JOHN".to_string()]));
    }

    #[test]
    fn test_extract_all_reads_all_orcs_and_repetitions() {
        // PV1-7 has 2 repetitions, ORC-12 of the first ORC has 2
        // repetitions, and a second ORC adds one more: all five
        // practitioners extract in source order.
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ORM^O01|12345|P|2.5\r\
                   PID|1||MRN123|||DOE^JOHN||19800101|M\r\
                   PV1|1|I|||||711^PV7A^G~712^PV7B^G\r\
                   ORC|NW|O1||||||||||111^A1^G1~121^A1B^G1B\r\
                   ORC|NW|O2||||||||||222^A2^G2";
        let message = parse_message(hl7).unwrap();
        let practitioners = PractitionerConverter::extract_all_practitioners(&message).unwrap();
        let ids: Vec<_> = practitioners
            .iter()
            .map(|p| p.id.as_deref().unwrap())
            .collect();
        assert_eq!(ids, vec!["711", "712", "111", "121", "222"]);
    }

    #[test]
    fn test_xcn10_maiden_maps_to_maiden_use() {
        // XCN-10 M (maiden, HL7 table 0200) maps to maiden use.
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123|||DOE^JOHN||19800101|M\r\
                   PV1|1|I|||||123^SMITH^JANE^^^^^^^M";
        let message = parse_message(hl7).unwrap();
        let practitioner = PractitionerConverter::convert_attending_doctor(&message).unwrap();
        let name = &practitioner.name.as_ref().unwrap()[0];
        assert_eq!(name.use_.as_deref(), Some("maiden"));
    }
}
