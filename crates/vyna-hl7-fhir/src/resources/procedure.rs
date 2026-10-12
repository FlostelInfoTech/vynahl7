//! FHIR Procedure resource definition

use serde::{Deserialize, Serialize};
use super::common::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Procedure {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    pub status: String, // preparation | in-progress | completed | entered-in-error

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub performed_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<ProcedurePerformer>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcedurePerformer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<CodeableConcept>,

    pub actor: Reference,
}

impl Procedure {
    pub fn new(status: String) -> Self {
        Self {
            resource_type: "Procedure".to_string(),
            id: None,
            identifier: None,
            status,
            code: None,
            subject: None,
            encounter: None,
            performed_date_time: None,
            performer: None,
        }
    }
}
