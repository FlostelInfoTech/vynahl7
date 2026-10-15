//! ACK (Acknowledgment) Message Builder
//!
//! This module provides builders for creating HL7 acknowledgment messages.
//! ACK messages are used to acknowledge receipt of HL7 messages and report
//! processing status.
//!
//! # Overview
//!
//! HL7 defines two acknowledgment modes:
//! - **Original Mode**: Uses AA (Accept), AE (Error), AR (Reject)
//! - **Enhanced Mode**: Uses CA (Commit Accept), CE (Commit Error), CR (Commit Reject)
//!
//! # Examples
//!
//! ## Creating an ACK from an incoming message
//!
//! ```rust
//! use vyna_hl7_core::builders::ack::AckBuilder;
//! use vyna_hl7_core::builders::adt::AdtA01Builder;
//! use vyna_hl7_core::Version;
//!
//! let incoming = AdtA01Builder::new(Version::V2_5)
//!     .sending_application("SendApp")
//!     .sending_facility("SendFac")
//!     .receiving_application("RecApp")
//!     .receiving_facility("RecFac")
//!     .control_id("MSG001")
//!     .build()
//!     .unwrap();
//!
//! // Create a successful acknowledgment
//! let ack = AckBuilder::for_message(&incoming)
//!     .accept()
//!     .build()
//!     .unwrap();
//!
//! // Create an error acknowledgment
//! let nack = AckBuilder::for_message(&incoming)
//!     .error("Patient ID not found")
//!     .error_code("204", "Unknown Key Identifier")
//!     .build()
//!     .unwrap();
//! ```
//!
//! ## Creating an ACK with ERR segment
//!
//! ```rust
//! use vyna_hl7_core::builders::ack::{AckBuilder, ErrorSeverity};
//! use vyna_hl7_core::builders::adt::AdtA01Builder;
//! use vyna_hl7_core::Version;
//!
//! let incoming = AdtA01Builder::new(Version::V2_5)
//!     .sending_application("App")
//!     .sending_facility("Fac")
//!     .control_id("MSG001")
//!     .build()
//!     .unwrap();
//!
//! let ack = AckBuilder::for_message(&incoming)
//!     .reject("Required field missing")
//!     .add_error(
//!         "PID",
//!         Some(5),
//!         "101",
//!         "Required field missing",
//!         ErrorSeverity::Error,
//!     )
//!     .build()
//!     .unwrap();
//! ```

use crate::{
    Version,
    charset::CharacterSet,
    delimiters::Delimiters,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
    types::format_timestamp,
};
use chrono::Local;

/// Acknowledgment codes for Original Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckCode {
    /// AA - Application Accept: Message was successfully processed
    Accept,
    /// AE - Application Error: Error in processing, message may be resent
    Error,
    /// AR - Application Reject: Message rejected, do not resend
    Reject,
}

impl AckCode {
    /// Get the HL7 code string
    pub fn as_str(&self) -> &'static str {
        match self {
            AckCode::Accept => "AA",
            AckCode::Error => "AE",
            AckCode::Reject => "AR",
        }
    }
}

/// Acknowledgment codes for Enhanced Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitAckCode {
    /// CA - Commit Accept: Message committed to safe storage
    Accept,
    /// CE - Commit Error: Error committing message
    Error,
    /// CR - Commit Reject: Message rejected, not committed
    Reject,
}

impl CommitAckCode {
    /// Get the HL7 code string
    pub fn as_str(&self) -> &'static str {
        match self {
            CommitAckCode::Accept => "CA",
            CommitAckCode::Error => "CE",
            CommitAckCode::Reject => "CR",
        }
    }
}

/// Error severity levels for ERR segment
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// W - Warning: Processing completed with warnings
    Warning,
    /// I - Information: Informational message
    Information,
    /// E - Error: Error in processing
    Error,
    /// F - Fatal Error: Fatal error, message not processed
    Fatal,
}

impl ErrorSeverity {
    /// Get the HL7 code string
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Warning => "W",
            ErrorSeverity::Information => "I",
            ErrorSeverity::Error => "E",
            ErrorSeverity::Fatal => "F",
        }
    }
}

