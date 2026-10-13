//! Message builders for creating HL7 messages programmatically
//!
//! This module provides builder patterns for creating HL7 messages of various types.
//! Builders help ensure messages are created with required fields and proper structure.

pub mod ack;
pub mod adt;
pub mod batch;
pub mod dft;
pub mod fields;
pub mod laboratory;
pub mod mdm;
pub mod omp;
pub mod orm;
pub mod oru;
pub mod pharmacy;
pub mod ppr;
pub mod qbp;
pub mod qry;
pub mod rsp;
pub mod siu;
pub mod vxu;

use crate::{
    charset::CharacterSet,
    delimiters::Delimiters,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
    types::format_timestamp,
    Version,
};
use chrono::Local;

/// Base message builder with common functionality
pub struct MessageBuilder {
    message: Message,
    version: Version,
    message_type: String,
    trigger_event: String,
    charset: Option<CharacterSet>,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new(version: Version, message_type: &str, trigger_event: &str) -> Self {
        Self {
            message: Message::new(),
            version,
            message_type: message_type.to_string(),
            trigger_event: trigger_event.to_string(),
            charset: None,
        }
    }

    /// Declare the wire character set in MSH-18 (HL7 table 0211)
    ///
    /// When set, [`create_msh`](Self::create_msh) emits MSH-13 through
    /// MSH-18 with the canonical declaration. When unset (default), MSH
    /// ends at MSH-12 and the message encodes as UTF-8.
    pub fn with_character_set(mut self, charset: CharacterSet) -> Self {
        self.charset = Some(charset);
        self
    }

    /// The trigger event this builder constructs (e.g. `A01`).
    pub fn trigger_event(&self) -> &str {
        &self.trigger_event
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
        let mut msh = Segment::new("MSH");
        let delims = Delimiters::default();

        // MSH-1: Field separator (special handling)
        msh.add_field(Field::from_value(delims.field_separator.to_string()));

        // MSH-2: Encoding characters
        msh.add_field(Field::from_value(format!(
            "{}{}{}{}",
            delims.component_separator, delims.repetition_separator, delims.escape_character, delims.subcomponent_separator
        )));

        // MSH-3: Sending Application
        msh.add_field(Field::from_value(sending_app));

        // MSH-4: Sending Facility
        msh.add_field(Field::from_value(sending_facility));

        // MSH-5: Receiving Application
        msh.add_field(Field::from_value(receiving_app));

        // MSH-6: Receiving Facility
        msh.add_field(Field::from_value(receiving_facility));

        // MSH-7: Date/Time of Message
        let timestamp = format_timestamp(&Local::now().naive_local());
        msh.add_field(Field::from_value(&timestamp));

        // MSH-8: Security (empty)
        msh.add_field(Field::from_value(""));

        // MSH-9: Message Type (MSG data type - proper component structure)
        let mut msg_type_field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.message_type));
        rep.add_component(Component::from_value(&self.trigger_event));
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
    pub fn create_evn(&self, event_type_code: &str, recorded_datetime: Option<&str>) -> Result<Segment> {
        let mut evn = Segment::new("EVN");

        // EVN-1: Event Type Code
        evn.add_field(Field::from_value(event_type_code));

        // EVN-2: Recorded Date/Time
        let timestamp = if let Some(dt) = recorded_datetime {
            dt.to_string()
        } else {
            format_timestamp(&Local::now().naive_local())
        };
        evn.add_field(Field::from_value(&timestamp));

        Ok(evn)
    }

    /// Build the final message
    pub fn build(self) -> Message {
        self.message
    }
}

/// Generate a message control ID (simple implementation)
pub fn generate_control_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("MSG{}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_msh_with_character_set() {
        let builder = MessageBuilder::new(Version::V2_5, "ADT", "A01")
            .with_character_set(CharacterSet::Iso8859(1));
        let msh = builder
            .create_msh("S", "F", "R", "RF", "1", "P")
            .unwrap();
        assert_eq!(msh.get_field_value(18), Some("8859/1"));
    }

    #[test]
    fn test_create_msh_without_character_set_ends_at_msh12() {
        let builder = MessageBuilder::new(Version::V2_5, "ADT", "A01");
        let msh = builder
            .create_msh("S", "F", "R", "RF", "1", "P")
            .unwrap();
        assert_eq!(msh.get_field_value(12), Some("2.5"));
        assert_eq!(msh.get_field_value(18), None);
    }
}
