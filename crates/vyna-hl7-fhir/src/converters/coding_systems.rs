//! Shared HL7 v2 coding-system to FHIR system-URI mapping.
//!
//! CWE-coded fields (RXA-5, PRB-3, DG1-3, ...) carry the coding
//! system in component 3. This table maps those identifiers to
//! FHIR system URIs so every converter resolves them identically.

/// Convert an HL7 v2 coding-system identifier to a FHIR system URI.
///
/// Unknown systems keep the historical `urn:oid:` fallback.
pub fn convert_coding_system(system: &str) -> String {
    convert_coding_system_opt(system).unwrap_or_else(|| format!("urn:oid:{system}"))
}

/// Convert a coding-system identifier, or `None` when unknown.
///
/// Table-bound CWE fields use this so an unknown component 3 keeps the
/// bound-table default instead of the `urn:oid:` fallback.
pub fn convert_coding_system_opt(system: &str) -> Option<String> {
    match system {
        "CVX" => Some("http://hl7.org/fhir/sid/cvx".to_string()),
        "NDC" => Some("http://hl7.org/fhir/sid/ndc".to_string()),
        "MVX" => Some("http://hl7.org/fhir/sid/mvx".to_string()),
        "RXNORM" => Some("http://www.nlm.nih.gov/research/umls/rxnorm".to_string()),
        "LN" | "LNC" => Some("http://loinc.org".to_string()),
        "SNM" | "SCT" => Some("http://snomed.info/sct".to_string()),
        "ICD9" | "I9" | "I9C" => Some("http://hl7.org/fhir/sid/icd-9-cm".to_string()),
        "ICD10" | "I10" => Some("http://hl7.org/fhir/sid/icd-10".to_string()),
        "I10P" => Some("http://hl7.org/fhir/sid/icd-10-pcs".to_string()),
        "CPT" | "C4" => Some("http://www.ama-assn.org/go/cpt".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_systems_and_fallback() {
        assert_eq!(convert_coding_system("CVX"), "http://hl7.org/fhir/sid/cvx");
        assert_eq!(convert_coding_system("NDC"), "http://hl7.org/fhir/sid/ndc");
        assert_eq!(convert_coding_system("MVX"), "http://hl7.org/fhir/sid/mvx");
        assert_eq!(
            convert_coding_system("RXNORM"),
            "http://www.nlm.nih.gov/research/umls/rxnorm"
        );
        assert_eq!(convert_coding_system("LN"), "http://loinc.org");
        assert_eq!(convert_coding_system("LNC"), "http://loinc.org");
        assert_eq!(convert_coding_system("SCT"), "http://snomed.info/sct");
        assert_eq!(
            convert_coding_system("ICD10"),
            "http://hl7.org/fhir/sid/icd-10"
        );
        assert_eq!(convert_coding_system("UNKNOWN"), "urn:oid:UNKNOWN");
    }

    #[test]
    fn test_hl7_0396_short_forms() {
        // HL7 table 0396 short forms resolve like the long forms
        // procedure.rs already maps (DG1 `I10`, OBR `C4`, ...).
        assert_eq!(
            convert_coding_system("I9"),
            "http://hl7.org/fhir/sid/icd-9-cm"
        );
        assert_eq!(
            convert_coding_system("I9C"),
            "http://hl7.org/fhir/sid/icd-9-cm"
        );
        assert_eq!(
            convert_coding_system("I10"),
            "http://hl7.org/fhir/sid/icd-10"
        );
        assert_eq!(
            convert_coding_system("I10P"),
            "http://hl7.org/fhir/sid/icd-10-pcs"
        );
        assert_eq!(
            convert_coding_system("C4"),
            "http://www.ama-assn.org/go/cpt"
        );
    }

    #[test]
    fn test_opt_returns_none_for_unknown_systems() {
        // Table-bound fields use the Option form so an unknown
        // component 3 keeps the bound-table default, not `urn:oid:`.
        assert_eq!(
            convert_coding_system_opt("SCT"),
            Some("http://snomed.info/sct".to_string())
        );
        assert_eq!(convert_coding_system_opt("XYZ"), None);
        assert_eq!(convert_coding_system_opt("HL70487"), None);
    }

    #[test]
    fn test_short_forms_keep_converter_mapping_parity() {
        // The shipped YAML maps mirror the shared table, so the
        // declarative side agrees with the converters on short forms.
        use crate::converters::allergy_intolerance::AllergyIntoleranceConverter;
        use crate::converters::condition::ConditionConverter;
        use crate::mapping::mappings;
        use crate::mapping::{Mapping, MappingEngine};
        use vyna_hl7_parser::parse_message;

        let message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||PPR^PC1|1|P|2.5\rPID|1||7\rPRB|AD|20240101|I10^HTN^I10|P1",
        )
        .unwrap();
        let mapping = Mapping::from_yaml(mappings::CONDITION).unwrap();
        let declarative = MappingEngine::apply(&mapping, &message).unwrap();
        let coded = ConditionConverter::convert_all(&message).unwrap();
        assert_eq!(
            declarative[0]["code"]["coding"][0]["system"],
            "http://hl7.org/fhir/sid/icd-10"
        );
        assert_eq!(declarative[0], serde_json::to_value(&coded[0]).unwrap());

        let message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\rAL1|1|DA|99213^Visit^C4|SV",
        )
        .unwrap();
        let mapping = Mapping::from_yaml(mappings::ALLERGY_INTOLERANCE).unwrap();
        let declarative = MappingEngine::apply(&mapping, &message).unwrap();
        let coded = AllergyIntoleranceConverter::convert_all(&message).unwrap();
        assert_eq!(
            declarative[0]["code"]["coding"][0]["system"],
            "http://www.ama-assn.org/go/cpt"
        );
        assert_eq!(declarative[0], serde_json::to_value(&coded[0]).unwrap());
    }
}
