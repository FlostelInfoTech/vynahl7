//! Batch and file message structures for HL7 v2.x
//!
//! This module provides support for batch and file messages, which allow multiple
//! HL7 messages to be grouped together for high-volume processing.
//!
//! ## Structures
//!
//! - **Batch**: A collection of messages enclosed in BHS (Batch Header) and BTS (Batch Trailer)
//! - **File**: A collection of batches enclosed in FHS (File Header) and FTS (File Trailer)
//!
//! ## Example Batch Structure
//!
//! ```text
//! BHS|^~\&|SENDER|FACILITY|RECEIVER|DEST|20251120120000|||B001
//! MSH|^~\&|...  (Message 1)
//! PID|...
//! MSH|^~\&|...  (Message 2)
//! PID|...
//! BTS|2
//! ```
//!
//! ## Example File Structure
//!
//! ```text
//! FHS|^~\&|SENDER|FACILITY|RECEIVER|DEST|20251120120000|||F001
//! BHS|^~\&|SENDER|FACILITY|RECEIVER|DEST|20251120120000|||B001
//! MSH|^~\&|... (Messages)
//! BTS|2
//! BHS|^~\&|SENDER|FACILITY|RECEIVER|DEST|20251120120100|||B002
//! MSH|^~\&|... (Messages)
//! BTS|3
//! FTS|2
//! ```

use crate::{error::{Error, Result}, message::Message, segment::Segment, field::Field};
use chrono::NaiveDateTime;

/// File Header Segment (FHS) - marks the beginning of a file containing batches
///
/// A file can contain one or more batches. The FHS segment provides file-level
/// metadata including sender/receiver information, creation timestamp, and control IDs.
#[derive(Debug, Clone)]
pub struct FileHeader {
    /// FHS-1: Field separator (typically '|')
    pub field_separator: char,
    /// FHS-2: Encoding characters (typically "^~\\&")
    pub encoding_characters: String,
    /// FHS-3: File sending application
    pub sending_application: Option<String>,
    /// FHS-4: File sending facility
    pub sending_facility: Option<String>,
    /// FHS-5: File receiving application
    pub receiving_application: Option<String>,
    /// FHS-6: File receiving facility
    pub receiving_facility: Option<String>,
    /// FHS-7: File creation date/time
    pub creation_datetime: Option<NaiveDateTime>,
    /// FHS-8: File security
    pub security: Option<String>,
    /// FHS-9: File name/ID
    pub file_name_id: Option<String>,
    /// FHS-10: File header comment
    pub comment: Option<String>,
    /// FHS-11: File control ID (unique identifier for this file)
    pub control_id: Option<String>,
    /// FHS-12: Reference file control ID (original control ID when retransmitting)
    pub reference_control_id: Option<String>,
    /// FHS-13: File sending network address (v2.6+)
    pub sending_network_address: Option<String>,
    /// FHS-14: File receiving network address (v2.6+)
    pub receiving_network_address: Option<String>,
}

impl FileHeader {
    /// Create a new FileHeader with default field separator and encoding characters
    pub fn new() -> Self {
        Self {
            field_separator: '|',
            encoding_characters: "^~\\&".to_string(),
            sending_application: None,
            sending_facility: None,
            receiving_application: None,
            receiving_facility: None,
            creation_datetime: None,
            security: None,
            file_name_id: None,
            comment: None,
            control_id: None,
            reference_control_id: None,
            sending_network_address: None,
            receiving_network_address: None,
        }
    }

