//! WebSocket support for bidirectional HL7 message streaming
//!
//! This module provides WebSocket endpoints for real-time HL7 message exchange,
//! suitable for applications requiring bidirectional communication or message subscriptions.

#[cfg(feature = "websocket")]
use crate::{Error, Result};
#[cfg(feature = "websocket")]
use axum::{
    extract::{
        State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
#[cfg(feature = "websocket")]
use futures_util::{
    sink::SinkExt,
    stream::{SplitSink, StreamExt},
};
#[cfg(feature = "websocket")]
use std::sync::Arc;
#[cfg(feature = "websocket")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "websocket")]
use std::time::Duration;
#[cfg(feature = "websocket")]
use tokio::sync::broadcast;
#[cfg(feature = "websocket")]
use vyna_hl7_core::{CharacterSet, Message};
#[cfg(feature = "websocket")]
use vyna_hl7_parser::parse_message;

/// Message handler function type for WebSocket connections
///
/// Takes an HL7 message and returns a response message (typically an ACK).
/// The handler is wrapped in an Arc for thread-safe sharing.
#[cfg(feature = "websocket")]
pub type WsMessageHandler = Arc<dyn Fn(Message) -> Result<Message> + Send + Sync>;

/// One fan-out item: an HL7 message received on some connection.
#[cfg(feature = "websocket")]
#[derive(Clone, Debug)]
struct BroadcastFrame {
    /// Connection that received the message; it is skipped on delivery
    /// so a sender never gets its own message echoed back.
    sender_id: u64,
    /// The HL7 message text as received.
    text: String,
}

/// Connection ids for broadcast de-duplication. Id 0 is never issued.
#[cfg(feature = "websocket")]
static NEXT_WS_CONN_ID: AtomicU64 = AtomicU64::new(1);

/// Default idle timeout for server connections (see
/// [`WebSocketConfig::with_idle_timeout`]).
#[cfg(feature = "websocket")]
pub const DEFAULT_WS_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// WebSocket server configuration
#[cfg(feature = "websocket")]
#[derive(Clone)]
pub struct WebSocketConfig {
    /// Handler invoked for each incoming HL7 message
    pub handler: WsMessageHandler,
    /// Maximum accepted message size in bytes (default: 1 MB)
    pub max_message_size: usize,
    /// Close connections idle longer than this (default: 5 minutes).
    ///
    /// Any inbound frame or broadcast delivery resets the timer. `None`
    /// disables the timeout (connections stay open indefinitely).
    pub idle_timeout: Option<Duration>,
    /// Broadcast each successfully parsed incoming message to all other
    /// connected clients (the sender gets only its direct response).
    ///
    /// The broadcast channel is shared by every clone of this config, so
    /// serving one config via axum `State` (which clones it per
    /// connection) fans out across all connections. See
    /// [`WebSocketConfig::with_broadcast`].
    pub enable_broadcast: bool,
    /// Shared fan-out channel. Cloned configs share the same channel, so
    /// enabling broadcast on the served config wires every connection.
    broadcast_tx: broadcast::Sender<BroadcastFrame>,
}

#[cfg(feature = "websocket")]
impl WebSocketConfig {
    /// Create a new WebSocket configuration
    ///
    /// # Arguments
    ///
    /// * `handler` - Function that processes incoming HL7 messages
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "websocket")]
    /// # {
    /// use vyna_hl7_http::websocket::{WebSocketConfig, WsMessageHandler};
    /// use std::sync::Arc;
    ///
    /// let handler: WsMessageHandler = Arc::new(|msg| Ok(msg));
    /// let config = WebSocketConfig::new(handler);
    /// # }
    /// ```
    pub fn new(handler: WsMessageHandler) -> Self {
        let (broadcast_tx, _) = broadcast::channel(128);
        Self {
            handler,
            max_message_size: 1024 * 1024, // 1 MB default
            idle_timeout: Some(DEFAULT_WS_IDLE_TIMEOUT),
            enable_broadcast: false,
            broadcast_tx,
        }
    }

    /// Set maximum message size in bytes
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Enable broadcast mode: every successfully parsed incoming HL7
    /// message is fanned out to all other connected clients as a text
    /// frame, in addition to the direct response sent back to the sender.
    ///
    /// Serve the returned config (via axum `State`) so its clones share
    /// one broadcast channel across connections.
    pub fn with_broadcast(mut self) -> Self {
        self.enable_broadcast = true;
        self
    }

    /// Close connections idle longer than `timeout` (default: 5 minutes).
    ///
    /// Without a bound, dead peers accumulate a server task each forever.
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = Some(timeout);
        self
    }

    /// Disable the idle timeout (connections stay open indefinitely).
    pub fn without_idle_timeout(mut self) -> Self {
        self.idle_timeout = None;
        self
    }
}

