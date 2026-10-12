//! Error types for orchestration operations

/// Result type for orchestration operations
pub type Result<T> = std::result::Result<T, OrchestrationError>;

/// Errors that can occur during message orchestration
#[derive(Debug, Clone, thiserror::Error)]
pub enum OrchestrationError {
    /// No matching route found for the message
    #[error("No matching route found for message")]
    NoMatchingRoute,

    /// Route execution failed
    #[error("Route '{0}' execution failed: {1}")]
    RouteExecutionFailed(String, String),

    /// Orchestration step failed
    #[error("Orchestration step '{0}' failed: {1}")]
    StepExecutionFailed(String, String),

    /// Message filtering failed
    #[error("Message filter '{0}' failed: {1}")]
    FilterFailed(String, String),

    /// Retry limit exceeded
    #[error("Retry limit exceeded after {0} attempts")]
    RetryLimitExceeded(usize),

    /// Dead letter queue error
    #[error("Dead letter queue error: {0}")]
    DeadLetterQueueError(String),

    /// Custom error
    #[error("{0}")]
    Custom(String),
}

impl OrchestrationError {
    /// Create a custom error
    pub fn custom(msg: impl Into<String>) -> Self {
        OrchestrationError::Custom(msg.into())
    }

    /// Create a route execution error
    pub fn route_failed(route_name: impl Into<String>, msg: impl Into<String>) -> Self {
        OrchestrationError::RouteExecutionFailed(route_name.into(), msg.into())
    }

    /// Create a step execution error
    pub fn step_failed(step_name: impl Into<String>, msg: impl Into<String>) -> Self {
        OrchestrationError::StepExecutionFailed(step_name.into(), msg.into())
    }

    /// Create a filter error
    pub fn filter_failed(filter_name: impl Into<String>, msg: impl Into<String>) -> Self {
        OrchestrationError::FilterFailed(filter_name.into(), msg.into())
    }
}