    /// Convert FileHeader to an FHS Segment
    pub fn to_segment(&self) -> Segment {
        let mut fhs = Segment::new("FHS");

        // FHS-1: Field separator
        fhs.add_field(Field::from_value(self.field_separator.to_string()));

        // FHS-2: Encoding characters
        fhs.add_field(Field::from_value(&self.encoding_characters));

        // FHS-3: Sending application
        fhs.add_field(Field::from_value(self.sending_application.as_deref().unwrap_or("")));

        // FHS-4: Sending facility
        fhs.add_field(Field::from_value(self.sending_facility.as_deref().unwrap_or("")));

        // FHS-5: Receiving application
        fhs.add_field(Field::from_value(self.receiving_application.as_deref().unwrap_or("")));

        // FHS-6: Receiving facility
        fhs.add_field(Field::from_value(self.receiving_facility.as_deref().unwrap_or("")));

        // FHS-7: Creation date/time
        let datetime_str = self.creation_datetime
            .map(|dt| crate::types::format_timestamp(&dt))
            .unwrap_or_default();
        fhs.add_field(Field::from_value(&datetime_str));

        // FHS-8: Security
        fhs.add_field(Field::from_value(self.security.as_deref().unwrap_or("")));

        // FHS-9: File name/ID
        fhs.add_field(Field::from_value(self.file_name_id.as_deref().unwrap_or("")));

        // FHS-10: Comment
        fhs.add_field(Field::from_value(self.comment.as_deref().unwrap_or("")));

        // FHS-11: Control ID
        fhs.add_field(Field::from_value(self.control_id.as_deref().unwrap_or("")));

        // FHS-12: Reference control ID
        fhs.add_field(Field::from_value(self.reference_control_id.as_deref().unwrap_or("")));

        // FHS-13: Sending network address (v2.6+)
        if let Some(ref addr) = self.sending_network_address {
            fhs.add_field(Field::from_value(addr));
        }

        // FHS-14: Receiving network address (v2.6+)
        if let Some(ref addr) = self.receiving_network_address {
            fhs.add_field(Field::from_value(addr));
        }

        fhs
    }
}

impl Default for FileHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch Header Segment (BHS) - marks the beginning of a batch within a file
///
/// A batch contains one or more HL7 messages. The BHS segment provides batch-level
/// metadata similar to FHS but at the batch level.
#[derive(Debug, Clone)]
pub struct BatchHeader {
    /// BHS-1: Field separator (typically '|')
    pub field_separator: char,
    /// BHS-2: Encoding characters (typically "^~\\&")
    pub encoding_characters: String,
    /// BHS-3: Batch sending application
    pub sending_application: Option<String>,
    /// BHS-4: Batch sending facility
    pub sending_facility: Option<String>,
    /// BHS-5: Batch receiving application
    pub receiving_application: Option<String>,
    /// BHS-6: Batch receiving facility
    pub receiving_facility: Option<String>,
    /// BHS-7: Batch creation date/time
    pub creation_datetime: Option<NaiveDateTime>,
    /// BHS-8: Batch security
    pub security: Option<String>,
    /// BHS-9: Batch name/ID/type
    pub batch_name_id_type: Option<String>,
    /// BHS-10: Batch comment
    pub comment: Option<String>,
    /// BHS-11: Batch control ID (unique identifier for this batch)
    pub control_id: Option<String>,
    /// BHS-12: Reference batch control ID (original control ID when retransmitting)
    pub reference_control_id: Option<String>,
    /// BHS-13: Batch sending network address (v2.6+)
    pub sending_network_address: Option<String>,
    /// BHS-14: Batch receiving network address (v2.6+)
    pub receiving_network_address: Option<String>,
}

impl BatchHeader {
    /// Create a new BatchHeader with default field separator and encoding characters
    pub fn new() -> Self {
        Self {
            field_separator: '|',
            encoding_characters: "^~\\&".to_string(),
            sending_application: None,
            sending_facility: None,
            receiving_application: None,
            receiving_facility: None,
            creation_datetime: None,
            security: None,
            batch_name_id_type: None,
            comment: None,
            control_id: None,
            reference_control_id: None,
            sending_network_address: None,
            receiving_network_address: None,
        }
    }

