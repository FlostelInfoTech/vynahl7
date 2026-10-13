//! Content-based message routing
//!
//! This module provides functionality for routing HL7 messages to different handlers
//! based on message content (field values, message types, etc.).
//!
//! ## Example
//!
//! ```rust,no_run
//! use vyna_hl7_orchestration::routing::ContentRouter;
//! use vyna_hl7_core::Message;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut router = ContentRouter::new();
//!
//! // Route ADT messages
//! router.add_route("adt", |msg| {
//!     use vyna_hl7_terser::Terser;
//!     let terser = Terser::new(msg);
//!     terser.get("MSH-9-1").ok().flatten().as_deref() == Some("ADT")
//! }, |msg| {
//!     Box::pin(async move {
//!         println!("Processing ADT message");
//!         Ok(msg.clone())
//!     })
//! });
//!
//! // Route ORU messages
//! router.add_route("oru", |msg| {
//!     use vyna_hl7_terser::Terser;
//!     let terser = Terser::new(msg);
//!     terser.get("MSH-9-1").ok().flatten().as_deref() == Some("ORU")
//! }, |msg| {
//!     Box::pin(async move {
//!         println!("Processing ORU message");
//!         Ok(msg.clone())
//!     })
//! });
//!
//! # let message = Message::default();
//! let result = router.route(message).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{OrchestrationError, Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use vyna_hl7_core::Message;

/// Type alias for route condition functions
pub type RouteCondition = Arc<dyn Fn(&Message) -> bool + Send + Sync>;

/// Type alias for route handler functions
pub type RouteHandler =
    Arc<dyn Fn(Message) -> Pin<Box<dyn Future<Output = Result<Message>> + Send>> + Send + Sync>;

/// A single content-based route
pub struct ContentRoute {
    /// Route name
    pub name: String,
    /// Condition that determines if this route should handle the message
    condition: RouteCondition,
    /// Handler function for the route
    handler: RouteHandler,
}

impl ContentRoute {
    /// Create a new content route
    pub fn new<C, H, F>(name: impl Into<String>, condition: C, handler: H) -> Self
    where
        C: Fn(&Message) -> bool + Send + Sync + 'static,
        H: Fn(Message) -> F + Send + Sync + 'static,
        F: Future<Output = Result<Message>> + Send + 'static,
    {
        Self {
            name: name.into(),
            condition: Arc::new(condition),
            handler: Arc::new(move |msg| Box::pin(handler(msg))),
        }
    }

    /// Check if this route matches the given message
    pub fn matches(&self, message: &Message) -> bool {
        (self.condition)(message)
    }

    /// Execute the route handler
    pub async fn execute(&self, message: Message) -> Result<Message> {
        (self.handler)(message).await
    }
}

/// Content-based message router
///
/// Routes messages to different handlers based on content predicates.
pub struct ContentRouter {
    routes: Vec<ContentRoute>,
    default_handler: Option<RouteHandler>,
}

impl ContentRouter {
    /// Create a new content router
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            default_handler: None,
        }
    }

    /// Add a route to the router
    pub fn add_route<C, H, F>(&mut self, name: impl Into<String>, condition: C, handler: H)
    where
        C: Fn(&Message) -> bool + Send + Sync + 'static,
        H: Fn(Message) -> F + Send + Sync + 'static,
        F: Future<Output = Result<Message>> + Send + 'static,
    {
        self.routes
            .push(ContentRoute::new(name, condition, handler));
    }

    /// Set a default handler for messages that don't match any route
    pub fn set_default_handler<H, F>(&mut self, handler: H)
    where
        H: Fn(Message) -> F + Send + Sync + 'static,
        F: Future<Output = Result<Message>> + Send + 'static,
    {
        self.default_handler = Some(Arc::new(move |msg| Box::pin(handler(msg))));
    }

    /// Get the number of routes
    pub fn route_count(&self) -> usize {
        self.routes.len()
    }

    /// Route a message to the first matching handler
    pub async fn route(&self, message: Message) -> Result<Message> {
        // Find the first matching route
        for route in &self.routes {
            if route.matches(&message) {
                return route
                    .execute(message)
                    .await
                    .map_err(|e| OrchestrationError::route_failed(&route.name, e));
            }
        }

        // No matching route found - use default handler if available
        if let Some(handler) = &self.default_handler {
            return handler(message).await;
        }

        // No matching route and no default handler
        Err(OrchestrationError::NoMatchingRoute)
    }

    /// Route a message to all matching handlers
    pub async fn route_all(&self, message: Message) -> Vec<Result<Message>> {
        let mut results = Vec::new();

        for route in &self.routes {
            if route.matches(&message) {
                let result = route
                    .execute(message.clone())
                    .await
                    .map_err(|e| OrchestrationError::route_failed(&route.name, e));
                results.push(result);
            }
        }

        // If no routes matched and there's a default handler, use it
        if results.is_empty()
            && let Some(handler) = &self.default_handler
        {
            results.push(handler(message).await);
        }

        results
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.routes.clear();
    }
}

