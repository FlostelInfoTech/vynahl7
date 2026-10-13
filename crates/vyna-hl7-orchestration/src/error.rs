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
    #[error("Route '{route}' execution failed: {source}")]
    RouteExecutionFailed {
        /// Route whose handler failed.
        route: String,
        /// Underlying failure, kept for [`std::error::Error::source`].
        #[source]
        source: Box<OrchestrationError>,
    },

    /// Orchestration step failed
    #[error("Orchestration step '{step}' failed: {source}")]
    StepExecutionFailed {
        /// Step whose handler failed.
        step: String,
        /// Underlying failure, kept for [`std::error::Error::source`].
        #[source]
        source: Box<OrchestrationError>,
    },

    /// Message filtering failed
    #[error("Message filter '{0}' failed: {1}")]
    FilterFailed(String, String),

    /// Retry limit exhausted; the last attempt's error is the source.
    #[error("Retry limit exceeded after {attempts} attempts: {source}")]
    RetryLimitExceeded {
        /// Attempts made, including the initial try.
        attempts: usize,
        /// Last attempt's failure, kept for [`std::error::Error::source`].
        #[source]
        source: Box<OrchestrationError>,
    },

    /// Custom error
    #[error("{0}")]
    Custom(String),
}

impl OrchestrationError {
    /// Create a custom error
    pub fn custom(msg: impl Into<String>) -> Self {
        OrchestrationError::Custom(msg.into())
    }

    /// Create a route execution error, preserving the cause
    pub fn route_failed(route_name: impl Into<String>, source: OrchestrationError) -> Self {
        OrchestrationError::RouteExecutionFailed {
            route: route_name.into(),
            source: Box::new(source),
        }
    }

    /// Create a step execution error, preserving the cause
    pub fn step_failed(step_name: impl Into<String>, source: OrchestrationError) -> Self {
        OrchestrationError::StepExecutionFailed {
            step: step_name.into(),
            source: Box::new(source),
        }
    }

    /// Create a retry-exhaustion error, preserving the last failure
    pub fn retry_limit_exceeded(attempts: usize, source: OrchestrationError) -> Self {
        OrchestrationError::RetryLimitExceeded {
            attempts,
            source: Box::new(source),
        }
    }

    /// Create a filter error
    pub fn filter_failed(filter_name: impl Into<String>, msg: impl Into<String>) -> Self {
        OrchestrationError::FilterFailed(filter_name.into(), msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn step_and_route_errors_preserve_their_source() {
        for err in [
            OrchestrationError::step_failed("validate", OrchestrationError::custom("bad field")),
            OrchestrationError::route_failed("adt", OrchestrationError::custom("bad field")),
        ] {
            assert!(err.to_string().contains("bad field"));
            let source = err.source().expect("cause must be preserved");
            assert_eq!(source.to_string(), "bad field");
            assert!(source.source().is_none());
        }
    }

    #[test]
    fn retry_exhaustion_reports_attempts_and_cause() {
        let err = OrchestrationError::retry_limit_exceeded(3, OrchestrationError::custom("boom"));
        assert!(err.to_string().contains("3 attempts"));
        assert!(err.to_string().contains("boom"));
        assert_eq!(err.source().expect("cause kept").to_string(), "boom");
    }
}
