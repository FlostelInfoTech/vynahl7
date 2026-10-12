//! FHIR R4 ServiceRequest resource
//!
//! Represents a request for a service to be performed.
//! Based on FHIR R4: <https://www.hl7.org/fhir/R4/servicerequest.html>

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR ServiceRequest resource - a record of a request for service such as diagnostic investigations, treatments, or operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRequest {
    pub resource_type: String,

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requisition: Option<Identifier>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_detail: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity_quantity: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub occurrence_period: Option<Period>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub authored_on: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requester: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer_type: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_code: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_reference: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_reference: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub specimen: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevant_history: Option<Vec<Reference>>,
}

/// FHIR Period - a time period defined by start and end date/time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
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

impl ServiceRequest {
    /// Create a new ServiceRequest with required fields
    pub fn new(
        status: String,
        intent: String,
        subject: Reference,
    ) -> Self {
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