    /// Convert BatchHeader to a BHS Segment
    pub fn to_segment(&self) -> Segment {
        let mut bhs = Segment::new("BHS");

        // BHS-1: Field separator
        bhs.add_field(Field::from_value(self.field_separator.to_string()));

        // BHS-2: Encoding characters
        bhs.add_field(Field::from_value(&self.encoding_characters));

        // BHS-3: Sending application
        bhs.add_field(Field::from_value(self.sending_application.as_deref().unwrap_or("")));

        // BHS-4: Sending facility
        bhs.add_field(Field::from_value(self.sending_facility.as_deref().unwrap_or("")));

        // BHS-5: Receiving application
        bhs.add_field(Field::from_value(self.receiving_application.as_deref().unwrap_or("")));

        // BHS-6: Receiving facility
        bhs.add_field(Field::from_value(self.receiving_facility.as_deref().unwrap_or("")));

        // BHS-7: Creation date/time
        let datetime_str = self.creation_datetime
            .map(|dt| crate::types::format_timestamp(&dt))
            .unwrap_or_default();
        bhs.add_field(Field::from_value(&datetime_str));

        // BHS-8: Security
        bhs.add_field(Field::from_value(self.security.as_deref().unwrap_or("")));

        // BHS-9: Batch name/ID/type
        bhs.add_field(Field::from_value(self.batch_name_id_type.as_deref().unwrap_or("")));

        // BHS-10: Comment
        bhs.add_field(Field::from_value(self.comment.as_deref().unwrap_or("")));

        // BHS-11: Control ID
        bhs.add_field(Field::from_value(self.control_id.as_deref().unwrap_or("")));

        // BHS-12: Reference control ID
        bhs.add_field(Field::from_value(self.reference_control_id.as_deref().unwrap_or("")));

        // BHS-13: Sending network address (v2.6+)
        if let Some(ref addr) = self.sending_network_address {
            bhs.add_field(Field::from_value(addr));
        }

        // BHS-14: Receiving network address (v2.6+)
        if let Some(ref addr) = self.receiving_network_address {
            bhs.add_field(Field::from_value(addr));
        }

        bhs
    }
}

impl Default for BatchHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// File Trailer Segment (FTS) - marks the end of a file and provides file summary
///
/// The FTS segment indicates the number of batches contained in the file and
/// optionally includes a comment.
#[derive(Debug, Clone)]
pub struct FileTrailer {
    /// FTS-1: File batch count (number of batches in the file)
    pub batch_count: Option<usize>,
    /// FTS-2: File trailer comment
    pub comment: Option<String>,
}

impl FileTrailer {
    /// Create a new FileTrailer
    pub fn new() -> Self {
        Self {
            batch_count: None,
            comment: None,
        }
    }

    /// Create a FileTrailer with a specified batch count
    pub fn with_count(batch_count: usize) -> Self {
        Self {
            batch_count: Some(batch_count),
            comment: None,
        }
    }

    /// Convert FileTrailer to an FTS Segment
    pub fn to_segment(&self) -> Segment {
        let mut fts = Segment::new("FTS");

        // FTS-1: Batch count
        let count_str = self.batch_count
            .map(|c| c.to_string())
            .unwrap_or_default();
        fts.add_field(Field::from_value(&count_str));

        // FTS-2: Comment
        fts.add_field(Field::from_value(self.comment.as_deref().unwrap_or("")));

        fts
    }
}

impl Default for FileTrailer {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch Trailer Segment (BTS) - marks the end of a batch and provides batch summary
///
/// The BTS segment indicates the number of messages contained in the batch and
/// optionally includes comments and batch totals.
#[derive(Debug, Clone)]
pub struct BatchTrailer {
    /// BTS-1: Batch message count (number of messages in the batch)
    pub message_count: Option<usize>,
    /// BTS-2: Batch comment
    pub comment: Option<String>,
    /// BTS-3: Batch totals (repeating field for application-specific totals)
    pub totals: Vec<f64>,
}

impl BatchTrailer {
    /// Create a new BatchTrailer
    pub fn new() -> Self {
        Self {
            message_count: None,
            comment: None,
            totals: Vec::new(),
        }
    }

    /// Create a BatchTrailer with a specified message count
    pub fn with_count(message_count: usize) -> Self {
        Self {
            message_count: Some(message_count),
            comment: None,
            totals: Vec::new(),
        }
    }

