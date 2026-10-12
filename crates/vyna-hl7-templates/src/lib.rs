//! Template system for HL7 v2.x messages
//!
//! This crate provides a comprehensive template system for creating and validating
//! HL7 v2.x messages. Templates allow you to define reusable message patterns with
//! variable substitution, inheritance, and validation.
//!
//! # Features
//!
//! - **Message Templates**: Reusable message patterns with segment and field definitions
//! - **Template Engine**: Create messages from templates with variable substitution
//! - **Template Validation**: Validate messages against template definitions
//! - **YAML/JSON Support**: Load and save templates from/to YAML and JSON files
//! - **Standard Library**: Pre-built templates for common message types (ADT, ORU, ORM, etc.)
//! - **Template Inheritance**: Extend base templates to create specialized variants
//! - **Variable Substitution**: Use `{{variable}}` placeholders for dynamic values
//!
//! # Quick Start
//!
//! ## Creating a Message from a Template
//!
//! ```
//! use vyna_hl7_templates::{TemplateEngine, TemplateLibrary};
//!
//! // Get a standard template from the library
//! let library = TemplateLibrary::new();
//! let template = library.get("ADT_A01").unwrap();
//!
//! // Create an engine and set variables
//! let mut engine = TemplateEngine::new();
//! engine.set_variable("sending_app", "MyApp");
//! engine.set_variable("sending_facility", "MyHospital");
//! engine.set_variable("receiving_app", "CentralSystem");
//! engine.set_variable("receiving_facility", "DataCenter");
//! engine.set_variable("patient_id", "12345");
//! engine.set_variable("patient_name", "Doe^John");
//! engine.set_variable("date_of_birth", "19900101");
//! engine.set_variable("sex", "M");
//! engine.set_variable("address", "123 Main St^^Springfield^IL^62701");
//! engine.set_variable("event_datetime", "20250120120000");
//! engine.set_variable("patient_class", "I");
//! engine.set_variable("assigned_location", "4N^401");
//! engine.set_variable("attending_doctor", "12345^Smith^Jane");
//! engine.set_variable("visit_number", "V123456");
//!
//! // Create the message
//! let message = engine.create_message(template).unwrap();
//! ```
//!
//! ## Loading a Template from YAML
//!
//! ```
//! use vyna_hl7_templates::MessageTemplate;
//!
//! let yaml = r#"
//! name: "Custom ADT"
//! version: "2.5"
//! message_type: "ADT"
//! trigger_event: "A01"
//! segments:
//!   - id: "MSH"
//!     required: true
//!     fields:
//!       3:
//!         required: true
//!         placeholder: "{{app}}"
//!   - id: "PID"
//!     required: true
//!     fields:
//!       3:
//!         required: true
//!         placeholder: "{{patient_id}}"
//! "#;
//!
//! let template = MessageTemplate::from_yaml(yaml).unwrap();
//! ```
//!
//! ## Validating a Message Against a Template
//!
//! ```
//! use vyna_hl7_templates::{TemplateValidator, MessageTemplate};
//! use vyna_hl7_core::Message;
//!
//! let template = MessageTemplate::new("Test", "2.5", "ADT", "A01");
//! let message = Message::new();
//!
//! let result = TemplateValidator::validate(&message, &template);
//! if result.valid {
//!     println!("Message is valid!");
//! } else {
//!     for error in result.errors {
//!         println!("Error: {}", error.message);
//!     }
//! }
//! ```
//!
//! ## Template Inheritance
//!
//! ```
//! use vyna_hl7_templates::{MessageTemplate, TemplateResolver, SegmentTemplate};
//!
//! // Base template
//! let base = MessageTemplate::new("BaseADT", "2.5", "ADT", "A01")
//!     .with_segment(SegmentTemplate::new("MSH").required())
//!     .with_segment(SegmentTemplate::new("PID").required());
//!
//! // Extended template
//! let extended = MessageTemplate::new("ExtendedADT", "2.5", "ADT", "A01")
//!     .with_extends("BaseADT")
//!     .with_segment(SegmentTemplate::new("PV1").required());
//!
//! // Resolve inheritance
//! let mut resolver = TemplateResolver::new();
//! resolver.register(base);
//! resolver.register(extended);
//!
//! let resolved = resolver.resolve("ExtendedADT").unwrap();
//! // resolved now contains MSH, PID (from base), and PV1 (from extended)
//! ```

mod engine;
mod error;
mod inheritance;
mod library;
mod parser;
mod template;
mod validation;

// Re-export public API
pub use engine::TemplateEngine;
pub use error::{Error, Result};
pub use inheritance::TemplateResolver;
pub use library::TemplateLibrary;
pub use template::{ComponentTemplate, FieldTemplate, MessageTemplate, SegmentTemplate};
pub use validation::{TemplateValidator, ValidationError, ValidationResult, ValidationWarning};