#[cfg(feature = "websocket")]
impl Default for WebSocketConfig {
    fn default() -> Self {
        Self::new(Arc::new(Ok))
    }
}

/// WebSocket upgrade handler
///
/// This handler upgrades an HTTP connection to a WebSocket connection.
///
/// # Example
///
/// ```no_run
/// # #[cfg(feature = "websocket")]
/// # {
/// use axum::{Router, routing::get};
/// use vyna_hl7_http::websocket::{websocket_handler, WebSocketConfig, WsMessageHandler};
/// use std::sync::Arc;
///
/// # async fn example() {
/// let handler: WsMessageHandler = Arc::new(|msg| Ok(msg));
/// let config = WebSocketConfig::new(handler);
///
/// let app: Router = Router::new()
///     .route("/ws", get(websocket_handler))
///     .with_state(config);
/// # }
/// # }
/// ```
#[cfg(feature = "websocket")]
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(config): State<WebSocketConfig>,
) -> Response {
    ws.on_upgrade(move |socket| handle_websocket(socket, config))
}

/// Publish one received message to the shared broadcast channel.
///
/// No-op unless broadcast mode is enabled on the served config.
#[cfg(feature = "websocket")]
fn publish_incoming(config: &WebSocketConfig, conn_id: u64, text: &str) {
    if config.enable_broadcast {
        let _ = config.broadcast_tx.send(BroadcastFrame {
            sender_id: conn_id,
            text: text.to_string(),
        });
    }
}

