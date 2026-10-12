//! Advanced HTTP Server Example
//!
//! This example demonstrates the full feature set of vyna-hl7-http including:
//! - TLS/mTLS for secure communication
//! - Authentication (Basic Auth, API Key, JWT)
//! - WebSocket support for bidirectional communication
//! - Structured logging with correlation IDs
//! - Prometheus metrics
//! - Rate limiting configuration
//! - Message compression (gzip/brotli)
//! - Message routing by type
//! - Webhook notifications
//! - Async message queue
//! - Batch message processing
//!
//! # Running the Example
//!
//! ## With Basic Features (HTTP only)
//! ```bash
//! cargo run --example advanced_server --features logging,metrics,compression
//! ```
//!
//! ## With All Features (HTTPS, Auth, WebSocket)
//! ```bash
//! cargo run --example advanced_server --features full
//! ```
//!
//! ## Environment Variables
//! - `RUST_LOG`: Set log level (e.g., RUST_LOG=info,vyna_hl7_http=debug)
//! - `HTTP_PORT`: HTTP server port (default: 8080)
//! - `HTTPS_PORT`: HTTPS server port (default: 8443)
//! - `METRICS_PORT`: Metrics endpoint port (default: 9090)

use vyna_hl7_core::Message;
use vyna_hl7_http::{Error, HttpServer, MessageHandler, Result};
use std::sync::Arc;

#[cfg(feature = "logging")]
use vyna_hl7_http::logging;

#[cfg(feature = "metrics")]
use vyna_hl7_http::metrics;

#[cfg(feature = "ratelimit")]
use vyna_hl7_http::ratelimit;

#[cfg(feature = "routing")]
use vyna_hl7_http::router::RouterBuilder;

#[cfg(feature = "queue")]
use vyna_hl7_http::queue::MessageQueue;

#[cfg(feature = "batch")]
use vyna_hl7_http::batch::{BatchConfig, BatchProcessor};

#[cfg(feature = "webhooks")]
use vyna_hl7_http::webhook::{WebhookClient, WebhookConfig};

#[cfg(feature = "websocket")]
use vyna_hl7_http::websocket::{websocket_handler, WebSocketConfig};

