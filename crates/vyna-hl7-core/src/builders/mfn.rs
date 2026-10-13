//! MFN (Master File Notification) message builders

use super::{MessageBuilder, generate_control_id};
use crate::{Version, error::Result, field::Field, message::Message, segment::Segment};

/// Builder for MFN^M01 - Master File Notification
///
/// Notifies about master file records (locations, staff, services). The
/// payload segments carrying the record data ride along via
/// [`MfnM01Builder::add_segment`] (e.g. LOC, STF, PRA); the schema's `Hxx`
/// slot accepts any segment there.
pub struct MfnM01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    master_file_id: Option<String>,
    master_file_text: Option<String>,
    file_event: Option<String>,
    response_level: Option<String>,
    record_event: Option<String>,
    mf_control_id: Option<String>,
    primary_key: Option<String>,
    primary_key_type: Option<String>,
    payload: Vec<Segment>,
}

impl MfnM01Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "MFN", "M01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            master_file_id: None,
            master_file_text: None,
            file_event: None,
            response_level: None,
            record_event: None,
            mf_control_id: None,
            primary_key: None,
            primary_key_type: None,
            payload: Vec::new(),
        }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.sending_app = app.to_string();
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.sending_facility = facility.to_string();
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.receiving_app = app.to_string();
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.receiving_facility = facility.to_string();
        self
    }

    pub fn control_id(mut self, id: &str) -> Self {
        self.control_id = Some(id.to_string());
        self
    }

    /// MFI-1 master file identifier, HL7 table 0175 (default `LOC`).
    pub fn master_file_id(mut self, id: &str) -> Self {
        self.master_file_id = Some(id.to_string());
        self
    }

    /// MFI-1 display text for the master file identifier.
    pub fn master_file_text(mut self, text: &str) -> Self {
        self.master_file_text = Some(text.to_string());
        self
    }

    /// MFI-3 file-level event code, HL7 table 0178 (default `UPD`).
    pub fn file_event(mut self, event: &str) -> Self {
        self.file_event = Some(event.to_string());
        self
    }

    /// MFI-6 response level code, HL7 table 0179 (default `NE`).
    pub fn response_level(mut self, level: &str) -> Self {
        self.response_level = Some(level.to_string());
        self
    }

    /// MFE-1 record-level event code, HL7 table 0180 (default `MUP`).
    pub fn record_event(mut self, event: &str) -> Self {
        self.record_event = Some(event.to_string());
        self
    }

    /// MFE-2 master file control identifier.
    pub fn mf_control_id(mut self, id: &str) -> Self {
        self.mf_control_id = Some(id.to_string());
        self
    }

    /// MFE-4 primary key value identifying the record.
    pub fn primary_key(mut self, key: &str) -> Self {
        self.primary_key = Some(key.to_string());
        self
    }

    /// MFE-5 primary key value type, HL7 table 0355 (default `CE`).
    pub fn primary_key_type(mut self, key_type: &str) -> Self {
        self.primary_key_type = Some(key_type.to_string());
        self
    }

    /// Append a payload segment carrying the master file record data.
    pub fn add_segment(mut self, segment: Segment) -> Self {
        self.payload.push(segment);
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self.control_id.clone().unwrap_or_else(generate_control_id);
        let msh = self.base.create_msh(
            &self.sending_app,
            &self.sending_facility,
            &self.receiving_app,
            &self.receiving_facility,
            &control_id,
            "P",
        )?;
        self.base.message.add_segment(msh);

        // MFI segment (master file identification)
        let mut mfi = Segment::new("MFI");
        let file_id = self.master_file_id.as_deref().unwrap_or("LOC");
        let file_text = self.master_file_text.as_deref().unwrap_or("");
        let mfi1 = if file_text.is_empty() {
            file_id.to_string()
        } else {
            format!("{file_id}^{file_text}")
        };
        mfi.add_field(Field::from_delimited(mfi1)); // MFI-1: Master File Identifier
        mfi.add_field(Field::from_value("")); // MFI-2: Master File Application ID
        mfi.add_field(Field::from_value(
            self.file_event.as_deref().unwrap_or("UPD"),
        )); // MFI-3: File-Level Event Code
        mfi.add_field(Field::from_value("")); // MFI-4: Entered Date/Time
        mfi.add_field(Field::from_value("")); // MFI-5: Effective Date/Time
        mfi.add_field(Field::from_value(
            self.response_level.as_deref().unwrap_or("NE"),
        )); // MFI-6: Response Level Code
        self.base.message.add_segment(mfi);

        // MFE segment (master file record event)
        let mut mfe = Segment::new("MFE");
        mfe.add_field(Field::from_value(
            self.record_event.as_deref().unwrap_or("MUP"),
        )); // MFE-1: Record-Level Event Code
        mfe.add_field(Field::from_value(
            self.mf_control_id.as_deref().unwrap_or(""),
        )); // MFE-2: MFN Control ID
        mfe.add_field(Field::from_value("")); // MFE-3: Effective Date/Time
        mfe.add_field(Field::from_delimited(
            self.primary_key.as_deref().unwrap_or(""),
        )); // MFE-4: Primary Key Value
        mfe.add_field(Field::from_value(
            self.primary_key_type.as_deref().unwrap_or("CE"),
        )); // MFE-5: Primary Key Value Type
        self.base.message.add_segment(mfe);

        for segment in self.payload {
            self.base.message.add_segment(segment);
        }

        Ok(self.base.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(msg: &Message, id: &str) -> String {
        msg.encode()
            .split(['\r', '\n'])
            .find(|l| l.starts_with(id))
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn mfn_m01_builds_msh_mfi_mfe() {
        let msg = MfnM01Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .control_id("MFN1")
            .master_file_id("LOC")
            .master_file_text("Location")
            .record_event("MUP")
            .mf_control_id("MF100")
            .primary_key("LOC1")
            .build()
            .unwrap();
        let wire = msg.encode();
        assert!(wire.contains("MFN^M01"), "{wire}");
        assert!(segment(&msg, "MFI|").contains("LOC^Location"), "{wire}");
        let mfe = segment(&msg, "MFE|");
        assert!(mfe.contains("|MUP|MF100||LOC1|CE"), "{mfe}");
    }

    #[test]
    fn mfn_m01_defaults_cover_required_content() {
        let msg = MfnM01Builder::new(Version::V2_5)
            .primary_key("LOC1")
            .build()
            .unwrap();
        let wire = msg.encode();
        assert!(segment(&msg, "MFI|").contains("|LOC||UPD|||NE"), "{wire}");
        assert!(segment(&msg, "MFE|").contains("|MUP|||LOC1|CE"), "{wire}");
    }

    #[test]
    fn mfn_m01_carries_payload_segments() {
        let mut loc = Segment::new("LOC");
        loc.add_field(Field::from_value("LOC1"));
        let msg = MfnM01Builder::new(Version::V2_5)
            .primary_key("LOC1")
            .add_segment(loc)
            .build()
            .unwrap();
        assert!(msg.encode().contains("\rLOC|LOC1"), "{}", msg.encode());
    }
}
