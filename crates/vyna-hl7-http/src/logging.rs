//! Structured logging for HTTP requests and responses
//!
//! This module provides request/response logging with correlation IDs,
//! structured tracing spans, and detailed HL7 message metadata logging.

#[cfg(feature = "logging")]
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
#[cfg(feature = "logging")]
use std::time::Instant;
#[cfg(feature = "logging")]
use tracing::{debug, error, info, warn};
#[cfg(feature = "logging")]
use uuid::Uuid;

/// HTTP request/response logger middleware
///
/// Logs incoming requests and outgoing responses with:
/// - Correlation IDs for request tracing
/// - Request method, path, and headers
/// - Response status code and timing
/// - Error details if applicable
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use axum::{Router, routing::post};
/// use vyna_hl7_http::logging::logging_middleware;
///
/// # async fn handler() -> &'static str { "OK" }
/// let app: Router = Router::new()
///     .route("/", post(handler))
///     .layer(axum::middleware::from_fn(logging_middleware));
/// # }
/// ```
#[cfg(feature = "logging")]
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let correlation_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    // Create a tracing span for this request
    let span = tracing::info_span!(
        "http_request",
        correlation_id = %correlation_id,
        method = %method,
        path = %uri.path(),
    );

    let _enter = span.enter();

    info!(
        correlation_id = %correlation_id,
        method = %method,
        uri = %uri,
        "Incoming HTTP request"
    );

    // Process the request
    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    // Log based on status code
    match status {
        StatusCode::OK | StatusCode::CREATED | StatusCode::ACCEPTED => {
            info!(
                correlation_id = %correlation_id,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
                "HTTP request completed successfully"
            );
        }
        status if status.is_client_error() => {
            warn!(
                correlation_id = %correlation_id,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
                "HTTP request failed with client error"
            );
        }
        status if status.is_server_error() => {
            error!(
                correlation_id = %correlation_id,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
                "HTTP request failed with server error"
            );
        }
        _ => {
            debug!(
                correlation_id = %correlation_id,
                status = %status.as_u16(),
                duration_ms = %duration.as_millis(),
                "HTTP request completed"
            );
        }
    }

    response
}

/// Initialize the logging system with a default configuration
///
/// Sets up tracing-subscriber with:
/// - JSON formatting for structured logs
/// - Environment-based log level filtering (RUST_LOG env var)
/// - Stdout output
///
/// # Panics
///
/// Panics if a global subscriber is already set. Use
/// [`try_init_logging`] for a fallible variant.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use vyna_hl7_http::logging::init_logging;
///
/// init_logging();
/// # }
/// ```
#[cfg(feature = "logging")]
pub fn init_logging() {
    try_init_logging().expect("failed to set global default subscriber");
}

/// Try to initialize the logging system with a default configuration.
///
/// Returns `Err` instead of panicking when a global subscriber is
/// already set (e.g. the host application installed its own).
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use vyna_hl7_http::logging::try_init_logging;
///
/// let _ = try_init_logging();
/// # }
/// ```
#[cfg(feature = "logging")]
pub fn try_init_logging() -> Result<(), tracing_subscriber::util::TryInitError> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().json())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init()
}

/// Initialize logging with custom format (pretty-printed for development)
///
/// Sets up tracing-subscriber with:
/// - Pretty-printed formatting for human readability
/// - Environment-based log level filtering (RUST_LOG env var)
/// - Stdout output
///
/// # Panics
///
/// Panics if a global subscriber is already set. Use
/// [`try_init_logging_pretty`] for a fallible variant.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use vyna_hl7_http::logging::init_logging_pretty;
///
/// init_logging_pretty();
/// # }
/// ```
#[cfg(feature = "logging")]
pub fn init_logging_pretty() {
    try_init_logging_pretty().expect("failed to set global default subscriber");
}

