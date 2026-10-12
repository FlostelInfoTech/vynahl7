//! Custom Z-Segment Framework for VynaHL7
//!
//! This crate provides a framework for defining, registering, and parsing custom Z-segments
//! in HL7 v2.x messages. Z-segments are organization-specific extensions to the HL7 standard
//! that are used to transmit site-specific data.
//!
//! # Overview
//!
//! The framework provides:
//! - **Type-safe segment definitions** via the [`CustomSegment`] trait
//! - **Global registration system** via [`CustomSegmentRegistry`]
//! - **Fluent builders** for creating Z-segments programmatically
//! - **Validation hooks** for custom business rules
//! - **Zero overhead** for standard HL7 segments
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use vyna_hl7_custom::{CustomSegment, CustomSegmentRegistry, z_segment};
//!
//! // Define a custom Z-segment
//! z_segment! {
//!     ZPV,
//!     id = "ZPV",
//!     fields = {
//!         1 => visit_type: String,
//!         2 => visit_number: String,
//!         3 => patient_class: Option<String>,
//!     }
//! }
//!
//! // Register it
//! CustomSegmentRegistry::global().register::<ZPV>();
//!
//! // Use it
//! let zpv = ZPV::builder()
//!     .visit_type("OUTPATIENT")
//!     .visit_number("V12345")
//!     .build()?;
//! ```
//!
//! # Architecture
//!
//! The framework uses a trait-based design where each Z-segment implements the
//! [`CustomSegment`] trait. Segments are registered in a global [`CustomSegmentRegistry`]
//! which can be queried by the parser when encountering unknown segment IDs.
//!
//! # Feature Flags
//!
//! This crate currently has no optional features.

pub mod error;
pub mod message_ext;
pub mod registry;
pub mod segment;

#[macro_use]
pub mod macros;

// Re-exports
pub use error::{CustomSegmentError, Result};
pub use message_ext::MessageExt;
pub use registry::{CustomSegmentRegistry, FactoryInfo};
pub use segment::{
    BuildableField, BuilderField, CustomSegment, FieldDefinition, ParseSegmentField,
    SerializeSegmentField,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        CustomSegment, CustomSegmentError, CustomSegmentRegistry, FieldDefinition, MessageExt,
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_structure() {
        // Basic smoke test to ensure modules compile
        assert_eq!(2 + 2, 4);
    }
}
