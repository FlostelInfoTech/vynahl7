//! HL7 v2.x to FHIR R4 conversion utilities
//!
//! This crate provides functionality to convert HL7 v2.x messages and segments
//! into FHIR R4 resources following the official HL7 v2-to-FHIR mapping specification.
//!
//! # Features
//!
//! - Convert HL7 v2.x segments to FHIR R4 resources
//! - Support for Patient, Observation, Practitioner, and other common resources
//! - Configurable conversion with extension support
//! - Validation of converted resources
//!
//! # Examples
//!
//! ```rust,ignore
//! use vyna_hl7_fhir::converters::patient::PatientConverter;
//! use vyna_hl7_parser::parse_message;
//!
//! let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\n\
//!            PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M";
//!
//! let message = parse_message(hl7)?;
//! let patient = PatientConverter::convert(&message)?;
//! let json = serde_json::to_string_pretty(&patient)?;
//! ```

pub mod converters;
pub mod resources;
pub mod error;
pub mod mapping;
pub mod bundle;

pub use error::{ConversionError, ConversionResult};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::bundle::{BundleBuilder, convert_message_to_bundle};
    pub use crate::converters::observation::ObservationConverter;
    pub use crate::converters::patient::PatientConverter;
    pub use crate::converters::practitioner::PractitionerConverter;
    pub use crate::error::{ConversionError, ConversionResult};
    pub use crate::mapping::{Mapping, MappingEngine};
    pub use crate::resources::*;
}
