//! Converters for transforming HL7 v2.x segments to FHIR resources
//!
//! This module provides bidirectional conversion between HL7 v2.x and FHIR R4:
//!
//! - Forward converters: HL7 v2.x -> FHIR R4
//! - Reverse converters: FHIR R4 -> HL7 v2.x

pub mod allergy_intolerance;
pub mod coding_systems;
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

/// Reverse converters for FHIR R4 -> HL7 v2.x conversion
pub mod reverse;

// Forward converters (HL7 v2.x -> FHIR R4)
pub use allergy_intolerance::AllergyIntoleranceConverter;
pub use condition::ConditionConverter;
pub use diagnostic_report::DiagnosticReportConverter;
pub use encounter::EncounterConverter;
pub use immunization::ImmunizationConverter;
pub use medication::MedicationConverter;
pub use observation::ObservationConverter;
pub use patient::PatientConverter;
pub use practitioner::PractitionerConverter;
pub use procedure::ProcedureConverter;
pub use service_request::ServiceRequestConverter;
pub use specimen::SpecimenConverter;

// Reverse converters (FHIR R4 -> HL7 v2.x)
pub use reverse::EncounterReverseConverter;
pub use reverse::ObservationReverseConverter;
pub use reverse::PatientReverseConverter;
pub use reverse::PractitionerReverseConverter;