    /// Convert BatchTrailer to a BTS Segment
    pub fn to_segment(&self) -> Segment {
        let mut bts = Segment::new("BTS");

        // BTS-1: Message count
        let count_str = self.message_count
            .map(|c| c.to_string())
            .unwrap_or_default();
        bts.add_field(Field::from_value(&count_str));

        // BTS-2: Comment
        bts.add_field(Field::from_value(self.comment.as_deref().unwrap_or("")));

        // BTS-3: Batch totals (repeating)
        if !self.totals.is_empty() {
            let totals_str = self.totals.iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>()
                .join("~");
            bts.add_field(Field::from_value(&totals_str));
        }

        bts
    }
}

impl Default for BatchTrailer {
    fn default() -> Self {
        Self::new()
    }
}

/// A batch of HL7 messages enclosed in BHS/BTS headers and trailers
///
/// Batches allow multiple messages to be grouped together for efficient processing.
/// Each message in the batch is a complete HL7 message starting with MSH.
#[derive(Debug, Clone)]
pub struct Batch {
    /// Batch header segment (BHS)
    pub header: BatchHeader,
    /// Messages contained in this batch
    pub messages: Vec<Message>,
    /// Batch trailer segment (BTS)
    pub trailer: BatchTrailer,
}

impl Batch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self {
            header: BatchHeader::new(),
            messages: Vec::new(),
            trailer: BatchTrailer::new(),
        }
    }

    /// Create a batch with a header
    pub fn with_header(header: BatchHeader) -> Self {
        Self {
            header,
            messages: Vec::new(),
            trailer: BatchTrailer::new(),
        }
    }

    /// Add a message to this batch
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Get the actual message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Validate the batch structure and message count
    ///
    /// Returns an error if:
    /// - The trailer's message count doesn't match the actual number of messages
    pub fn validate(&self) -> Result<()> {
        if let Some(expected_count) = self.trailer.message_count {
            let actual_count = self.messages.len();
            if expected_count != actual_count {
                return Err(Error::validation(format!(
                    "Batch message count mismatch: BTS-1 indicates {} messages but batch contains {} messages",
                    expected_count, actual_count
                )));
            }
        }
        Ok(())
    }

    /// Encode the batch to HL7 format with the specified separator
    pub fn encode_with_separator(&self, separator: &str) -> String {
        let mut result = Vec::new();
        let delimiters = crate::Delimiters::default();

        // Add BHS
        result.push(self.header.to_segment().encode(&delimiters));

        // Add all messages
        for message in &self.messages {
            result.push(message.encode());
        }

        // Add BTS
        result.push(self.trailer.to_segment().encode(&delimiters));

        result.join(separator)
    }

    /// Encode the batch to standard HL7 format (carriage return separator)
    pub fn encode(&self) -> String {
        self.encode_with_separator("\r")
    }
}

impl Default for Batch {
    fn default() -> Self {
        Self::new()
    }
}

/// A file containing one or more batches, enclosed in FHS/FTS headers and trailers
///
/// Files provide an additional level of grouping beyond batches, allowing multiple
/// batches to be transmitted together.
#[derive(Debug, Clone)]
pub struct File {
    /// File header segment (FHS)
    pub header: FileHeader,
    /// Batches contained in this file
    pub batches: Vec<Batch>,
    /// File trailer segment (FTS)
    pub trailer: FileTrailer,
}

impl File {
    /// Create a new empty file
    pub fn new() -> Self {
        Self {
            header: FileHeader::new(),
            batches: Vec::new(),
            trailer: FileTrailer::new(),
        }
    }

    /// Create a file with a header
    pub fn with_header(header: FileHeader) -> Self {
        Self {
            header,
            batches: Vec::new(),
            trailer: FileTrailer::new(),
        }
    }

    /// Add a batch to this file
    pub fn add_batch(&mut self, batch: Batch) {
        self.batches.push(batch);
    }

    /// Get the actual batch count
    pub fn batch_count(&self) -> usize {
        self.batches.len()
    }

    /// Get the total message count across all batches
    pub fn total_message_count(&self) -> usize {
        self.batches.iter().map(|b| b.message_count()).sum()
    }

    /// Validate the file structure and counts
    ///
    /// Returns an error if:
    /// - The trailer's batch count doesn't match the actual number of batches
    /// - Any batch fails validation
    pub fn validate(&self) -> Result<()> {
        // Validate file-level counts
        if let Some(expected_count) = self.trailer.batch_count {
            let actual_count = self.batches.len();
            if expected_count != actual_count {
                return Err(Error::validation(format!(
                    "File batch count mismatch: FTS-1 indicates {} batches but file contains {} batches",
                    expected_count, actual_count
                )));
            }
        }

        // Validate each batch
        for (idx, batch) in self.batches.iter().enumerate() {
            batch.validate().map_err(|e| {
                Error::validation(format!("Batch {} validation failed: {}", idx + 1, e))
            })?;
        }

        Ok(())
    }

