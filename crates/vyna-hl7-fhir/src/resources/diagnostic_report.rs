//! FHIR DiagnosticReport resource definition

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR DiagnosticReport resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// registered | partial | preliminary | final
    pub status: String,

    /// Service category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<CodeableConcept>>,

    /// Name/Code for this diagnostic report
    pub code: CodeableConcept,

    /// The subject of the report
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    /// Health care event when test ordered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    /// Clinically relevant time/time-period for report
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_date_time: Option<String>,

    /// DateTime this version was made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued: Option<String>,

    /// Responsible Diagnostic Service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<Reference>>,

    /// Specimens this report is based on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specimen: Option<Vec<Reference>>,

    /// Observations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<Reference>>,

    /// Clinical conclusion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<String>,
}

impl DiagnosticReport {
    pub fn new(status: String, code: CodeableConcept) -> Self {
        Self {
            resource_type: "DiagnosticReport".to_string(),
            id: None,
            identifier: None,
            status,
            category: None,
            code,
            subject: None,
            encounter: None,
            effective_date_time: None,
            issued: None,
            performer: None,
            specimen: None,
            result: None,
            conclusion: None,
        }
    }
}
