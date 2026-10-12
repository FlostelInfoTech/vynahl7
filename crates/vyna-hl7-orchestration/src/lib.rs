//! Message routing, orchestration, and workflow engine for HL7 v2.x messages
//!
//! This crate provides tools for building message processing pipelines with:
//! - **Content-Based Routing**: Route messages based on field values
//! - **Message Orchestration**: Multi-step async workflows
//! - **Message Filtering**: Predicate-based message filtering
//! - **Error Handling**: Retry logic and dead letter queues
//! - **Workflow Builder**: Fluent API for pipeline definition
//!
//! ## Features
//!
//! ### Content-Based Routing
//!
//! Route messages to different handlers based on message content:
//!
//! ```rust,no_run
//! use vyna_hl7_orchestration::routing::ContentRouter;
//! # use vyna_hl7_core::Message;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut router = ContentRouter::new();
//!
//! // Route ADT messages
//! router.add_route("adt_route", |msg| {
//!     use vyna_hl7_terser::Terser;
//!     let terser = Terser::new(msg);
//!     terser.get("MSH-9-1").ok().flatten().as_deref() == Some("ADT")
//! }, |msg| {
//!     Box::pin(async move {
//!         println!("Processing ADT message");
//!         Ok(msg.clone())
//!     })
//! });
//! # Ok(())
//! # }
//! ```
//!
//! ### Message Orchestration
//!
//! Build multi-step async workflows:
//!
//! ```rust,no_run
//! use vyna_hl7_orchestration::orchestration::MessageOrchestrator;
//! # use vyna_hl7_core::Message;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut orchestrator = MessageOrchestrator::new();
//!
//! orchestrator
//!     .add_step("validate", |msg| {
//!         Box::pin(async move {
//!             println!("Validating message");
//!             Ok(msg.clone())
//!         })
//!     })
//!     .add_step("transform", |msg| {
//!         Box::pin(async move {
//!             println!("Transforming message");
//!             Ok(msg.clone())
//!         })
//!     });
//!
//! # let message = Message::default();
//! orchestrator.execute(message).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Message Filtering
//!
//! Filter messages based on predicates:
//!
//! ```rust
//! use vyna_hl7_orchestration::filtering::MessageFilter;
//! # use vyna_hl7_core::Message;
//!
//! let mut filter = MessageFilter::new();
//!
//! // Only allow production messages
//! filter.add_rule("production_only", |msg| {
//!     use vyna_hl7_terser::Terser;
//!     let terser = Terser::new(msg);
//!     terser.get("MSH-11").ok().flatten().as_deref() == Some("P")
//! });
//! ```

pub mod routing;
pub mod orchestration;
pub mod filtering;
pub mod error;

pub use error::{OrchestrationError, Result};
