//! FHIR R4 ServiceRequest resource
//!
//! Represents a request for a service to be performed.
//! Based on FHIR R4: <https://www.hl7.org/fhir/R4/servicerequest.html>

use super::common::*;
use serde::{Deserialize, Serialize};

/// FHIR ServiceRequest resource - a record of a request for service such as diagnostic investigations, treatments, or operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRequest {
    /// Resource type (always "ServiceRequest")
    pub resource_type: String,

    /// Logical id of this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    // Required fields
    /// draft | active | on-hold | revoked | completed | entered-in-error | unknown
    pub status: String,

    /// proposal | plan | directive | order | original-order | reflex-order | filler-order | instance-order | option
    pub intent: String,

    /// Individual or Entity the service is ordered for
    pub subject: Reference,

    // Optional fields
    /// Business identifiers for this request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// Shared identifier for a composite order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requisition: Option<Identifier>,

    /// Classification of the requested service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<CodeableConcept>>,

    /// routine | urgent | asap | stat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,

    /// What is being requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    /// Additional details for the order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_detail: Option<Vec<CodeableConcept>>,

    /// Amount of service requested
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity_quantity: Option<Quantity>,

    /// Encounter during which the request was made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    /// Date/time the service should occur
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_date_time: Option<String>,

    /// Period in which the service should occur
    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_period: Option<Period>,

    /// Date the request was authored
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored_on: Option<String>,

    /// Practitioner or device that made the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requester: Option<Reference>,

    /// Discipline of the performer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer_type: Option<CodeableConcept>,

    /// Requested performers of the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<Reference>>,

    /// Coded locations for the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_code: Option<Vec<CodeableConcept>>,

    /// Location references for the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_reference: Option<Vec<Reference>>,

    /// Explanations for the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<Vec<CodeableConcept>>,

    /// Conditions or observations justifying the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_reference: Option<Vec<Reference>>,

    /// Specimens to use for the service
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specimen: Option<Vec<Reference>>,

    /// Notes about the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,

    /// Prior requests or events this request follows
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevant_history: Option<Vec<Reference>>,
}

/// FHIR Period - a time period defined by start and end date/time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    /// Start of the period
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    /// End of the period
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
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

impl ServiceRequest {
    /// Create a new ServiceRequest with required fields
    pub fn new(status: String, intent: String, subject: Reference) -> Self {
        Self {
            resource_type: "ServiceRequest".to_string(),
            id: None,
            status,
            intent,
            subject,
            identifier: None,
            requisition: None,
            category: None,
            priority: None,
            code: None,
            order_detail: None,
            quantity_quantity: None,
            encounter: None,
            occurrence_date_time: None,
            occurrence_period: None,
            authored_on: None,
            requester: None,
            performer_type: None,
            performer: None,
            location_code: None,
            location_reference: None,
            reason_code: None,
            reason_reference: None,
            specimen: None,
            note: None,
            relevant_history: None,
        }
    }
}
