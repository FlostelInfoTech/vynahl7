//! FHIR R4 resource definitions
//!
//! Lightweight FHIR resource structures for common healthcare data.
//! Based on FHIR R4 specification: <https://www.hl7.org/fhir/R4/>

pub mod allergy_intolerance;
pub mod bundle;
pub mod common;
pub mod condition;
pub mod diagnostic_report;
pub mod encounter;
pub mod immunization;
pub mod medication;
pub mod observation;
pub mod patient;
pub mod practitioner;
pub mod procedure;
pub mod service_request;
pub mod specimen;

pub use allergy_intolerance::AllergyIntolerance;
pub use bundle::{Bundle, BundleEntry};
pub use common::*;
pub use condition::Condition;
pub use diagnostic_report::DiagnosticReport;
pub use encounter::Encounter;
pub use immunization::Immunization;
pub use medication::{Medication, MedicationAdministration};
pub use observation::Observation;
pub use patient::Patient;
pub use practitioner::Practitioner;
pub use procedure::Procedure;
pub use service_request::ServiceRequest;
pub use specimen::Specimen;