#[cfg(feature = "tls")]
use vyna_hl7_http::tls::TlsServerConfig;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    #[cfg(feature = "logging")]
    {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into()),
            )
            .with_target(false)
            .with_thread_ids(true)
            .with_line_number(true)
            .init();

        tracing::info!("🚀 Advanced HL7 HTTP Server Starting");
    }

    // Initialize metrics
    #[cfg(feature = "metrics")]
    let _metrics_handle = {
        let handle = metrics::init_metrics();
        tracing::info!("📊 Metrics initialized - available at http://0.0.0.0:9090/metrics");

        // Spawn metrics server
        let handle_clone = handle.clone();
        tokio::spawn(async move {
            serve_metrics(handle_clone).await;
        });

        handle
    };

    // Create message queue
    #[cfg(feature = "queue")]
    let queue = {
        let q = MessageQueue::bounded(1000);
        tracing::info!("📬 Message queue initialized (capacity: 1000)");

        // Spawn queue processor
        let receiver = q.receiver();
        tokio::spawn(async move {
            while let Ok(message) = receiver.recv().await {
                tracing::info!(
                    "Queue processed message: {:?}",
                    message.get_control_id()
                );
            }
        });

        q
    };

    // Create webhook client
    #[cfg(feature = "webhooks")]
    let webhook_client = {
        let config = WebhookConfig {
            url: std::env::var("WEBHOOK_URL")
                .unwrap_or_else(|_| "http://localhost:3000/webhook".to_string()),
            timeout: std::time::Duration::from_secs(5),
            max_retries: 3,
            include_message_content: false,
            headers: vec![("X-Service".to_string(), "VynaHL7-HTTP".to_string())],
        };

        match WebhookClient::new(config) {
            Ok(client) => {
                tracing::info!("🔔 Webhook client initialized");
                Some(Arc::new(client))
            }
            Err(e) => {
                tracing::warn!("Failed to initialize webhook client: {}", e);
                None
            }
        }
    };

    // Create message router
    #[cfg(feature = "routing")]
    let router = {
        let r = RouterBuilder::new()
            .route("ADT", "A01", Arc::new(handle_adt_a01))
            .route("ADT", "A04", Arc::new(handle_adt_a04))
            .route("ORU", "R01", Arc::new(handle_oru_r01))
            .type_route("SIU", Arc::new(handle_siu))
            .default(Arc::new(handle_default))
            .build();

        tracing::info!("🔀 Message router initialized with 5 routes");
        Arc::new(r)
    };

    // Create batch processor
    #[cfg(feature = "batch")]
    let _batch_processor = {
        let config = BatchConfig::medium(); // 50 messages or 15 seconds
        tracing::info!(
            "📦 Batch processor initialized (max_size: {}, max_age: {:?})",
            config.max_size,
            config.max_age
        );

        BatchProcessor::new(config.max_size, config.max_age, |messages| {
            tracing::info!("Processing batch of {} messages", messages.len());
            // Batch processing logic here
        })
    };

    // Create main message handler
    let handler: MessageHandler = Arc::new(move |message: Message| {
        #[cfg(feature = "logging")]
        {
            logging::log_hl7_message(&message, "received");
        }

        #[cfg(feature = "metrics")]
        {
            metrics::record_hl7_message_metrics(&message, "received", true);
        }

        #[cfg(feature = "queue")]
        {
            if let Err(e) = queue.try_send(message.clone()) {
                tracing::warn!("Failed to enqueue message: {}", e);
            }
        }

        #[cfg(feature = "webhooks")]
        {
            if let Some(ref webhook) = webhook_client {
                let webhook = webhook.clone();
                let msg = message.clone();
                tokio::spawn(async move {
                    if let Err(e) = webhook.notify_received(&msg).await {
                        tracing::warn!("Webhook notification failed: {}", e);
                    }
                });
            }
        }

        // Route the message
        #[cfg(feature = "routing")]
        {
            match router.route(message) {
                Ok(response) => {
                    #[cfg(feature = "logging")]
                    logging::log_hl7_message(&response, "sent");

                    return Ok(response);
                }
                Err(e) => {
                    tracing::error!("Routing error: {}", e);
                    return Err(e);
                }
            }
        }

        #[cfg(not(feature = "routing"))]
        {
            // Simple echo response
            Ok(message)
        }
    });

    // Configure rate limiting (requires manual integration)
    #[cfg(feature = "ratelimit")]
    {
        let rate_config = ratelimit::presets::moderate(); // 60 req/min
        tracing::info!(
            "⏱️  Rate limiting configured: {} requests per {:?}",
            rate_config.requests,
            rate_config.period
        );
        tracing::info!(
            "   Note: Rate limiting requires manual integration with governor crate"
        );
    }

    // Build HTTP server
    let mut http_server = HttpServer::new().with_handler(handler.clone());

    // Enable compression
    #[cfg(feature = "compression")]
    {
        http_server = http_server.with_compression();
        tracing::info!("🗜️  Compression enabled (gzip/brotli)");
    }

    // Configure authentication
    #[cfg(feature = "auth")]
    {
        if let (Ok(username), Ok(password)) =
            (std::env::var("AUTH_USERNAME"), std::env::var("AUTH_PASSWORD"))
        {
            http_server = http_server.with_auth(username.clone(), password);
            tracing::info!("🔐 Basic authentication enabled (user: {})", username);
        }
    }

    let http_port = std::env::var("HTTP_PORT").unwrap_or_else(|_| "8080".to_string());
    let http_addr = format!("0.0.0.0:{}", http_port);

    // Spawn HTTP server
    let http_handle = {
        let server = http_server.clone();
        let addr = http_addr.clone();
        tokio::spawn(async move {
            tracing::info!("🌐 HTTP server listening on {}", addr);
            if let Err(e) = server.serve(&addr).await {
                tracing::error!("HTTP server error: {}", e);
            }
        })
    };

    // Spawn HTTPS server with TLS
    #[cfg(feature = "tls")]
    let https_handle = {
        if let (Ok(cert_path), Ok(key_path)) =
            (std::env::var("TLS_CERT"), std::env::var("TLS_KEY"))
        {
            match TlsServerConfig::new(&cert_path, &key_path) {
                Ok(tls_config) => {
                    let https_port =
                        std::env::var("HTTPS_PORT").unwrap_or_else(|_| "8443".to_string());
                    let https_addr = format!("0.0.0.0:{}", https_port);

                    let server = http_server.clone().with_tls(tls_config);

                    tracing::info!("🔒 HTTPS server listening on {}", https_addr);

                    Some(tokio::spawn(async move {
                        if let Err(e) = server.serve_tls(&https_addr).await {
                            tracing::error!("HTTPS server error: {}", e);
                        }
                    }))
                }
                Err(e) => {
                    tracing::error!("Failed to configure TLS: {}", e);
                    None
                }
            }
        } else {
            tracing::info!("🔒 HTTPS server disabled (set TLS_CERT and TLS_KEY to enable)");
            None
        }
    };

    // Spawn WebSocket server
    #[cfg(feature = "websocket")]
    let ws_handle = {
        use axum::{routing::get, Router};

        let ws_config = WebSocketConfig::new(handler.clone())
            .with_max_message_size(2 * 1024 * 1024) // 2 MB
            .with_broadcast();

        let ws_port = std::env::var("WS_PORT").unwrap_or_else(|_| "8081".to_string());
        let ws_addr = format!("0.0.0.0:{}", ws_port);

        let app = Router::new()
            .route("/ws", get(websocket_handler))
            .with_state(ws_config);

        let listener = tokio::net::TcpListener::bind(&ws_addr).await?;

        tracing::info!("🔌 WebSocket server listening on {}", ws_addr);

        Some(tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("WebSocket server error: {}", e);
            }
        }))
    };

    tracing::info!("✅ All services started successfully");
    tracing::info!("");
    tracing::info!("Available endpoints:");
    tracing::info!("  - HTTP:       http://{}", http_addr);
    #[cfg(feature = "tls")]
    if std::env::var("TLS_CERT").is_ok() {
        let https_port = std::env::var("HTTPS_PORT").unwrap_or_else(|_| "8443".to_string());
        tracing::info!("  - HTTPS:      https://0.0.0.0:{}", https_port);
    }
    #[cfg(feature = "websocket")]
    {
        let ws_port = std::env::var("WS_PORT").unwrap_or_else(|_| "8081".to_string());
        tracing::info!("  - WebSocket:  ws://0.0.0.0:{}/ws", ws_port);
    }
    #[cfg(feature = "metrics")]
    tracing::info!("  - Metrics:    http://0.0.0.0:9090/metrics");
    tracing::info!("");

    // Wait for all services
    http_handle.await.map_err(|e| Error::Io(std::io::Error::other(e)))?;

    #[cfg(feature = "tls")]
    if let Some(handle) = https_handle {
        handle.await.map_err(|e| Error::Io(std::io::Error::other(e)))?;
    }

    #[cfg(feature = "websocket")]
    if let Some(handle) = ws_handle {
        handle.await.map_err(|e| Error::Io(std::io::Error::other(e)))?;
    }

    Ok(())
}

