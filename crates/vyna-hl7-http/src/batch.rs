//! Batch message processing for HL7 messages
//!
//! This module provides utilities for batching multiple HL7 messages together
//! for efficient processing and transmission. Batching can significantly improve
//! throughput when dealing with high message volumes.

#[cfg(feature = "batch")]
use std::time::{Duration, Instant};
#[cfg(feature = "batch")]
use vyna_hl7_core::Message;

/// Batch of HL7 messages
///
/// Collects multiple messages for batch processing.
#[cfg(feature = "batch")]
#[derive(Debug, Clone)]
pub struct MessageBatch {
    messages: Vec<Message>,
    created_at: Instant,
    max_size: usize,
    max_age: Duration,
}

#[cfg(feature = "batch")]
impl MessageBatch {
    /// Create a new message batch
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum number of messages in the batch
    /// * `max_age` - Maximum age of the batch before it should be flushed
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "batch")]
    /// # {
    /// use vyna_hl7_http::batch::MessageBatch;
    /// use std::time::Duration;
    ///
    /// let batch = MessageBatch::new(100, Duration::from_secs(30));
    /// # }
    /// ```
    pub fn new(max_size: usize, max_age: Duration) -> Self {
        Self {
            messages: Vec::with_capacity(max_size),
            created_at: Instant::now(),
            max_size,
            max_age,
        }
    }

    /// Add a message to the batch
    ///
    /// # Arguments
    ///
    /// * `message` - The message to add
    ///
    /// # Returns
    ///
    /// `true` if the batch should be flushed (max size reached), `false` otherwise
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "batch")]
    /// # {
    /// use vyna_hl7_http::batch::MessageBatch;
    /// use vyna_hl7_core::Message;
    /// use std::time::Duration;
    ///
    /// # fn example(message: Message) {
    /// let mut batch = MessageBatch::new(10, Duration::from_secs(30));
    /// if batch.add(message) {
    ///     // Batch is full, should flush
    /// }
    /// # }
    /// # }
    /// ```
    pub fn add(&mut self, message: Message) -> bool {
        self.messages.push(message);
        self.is_full()
    }

    /// Check if the batch is full
    pub fn is_full(&self) -> bool {
        self.messages.len() >= self.max_size
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get the number of messages in the batch
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the batch has exceeded its maximum age
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.max_age
    }

    /// Check if the batch should be flushed (full or expired)
    pub fn should_flush(&self) -> bool {
        self.is_full() || self.is_expired()
    }

    /// Get the age of the batch
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// Get a reference to the messages in the batch
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Consume the batch and return all messages
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "batch")]
    /// # {
    /// use vyna_hl7_http::batch::MessageBatch;
    /// use std::time::Duration;
    ///
    /// # fn example() {
    /// let batch = MessageBatch::new(10, Duration::from_secs(30));
    /// let messages = batch.into_messages();
    /// # }
    /// # }
    /// ```
    pub fn into_messages(self) -> Vec<Message> {
        self.messages
    }

    /// Clear the batch and reset the creation time
    pub fn clear(&mut self) {
        self.messages.clear();
        self.created_at = Instant::now();
    }
}

/// Batch processor for accumulating and flushing message batches
///
/// Automatically batches messages and invokes a flush callback when the batch
/// is full or expired.
///
/// Pending messages are NOT flushed on drop: call [`flush`](Self::flush)
/// before dropping, or buffered messages are silently discarded.
#[cfg(feature = "batch")]
pub struct BatchProcessor<F>
where
    F: FnMut(Vec<Message>),
{
    batch: MessageBatch,
    flush_callback: F,
}

