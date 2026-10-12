//! Common FHIR data types and structures

use serde::{Deserialize, Serialize};

/// FHIR HumanName - Name of a human
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HumanName {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>, // usual | official | temp | nickname | anonymous | old | maiden

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub given: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<Vec<String>>,
}

/// FHIR Address
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>, // home | work | temp | old | billing

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>, // postal | physical | both

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// FHIR ContactPoint - Details for all kinds of technology-mediated contact points
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContactPoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>, // phone | fax | email | pager | url | sms | other

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>, // home | work | temp | old | mobile
}

/// FHIR Identifier - An identifier intended for computation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_: Option<String>, // usual | official | temp | secondary | old

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<CodeableConcept>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigner: Option<Box<Reference>>,
}

/// FHIR CodeableConcept - A concept that may be defined by a formal reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodeableConcept {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coding: Option<Vec<Coding>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// FHIR Coding - A reference to a code defined by a terminology system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Coding {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR Reference - A reference from one resource to another
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Box<Identifier>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

/// FHIR Quantity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Quantity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// FHIR Meta - Metadata about a resource
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// FHIR Period - Time period defined by a start and end date/time
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
}
