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
use futures_util::{sink::SinkExt, stream::StreamExt};
#[cfg(feature = "websocket")]
use std::sync::Arc;
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

/// WebSocket server configuration
#[cfg(feature = "websocket")]
#[derive(Clone)]
pub struct WebSocketConfig {
    /// Handler invoked for each incoming HL7 message
    pub handler: WsMessageHandler,
    /// Maximum accepted message size in bytes (default: 1 MB)
    pub max_message_size: usize,
    /// Broadcast each message to all connected clients
    pub enable_broadcast: bool,
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
        Self {
            handler,
            max_message_size: 1024 * 1024, // 1 MB default
            enable_broadcast: false,
        }
    }

    /// Set maximum message size in bytes
    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = size;
        self
    }

    /// Enable broadcast mode (messages sent to all connected clients)
    pub fn with_broadcast(mut self) -> Self {
        self.enable_broadcast = true;
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

/// Handle WebSocket connection lifecycle
#[cfg(feature = "websocket")]
async fn handle_websocket(socket: WebSocket, config: WebSocketConfig) {
    let (mut sender, mut receiver) = socket.split();

    #[cfg(feature = "logging")]
    tracing::info!("WebSocket connection established");

    while let Some(msg) = receiver.next().await {
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
                    continue;
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

                        // Handle message
                        match (config.handler)(hl7_msg) {
                            Ok(response) => {
                                let response_text = response.encode();
                                if let Err(_e) =
                                    sender.send(WsMessage::Text(response_text.into())).await
                                {
                                    #[cfg(feature = "logging")]
                                    tracing::error!("Failed to send response: {}", _e);
                                    break;
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
                    continue;
                }

                // Decode binary using the MSH-18 declaration
                let text = match CharacterSet::decode_message(&data) {
                    Ok(text) => text,
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        tracing::warn!("Binary message is not valid UTF-8: {}", e);

                        let error_msg = format!("Error decoding binary message: {}", e);
                        let _ = sender.send(WsMessage::Text(error_msg.into())).await;
                        continue;
                    }
                };

                let hl7_msg = match parse_message(&text) {
                    Ok(hl7_msg) => hl7_msg,
                    Err(e) => {
                        #[cfg(feature = "logging")]
                        tracing::warn!("Failed to parse HL7 message: {}", e);

                        let error_msg = format!("Error parsing HL7 message: {}", e);
                        let _ = sender.send(WsMessage::Text(error_msg.into())).await;
                        continue;
                    }
                };

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
                break;
            }
            Err(_e) => {
                #[cfg(feature = "logging")]
                tracing::error!("WebSocket error: {}", _e);
                break;
            }
        }
    }

    #[cfg(feature = "logging")]
    tracing::info!("WebSocket connection terminated");
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

/// WebSocket client for connecting to remote HL7 WebSocket servers
#[cfg(feature = "websocket")]
pub struct WebSocketClient {
    url: String,
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
        Self { url: url.into() }
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

        // Wait for response
        if let Some(msg) = read.next().await {
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
