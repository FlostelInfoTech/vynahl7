//! FHIR R4 Immunization resource
//!
//! Represents immunization events (vaccine administration).
//! Based on FHIR R4: <https://www.hl7.org/fhir/R4/immunization.html>

use super::common::*;
use serde::{Deserialize, Serialize};

/// FHIR Immunization resource - describes the event of a patient being administered a vaccine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Immunization {
    /// Resource type (always "Immunization")
    pub resource_type: String,

    /// Logical id of this resource
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
    /// Encounter where the vaccine was administered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    /// Date the record was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded: Option<String>,

    /// Whether the record comes from the administering provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_source: Option<bool>,

    /// Source of the record when not the primary source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_origin: Option<CodeableConcept>,

    /// Location where the vaccine was administered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Reference>,

    /// Vaccine manufacturer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<Reference>,

    /// Manufacturer lot number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lot_number: Option<String>,

    /// Expiration date of the vaccine lot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<String>,

    /// Body site of administration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site: Option<CodeableConcept>,

    /// Route of administration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<CodeableConcept>,

    /// Amount of vaccine administered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dose_quantity: Option<Quantity>,

    /// Who performed the administration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<ImmunizationPerformer>>,

    /// Notes about the administration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,

    /// Reasons the vaccine was given
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<Vec<CodeableConcept>>,

    /// Whether the dose potency was reduced
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_subpotent: Option<bool>,

    /// Reason the current status applies
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_reason: Option<CodeableConcept>,

    /// Vaccination protocol that was followed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_applied: Option<Vec<ImmunizationProtocolApplied>>,
}

/// Who performed the immunization event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImmunizationPerformer {
    /// Role in the administration (administering provider, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<CodeableConcept>,

    /// Practitioner or organization that performed the administration
    pub actor: Reference,
}

/// Protocol followed by the provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImmunizationProtocolApplied {
    /// Name of the vaccine series
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series: Option<String>,

    /// Authority that publishes the protocol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<Reference>,

    /// Diseases the protocol protects against
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_disease: Option<Vec<CodeableConcept>>,

    /// Dose number within the series
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dose_number_positive_int: Option<u32>,

    /// Dose number as text (when not a plain count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dose_number_string: Option<String>,

    /// Recommended number of doses for the series
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_doses_positive_int: Option<u32>,

    /// Recommended number of doses as text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_doses_string: Option<String>,
}

/// FHIR Annotation - text node with attribution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    /// Practitioner or organization that authored the note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_reference: Option<Reference>,

    /// Author name when no reference is available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_string: Option<String>,

    /// When the note was made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,

    /// The note text
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
