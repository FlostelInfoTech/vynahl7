//! HL7 message structures

use crate::charset::CharacterSet;
use crate::delimiters::Delimiters;
use crate::error::{Error, Result};
use crate::segment::Segment;
use crate::Version;

/// An HL7 message
///
/// A message consists of multiple segments, starting with an MSH segment.
/// The message structure follows HL7 v2.x specifications.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// Message segments
    pub segments: Vec<Segment>,
    /// Delimiters used in this message
    pub delimiters: Delimiters,
}

impl Message {
    /// Create a new empty message with default delimiters
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            delimiters: Delimiters::default(),
        }
    }

    /// Create a new message with custom delimiters
    pub fn with_delimiters(delimiters: Delimiters) -> Self {
        Self {
            segments: Vec::new(),
            delimiters,
        }
    }

    /// Add a segment to the message
    pub fn add_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    /// Get a segment by index
    pub fn get_segment(&self, index: usize) -> Option<&Segment> {
        self.segments.get(index)
    }

    /// Get a mutable segment by index
    pub fn get_segment_mut(&mut self, index: usize) -> Option<&mut Segment> {
        self.segments.get_mut(index)
    }

    /// Get all segments with a specific ID
    pub fn get_segments_by_id(&self, id: &str) -> Vec<&Segment> {
        self.segments.iter().filter(|s| s.id == id).collect()
    }

    /// Get the first segment with a specific ID
    ///
    /// Alias for `get_segment_by_id` for convenience
    pub fn get_segment_by_id(&self, id: &str) -> Option<&Segment> {
        self.segments.iter().find(|s| s.id == id)
    }

    /// Get a mutable reference to the first segment with a specific ID
    pub fn get_segment_by_id_mut(&mut self, id: &str) -> Option<&mut Segment> {
        self.segments.iter_mut().find(|s| s.id == id)
    }

    /// Convenience alias for get_segment_by_id
    #[inline]
    pub fn segment(&self, id: &str) -> Option<&Segment> {
        self.get_segment_by_id(id)
    }

    /// Get the MSH segment (should be the first segment)
    pub fn get_msh(&self) -> Option<&Segment> {
        self.segments.first().filter(|s| s.id == "MSH")
    }

    /// Get the message type from MSH-9
    ///
    /// MSH-9 format: MessageType^TriggerEvent (e.g., "ADT^A01")
    pub fn get_message_type(&self) -> Option<(String, String)> {
        self.get_msh().and_then(|msh| {
            msh.get_field(9).and_then(|field| {
                let rep = field.get_repetition(0)?;
                let msg_type = rep.get_component(0)?.value()?.to_string();
                let trigger = rep.get_component(1)?.value()?.to_string();
                Some((msg_type, trigger))
            })
        })
    }

    /// Get the message control ID from MSH-10
    pub fn get_control_id(&self) -> Option<&str> {
        self.get_msh()
            .and_then(|msh| msh.get_field_value(10))
    }

    /// Get the HL7 version from MSH-12
    pub fn get_version(&self) -> Option<Version> {
        self.get_msh()
            .and_then(|msh| msh.get_field_value(12))
            .and_then(Version::from_str)
    }

    /// Set the HL7 version in MSH-12
    pub fn set_version(&mut self, version: Version) -> Result<()> {
        if let Some(msh) = self.segments.first_mut() {
            if msh.id == "MSH" {
                msh.set_field_value(12, version.as_str())?;
                Ok(())
            } else {
                Err(Error::InvalidSegment(
                    "First segment must be MSH".to_string(),
                ))
            }
        } else {
            Err(Error::InvalidSegment("No segments in message".to_string()))
        }
    }

    /// Get sending application from MSH-3
    pub fn get_sending_application(&self) -> Option<&str> {
        self.get_msh().and_then(|msh| msh.get_field_value(3))
    }

    /// Get sending facility from MSH-4
    pub fn get_sending_facility(&self) -> Option<&str> {
        self.get_msh().and_then(|msh| msh.get_field_value(4))
    }

    /// Get receiving application from MSH-5
    pub fn get_receiving_application(&self) -> Option<&str> {
        self.get_msh().and_then(|msh| msh.get_field_value(5))
    }

    /// Get receiving facility from MSH-6
    pub fn get_receiving_facility(&self) -> Option<&str> {
        self.get_msh().and_then(|msh| msh.get_field_value(6))
    }

    /// Validate the message structure
    pub fn validate(&self) -> Result<()> {
        // Check that message has at least one segment
        if self.segments.is_empty() {
            return Err(Error::Validation(
                "Message must contain at least one segment".to_string(),
            ));
        }

        // Check that first segment is MSH
        let first = &self.segments[0];
        if first.id != "MSH" {
            return Err(Error::Validation(
                "First segment must be MSH".to_string(),
            ));
        }

        // Validate all segment IDs
        for segment in &self.segments {
            segment.validate_id()?;
        }

        // Validate delimiters
        self.delimiters.validate()?;

        Ok(())
    }

    /// Get the character set declared in MSH-18 (HL7 table 0211)
    ///
    /// Returns the UTF-8 default when MSH-18 is empty or missing, and an
    /// error when it names an unsupported character set.
    pub fn get_character_set(&self) -> Result<CharacterSet> {
        match self.get_msh().and_then(|msh| msh.get_field_value(18)) {
            None => Ok(CharacterSet::default()),
            Some(value) => CharacterSet::from_msh18(value),
        }
    }

    /// Set the character set in MSH-18 to its canonical table 0211 value
    pub fn set_character_set(&mut self, charset: CharacterSet) -> Result<()> {
        if let Some(msh) = self.segments.first_mut() {
            if msh.id == "MSH" {
                msh.set_field_value(18, charset.msh18())?;
                Ok(())
            } else {
                Err(Error::InvalidSegment(
                    "First segment must be MSH".to_string(),
                ))
            }
        } else {
            Err(Error::InvalidSegment("No segments in message".to_string()))
        }
    }

    /// Encode the message to HL7 format
    ///
    /// Segments are separated by carriage return (\r) or \r\n
    pub fn encode(&self) -> String {
        self.encode_with_separator("\r")
    }

    /// Encode with a custom segment separator
    pub fn encode_with_separator(&self, separator: &str) -> String {
        self.segments
            .iter()
            .map(|s| s.encode(&self.delimiters))
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Encode the message to wire bytes in its MSH-18 character set
    ///
    /// This is [`encode`](Self::encode) followed by conversion to the
    /// declared character set. Characters with no representation in that
    /// set are an error; use [`set_character_set`](Self::set_character_set)
    /// to declare a set that covers the message content.
    pub fn encode_bytes(&self) -> Result<Vec<u8>> {
        self.encode_bytes_with_separator("\r")
    }

    /// Encode to wire bytes with a custom segment separator
    pub fn encode_bytes_with_separator(&self, separator: &str) -> Result<Vec<u8>> {
        let charset = self.get_character_set()?;
        charset.encode(&self.encode_with_separator(separator))
    }

    /// Get the number of segments
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Check if this is an acknowledgment message (ACK)
    ///
    /// Only MSH-9-1 (the message type) is consulted, so genuine ACKs with
    /// a bare MSH-9 (`ACK` with no trigger component) are recognized.
    pub fn is_acknowledgment(&self) -> bool {
        self.get_msh()
            .and_then(|msh| msh.get_field_value(9))
            .map(|msg_type| msg_type == "ACK")
            .unwrap_or(false)
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

impl Message {
    /// Convert this message into an Arc for cheap cloning
    ///
    /// This is useful when the same message needs to be shared across multiple
    /// contexts (e.g., routing to multiple destinations, audit logging, etc.)
    /// without incurring the cost of deep cloning the entire message structure.
    ///
    /// # Example
    ///
    /// ```rust
    /// use vyna_hl7_core::message::Message;
    /// use std::sync::Arc;
    ///
    /// let message = Message::new();
    /// let shared: Arc<Message> = message.into_arc();
    ///
    /// // Cheap clone - just increments reference count
    /// let shared2 = Arc::clone(&shared);
    /// ```
    #[inline]
    pub fn into_arc(self) -> std::sync::Arc<Self> {
        std::sync::Arc::new(self)
    }

    /// Create an Arc-wrapped message directly
    ///
    /// This is a convenience method that creates a new message and wraps it
    /// in an Arc in one step.
    #[inline]
    pub fn new_arc() -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self::new())
    }
}

