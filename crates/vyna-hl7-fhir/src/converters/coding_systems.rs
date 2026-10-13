//! Shared HL7 v2 coding-system to FHIR system-URI mapping.
//!
//! CWE-coded fields (RXA-5, PRB-3, DG1-3, ...) carry the coding
//! system in component 3. This table maps those identifiers to
//! FHIR system URIs so every converter resolves them identically.

/// Convert an HL7 v2 coding-system identifier to a FHIR system URI.
///
/// Unknown systems keep the historical `urn:oid:` fallback.
pub fn convert_coding_system(system: &str) -> String {
    match system {
        "CVX" => "http://hl7.org/fhir/sid/cvx".to_string(),
        "NDC" => "http://hl7.org/fhir/sid/ndc".to_string(),
        "MVX" => "http://hl7.org/fhir/sid/mvx".to_string(),
        "RXNORM" => "http://www.nlm.nih.gov/research/umls/rxnorm".to_string(),
        "LN" | "LNC" => "http://loinc.org".to_string(),
        "SNM" | "SCT" => "http://snomed.info/sct".to_string(),
        "ICD9" => "http://hl7.org/fhir/sid/icd-9-cm".to_string(),
        "ICD10" => "http://hl7.org/fhir/sid/icd-10".to_string(),
        "CPT" => "http://www.ama-assn.org/go/cpt".to_string(),
        _ => format!("urn:oid:{system}"),
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
}
