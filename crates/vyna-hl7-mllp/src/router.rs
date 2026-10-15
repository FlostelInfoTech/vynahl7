//! Application Router for MLLP servers
//!
//! This module provides routing capabilities for MLLP servers, allowing messages
//! to be routed to different handlers based on message type and trigger event.
//!
//! # Overview
//!
//! The router follows patterns similar to HAPI's ApplicationRouter, enabling:
//! - Route messages by message type (e.g., "ADT", "ORU")
//! - Route messages by specific trigger event (e.g., "ADT^A01")
//! - Wildcard matching for trigger events
//! - Default handler for unmatched messages
//! - Sync handlers with automatic ACK generation
//!
//! # Examples
//!
//! ## Basic Routing
//!
//! ```rust
//! use vyna_hl7_mllp::router::{MessageRouter, RouteResult};
//! use vyna_hl7_core::Message;
//!
//! let mut router = MessageRouter::new();
//!
//! // Route all ADT messages
//! router.route("ADT", "*", |msg| {
//!     println!("Received ADT message");
//!     RouteResult::Ack
//! });
//!
//! // Route specific trigger event
//! router.route("ORU", "R01", |msg| {
//!     println!("Received lab result");
//!     RouteResult::Ack
//! });
//!
//! // Default handler
//! router.route_default(|msg| {
//!     println!("Received unknown message type");
//!     RouteResult::Reject("Unknown message type".to_string())
//! });
//! ```
//!
//! ## With MLLP Server
//!
//! ```rust,no_run
//! use vyna_hl7_mllp::{MllpServer, router::MessageRouter};
//!
//! # async fn example() -> vyna_hl7_core::error::Result<()> {
//! let mut router = MessageRouter::new();
//! // ... configure routes ...
//!
//! let server = MllpServer::bind("0.0.0.0:2575").await?;
//!
//! loop {
//!     let mut conn = server.accept().await?;
//!     let router = router.clone();
//!
//!     tokio::spawn(async move {
//!         if let Ok(msg) = conn.receive_message().await {
//!             if let Some(response) = router.handle(&msg) {
//!                 let _ = conn.send_message(&response).await;
//!             }
//!         }
//!     });
//! }
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use vyna_hl7_core::{Message, builders::ack::AckBuilder};

/// Result of handling a routed message
#[derive(Debug, Clone)]
pub enum RouteResult {
    /// Accept the message (generates AA acknowledgment)
    Ack,
    /// Accept with a custom message
    AckWithMessage(String),
    /// Error processing the message (generates AE acknowledgment)
    Error(String),
    /// Error with detailed error info
    ErrorWithCode(String, String, String), // (message, code, description)
    /// Reject the message (generates AR acknowledgment)
    Reject(String),
    /// Return a custom response message
    Custom(Message),
    /// No response (for fire-and-forget scenarios)
    NoResponse,
}

/// Type alias for sync message handlers
pub type SyncHandler = Arc<dyn Fn(&Message) -> RouteResult + Send + Sync + 'static>;

/// A route definition
#[derive(Clone)]
struct Route {
    /// Message type (e.g., "ADT", "ORU")
    message_type: String,
    /// Trigger event (e.g., "A01", "R01") or "*" for wildcard
    trigger_event: String,
    /// Handler function
    handler: SyncHandler,
    /// Route priority (higher = checked first)
    priority: i32,
}