/// Error information for ERR segment
#[derive(Debug, Clone)]
pub struct ErrorInfo {
    /// Segment ID where error occurred (e.g., "PID")
    pub segment_id: String,
    /// Sequence number (1-based segment occurrence)
    pub sequence: Option<usize>,
    /// Field position (1-based)
    pub field_position: Option<usize>,
    /// Error code (from HL7 Table 0357)
    pub error_code: String,
    /// Error code description
    pub error_code_description: String,
    /// Severity level
    pub severity: ErrorSeverity,
    /// User-friendly message
    pub user_message: Option<String>,
    /// Diagnostic information
    pub diagnostic_info: Option<String>,
}

/// Builder for ACK (Acknowledgment) messages
///
/// This builder creates properly formatted ACK messages in response to
/// incoming HL7 messages. It handles both Original Mode and Enhanced Mode
/// acknowledgments.
#[derive(Debug, Clone)]
pub struct AckBuilder {
    /// Version from the original message
    version: Version,
    /// Original message control ID (for MSA-2)
    original_control_id: String,
    /// Original sending application (becomes receiving app in ACK)
    original_sending_app: String,
    /// Original sending facility (becomes receiving facility in ACK)
    original_sending_facility: String,
    /// Original receiving application (becomes sending app in ACK)
    original_receiving_app: String,
    /// Original receiving facility (becomes sending facility in ACK)
    original_receiving_facility: String,
    /// Acknowledgment code
    ack_code: AckCode,
    /// Text message for MSA-3
    text_message: Option<String>,
    /// Error code for MSA-6
    error_code: Option<String>,
    /// Errors for ERR segment
    errors: Vec<ErrorInfo>,
    /// Sending application override
    sending_app_override: Option<String>,
    /// Sending facility override
    sending_facility_override: Option<String>,
    /// Custom message control ID
    control_id_override: Option<String>,
    /// Processing ID (P, D, T)
    processing_id: String,
    /// Character set for MSH-18, echoed from the incoming message
    charset: Option<CharacterSet>,
    /// Trigger event echoed in MSH-9-2 (e.g. `A01` for `ACK^A01`)
    trigger_event: Option<String>,
}

impl AckBuilder {
    /// Create an ACK builder for an incoming message
    ///
    /// This extracts the necessary information from the incoming message
    /// to create a proper acknowledgment response.
    ///
    /// # Arguments
    ///
    /// * `message` - The incoming HL7 message to acknowledge
    ///
    /// # Returns
    ///
    /// A new `AckBuilder` configured for the incoming message
    ///
    /// # Example
    ///
    /// ```rust
    /// use vyna_hl7_core::builders::ack::AckBuilder;
    /// use vyna_hl7_core::builders::adt::AdtA01Builder;
    /// use vyna_hl7_core::Version;
    ///
    /// let incoming = AdtA01Builder::new(Version::V2_5)
    ///     .sending_application("SendApp")
    ///     .sending_facility("SendFac")
    ///     .receiving_application("RecApp")
    ///     .receiving_facility("RecFac")
    ///     .control_id("MSG001")
    ///     .build()
    ///     .unwrap();
    /// let ack = AckBuilder::for_message(&incoming)
    ///     .accept()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn for_message(message: &Message) -> Self {
        let msh = message.get_msh();

        // Extract values from MSH, with defaults
        let version = message.get_version().unwrap_or(Version::V2_5);
        let original_control_id = message.get_control_id().unwrap_or("").to_string();
        let original_sending_app = message.get_sending_application().unwrap_or("").to_string();
        let original_sending_facility = message.get_sending_facility().unwrap_or("").to_string();
        let original_receiving_app = message
            .get_receiving_application()
            .unwrap_or("")
            .to_string();
        let original_receiving_facility =
            message.get_receiving_facility().unwrap_or("").to_string();

        // Get processing ID from MSH-11
        let processing_id = msh
            .and_then(|m| m.get_field_value(11))
            .unwrap_or("P")
            .to_string();

        // Echo the declared character set so the ACK encodes like the
        // incoming message. Undeclared or invalid values stay unset.
        let charset = msh
            .and_then(|m| m.get_field_value(18))
            .filter(|value| !value.is_empty())
            .and_then(|value| CharacterSet::from_msh18(value).ok());

        // Echo the trigger event so MSH-9 reads e.g. `ACK^A01`.
        let trigger_event = message
            .get_message_type()
            .map(|(_, trigger)| trigger)
            .filter(|trigger| !trigger.is_empty());