// Message routing handlers
#[cfg(feature = "routing")]
fn handle_adt_a01(message: Message) -> Result<Message> {
    tracing::info!("📋 Handling ADT^A01 (Patient Admit)");
    // Custom logic for patient admission
    Ok(message)
}

#[cfg(feature = "routing")]
fn handle_adt_a04(message: Message) -> Result<Message> {
    tracing::info!("📋 Handling ADT^A04 (Patient Registration)");
    // Custom logic for patient registration
    Ok(message)
}

#[cfg(feature = "routing")]
fn handle_oru_r01(message: Message) -> Result<Message> {
    tracing::info!("🧪 Handling ORU^R01 (Observation Result)");
    // Custom logic for lab results
    Ok(message)
}

#[cfg(feature = "routing")]
fn handle_siu(message: Message) -> Result<Message> {
    tracing::info!("📅 Handling SIU (Scheduling Information)");
    // Custom logic for scheduling
    Ok(message)
}

#[cfg(feature = "routing")]
fn handle_default(message: Message) -> Result<Message> {
    tracing::info!(
        "📨 Handling message type: {:?}",
        message.get_message_type()
    );
    // Default handler for unrouted messages
    Ok(message)
}

// Metrics HTTP server
#[cfg(feature = "metrics")]
async fn serve_metrics(handle: metrics_exporter_prometheus::PrometheusHandle) {
    use axum::{routing::get, Router};

    let app = Router::new().route(
        "/metrics",
        get(|| async move { handle.render() }),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9090")
        .await
        .expect("Failed to bind metrics server");

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!("Metrics server error: {}", e);
    }
}
