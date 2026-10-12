//! Builders for creating HL7 batch and file messages
//!
//! This module provides fluent builder APIs for creating batch and file messages,
//! which allow high-volume message processing.
//!
//! # Examples
//!
//! ## Creating a Batch
//!
//! ```rust
//! use vyna_hl7_core::builders::batch::BatchBuilder;
//! use vyna_hl7_core::builders::adt::AdtBuilder;
//! use vyna_hl7_core::Version;
//!
//! // Create a message
//! let msg1 = AdtBuilder::a01(Version::V2_5)
//!     .patient_id("12345")
//!     .build()
//!     .unwrap();
//!
//! // Build a batch containing the message
//! let batch = BatchBuilder::new()
//!     .sending_application("LAB")
//!     .sending_facility("HOSPITAL")
//!     .receiving_application("EMR")
//!     .receiving_facility("CLINIC")
//!     .control_id("B001")
//!     .add_message(msg1)
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Creating a File
//!
//! ```rust
//! use vyna_hl7_core::builders::batch::FileBuilder;
//!
//! let mut builder = FileBuilder::new()
//!     .sending_application("LAB")
//!     .control_id("F001");
//!
//! // Add batches
//! let file = builder.build().unwrap();
//! ```

use crate::{
    batch::{Batch, BatchHeader, BatchTrailer, File, FileHeader, FileTrailer},
    error::Result,
    message::Message,
};
use chrono::{Local, NaiveDateTime};

/// Builder for creating batch messages with a fluent API
///
/// A batch contains multiple HL7 messages enclosed in BHS/BTS headers and trailers.
///
/// # Example
///
/// ```rust
/// use vyna_hl7_core::builders::batch::BatchBuilder;
/// use vyna_hl7_core::builders::adt::AdtBuilder;
/// use vyna_hl7_core::Version;
///
/// let msg = AdtBuilder::a01(Version::V2_5)
///     .patient_id("12345")
///     .build()
///     .unwrap();
///
/// let batch = BatchBuilder::new()
///     .sending_application("APP")
///     .sending_facility("FAC")
///     .control_id("B001")
///     .add_message(msg)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct BatchBuilder {
    header: BatchHeader,
    messages: Vec<Message>,
    trailer: BatchTrailer,
}

impl BatchBuilder {
    /// Create a new BatchBuilder with default values
    pub fn new() -> Self {
        Self {
            header: BatchHeader::new(),
            messages: Vec::new(),
            trailer: BatchTrailer::new(),
        }
    }

    /// Set the sending application (BHS-3)
    pub fn sending_application(mut self, app: &str) -> Self {
        self.header.sending_application = Some(app.to_string());
        self
    }

