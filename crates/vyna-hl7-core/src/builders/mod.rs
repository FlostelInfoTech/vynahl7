//! Message builders for creating HL7 messages programmatically
//!
//! This module provides builder patterns for creating HL7 messages of various types.
//! Builders help ensure messages are created with required fields and proper structure.

pub mod ack;
pub mod adt;
pub mod bar;
pub mod batch;
pub mod dft;
pub mod fields;
pub mod laboratory;
pub mod mdm;
pub mod mfn;
pub mod omp;
pub mod orm;
pub mod oru;
pub mod pharmacy;
pub mod ppr;
pub mod qbp;
pub mod qry;
pub mod rich;
pub mod rsp;
pub mod siu;
pub mod vxu;

use crate::{
    Version,
    charset::CharacterSet,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
    types::format_timestamp,
};
use chrono::{Local, Utc};
use std::sync::atomic::{AtomicU64, Ordering};

/// Base message builder with common functionality
pub struct MessageBuilder {
    message: Message,
    version: Version,
    message_type: String,
    trigger_event: String,
    charset: Option<CharacterSet>,
    message_structure: Option<String>,
    utc_timestamps: bool,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new(version: Version, message_type: &str, trigger_event: &str) -> Self {
        Self {
            message: Message::new(),
            version,
            message_type: message_type.to_string(),
            trigger_event: trigger_event.to_string(),
            charset: Some(CharacterSet::Utf8),
            message_structure: None,
            utc_timestamps: true,
        }
    }

    /// Declare the wire character set in MSH-18 (HL7 table 0211).
    ///
    /// UTF-8 is declared by default, matching what the encoder emits;
    /// use this to declare a different set, or
    /// [`without_character_set`](Self::without_character_set) to end
    /// MSH at MSH-12.
    pub fn with_character_set(mut self, charset: CharacterSet) -> Self {
        self.charset = Some(charset);
        self
    }

    /// Omit the MSH-18 character set declaration (MSH ends at MSH-12).
    ///
    /// The encoder still writes UTF-8 bytes; a receiver then applies
    /// its own default per HL7 v2.5 §2.16.9. Prefer the default
    /// declaration unless a partner requires the shorter header.
    pub fn without_character_set(mut self) -> Self {
        self.charset = None;
        self
    }

    /// The trigger event this builder constructs (e.g. `A01`).
    pub fn trigger_event(&self) -> &str {
        &self.trigger_event
    }

    /// Mint MSH-7 and default EVN-2 from local time instead of UTC.
    ///
    /// UTC is the default: an HL7 timestamp carries no zone, and local
    /// minting disagrees with every UTC-rendered timestamp in the
    /// message by the host's offset. Use this only when the receiving
    /// ecosystem reads naive timestamps as local time.
    pub fn with_local_timestamps(mut self) -> Self {
        self.utc_timestamps = false;
        self
    }

    /// Set the MSH-9 message structure (third component of the MSG data
    /// type — e.g. `ADT_A01`). Required by HL7 v2.5+ and by partners that
    /// route on the full triple. When unset, MSH-9 stays two-component.
    pub fn message_structure(&mut self, structure: &str) -> &mut Self {
        self.message_structure = Some(structure.to_string());
        self
    }

    /// Override the trigger event after construction, so one specialised
    /// builder can emit variant messages (A02/A03/…) without rebuilding
    /// the base.
    pub fn set_trigger_event(&mut self, trigger: &str) -> &mut Self {
        self.trigger_event = trigger.to_string();
        self
    }

    /// Override the message type after construction, so one specialised
    /// builder can emit a related response (e.g. a REF builder emitting
    /// RRI) while reusing its segment logic.
    pub fn set_message_type(&mut self, message_type: &str) -> &mut Self {
        self.message_type = message_type.to_string();
        self
    }

