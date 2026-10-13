//! Prometheus metrics for HTTP server monitoring
//!
//! This module provides comprehensive metrics collection including:
//! - HTTP request/response metrics (count, duration, status codes)
//! - HL7 message metrics (by message type and trigger event)
//! - Error tracking
//! - Active connections

#[cfg(feature = "metrics")]
use axum::{Router, extract::Request, middleware::Next, response::Response, routing::get};
#[cfg(feature = "metrics")]
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
#[cfg(feature = "metrics")]
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
#[cfg(feature = "metrics")]
use std::time::Instant;

/// Cached handle so repeat calls return the installed recorder instead
/// of panicking (a global recorder can only be installed once).
#[cfg(feature = "metrics")]
static METRICS_HANDLE: std::sync::OnceLock<PrometheusHandle> = std::sync::OnceLock::new();

/// Initialize Prometheus metrics registry
///
/// Sets up all metrics with descriptions and returns a handle for
/// exposing metrics via HTTP endpoint.
///
/// This function is idempotent: the first call installs the global
/// recorder, and subsequent calls return a clone of the same handle.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "metrics")]
/// # {
/// use vyna_hl7_http::metrics::init_metrics;
///
/// let metrics_handle = init_metrics();
/// # }
/// ```
#[cfg(feature = "metrics")]
pub fn init_metrics() -> PrometheusHandle {
    METRICS_HANDLE
        .get_or_init(|| {
            // Configure histogram buckets for response times
            // Buckets: 1ms, 5ms, 10ms, 25ms, 50ms, 100ms, 250ms, 500ms,
            // 1s, 2.5s, 5s, 10s
            let builder = PrometheusBuilder::new()
                .set_buckets_for_metric(
                    Matcher::Full("http_request_duration_seconds".to_string()),
                    &[
                        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                    ],
                )
                .unwrap();

            let handle = builder.install_recorder().unwrap();

            // Describe all metrics
            describe_counter!(
                "http_requests_total",
                "Total number of HTTP requests processed"
            );
            describe_histogram!(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            );
            describe_counter!("http_request_errors_total", "Total number of HTTP errors");
            describe_gauge!(
                "http_active_connections",
                "Number of active HTTP connections"
            );

            describe_counter!(
                "hl7_messages_total",
                "Total number of HL7 messages processed"
            );
            describe_counter!(
                "hl7_message_errors_total",
                "Total number of HL7 message processing errors"
            );
            describe_histogram!("hl7_message_size_bytes", "Size of HL7 messages in bytes");

            handle
        })
        .clone()
}

/// Metrics middleware for HTTP requests
///
/// Tracks request count, duration, and errors with labels for:
/// - HTTP method
/// - HTTP status code
/// - Endpoint path
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "metrics")]
/// # {
/// use axum::{Router, routing::post};
/// use vyna_hl7_http::metrics::metrics_middleware;
///
/// # async fn handler() -> &'static str { "OK" }
/// let app: Router = Router::new()
///     .route("/", post(handler))
///     .layer(axum::middleware::from_fn(metrics_middleware));
/// # }
/// ```
#[cfg(feature = "metrics")]
pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let start = Instant::now();

    // Increment active connections
    gauge!("http_active_connections").increment(1.0);

    // Process request
    let response = next.run(request).await;

    // Decrement active connections
    gauge!("http_active_connections").decrement(1.0);

    let duration = start.elapsed();
    let status = response.status().as_u16().to_string();

    // Record metrics with labels
    counter!("http_requests_total", "method" => method.clone(), "status" => status.clone(), "path" => path.clone())
        .increment(1);

    histogram!("http_request_duration_seconds", "method" => method.clone(), "path" => path.clone())
        .record(duration.as_secs_f64());

    // Track errors
    if response.status().is_client_error() || response.status().is_server_error() {
        counter!("http_request_errors_total", "method" => method, "status" => status, "path" => path)
            .increment(1);
    }

    response
}

/// Record HL7 message metrics
///
/// Tracks HL7 message processing with labels for:
/// - Message type (e.g., ADT, ORU)
/// - Trigger event (e.g., A01, R01)
/// - Processing direction (inbound/outbound)
/// - Processing status (success/error)
///
/// # Arguments
///
/// * `message` - The HL7 message to record metrics for
/// * `direction` - "inbound" or "outbound"
/// * `success` - Whether the message was processed successfully
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "metrics")]
/// # {
/// use vyna_hl7_http::metrics::record_hl7_message_metrics;
/// use vyna_hl7_core::Message;
///
/// # fn example(message: &Message) {
/// record_hl7_message_metrics(message, "inbound", true);
/// # }
/// # }
/// ```
#[cfg(feature = "metrics")]
pub fn record_hl7_message_metrics(
    message: &vyna_hl7_core::Message,
    direction: &str,
    success: bool,
) {
    use vyna_hl7_terser::Terser;

    let terser = Terser::new(message);

    let message_type = terser.get("MSH-9-1").ok().flatten().unwrap_or("UNKNOWN");

    let trigger_event = terser.get("MSH-9-2").ok().flatten().unwrap_or("UNKNOWN");

    let status = if success { "success" } else { "error" };

    // Encode the message to get size
    let message_size = message.encode().len();

    // Record metrics with labels
    counter!(
        "hl7_messages_total",
        "message_type" => message_type.to_string(),
        "trigger_event" => trigger_event.to_string(),
        "direction" => direction.to_string(),
        "status" => status.to_string()
    )
    .increment(1);

    histogram!(
        "hl7_message_size_bytes",
        "message_type" => message_type.to_string(),
        "direction" => direction.to_string()
    )
    .record(message_size as f64);

    // Record errors
    if !success {
        counter!(
            "hl7_message_errors_total",
            "message_type" => message_type.to_string(),
            "trigger_event" => trigger_event.to_string(),
            "direction" => direction.to_string()
        )
        .increment(1);
    }
}

/// Create a metrics endpoint router
///
/// Returns a Router with a `/metrics` endpoint that exposes Prometheus metrics.
///
/// # Arguments
///
/// * `handle` - The PrometheusHandle from `init_metrics()`
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "metrics")]
/// # {
/// use vyna_hl7_http::metrics::{init_metrics, metrics_endpoint};
/// use axum::Router;
///
/// # async fn example() {
/// let metrics_handle = init_metrics();
/// let metrics_router = metrics_endpoint(metrics_handle);
///
/// // Merge with your main router
/// let app = Router::new()
///     .merge(metrics_router);
/// # }
/// # }
/// ```
#[cfg(feature = "metrics")]
pub fn metrics_endpoint(handle: PrometheusHandle) -> Router {
    Router::new().route("/metrics", get(move || async move { handle.render() }))
}

/// Increment the active connections gauge
///
/// Call this when a new connection is accepted.
#[cfg(feature = "metrics")]
pub fn increment_active_connections() {
    gauge!("http_active_connections").increment(1.0);
}

/// Decrement the active connections gauge
///
/// Call this when a connection is closed.
#[cfg(feature = "metrics")]
pub fn decrement_active_connections() {
    gauge!("http_active_connections").decrement(1.0);
}

#[cfg(test)]
#[cfg(feature = "metrics")]
mod tests {
    use super::*;

    #[test]
    fn test_init_metrics() {
        let _handle = init_metrics();
        // Just verify it doesn't panic
    }

    #[test]
    fn test_init_metrics_idempotent() {
        // A second call must not panic (installing a second global
        // recorder would) and must return the same recorder.
        let first = init_metrics();
        let second = init_metrics();
        assert_eq!(first.render(), second.render());
    }
}