/// Message router for MLLP servers
///
/// Routes incoming HL7 messages to appropriate handlers based on
/// message type and trigger event.
#[derive(Clone)]
pub struct MessageRouter {
    /// Registered routes
    routes: Vec<Route>,
    /// Default handler for unmatched messages
    default_handler: Option<SyncHandler>,
    /// Whether to auto-generate ACK for unhandled messages
    auto_ack_unhandled: bool,
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            default_handler: None,
            auto_ack_unhandled: true,
        }
    }

    /// Configure whether to auto-acknowledge unhandled messages
    ///
    /// If true (default), messages that don't match any route and have no
    /// default handler will receive an AR (reject) acknowledgment.
    /// If false, no response will be sent.
    pub fn auto_ack_unhandled(mut self, enabled: bool) -> Self {
        self.auto_ack_unhandled = enabled;
        self
    }

    /// Add a route for a specific message type and trigger event
    ///
    /// # Arguments
    ///
    /// * `message_type` - The message type (e.g., "ADT", "ORU", "ORM")
    /// * `trigger_event` - The trigger event (e.g., "A01", "R01") or "*" for all
    /// * `handler` - Function to handle matching messages
    ///
    /// # Example
    ///
    /// ```rust
    /// use vyna_hl7_mllp::router::{MessageRouter, RouteResult};
    ///
    /// let mut router = MessageRouter::new();
    ///
    /// // Handle all ADT messages
    /// router.route("ADT", "*", |msg| {
    ///     RouteResult::Ack
    /// });
    ///
    /// // Handle specific ORU^R01 messages
    /// router.route("ORU", "R01", |msg| {
    ///     RouteResult::Ack
    /// });
    /// ```
    pub fn route<F>(&mut self, message_type: &str, trigger_event: &str, handler: F)
    where
        F: Fn(&Message) -> RouteResult + Send + Sync + 'static,
    {
        self.route_with_priority(message_type, trigger_event, 0, handler);
    }

    /// Add a route with a specific priority
    ///
    /// Higher priority routes are checked first. Use this when you need
    /// fine-grained control over route matching order.
    pub fn route_with_priority<F>(
        &mut self,
        message_type: &str,
        trigger_event: &str,
        priority: i32,
        handler: F,
    ) where
        F: Fn(&Message) -> RouteResult + Send + Sync + 'static,
    {
        let route = Route {
            message_type: message_type.to_uppercase(),
            trigger_event: trigger_event.to_uppercase(),
            handler: Arc::new(handler),
            priority,
        };

        // Insert in priority order (higher priority first)
        let insert_pos = self
            .routes
            .iter()
            .position(|r| r.priority < priority)
            .unwrap_or(self.routes.len());
        self.routes.insert(insert_pos, route);
    }

    /// Set the default handler for messages that don't match any route
    ///
    /// # Example
    ///
    /// ```rust
    /// use vyna_hl7_mllp::router::{MessageRouter, RouteResult};
    ///
    /// let mut router = MessageRouter::new();
    ///
    /// router.route_default(|msg| {
    ///     RouteResult::Reject("Unknown message type".to_string())
    /// });
    /// ```
    pub fn route_default<F>(&mut self, handler: F)
    where
        F: Fn(&Message) -> RouteResult + Send + Sync + 'static,
    {
        self.default_handler = Some(Arc::new(handler));
    }

    /// Handle an incoming message
    ///
    /// Routes the message to the appropriate handler and generates
    /// an acknowledgment response.
    ///
    /// # Returns
    ///
    /// - `Some(Message)` - The response message (usually an ACK)
    /// - `None` - No response should be sent
    pub fn handle(&self, message: &Message) -> Option<Message> {
        // Get message type info
        let (msg_type, trigger) = message.get_message_type().unwrap_or_default();
        let msg_type = msg_type.to_uppercase();
        let trigger = trigger.to_uppercase();

        // Find matching route
        let handler = self.find_route(&msg_type, &trigger);

        // Execute handler or use default
        let result = if let Some(h) = handler {
            h(message)
        } else if let Some(ref default) = self.default_handler {
            default(message)
        } else if self.auto_ack_unhandled {
            RouteResult::Reject(format!(
                "No handler registered for message type {}^{}",
                msg_type, trigger
            ))
        } else {
            return None;
        };

        // Generate response
        self.generate_response(message, result)
    }

    /// Find a matching route for the given message type and trigger
    fn find_route(&self, message_type: &str, trigger_event: &str) -> Option<&SyncHandler> {
        for route in &self.routes {
            let type_match = route.message_type == message_type || route.message_type == "*";
            let trigger_match = route.trigger_event == trigger_event || route.trigger_event == "*";

            if type_match && trigger_match {
                return Some(&route.handler);
            }
        }
        None
    }

    /// Generate a response message from a route result
    fn generate_response(&self, original: &Message, result: RouteResult) -> Option<Message> {
        match result {
            RouteResult::Ack => AckBuilder::for_message(original).accept().build().ok(),

            RouteResult::AckWithMessage(msg) => AckBuilder::for_message(original)
                .accept()
                .text_message(&msg)
                .build()
                .ok(),

            RouteResult::Error(msg) => AckBuilder::for_message(original).error(&msg).build().ok(),

            RouteResult::ErrorWithCode(msg, code, desc) => AckBuilder::for_message(original)
                .error(&msg)
                .error_code(&code, &desc)
                .build()
                .ok(),

            RouteResult::Reject(msg) => AckBuilder::for_message(original).reject(&msg).build().ok(),

            RouteResult::Custom(response) => Some(response),

            RouteResult::NoResponse => None,
        }
    }

    /// Get the number of registered routes
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Check if a route exists for the given message type and trigger
    pub fn has_route(&self, message_type: &str, trigger_event: &str) -> bool {
        self.find_route(&message_type.to_uppercase(), &trigger_event.to_uppercase())
            .is_some()
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.routes.clear();
        self.default_handler = None;
    }
}