        Self {
            version,
            original_control_id,
            original_sending_app,
            original_sending_facility,
            original_receiving_app,
            original_receiving_facility,
            ack_code: AckCode::Accept,
            text_message: None,
            error_code: None,
            errors: Vec::new(),
            sending_app_override: None,
            sending_facility_override: None,
            control_id_override: None,
            processing_id,
            charset,
            trigger_event,
        }
    }

    /// Create a new ACK builder with explicit parameters
    ///
    /// Use this when you don't have access to the original message object
    /// but know the required parameters.
    pub fn new(
        version: Version,
        original_control_id: &str,
        original_sending_app: &str,
        original_sending_facility: &str,
    ) -> Self {
        Self {
            version,
            original_control_id: original_control_id.to_string(),
            original_sending_app: original_sending_app.to_string(),
            original_sending_facility: original_sending_facility.to_string(),
            original_receiving_app: String::new(),
            original_receiving_facility: String::new(),
            ack_code: AckCode::Accept,
            text_message: None,
            error_code: None,
            errors: Vec::new(),
            sending_app_override: None,
            sending_facility_override: None,
            control_id_override: None,
            processing_id: "P".to_string(),
            charset: None,
            trigger_event: None,
        }
    }

    /// Set the trigger event echoed in MSH-9-2 (e.g. `A01`).
    ///
    /// Required by v2.3+ schemas; [`for_message`](Self::for_message) fills it
    /// from the incoming message automatically.
    pub fn trigger_event(mut self, trigger: &str) -> Self {
        self.trigger_event = Some(trigger.to_string());
        self
    }

    /// Declare the ACK wire character set in MSH-18 (HL7 table 0211)
    ///
    /// Overrides the value echoed from the incoming message, if any.
    pub fn character_set(mut self, charset: CharacterSet) -> Self {
        self.charset = Some(charset);
        self
    }

    /// Set acknowledgment to Accept (AA)
    ///
    /// Indicates the message was successfully received and processed.
    pub fn accept(mut self) -> Self {
        self.ack_code = AckCode::Accept;
        self
    }

    /// Set acknowledgment to Error (AE) with a message
    ///
    /// Indicates an error occurred during processing. The sender may
    /// attempt to resend the message after correcting the issue.
    ///
    /// # Arguments
    ///
    /// * `message` - Description of the error
    pub fn error(mut self, message: &str) -> Self {
        self.ack_code = AckCode::Error;
        self.text_message = Some(message.to_string());
        self
    }

    /// Set acknowledgment to Reject (AR) with a message
    ///
    /// Indicates the message was rejected and should not be resent.
    ///
    /// # Arguments
    ///
    /// * `message` - Reason for rejection
    pub fn reject(mut self, message: &str) -> Self {
        self.ack_code = AckCode::Reject;
        self.text_message = Some(message.to_string());
        self
    }

    /// Set the acknowledgment code directly
    pub fn ack_code(mut self, code: AckCode) -> Self {
        self.ack_code = code;
        self
    }

    /// Set the text message (MSA-3)
    pub fn text_message(mut self, message: &str) -> Self {
        self.text_message = Some(message.to_string());
        self
    }

    /// Set the error code and description (for MSA-6 and ERR)
    ///
    /// Common error codes from HL7 Table 0357:
    /// - "0" - Message accepted
    /// - "100" - Segment sequence error
    /// - "101" - Required field missing
    /// - "102" - Data type error
    /// - "103" - Table value not found
    /// - "200" - Unsupported message type
    /// - "201" - Unsupported event code
    /// - "202" - Unsupported processing id
    /// - "203" - Unsupported version id
    /// - "204" - Unknown key identifier
    /// - "205" - Duplicate key identifier
    /// - "206" - Application record locked
    /// - "207" - Application internal error
    pub fn error_code(mut self, code: &str, description: &str) -> Self {
        self.error_code = Some(format!("{}^{}", code, description));
        self
    }

    /// Add a detailed error to the ERR segment
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The segment where the error occurred (e.g., "PID")
    /// * `field_position` - The field position (1-based), if applicable
    /// * `error_code` - Error code from HL7 Table 0357
    /// * `error_description` - Description of the error
    /// * `severity` - Error severity level
    pub fn add_error(
        mut self,
        segment_id: &str,
        field_position: Option<usize>,
        error_code: &str,
        error_description: &str,
        severity: ErrorSeverity,
    ) -> Self {
        self.errors.push(ErrorInfo {
            segment_id: segment_id.to_string(),
            sequence: None,
            field_position,
            error_code: error_code.to_string(),
            error_code_description: error_description.to_string(),
            severity,
            user_message: None,
            diagnostic_info: None,
        });
        self
    }

    /// Add a detailed error with full information
    pub fn add_error_detail(mut self, error: ErrorInfo) -> Self {
        self.errors.push(error);
        self
    }

    /// Override the sending application in the ACK
    pub fn sending_application(mut self, app: &str) -> Self {
        self.sending_app_override = Some(app.to_string());
        self
    }

    /// Override the sending facility in the ACK
    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.sending_facility_override = Some(facility.to_string());
        self
    }

    /// Set a custom message control ID
    ///
    /// If not set, a unique ID will be generated automatically.
    pub fn control_id(mut self, id: &str) -> Self {
        self.control_id_override = Some(id.to_string());
        self
    }

    /// Set the processing ID (P=Production, D=Debugging, T=Training)
    pub fn processing_id(mut self, id: &str) -> Self {
        self.processing_id = id.to_string();
        self
    }

    /// Build the ACK message
    ///
    /// # Returns
    ///
    /// The complete ACK message with MSH, MSA, and optionally ERR segments.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or invalid.
    pub fn build(self) -> Result<Message> {
        let mut message = Message::new();

        // Build MSH segment
        let msh = self.build_msh()?;
        message.add_segment(msh);

        // Build MSA segment
        let msa = self.build_msa()?;
        message.add_segment(msa);

        // Build ERR segment(s) if there are errors
        for error in &self.errors {
            let err = self.build_err(error)?;
            message.add_segment(err);
        }

        Ok(message)
    }

    /// Build the MSH segment for the ACK
    fn build_msh(&self) -> Result<Segment> {
        let mut msh = Segment::new("MSH");
        let delims = Delimiters::default();

        // MSH-1: Field separator
        msh.add_field(Field::from_value(delims.field_separator.to_string()));

        // MSH-2: Encoding characters
        msh.add_field(Field::from_value(delims.encoding_characters()));

        // MSH-3: Sending Application (was receiving app in original)
        let sending_app = self
            .sending_app_override
            .as_deref()
            .unwrap_or(&self.original_receiving_app);
        msh.add_field(Field::from_value(sending_app));

        // MSH-4: Sending Facility (was receiving facility in original)
        let sending_facility = self
            .sending_facility_override
            .as_deref()
            .unwrap_or(&self.original_receiving_facility);
        msh.add_field(Field::from_value(sending_facility));

        // MSH-5: Receiving Application (was sending app in original)
        msh.add_field(Field::from_value(&self.original_sending_app));

        // MSH-6: Receiving Facility (was sending facility in original)
        msh.add_field(Field::from_value(&self.original_sending_facility));

        // MSH-7: Date/Time of Message
        let timestamp = format_timestamp(&Local::now().naive_local());
        msh.add_field(Field::from_value(&timestamp));

        // MSH-8: Security (empty)
        msh.add_field(Field::from_value(""));

        // MSH-9: Message Type (ACK) with trigger echo, e.g. ACK^A01.
        // Components are split (not a single "ACK^A01" value) so MSH-9-2
        // validates as its own component.
        let mut msg_type_field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("ACK"));
        if let Some(trigger) = &self.trigger_event {
            rep.add_component(Component::from_value(trigger));
        }
        msg_type_field.add_repetition(rep);
        msh.add_field(msg_type_field);

        // MSH-10: Message Control ID
        let control_id = self
            .control_id_override
            .clone()
            .unwrap_or_else(super::generate_control_id);
        msh.add_field(Field::from_value(&control_id));

        // MSH-11: Processing ID
        msh.add_field(Field::from_value(&self.processing_id));

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

    /// Build the MSA (Message Acknowledgment) segment
    fn build_msa(&self) -> Result<Segment> {
        let mut msa = Segment::new("MSA");

        // MSA-1: Acknowledgment Code
        msa.add_field(Field::from_value(self.ack_code.as_str()));

        // MSA-2: Message Control ID (from original message)
        msa.add_field(Field::from_value(&self.original_control_id));

        // MSA-3: Text Message (optional)
        if let Some(ref text) = self.text_message {
            msa.add_field(Field::from_value(text));
        } else {
            msa.add_field(Field::from_value(""));
        }

        // MSA-4: Expected Sequence Number (empty)
        msa.add_field(Field::from_value(""));

        // MSA-5: Delayed Acknowledgment Type (empty, deprecated)
        msa.add_field(Field::from_value(""));

        // MSA-6: Error Condition (optional)
        if let Some(ref error_code) = self.error_code {
            msa.add_field(Field::from_delimited(error_code));
        }

        Ok(msa)
    }

    /// Build an ERR segment for error details
    fn build_err(&self, error: &ErrorInfo) -> Result<Segment> {
        let mut err = Segment::new("ERR");

        // ERR-1: Error Code and Location (deprecated in v2.5+, but included for compatibility)
        // Format: Segment^Sequence^Field^Component^Subcomponent
        let mut location_parts = vec![error.segment_id.clone()];
        if let Some(seq) = error.sequence {
            location_parts.push(seq.to_string());
        } else {
            location_parts.push("1".to_string());
        }
        if let Some(field) = error.field_position {
            location_parts.push(field.to_string());
        }
        let error_location = location_parts.join("^");
        err.add_field(Field::from_delimited(&error_location));

        // ERR-2: Error Location (v2.5+)
        // Format: Segment ID^Segment Sequence^Field Position^Field Repetition^Component Number^Subcomponent Number
        err.add_field(Field::from_delimited(&error_location));

        // ERR-3: HL7 Error Code
        let error_code_field = format!(
            "{}^{}^HL70357",
            error.error_code, error.error_code_description
        );
        err.add_field(Field::from_delimited(&error_code_field));

        // ERR-4: Severity
        err.add_field(Field::from_value(error.severity.as_str()));

        // ERR-5: Application Error Code (empty)
        err.add_field(Field::from_value(""));

        // ERR-6: Application Error Parameter (empty)
        err.add_field(Field::from_value(""));

        // ERR-7: Diagnostic Information
        if let Some(ref diag) = error.diagnostic_info {
            err.add_field(Field::from_value(diag));
        } else {
            err.add_field(Field::from_value(""));
        }

        // ERR-8: User Message
        if let Some(ref user_msg) = error.user_message {
            err.add_field(Field::from_value(user_msg));
        }

        Ok(err)
    }
}