/// Handle WebSocket connection lifecycle
#[cfg(feature = "websocket")]
async fn handle_websocket(socket: WebSocket, config: WebSocketConfig) {
    let (mut sender, mut receiver) = socket.split();
    let conn_id = NEXT_WS_CONN_ID.fetch_add(1, Ordering::Relaxed);
    let mut broadcast_rx = config.broadcast_tx.subscribe();

    #[cfg(feature = "logging")]
    tracing::info!("WebSocket connection established");

    loop {
        // Fresh deadline each iteration: any event below (inbound frame
        // or broadcast delivery) resets the idle timer.
        let idle = async {
            match config.idle_timeout {
                Some(timeout) => tokio::time::sleep(timeout).await,
                None => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(msg) => {
                        if !process_inbound(msg, &mut sender, &config, conn_id).await {
                            break;
                        }
                    }
                    None => break,
                }
            }
            frame = broadcast_rx.recv() => {
                match frame {
                    // Nothing is published unless broadcast mode is on, so
                    // this only fires for opted-in connections.
                    Ok(frame) if frame.sender_id != conn_id => {
                        if sender
                            .send(WsMessage::Text(frame.text.into()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(_) => {} // Own message; already answered directly.
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    // Unreachable in practice: every live connection holds
                    // a clone of the sender via its config. Break rather
                    // than spin, defensively.
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = idle => {
                #[cfg(feature = "logging")]
                tracing::info!("WebSocket connection closed after idle timeout");
                break;
            }
        }
    }

    #[cfg(feature = "logging")]
    tracing::info!("WebSocket connection terminated");
}

/// Process one inbound WebSocket message.
///
/// Returns `false` when the connection should be closed.
#[cfg(feature = "websocket")]
async fn process_inbound(
    msg: std::result::Result<WsMessage, axum::Error>,
    sender: &mut SplitSink<WebSocket, WsMessage>,
    config: &WebSocketConfig,
    conn_id: u64,
) -> bool {
    match msg {
        Ok(WsMessage::Text(text)) => {
            let text_str = text.as_str();
            #[cfg(feature = "logging")]
            tracing::debug!("Received text message: {} bytes", text_str.len());

            // Check message size
            if text_str.len() > config.max_message_size {
                #[cfg(feature = "logging")]
                tracing::warn!(
                    "Message exceeds maximum size: {} > {}",
                    text_str.len(),
                    config.max_message_size
                );

                let _ = sender
                    .send(WsMessage::Text(
                        format!(
                            "Error: Message too large ({} bytes, max {})",
                            text_str.len(),
                            config.max_message_size
                        )
                        .into(),
                    ))
                    .await;
                return true;
            }

            // Parse HL7 message
            match parse_message(text_str) {
                Ok(hl7_msg) => {
                    #[cfg(feature = "logging")]
                    {
                        tracing::info!(
                            message_type = ?hl7_msg.get_message_type(),
                            control_id = ?hl7_msg.get_control_id(),
                            "Received HL7 message via WebSocket"
                        );
                    }

                    publish_incoming(config, conn_id, text_str);

                    // Handle message
                    match (config.handler)(hl7_msg) {
                        Ok(response) => {
                            let response_text = response.encode();
                            if let Err(_e) =
                                sender.send(WsMessage::Text(response_text.into())).await
                            {
                                #[cfg(feature = "logging")]
                                tracing::error!("Failed to send response: {}", _e);
                                return false;
                            }
                        }
                        Err(_e) => {
                            #[cfg(feature = "logging")]
                            tracing::error!("Handler error: {}", _e);

                            let error_msg = format!("Error processing message: {}", _e);
                            let _ = sender.send(WsMessage::Text(error_msg.into())).await;
                        }
                    }
                }
                Err(e) => {
                    #[cfg(feature = "logging")]
                    tracing::warn!("Failed to parse HL7 message: {}", e);

                    let error_msg = format!("Error parsing HL7 message: {}", e);
                    let _ = sender.send(WsMessage::Text(error_msg.into())).await;
                }
            }
        }
        Ok(WsMessage::Binary(data)) => {
            #[cfg(feature = "logging")]
            tracing::debug!("Received binary message: {} bytes", data.len());

            // Enforce the same size cap as text frames.
            if data.len() > config.max_message_size {
                #[cfg(feature = "logging")]
                tracing::warn!(
                    "Message exceeds maximum size: {} > {}",
                    data.len(),
                    config.max_message_size
                );

                let _ = sender
                    .send(WsMessage::Text(
                        format!(
                            "Error: Message too large ({} bytes, max {})",
                            data.len(),
                            config.max_message_size
                        )
                        .into(),
                    ))
                    .await;
                return true;
            }

            // Decode binary using the MSH-18 declaration
            let text = match CharacterSet::decode_message(&data) {
                Ok(text) => text,
                Err(e) => {
                    #[cfg(feature = "logging")]
                    tracing::warn!("Binary message is not valid UTF-8: {}", e);

                    let error_msg = format!("Error decoding binary message: {}", e);
                    let _ = sender.send(WsMessage::Text(error_msg.into())).await;
                    return true;
                }
            };

            let hl7_msg = match parse_message(&text) {
                Ok(hl7_msg) => hl7_msg,
                Err(e) => {
                    #[cfg(feature = "logging")]
                    tracing::warn!("Failed to parse HL7 message: {}", e);

                    let error_msg = format!("Error parsing HL7 message: {}", e);
                    let _ = sender.send(WsMessage::Text(error_msg.into())).await;
                    return true;
                }
            };

            publish_incoming(config, conn_id, &text);

            match (config.handler)(hl7_msg) {
                Ok(response) => {
                    // Best effort: fall back to UTF-8 when the
                    // response cannot be encoded as declared.
                    let response_bytes = response
                        .encode_bytes()
                        .unwrap_or_else(|_| response.encode().into_bytes());
                    let _ = sender.send(WsMessage::Binary(response_bytes.into())).await;
                }
                Err(_e) => {
                    #[cfg(feature = "logging")]
                    tracing::error!("Handler error: {}", _e);

                    let error_msg = format!("Error processing message: {}", _e);
                    let _ = sender.send(WsMessage::Text(error_msg.into())).await;
                }
            }
        }
        Ok(WsMessage::Ping(data)) => {
            #[cfg(feature = "logging")]
            tracing::trace!("Received ping");

            let _ = sender.send(WsMessage::Pong(data)).await;
        }
        Ok(WsMessage::Pong(_)) => {
            #[cfg(feature = "logging")]
            tracing::trace!("Received pong");
        }
        Ok(WsMessage::Close(_frame)) => {
            #[cfg(feature = "logging")]
            tracing::info!("WebSocket connection closed: {:?}", _frame);
            return false;
        }
        Err(_e) => {
            #[cfg(feature = "logging")]
            tracing::error!("WebSocket error: {}", _e);
            return false;
        }
    }
    true
}

/// WebSocket broadcast channel
///
/// Enables broadcasting HL7 messages to all connected WebSocket clients.
#[cfg(feature = "websocket")]
pub struct BroadcastChannel {
    sender: broadcast::Sender<String>,
}

#[cfg(feature = "websocket")]
impl BroadcastChannel {
    /// Create a new broadcast channel
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of messages in the channel buffer
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "websocket")]
    /// # {
    /// use vyna_hl7_http::websocket::BroadcastChannel;
    ///
    /// let channel = BroadcastChannel::new(100);
    /// # }
    /// ```
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Broadcast an HL7 message to all connected clients
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message to broadcast
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "websocket")]
    /// # {
    /// use vyna_hl7_http::websocket::BroadcastChannel;
    /// use vyna_hl7_core::Message;
    ///
    /// # fn example(channel: BroadcastChannel, message: Message) {
    /// channel.broadcast(&message);
    /// # }
    /// # }
    /// ```
    pub fn broadcast(&self, message: &Message) {
        let encoded = message.encode();
        let _ = self.sender.send(encoded);
    }

    /// Get a subscriber for receiving broadcast messages
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.sender.subscribe()
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// Default bound on [`WebSocketClient::send_message`] waiting for a response.
#[cfg(feature = "websocket")]
pub const DEFAULT_WS_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);

