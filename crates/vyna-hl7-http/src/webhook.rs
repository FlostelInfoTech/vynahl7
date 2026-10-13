//! Webhook support for sending HTTP notifications
//!
//! This module provides utilities for sending webhooks when HL7 messages
//! are received or processed. Useful for integrating with external systems
//! and triggering workflows.

#[cfg(feature = "webhooks")]
use crate::{Error, Result};
#[cfg(feature = "webhooks")]
use reqwest::Client;
#[cfg(feature = "webhooks")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "webhooks")]
use std::time::Duration;
#[cfg(feature = "webhooks")]
use vyna_hl7_core::Message;

/// Webhook payload containing HL7 message information
#[cfg(feature = "webhooks")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Event type (e.g., "message.received", "message.processed")
    pub event: String,
    /// Message type (e.g., "ADT", "ORU")
    pub message_type: Option<String>,
    /// Trigger event (e.g., "A01", "R01")
    pub trigger_event: Option<String>,
    /// Message control ID
    pub message_control_id: Option<String>,
    /// The complete HL7 message (optional, can be large)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_content: Option<String>,
    /// Timestamp of the event
    pub timestamp: String,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[cfg(feature = "webhooks")]
impl WebhookPayload {
    /// Create a new webhook payload from a message
    ///
    /// # Arguments
    ///
    /// * `event` - Event type (e.g., "message.received")
    /// * `message` - The HL7 message
    /// * `include_content` - Whether to include the full message content
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "webhooks")]
    /// # {
    /// use vyna_hl7_http::webhook::WebhookPayload;
    /// use vyna_hl7_core::Message;
    ///
    /// # fn example(message: &Message) {
    /// let payload = WebhookPayload::from_message("message.received", message, false);
    /// # }
    /// # }
    /// ```
    pub fn from_message(event: &str, message: &Message, include_content: bool) -> Self {
        use vyna_hl7_terser::Terser;

        let terser = Terser::new(message);

        let message_type = terser.get("MSH-9-1").ok().flatten().map(|s| s.to_string());

        let trigger_event = terser.get("MSH-9-2").ok().flatten().map(|s| s.to_string());

        let message_control_id = terser.get("MSH-10").ok().flatten().map(|s| s.to_string());

        Self {
            event: event.to_string(),
            message_type,
            trigger_event,
            message_control_id,
            message_content: if include_content {
                Some(message.encode())
            } else {
                None
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: None,
        }
    }

    /// Add metadata to the payload
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Webhook configuration
///
/// The `Debug` impl redacts header values: custom headers routinely carry
/// secrets such as API keys.
#[cfg(feature = "webhooks")]
#[derive(Clone)]
pub struct WebhookConfig {
    /// The webhook URL to send notifications to
    pub url: String,
    /// Timeout for webhook requests
    pub timeout: Duration,
    /// Maximum number of retries on failure
    pub max_retries: u32,
    /// Whether to include the full message content in the payload
    pub include_message_content: bool,
    /// Custom headers to include in the webhook request
    pub headers: Vec<(String, String)>,
}

#[cfg(feature = "webhooks")]
impl std::fmt::Debug for WebhookConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redacted: Vec<(&str, &str)> = self
            .headers
            .iter()
            .map(|(name, _)| (name.as_str(), "[REDACTED]"))
            .collect();
        f.debug_struct("WebhookConfig")
            .field("url", &self.url)
            .field("timeout", &self.timeout)
            .field("max_retries", &self.max_retries)
            .field("include_message_content", &self.include_message_content)
            .field("headers", &redacted)
            .finish()
    }
}

#[cfg(feature = "webhooks")]
impl WebhookConfig {
    /// Create a new webhook configuration
    ///
    /// # Arguments
    ///
    /// * `url` - The webhook URL
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "webhooks")]
    /// # {
    /// use vyna_hl7_http::webhook::WebhookConfig;
    ///
    /// let config = WebhookConfig::new("https://example.com/webhook");
    /// # }
    /// ```
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            include_message_content: false,
            headers: Vec::new(),
        }
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Include the full message content in the payload
    pub fn with_message_content(mut self, include: bool) -> Self {
        self.include_message_content = include;
        self
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }
}

/// Upper bound for the exponential retry backoff in [`WebhookClient::send`].
#[cfg(feature = "webhooks")]
const MAX_BACKOFF_MILLIS: u64 = 30_000;

/// Exponential backoff for the given (1-based) retry count: 100ms * 2^retries.
///
/// Saturating arithmetic plus a [`MAX_BACKOFF_MILLIS`] cap keep large
/// `max_retries` values from overflowing (or sleeping absurdly long).
#[cfg(feature = "webhooks")]
fn backoff_delay(retries: u32) -> Duration {
    let shift = 1u64.checked_shl(retries).unwrap_or(u64::MAX);
    Duration::from_millis(100u64.saturating_mul(shift).min(MAX_BACKOFF_MILLIS))
}

/// Webhook client for sending notifications
#[cfg(feature = "webhooks")]
pub struct WebhookClient {
    config: WebhookConfig,
    client: Client,
}

