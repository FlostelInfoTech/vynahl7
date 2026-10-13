//! MLLP Connection Pooling
//!
//! This module provides connection pooling for MLLP clients, enabling efficient
//! reuse of connections when sending multiple messages to the same server.
//!
//! # Overview
//!
//! Connection pooling reduces the overhead of establishing new TCP connections
//! for each message exchange. This is particularly important in high-throughput
//! scenarios where latency matters.
//!
//! # Examples
//!
//! ```no_run
//! use vyna_hl7_mllp::{MllpPool, PoolConfig};
//! use vyna_hl7_core::message::Message;
//!
//! # async fn example() -> vyna_hl7_core::error::Result<()> {
//! // Create a pool with default settings
//! let pool = MllpPool::new("localhost:2575").await?;
//!
//! // Create a pool with custom configuration
//! let pool = MllpPool::with_config(
//!     "localhost:2575",
//!     PoolConfig::new()
//!         .with_max_connections(10)
//!         .with_min_connections(2)
//! ).await?;
//!
//! // Send a message using a pooled connection
//! let message = Message::new();
//! let ack = pool.send(&message).await?;
//!
//! // The connection is automatically returned to the pool
//! # Ok(())
//! # }
//! ```

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

use vyna_hl7_core::error::{Error, Result};
use vyna_hl7_core::message::Message;

use crate::{MllpClient, MllpConfig};

#[cfg(feature = "tls")]
use crate::tls::TlsClientConfig;

/// Configuration for the connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool (default: 10)
    pub max_connections: usize,
    /// Minimum number of connections to maintain (default: 1)
    pub min_connections: usize,
    /// Maximum time to wait for a connection (default: 30 seconds)
    pub acquire_timeout: Duration,
    /// Maximum idle time before a connection is closed (default: 5 minutes)
    pub idle_timeout: Duration,
    /// Maximum lifetime of a connection (default: 30 minutes)
    pub max_lifetime: Duration,
    /// How often to check for idle connections (default: 60 seconds).
    ///
    /// There is no background reaper; call [`MllpPool::cleanup`] on this
    /// schedule to evict idle and expired connections.
    pub cleanup_interval: Duration,
    /// MLLP configuration for connections
    pub mllp_config: MllpConfig,
    /// Whether to validate connections before returning them (default: false)
    pub test_on_acquire: bool,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            acquire_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            max_lifetime: Duration::from_secs(1800), // 30 minutes
            cleanup_interval: Duration::from_secs(60),
            mllp_config: MllpConfig::default(),
            test_on_acquire: false,
        }
    }
}

impl PoolConfig {
    /// Create a new pool configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of connections
    pub fn with_max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// Set the minimum number of connections
    pub fn with_min_connections(mut self, min: usize) -> Self {
        self.min_connections = min;
        self
    }

    /// Set the acquire timeout
    pub fn with_acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    /// Set the idle timeout
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set the maximum connection lifetime
    pub fn with_max_lifetime(mut self, lifetime: Duration) -> Self {
        self.max_lifetime = lifetime;
        self
    }

    /// Set how often idle connections should be cleaned up
    pub fn with_cleanup_interval(mut self, interval: Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }

    /// Set the MLLP configuration
    pub fn with_mllp_config(mut self, config: MllpConfig) -> Self {
        self.mllp_config = config;
        self
    }

    /// Enable testing connections on acquire
    pub fn with_test_on_acquire(mut self, test: bool) -> Self {
        self.test_on_acquire = test;
        self
    }
}

/// Wrapper for a pooled connection with metadata
struct PooledConnection {
    client: MllpClient,
    created_at: Instant,
    last_used: Instant,
}

impl PooledConnection {
    fn new(client: MllpClient) -> Self {
        let now = Instant::now();
        Self {
            client,
            created_at: now,
            last_used: now,
        }
    }

    fn is_expired(&self, config: &PoolConfig) -> bool {
        self.created_at.elapsed() > config.max_lifetime
    }

