//! Message orchestration and workflow engine
//!
//! This module provides tools for building multi-step async workflows
//! for processing HL7 messages.
//!
//! ## Example
//!
//! ```rust,no_run
//! use vyna_hl7_orchestration::orchestration::MessageOrchestrator;
//! use vyna_hl7_core::Message;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut orchestrator = MessageOrchestrator::new();
//!
//! orchestrator
//!     .add_step("validate", |msg| {
//!         Box::pin(async move {
//!             println!("Validating...");
//!             Ok(msg.clone())
//!         })
//!     })
//!     .add_step("transform", |msg| {
//!         Box::pin(async move {
//!             println!("Transforming...");
//!             Ok(msg.clone())
//!         })
//!     })
//!     .add_step("route", |msg| {
//!         Box::pin(async move {
//!             println!("Routing...");
//!             Ok(msg.clone())
//!         })
//!     });
//!
//! # let message = Message::default();
//! let result = orchestrator.execute(message).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{OrchestrationError, Result};
use vyna_hl7_core::Message;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Type alias for orchestration step functions
pub type StepHandler =
    Arc<dyn Fn(Message) -> Pin<Box<dyn Future<Output = Result<Message>> + Send>> + Send + Sync>;

/// An orchestration step in a workflow
pub struct OrchestrationStep {
    /// Step name
    pub name: String,
    /// Step handler function
    handler: StepHandler,
    /// Retry configuration for this step
    retry_config: Option<RetryConfig>,
}

impl OrchestrationStep {
    /// Create a new orchestration step
    pub fn new<H, F>(name: impl Into<String>, handler: H) -> Self
    where
        H: Fn(Message) -> F + Send + Sync + 'static,
        F: Future<Output = Result<Message>> + Send + 'static,
    {
        Self {
            name: name.into(),
            handler: Arc::new(move |msg| Box::pin(handler(msg))),
            retry_config: None,
        }
    }

    /// Set retry configuration for this step
    pub fn with_retry(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }

    /// Execute the step
    pub async fn execute(&self, message: Message) -> Result<Message> {
        if let Some(retry_config) = &self.retry_config {
            self.execute_with_retry(message, retry_config).await
        } else {
            (self.handler)(message).await
        }
    }

    /// Execute with retry logic
    async fn execute_with_retry(
        &self,
        message: Message,
        config: &RetryConfig,
    ) -> Result<Message> {
        let mut attempts = 0;
        let mut last_error = None;
        // The field is public, so clamp here too: a step always runs once.
        let max_attempts = config.max_attempts.max(1);

        while attempts < max_attempts {
            attempts += 1;

            match (self.handler)(message.clone()).await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempts < max_attempts {
                        let delay = config.delay_ms * attempts as u64;
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(OrchestrationError::step_failed(
            &self.name,
            format!(
                "Failed after {} attempts: {}",
                attempts,
                last_error.unwrap()
            ),
        ))
    }
}

/// Retry configuration for orchestration steps
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of attempts
    pub max_attempts: usize,
    /// Base delay between retries in milliseconds
    pub delay_ms: u64,
}

impl RetryConfig {
    /// Create a new retry configuration
    ///
    /// `max_attempts` counts the initial try, so values below 1 are clamped
    /// to 1: a step always runs at least once and never panics on 0.
    pub fn new(max_attempts: usize, delay_ms: u64) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            delay_ms,
        }
    }

    /// No retries (default)
    pub fn none() -> Self {
        Self {
            max_attempts: 1,
            delay_ms: 0,
        }
    }

    /// Standard retry (3 attempts, 100ms delay)
    pub fn standard() -> Self {
        Self {
            max_attempts: 3,
            delay_ms: 100,
        }
    }

    /// Aggressive retry (5 attempts, 50ms delay)
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            delay_ms: 50,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::none()
    }
}

/// Message orchestrator for building multi-step workflows
pub struct MessageOrchestrator {
    steps: Vec<OrchestrationStep>,
    error_handler: Option<ErrorHandler>,
}

/// Type alias for error handler functions
pub type ErrorHandler = Arc<
    dyn Fn(String, OrchestrationError, Message) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

