//! Reverse converters for transforming FHIR resources to HL7 v2.x segments
//!
//! These converters complement the forward converters by enabling bidirectional
//! conversion between FHIR R4 resources and HL7 v2.x messages.

pub mod encounter;
pub mod observation;
pub mod patient;
pub mod practitioner;

pub use encounter::EncounterReverseConverter;
pub use observation::ObservationReverseConverter;
pub use patient::PatientReverseConverter;
pub use practitioner::PractitionerReverseConverter;
