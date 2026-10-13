//! Fluent builder for outbound batch files.

use std::sync::atomic::{AtomicU64, Ordering};

use chrono::NaiveDateTime;
use vyna_hl7_core::Message;
use vyna_hl7_core::batch::{Batch, BatchHeader, BatchTrailer, File, FileHeader, FileTrailer};

static CONTROL_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Unique control id: prefix + local timestamp + process id + full-width
/// process-wide sequence (monotonic, never wraps in practice).
fn generate_control_id(prefix: char) -> String {
    let seq = CONTROL_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    let now = chrono::Local::now().naive_local();
    let pid = std::process::id();
    format!("{}{}-{pid}-{seq}", prefix, now.format("%Y%m%d%H%M%S"))
}

/// Wrap messages in `FHS`/`BHS`/`BTS`/`FTS` envelopes.
///
/// Sender/receiver identities are copied to both the file and batch headers,
/// timestamps default to now, control ids are generated when not supplied,
/// and trailer counts always reflect the actual content.
///
/// ```
/// use vyna_hl7_batch::BatchFileBuilder;
///
/// let message = vyna_hl7_parser::parse_message(
///     "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|1|P|2.5",
/// )
/// .unwrap();
/// let file = BatchFileBuilder::new()
///     .sending_application("SENDER")
///     .sending_facility("FACILITY")
///     .message(message)
///     .build_file();
/// assert_eq!(file.total_message_count(), 1);
/// assert!(file.validate().is_ok());
/// ```
#[derive(Debug, Default)]
pub struct BatchFileBuilder {
    sending_application: Option<String>,
    sending_facility: Option<String>,
    receiving_application: Option<String>,
    receiving_facility: Option<String>,
    datetime: Option<NaiveDateTime>,
    file_control_id: Option<String>,
    batch_control_id: Option<String>,
    file_name: Option<String>,
    batch_name: Option<String>,
    messages: Vec<Message>,
    batches: Vec<Batch>,
}

impl BatchFileBuilder {
    /// Start a builder with generated timestamps, control ids, and counts.
    pub fn new() -> Self {
        Self::default()
    }

    /// FHS-3 / BHS-3.
    pub fn sending_application(mut self, value: &str) -> Self {
        self.sending_application = Some(value.to_string());
        self
    }

    /// FHS-4 / BHS-4.
    pub fn sending_facility(mut self, value: &str) -> Self {
        self.sending_facility = Some(value.to_string());
        self
    }

    /// FHS-5 / BHS-5.
    pub fn receiving_application(mut self, value: &str) -> Self {
        self.receiving_application = Some(value.to_string());
        self
    }

    /// FHS-6 / BHS-6.
    pub fn receiving_facility(mut self, value: &str) -> Self {
        self.receiving_facility = Some(value.to_string());
        self
    }

    /// FHS-7 / BHS-7. Defaults to the current local time.
    pub fn datetime(mut self, value: NaiveDateTime) -> Self {
        self.datetime = Some(value);
        self
    }

    /// FHS-11. Defaults to a generated id.
    pub fn file_control_id(mut self, value: &str) -> Self {
        self.file_control_id = Some(value.to_string());
        self
    }

    /// BHS-11 for the batch holding loose messages. Defaults to a generated id.
    pub fn batch_control_id(mut self, value: &str) -> Self {
        self.batch_control_id = Some(value.to_string());
        self
    }

    /// FHS-9 file name/id.
    pub fn file_name(mut self, value: &str) -> Self {
        self.file_name = Some(value.to_string());
        self
    }

    /// BHS-9 batch name/id for the batch holding loose messages.
    pub fn batch_name(mut self, value: &str) -> Self {
        self.batch_name = Some(value.to_string());
        self
    }

    /// Add one message to the batch of loose messages.
    pub fn message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add several messages to the batch of loose messages.
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Add a pre-built batch. Its own header is kept; only a missing trailer
    /// count is filled in.
    pub fn batch(mut self, batch: Batch) -> Self {
        self.batches.push(batch);
        self
    }

    fn datetime_or_now(&self) -> NaiveDateTime {
        self.datetime
            .unwrap_or_else(|| chrono::Local::now().naive_local())
    }

    fn file_header(&self) -> FileHeader {
        FileHeader {
            sending_application: self.sending_application.clone(),
            sending_facility: self.sending_facility.clone(),
            receiving_application: self.receiving_application.clone(),
            receiving_facility: self.receiving_facility.clone(),
            creation_datetime: Some(self.datetime_or_now()),
            file_name_id: self.file_name.clone(),
            control_id: Some(
                self.file_control_id
                    .clone()
                    .unwrap_or_else(|| generate_control_id('F')),
            ),
            ..FileHeader::new()
        }
    }

