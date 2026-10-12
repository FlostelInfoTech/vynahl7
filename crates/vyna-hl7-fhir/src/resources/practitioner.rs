//! FHIR Practitioner resource
//!
//! Based on FHIR R4 Practitioner: <https://www.hl7.org/fhir/R4/practitioner.html>

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR Practitioner resource - A person with a formal responsibility in the provisioning of healthcare
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Practitioner {
    /// Resource type (always "Practitioner")
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,

    /// An identifier for the person as this agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// Whether this practitioner's record is in active use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// The name(s) associated with the practitioner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Vec<HumanName>>,

    /// A contact detail for the practitioner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<ContactPoint>>,

    /// Address(es) of the practitioner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<Address>>,

    /// Administrative Gender - male | female | other | unknown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,

    /// The date of birth for the practitioner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,

    /// Qualifications obtained by training and certification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qualification: Option<Vec<PractitionerQualification>>,

    /// A language the practitioner can use in patient communication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication: Option<Vec<CodeableConcept>>,
}

/// Qualifications obtained by training and certification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PractitionerQualification {
    /// An identifier for this qualification for the practitioner
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// Coded representation of the qualification
    pub code: CodeableConcept,

    /// Period during which the qualification is valid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,

    /// Organization that regulates and issues the qualification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<Reference>,
}

/// Time period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

impl Practitioner {
    /// Create a new Practitioner resource with default values
    pub fn new() -> Self {
        Self {
            resource_type: "Practitioner".to_string(),
            id: None,
            meta: None,
            identifier: None,
            active: None,
            name: None,
            telecom: None,
            address: None,
            gender: None,
            birth_date: None,
            qualification: None,
            communication: None,
        }
    }
}

impl Default for Practitioner {
    fn default() -> Self {
        Self::new()
    }
}
