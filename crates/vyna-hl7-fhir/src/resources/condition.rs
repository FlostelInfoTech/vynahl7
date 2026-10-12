//! FHIR Condition resource definition

use serde::{Deserialize, Serialize};
use super::common::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
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
    pub category: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub onset_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_date: Option<String>,
}

impl Default for Condition {
    fn default() -> Self {
        Self::new()
    }
}

impl Condition {
    pub fn new() -> Self {
        Self {
            resource_type: "Condition".to_string(),
            id: None,
            identifier: None,
            clinical_status: None,
            verification_status: None,
            category: None,
            severity: None,
            code: None,
            subject: None,
            encounter: None,
            onset_date_time: None,
            recorded_date: None,
        }
    }
}
