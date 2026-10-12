//! # VynaHL7 - HL7 v2.x Library for Rust
//!
//! A comprehensive Rust library for parsing, validating, and creating HL7 v2.x healthcare messages.
//!
//! ## Features
//!
//! - **Parsing and Serialization**: Parse HL7 pipe-delimited messages and serialize back
//! - **Multiple HL7 Versions**: Support for v2.3, v2.4, v2.5, v2.6, v2.7, v2.8
//! - **Message Validation**: 43 message schemas across all HL7 versions (v2.3-v2.8)
//! - **Terser API**: Easy field access using path notation
//! - **MLLP Support**: Network transmission protocol
//!
//! ## Supported Message Types
//!
//! - **ADT** (17 schemas): A01-A13, A17, A28, A31, A40
//! - **SIU** (4 schemas): S12-S15 (Scheduling)
//! - **MDM** (3 schemas): T01, T02, T04 (Medical Documents)
//! - **DFT** (2 schemas): P03, P11 (Financial Transactions)
//! - **QRY** (3 schemas): A19, Q01, Q02 (Queries)
//! - **ORU** (1 schema): R01 (Observation Results)
//! - **ORM** (1 schema): O01 (Orders)
//! - **ACK** (1 schema): General Acknowledgment
//!
//! ## Quick Start
//!
//! ```rust
//! use vyna_hl7::parser::parse_message;
//! use vyna_hl7::terser::Terser;
//!
//! let hl7 = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5
//! PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M";
//!
//! // Parse the message
//! let message = parse_message(hl7).unwrap();
//!
//! // Access fields using Terser
//! let terser = Terser::new(&message);
//! let family_name = terser.get("PID-5-1").unwrap();  // "DOE"
//! ```

pub use vyna_hl7_core as core;
pub use vyna_hl7_parser as parser;
pub use vyna_hl7_terser as terser;
pub use vyna_hl7_validator as validator;
pub use vyna_hl7_mllp as mllp;

// Re-export commonly used types
pub use vyna_hl7_core::{
    delimiters::Delimiters,
    encoding::Encoding,
    error::{Error, Result},
    field::{Component, Field, Repetition, SubComponent},
    message::Message,
    segment::Segment,
    Version,
};