/// Message type identifiers
pub mod message_types {
    /// ADT - Admit/Discharge/Transfer
    pub const ADT: &str = "ADT";

    /// ORM - Order Message
    pub const ORM: &str = "ORM";

    /// ORU - Observation Result
    pub const ORU: &str = "ORU";

    /// ACK - General Acknowledgment
    pub const ACK: &str = "ACK";

    /// SIU - Scheduling Information Unsolicited
    pub const SIU: &str = "SIU";

    /// MDM - Medical Document Management
    pub const MDM: &str = "MDM";

    /// DFT - Detailed Financial Transaction
    pub const DFT: &str = "DFT";

    /// BAR - Billing Account Record
    pub const BAR: &str = "BAR";

    /// RAS - Pharmacy/Treatment Administration
    pub const RAS: &str = "RAS";

    /// RDE - Pharmacy/Treatment Encoded Order
    pub const RDE: &str = "RDE";
}

/// Common trigger events for ADT messages
pub mod trigger_events {
    // ADT trigger events
    /// A01 - Admit/visit notification
    pub const A01: &str = "A01";
    /// A02 - Transfer a patient
    pub const A02: &str = "A02";
    /// A03 - Discharge/end visit
    pub const A03: &str = "A03";
    /// A04 - Register a patient
    pub const A04: &str = "A04";
    /// A05 - Pre-admit a patient
    pub const A05: &str = "A05";
    /// A06 - Change an outpatient to an inpatient
    pub const A06: &str = "A06";
    /// A07 - Change an inpatient to an outpatient
    pub const A07: &str = "A07";
    /// A08 - Update patient information
    pub const A08: &str = "A08";
    /// A09 - Patient departing - tracking
    pub const A09: &str = "A09";
    /// A10 - Patient arriving - tracking
    pub const A10: &str = "A10";
    /// A11 - Cancel admit/visit notification
    pub const A11: &str = "A11";
    /// A12 - Cancel transfer
    pub const A12: &str = "A12";
    /// A13 - Cancel discharge/end visit
    pub const A13: &str = "A13";
    /// A17 - Swap patients
    pub const A17: &str = "A17";
    /// A28 - Add person information
    pub const A28: &str = "A28";
    /// A31 - Update person information
    pub const A31: &str = "A31";
    /// A40 - Merge patient - patient identifier list
    pub const A40: &str = "A40";