impl MessageOrchestrator {
    /// Create a new message orchestrator
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            error_handler: None,
        }
    }

    /// Add a step to the orchestration workflow
    pub fn add_step<H, F>(&mut self, name: impl Into<String>, handler: H) -> &mut Self
    where
        H: Fn(Message) -> F + Send + Sync + 'static,
        F: Future<Output = Result<Message>> + Send + 'static,
    {
        self.steps.push(OrchestrationStep::new(name, handler));
        self
    }

    /// Add a step with retry configuration
    pub fn add_step_with_retry<H, F>(
        &mut self,
        name: impl Into<String>,
        handler: H,
        retry_config: RetryConfig,
    ) -> &mut Self
    where
        H: Fn(Message) -> F + Send + Sync + 'static,
        F: Future<Output = Result<Message>> + Send + 'static,
    {
        self.steps
            .push(OrchestrationStep::new(name, handler).with_retry(retry_config));
        self
    }

    /// Set an error handler for failed steps
    pub fn set_error_handler<H, F>(&mut self, handler: H)
    where
        H: Fn(String, OrchestrationError, Message) -> F + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        self.error_handler = Some(Arc::new(move |name, err, msg| Box::pin(handler(name, err, msg))));
    }

    /// Get the number of steps
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Execute the orchestration workflow
    pub async fn execute(&self, mut message: Message) -> Result<Message> {
        for step in &self.steps {
            match step.execute(message.clone()).await {
                Ok(result) => {
                    message = result;
                }
                Err(e) => {
                    // Call error handler if configured
                    if let Some(handler) = &self.error_handler {
                        handler(step.name.clone(), e.clone(), message.clone()).await;
                    }

                    return Err(OrchestrationError::step_failed(&step.name, e.to_string()));
                }
            }
        }

        Ok(message)
    }

    /// Execute the workflow and continue on errors
    pub async fn execute_continue_on_error(&self, mut message: Message) -> (Message, Vec<OrchestrationError>) {
        let mut errors = Vec::new();

        for step in &self.steps {
            match step.execute(message.clone()).await {
                Ok(result) => {
                    message = result;
                }
                Err(e) => {
                    // Call error handler if configured
                    if let Some(handler) = &self.error_handler {
                        handler(step.name.clone(), e.clone(), message.clone()).await;
                    }

                    errors.push(OrchestrationError::step_failed(&step.name, e.to_string()));
                    // Continue with next step even though this one failed
                }
            }
        }

        (message, errors)
    }

    /// Clear all steps
    pub fn clear(&mut self) {
        self.steps.clear();
    }
}

impl Default for MessageOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::{Field, Segment};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_test_message() -> Message {
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msg.segments.push(msh);
        msg
    }

    #[tokio::test]
    async fn test_orchestrator_basic() {
        let mut orchestrator = MessageOrchestrator::new();

        orchestrator.add_step("step1", |msg| async move { Ok(msg) });
        orchestrator.add_step("step2", |msg| async move { Ok(msg) });

        let message = create_test_message();
        let result = orchestrator.execute(message).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_orchestrator_failure() {
        let mut orchestrator = MessageOrchestrator::new();

        orchestrator.add_step("step1", |msg| async move { Ok(msg) });
        orchestrator.add_step("failing_step", |_msg| async move {
            Err(OrchestrationError::custom("Test failure"))
        });
        orchestrator.add_step("step3", |msg| async move { Ok(msg) });

        let message = create_test_message();
        let result = orchestrator.execute(message).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_orchestrator_continue_on_error() {
        let mut orchestrator = MessageOrchestrator::new();

        orchestrator.add_step("step1", |msg| async move { Ok(msg) });
        orchestrator.add_step("failing_step", |_msg| async move {
            Err(OrchestrationError::custom("Test failure"))
        });
        orchestrator.add_step("step3", |msg| async move { Ok(msg) });

        let message = create_test_message();
        let (result, errors) = orchestrator.execute_continue_on_error(message).await;

        assert_eq!(errors.len(), 1);
        assert!(!result.segments.is_empty());
    }

    #[tokio::test]
    async fn test_step_count() {
        let mut orchestrator = MessageOrchestrator::new();
        assert_eq!(orchestrator.step_count(), 0);

        orchestrator.add_step("step1", |msg| async move { Ok(msg) });
        assert_eq!(orchestrator.step_count(), 1);

        orchestrator.clear();
        assert_eq!(orchestrator.step_count(), 0);
    }

    #[tokio::test]
    async fn test_retry_config() {
        let config = RetryConfig::standard();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.delay_ms, 100);

        let config = RetryConfig::aggressive();
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.delay_ms, 50);

        let config = RetryConfig::none();
        assert_eq!(config.max_attempts, 1);
    }

    #[tokio::test]
    async fn test_retry_config_zero_attempts_runs_once() {
        // Zero is clamped: the step runs once and returns the error instead
        // of panicking on an empty attempt history.
        assert_eq!(RetryConfig::new(0, 10).max_attempts, 1);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let mut orchestrator = MessageOrchestrator::new();
        orchestrator.add_step_with_retry(
            "zero_retry",
            move |_msg| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<Message, _>(OrchestrationError::custom("always fails"))
                }
            },
            RetryConfig::new(0, 10),
        );

        let result = orchestrator.execute(create_test_message()).await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_orchestrator_with_retry() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut orchestrator = MessageOrchestrator::new();

        orchestrator.add_step_with_retry(
            "retry_step",
            move |msg| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        // Fail first two attempts
                        Err(OrchestrationError::custom("Retry test"))
                    } else {
                        Ok(msg)
                    }
                }
            },
            RetryConfig::new(3, 10),
        );

        let message = create_test_message();
        let result = orchestrator.execute(message).await;

        assert!(result.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Should have tried 3 times
    }

    #[tokio::test]
    async fn test_error_handler() {
        let error_count = Arc::new(AtomicUsize::new(0));
        let error_count_clone = error_count.clone();

        let mut orchestrator = MessageOrchestrator::new();

        orchestrator.set_error_handler(move |_name, _err, _msg| {
            let count = error_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
            }
        });

        orchestrator.add_step("failing_step", |_msg| async move {
            Err(OrchestrationError::custom("Test error"))
        });

        let message = create_test_message();
        let _ = orchestrator.execute(message).await;

        assert_eq!(error_count.load(Ordering::SeqCst), 1);
    }
}