/// Builder for creating message routers with a fluent API
pub struct MessageRouterBuilder {
    router: MessageRouter,
}

impl MessageRouterBuilder {
    /// Create a new router builder
    pub fn new() -> Self {
        Self {
            router: MessageRouter::new(),
        }
    }

    /// Add a route
    pub fn route<F>(mut self, message_type: &str, trigger_event: &str, handler: F) -> Self
    where
        F: Fn(&Message) -> RouteResult + Send + Sync + 'static,
    {
        self.router.route(message_type, trigger_event, handler);
        self
    }

    /// Set the default handler
    pub fn default_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&Message) -> RouteResult + Send + Sync + 'static,
    {
        self.router.route_default(handler);
        self
    }

    /// Set auto-ack behavior
    pub fn auto_ack_unhandled(mut self, enabled: bool) -> Self {
        self.router.auto_ack_unhandled = enabled;
        self
    }

    /// Build the router
    pub fn build(self) -> MessageRouter {
        self.router
    }
}

impl Default for MessageRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience trait for routing messages
pub trait Routable {
    /// Get the message type for routing
    fn route_key(&self) -> Option<(String, String)>;
}

impl Routable for Message {
    fn route_key(&self) -> Option<(String, String)> {
        self.get_message_type()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use vyna_hl7_core::{field::Field, segment::Segment};

    fn create_test_message(msg_type: &str, trigger: &str) -> Message {
        use vyna_hl7_core::field::{Component, Repetition};

        let mut msg = Message::new();
        let mut msh = Segment::new("MSH");

        msh.add_field(Field::from_value("|"));
        msh.add_field(Field::from_value("^~\\&"));
        msh.add_field(Field::from_value("SendApp"));
        msh.add_field(Field::from_value("SendFac"));
        msh.add_field(Field::from_value("RecApp"));
        msh.add_field(Field::from_value("RecFac"));
        msh.add_field(Field::from_value("20240315120000"));
        msh.add_field(Field::from_value(""));

        // MSH-9: Message Type with proper component structure
        let mut msg_type_field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(msg_type));
        rep.add_component(Component::from_value(trigger));
        msg_type_field.add_repetition(rep);
        msh.add_field(msg_type_field);

        msh.add_field(Field::from_value("MSG001"));
        msh.add_field(Field::from_value("P"));
        msh.add_field(Field::from_value("2.5"));

        msg.add_segment(msh);
        msg
    }

    #[test]
    fn test_basic_routing() {
        let mut router = MessageRouter::new();

        router.route("ADT", "A01", |_msg| RouteResult::Ack);

        let msg = create_test_message("ADT", "A01");
        let response = router.handle(&msg);

        assert!(response.is_some());
        let ack = response.unwrap();
        assert_eq!(ack.segment("MSA").unwrap().get_field_value(1), Some("AA"));
    }

    #[test]
    fn test_wildcard_trigger() {
        let mut router = MessageRouter::new();

        router.route("ADT", "*", |_msg| RouteResult::Ack);

        // Should match any ADT message
        let msg_a01 = create_test_message("ADT", "A01");
        let msg_a08 = create_test_message("ADT", "A08");

        assert!(router.handle(&msg_a01).is_some());
        assert!(router.handle(&msg_a08).is_some());
    }