/// Builder for Enhanced Mode Acknowledgments
///
/// Enhanced Mode acknowledgments use CA/CE/CR codes instead of AA/AE/AR.
/// They indicate whether the message has been committed to safe storage.
#[derive(Debug, Clone)]
pub struct CommitAckBuilder {
    inner: AckBuilder,
    commit_code: CommitAckCode,
}

impl CommitAckBuilder {
    /// Create a Commit ACK builder for an incoming message
    pub fn for_message(message: &Message) -> Self {
        Self {
            inner: AckBuilder::for_message(message),
            commit_code: CommitAckCode::Accept,
        }
    }

    /// Set commit acknowledgment to Accept (CA)
    pub fn commit_accept(mut self) -> Self {
        self.commit_code = CommitAckCode::Accept;
        self
    }

    /// Set commit acknowledgment to Error (CE)
    pub fn commit_error(mut self, message: &str) -> Self {
        self.commit_code = CommitAckCode::Error;
        self.inner.text_message = Some(message.to_string());
        self
    }

    /// Set commit acknowledgment to Reject (CR)
    pub fn commit_reject(mut self, message: &str) -> Self {
        self.commit_code = CommitAckCode::Reject;
        self.inner.text_message = Some(message.to_string());
        self
    }

    /// Add an error detail
    pub fn add_error(
        mut self,
        segment_id: &str,
        field_position: Option<usize>,
        error_code: &str,
        error_description: &str,
        severity: ErrorSeverity,
    ) -> Self {
        self.inner = self.inner.add_error(
            segment_id,
            field_position,
            error_code,
            error_description,
            severity,
        );
        self
    }

