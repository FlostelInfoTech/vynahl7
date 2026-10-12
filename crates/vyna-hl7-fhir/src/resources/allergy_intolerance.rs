//! FHIR AllergyIntolerance resource definition

use serde::{Deserialize, Serialize};
use super::common::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllergyIntolerance {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub clinical_status: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>, // allergy | intolerance

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<String>>, // food | medication | environment | biologic

    #[serde(skip_serializing_if = "Option::is_none")]
    pub criticality: Option<String>, // low | high | unable-to-assess

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub patient: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub onset_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reaction: Option<Vec<AllergyIntoleranceReaction>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllergyIntoleranceReaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub substance: Option<CodeableConcept>,

    pub manifestation: Vec<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>, // mild | moderate | severe
}

impl Default for AllergyIntolerance {
    fn default() -> Self {
        Self::new()
    }
}

impl AllergyIntolerance {
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