    /// Encode the file to HL7 format with the specified separator
    pub fn encode_with_separator(&self, separator: &str) -> String {
        let mut result = Vec::new();
        let delimiters = crate::Delimiters::default();

        // Add FHS
        result.push(self.header.to_segment().encode(&delimiters));

        // Add all batches
        for batch in &self.batches {
            result.push(batch.encode_with_separator(separator));
        }

        // Add FTS
        result.push(self.trailer.to_segment().encode(&delimiters));

        result.join(separator)
    }

    /// Encode the file to standard HL7 format (carriage return separator)
    pub fn encode(&self) -> String {
        self.encode_with_separator("\r")
    }
}

impl Default for File {
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
    fn test_batch_header_to_segment() {
        let mut header = BatchHeader::new();
        header.sending_application = Some("SENDER".to_string());
        header.sending_facility = Some("FAC".to_string());
        header.control_id = Some("B001".to_string());

        let segment = header.to_segment();
        assert_eq!(segment.id, "BHS");

        let sending_app = segment.get_field(3).unwrap().value().unwrap();
        assert_eq!(sending_app, "SENDER");
    }

    #[test]
    fn test_batch_message_count_validation() {
        let mut batch = Batch::new();
        batch.trailer.message_count = Some(2);

        // Add only 1 message
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();
        batch.add_message(msg);

        // Should fail validation
        assert!(batch.validate().is_err());

        // Add second message
        let msg2 = AdtBuilder::a01(Version::V2_5)
            .patient_id("67890")
            .build()
            .unwrap();
        batch.add_message(msg2);

        // Should pass validation
        assert!(batch.validate().is_ok());
    }

    #[test]
    fn test_file_batch_count_validation() {
        let mut file = File::new();
        file.trailer.batch_count = Some(2);

        // Add only 1 batch
        let batch = Batch::new();
        file.add_batch(batch);

        // Should fail validation
        assert!(file.validate().is_err());

        // Add second batch
        let batch2 = Batch::new();
        file.add_batch(batch2);

        // Should pass validation
        assert!(file.validate().is_ok());
    }

    #[test]
    fn test_batch_encoding() {
        let mut batch = Batch::new();
        batch.header.sending_application = Some("APP".to_string());
        batch.header.control_id = Some("B001".to_string());

        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .patient_name("DOE", "JOHN")
            .build()
            .unwrap();
        batch.add_message(msg);

        batch.trailer.message_count = Some(1);

        let encoded = batch.encode_with_separator("\n");
        assert!(encoded.contains("BHS|"));
        assert!(encoded.contains("MSH|"));
        assert!(encoded.contains("BTS|1"));
    }

    #[test]
    fn test_file_encoding() {
        let mut file = File::new();
        file.header.sending_application = Some("APP".to_string());
        file.header.control_id = Some("F001".to_string());

        let mut batch = Batch::new();
        batch.header.control_id = Some("B001".to_string());
        let msg = AdtBuilder::a01(Version::V2_5)
            .patient_id("12345")
            .build()
            .unwrap();
        batch.add_message(msg);
        batch.trailer.message_count = Some(1);

        file.add_batch(batch);
        file.trailer.batch_count = Some(1);

        let encoded = file.encode_with_separator("\n");
        assert!(encoded.contains("FHS|"));
        assert!(encoded.contains("BHS|"));
        assert!(encoded.contains("MSH|"));
        assert!(encoded.contains("BTS|1"));
        assert!(encoded.contains("FTS|1"));
    }

    #[test]
    fn test_file_total_message_count() {
        let mut file = File::new();

        // Batch 1 with 2 messages
        let mut batch1 = Batch::new();
        batch1.add_message(AdtBuilder::a01(Version::V2_5).patient_id("1").build().unwrap());
        batch1.add_message(AdtBuilder::a01(Version::V2_5).patient_id("2").build().unwrap());
        file.add_batch(batch1);

        // Batch 2 with 3 messages
        let mut batch2 = Batch::new();
        batch2.add_message(AdtBuilder::a01(Version::V2_5).patient_id("3").build().unwrap());
        batch2.add_message(AdtBuilder::a01(Version::V2_5).patient_id("4").build().unwrap());
        batch2.add_message(AdtBuilder::a01(Version::V2_5).patient_id("5").build().unwrap());
        file.add_batch(batch2);

        assert_eq!(file.batch_count(), 2);
        assert_eq!(file.total_message_count(), 5);
    }
}