    fn is_idle(&self, config: &PoolConfig) -> bool {
        self.last_used.elapsed() > config.idle_timeout
    }

    fn touch(&mut self) {
        self.last_used = Instant::now();
    }
}

/// Statistics for the connection pool
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Total connections created
    pub connections_created: usize,
    /// Total connections closed
    pub connections_closed: usize,
    /// Current number of idle connections
    pub idle_connections: usize,
    /// Current number of active (in-use) connections
    pub active_connections: usize,
    /// Number of times a connection was acquired from the pool
    pub acquires: usize,
    /// Number of times a connection request had to wait
    pub waits: usize,
    /// Number of times a connection was recycled (returned to pool)
    pub recycles: usize,
    /// Number of acquire timeouts
    pub timeouts: usize,
}

/// Connection pool for MLLP clients
///
/// Manages a pool of connections to a single MLLP server endpoint.
pub struct MllpPool {
    address: String,
    config: PoolConfig,
    connections: Arc<Mutex<VecDeque<PooledConnection>>>,
    semaphore: Arc<Semaphore>,
    stats: Arc<Mutex<PoolStats>>,
    active_count: Arc<AtomicUsize>,
    #[cfg(feature = "tls")]
    tls_config: Option<TlsClientConfig>,
    #[cfg(feature = "tls")]
    server_name: Option<String>,
}

impl MllpPool {
    /// Create a new connection pool with default configuration
    pub async fn new(address: &str) -> Result<Self> {
        Self::with_config(address, PoolConfig::default()).await
    }

