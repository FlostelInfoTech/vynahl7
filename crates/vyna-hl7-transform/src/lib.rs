//! Message transformation framework for HL7 v2.x messages
//!
//! This crate provides a flexible framework for transforming HL7 messages:
//! - Field-to-field mappings
//! - Built-in transformation functions
//! - Custom user-defined transforms
//! - Declarative YAML/JSON configuration (with `serde` feature)
//! - Message type and version transformations
//!
//! # Examples
//!
//! ```rust
//! use vyna_hl7_transform::{MessageTransformer, transforms};
//! use vyna_hl7_core::Message;
//!
//! // Create a transformer
//! let mut transformer = MessageTransformer::new();
//!
//! // Add simple field mapping
//! transformer.add_mapping("PID-5-1", "PID-5-1"); // Copy family name
//!
//! // Add mapping with transformation
//! transformer.add_transform("PID-5-1", "PID-5-1", transforms::uppercase);
//!
//! // Transform a message
//! # let source = Message::new();
//! let result = transformer.transform(&source);
//! ```

pub mod error;
pub mod rule;
pub mod transformer;
pub mod transforms;

#[cfg(feature = "serde")]
pub mod config;

pub use error::{Error, Result};
pub use rule::{TransformContext, TransformFn, TransformationRule};
pub use transformer::MessageTransformer;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::rule::{TransformContext, TransformFn, TransformationRule};
    pub use crate::transformer::MessageTransformer;
    pub use crate::transforms;

    #[cfg(feature = "serde")]
    pub use crate::config::{TransformConfig, RuleConfig};
}
