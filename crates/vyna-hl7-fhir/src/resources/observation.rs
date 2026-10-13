//! FHIR Observation resource
//!
//! Based on FHIR R4 Observation: <https://www.hl7.org/fhir/R4/observation.html>

use super::common::*;
use serde::{Deserialize, Serialize};

/// FHIR Observation resource - Measurements and simple assertions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    /// Resource type (always "Observation")
    pub resource_type: String,

    /// Logical id of this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Metadata about the resource
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

    /// Clinically relevant time period for the observation
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

    /// Result as a coded value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_codeable_concept: Option<CodeableConcept>,

    /// Result as plain text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,

    /// Result as true/false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_boolean: Option<bool>,

    /// Result as a whole number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_integer: Option<i32>,

    /// Result as a range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_range: Option<Range>,

    /// Result as a ratio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_ratio: Option<Ratio>,

    /// Result as a time of day
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_time: Option<String>,

    /// Result as a date/time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_date_time: Option<String>,

    /// Result as a time period
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
    /// Lower bound of the reference range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<Quantity>,

    /// Upper bound of the reference range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<Quantity>,

    /// Type of reference range (normal, therapeutic, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<CodeableConcept>,

    /// Text description of the range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Component results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ObservationComponent {
    /// Type of the component result
    pub code: CodeableConcept,

    /// Component result as a quantity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_quantity: Option<Quantity>,

    /// Component result as a coded value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_codeable_concept: Option<CodeableConcept>,

    /// Component result as plain text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_string: Option<String>,

    /// High, low, normal, etc. for this component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpretation: Option<Vec<CodeableConcept>>,

    /// Reference ranges for this component
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_range: Option<Vec<ObservationReferenceRange>>,
}

/// Time period with optional start and end instants
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

/// Range with low and high values
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    /// Lower bound of the range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<Quantity>,

    /// Upper bound of the range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<Quantity>,
}

/// Ratio of two quantities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Ratio {
    /// Numerator of the ratio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numerator: Option<Quantity>,

    /// Denominator of the ratio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub denominator: Option<Quantity>,
}

/// Text note with attribution
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
