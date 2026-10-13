//! FHIR Condition resource definition

use super::common::*;
use serde::{Deserialize, Serialize};

/// FHIR Condition resource - a clinical condition, problem, or diagnosis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condition {
    /// Resource type (always "Condition")
    pub resource_type: String,

    /// Logical id of this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Business identifiers for this condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// active | recurrence | relapse | inactive | remission | resolved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clinical_status: Option<CodeableConcept>,

    /// unconfirmed | provisional | differential | confirmed | refuted | entered-in-error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<CodeableConcept>,

    /// Categories such as problem-list-item or encounter-diagnosis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<CodeableConcept>>,

    /// Subjective severity of the condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<CodeableConcept>,

    /// Identification of the condition or diagnosis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    /// Patient who has the condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    /// Encounter during which the condition was asserted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    /// Estimated or actual date the condition began
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onset_date_time: Option<String>,

    /// Date the record was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded_date: Option<String>,
}

impl Default for Condition {
    fn default() -> Self {
        Self::new()
    }
}

impl Condition {
    /// Create a new Condition resource with default values
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