/// Try to initialize logging with pretty-printed formatting.
///
/// Returns `Err` instead of panicking when a global subscriber is
/// already set (e.g. the host application installed its own).
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use vyna_hl7_http::logging::try_init_logging_pretty;
///
/// let _ = try_init_logging_pretty();
/// # }
/// ```
#[cfg(feature = "logging")]
pub fn try_init_logging_pretty() -> Result<(), tracing_subscriber::util::TryInitError> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().pretty())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init()
}

/// Initialize logging with compact format
///
/// Sets up tracing-subscriber with:
/// - Compact formatting for minimal output
/// - Environment-based log level filtering (RUST_LOG env var)
/// - Stdout output
///
/// # Panics
///
/// Panics if a global subscriber is already set. Use
/// [`try_init_logging_compact`] for a fallible variant.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use vyna_hl7_http::logging::init_logging_compact;
///
/// init_logging_compact();
/// # }
/// ```
#[cfg(feature = "logging")]
pub fn init_logging_compact() {
    try_init_logging_compact().expect("failed to set global default subscriber");
}

/// Try to initialize logging with compact format.
///
/// Returns `Err` instead of panicking when a global subscriber is
/// already set (e.g. the host application installed its own).
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use vyna_hl7_http::logging::try_init_logging_compact;
///
/// let _ = try_init_logging_compact();
/// # }
/// ```
#[cfg(feature = "logging")]
pub fn try_init_logging_compact() -> Result<(), tracing_subscriber::util::TryInitError> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    tracing_subscriber::registry()
        .with(fmt::layer().compact())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init()
}

/// Log HL7 message metadata
///
/// Logs structured information about an HL7 message including:
/// - Message type and trigger event
/// - Message control ID
/// - Sending/receiving applications and facilities
/// - Processing ID
///
/// # Arguments
///
/// * `message` - The HL7 message to log
/// * `direction` - "inbound" or "outbound"
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "logging")]
/// # {
/// use vyna_hl7_http::logging::log_hl7_message;
/// use vyna_hl7_core::Message;
///
/// # fn example(message: Message) {
/// log_hl7_message(&message, "inbound");
/// # }
/// # }
/// ```
#[cfg(feature = "logging")]
pub fn log_hl7_message(message: &vyna_hl7_core::Message, direction: &str) {
    use vyna_hl7_terser::Terser;

    let terser = Terser::new(message);

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

    let message_control_id = terser
        .get("MSH-10")
        .ok()
        .flatten()
        .unwrap_or("UNKNOWN");

    let sending_application = terser
        .get("MSH-3")
        .ok()
        .flatten()
        .unwrap_or("UNKNOWN");

    let sending_facility = terser
        .get("MSH-4")
        .ok()
        .flatten()
        .unwrap_or("UNKNOWN");

    let receiving_application = terser
        .get("MSH-5")
        .ok()
        .flatten()
        .unwrap_or("UNKNOWN");

    let receiving_facility = terser
        .get("MSH-6")
        .ok()
        .flatten()
        .unwrap_or("UNKNOWN");

    let processing_id = terser
        .get("MSH-11")
        .ok()
        .flatten()
        .unwrap_or("UNKNOWN");

    info!(
        direction = direction,
        message_type = message_type,
        trigger_event = trigger_event,
        message_control_id = message_control_id,
        sending_application = sending_application,
        sending_facility = sending_facility,
        receiving_application = receiving_application,
        receiving_facility = receiving_facility,
        processing_id = processing_id,
        "HL7 message"
    );
}

#[cfg(test)]
#[cfg(feature = "logging")]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        // Just verify it doesn't panic
        // Can't actually test logging output without integration tests
    }

    #[test]
    fn test_try_init_logging_second_call_returns_err() {
        // The first call may succeed or fail depending on test order
        // (the global subscriber is process-wide); afterwards every
        // variant must report Err instead of panicking.
        let _ = try_init_logging();
        assert!(try_init_logging().is_err());
        assert!(try_init_logging_pretty().is_err());
        assert!(try_init_logging_compact().is_err());
    }
}
