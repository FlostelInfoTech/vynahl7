//! FHIR Medication and MedicationAdministration resources

use serde::{Deserialize, Serialize};
use super::common::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Medication {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<CodeableConcept>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MedicationAdministration {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    pub status: String, // in-progress | on-hold | completed | entered-in-error | stopped

    #[serde(skip_serializing_if = "Option::is_none")]
    pub medication_codeable_concept: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dosage: Option<MedicationAdministrationDosage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MedicationAdministrationDosage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dose: Option<Quantity>,
}

impl MedicationAdministration {
    pub fn new(status: String) -> Self {
        Self {
            resource_type: "MedicationAdministration".to_string(),
            id: None,
            status,
            medication_codeable_concept: None,
            subject: None,
            effective_date_time: None,
            dosage: None,
        }
    }
}
