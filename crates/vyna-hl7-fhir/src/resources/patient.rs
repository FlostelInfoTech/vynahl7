//! FHIR Patient resource
//!
//! Based on FHIR R4 Patient: <https://www.hl7.org/fhir/R4/patient.html>

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR Patient resource - Information about an individual receiving health care services
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Patient {
    /// Resource type (always "Patient")
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,

    /// An identifier for this patient
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// Whether this patient record is in active use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    /// A name associated with the patient
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<Vec<HumanName>>,

    /// A contact detail for the individual
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telecom: Option<Vec<ContactPoint>>,

    /// Administrative Gender - male | female | other | unknown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,

    /// The date of birth for the individual
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,

    /// Indicates if the individual is deceased or not
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased_boolean: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased_date_time: Option<String>,

    /// An address for the individual
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<Vec<Address>>,

    /// Marital (civil) status of a patient
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marital_status: Option<CodeableConcept>,

    /// Whether patient is part of a multiple birth
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_birth_boolean: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiple_birth_integer: Option<i32>,

    /// A language which may be used to communicate with the patient about their health
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication: Option<Vec<PatientCommunication>>,

    /// Patient's nominated primary care provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub general_practitioner: Option<Vec<Reference>>,

    /// Organization that is the custodian of the patient record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managing_organization: Option<Reference>,
}

/// A language which may be used to communicate with the patient about their health
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatientCommunication {
    /// The language which can be used to communicate with the patient
    pub language: CodeableConcept,

    /// Language preference indicator
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred: Option<bool>,
}

impl Patient {
    /// Create a new Patient resource with default values
    pub fn new() -> Self {
        Self {
            resource_type: "Patient".to_string(),
            id: None,
            meta: None,
            identifier: None,
            active: None,
            name: None,
            telecom: None,
            gender: None,
            birth_date: None,
            deceased_boolean: None,
            deceased_date_time: None,
            address: None,
            marital_status: None,
            multiple_birth_boolean: None,
            multiple_birth_integer: None,
            communication: None,
            general_practitioner: None,
            managing_organization: None,
        }
    }
}

impl Default for Patient {
    fn default() -> Self {
        Self::new()
    }
}
