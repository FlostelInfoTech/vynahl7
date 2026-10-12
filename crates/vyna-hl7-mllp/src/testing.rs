//! Testing utilities for MLLP
//!
//! This module provides mock servers and test helpers for integration testing.

use crate::{MllpConfig, MllpServer};
use vyna_hl7_core::{error::Result, Message};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Message handler function type
pub type MessageHandler = Arc<dyn Fn(Message) -> Result<Message> + Send + Sync>;

/// Mock MLLP server for integration testing
///
/// Provides an in-process MLLP server with configurable message handlers,
/// automatic port allocation, and cleanup.
///
/// # Example
///
/// ```no_run
/// use vyna_hl7_mllp::testing::MockMllpServer;
/// use vyna_hl7_core::Message;
///
/// #[tokio::test]
/// async fn test_mllp_client() {
///     // Create mock server with custom handler
///     let mut server = MockMllpServer::new()
///         .with_handler(|msg| {
///             // Echo the message back
///             Ok(msg)
///         })
///         .start()
///         .await
///         .unwrap();
///
///     let addr = server.local_addr();
///
///     // Use the server in tests...
///     // The server will automatically shut down when dropped
/// }
/// ```
pub struct MockMllpServer {
    handler: MessageHandler,
    config: MllpConfig,
    #[cfg(feature = "tls")]
    tls_config: Option<crate::tls::TlsServerConfig>,
    server_task: Option<tokio::task::JoinHandle<()>>,
    local_addr: Option<SocketAddr>,
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

impl MockMllpServer {
    /// Create a new mock MLLP server with default configuration
    ///
    /// By default, the server echoes received messages back to the client.
    pub fn new() -> Self {
        Self {
            handler: Arc::new(|msg| Ok(msg.clone())),
            config: MllpConfig::default(),
            #[cfg(feature = "tls")]
            tls_config: None,
            server_task: None,
            local_addr: None,
            shutdown_tx: None,
        }
    }

    /// Set a custom message handler
    ///
    /// The handler receives each message and returns a response message.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vyna_hl7_mllp::testing::MockMllpServer;
    /// use vyna_hl7_core::{Message, Segment, Field};
    ///
    /// let server = MockMllpServer::new()
    ///     .with_handler(|_msg| {
    ///         // Return a custom ACK
    ///         let mut ack = Message::default();
    ///         let mut msa = Segment::new("MSA");
    ///         msa.fields.push(Field::from_value("AA"));
    ///         ack.segments.push(msa);
    ///         Ok(ack)
    ///     });
    /// ```
    pub fn with_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(Message) -> Result<Message> + Send + Sync + 'static,
    {
        self.handler = Arc::new(handler);
        self
    }

    /// Set custom MLLP configuration
    pub fn with_config(mut self, config: MllpConfig) -> Self {
        self.config = config;
        self
    }

    /// Enable TLS with the given configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "tls")]
    /// # async fn example() -> vyna_hl7_core::error::Result<()> {
    /// use vyna_hl7_mllp::testing::MockMllpServer;
    /// use vyna_hl7_mllp::tls::TlsServerConfig;
    ///
    /// let tls_config = TlsServerConfig::new("server-cert.pem", "server-key.pem")?;
    /// let server = MockMllpServer::new()
    ///     .with_tls(tls_config)
    ///     .start()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, tls_config: crate::tls::TlsServerConfig) -> Self {
        self.tls_config = Some(tls_config);
        self
    }

    /// Start the mock server
    ///
    /// The server will bind to a random available port on localhost and
    /// begin accepting connections.
    pub async fn start(mut self) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let shutdown_rx = Arc::new(Mutex::new(Some(shutdown_rx)));

        // Bind to random port
        #[cfg(feature = "tls")]
        let server = if let Some(ref tls_config) = self.tls_config {
            MllpServer::bind_tls_with_config("127.0.0.1:0", tls_config.clone(), self.config.clone()).await?
        } else {
            MllpServer::bind_with_config("127.0.0.1:0", self.config.clone()).await?
        };