/// WebSocket client for connecting to remote HL7 WebSocket servers
#[cfg(feature = "websocket")]
pub struct WebSocketClient {
    url: String,
    response_timeout: Duration,
}

#[cfg(feature = "websocket")]
impl WebSocketClient {
    /// Create a new WebSocket client
    ///
    /// # Arguments
    ///
    /// * `url` - The WebSocket URL (e.g., "ws://localhost:8080/ws")
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "websocket")]
    /// # {
    /// use vyna_hl7_http::websocket::WebSocketClient;
    ///
    /// let client = WebSocketClient::new("ws://localhost:8080/ws");
    /// # }
    /// ```
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            response_timeout: DEFAULT_WS_RESPONSE_TIMEOUT,
        }
    }

    /// Bound how long [`WebSocketClient::send_message`] waits for a
    /// response (default: 30 s).
    pub fn with_response_timeout(mut self, timeout: Duration) -> Self {
        self.response_timeout = timeout;
        self
    }

    /// Connect to the WebSocket server and process messages
    ///
    /// # Arguments
    ///
    /// * `message_callback` - Function called for each received message
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "websocket")]
    /// # {
    /// use vyna_hl7_http::websocket::WebSocketClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = WebSocketClient::new("ws://localhost:8080/ws");
    ///
    /// client.connect(|message| {
    ///     println!("Received: {:?}", message.get_message_type());
    /// }).await?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn connect<F>(self, message_callback: F) -> Result<()>
    where
        F: Fn(Message) + Send + 'static,
    {
        use tokio_tungstenite::connect_async;

        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        #[cfg(feature = "logging")]
        tracing::info!("Connected to WebSocket server: {}", self.url);

        let (_write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    if let Ok(hl7_msg) = parse_message(&text) {
                        message_callback(hl7_msg);
                    } else {
                        #[cfg(feature = "logging")]
                        tracing::warn!("Failed to parse received HL7 message");
                    }
                }
                Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => {
                    #[cfg(feature = "logging")]
                    tracing::info!("WebSocket connection closed by server");
                    break;
                }
                Err(e) => {
                    #[cfg(feature = "logging")]
                    tracing::error!("WebSocket error: {}", e);
                    return Err(Error::Io(std::io::Error::other(e)));
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Send a single message and receive response
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message to send
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "websocket")]
    /// # {
    /// use vyna_hl7_http::websocket::WebSocketClient;
    /// use vyna_hl7_core::Message;
    ///
    /// # async fn example(message: Message) -> Result<(), Box<dyn std::error::Error>> {
    /// let client = WebSocketClient::new("ws://localhost:8080/ws");
    ///
    /// let response = client.send_message(&message).await?;
    /// println!("Received ACK: {:?}", response.get_control_id());
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub async fn send_message(&self, message: &Message) -> Result<Message> {
        use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};

        let (ws_stream, _) = connect_async(&self.url)
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        let (mut write, mut read) = ws_stream.split();

        // Send message
        let encoded = message.encode();
        write
            .send(TungsteniteMessage::Text(encoded.into()))
            .await
            .map_err(|e| Error::Io(std::io::Error::other(e)))?;

        // Wait for response, bounded so a silent peer cannot hang us forever.
        let reply = tokio::time::timeout(self.response_timeout, read.next())
            .await
            .map_err(|_| {
                Error::Http(format!(
                    "WebSocket response timed out after {:?}",
                    self.response_timeout
                ))
            })?;
        if let Some(msg) = reply {
            match msg {
                Ok(TungsteniteMessage::Text(text)) => {
                    let response = parse_message(&text)?;
                    Ok(response)
                }
                Ok(_) => Err(Error::Http("Unexpected WebSocket message type".to_string())),
                Err(e) => Err(Error::Io(std::io::Error::other(e))),
            }
        } else {
            Err(Error::Http("No response received".to_string()))
        }
    }
}

