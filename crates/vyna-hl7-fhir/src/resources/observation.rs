//! FHIR Observation resource
//!
//! Based on FHIR R4 Observation: <https://www.hl7.org/fhir/R4/observation.html>

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR Observation resource - Measurements and simple assertions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    /// Resource type (always "Observation")
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Meta>,

    /// Business identifier for the observation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// registered | preliminary | final | amended | corrected | cancelled | entered-in-error | unknown
    pub status: String,

    /// Classification of type of observation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Vec<CodeableConcept>>,

    /// Type of observation (code / type)
    pub code: CodeableConcept,

    /// Who and/or what the observation is about
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Reference>,

    /// Healthcare event during which this observation is made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encounter: Option<Reference>,

    /// Clinically relevant time/time-period for observation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_period: Option<Period>,

    /// Date/Time this version was made available
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued: Option<String>,

    /// Who is responsible for the observation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performer: Option<Vec<Reference>>,

    /// Actual result - one of these
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_quantity: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_codeable_concept: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_boolean: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_integer: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_range: Option<Range>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_ratio: Option<Ratio>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_period: Option<Period>,

    /// Why the result is missing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_absent_reason: Option<CodeableConcept>,

    /// High, low, normal, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation: Option<Vec<CodeableConcept>>,

    /// Comments about the observation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,

    /// Provides guide for interpretation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_range: Option<Vec<ObservationReferenceRange>>,

    /// Related resource that belongs to the Observation group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_member: Option<Vec<Reference>>,

    /// Related measurements the observation is made from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<Vec<Reference>>,

    /// Component results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component: Option<Vec<ObservationComponent>>,
}

/// Provides guide for interpretation of component result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationReferenceRange {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Component results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationComponent {
    pub code: CodeableConcept,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_quantity: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_codeable_concept: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_range: Option<Vec<ObservationReferenceRange>>,
}

/// Time period
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}

/// Range with low and high values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<Quantity>,
}

/// Ratio of two quantities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ratio {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numerator: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub denominator: Option<Quantity>,
}

/// Annotation
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

impl Observation {
    /// Create a new Observation resource with required fields
    pub fn new(status: String, code: CodeableConcept) -> Self {
        Self {
            resource_type: "Observation".to_string(),
            id: None,
            meta: None,
            identifier: None,
            status,
            category: None,
            code,
            subject: None,
            encounter: None,
            effective_date_time: None,
            effective_period: None,
            issued: None,
            performer: None,
            value_quantity: None,
            value_codeable_concept: None,
            value_string: None,
            value_boolean: None,
            value_integer: None,
            value_range: None,
            value_ratio: None,
            value_time: None,
            value_date_time: None,
            value_period: None,
            data_absent_reason: None,
            interpretation: None,
            note: None,
            reference_range: None,
            has_member: None,
            derived_from: None,
            component: None,
        }
    }
}