        #[cfg(not(feature = "tls"))]
        let server = MllpServer::bind_with_config("127.0.0.1:0", self.config.clone()).await?;

        let local_addr = server.local_addr()?;
        self.local_addr = Some(local_addr);

        let handler = self.handler.clone();

        // Spawn server task
        let server_task = tokio::spawn(async move {
            let shutdown_rx = shutdown_rx;

            loop {
                // Accept connection with timeout to allow checking shutdown
                let conn_result = tokio::time::timeout(
                    std::time::Duration::from_millis(100),
                    server.accept()
                ).await;

                // Check if shutdown was signaled
                let mut rx_guard = shutdown_rx.lock().await;
                if let Some(ref mut rx) = *rx_guard {
                    match rx.try_recv() {
                        Ok(_) | Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                            break;
                        }
                        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                            // Continue
                        }
                    }
                }
                drop(rx_guard);

                let mut conn = match conn_result {
                    Ok(Ok(conn)) => conn,
                    Ok(Err(_e)) => continue, // Accept error
                    Err(_) => continue, // Timeout, check shutdown again
                };

                let handler = handler.clone();

                // Handle connection
                tokio::spawn(async move {
                    loop {
                        match conn.receive_message().await {
                            Ok(msg) => {
                                // Process message with handler
                                match handler(msg) {
                                    Ok(response) => {
                                        if let Err(_e) = conn.send_message(&response).await {
                                            break; // Send failed
                                        }
                                    }
                                    Err(_e) => {
                                        break; // Handler error
                                    }
                                }
                            }
                            Err(_e) => {
                                break; // Receive error or connection closed
                            }
                        }
                    }
                });
            }
        });

        self.server_task = Some(server_task);
        self.shutdown_tx = Some(shutdown_tx);

        Ok(self)
    }

    /// Get the local address the server is bound to
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr.expect("Server not started")
    }

    /// Get the URL to connect to this server
    pub fn url(&self) -> String {
        format!("{}", self.local_addr())
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(task) = self.server_task.take() {
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                task
            ).await;
        }

        Ok(())
    }
}

impl Default for MockMllpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MockMllpServer {
    fn drop(&mut self) {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }

        if let Some(task) = self.server_task.take() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MllpClient;
    use vyna_hl7_core::{Field, Segment};

    #[tokio::test]
    async fn test_mock_server_basic() {
        // Create and start mock server
        let server = MockMllpServer::new()
            .start()
            .await
            .unwrap();

        let addr = server.url();

        // Connect client
        let mut client = MllpClient::connect(&addr).await.unwrap();

        // Create test message
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msh.fields.push(Field::from_value("TEST"));
        msg.segments.push(msh);

        // Send and receive
        let response = client.send_message(&msg).await.unwrap();

        // Server echoes by default
        assert_eq!(msg.encode(), response.encode());

        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_server_custom_handler() {
        // Create server with custom handler
        let server = MockMllpServer::new()
            .with_handler(|_msg| {
                let mut ack = Message::default();

                // Create MSH segment
                let mut msh = Segment::new("MSH");
                msh.fields.push(Field::from_value("|"));
                msh.fields.push(Field::from_value("^~\\&"));
                ack.segments.push(msh);

                // Create MSA segment
                let mut msa = Segment::new("MSA");
                msa.fields.push(Field::from_value("AA"));
                msa.fields.push(Field::from_value("TEST001"));
                ack.segments.push(msa);

                Ok(ack)
            })
            .start()
            .await
            .unwrap();

        let addr = server.url();

        // Connect and send message
        let mut client = MllpClient::connect(&addr).await.unwrap();

        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msh.fields.push(Field::from_value("^~\\&"));
        msg.segments.push(msh);

        let response = client.send_message(&msg).await.unwrap();

        // Check custom response - should have MSH first, then MSA
        assert_eq!(response.segments[0].id, "MSH");
        assert_eq!(response.segments[1].id, "MSA");
        assert_eq!(
            response.segments[1].fields[0].value(),
            Some("AA")
        );

        client.close().await.unwrap();
        server.shutdown().await.unwrap();
    }
}