#[cfg(test)]
#[cfg(feature = "websocket")]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config() {
        let handler: WsMessageHandler = Arc::new(Ok);
        let config = WebSocketConfig::new(handler)
            .with_max_message_size(2 * 1024 * 1024)
            .with_broadcast();

        assert_eq!(config.max_message_size, 2 * 1024 * 1024);
        assert!(config.enable_broadcast);
    }

    #[test]
    fn test_broadcast_channel() {
        let channel = BroadcastChannel::new(100);
        assert_eq!(channel.subscriber_count(), 0);

        let _sub1 = channel.subscribe();
        assert_eq!(channel.subscriber_count(), 1);

        let _sub2 = channel.subscribe();
        assert_eq!(channel.subscriber_count(), 2);
    }

    #[test]
    fn test_websocket_client_creation() {
        let client = WebSocketClient::new("ws://localhost:8080/ws");
        assert_eq!(client.url, "ws://localhost:8080/ws");
    }

    #[test]
    fn test_ws_timeout_defaults_and_overrides() {
        use std::time::Duration;

        let client = WebSocketClient::new("ws://localhost:8080/ws");
        assert_eq!(client.response_timeout, DEFAULT_WS_RESPONSE_TIMEOUT);
        let client = client.with_response_timeout(Duration::from_secs(5));
        assert_eq!(client.response_timeout, Duration::from_secs(5));

        let config = WebSocketConfig::new(Arc::new(Ok));
        assert_eq!(config.idle_timeout, Some(DEFAULT_WS_IDLE_TIMEOUT));
        let config = config.with_idle_timeout(Duration::from_secs(60));
        assert_eq!(config.idle_timeout, Some(Duration::from_secs(60)));
        let config = config.without_idle_timeout();
        assert_eq!(config.idle_timeout, None);
    }

    /// A server that never responds must not hang `send_message` forever.
    #[tokio::test]
    async fn test_client_response_timeout() {
        use axum::extract::ws::{WebSocket, WebSocketUpgrade};
        use axum::response::Response;
        use axum::{Router, routing::get};
        use std::time::{Duration, Instant};

        async fn black_hole(ws: WebSocketUpgrade) -> Response {
            ws.on_upgrade(
                |mut socket: WebSocket| async move { while socket.recv().await.is_some() {} },
            )
        }
        let app = Router::new().route("/ws", get(black_hole));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let client = WebSocketClient::new(format!("ws://{addr}/ws"))
            .with_response_timeout(Duration::from_millis(100));
        let msg = parse_message("MSH|^~\\&|S|A|R|F|20240101||ADT^A01|T1|P|2.5\r").unwrap();
        let start = Instant::now();
        let err = client.send_message(&msg).await.unwrap_err();
        assert!(start.elapsed() < Duration::from_secs(5));
        assert!(err.to_string().contains("timed out"), "unexpected: {err}");
    }

    /// Idle server connections are closed once the idle timeout elapses.
    #[tokio::test]
    async fn test_server_idle_timeout_closes_connection() {
        use axum::{Router, routing::get};
        use std::time::Duration;
        use tokio_tungstenite::connect_async;

        let handler: WsMessageHandler = Arc::new(Ok);
        let config = WebSocketConfig::new(handler).with_idle_timeout(Duration::from_millis(100));
        let app = Router::new()
            .route("/ws", get(websocket_handler))
            .with_state(config);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
        // Stay idle: the server must close us promptly.
        tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("idle connection was never closed by the server");
    }

    /// Repro for finding 4: with broadcast enabled, a message received
    /// from one client must reach the other connected clients.
    #[tokio::test]
    async fn test_broadcast_reaches_other_clients() {
        use axum::{Router, routing::get};
        use tokio_tungstenite::{connect_async, tungstenite::Message as TMessage};

        let handler: WsMessageHandler = Arc::new(Ok);
        let config = WebSocketConfig::new(handler).with_broadcast();
        let app = Router::new()
            .route("/ws", get(websocket_handler))
            .with_state(config);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let (mut ws_a, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();
        let (mut ws_b, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();

        // A sends a valid HL7 message; A gets the echo response ...
        let hl7 = "MSH|^~\\&|S|A|R|F|20240101||ADT^A01|BC1|P|2.5\r";
        ws_a.send(TMessage::Text(hl7.into())).await.unwrap();
        let reply = tokio::time::timeout(std::time::Duration::from_secs(5), ws_a.next())
            .await
            .expect("sender should get its echo response")
            .unwrap()
            .unwrap();
        assert!(
            matches!(&reply, TMessage::Text(t) if t.as_str().contains("BC1")),
            "unexpected reply to sender: {reply:?}"
        );

        // ... and B must receive the broadcast of A's message.
        let bcast = tokio::time::timeout(std::time::Duration::from_secs(5), ws_b.next())
            .await
            .expect("broadcast was not delivered to the other client")
            .unwrap()
            .unwrap();
        assert!(
            matches!(&bcast, TMessage::Text(t) if t.as_str().contains("BC1")),
            "unexpected broadcast: {bcast:?}"
        );

        // The sender must not get its own message echoed back as a broadcast.
        let nothing =
            tokio::time::timeout(std::time::Duration::from_millis(300), ws_a.next()).await;
        assert!(
            nothing.is_err(),
            "sender received its own broadcast echo: {nothing:?}"
        );
    }

    #[tokio::test]
    async fn test_binary_frames_size_limit_and_error_frames() {
        use axum::{Router, routing::get};
        use tokio_tungstenite::{connect_async, tungstenite::Message as TMessage};

        // Echo handler that rejects control id "FAIL".
        let handler: WsMessageHandler = Arc::new(|msg: Message| {
            if msg.get_control_id() == Some("FAIL") {
                return Err(Error::Http("boom".to_string()));
            }
            Ok(msg)
        });
        let config = WebSocketConfig::new(handler).with_max_message_size(64);
        let app = Router::new()
            .route("/ws", get(websocket_handler))
            .with_state(config);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let (mut ws, _) = connect_async(format!("ws://{addr}/ws")).await.unwrap();

        // 1. Oversized binary frame -> "too large" text error.
        ws.send(TMessage::Binary(vec![b'A'; 65].into()))
            .await
            .unwrap();
        let reply = ws.next().await.unwrap().unwrap();
        assert!(
            matches!(&reply, TMessage::Text(t) if t.as_str().contains("too large")),
            "unexpected reply: {reply:?}"
        );

        // 2. Undecodable bytes (UTF-16 BOM) -> decode error text.
        ws.send(TMessage::Binary(vec![0xff, 0xfe, 0xfd].into()))
            .await
            .unwrap();
        let reply = ws.next().await.unwrap().unwrap();
        assert!(
            matches!(&reply, TMessage::Text(t)
                if t.as_str().contains("Error decoding")),
            "unexpected reply: {reply:?}"
        );

        // 3. Valid UTF-8 but not HL7 -> parse error text.
        ws.send(TMessage::Binary(b"NOT AN HL7 MESSAGE".to_vec().into()))
            .await
            .unwrap();
        let reply = ws.next().await.unwrap().unwrap();
        assert!(
            matches!(&reply, TMessage::Text(t)
                if t.as_str().contains("Error parsing")),
            "unexpected reply: {reply:?}"
        );

        // 4. Handler failure -> processing error text.
        let failing = b"MSH|^~\\&|S|A|R|F|20240101||ADT^A01|FAIL|P|2.5\r";
        ws.send(TMessage::Binary(failing.to_vec().into()))
            .await
            .unwrap();
        let reply = ws.next().await.unwrap().unwrap();
        assert!(
            matches!(&reply, TMessage::Text(t)
                if t.as_str().contains("Error processing")),
            "unexpected reply: {reply:?}"
        );

        // 5. Valid binary frame still echoes a binary response.
        let valid = b"MSH|^~\\&|S|A|R|F|20240101||ADT^A01|OK1|P|2.5\r";
        ws.send(TMessage::Binary(valid.to_vec().into()))
            .await
            .unwrap();
        let reply = ws.next().await.unwrap().unwrap();
        match &reply {
            TMessage::Binary(bytes) => {
                let text = std::str::from_utf8(bytes).unwrap();
                let echoed = parse_message(text).unwrap();
                assert_eq!(echoed.get_control_id(), Some("OK1"));
            }
            other => panic!("unexpected reply: {other:?}"),
        }
    }
}