    #[test]
    fn test_specific_route_priority() {
        let mut router = MessageRouter::new();
        let counter = Arc::new(AtomicUsize::new(0));

        // Add wildcard route first (lower priority)
        let c1 = counter.clone();
        router.route("ADT", "*", move |_msg| {
            c1.store(1, Ordering::SeqCst);
            RouteResult::Ack
        });

        // Add specific route with higher priority
        let c2 = counter.clone();
        router.route_with_priority("ADT", "A01", 10, move |_msg| {
            c2.store(2, Ordering::SeqCst);
            RouteResult::Ack
        });

        let msg = create_test_message("ADT", "A01");
        router.handle(&msg);

        // Should have called the specific route (value 2) due to higher priority
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_default_handler() {
        let mut router = MessageRouter::new();

        router.route_default(|_msg| RouteResult::Reject("Unknown type".to_string()));

        let msg = create_test_message("XYZ", "Z99");
        let response = router.handle(&msg);

        assert!(response.is_some());
        let ack = response.unwrap();
        assert_eq!(ack.segment("MSA").unwrap().get_field_value(1), Some("AR"));
    }

    #[test]
    fn test_error_response() {
        let mut router = MessageRouter::new();

        router.route("ADT", "A01", |_msg| {
            RouteResult::Error("Processing failed".to_string())
        });

        let msg = create_test_message("ADT", "A01");
        let response = router.handle(&msg);

        let ack = response.unwrap();
        assert_eq!(ack.segment("MSA").unwrap().get_field_value(1), Some("AE"));
    }

    #[test]
    fn test_custom_response() {
        let mut router = MessageRouter::new();

        router.route("QRY", "A19", |_msg| {
            // Return a custom response instead of ACK
            let mut response = Message::new();
            let mut msh = Segment::new("MSH");
            msh.add_field(Field::from_value("|"));
            msh.add_field(Field::from_value("^~\\&"));
            msh.add_field(Field::from_value("RSP"));
            response.add_segment(msh);
            RouteResult::Custom(response)
        });

        let msg = create_test_message("QRY", "A19");
        let response = router.handle(&msg);

        assert!(response.is_some());
        // Should be our custom response, not an ACK
        let resp = response.unwrap();
        assert_eq!(resp.segment("MSH").unwrap().get_field_value(3), Some("RSP"));
    }

    #[test]
    fn test_no_response() {
        let mut router = MessageRouter::new();

        router.route("ADT", "A01", |_msg| RouteResult::NoResponse);

        let msg = create_test_message("ADT", "A01");
        let response = router.handle(&msg);

        assert!(response.is_none());
    }

    #[test]
    fn test_auto_ack_disabled() {
        let router = MessageRouter::new().auto_ack_unhandled(false);

        let msg = create_test_message("XYZ", "Z99");
        let response = router.handle(&msg);

        // Should be None since auto-ack is disabled
        assert!(response.is_none());
    }

    #[test]
    fn test_route_count() {
        let mut router = MessageRouter::new();

        router.route("ADT", "A01", |_| RouteResult::Ack);
        router.route("ORU", "R01", |_| RouteResult::Ack);

        assert_eq!(router.route_count(), 2);
    }

    #[test]
    fn test_has_route() {
        let mut router = MessageRouter::new();

        router.route("ADT", "A01", |_| RouteResult::Ack);

        assert!(router.has_route("ADT", "A01"));
        assert!(router.has_route("adt", "a01")); // Case insensitive
        assert!(!router.has_route("ORU", "R01"));
    }

    #[test]
    fn test_builder() {
        let router = MessageRouterBuilder::new()
            .route("ADT", "A01", |_| RouteResult::Ack)
            .route("ORU", "R01", |_| RouteResult::Ack)
            .auto_ack_unhandled(false)
            .build();

        assert_eq!(router.route_count(), 2);
    }

    #[test]
    fn test_error_with_code() {
        let mut router = MessageRouter::new();

        router.route("ADT", "A01", |_msg| {
            RouteResult::ErrorWithCode(
                "Patient not found".to_string(),
                "204".to_string(),
                "Unknown Key Identifier".to_string(),
            )
        });

        let msg = create_test_message("ADT", "A01");
        let response = router.handle(&msg);

        let ack = response.unwrap();
        assert_eq!(ack.segment("MSA").unwrap().get_field_value(1), Some("AE"));
    }

    #[test]
    fn test_ack_with_message() {
        let mut router = MessageRouter::new();

        router.route("ADT", "A01", |_msg| {
            RouteResult::AckWithMessage("Message processed successfully".to_string())
        });

        let msg = create_test_message("ADT", "A01");
        let response = router.handle(&msg);

        let ack = response.unwrap();
        assert_eq!(ack.segment("MSA").unwrap().get_field_value(1), Some("AA"));
        assert_eq!(
            ack.segment("MSA").unwrap().get_field_value(3),
            Some("Message processed successfully")
        );
    }
}