#[cfg(feature = "batch")]
impl<F> BatchProcessor<F>
where
    F: FnMut(Vec<Message>),
{
    /// Create a new batch processor
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum batch size
    /// * `max_age` - Maximum batch age
    /// * `flush_callback` - Function to call when batch is flushed
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "batch")]
    /// # {
    /// use vyna_hl7_http::batch::BatchProcessor;
    /// use std::time::Duration;
    ///
    /// # fn example() {
    /// let processor = BatchProcessor::new(
    ///     100,
    ///     Duration::from_secs(30),
    ///     |messages| {
    ///         println!("Flushing {} messages", messages.len());
    ///         // Process batch
    ///     }
    /// );
    /// # }
    /// # }
    /// ```
    pub fn new(max_size: usize, max_age: Duration, flush_callback: F) -> Self {
        Self {
            batch: MessageBatch::new(max_size, max_age),
            flush_callback,
        }
    }

    /// Add a message to the batch
    ///
    /// Automatically flushes if the batch becomes full.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to add
    ///
    /// # Example
    ///
    /// ```no_run
    /// # #[cfg(feature = "batch")]
    /// # {
    /// use vyna_hl7_http::batch::BatchProcessor;
    /// use vyna_hl7_core::Message;
    /// use std::time::Duration;
    ///
    /// # fn example(message: Message) {
    /// let mut processor = BatchProcessor::new(
    ///     100,
    ///     Duration::from_secs(30),
    ///     |messages| { /* process */ }
    /// );
    ///
    /// processor.add(message);
    /// # }
    /// # }
    /// ```
    pub fn add(&mut self, message: Message) {
        if self.batch.add(message) {
            self.flush();
        }
    }

    /// Flush the current batch if it should be flushed
    ///
    /// Checks if the batch is full or expired and flushes if necessary.
    pub fn flush_if_needed(&mut self) {
        if self.batch.should_flush() {
            self.flush();
        }
    }

    /// Force flush the current batch
    ///
    /// Flushes the batch regardless of size or age.
    pub fn flush(&mut self) {
        if !self.batch.is_empty() {
            let max_size = self.batch.max_size;
            let max_age = self.batch.max_age;

            let messages = std::mem::replace(&mut self.batch, MessageBatch::new(max_size, max_age))
                .into_messages();

            (self.flush_callback)(messages);
        }
    }

    /// Get the current batch size
    pub fn batch_size(&self) -> usize {
        self.batch.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }
}

/// Configuration for batch processing
#[cfg(feature = "batch")]
#[derive(Debug, Clone, Copy)]
pub struct BatchConfig {
    /// Maximum number of messages per batch
    pub max_size: usize,
    /// Maximum age of a batch before flushing
    pub max_age: Duration,
}

#[cfg(feature = "batch")]
impl BatchConfig {
    /// Create a new batch configuration
    ///
    /// # Arguments
    ///
    /// * `max_size` - Maximum batch size
    /// * `max_age` - Maximum batch age
    ///
    /// # Example
    ///
    /// ```
    /// # #[cfg(feature = "batch")]
    /// # {
    /// use vyna_hl7_http::batch::BatchConfig;
    /// use std::time::Duration;
    ///
    /// let config = BatchConfig::new(100, Duration::from_secs(30));
    /// # }
    /// ```
    pub fn new(max_size: usize, max_age: Duration) -> Self {
        Self { max_size, max_age }
    }

    /// Create a configuration for small batches (10 messages, 5 seconds)
    pub fn small() -> Self {
        Self::new(10, Duration::from_secs(5))
    }

    /// Create a configuration for medium batches (50 messages, 15 seconds)
    pub fn medium() -> Self {
        Self::new(50, Duration::from_secs(15))
    }

    /// Create a configuration for large batches (200 messages, 60 seconds)
    pub fn large() -> Self {
        Self::new(200, Duration::from_secs(60))
    }
}

#[cfg(feature = "batch")]
impl Default for BatchConfig {
    fn default() -> Self {
        Self::medium()
    }
}

#[cfg(test)]
#[cfg(feature = "batch")]
mod tests {
    use super::*;

    #[test]
    fn test_batch_creation() {
        let batch = MessageBatch::new(10, Duration::from_secs(30));
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
        assert!(!batch.is_full());
        assert!(!batch.is_expired());
    }

    #[test]
    fn test_drop_without_flush_discards_pending() {
        // Contract: drop never flushes; the caller must flush first.
        use std::cell::Cell;
        use std::rc::Rc;

        let calls = Rc::new(Cell::new(0usize));
        {
            let mut processor = BatchProcessor::new(100, Duration::from_secs(30), {
                let calls = calls.clone();
                move |_messages| {
                    calls.set(calls.get() + 1);
                }
            });
            processor.add(Message::default());
            assert_eq!(processor.batch_size(), 1);
        }
        assert_eq!(calls.get(), 0);
    }

    #[test]
    fn test_batch_config() {
        let config = BatchConfig::default();
        assert_eq!(config.max_size, 50);
        assert_eq!(config.max_age, Duration::from_secs(15));

        let small = BatchConfig::small();
        assert_eq!(small.max_size, 10);

        let large = BatchConfig::large();
        assert_eq!(large.max_size, 200);
    }
}