    /// Create MSH segment with basic fields
    pub fn create_msh(
        &self,
        sending_app: &str,
        sending_facility: &str,
        receiving_app: &str,
        receiving_facility: &str,
        control_id: &str,
        processing_id: &str,
    ) -> Result<Segment> {
        // MSH-1/MSH-2 delimiter placeholders are pushed by the constructor.
        let mut msh = Segment::new_header("MSH");

        // MSH-3: Sending Application
        msh.add_field(Field::from_value(sending_app));

        // MSH-4: Sending Facility
        msh.add_field(Field::from_value(sending_facility));

        // MSH-5: Receiving Application
        msh.add_field(Field::from_value(receiving_app));

        // MSH-6: Receiving Facility
        msh.add_field(Field::from_value(receiving_facility));

        // MSH-7: Date/Time of Message. UTC, matching every other
        // timestamp in the message; local only via `with_local_timestamps`.
        let timestamp = if self.utc_timestamps {
            format_timestamp(&Utc::now().naive_utc())
        } else {
            format_timestamp(&Local::now().naive_local())
        };
        msh.add_field(Field::from_value(&timestamp));

        // MSH-8: Security (empty)
        msh.add_field(Field::from_value(""));

        // MSH-9: Message Type (MSG data type - proper component structure),
        // with the message structure third component when set.
        let mut msg_type_field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.message_type));
        rep.add_component(Component::from_value(&self.trigger_event));
        if let Some(structure) = &self.message_structure {
            rep.add_component(Component::from_value(structure));
        }
        msg_type_field.add_repetition(rep);
        msh.add_field(msg_type_field);

        // MSH-10: Message Control ID
        msh.add_field(Field::from_value(control_id));

        // MSH-11: Processing ID
        msh.add_field(Field::from_value(processing_id));

        // MSH-12: Version ID
        msh.add_field(Field::from_value(self.version.as_str()));

        // MSH-13..17: empty; MSH-18: Character Set (only when declared)
        if let Some(charset) = self.charset {
            for _ in 13..=17 {
                msh.add_field(Field::from_value(""));
            }
            msh.add_field(Field::from_value(charset.msh18()));
        }

        Ok(msh)
    }

    /// Create EVN segment
    pub fn create_evn(
        &self,
        event_type_code: &str,
        recorded_datetime: Option<&str>,
    ) -> Result<Segment> {
        let mut evn = Segment::new("EVN");

        // EVN-1: Event Type Code
        evn.add_field(Field::from_value(event_type_code));

        // EVN-2: Recorded Date/Time
        let timestamp = if let Some(dt) = recorded_datetime {
            dt.to_string()
        } else if self.utc_timestamps {
            format_timestamp(&Utc::now().naive_utc())
        } else {
            format_timestamp(&Local::now().naive_local())
        };
        evn.add_field(Field::from_value(&timestamp));

        Ok(evn)
    }

    /// EVN with `EVN-6` — when the event actually OCCURRED.
    ///
    /// `EVN-2` answers a different question (when the transaction was
    /// recorded); a receiver reading it as the event time gets the moment
    /// the message was built.
    pub fn create_evn_with_occurrence(
        &self,
        event_type_code: &str,
        recorded_datetime: Option<&str>,
        event_occurred: Option<&str>,
    ) -> Result<Segment> {
        let mut evn = self.create_evn(event_type_code, recorded_datetime)?;
        // EVN-3 .. EVN-5 have no source here; EVN-6 is the point of this.
        for _ in 3..=5 {
            evn.add_field(Field::from_value(""));
        }
        evn.add_field(Field::from_value(event_occurred.unwrap_or("")));
        Ok(evn)
    }

    /// Build the final message
    pub fn build(self) -> Message {
        self.message
    }
}

/// Process-wide sequence mixed into generated control IDs.
static CONTROL_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a unique message control ID.
///
/// Combines millisecond time with a process-wide counter so IDs stay unique
/// even when many messages are created within one millisecond (e.g. every
/// builder default MSH-10). The counter is offset by the process ID so two
/// processes starting together do not mint the same IDs in the same
/// millisecond. `MSG<millis><seq>` stays within MSH-10's 20-character limit.
pub fn generate_control_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // A pre-1970 system clock (broken RTC/VM) must not panic ID
    // generation: fall back to zero, uniqueness still holds via the
    // counter and PID mix below.
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id() as u64;
    let seq = CONTROL_ID_COUNTER
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(pid.wrapping_mul(7919))
        % 10_000;
    format!("MSG{millis}{seq:04}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_msh_with_character_set() {
        let builder = MessageBuilder::new(Version::V2_5, "ADT", "A01")
            .with_character_set(CharacterSet::Iso8859(1));
        let msh = builder.create_msh("S", "F", "R", "RF", "1", "P").unwrap();
        assert_eq!(msh.get_field_value(18), Some("8859/1"));
    }

    #[test]
    fn test_create_msh_declares_utf8_by_default() {
        let builder = MessageBuilder::new(Version::V2_5, "ADT", "A01");
        let msh = builder.create_msh("S", "F", "R", "RF", "1", "P").unwrap();
        assert_eq!(msh.get_field_value(18), Some("UNICODE UTF-8"));
    }

    #[test]
    fn test_create_msh_without_character_set_ends_at_msh12() {
        let builder = MessageBuilder::new(Version::V2_5, "ADT", "A01").without_character_set();
        let msh = builder.create_msh("S", "F", "R", "RF", "1", "P").unwrap();
        assert_eq!(msh.get_field_value(12), Some("2.5"));
        assert_eq!(msh.get_field_value(18), None);
    }

    #[test]
    fn generated_control_ids_are_unique_and_bounded() {
        use std::collections::HashSet;
        let ids: Vec<String> = (0..1000).map(|_| generate_control_id()).collect();
        let unique: HashSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate control IDs generated");
        for id in &ids {
            assert!(id.starts_with("MSG"), "unexpected control ID: {id}");
            assert!(id.len() <= 20, "control ID exceeds MSH-10 limit: {id}");
        }
    }
}
