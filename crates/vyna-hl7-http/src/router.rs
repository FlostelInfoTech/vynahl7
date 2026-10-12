//! Message routing based on HL7 message types and trigger events
//!
//! This module provides flexible routing of HL7 messages to different handlers
//! based on message type (e.g., ADT, ORU) and trigger event (e.g., A01, R01).

#[cfg(feature = "routing")]
use crate::{Error, Result};
#[cfg(feature = "routing")]
use vyna_hl7_core::Message;
#[cfg(feature = "routing")]
use vyna_hl7_terser::Terser;
#[cfg(feature = "routing")]
use std::collections::HashMap;
#[cfg(feature = "routing")]
use std::sync::Arc;

/// Message handler function type for routing
#[cfg(feature = "routing")]
pub type RouteHandler = Arc<dyn Fn(Message) -> Result<Message> + Send + Sync>;

/// Message router for directing messages to appropriate handlers
///
/// Routes messages based on message type and trigger event extracted from MSH-9.
#[cfg(feature = "routing")]
pub struct MessageRouter {
    routes: HashMap<String, RouteHandler>,
    default_handler: Option<RouteHandler>,
}

#[cfg(feature = "routing")]
impl MessageRouter {
    /// Create a new message router
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "routing")]
    /// # {
    /// use vyna_hl7_http::router::MessageRouter;
    ///
    /// let router = MessageRouter::new();
    /// # }
    /// ```
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            default_handler: None,
        }
    }

    /// Add a route for a specific message type and trigger event
    ///
    /// # Arguments
    ///
    /// * `message_type` - The message type (e.g., "ADT", "ORU")
    /// * `trigger_event` - The trigger event (e.g., "A01", "R01")
    /// * `handler` - The handler function for this route
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "routing")]
    /// # {
    /// use vyna_hl7_http::router::MessageRouter;
    /// use std::sync::Arc;
    ///
    /// let mut router = MessageRouter::new();
    /// router.add_route("ADT", "A01", Arc::new(|msg| {
    ///     println!("Handling ADT^A01");
    ///     Ok(msg)
    /// }));
    /// # }
    /// ```
    pub fn add_route(&mut self, message_type: &str, trigger_event: &str, handler: RouteHandler) {
        let key = format!("{}^{}", message_type, trigger_event);
        self.routes.insert(key, handler);
    }

    /// Add a route for all trigger events of a message type
    ///
    /// # Arguments
    ///
    /// * `message_type` - The message type (e.g., "ADT")
    /// * `handler` - The handler function for this message type
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "routing")]
    /// # {
    /// use vyna_hl7_http::router::MessageRouter;
    /// use std::sync::Arc;
    ///
    /// let mut router = MessageRouter::new();
    /// router.add_type_route("ADT", Arc::new(|msg| {
    ///     println!("Handling any ADT message");
    ///     Ok(msg)
    /// }));
    /// # }
    /// ```
    pub fn add_type_route(&mut self, message_type: &str, handler: RouteHandler) {
        let key = format!("{}^*", message_type);
        self.routes.insert(key, handler);
    }

    /// Set the default handler for unmatched messages
    ///
    /// # Arguments
    ///
    /// * `handler` - The default handler function
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "routing")]
    /// # {
    /// use vyna_hl7_http::router::MessageRouter;
    /// use std::sync::Arc;
    ///
    /// let mut router = MessageRouter::new();
    /// router.set_default_handler(Arc::new(|msg| {
    ///     println!("Unmatched message");
    ///     Ok(msg)
    /// }));
    /// # }
    /// ```
    pub fn set_default_handler(&mut self, handler: RouteHandler) {
        self.default_handler = Some(handler);
    }

    /// Route a message to the appropriate handler
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message to route
    ///
    /// # Returns
    ///
    /// The result from the matched handler, or an error if no handler matches
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "routing")]
    /// # {
    /// use vyna_hl7_http::router::MessageRouter;
    /// use vyna_hl7_core::Message;
    ///
    /// # fn example(router: MessageRouter, message: Message) -> Result<(), Box<dyn std::error::Error>> {
    /// let response = router.route(message)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn route(&self, message: Message) -> Result<Message> {
        let terser = Terser::new(&message);

        let message_type = terser
            .get("MSH-9-1")
            .ok()
            .flatten()
            .unwrap_or("UNKNOWN");

        let trigger_event = terser
            .get("MSH-9-2")
            .ok()
            .flatten()
            .unwrap_or("UNKNOWN");

        // Try exact match first (e.g., "ADT^A01")
        let exact_key = format!("{}^{}", message_type, trigger_event);
        if let Some(handler) = self.routes.get(&exact_key) {
            return handler(message);
        }

        // Try type wildcard match (e.g., "ADT^*")
        let type_key = format!("{}^*", message_type);
        if let Some(handler) = self.routes.get(&type_key) {
            return handler(message);
        }

        // Use default handler if available
        if let Some(handler) = &self.default_handler {
            return handler(message);
        }

        // No handler found
        Err(Error::Http(format!(
            "No route found for message type: {}^{}",
            message_type, trigger_event
        )))
    }

    /// Check if a route exists for a message type and trigger event
    ///
    /// # Arguments
    ///
    /// * `message_type` - The message type
    /// * `trigger_event` - The trigger event
    pub fn has_route(&self, message_type: &str, trigger_event: &str) -> bool {
        let exact_key = format!("{}^{}", message_type, trigger_event);
        let type_key = format!("{}^*", message_type);

        self.routes.contains_key(&exact_key)
            || self.routes.contains_key(&type_key)
            || self.default_handler.is_some()
    }

    /// Get the number of registered routes
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// List all registered route keys
    pub fn route_keys(&self) -> Vec<String> {
        self.routes.keys().cloned().collect()
    }
}

