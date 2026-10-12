//! FHIR Encounter resource definition
//!
//! An interaction between a patient and healthcare provider(s) for the purpose
//! of providing healthcare service(s) or assessing the health status of a patient.

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR Encounter resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encounter {
    /// Resource type (always "Encounter")
    pub resource_type: String,

    /// Logical id of this artifact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Identifier(s) by which this encounter is known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// planned | arrived | triaged | in-progress | onleave | finished | cancelled
    pub status: String,

    /// Classification of patient encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<Coding>,

    /// Specific type of encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<Vec<CodeableConcept>>,

    /// Indicates the urgency of the encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<CodeableConcept>,

    /// The patient present at the encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    /// List of participants involved in the encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub participant: Option<Vec<EncounterParticipant>>,

    /// The start and end time of the encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,

    /// List of locations where the patient has been
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Vec<EncounterLocation>>,

    /// The organization (facility) responsible for this encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_provider: Option<Reference>,

    /// Details about the admission to a healthcare service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hospitalization: Option<EncounterHospitalization>,
}

/// Encounter participant
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterParticipant {
    /// Role of participant in encounter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<Vec<CodeableConcept>>,

    /// Period of time during the encounter that the participant participated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,

    /// Persons involved in the encounter other than the patient
    #[serde(skip_serializing_if = "Option::is_none")]
    pub individual: Option<Reference>,
}

/// Encounter location
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterLocation {
    /// Location the encounter takes place
    pub location: Reference,

    /// planned | active | reserved | completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Time period during which the patient was present at the location
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<Period>,
}

/// Encounter hospitalization details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterHospitalization {
    /// Pre-admission identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_admission_identifier: Option<Identifier>,

    /// The location from which the patient came before admission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Reference>,

    /// From where patient was admitted (physician referral, transfer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admit_source: Option<CodeableConcept>,

    /// Whether this hospitalization is a readmission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub re_admission: Option<CodeableConcept>,

    /// Diet preferences reported by the patient
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diet_preference: Option<Vec<CodeableConcept>>,

    /// Special courtesies (VIP, board member)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_courtesy: Option<Vec<CodeableConcept>>,

    /// Wheelchair, translator, stretcher, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special_arrangement: Option<Vec<CodeableConcept>>,

    /// Location to which the patient is discharged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<Reference>,

    /// Category or kind of location after discharge
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discharge_disposition: Option<CodeableConcept>,
}

impl Encounter {
    /// Create a new Encounter with required fields
    pub fn new(status: String) -> Self {
        Self {
            resource_type: "Encounter".to_string(),
            id: None,
            identifier: None,
            status,
            class: None,
            type_: None,
            priority: None,
            subject: None,
            participant: None,
            period: None,
            location: None,
            service_provider: None,
            hospitalization: None,
        }
    }
}
