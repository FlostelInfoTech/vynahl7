//! Async message queue for buffering HL7 messages
//!
//! This module provides an asynchronous message queue for buffering and processing
//! HL7 messages. It's useful for decoupling message reception from processing,
//! handling backpressure, and implementing retry logic.

#[cfg(feature = "queue")]
use async_channel::{Receiver, Sender, bounded, unbounded};
#[cfg(feature = "queue")]
use std::sync::Arc;
#[cfg(feature = "queue")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "queue")]
use vyna_hl7_core::Message;

/// Async message queue for HL7 messages
///
/// Provides a thread-safe, async-aware queue for buffering HL7 messages
/// between producers and consumers.
#[cfg(feature = "queue")]
pub struct MessageQueue {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    stats: Arc<QueueStats>,
}

/// Queue statistics for monitoring
#[cfg(feature = "queue")]
#[derive(Debug, Clone)]
pub struct QueueStats {
    /// Total messages enqueued
    pub enqueued: Arc<AtomicU64>,
    /// Total messages dequeued
    pub dequeued: Arc<AtomicU64>,
    /// Total messages dropped (for bounded queues when full)
    pub dropped: Arc<AtomicU64>,
}

#[cfg(feature = "queue")]
impl QueueStats {
    fn new() -> Self {
        Self {
            enqueued: Arc::new(AtomicU64::new(0)),
            dequeued: Arc::new(AtomicU64::new(0)),
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the number of messages currently in the queue (approximate)
    pub fn pending(&self) -> u64 {
        let enqueued = self.enqueued.load(Ordering::Relaxed);
        let dequeued = self.dequeued.load(Ordering::Relaxed);
        enqueued.saturating_sub(dequeued)
    }

    /// Get total enqueued count
    pub fn enqueued(&self) -> u64 {
        self.enqueued.load(Ordering::Relaxed)
    }

    /// Get total dequeued count
    pub fn dequeued(&self) -> u64 {
        self.dequeued.load(Ordering::Relaxed)
    }

    /// Get total dropped count
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

#[cfg(feature = "queue")]
impl MessageQueue {
    /// Create a new unbounded message queue
    ///
    /// An unbounded queue can grow indefinitely and will never block producers.
    /// Use this when you want maximum throughput and have sufficient memory.
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    ///
    /// let queue = MessageQueue::unbounded();
    /// # }
    /// ```
    pub fn unbounded() -> Self {
        let (sender, receiver) = unbounded();
        Self {
            sender,
            receiver,
            stats: Arc::new(QueueStats::new()),
        }
    }

    /// Create a new bounded message queue with a capacity limit
    ///
    /// A bounded queue has a maximum capacity. When full, `try_send` will fail
    /// and `send` will await until space is available.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of messages the queue can hold
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    ///
    /// let queue = MessageQueue::bounded(1000);
    /// # }
    /// ```
    pub fn bounded(capacity: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        Self {
            sender,
            receiver,
            stats: Arc::new(QueueStats::new()),
        }
    }

    /// Send a message to the queue (async)
    ///
    /// For bounded queues, this will wait if the queue is full.
    /// For unbounded queues, this always succeeds immediately.
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message to enqueue
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    /// use vyna_hl7_core::Message;
    ///
    /// # async fn example(message: Message) {
    /// let queue = MessageQueue::unbounded();
    /// queue.send(message).await.unwrap();
    /// # }
    /// # }
    /// ```
    pub async fn send(&self, message: Message) -> Result<(), async_channel::SendError<Message>> {
        let result = self.sender.send(message).await;
        if result.is_ok() {
            self.stats.enqueued.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// Try to send a message without blocking
    ///
    /// For bounded queues, returns an error if the queue is full.
    /// For unbounded queues, always succeeds.
    ///
    /// # Arguments
    ///
    /// * `message` - The HL7 message to enqueue
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    /// use vyna_hl7_core::Message;
    ///
    /// # fn example(message: Message) {
    /// let queue = MessageQueue::bounded(100);
    /// match queue.try_send(message) {
    ///     Ok(()) => println!("Message queued"),
    ///     Err(e) => eprintln!("Queue full: {}", e),
    /// }
    /// # }
    /// # }
    /// ```
    pub fn try_send(&self, message: Message) -> Result<(), async_channel::TrySendError<Message>> {
        let result = self.sender.try_send(message);
        match &result {
            Ok(()) => {
                self.stats.enqueued.fetch_add(1, Ordering::Relaxed);
            }
            Err(async_channel::TrySendError::Full(_)) => {
                self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(async_channel::TrySendError::Closed(_)) => {}
        }
        result
    }

    /// Receive a message from the queue (async)
    ///
    /// Waits until a message is available.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    ///
    /// # async fn example() {
    /// let queue = MessageQueue::unbounded();
    /// let message = queue.recv().await.unwrap();
    /// # }
    /// # }
    /// ```
    pub async fn recv(&self) -> Result<Message, async_channel::RecvError> {
        let result = self.receiver.recv().await;
        if result.is_ok() {
            self.stats.dequeued.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// Try to receive a message without blocking
    ///
    /// Returns None if the queue is empty.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    ///
    /// # fn example() {
    /// let queue = MessageQueue::unbounded();
    /// match queue.try_recv() {
    ///     Ok(message) => println!("Got message"),
    ///     Err(_) => println!("Queue empty"),
    /// }
    /// # }
    /// # }
    /// ```
    pub fn try_recv(&self) -> Result<Message, async_channel::TryRecvError> {
        let result = self.receiver.try_recv();
        if result.is_ok() {
            self.stats.dequeued.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// Get queue statistics
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    ///
    /// # fn example() {
    /// let queue = MessageQueue::unbounded();
    /// let stats = queue.stats();
    /// println!("Pending: {}", stats.pending());
    /// println!("Enqueued: {}", stats.enqueued());
    /// println!("Dequeued: {}", stats.dequeued());
    /// # }
    /// # }
    /// ```
    pub fn stats(&self) -> QueueStats {
        self.stats.as_ref().clone()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }

    /// Check if a bounded queue is full
    ///
    /// Always returns false for unbounded queues.
    pub fn is_full(&self) -> bool {
        self.receiver.is_full()
    }

    /// Get the number of messages currently in the queue
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Get the capacity of the queue (None for unbounded)
    pub fn capacity(&self) -> Option<usize> {
        self.receiver.capacity()
    }

    /// Create a new sender for this queue
    ///
    /// This allows multiple producers to send to the same queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    ///
    /// # async fn example() {
    /// let queue = MessageQueue::unbounded();
    /// let sender1 = queue.sender();
    /// let sender2 = queue.sender();
    ///
    /// // Both senders can send to the same queue
    /// # }
    /// # }
    /// ```
    pub fn sender(&self) -> MessageSender {
        MessageSender {
            sender: self.sender.clone(),
            stats: self.stats.clone(),
        }
    }

    /// Create a new receiver for this queue
    ///
    /// This allows multiple consumers to receive from the same queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "queue")]
    /// # {
    /// use vyna_hl7_http::queue::MessageQueue;
    ///
    /// # async fn example() {
    /// let queue = MessageQueue::unbounded();
    /// let receiver1 = queue.receiver();
    /// let receiver2 = queue.receiver();
    ///
    /// // Both receivers can receive from the same queue
    /// # }
    /// # }
    /// ```
    pub fn receiver(&self) -> MessageReceiver {
        MessageReceiver {
            receiver: self.receiver.clone(),
            stats: self.stats.clone(),
        }
    }
}

/// Message sender (producer side)
#[cfg(feature = "queue")]
#[derive(Clone)]
pub struct MessageSender {
    sender: Sender<Message>,
    stats: Arc<QueueStats>,
}

#[cfg(feature = "queue")]
impl MessageSender {
    /// Send a message (async)
    pub async fn send(&self, message: Message) -> Result<(), async_channel::SendError<Message>> {
        let result = self.sender.send(message).await;
        if result.is_ok() {
            self.stats.enqueued.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// Try to send a message without blocking
    pub fn try_send(&self, message: Message) -> Result<(), async_channel::TrySendError<Message>> {
        let result = self.sender.try_send(message);
        match &result {
            Ok(()) => {
                self.stats.enqueued.fetch_add(1, Ordering::Relaxed);
            }
            Err(async_channel::TrySendError::Full(_)) => {
                self.stats.dropped.fetch_add(1, Ordering::Relaxed);
            }
            Err(async_channel::TrySendError::Closed(_)) => {}
        }
        result
    }
}

/// Message receiver (consumer side)
#[cfg(feature = "queue")]
#[derive(Clone)]
pub struct MessageReceiver {
    receiver: Receiver<Message>,
    stats: Arc<QueueStats>,
}

#[cfg(feature = "queue")]
impl MessageReceiver {
    /// Receive a message (async)
    pub async fn recv(&self) -> Result<Message, async_channel::RecvError> {
        let result = self.receiver.recv().await;
        if result.is_ok() {
            self.stats.dequeued.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&self) -> Result<Message, async_channel::TryRecvError> {
        let result = self.receiver.try_recv();
        if result.is_ok() {
            self.stats.dequeued.fetch_add(1, Ordering::Relaxed);
        }
        result
    }
}

#[cfg(test)]
#[cfg(feature = "queue")]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unbounded_queue() {
        let queue = MessageQueue::unbounded();
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.capacity(), None);
    }

    #[tokio::test]
    async fn test_bounded_queue() {
        let queue = MessageQueue::bounded(10);
        assert!(queue.is_empty());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.capacity(), Some(10));
    }

    #[test]
    fn test_queue_stats() {
        let stats = QueueStats::new();
        assert_eq!(stats.enqueued(), 0);
        assert_eq!(stats.dequeued(), 0);
        assert_eq!(stats.dropped(), 0);
        assert_eq!(stats.pending(), 0);
    }
}
