//! Common FHIR data types and structures

use serde::{Deserialize, Serialize};

/// FHIR HumanName - Name of a human
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HumanName {
    /// usual | official | temp | nickname | anonymous | old | maiden
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,

    /// Full text representation of the name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Family (surname) part of the name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,

    /// Given (first) names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<Vec<String>>,

    /// Name prefixes (Mr, Dr, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<Vec<String>>,

    /// Name suffixes (Jr, PhD, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<Vec<String>>,
}

/// FHIR Address
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    /// home | work | temp | old | billing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,

    /// postal | physical | both
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Full text representation of the address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Street names and numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Vec<String>>,

    /// City name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    /// State or province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// Postal (ZIP) code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    /// Country code or name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// FHIR ContactPoint - Details for all kinds of technology-mediated contact points
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContactPoint {
    /// phone | fax | email | pager | url | sms | other
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// The contact value (number, address, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// home | work | temp | old | mobile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,
}

/// FHIR Identifier - An identifier intended for computation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    /// usual | official | temp | secondary | old
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>,

    /// Coded identifier type (MRN, SSN, ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<CodeableConcept>,

    /// Namespace for the identifier value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// The identifier value itself
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Organization that assigned the identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigner: Option<Box<Reference>>,
}

/// FHIR CodeableConcept - A concept that may be defined by a formal reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodeableConcept {
    /// Codes from terminology systems
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<Coding>>,

    /// Plain-text representation of the concept
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// FHIR Coding - A reference to a code defined by a terminology system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Coding {
    /// Identity of the terminology system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Version of the terminology system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Symbol in the terminology system
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Human-readable meaning of the code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR Reference - A reference from one resource to another
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    /// Relative URL of the target (e.g. "Patient/7")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,

    /// Resource type of the target
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    /// Logical identifier of the target
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Box<Identifier>>,

    /// Text alternative for the resource
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR Quantity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Quantity {
    /// Numerical value of the quantity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,

    /// Human-readable unit of measure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,

    /// Terminology system for the coded unit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Coded form of the unit
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// FHIR Meta - Metadata about a resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    /// Version id assigned by the server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,

    /// When the resource was last changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,

    /// Source system that created the record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// FHIR Period - Time period defined by a start and end date/time
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
