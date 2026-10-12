//! # VynaHL7 Conformance Profile Validation
//!
//! This crate provides HL7 v2.x conformance profile validation for the VynaHL7 library.
//! Conformance profiles allow organizations to constrain and customize HL7 message
//! specifications to meet specific implementation requirements.
//!
//! ## Features
//!
//! - **XML Profile Parsing**: Load conformance profiles from XML files
//! - **Usage Validation**: Enforce R (Required), RE (Required if Known), O (Optional), X (Not Used)
//! - **Cardinality Validation**: Check min/max occurrence constraints
//! - **Length Validation**: Enforce maximum field lengths
//! - **Integration**: Seamless integration with vyna-hl7-validator
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use vyna_hl7_conformance::{ProfileParser, ConformanceValidator};
//! use vyna_hl7_parser::parse_message;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Load conformance profile from XML file
//! let profile = ProfileParser::parse_file("profiles/adt_a01.xml")?;
//!
//! // Create validator
//! let validator = ConformanceValidator::new(profile);
//!
//! // Parse and validate message
//! let hl7_text = "MSH|^~\\&|..."; // Your HL7 message here
//! let message = parse_message(hl7_text)?;
//! let result = validator.validate(&message);
//!
//! // Check results
//! if !result.is_valid() {
//!     for error in &result.errors {
//!         println!("{}: {}", error.location, error.message);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Phase 1 MVP Scope
//!
//! This initial release focuses on:
//! - Basic XML profile parsing (segments and fields)
//! - Usage code validation (R, RE, O, X)
//! - Min/Max cardinality validation
//! - Field length constraints
//!
//! Future releases will add:
//! - Conditional predicates (C usage codes)
//! - Component-level validation
//! - Data type flavors
//! - Value set bindings
//! - Co-constraints

pub mod error;
pub mod predicate;
pub mod profile;
pub mod validator;

// Re-export main types
pub use error::{ConformanceError, Result};
pub use predicate::{Condition, PredicateEvaluator, PredicateParser};
pub use profile::{
    parser::ProfileParser, BindingStrength, Cardinality, CoConstraint, ComponentProfile,
    ConditionalUsage, ConformanceProfile, FieldProfile, MessageProfile, Predicate,
    ProfileMetadata, SegmentProfile, Usage, ValueSetBinding,
};
pub use validator::{
    ConformanceErrorType, ConformanceValidationResult, ConformanceValidator, Severity,
    ValidationLocation,
};
