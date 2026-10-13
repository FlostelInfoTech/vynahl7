//! Message filtering with predicate-based rules
//!
//! This module provides functionality for filtering HL7 messages based on
//! predicate rules. Messages that pass all filters are accepted, while
//! messages that fail any filter are rejected.
//!
//! ## Example
//!
//! ```rust
//! use vyna_hl7_orchestration::filtering::MessageFilter;
//! use vyna_hl7_core::Message;
//!
//! let mut filter = MessageFilter::new();
//!
//! // Only allow production messages
//! filter.add_rule("production_only", |msg| {
//!     use vyna_hl7_terser::Terser;
//!     let terser = Terser::new(msg);
//!     terser.get("MSH-11").ok().flatten().as_deref() == Some("P")
//! });
//!
//! // Only allow ADT messages
//! filter.add_rule("adt_only", |msg| {
//!     use vyna_hl7_terser::Terser;
//!     let terser = Terser::new(msg);
//!     terser.get("MSH-9-1").ok().flatten().map(|v| v.contains("ADT")).unwrap_or(false)
//! });
//! ```

use crate::error::{OrchestrationError, Result};
use vyna_hl7_core::Message;
use std::sync::Arc;

/// Type alias for filter rule functions
pub type FilterRule = Arc<dyn Fn(&Message) -> bool + Send + Sync>;

/// A single filter rule
pub struct Filter {
    /// Filter name
    pub name: String,
    /// Rule function
    rule: FilterRule,
}

impl Filter {
    /// Create a new filter
    pub fn new<F>(name: impl Into<String>, rule: F) -> Self
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
    {
        Self {
            name: name.into(),
            rule: Arc::new(rule),
        }
    }

    /// Check if the message passes this filter
    pub fn matches(&self, message: &Message) -> bool {
        (self.rule)(message)
    }
}

/// Message filter that applies multiple filter rules
///
/// All rules must pass for the message to be accepted.
pub struct MessageFilter {
    filters: Vec<Filter>,
    mode: FilterMode,
}

/// Filter mode determines how multiple filters are combined
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    /// All filters must pass (AND logic)
    All,
    /// At least one filter must pass (OR logic)
    Any,
}

