//! FHIR R4 resource definitions
//!
//! Lightweight FHIR resource structures for common healthcare data.
//! Based on FHIR R4 specification: <https://www.hl7.org/fhir/R4/>

pub mod patient;
pub mod observation;
pub mod practitioner;
pub mod encounter;
pub mod diagnostic_report;
pub mod allergy_intolerance;
pub mod medication;
pub mod condition;
pub mod procedure;
pub mod immunization;
pub mod service_request;
pub mod specimen;
pub mod common;

pub use patient::Patient;
pub use observation::Observation;
pub use practitioner::Practitioner;
pub use encounter::Encounter;
pub use diagnostic_report::DiagnosticReport;
pub use allergy_intolerance::AllergyIntolerance;
pub use medication::{Medication, MedicationAdministration};
pub use condition::Condition;
pub use procedure::Procedure;
pub use immunization::Immunization;
pub use service_request::ServiceRequest;
pub use specimen::Specimen;
pub use common::*;