    fn batch_header(&self) -> BatchHeader {
        BatchHeader {
            sending_application: self.sending_application.clone(),
            sending_facility: self.sending_facility.clone(),
            receiving_application: self.receiving_application.clone(),
            receiving_facility: self.receiving_facility.clone(),
            creation_datetime: Some(self.datetime_or_now()),
            batch_name_id_type: self.batch_name.clone(),
            control_id: Some(
                self.batch_control_id
                    .clone()
                    .unwrap_or_else(|| generate_control_id('B')),
            ),
            ..BatchHeader::new()
        }
    }

    /// Build a single `BHS`...`BTS` batch from the loose messages.
    pub fn build_batch(mut self) -> Batch {
        let mut batch = Batch::with_header(self.batch_header());
        batch.messages.append(&mut self.messages);
        batch.trailer = BatchTrailer::with_count(batch.messages.len());
        batch
    }

    /// Build an `FHS`...`FTS` file. Loose messages form one batch appended
    /// after any pre-built batches.
    pub fn build_file(mut self) -> File {
        let mut file = File::with_header(self.file_header());
        for mut batch in self.batches.drain(..) {
            if batch.trailer.message_count.is_none() {
                batch.trailer.message_count = Some(batch.messages.len());
            }
            file.add_batch(batch);
        }
        if !self.messages.is_empty() {
            let mut batch = Batch::with_header(self.batch_header());
            batch.messages.append(&mut self.messages);
            batch.trailer = BatchTrailer::with_count(batch.messages.len());
            file.add_batch(batch);
        }
        file.trailer = FileTrailer::with_count(file.batches.len());
        file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::Version;

    fn adt(id: &str) -> Message {
        vyna_hl7_core::builders::adt::AdtBuilder::a01(Version::V2_5)
            .patient_id(id)
            .build()
            .unwrap()
    }

    #[test]
    fn build_batch_sets_counts_and_header() {
        let batch = BatchFileBuilder::new()
            .sending_application("SENDER")
            .batch_control_id("B001")
            .message(adt("1"))
            .message(adt("2"))
            .build_batch();

        assert_eq!(batch.message_count(), 2);
        assert_eq!(batch.trailer.message_count, Some(2));
        assert_eq!(batch.header.control_id.as_deref(), Some("B001"));
        assert_eq!(batch.header.sending_application.as_deref(), Some("SENDER"));
        assert!(batch.validate().is_ok());

        let encoded = batch.encode();
        assert!(encoded.starts_with("BHS|"));
        assert!(encoded.contains("\rBTS|2"));
    }

    #[test]
    fn build_file_generates_ids_and_counts() {
        let file = BatchFileBuilder::new()
            .sending_application("S")
            .message(adt("1"))
            .build_file();

        assert_eq!(file.batch_count(), 1);
        assert_eq!(file.total_message_count(), 1);
        assert!(file.validate().is_ok());
        let fhs_id = file.header.control_id.clone().unwrap();
        let bhs_id = file.batches[0].header.control_id.clone().unwrap();
        assert!(fhs_id.starts_with('F'));
        assert!(bhs_id.starts_with('B'));
        assert_ne!(fhs_id, bhs_id);

        let encoded = file.encode();
        assert!(encoded.starts_with("FHS|"));
        assert!(encoded.contains("\rFTS|1"));
    }

    #[test]
    fn build_file_round_trips_through_strict_parser() {
        let file = BatchFileBuilder::new()
            .sending_application("S")
            .sending_facility("F")
            .receiving_application("R")
            .receiving_facility("D")
            .message(adt("1"))
            .message(adt("2"))
            .build_file();

        let reparsed = vyna_hl7_parser::parse_file(&file.encode()).unwrap();
        assert_eq!(reparsed.total_message_count(), 2);
        assert!(reparsed.validate().is_ok());
    }

    #[test]
    fn prebuilt_batches_keep_headers_counts_filled() {
        let mut batch = Batch::new();
        batch.header.control_id = Some("KEEP".to_string());
        batch.add_message(adt("1"));

        let file = BatchFileBuilder::new()
            .batch(batch)
            .message(adt("2"))
            .build_file();

        assert_eq!(file.batch_count(), 2);
        assert_eq!(file.batches[0].header.control_id.as_deref(), Some("KEEP"));
        assert_eq!(file.batches[0].trailer.message_count, Some(1));
        assert_eq!(file.batches[1].trailer.message_count, Some(1));
        assert!(file.validate().is_ok());
    }

    #[test]
    fn control_ids_stay_unique_past_10000_builds() {
        let ids: std::collections::HashSet<String> =
            (0..100_001).map(|_| generate_control_id('F')).collect();
        assert_eq!(ids.len(), 100_001);
    }
}