impl MessageFilter {
    /// Create a new message filter with ALL mode (all filters must pass)
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            mode: FilterMode::All,
        }
    }

    /// Create a new message filter with ANY mode (at least one filter must pass)
    pub fn new_any() -> Self {
        Self {
            filters: Vec::new(),
            mode: FilterMode::Any,
        }
    }

    /// Set the filter mode
    pub fn with_mode(mut self, mode: FilterMode) -> Self {
        self.mode = mode;
        self
    }

    /// Add a filter rule
    pub fn add_rule<F>(&mut self, name: impl Into<String>, rule: F)
    where
        F: Fn(&Message) -> bool + Send + Sync + 'static,
    {
        self.filters.push(Filter::new(name, rule));
    }

    /// Get the number of filters
    pub fn filter_count(&self) -> usize {
        self.filters.len()
    }

    /// Check if a message passes all filters
    pub fn matches(&self, message: &Message) -> bool {
        if self.filters.is_empty() {
            return true; // No filters means everything passes
        }

        match self.mode {
            FilterMode::All => {
                // All filters must pass
                self.filters.iter().all(|filter| filter.matches(message))
            }
            FilterMode::Any => {
                // At least one filter must pass
                self.filters.iter().any(|filter| filter.matches(message))
            }
        }
    }

    /// Filter a message, returning Ok if it passes all filters
    pub fn filter(&self, message: &Message) -> Result<()> {
        if self.matches(message) {
            Ok(())
        } else {
            // A mismatch always names its failed filters: ALL mode fails at
            // least one, ANY mode fails all of them.
            let failed_filters: Vec<&str> = self
                .filters
                .iter()
                .filter(|f| !f.matches(message))
                .map(|f| f.name.as_str())
                .collect();

            Err(OrchestrationError::filter_failed(
                "MessageFilter",
                format!("Failed filters: {}", failed_filters.join(", ")),
            ))
        }
    }

    /// Clear all filters
    pub fn clear(&mut self) {
        self.filters.clear();
    }
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::{Field, Segment};

    fn create_production_adt_message() -> Message {
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msh.fields.push(Field::from_value("SendingApp"));
        msh.fields.push(Field::from_value("SendingFac"));
        msh.fields.push(Field::from_value("ReceivingApp"));
        msh.fields.push(Field::from_value("ReceivingFac"));
        msh.fields.push(Field::from_value("20231101120000"));
        msh.fields.push(Field::from_value(""));
        msh.fields.push(Field::from_value("ADT^A01"));
        msh.fields.push(Field::from_value("MSG001"));
        msh.fields.push(Field::from_value("P")); // Production
        msg.segments.push(msh);
        msg
    }

    fn create_test_oru_message() -> Message {
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msh.fields.push(Field::from_value("LabSystem"));
        msh.fields.push(Field::from_value("Hospital"));
        msh.fields.push(Field::from_value("ReceivingApp"));
        msh.fields.push(Field::from_value("ReceivingFac"));
        msh.fields.push(Field::from_value("20231101120000"));
        msh.fields.push(Field::from_value(""));
        msh.fields.push(Field::from_value("ORU^R01"));
        msh.fields.push(Field::from_value("MSG002"));
        msh.fields.push(Field::from_value("T")); // Test
        msg.segments.push(msh);
        msg
    }

    #[test]
    fn test_message_filter_all_mode() {
        let mut filter = MessageFilter::new();

        // Production only
        filter.add_rule("production", |msg| {
            use vyna_hl7_terser::Terser;
            let terser = Terser::new(msg);
            terser.get("MSH-11").ok().flatten() == Some("P")
        });

        // ADT only
        filter.add_rule("adt", |msg| {
            use vyna_hl7_terser::Terser;
            let terser = Terser::new(msg);
            terser
                .get("MSH-9-1")
                .ok()
                .flatten()
                .map(|v| v.contains("ADT"))
                .unwrap_or(false)
        });

        let prod_adt = create_production_adt_message();
        let test_oru = create_test_oru_message();

        assert!(filter.matches(&prod_adt)); // Passes both filters
        assert!(!filter.matches(&test_oru)); // Fails both filters
    }

    #[test]
    fn test_message_filter_any_mode() {
        let mut filter = MessageFilter::new_any();

        // Production OR Test
        filter.add_rule("production", |msg| {
            use vyna_hl7_terser::Terser;
            let terser = Terser::new(msg);
            terser.get("MSH-11").ok().flatten() == Some("P")
        });

        filter.add_rule("test", |msg| {
            use vyna_hl7_terser::Terser;
            let terser = Terser::new(msg);
            terser.get("MSH-11").ok().flatten() == Some("T")
        });

        let prod_adt = create_production_adt_message();
        let test_oru = create_test_oru_message();

        assert!(filter.matches(&prod_adt)); // Passes "production" filter
        assert!(filter.matches(&test_oru)); // Passes "test" filter
    }

    #[test]
    fn test_filter_result() {
        let mut filter = MessageFilter::new();

        filter.add_rule("production", |msg| {
            use vyna_hl7_terser::Terser;
            let terser = Terser::new(msg);
            terser.get("MSH-11").ok().flatten() == Some("P")
        });

        let prod_adt = create_production_adt_message();
        let test_oru = create_test_oru_message();

        assert!(filter.filter(&prod_adt).is_ok());
        assert!(filter.filter(&test_oru).is_err());
    }

    #[test]
    fn test_filter_count() {
        let mut filter = MessageFilter::new();
        assert_eq!(filter.filter_count(), 0);

        filter.add_rule("test", |_| true);
        assert_eq!(filter.filter_count(), 1);

        filter.clear();
        assert_eq!(filter.filter_count(), 0);
    }

    #[test]
    fn test_empty_filter() {
        let filter = MessageFilter::new();
        let msg = create_production_adt_message();

        // Empty filter should pass everything
        assert!(filter.matches(&msg));
        assert!(filter.filter(&msg).is_ok());
    }
}