    /// Build the commit acknowledgment message
    pub fn build(mut self) -> Result<Message> {
        // Override the ack code based on commit code
        self.inner.ack_code = match self.commit_code {
            CommitAckCode::Accept => AckCode::Accept,
            CommitAckCode::Error => AckCode::Error,
            CommitAckCode::Reject => AckCode::Reject,
        };

        let mut message = self.inner.build()?;

        // Update MSA-1 to use commit code instead
        if let Some(msa) = message.get_segment_by_id_mut("MSA")
            && let Some(field) = msa.get_field_mut(1)
        {
            *field = Field::from_value(self.commit_code.as_str());
        }

        Ok(message)
    }
}

/// Quick helper functions for common ACK scenarios
impl AckBuilder {
    /// Create a simple accept acknowledgment
    ///
    /// # Example
    ///
    /// ```rust
    /// use vyna_hl7_core::builders::ack::AckBuilder;
    /// use vyna_hl7_core::builders::adt::AdtA01Builder;
    /// use vyna_hl7_core::Version;
    ///
    /// let msg = AdtA01Builder::new(Version::V2_5)
    ///     .sending_application("App")
    ///     .sending_facility("Fac")
    ///     .control_id("123")
    ///     .build()
    ///     .unwrap();
    /// let ack = AckBuilder::accept_message(&msg).unwrap();
    /// ```
    pub fn accept_message(message: &Message) -> Result<Message> {
        Self::for_message(message).accept().build()
    }

