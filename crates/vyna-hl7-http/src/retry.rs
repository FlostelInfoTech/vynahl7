//! Retry logic with exponential backoff for HTTP requests
//!
//! This module provides retry functionality for handling transient failures
//! when sending HL7 messages over HTTP.

#[cfg(feature = "retry")]
use crate::{Error, Result};
#[cfg(feature = "retry")]
use std::time::Duration;

/// Retry policy configuration
#[cfg(feature = "retry")]
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Maximum backoff duration
    pub max_backoff: Duration,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Whether to add jitter to backoff durations
    pub jitter: bool,
}

#[cfg(feature = "retry")]
impl RetryPolicy {
    /// Create a new retry policy
    ///
    /// # Arguments
    ///
    /// * `max_attempts` - Maximum number of retry attempts
    /// * `initial_backoff` - Initial backoff duration
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::RetryPolicy;
    /// use std::time::Duration;
    ///
    /// let policy = RetryPolicy::new(3, Duration::from_millis(100));
    /// # }
    /// ```
    pub fn new(max_attempts: u32, initial_backoff: Duration) -> Self {
        Self {
            max_attempts,
            initial_backoff,
            max_backoff: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }

    /// Create an exponential backoff policy (3 retries, 100ms initial, 2x multiplier)
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::RetryPolicy;
    ///
    /// let policy = RetryPolicy::exponential();
    /// # }
    /// ```
    pub fn exponential() -> Self {
        Self::new(3, Duration::from_millis(100))
    }

    /// Create a linear backoff policy (3 retries, 1s backoff, no multiplier)
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::RetryPolicy;
    ///
    /// let policy = RetryPolicy::linear();
    /// # }
    /// ```
    pub fn linear() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            backoff_multiplier: 1.0,
            jitter: false,
        }
    }

    /// Create a fixed backoff policy (3 retries, 1s fixed delay)
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::RetryPolicy;
    ///
    /// let policy = RetryPolicy::fixed();
    /// # }
    /// ```
    pub fn fixed() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(1),
            backoff_multiplier: 1.0,
            jitter: false,
        }
    }

    /// Set the maximum backoff duration
    pub fn with_max_backoff(mut self, max_backoff: Duration) -> Self {
        self.max_backoff = max_backoff;
        self
    }

    /// Set the backoff multiplier
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Enable or disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Calculate the backoff duration for a given attempt
    ///
    /// # Arguments
    ///
    /// * `attempt` - The current attempt number (0-indexed)
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::RetryPolicy;
    /// use std::time::Duration;
    ///
    /// let policy = RetryPolicy::exponential();
    /// let backoff = policy.backoff_duration(0);
    /// # }
    /// ```
    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        let base_duration = self.initial_backoff.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32);

        let duration_ms = base_duration.min(self.max_backoff.as_millis() as f64);

        let final_duration = if self.jitter {
            // Add random jitter up to ±25%
            use rand::RngExt;
            let mut rng = rand::rng();
            let jitter_factor = rng.random_range(0.75..=1.25);
            (duration_ms * jitter_factor) as u64
        } else {
            duration_ms as u64
        };

        Duration::from_millis(final_duration)
    }

    /// Check if an error is retryable
    ///
    /// By default, only network/timeout errors are retryable.
    /// HTTP 5xx errors are also retryable.
    pub fn is_retryable(&self, error: &Error) -> bool {
        match error {
            Error::Io(_) => true,
            Error::Http(msg) => {
                // Retry on 5xx server errors
                msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("504")
            }
            _ => false,
        }
    }
}

#[cfg(feature = "retry")]
impl Default for RetryPolicy {
    fn default() -> Self {
        Self::exponential()
    }
}

/// Retry executor for running operations with retry logic
#[cfg(feature = "retry")]
pub struct RetryExecutor {
    policy: RetryPolicy,
}

#[cfg(feature = "retry")]
impl RetryExecutor {
    /// Create a new retry executor
    ///
    /// # Arguments
    ///
    /// * `policy` - The retry policy to use
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::{RetryExecutor, RetryPolicy};
    ///
    /// let executor = RetryExecutor::new(RetryPolicy::exponential());
    /// # }
    /// ```
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }

    /// Execute an async operation with retries
    ///
    /// # Arguments
    ///
    /// * `operation` - The async operation to execute
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::{RetryExecutor, RetryPolicy};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = RetryExecutor::new(RetryPolicy::exponential());
    ///
    /// let result = executor.execute(|| async {
    ///     // Your async operation here
    ///     Ok(())
    /// }).await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt >= self.policy.max_attempts || !self.policy.is_retryable(&error) {
                        return Err(error);
                    }

                    let backoff = self.policy.backoff_duration(attempt);

                    #[cfg(feature = "logging")]
                    {
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = self.policy.max_attempts,
                            backoff_ms = backoff.as_millis(),
                            "Retrying after error"
                        );
                    }

                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }

    /// Execute an async operation with retries and a custom retry predicate
    ///
    /// # Arguments
    ///
    /// * `operation` - The async operation to execute
    /// * `should_retry` - Predicate function to determine if an error is retryable
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "retry")]
    /// # {
    /// use vyna_hl7_http::retry::{RetryExecutor, RetryPolicy};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let executor = RetryExecutor::new(RetryPolicy::exponential());
    ///
    /// let result = executor.execute_with_predicate(
    ///     || async { Ok(()) },
    ///     |_error| true  // Always retry
    /// ).await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn execute_with_predicate<F, Fut, T, P>(
        &self,
        mut operation: F,
        should_retry: P,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
        P: Fn(&Error) -> bool,
    {
        let mut attempt = 0;

        loop {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt >= self.policy.max_attempts || !should_retry(&error) {
                        return Err(error);
                    }

                    let backoff = self.policy.backoff_duration(attempt);

                    #[cfg(feature = "logging")]
                    {
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_attempts = self.policy.max_attempts,
                            backoff_ms = backoff.as_millis(),
                            "Retrying after error"
                        );
                    }

                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "retry")]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy_creation() {
        let policy = RetryPolicy::exponential();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_backoff, Duration::from_millis(100));
        assert_eq!(policy.backoff_multiplier, 2.0);
        assert!(policy.jitter);
    }

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::exponential().with_jitter(false);

        let backoff0 = policy.backoff_duration(0);
        let backoff1 = policy.backoff_duration(1);
        let backoff2 = policy.backoff_duration(2);

        assert_eq!(backoff0, Duration::from_millis(100));
        assert_eq!(backoff1, Duration::from_millis(200));
        assert_eq!(backoff2, Duration::from_millis(400));
    }

    #[test]
    fn test_linear_backoff() {
        let policy = RetryPolicy::linear();

        let backoff0 = policy.backoff_duration(0);
        let backoff1 = policy.backoff_duration(1);

        assert_eq!(backoff0, Duration::from_secs(1));
        assert_eq!(backoff1, Duration::from_secs(1));
    }

    #[test]
    fn test_max_backoff() {
        let policy = RetryPolicy::exponential()
            .with_max_backoff(Duration::from_millis(300))
            .with_jitter(false);

        let backoff2 = policy.backoff_duration(2);
        let backoff10 = policy.backoff_duration(10);

        assert!(backoff2 <= Duration::from_millis(300));
        assert_eq!(backoff10, Duration::from_millis(300));
    }
}
