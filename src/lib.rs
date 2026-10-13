//! # VynaHL7 - HL7 v2.x Library for Rust
//!
//! A comprehensive Rust library for parsing, validating, and creating HL7 v2.x healthcare messages.
//!
//! ## Features
//!
//! - **Parsing and Serialization**: Parse HL7 pipe-delimited messages and serialize back
//! - **Multiple HL7 Versions**: v2.1 through v2.8.2 (2.1, 2.2, 2.3, 2.3.1, 2.4, 2.5,
//!   2.5.1, 2.6, 2.7, 2.7.1, 2.8, 2.8.1, 2.8.2)
//! - **Message Validation**: JSON schemas per HL7 version (e.g. 47 message schemas
//!   for v2.5 covering ADT, ORU, ORM, SIU, MDM, DFT, QRY, BAR, pharmacy, laboratory,
//!   immunization, problem, and acknowledgment messages)
//! - **Terser API**: Easy field access using path notation
//! - **MLLP Support**: Network transmission protocol for intra-organization messaging
//! - **HTTP Support**: HL7-over-HTTP transport for inter-organization messaging
//!
//! ## Supported Message Types
//!
//! Message schemas ship per HL7 version (23 for v2.1 up to 47 for v2.4-v2.6).
//! The v2.5 set covers:
//!
//! - **ADT** (18 schemas): A01-A13, A17, A28, A31, A34, A40
//! - **SIU** (4 schemas): S12-S15 (Scheduling)
//! - **MDM** (3 schemas): T01, T02, T04 (Medical Documents)
//! - **DFT** (2 schemas): P03, P11 (Financial Transactions)
//! - **QRY** (3 schemas): A19, Q01, Q02 (Queries)
//! - **BAR** (2 schemas): P01, P02 (Billing Accounts)
//! - **Pharmacy** (6 schemas): RDE O11, RAS O17, RDS O13, RGV O15, RRD O14, RRA O18
//! - **Laboratory/Orders** (4 schemas): OUL R21, OML O21, OMP O09, PPR PC1
//! - **ORU** (1 schema): R01 (Observation Results)
//! - **ORM** (1 schema): O01 (Orders)
//! - **VXU** (1 schema): V04 (Immunization)
//! - **MFN** (1 schema): M01 (Master File Notification)
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
pub use vyna_hl7_http as http;
pub use vyna_hl7_mllp as mllp;
pub use vyna_hl7_parser as parser;
pub use vyna_hl7_terser as terser;
pub use vyna_hl7_validator as validator;

// Re-export commonly used types
pub use vyna_hl7_core::{
    Version,
    delimiters::Delimiters,
    encoding::Encoding,
    error::{Error, Result},
    field::{Component, Field, Repetition, SubComponent},
    message::Message,
    segment::Segment,
};