    /// Create a simple error acknowledgment
    pub fn error_message(message: &Message, error_text: &str) -> Result<Message> {
        Self::for_message(message).error(error_text).build()
    }

    /// Create a simple reject acknowledgment
    pub fn reject_message(message: &Message, reject_reason: &str) -> Result<Message> {
        Self::for_message(message).reject(reject_reason).build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn create_test_message() -> Message {
        let mut msg = Message::new();
        let mut msh = Segment::new("MSH");

        // MSH-1
        msh.add_field(Field::from_value("|"));
        // MSH-2
        msh.add_field(Field::from_value("^~\\&"));
        // MSH-3: Sending Application
        msh.add_field(Field::from_value("SendApp"));
        // MSH-4: Sending Facility
        msh.add_field(Field::from_value("SendFac"));
        // MSH-5: Receiving Application
        msh.add_field(Field::from_value("RecApp"));
        // MSH-6: Receiving Facility
        msh.add_field(Field::from_value("RecFac"));
        // MSH-7: DateTime
        msh.add_field(Field::from_value("20240315120000"));
        // MSH-8: Security
        msh.add_field(Field::from_value(""));
        // MSH-9: Message Type
        msh.add_field(Field::from_delimited("ADT^A01"));
        // MSH-10: Control ID
        msh.add_field(Field::from_value("MSG001"));
        // MSH-11: Processing ID
        msh.add_field(Field::from_value("P"));
        // MSH-12: Version
        msh.add_field(Field::from_value("2.5"));

        msg.add_segment(msh);
        msg
    }

    #[test]
    fn test_accept_ack() {
        let incoming = create_test_message();
        let ack = AckBuilder::for_message(&incoming).accept().build().unwrap();

        // Verify MSH
        let msh = ack.get_msh().unwrap();
        assert_eq!(msh.get_field_value(9), Some("ACK")); // Message type
        assert_eq!(msh.get_field_value(3), Some("RecApp")); // Sending app (was receiving)
        assert_eq!(msh.get_field_value(5), Some("SendApp")); // Receiving app (was sending)

        // Verify MSA
        let msa = ack.segment("MSA").unwrap();
        assert_eq!(msa.get_field_value(1), Some("AA")); // Accept code
        assert_eq!(msa.get_field_value(2), Some("MSG001")); // Original control ID
    }

    #[test]
    fn test_error_ack() {
        let incoming = create_test_message();
        let ack = AckBuilder::for_message(&incoming)
            .error("Invalid patient ID")
            .error_code("204", "Unknown Key Identifier")
            .build()
            .unwrap();

        let msa = ack.segment("MSA").unwrap();
        assert_eq!(msa.get_field_value(1), Some("AE"));
        assert_eq!(msa.get_field_value(3), Some("Invalid patient ID"));
        assert_eq!(msa.get_field_value(6), Some("204"));
        assert_eq!(
            msa.get_field(6)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("Unknown Key Identifier")
        );
    }

    #[test]
    fn test_reject_ack() {
        let incoming = create_test_message();
        let ack = AckBuilder::for_message(&incoming)
            .reject("Unsupported message type")
            .build()
            .unwrap();

        let msa = ack.segment("MSA").unwrap();
        assert_eq!(msa.get_field_value(1), Some("AR"));
    }

    #[test]
    fn test_ack_with_err_segment() {
        let incoming = create_test_message();
        let ack = AckBuilder::for_message(&incoming)
            .error("Validation failed")
            .add_error(
                "PID",
                Some(5),
                "101",
                "Required field missing",
                ErrorSeverity::Error,
            )
            .build()
            .unwrap();

        // Should have ERR segment
        let err = ack.segment("ERR").unwrap();
        assert!(err.get_field_value(1).unwrap().contains("PID"));
        assert!(err.get_field_value(3).unwrap().contains("101"));
        assert_eq!(err.get_field_value(4), Some("E"));
    }

    #[test]
    fn test_commit_ack() {
        let incoming = create_test_message();
        let ack = CommitAckBuilder::for_message(&incoming)
            .commit_accept()
            .build()
            .unwrap();

        let msa = ack.segment("MSA").unwrap();
        assert_eq!(msa.get_field_value(1), Some("CA"));
    }

    #[test]
    fn test_quick_accept() {
        let incoming = create_test_message();
        let ack = AckBuilder::accept_message(&incoming).unwrap();

        let msa = ack.segment("MSA").unwrap();
        assert_eq!(msa.get_field_value(1), Some("AA"));
    }

    #[test]
    fn test_custom_sending_app() {
        let incoming = create_test_message();
        let ack = AckBuilder::for_message(&incoming)
            .sending_application("MyApp")
            .sending_facility("MyFacility")
            .accept()
            .build()
            .unwrap();

        let msh = ack.get_msh().unwrap();
        assert_eq!(msh.get_field_value(3), Some("MyApp"));
        assert_eq!(msh.get_field_value(4), Some("MyFacility"));
    }

    #[test]
    fn test_ack_code_strings() {
        assert_eq!(AckCode::Accept.as_str(), "AA");
        assert_eq!(AckCode::Error.as_str(), "AE");
        assert_eq!(AckCode::Reject.as_str(), "AR");

        assert_eq!(CommitAckCode::Accept.as_str(), "CA");
        assert_eq!(CommitAckCode::Error.as_str(), "CE");
        assert_eq!(CommitAckCode::Reject.as_str(), "CR");
    }

    #[test]
    fn test_error_severity_strings() {
        assert_eq!(ErrorSeverity::Warning.as_str(), "W");
        assert_eq!(ErrorSeverity::Information.as_str(), "I");
        assert_eq!(ErrorSeverity::Error.as_str(), "E");
        assert_eq!(ErrorSeverity::Fatal.as_str(), "F");
    }

    fn incoming_with_charset(declaration: Option<&str>) -> Message {
        let mut message = Message::new();
        let mut msh = Segment::new("MSH");
        for value in [
            "|",
            "^~\\&",
            "SendApp",
            "SendFac",
            "RecApp",
            "RecFac",
            "20240315120000",
            "",
            "ADT^A01",
            "MSG001",
            "P",
            "2.5",
        ] {
            msh.add_field(Field::from_delimited(value));
        }
        if let Some(charset) = declaration {
            for _ in 13..=17 {
                msh.add_field(Field::from_value(""));
            }
            msh.add_field(Field::from_value(charset));
        }
        message.add_segment(msh);
        message
    }

    #[test]
    fn test_ack_echoes_declared_charset() {
        let incoming = incoming_with_charset(Some("8859/1"));
        let ack = AckBuilder::for_message(&incoming).accept().build().unwrap();
        assert_eq!(ack.get_msh().unwrap().get_field_value(18), Some("8859/1"));
        assert_eq!(ack.get_character_set().unwrap(), CharacterSet::Iso8859(1));
    }

    #[test]
    fn test_ack_omits_undeclared_charset() {
        let incoming = incoming_with_charset(None);
        let ack = AckBuilder::for_message(&incoming).accept().build().unwrap();
        assert_eq!(ack.get_msh().unwrap().get_field_value(18), None);
    }

    #[test]
    fn test_ack_charset_override() {
        let incoming = incoming_with_charset(Some("8859/1"));
        let ack = AckBuilder::for_message(&incoming)
            .character_set(CharacterSet::Utf8)
            .accept()
            .build()
            .unwrap();
        assert_eq!(
            ack.get_msh().unwrap().get_field_value(18),
            Some("UNICODE UTF-8")
        );
    }
}
