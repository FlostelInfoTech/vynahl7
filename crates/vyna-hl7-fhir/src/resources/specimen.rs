//! FHIR R4 Specimen resource
//!
//! Represents a sample to be used for analysis.
//! Based on FHIR R4: <https://www.hl7.org/fhir/R4/specimen.html>

use serde::{Deserialize, Serialize};
use super::common::*;

/// FHIR Specimen resource - a sample to be used for analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Specimen {
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    // Required fields
    /// Kind of material that forms the specimen
    pub type_: CodeableConcept,

    /// Where the specimen came from (Patient or Group)
    pub subject: Reference,

    // Optional fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub accession_identifier: Option<Identifier>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>, // available | unavailable | unsatisfactory | entered-in-error

    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<SpecimenCollection>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing: Option<Vec<SpecimenProcessing>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Vec<SpecimenContainer>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<Vec<CodeableConcept>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,
}

/// Details concerning the specimen collection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenCollection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collector: Option<Reference>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub collected_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub collected_period: Option<Period>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_site: Option<CodeableConcept>,
}

/// Details concerning processing and processing steps for the specimen
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenProcessing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub procedure: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub additive: Option<Vec<Reference>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_date_time: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_period: Option<Period>,
}

/// Direct container of specimen (tube/slide/etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenContainer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub specimen_quantity: Option<Quantity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub additive_codeable_concept: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub additive_reference: Option<Reference>,
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

impl Specimen {
    /// Create a new Specimen with required fields
    pub fn new(
        type_: CodeableConcept,
        subject: Reference,
    ) -> Self {
        Self {
            resource_type: "Specimen".to_string(),
            id: None,
            type_,
            subject,
            identifier: None,
            accession_identifier: None,
            status: None,
            received_time: None,
            parent: None,
            request: None,
            collection: None,
            processing: None,
            container: None,
            condition: None,
            note: None,
        }
    }
}
