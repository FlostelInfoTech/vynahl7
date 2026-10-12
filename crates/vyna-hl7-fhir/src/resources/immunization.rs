//! FHIR R4 Immunization resource
//!
//! Represents immunization events (vaccine administration).
//! Based on FHIR R4: <https://www.hl7.org/fhir/R4/immunization.html>

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR Immunization resource - describes the event of a patient being administered a vaccine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Immunization {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    // Required fields
    /// completed | entered-in-error | not-done
    pub status: String,

    /// Vaccine product administered
    pub vaccine_code: CodeableConcept,

    /// Who was immunized
    pub patient: Reference,

    /// Vaccine administration date
    pub occurrence_date_time: String,

    // Optional fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_source: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_origin: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_number: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dose_quantity: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<ImmunizationPerformer>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_subpotent: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_reason: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_applied: Option<Vec<ImmunizationProtocolApplied>>,
}

/// Who performed the immunization event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImmunizationPerformer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<CodeableConcept>,

    pub actor: Reference,
}

/// Protocol followed by the provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImmunizationProtocolApplied {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_disease: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dose_number_positive_int: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub dose_number_string: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_doses_positive_int: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_doses_string: Option<String>,
}

/// FHIR Annotation - text node with attribution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_reference: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_string: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,

    pub text: String,
}

impl Immunization {
    /// Create a new Immunization with required fields
    pub fn new(
        status: String,
        vaccine_code: CodeableConcept,
        patient: Reference,
        occurrence_date_time: String,
    ) -> Self {
        Self {
            resource_type: "Immunization".to_string(),
            id: None,
            status,
            vaccine_code,
            patient,
            occurrence_date_time,
            encounter: None,
            recorded: None,
            primary_source: None,
            report_origin: None,
            location: None,
            manufacturer: None,
            lot_number: None,
            expiration_date: None,
            site: None,
            route: None,
            dose_quantity: None,
            performer: None,
            note: None,
            reason_code: None,
            is_subpotent: None,
            status_reason: None,
            protocol_applied: None,
        }
    }
}
