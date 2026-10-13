//! FHIR AllergyIntolerance resource definition

use super::common::*;
use serde::{Deserialize, Serialize};

/// FHIR AllergyIntolerance resource - risk of harmful reaction to a substance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllergyIntolerance {
    /// Resource type (always "AllergyIntolerance")
    pub resource_type: String,

    /// Logical id of this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Business identifiers for this record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// active | inactive | resolved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clinical_status: Option<CodeableConcept>,

    /// unconfirmed | confirmed | refuted | entered-in-error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<CodeableConcept>,

    /// allergy | intolerance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// food | medication | environment | biologic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<String>>,

    /// low | high | unable-to-assess
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criticality: Option<String>,

    /// Code for the allergen or substance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    /// Patient who has the allergy or intolerance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patient: Option<Reference>,

    /// Date/time the allergy or intolerance was first identified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onset_date_time: Option<String>,

    /// Date the record was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_date: Option<String>,

    /// Adverse reactions observed for this substance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction: Option<Vec<AllergyIntoleranceReaction>>,
}

/// Details of an adverse reaction to the substance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllergyIntoleranceReaction {
    /// Specific substance considered responsible for the reaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substance: Option<CodeableConcept>,

    /// Clinical symptoms/signs observed with the reaction
    pub manifestation: Vec<CodeableConcept>,

    /// mild | moderate | severe
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
}

impl Default for AllergyIntolerance {
    fn default() -> Self {
        Self::new()
    }
}

impl AllergyIntolerance {
    /// Create a new AllergyIntolerance resource with default values
    pub fn new() -> Self {
        Self {
            resource_type: "AllergyIntolerance".to_string(),
            id: None,
            identifier: None,
            clinical_status: None,
            verification_status: None,
            type_: None,
            category: None,
            criticality: None,
            code: None,
            patient: None,
            onset_date_time: None,
            recorded_date: None,
            reaction: None,
        }
    }
}
