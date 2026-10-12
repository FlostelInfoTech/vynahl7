//! Rate limiting configuration for HTTP endpoints
//!
//! This module provides rate limiting configuration helpers using the governor algorithm.
//! Due to dependency constraints with axum 0.8, users should integrate rate limiting manually
//! using these configurations with the `governor` and `tower` crates directly.
//!
//! # Manual Integration Example
//!
//! ```toml
//! [dependencies]
//! governor = "0.7"
//! tower = "0.5"
//! ```
//!
//! ```rust,ignore
//! use vyna_hl7_http::ratelimit::RateLimitConfig;
//! use governor::{Quota, RateLimiter};
//! use std::num::NonZeroU32;
//! use std::sync::Arc;
//!
//! // Create rate limiter from config
//! let config = RateLimitConfig::per_minute(100);
//! let quota = Quota::with_period(config.period)
//!     .unwrap()
//!     .allow_burst(NonZeroU32::new(config.burst_size).unwrap());
//! let limiter = Arc::new(RateLimiter::direct(quota));
//! ```

#[cfg(feature = "ratelimit")]
use std::time::Duration;

/// Rate limit configuration
///
/// Defines the rate limiting policy for the HTTP server.
#[cfg(feature = "ratelimit")]
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Number of requests allowed per period
    pub requests: u32,
    /// Time period for the rate limit
    pub period: Duration,
    /// Burst size (number of requests that can be made in a burst)
    pub burst_size: u32,
}

#[cfg(feature = "ratelimit")]
impl RateLimitConfig {
    /// Create a new rate limit configuration
    ///
    /// # Arguments
    ///
    /// * `requests` - Number of requests allowed per period
    /// * `period` - Time period for the rate limit
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::RateLimitConfig;
    /// use std::time::Duration;
    ///
    /// // Allow 100 requests per minute
    /// let config = RateLimitConfig::new(100, Duration::from_secs(60));
    /// # }
    /// ```
    pub fn new(requests: u32, period: Duration) -> Self {
        Self {
            requests,
            period,
            burst_size: requests,
        }
    }

    /// Create a rate limit of N requests per second
    ///
    /// # Arguments
    ///
    /// * `requests` - Number of requests allowed per second
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::RateLimitConfig;
    ///
    /// // Allow 10 requests per second
    /// let config = RateLimitConfig::per_second(10);
    /// # }
    /// ```
    pub fn per_second(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(1))
    }

    /// Create a rate limit of N requests per minute
    ///
    /// # Arguments
    ///
    /// * `requests` - Number of requests allowed per minute
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::RateLimitConfig;
    ///
    /// // Allow 100 requests per minute
    /// let config = RateLimitConfig::per_minute(100);
    /// # }
    /// ```
    pub fn per_minute(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(60))
    }

    /// Create a rate limit of N requests per hour
    ///
    /// # Arguments
    ///
    /// * `requests` - Number of requests allowed per hour
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::RateLimitConfig;
    ///
    /// // Allow 1000 requests per hour
    /// let config = RateLimitConfig::per_hour(1000);
    /// # }
    /// ```
    pub fn per_hour(requests: u32) -> Self {
        Self::new(requests, Duration::from_secs(3600))
    }

    /// Set a custom burst size
    ///
    /// The burst size determines how many requests can be made in a short burst.
    /// By default, it equals the requests limit.
    ///
    /// # Arguments
    ///
    /// * `burst_size` - Maximum number of requests in a burst
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::RateLimitConfig;
    ///
    /// // Allow 100 requests per minute, but burst up to 20 at once
    /// let config = RateLimitConfig::per_minute(100)
    ///     .with_burst_size(20);
    /// # }
    /// ```
    pub fn with_burst_size(mut self, burst_size: u32) -> Self {
        self.burst_size = burst_size;
        self
    }
}


/// Default rate limiting configurations for common use cases
#[cfg(feature = "ratelimit")]
pub mod presets {
    use super::*;

    /// Strict rate limit: 10 requests per second per IP
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::presets::strict;
    ///
    /// let config = strict();
    /// # }
    /// ```
    pub fn strict() -> RateLimitConfig {
        RateLimitConfig::per_second(10)
    }

    /// Moderate rate limit: 60 requests per minute per IP
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::presets::moderate;
    ///
    /// let config = moderate();
    /// # }
    /// ```
    pub fn moderate() -> RateLimitConfig {
        RateLimitConfig::per_minute(60)
    }

    /// Permissive rate limit: 1000 requests per hour per IP
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::presets::permissive;
    ///
    /// let config = permissive();
    /// # }
    /// ```
    pub fn permissive() -> RateLimitConfig {
        RateLimitConfig::per_hour(1000)
    }

    /// Development rate limit: 1000 requests per second per IP (essentially unlimited)
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "ratelimit")]
    /// # {
    /// use vyna_hl7_http::ratelimit::presets::dev;
    ///
    /// let config = dev();
    /// # }
    /// ```
    pub fn dev() -> RateLimitConfig {
        RateLimitConfig::per_second(1000)
    }
}

#[cfg(test)]
#[cfg(feature = "ratelimit")]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::per_second(10);
        assert_eq!(config.requests, 10);
        assert_eq!(config.period, Duration::from_secs(1));
        assert_eq!(config.burst_size, 10);
    }

    #[test]
    fn test_rate_limit_with_burst() {
        let config = RateLimitConfig::per_minute(100).with_burst_size(20);
        assert_eq!(config.requests, 100);
        assert_eq!(config.period, Duration::from_secs(60));
        assert_eq!(config.burst_size, 20);
    }

    #[test]
    fn test_presets() {
        let strict = presets::strict();
        assert_eq!(strict.requests, 10);

        let moderate = presets::moderate();
        assert_eq!(moderate.requests, 60);

        let permissive = presets::permissive();
        assert_eq!(permissive.requests, 1000);

        let dev = presets::dev();
        assert_eq!(dev.requests, 1000);
    }
}
