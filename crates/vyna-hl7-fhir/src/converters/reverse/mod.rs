//! Reverse converters for transforming FHIR resources to HL7 v2.x segments
//!
//! These converters complement the forward converters by enabling bidirectional
//! conversion between FHIR R4 resources and HL7 v2.x messages.

pub mod patient;
pub mod observation;
pub mod encounter;
pub mod practitioner;

pub use patient::PatientReverseConverter;
pub use observation::ObservationReverseConverter;
pub use encounter::EncounterReverseConverter;
pub use practitioner::PractitionerReverseConverter;
