//! Practitioner reverse converter - FHIR Practitioner resource to XCN data type
//!
//! Converts FHIR Practitioner resources back to HL7 v2.x XCN (Extended Composite ID
//! Number and Name for Persons) data type, used in fields like PV1-7, PV1-8, OBR-16, etc.

use crate::error::ConversionResult;
use crate::resources::practitioner::Practitioner;

/// Reverse converter for transforming FHIR Practitioner resources to XCN format
///
/// The XCN data type is used in many HL7 v2 segments to represent healthcare providers:
/// - PV1-7: Attending Doctor
/// - PV1-8: Referring Doctor
/// - PV1-9: Consulting Doctor
/// - OBR-16: Ordering Provider
/// - ORC-12: Ordering Provider
pub struct PractitionerReverseConverter;

impl PractitionerReverseConverter {
    /// Convert a FHIR Practitioner resource to an XCN-formatted string
    ///
    /// XCN Structure (simplified):
    /// - Component 1: ID Number
    /// - Component 2: Family Name
    /// - Component 3: Given Name
    /// - Component 4: Second Name
    /// - Component 5: Suffix
    /// - Component 6: Prefix
    /// - Component 9: Assigning Authority
    /// - Component 10: Name Type Code
    /// - Component 13: Identifier Type Code
    ///
    /// # Arguments
    ///
    /// * `practitioner` - The FHIR Practitioner resource
    ///
    /// # Returns
    ///
    /// An XCN-formatted string (components separated by ^)
    pub fn convert_to_xcn(practitioner: &Practitioner) -> ConversionResult<String> {
        let mut components: Vec<String> = vec![String::new(); 15];

        // Component 1: ID Number
        if let Some(ref identifiers) = practitioner.identifier
            && let Some(first_id) = identifiers.first()
        {
            if let Some(ref value) = first_id.value {
                components[0] = value.clone();
            }

            // Component 9: Assigning Authority
            if let Some(ref system) = first_id.system {
                // Extract authority from system URL
                let authority = system
                    .strip_prefix("urn:oid:")
                    .unwrap_or(system.split('/').next_back().unwrap_or(""));
                components[8] = authority.to_string();
            }

            // Component 13: Identifier Type Code
            if let Some(ref type_) = first_id.type_
                && let Some(ref coding) = type_.coding
                && let Some(first_coding) = coding.first()
                && let Some(ref code) = first_coding.code
            {
                components[12] = code.clone();
            }
        }

        // Components 2-6: Name parts
        if let Some(ref names) = practitioner.name
            && let Some(first_name) = names.first()
        {
            // Component 2: Family Name
            if let Some(ref family) = first_name.family {
                components[1] = family.clone();
            }

            // Component 3: Given Name
            if let Some(ref given) = first_name.given {
                if let Some(first_given) = given.first() {
                    components[2] = first_given.clone();
                }
                // Component 4: Second Name (Middle)
                if given.len() > 1 {
                    components[3] = given[1].clone();
                }
            }

            // Component 5: Suffix
            if let Some(ref suffix) = first_name.suffix
                && let Some(first_suffix) = suffix.first()
            {
                components[4] = first_suffix.clone();
            }

            // Component 6: Prefix
            if let Some(ref prefix) = first_name.prefix
                && let Some(first_prefix) = prefix.first()
            {
                components[5] = first_prefix.clone();
            }

            // Component 10: Name Type Code
            if let Some(ref use_) = first_name.use_ {
                components[9] = match use_.as_str() {
                    "official" => "L".to_string(),
                    "nickname" => "N".to_string(),
                    _ => "L".to_string(),
                };
            }
        }

        // Component 7: Degree, from the first qualification's code
        // text (where the forward converter puts XCN-7).
        if let Some(ref qualifications) = practitioner.qualification
            && let Some(first) = qualifications.first()
            && let Some(ref text) = first.code.text
            && !text.is_empty()
        {
            components[6] = text.clone();
        }

        // Build the XCN string, trimming trailing empty components
        let mut result = components.join("^");
        while result.ends_with('^') {
            result.pop();
        }

        Ok(result)
    }