    // ORU trigger events
    /// R01 - Unsolicited transmission of an observation message
    pub const R01: &str = "R01";

    // ORM trigger events
    /// O01 - Order message
    pub const O01: &str = "O01";

    // SIU trigger events
    /// S12 - Notification of new appointment booking
    pub const S12: &str = "S12";
    /// S13 - Notification of appointment rescheduling
    pub const S13: &str = "S13";
    /// S14 - Notification of appointment modification
    pub const S14: &str = "S14";
    /// S15 - Notification of appointment cancellation
    pub const S15: &str = "S15";

    // MDM trigger events
    /// T01 - Original document notification
    pub const T01: &str = "T01";
    /// T02 - Original document notification and content
    pub const T02: &str = "T02";
    /// T04 - Document status change notification
    pub const T04: &str = "T04";

    // DFT trigger events
    /// P03 - Post detail financial transaction
    pub const P03: &str = "P03";
    /// P11 - Post detail financial transactions - expanded
    pub const P11: &str = "P11";

    // QRY trigger events
    /// A19 - Patient query
    pub const A19: &str = "A19";
    /// Q01 - Query sent for immediate response
    pub const Q01: &str = "Q01";
    /// Q02 - Query sent for deferred response
    pub const Q02: &str = "Q02";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Field;