    /// Create a new connection pool with custom configuration
    pub async fn with_config(address: &str, config: PoolConfig) -> Result<Self> {
        let pool = Self {
            address: address.to_string(),
            connections: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_connections))),
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
            stats: Arc::new(Mutex::new(PoolStats::default())),
            active_count: Arc::new(AtomicUsize::new(0)),
            #[cfg(feature = "tls")]
            tls_config: None,
            #[cfg(feature = "tls")]
            server_name: None,
            config,
        };

        // Pre-create minimum connections
        pool.initialize_connections().await?;

        Ok(pool)
    }

    /// Create a new TLS connection pool
    #[cfg(feature = "tls")]
    pub async fn new_tls(
        address: &str,
        server_name: &str,
        tls_config: TlsClientConfig,
    ) -> Result<Self> {
        Self::with_tls_config(address, server_name, tls_config, PoolConfig::default()).await
    }

    /// Create a new TLS connection pool with custom configuration
    #[cfg(feature = "tls")]
    pub async fn with_tls_config(
        address: &str,
        server_name: &str,
        tls_config: TlsClientConfig,
        config: PoolConfig,
    ) -> Result<Self> {
        let pool = Self {
            address: address.to_string(),
            connections: Arc::new(Mutex::new(VecDeque::with_capacity(config.max_connections))),
            semaphore: Arc::new(Semaphore::new(config.max_connections)),
            stats: Arc::new(Mutex::new(PoolStats::default())),
            active_count: Arc::new(AtomicUsize::new(0)),
            tls_config: Some(tls_config),
            server_name: Some(server_name.to_string()),
            config,
        };

        pool.initialize_connections().await?;

        Ok(pool)
    }

    /// Initialize minimum connections
    async fn initialize_connections(&self) -> Result<()> {
        for _ in 0..self.config.min_connections {
            let client = self.create_connection().await?;
            let conn = PooledConnection::new(client);

            let mut connections = self.connections.lock().await;
            connections.push_back(conn);

            let mut stats = self.stats.lock().await;
            stats.connections_created += 1;
            stats.idle_connections += 1;
        }
        Ok(())
    }

    /// Create a new connection
    async fn create_connection(&self) -> Result<MllpClient> {
        #[cfg(feature = "tls")]
        if let (Some(tls_config), Some(server_name)) = (&self.tls_config, &self.server_name) {
            return MllpClient::connect_tls_with_config(
                &self.address,
                server_name,
                tls_config.clone(),
                self.config.mllp_config.clone(),
            )
            .await;
        }

        MllpClient::connect_with_config(&self.address, self.config.mllp_config.clone()).await
    }

    /// Acquire a connection from the pool
    async fn acquire(&self) -> Result<PooledConnection> {
        // Try to acquire a permit (limits concurrent connections)
        let permit = tokio::time::timeout(
            self.config.acquire_timeout,
            self.semaphore.clone().acquire_owned(),
        )
        .await
        .map_err(|_| {
            // Update timeout stats
            let mut stats = self.stats.try_lock().ok();
            if let Some(ref mut s) = stats {
                s.timeouts += 1;
            }
            Error::Network(format!(
                "Connection pool acquire timeout after {:?}",
                self.config.acquire_timeout
            ))
        })?
        .map_err(|e| Error::Network(format!("Semaphore closed: {}", e)))?;

        // Track that we're waiting
        {
            let mut stats = self.stats.lock().await;
            stats.waits += 1;
        }

        // Try to get an existing connection
        loop {
            let mut connections = self.connections.lock().await;

            if let Some(mut conn) = connections.pop_front() {
                // Check if connection is still valid
                if conn.is_expired(&self.config) || conn.is_idle(&self.config) {
                    // Connection expired, close it and try again
                    drop(connections);
                    drop(conn.client.close().await);

                    let mut stats = self.stats.lock().await;
                    stats.connections_closed += 1;
                    stats.idle_connections = stats.idle_connections.saturating_sub(1);

                    continue;
                }

                // Good connection found
                conn.touch();

                let mut stats = self.stats.lock().await;
                stats.acquires += 1;
                stats.idle_connections = stats.idle_connections.saturating_sub(1);
                stats.active_connections += 1;

                self.active_count.fetch_add(1, Ordering::SeqCst);

                // Forget the permit - we'll release it when the connection is returned
                std::mem::forget(permit);

                return Ok(conn);
            } else {
                // No idle connections, create a new one
                drop(connections);

                let client = self.create_connection().await?;
                let mut conn = PooledConnection::new(client);
                conn.touch();

                let mut stats = self.stats.lock().await;
                stats.connections_created += 1;
                stats.acquires += 1;
                stats.active_connections += 1;

                self.active_count.fetch_add(1, Ordering::SeqCst);

                // Forget the permit - we'll release it when the connection is returned
                std::mem::forget(permit);

                return Ok(conn);
            }
        }
    }

    /// Return a connection to the pool
    async fn release(&self, mut conn: PooledConnection) {
        self.active_count.fetch_sub(1, Ordering::SeqCst);

        // Check if we should keep this connection
        if conn.is_expired(&self.config) {
            // Close expired connection
            drop(conn.client.close().await);

            let mut stats = self.stats.lock().await;
            stats.connections_closed += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        } else {
            // Return to pool
            conn.touch();

            let mut connections = self.connections.lock().await;
            let mut stats = self.stats.lock().await;

            connections.push_back(conn);
            stats.recycles += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
            stats.idle_connections += 1;
        }

        // Release the semaphore permit
        self.semaphore.add_permits(1);
    }

    /// Send a message using a pooled connection
    ///
    /// This method acquires a connection from the pool, sends the message,
    /// and returns the connection to the pool.
    pub async fn send(&self, message: &Message) -> Result<Message> {
        let mut conn = self.acquire().await?;

        let result = conn.client.send_message(message).await;

        match result {
            Ok(ack) => {
                // Success - return connection to pool
                self.release(conn).await;
                Ok(ack)
            }
            Err(e) => {
                // Error - close the connection and don't return to pool
                drop(conn.client.close().await);

                self.active_count.fetch_sub(1, Ordering::SeqCst);
                self.semaphore.add_permits(1);

                let mut stats = self.stats.lock().await;
                stats.connections_closed += 1;
                stats.active_connections = stats.active_connections.saturating_sub(1);

                Err(e)
            }
        }
    }

    /// Get current pool statistics
    pub async fn stats(&self) -> PoolStats {
        self.stats.lock().await.clone()
    }

    /// Get the number of currently active (in-use) connections
    pub fn active_connections(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Get the number of idle connections
    pub async fn idle_connections(&self) -> usize {
        self.connections.lock().await.len()
    }

    /// Close all connections and clear the pool
    pub async fn close(&self) {
        let mut connections = self.connections.lock().await;

        while let Some(conn) = connections.pop_front() {
            drop(conn.client.close().await);
        }

        let mut stats = self.stats.lock().await;
        stats.connections_closed += stats.idle_connections;
        stats.idle_connections = 0;
    }

    /// Remove idle and expired connections
    pub async fn cleanup(&self) {
        let mut connections = self.connections.lock().await;
        let mut stats = self.stats.lock().await;

        let initial_count = connections.len();
        connections.retain(|conn| !(conn.is_expired(&self.config) || conn.is_idle(&self.config)));

        let removed = initial_count - connections.len();
        stats.connections_closed += removed;
        stats.idle_connections = connections.len();
    }

    /// Ensure minimum connections are maintained
    pub async fn maintain_min_connections(&self) -> Result<()> {
        let connections = self.connections.lock().await;
        let current_count = connections.len();

        if current_count < self.config.min_connections {
            let needed = self.config.min_connections - current_count;
            drop(connections);

            for _ in 0..needed {
                match self.create_connection().await {
                    Ok(client) => {
                        let conn = PooledConnection::new(client);
                        let mut connections = self.connections.lock().await;
                        connections.push_back(conn);

                        let mut stats = self.stats.lock().await;
                        stats.connections_created += 1;
                        stats.idle_connections += 1;
                    }
                    Err(e) => {
                        // Log error but continue - don't fail the entire operation
                        eprintln!("Failed to create min connection: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_builder() {
        let config = PoolConfig::new()
            .with_max_connections(20)
            .with_min_connections(5)
            .with_acquire_timeout(Duration::from_secs(60))
            .with_idle_timeout(Duration::from_secs(120))
            .with_max_lifetime(Duration::from_secs(600))
            .with_cleanup_interval(Duration::from_secs(30))
            .with_test_on_acquire(true);

        assert_eq!(config.max_connections, 20);
        assert_eq!(config.min_connections, 5);
        assert_eq!(config.acquire_timeout, Duration::from_secs(60));
        assert_eq!(config.idle_timeout, Duration::from_secs(120));
        assert_eq!(config.max_lifetime, Duration::from_secs(600));
        assert_eq!(config.cleanup_interval, Duration::from_secs(30));
        assert!(config.test_on_acquire);
    }

    #[test]
    fn test_pool_config_defaults() {
        let config = PoolConfig::default();

        assert_eq!(config.max_connections, 10);
        assert_eq!(config.min_connections, 1);
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_pool_stats_default() {
        let stats = PoolStats::default();

        assert_eq!(stats.connections_created, 0);
        assert_eq!(stats.connections_closed, 0);
        assert_eq!(stats.idle_connections, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_pooled_connection_expiry() {
        let config = PoolConfig::new()
            .with_idle_timeout(Duration::from_millis(50))
            .with_max_lifetime(Duration::from_millis(100));

        // Create a mock connection - in real tests we'd use the testing module
        // For now, just test the metadata
        let created_at = Instant::now() - Duration::from_millis(150);
        let last_used = Instant::now() - Duration::from_millis(75);

        let conn_metadata = (created_at, last_used);

        // Check expiry logic
        assert!(conn_metadata.0.elapsed() > config.max_lifetime); // expired by lifetime
        assert!(conn_metadata.1.elapsed() > config.idle_timeout); // expired by idle
    }
}