#[cfg(feature = "webhooks")]
impl WebhookClient {
    /// Create a new webhook client
    ///
    /// # Arguments
    ///
    /// * `config` - Webhook configuration
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "webhooks")]
    /// # {
    /// use vyna_hl7_http::webhook::{WebhookClient, WebhookConfig};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WebhookConfig::new("https://example.com/webhook");
    /// let client = WebhookClient::new(config)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn new(config: WebhookConfig) -> Result<Self> {
        let client = Client::builder().timeout(config.timeout).build()?;

        Ok(Self { config, client })
    }

    /// Send a webhook notification
    ///
    /// # Arguments
    ///
    /// * `payload` - The webhook payload to send
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "webhooks")]
    /// # {
    /// use vyna_hl7_http::webhook::{WebhookClient, WebhookConfig, WebhookPayload};
    /// use vyna_hl7_core::Message;
    ///
    /// # async fn example(message: &Message) -> Result<(), Box<dyn std::error::Error>> {
    /// let config = WebhookConfig::new("https://example.com/webhook");
    /// let client = WebhookClient::new(config)?;
    ///
    /// let payload = WebhookPayload::from_message("message.received", message, false);
    /// client.send(payload).await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn send(&self, payload: WebhookPayload) -> Result<()> {
        let mut retries = 0;

        loop {
            let mut request = self
                .client
                .post(&self.config.url)
                .header("Content-Type", "application/json")
                .header("User-Agent", "vyna-hl7-http-webhook")
                .json(&payload);

            // Add custom headers
            for (key, value) in &self.config.headers {
                request = request.header(key, value);
            }

            match request.send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(());
                    } else if response.status().is_server_error()
                        && retries < self.config.max_retries
                    {
                        retries += 1;
                        tokio::time::sleep(backoff_delay(retries)).await;
                        continue;
                    } else {
                        return Err(Error::Http(format!(
                            "Webhook failed with status: {}",
                            response.status()
                        )));
                    }
                }
                Err(_) if retries < self.config.max_retries => {
                    retries += 1;
                    tokio::time::sleep(backoff_delay(retries)).await;
                    continue;
                }
                Err(e) => {
                    return Err(Error::Http(format!("Webhook request failed: {}", e)));
                }
            }
        }
    }

    /// Send a notification for a received message
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message that was received
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "webhooks")]
    /// # {
    /// use vyna_hl7_http::webhook::{WebhookClient, WebhookConfig};
    /// use vyna_hl7_core::Message;
    ///
    /// # async fn example(client: WebhookClient, message: &Message) -> Result<(), Box<dyn std::error::Error>> {
    /// client.notify_received(message).await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn notify_received(&self, message: &Message) -> Result<()> {
        let payload = WebhookPayload::from_message(
            "message.received",
            message,
            self.config.include_message_content,
        );
        self.send(payload).await
    }

    /// Send a notification for a processed message
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message that was processed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "webhooks")]
    /// # {
    /// use vyna_hl7_http::webhook::{WebhookClient, WebhookConfig};
    /// use vyna_hl7_core::Message;
    ///
    /// # async fn example(client: WebhookClient, message: &Message) -> Result<(), Box<dyn std::error::Error>> {
    /// client.notify_processed(message).await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn notify_processed(&self, message: &Message) -> Result<()> {
        let payload = WebhookPayload::from_message(
            "message.processed",
            message,
            self.config.include_message_content,
        );
        self.send(payload).await
    }

    /// Send a notification for a failed message
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message that failed
    /// * `error` - The error message
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "webhooks")]
    /// # {
    /// use vyna_hl7_http::webhook::{WebhookClient, WebhookConfig};
    /// use vyna_hl7_core::Message;
    ///
    /// # async fn example(client: WebhookClient, message: &Message) -> Result<(), Box<dyn std::error::Error>> {
    /// client.notify_failed(message, "Processing error").await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn notify_failed(&self, message: &Message, error: &str) -> Result<()> {
        let mut payload = WebhookPayload::from_message(
            "message.failed",
            message,
            self.config.include_message_content,
        );

        payload.metadata = Some(serde_json::json!({
            "error": error
        }));

        self.send(payload).await
    }
}

#[cfg(test)]
#[cfg(feature = "webhooks")]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_config() {
        let config = WebhookConfig::new("https://example.com/webhook")
            .with_timeout(Duration::from_secs(10))
            .with_max_retries(5)
            .with_message_content(true)
            .with_header("X-API-Key", "secret");

        assert_eq!(config.url, "https://example.com/webhook");
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 5);
        assert!(config.include_message_content);
        assert_eq!(config.headers.len(), 1);
    }

    #[test]
    fn test_backoff_delay_capped_no_overflow() {
        // Exact exponential backoff for small retry counts ...
        assert_eq!(backoff_delay(1), Duration::from_millis(200));
        assert_eq!(backoff_delay(2), Duration::from_millis(400));
        assert_eq!(backoff_delay(3), Duration::from_millis(800));
        assert_eq!(backoff_delay(8), Duration::from_millis(25_600));
        // ... capped, never overflowing, even at absurd counts.
        assert_eq!(backoff_delay(9), Duration::from_secs(30));
        assert_eq!(backoff_delay(60), Duration::from_secs(30));
        assert_eq!(backoff_delay(u32::MAX), Duration::from_secs(30));
    }

    #[test]
    fn test_webhook_debug_redacts_header_values() {
        let config = WebhookConfig::new("https://example.com/webhook")
            .with_header("X-API-Key", "super-secret-value");

        let rendered = format!("{:?}", config);
        assert!(!rendered.contains("super-secret-value"));
        assert!(rendered.contains("X-API-Key"));
        assert!(rendered.contains("[REDACTED]"));
    }
}