impl Default for ContentRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::{Field, Segment};

    fn create_adt_message() -> Message {
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
        msg.segments.push(msh);
        msg
    }

    fn create_oru_message() -> Message {
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
        msg.segments.push(msh);
        msg
    }

    #[tokio::test]
    async fn test_content_router_basic() {
        let mut router = ContentRouter::new();

        router.add_route(
            "adt",
            |msg| {
                use vyna_hl7_terser::Terser;
                let terser = Terser::new(msg);
                terser
                    .get("MSH-9-1")
                    .ok()
                    .flatten()
                    .map(|v| v.contains("ADT"))
                    .unwrap_or(false)
            },
            |msg| async move { Ok(msg) },
        );

        let adt_msg = create_adt_message();
        let result = router.route(adt_msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_content_router_no_match() {
        let router = ContentRouter::new();
        let adt_msg = create_adt_message();
        let result = router.route(adt_msg).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OrchestrationError::NoMatchingRoute)));
    }

    #[tokio::test]
    async fn test_content_router_default_handler() {
        let mut router = ContentRouter::new();

        router.set_default_handler(|msg| async move { Ok(msg) });

        let adt_msg = create_adt_message();
        let result = router.route(adt_msg).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_content_router_multiple_routes() {
        let mut router = ContentRouter::new();

        router.add_route(
            "adt",
            |msg| {
                use vyna_hl7_terser::Terser;
                let terser = Terser::new(msg);
                terser
                    .get("MSH-9-1")
                    .ok()
                    .flatten()
                    .map(|v| v.contains("ADT"))
                    .unwrap_or(false)
            },
            |msg| async move { Ok(msg) },
        );

        router.add_route(
            "oru",
            |msg| {
                use vyna_hl7_terser::Terser;
                let terser = Terser::new(msg);
                terser
                    .get("MSH-9-1")
                    .ok()
                    .flatten()
                    .map(|v| v.contains("ORU"))
                    .unwrap_or(false)
            },
            |msg| async move { Ok(msg) },
        );

        let adt_msg = create_adt_message();
        let oru_msg = create_oru_message();

        let adt_result = router.route(adt_msg).await;
        let oru_result = router.route(oru_msg).await;

        assert!(adt_result.is_ok());
        assert!(oru_result.is_ok());
    }

    #[tokio::test]
    async fn test_route_count() {
        let mut router = ContentRouter::new();
        assert_eq!(router.route_count(), 0);

        router.add_route("test", |_| true, |msg| async move { Ok(msg) });
        assert_eq!(router.route_count(), 1);

        router.clear();
        assert_eq!(router.route_count(), 0);
    }

    #[tokio::test]
    async fn test_route_all() {
        let mut router = ContentRouter::new();

        // Add two routes that both match ADT messages
        router.add_route(
            "adt1",
            |msg| {
                use vyna_hl7_terser::Terser;
                let terser = Terser::new(msg);
                terser
                    .get("MSH-9-1")
                    .ok()
                    .flatten()
                    .map(|v| v.contains("ADT"))
                    .unwrap_or(false)
            },
            |msg| async move { Ok(msg) },
        );

        router.add_route(
            "adt2",
            |msg| {
                use vyna_hl7_terser::Terser;
                let terser = Terser::new(msg);
                terser
                    .get("MSH-9-1")
                    .ok()
                    .flatten()
                    .map(|v| v.contains("ADT"))
                    .unwrap_or(false)
            },
            |msg| async move { Ok(msg) },
        );

        let adt_msg = create_adt_message();
        let results = router.route_all(adt_msg).await;

        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn test_route_all_no_match_uses_default_handler() {
        let mut router = ContentRouter::new();
        router.add_route("adt", |_| false, |msg| async move { Ok(msg) });
        router.set_default_handler(|msg| async move { Ok(msg) });

        let results = router.route_all(create_adt_message()).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[tokio::test]
    async fn test_route_all_no_match_no_default_is_empty() {
        let mut router = ContentRouter::new();
        router.add_route("adt", |_| false, |msg| async move { Ok(msg) });

        let results = router.route_all(create_adt_message()).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_route_all_wraps_handler_error_with_route_name() {
        let mut router = ContentRouter::new();
        router.add_route(
            "failing",
            |_| true,
            |_| async move { Err(OrchestrationError::custom("boom")) },
        );

        let results = router.route_all(create_adt_message()).await;
        assert_eq!(results.len(), 1);
        assert!(matches!(
            &results[0],
            Err(OrchestrationError::RouteExecutionFailed { route, .. })
                if route == "failing"
        ));
        assert!(
            results[0]
                .as_ref()
                .unwrap_err()
                .to_string()
                .contains("boom")
        );
    }
}