    /// Create individual XCN components for use in segment fields
    ///
    /// Returns a vector of (component_index, value) tuples for setting in a segment
    pub fn convert_to_components(
        practitioner: &Practitioner,
    ) -> ConversionResult<Vec<(usize, String)>> {
        let mut components = Vec::new();

        // Component 1: ID Number (0-indexed: 0)
        if let Some(ref identifiers) = practitioner.identifier
            && let Some(first_id) = identifiers.first()
        {
            if let Some(ref value) = first_id.value {
                components.push((0, value.clone()));
            }

            // Component 9: Assigning Authority (0-indexed: 8)
            if let Some(ref system) = first_id.system {
                let authority = system
                    .strip_prefix("urn:oid:")
                    .unwrap_or(system.split('/').next_back().unwrap_or(""));
                if !authority.is_empty() {
                    components.push((8, authority.to_string()));
                }
            }

            // Component 13: Identifier Type Code (0-indexed: 12)
            if let Some(ref type_) = first_id.type_
                && let Some(ref coding) = type_.coding
                && let Some(first_coding) = coding.first()
                && let Some(ref code) = first_coding.code
            {
                components.push((12, code.clone()));
            }
        }

        // Name components
        if let Some(ref names) = practitioner.name
            && let Some(first_name) = names.first()
        {
            // Component 2: Family Name (0-indexed: 1)
            if let Some(ref family) = first_name.family {
                components.push((1, family.clone()));
            }

            // Component 3: Given Name (0-indexed: 2)
            if let Some(ref given) = first_name.given {
                if let Some(first_given) = given.first() {
                    components.push((2, first_given.clone()));
                }
                // Component 4: Second Name (0-indexed: 3)
                if given.len() > 1 {
                    components.push((3, given[1].clone()));
                }
            }

            // Component 5: Suffix (0-indexed: 4)
            if let Some(ref suffix) = first_name.suffix
                && let Some(first_suffix) = suffix.first()
            {
                components.push((4, first_suffix.clone()));
            }

            // Component 6: Prefix (0-indexed: 5)
            if let Some(ref prefix) = first_name.prefix
                && let Some(first_prefix) = prefix.first()
            {
                components.push((5, first_prefix.clone()));
            }

            // Component 10: Name Type Code (0-indexed: 9)
            if let Some(ref use_) = first_name.use_ {
                let type_code = match use_.as_str() {
                    "official" => "L",
                    "nickname" => "N",
                    _ => "L",
                };
                components.push((9, type_code.to_string()));
            }
        }

        // Component 7: Degree (0-indexed: 6), from the first
        // qualification's code text (where the forward converter
        // puts XCN-7).
        if let Some(ref qualifications) = practitioner.qualification
            && let Some(first) = qualifications.first()
            && let Some(ref text) = first.code.text
            && !text.is_empty()
        {
            components.push((6, text.clone()));
        }

        Ok(components)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::common::*;

    fn create_test_practitioner() -> Practitioner {
        let mut practitioner = Practitioner::new();

        practitioner.identifier = Some(vec![Identifier {
            use_: Some("official".to_string()),
            type_: Some(CodeableConcept {
                coding: Some(vec![Coding {
                    system: None,
                    version: None,
                    code: Some("NPI".to_string()),
                    display: Some("National Provider Identifier".to_string()),
                }]),
                text: None,
            }),
            system: Some("urn:oid:2.16.840.1.113883.4.6".to_string()),
            value: Some("1234567890".to_string()),
            assigner: None,
        }]);

        practitioner.name = Some(vec![HumanName {
            use_: Some("official".to_string()),
            text: None,
            family: Some("Smith".to_string()),
            given: Some(vec!["John".to_string(), "A".to_string()]),
            prefix: Some(vec!["Dr".to_string()]),
            suffix: Some(vec!["MD".to_string()]),
        }]);

        practitioner
    }

    #[test]
    fn test_convert_to_xcn() {
        let practitioner = create_test_practitioner();
        let xcn = PractitionerReverseConverter::convert_to_xcn(&practitioner).unwrap();

        assert!(xcn.contains("1234567890"));
        assert!(xcn.contains("Smith"));
        assert!(xcn.contains("John"));
        assert!(xcn.contains("MD"));
        assert!(xcn.contains("Dr"));
    }

    #[test]
    fn test_convert_to_components() {
        let practitioner = create_test_practitioner();
        let components =
            PractitionerReverseConverter::convert_to_components(&practitioner).unwrap();

        // Check that we have the expected components
        let id_component = components.iter().find(|(idx, _)| *idx == 0);
        assert!(id_component.is_some());
        assert_eq!(id_component.unwrap().1, "1234567890");

        let family_component = components.iter().find(|(idx, _)| *idx == 1);
        assert!(family_component.is_some());
        assert_eq!(family_component.unwrap().1, "Smith");

        let given_component = components.iter().find(|(idx, _)| *idx == 2);
        assert!(given_component.is_some());
        assert_eq!(given_component.unwrap().1, "John");
    }

    #[test]
    fn test_qualification_round_trips_to_xcn7() {
        use crate::converters::practitioner::PractitionerConverter;
        use vyna_hl7_parser::parse_message;

        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123|||DOE^JOHN||19800101|M\r\
                   PV1|1|I|ER^101^1||||1234567890^SMITH^JAMES^^^DR^MD";
        let message = parse_message(hl7).unwrap();
        let practitioner = PractitionerConverter::convert_attending_doctor(&message).unwrap();
        let xcn = PractitionerReverseConverter::convert_to_xcn(&practitioner).unwrap();
        assert_eq!(xcn.split('^').nth(6), Some("MD"));
        let components =
            PractitionerReverseConverter::convert_to_components(&practitioner).unwrap();
        assert!(components.contains(&(6, "MD".to_string())));
    }

    #[test]
    fn test_minimal_practitioner() {
        let mut practitioner = Practitioner::new();
        practitioner.name = Some(vec![HumanName {
            use_: None,
            text: None,
            family: Some("Doe".to_string()),
            given: None,
            prefix: None,
            suffix: None,
        }]);

        let xcn = PractitionerReverseConverter::convert_to_xcn(&practitioner).unwrap();
        assert!(xcn.contains("Doe"));
    }
}
