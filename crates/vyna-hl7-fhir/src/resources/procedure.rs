//! FHIR Procedure resource definition

use super::common::*;
use serde::{Deserialize, Serialize};

/// FHIR Procedure resource - an action performed on or for a patient
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Procedure {
    /// Resource type (always "Procedure")
    pub resource_type: String,

    /// Logical id of this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Business identifiers for this procedure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// preparation | in-progress | not-done | on-hold | stopped | completed
    /// | entered-in-error | unknown
    pub status: String,

    /// Identification of the procedure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    /// Patient on whom the procedure was performed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    /// Encounter during which the procedure was performed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    /// Date/time the procedure was performed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performed_date_time: Option<String>,

    /// Who performed the procedure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<ProcedurePerformer>>,
}

/// Performer of a procedure and their role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcedurePerformer {
    /// Role the performer held (surgeon, anesthetist, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<CodeableConcept>,

    /// Practitioner or device that performed the procedure
    pub actor: Reference,
}

impl Procedure {
    /// Create a new Procedure with the given status
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