    fn create_test_msh() -> Segment {
        let mut msh = Segment::new("MSH");
        msh.add_field(Field::from_value("^~\\&")); // MSH-1: encoding characters
        msh.add_field(Field::from_value("")); // MSH-2: empty placeholder
        msh.add_field(Field::from_value("SendingApp")); // MSH-3
        msh.add_field(Field::from_value("SendingFac")); // MSH-4
        msh.add_field(Field::from_value("ReceivingApp")); // MSH-5
        msh.add_field(Field::from_value("ReceivingFac")); // MSH-6
        msh
    }

    #[test]
    fn test_new_message() {
        let msg = Message::new();
        assert_eq!(msg.segments.len(), 0);
        assert_eq!(msg.delimiters, Delimiters::default());
    }

    #[test]
    fn test_add_segment() {
        let mut msg = Message::new();
        msg.add_segment(create_test_msh());
        assert_eq!(msg.segments.len(), 1);
    }

    #[test]
    fn test_get_msh() {
        let mut msg = Message::new();
        msg.add_segment(create_test_msh());

        let msh = msg.get_msh();
        assert!(msh.is_some());
        assert_eq!(msh.unwrap().id, "MSH");
    }

    #[test]
    fn test_get_sending_application() {
        let mut msg = Message::new();
        msg.add_segment(create_test_msh());

        assert_eq!(msg.get_sending_application(), Some("SendingApp"));
    }

    #[test]
    fn test_validate_empty_message() {
        let msg = Message::new();
        assert!(msg.validate().is_err());
    }

    #[test]
    fn test_validate_msh_first() {
        let mut msg = Message::new();
        msg.add_segment(Segment::new("PID"));
        assert!(msg.validate().is_err());
    }

    #[test]
    fn test_validate_valid_message() {
        let mut msg = Message::new();
        msg.add_segment(create_test_msh());
        msg.add_segment(Segment::new("PID"));
        assert!(msg.validate().is_ok());
    }

    #[test]
    fn test_encode_message() {
        let mut msg = Message::new();
        msg.add_segment(create_test_msh());

        let encoded = msg.encode();
        assert!(encoded.starts_with("MSH|^~\\&|"));
    }

    fn create_msh_with_type(msh9: Field) -> Segment {
        let mut msh = Segment::new("MSH");
        msh.add_field(Field::from_value("^~\\&")); // MSH-1: encoding characters
        msh.add_field(Field::from_value("")); // MSH-2: empty placeholder
        msh.add_field(Field::from_value("SendingApp")); // MSH-3
        msh.add_field(Field::from_value("SendingFac")); // MSH-4
        msh.add_field(Field::from_value("ReceivingApp")); // MSH-5
        msh.add_field(Field::from_value("ReceivingFac")); // MSH-6
        msh.add_field(Field::from_value("20240115120000")); // MSH-7
        msh.add_field(Field::from_value("")); // MSH-8
        msh.add_field(msh9); // MSH-9
        msh
    }

    fn create_message_with_type(msh9: Field) -> Message {
        let mut msg = Message::new();
        msg.add_segment(create_msh_with_type(msh9));
        msg
    }

    #[test]
    fn test_is_acknowledgment_with_bare_msh9() {
        // AckBuilder without a trigger event emits MSH-9 as a single
        // component ("ACK"); it must still be recognized as an ACK.
        let msg = create_message_with_type(Field::from_value("ACK"));
        assert!(msg.is_acknowledgment());
    }

    #[test]
    fn test_is_acknowledgment_with_trigger() {
        let msg = create_message_with_type(Field::from_delimited("ACK^A01"));
        assert!(msg.is_acknowledgment());
    }

    #[test]
    fn test_is_acknowledgment_false_for_non_ack() {
        let msg = create_message_with_type(Field::from_delimited("ADT^A01"));
        assert!(!msg.is_acknowledgment());
    }

    #[test]
    fn test_is_acknowledgment_false_without_msh9() {
        let mut msg = Message::new();
        msg.add_segment(create_test_msh());
        assert!(!msg.is_acknowledgment());
    }

    #[test]
    fn test_get_segments_by_id() {
        let mut msg = Message::new();
        msg.add_segment(create_test_msh());
        msg.add_segment(Segment::new("PID"));
        msg.add_segment(Segment::new("PID"));

        let pid_segments = msg.get_segments_by_id("PID");
        assert_eq!(pid_segments.len(), 2);
    }
}
