//! FHIR R4 Specimen resource
//!
//! Represents a sample to be used for analysis.
//! Based on FHIR R4: <https://www.hl7.org/fhir/R4/specimen.html>

use super::common::*;
use serde::{Deserialize, Serialize};

/// FHIR Specimen resource - a sample to be used for analysis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Specimen {
    /// Resource type (always "Specimen")
    pub resource_type: String,

    /// Logical id of this resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    // Required fields
    /// Kind of material that forms the specimen
    pub type_: CodeableConcept,

    /// Where the specimen came from (Patient or Group)
    pub subject: Reference,

    // Optional fields
    /// Business identifiers for the specimen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// Identifier assigned by the lab
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accession_identifier: Option<Identifier>,

    /// available | unavailable | unsatisfactory | entered-in-error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Date/time the specimen was received
    #[serde(skip_serializing_if = "Option::is_none")]
    pub received_time: Option<String>,

    /// Parent specimens this specimen came from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Vec<Reference>>,

    /// ServiceRequests that ordered the specimen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<Vec<Reference>>,

    /// How the specimen was collected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<SpecimenCollection>,

    /// Processing steps applied to the specimen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing: Option<Vec<SpecimenProcessing>>,

    /// Containers holding the specimen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<Vec<SpecimenContainer>>,

    /// State of the specimen (frozen, hemolyzed, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<Vec<CodeableConcept>>,

    /// Notes about the specimen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<Vec<Annotation>>,
}

/// Details concerning the specimen collection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenCollection {
    /// Practitioner or device that collected the specimen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collector: Option<Reference>,

    /// Date/time the specimen was collected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collected_date_time: Option<String>,

    /// Period over which the specimen was collected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collected_period: Option<Period>,

    /// Amount collected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Quantity>,

    /// Collection technique (venipuncture, biopsy, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<CodeableConcept>,

    /// Anatomical site of collection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_site: Option<CodeableConcept>,
}

/// Details concerning processing and processing steps for the specimen
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenProcessing {
    /// Text description of the processing step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Coded processing procedure (centrifugation, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub procedure: Option<CodeableConcept>,

    /// Additives introduced during processing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additive: Option<Vec<Reference>>,

    /// Date/time the processing occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_date_time: Option<String>,

    /// Period over which the processing occurred
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_period: Option<Period>,
}

/// Direct container of specimen (tube/slide/etc.)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpecimenContainer {
    /// Business identifiers for the container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,

    /// Text description of the container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Kind of container (tube, slide, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<CodeableConcept>,

    /// Container capacity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<Quantity>,

    /// Amount of specimen in the container
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specimen_quantity: Option<Quantity>,

    /// Additive in the container, as a code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additive_codeable_concept: Option<CodeableConcept>,

    /// Additive in the container, as a substance reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additive_reference: Option<Reference>,
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

impl Specimen {
    /// Create a new Specimen with required fields
    pub fn new(type_: CodeableConcept, subject: Reference) -> Self {
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