#[cfg(feature = "routing")]
impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing message routers
#[cfg(feature = "routing")]
pub struct RouterBuilder {
    router: MessageRouter,
}

#[cfg(feature = "routing")]
impl RouterBuilder {
    /// Create a new router builder
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "routing")]
    /// # {
    /// use vyna_hl7_http::router::RouterBuilder;
    /// use std::sync::Arc;
    ///
    /// let router = RouterBuilder::new()
    ///     .route("ADT", "A01", Arc::new(|msg| Ok(msg)))
    ///     .route("ORU", "R01", Arc::new(|msg| Ok(msg)))
    ///     .default(Arc::new(|msg| Ok(msg)))
    ///     .build();
    /// # }
    /// ```
    pub fn new() -> Self {
        Self {
            router: MessageRouter::new(),
        }
    }

    /// Add a route
    pub fn route(mut self, message_type: &str, trigger_event: &str, handler: RouteHandler) -> Self {
        self.router.add_route(message_type, trigger_event, handler);
        self
    }

    /// Add a type route (matches all trigger events)
    pub fn type_route(mut self, message_type: &str, handler: RouteHandler) -> Self {
        self.router.add_type_route(message_type, handler);
        self
    }

    /// Set the default handler
    pub fn default(mut self, handler: RouteHandler) -> Self {
        self.router.set_default_handler(handler);
        self
    }

    /// Build the router
    pub fn build(self) -> MessageRouter {
        self.router
    }
}

#[cfg(feature = "routing")]
impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Routing middleware for applying a router to messages
#[cfg(feature = "routing")]
pub fn routing_middleware(router: Arc<MessageRouter>) -> impl Fn(Message) -> Result<Message> {
    move |message: Message| router.route(message)
}

#[cfg(test)]
#[cfg(feature = "routing")]
mod tests {
    use super::*;

    #[test]
    fn test_router_creation() {
        let router = MessageRouter::new();
        assert_eq!(router.route_count(), 0);
    }

    #[test]
    fn test_add_route() {
        let mut router = MessageRouter::new();
        router.add_route("ADT", "A01", Arc::new(|msg| Ok(msg)));
        assert_eq!(router.route_count(), 1);
        assert!(router.has_route("ADT", "A01"));
    }

    #[test]
    fn test_add_type_route() {
        let mut router = MessageRouter::new();
        router.add_type_route("ADT", Arc::new(|msg| Ok(msg)));
        assert_eq!(router.route_count(), 1);
        assert!(router.has_route("ADT", "A01"));
        assert!(router.has_route("ADT", "A04"));
    }

    #[test]
    fn test_router_builder() {
        let router = RouterBuilder::new()
            .route("ADT", "A01", Arc::new(|msg| Ok(msg)))
            .route("ORU", "R01", Arc::new(|msg| Ok(msg)))
            .default(Arc::new(|msg| Ok(msg)))
            .build();

        assert_eq!(router.route_count(), 2);
        assert!(router.has_route("ADT", "A01"));
        assert!(router.has_route("ORU", "R01"));
        assert!(router.has_route("XXX", "YYY")); // default exists
    }

    #[test]
    fn test_route_keys() {
        let mut router = MessageRouter::new();
        router.add_route("ADT", "A01", Arc::new(|msg| Ok(msg)));
        router.add_route("ORU", "R01", Arc::new(|msg| Ok(msg)));

        let keys = router.route_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"ADT^A01".to_string()));
        assert!(keys.contains(&"ORU^R01".to_string()));
    }
}
