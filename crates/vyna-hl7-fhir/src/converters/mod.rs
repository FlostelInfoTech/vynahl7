//! Converters for transforming HL7 v2.x segments to FHIR resources
//!
//! This module provides bidirectional conversion between HL7 v2.x and FHIR R4:
//!
//! - Forward converters: HL7 v2.x -> FHIR R4
//! - Reverse converters: FHIR R4 -> HL7 v2.x

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

/// Reverse converters for FHIR R4 -> HL7 v2.x conversion
pub mod reverse;

// Forward converters (HL7 v2.x -> FHIR R4)
pub use patient::PatientConverter;
pub use observation::ObservationConverter;
pub use practitioner::PractitionerConverter;
pub use encounter::EncounterConverter;
pub use diagnostic_report::DiagnosticReportConverter;
pub use allergy_intolerance::AllergyIntoleranceConverter;
pub use medication::MedicationConverter;
pub use condition::ConditionConverter;
pub use procedure::ProcedureConverter;
pub use immunization::ImmunizationConverter;
pub use service_request::ServiceRequestConverter;
pub use specimen::SpecimenConverter;

// Reverse converters (FHIR R4 -> HL7 v2.x)
pub use reverse::PatientReverseConverter;
pub use reverse::ObservationReverseConverter;
pub use reverse::EncounterReverseConverter;
pub use reverse::PractitionerReverseConverter;