    /// Set the sending facility (BHS-4)
    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.header.sending_facility = Some(facility.to_string());
        self
    }

    /// Set the receiving application (BHS-5)
    pub fn receiving_application(mut self, app: &str) -> Self {
        self.header.receiving_application = Some(app.to_string());
        self
    }

    /// Set the receiving facility (BHS-6)
    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.header.receiving_facility = Some(facility.to_string());
        self
    }

    /// Set the creation date/time (BHS-7)
    ///
    /// If not set, the current time will be used when building
    pub fn creation_datetime(mut self, datetime: NaiveDateTime) -> Self {
        self.header.creation_datetime = Some(datetime);
        self
    }

    /// Set the batch control ID (BHS-11)
    pub fn control_id(mut self, id: &str) -> Self {
        self.header.control_id = Some(id.to_string());
        self
    }

    /// Set the batch name/ID/type (BHS-9)
    pub fn batch_name(mut self, name: &str) -> Self {
        self.header.batch_name_id_type = Some(name.to_string());
        self
    }

    /// Set the batch comment (BHS-10)
    pub fn comment(mut self, comment: &str) -> Self {
        self.header.comment = Some(comment.to_string());
        self
    }

    /// Set the reference batch control ID (BHS-12)
    pub fn reference_control_id(mut self, id: &str) -> Self {
        self.header.reference_control_id = Some(id.to_string());
        self
    }

    /// Set the sending network address (BHS-13, v2.6+)
    pub fn sending_network_address(mut self, address: &str) -> Self {
        self.header.sending_network_address = Some(address.to_string());
        self
    }

    /// Set the receiving network address (BHS-14, v2.6+)
    pub fn receiving_network_address(mut self, address: &str) -> Self {
        self.header.receiving_network_address = Some(address.to_string());
        self
    }

    /// Add a message to the batch
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add multiple messages to the batch
    pub fn add_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Set the trailer comment (BTS-2)
    pub fn trailer_comment(mut self, comment: &str) -> Self {
        self.trailer.comment = Some(comment.to_string());
        self
    }

    /// Add a batch total (BTS-3)
    pub fn add_total(mut self, total: f64) -> Self {
        self.trailer.totals.push(total);
        self
    }

    /// Build the batch
    ///
    /// This will:
    /// - Set the creation datetime to now if not already set
    /// - Set the message count in the trailer to match the actual number of messages
    /// - Validate the batch structure
    pub fn build(mut self) -> Result<Batch> {
        // Set creation datetime if not set
        if self.header.creation_datetime.is_none() {
            self.header.creation_datetime = Some(Local::now().naive_local());
        }

        // Set message count in trailer
        self.trailer.message_count = Some(self.messages.len());

        let batch = Batch {
            header: self.header,
            messages: self.messages,
            trailer: self.trailer,
        };

        // Validate before returning
        batch.validate()?;

        Ok(batch)
    }
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating file messages with a fluent API
///
/// A file contains one or more batches enclosed in FHS/FTS headers and trailers.
///
/// # Example
///
/// ```rust
/// use vyna_hl7_core::builders::batch::{FileBuilder, BatchBuilder};
/// use vyna_hl7_core::builders::adt::AdtBuilder;
/// use vyna_hl7_core::Version;
///
/// let msg = AdtBuilder::a01(Version::V2_5)
///     .patient_id("12345")
///     .build()
///     .unwrap();
///
/// let batch = BatchBuilder::new()
///     .add_message(msg)
///     .build()
///     .unwrap();
///
/// let file = FileBuilder::new()
///     .sending_application("APP")
///     .control_id("F001")
///     .add_batch(batch)
///     .build()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct FileBuilder {
    header: FileHeader,
    batches: Vec<Batch>,
    trailer: FileTrailer,
}

impl FileBuilder {
    /// Create a new FileBuilder with default values
    pub fn new() -> Self {
        Self {
            header: FileHeader::new(),
            batches: Vec::new(),
            trailer: FileTrailer::new(),
        }
    }

    /// Set the sending application (FHS-3)
    pub fn sending_application(mut self, app: &str) -> Self {
        self.header.sending_application = Some(app.to_string());
        self
    }

    /// Set the sending facility (FHS-4)
    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.header.sending_facility = Some(facility.to_string());
        self
    }

    /// Set the receiving application (FHS-5)
    pub fn receiving_application(mut self, app: &str) -> Self {
        self.header.receiving_application = Some(app.to_string());
        self
    }

    /// Set the receiving facility (FHS-6)
    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.header.receiving_facility = Some(facility.to_string());
        self
    }

    /// Set the creation date/time (FHS-7)
    ///
    /// If not set, the current time will be used when building
    pub fn creation_datetime(mut self, datetime: NaiveDateTime) -> Self {
        self.header.creation_datetime = Some(datetime);
        self
    }

    /// Set the file control ID (FHS-11)
    pub fn control_id(mut self, id: &str) -> Self {
        self.header.control_id = Some(id.to_string());
        self
    }

    /// Set the file name/ID (FHS-9)
    pub fn file_name(mut self, name: &str) -> Self {
        self.header.file_name_id = Some(name.to_string());
        self
    }

    /// Set the file header comment (FHS-10)
    pub fn comment(mut self, comment: &str) -> Self {
        self.header.comment = Some(comment.to_string());
        self
    }

    /// Set the reference file control ID (FHS-12)
    pub fn reference_control_id(mut self, id: &str) -> Self {
        self.header.reference_control_id = Some(id.to_string());
        self
    }

    /// Set the sending network address (FHS-13, v2.6+)
    pub fn sending_network_address(mut self, address: &str) -> Self {
        self.header.sending_network_address = Some(address.to_string());
        self
    }

    /// Set the receiving network address (FHS-14, v2.6+)
    pub fn receiving_network_address(mut self, address: &str) -> Self {
        self.header.receiving_network_address = Some(address.to_string());
        self
    }

    /// Add a batch to the file
    pub fn add_batch(mut self, batch: Batch) -> Self {
        self.batches.push(batch);
        self
    }

    /// Add multiple batches to the file
    pub fn add_batches(mut self, batches: Vec<Batch>) -> Self {
        self.batches.extend(batches);
        self
    }

    /// Set the trailer comment (FTS-2)
    pub fn trailer_comment(mut self, comment: &str) -> Self {
        self.trailer.comment = Some(comment.to_string());
        self
    }

    /// Build the file
    ///
    /// This will:
    /// - Set the creation datetime to now if not already set
    /// - Set the batch count in the trailer to match the actual number of batches
    /// - Validate the file structure (including all batches)
    pub fn build(mut self) -> Result<File> {
        // Set creation datetime if not set
        if self.header.creation_datetime.is_none() {
            self.header.creation_datetime = Some(Local::now().naive_local());
        }

        // Set batch count in trailer
        self.trailer.batch_count = Some(self.batches.len());

        let file = File {
            header: self.header,
            batches: self.batches,
            trailer: self.trailer,
        };

        // Validate before returning
        file.validate()?;

        Ok(file)
    }
}

impl Default for FileBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::adt::AdtBuilder;
    use crate::Version;

    #[test]
    fn test_batch_builder() {
        let msg1 = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();

        let msg2 = AdtBuilder::a01(Version::V2_5)
            .patient_id("67890")
            .build()
            .unwrap();

        let batch = BatchBuilder::new()
            .sending_application("LAB")
            .sending_facility("HOSPITAL")
            .receiving_application("EMR")
            .receiving_facility("CLINIC")
            .control_id("B001")
            .add_message(msg1)
            .add_message(msg2)
            .build()
            .unwrap();

        assert_eq!(batch.header.sending_application, Some("LAB".to_string()));
        assert_eq!(batch.header.control_id, Some("B001".to_string()));
        assert_eq!(batch.messages.len(), 2);
        assert_eq!(batch.trailer.message_count, Some(2));

        // Should validate successfully
        assert!(batch.validate().is_ok());
    }

    #[test]
    fn test_batch_builder_auto_message_count() {
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();

        let batch = BatchBuilder::new()
            .add_message(msg)
            .build()
            .unwrap();

        // Trailer should automatically have message_count set to 1
        assert_eq!(batch.trailer.message_count, Some(1));
    }

    #[test]
    fn test_file_builder() {
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();

        let batch1 = BatchBuilder::new()
            .control_id("B001")
            .add_message(msg.clone())
            .build()
            .unwrap();

        let batch2 = BatchBuilder::new()
            .control_id("B002")
            .add_message(msg)
            .build()
            .unwrap();

        let file = FileBuilder::new()
            .sending_application("LAB")
            .sending_facility("HOSPITAL")
            .control_id("F001")
            .add_batch(batch1)
            .add_batch(batch2)
            .build()
            .unwrap();

        assert_eq!(file.header.sending_application, Some("LAB".to_string()));
        assert_eq!(file.header.control_id, Some("F001".to_string()));
        assert_eq!(file.batches.len(), 2);
        assert_eq!(file.trailer.batch_count, Some(2));

        // Should validate successfully
        assert!(file.validate().is_ok());
    }

    #[test]
    fn test_file_builder_auto_batch_count() {
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();

        let batch = BatchBuilder::new()
            .add_message(msg)
            .build()
            .unwrap();

        let file = FileBuilder::new()
            .add_batch(batch)
            .build()
            .unwrap();

        // Trailer should automatically have batch_count set to 1
        assert_eq!(file.trailer.batch_count, Some(1));
    }

    #[test]
    fn test_batch_encoding() {
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();

        let batch = BatchBuilder::new()
            .sending_application("APP")
            .control_id("B001")
            .add_message(msg)
            .build()
            .unwrap();

        let encoded = batch.encode_with_separator("\n");
        assert!(encoded.contains("BHS|"));
        assert!(encoded.contains("MSH|"));
        assert!(encoded.contains("BTS|1"));
    }

    #[test]
    fn test_file_total_message_count() {
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();

        let batch1 = BatchBuilder::new()
            .add_message(msg.clone())
            .add_message(msg.clone())
            .build()
            .unwrap();

        let batch2 = BatchBuilder::new()
            .add_message(msg.clone())
            .add_message(msg.clone())
            .add_message(msg)
            .build()
            .unwrap();

        let file = FileBuilder::new()
            .add_batch(batch1)
            .add_batch(batch2)
            .build()
            .unwrap();

        assert_eq!(file.total_message_count(), 5);
    }
}
