//! Streaming parser for HL7 messages
//!
//! This module provides a streaming parser that can process HL7 messages
//! segment by segment without loading the entire message into memory.
//! This is useful for processing very large messages or when memory is constrained.
//!
//! # Examples
//!
//! ```rust
//! use vyna_hl7_parser::streaming::{StreamingParser, SegmentEvent};
//!
//! let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
//! PID|1||PAT001||DOE^JOHN
//! PV1|1|I|ICU";
//!
//! let mut parser = StreamingParser::new(hl7);
//!
//! while let Some(event) = parser.next_event() {
//!     match event {
//!         Ok(SegmentEvent::Delimiters { delimiters }) => {
//!             println!("Delimiters: field={}", delimiters.field_separator);
//!         }
//!         Ok(SegmentEvent::Start { id, line }) => {
//!             println!("Starting segment {} at line {}", id, line);
//!         }
//!         Ok(SegmentEvent::Field { index, value }) => {
//!             println!("  Field {}: {}", index, value);
//!         }
//!         Ok(SegmentEvent::End { id }) => {
//!             println!("Finished segment {}", id);
//!         }
//!         Ok(SegmentEvent::EndOfMessage) => {
//!             println!("End of message");
//!         }
//!         Err(e) => {
//!             eprintln!("Error: {}", e);
//!             break;
//!         }
//!     }
//! }
//! ```

use vyna_hl7_core::delimiters::Delimiters;
use vyna_hl7_core::encoding::Encoding;
use vyna_hl7_core::error::{Error, ErrorLocation, Result};
use vyna_hl7_core::field::{Component, Field, Repetition, SubComponent};
use vyna_hl7_core::message::Message;
use vyna_hl7_core::segment::Segment;

/// Events emitted by the streaming parser
#[derive(Debug, Clone)]
pub enum SegmentEvent<'a> {
    /// Start of a new segment
    Start {
        /// Segment ID (e.g., "MSH", "PID")
        id: &'a str,
        /// Line number in the source (1-based)
        line: usize,
    },
    /// A field within the current segment
    Field {
        /// Field index (1-based as per HL7 convention)
        index: usize,
        /// Raw field value
        value: &'a str,
    },
    /// End of the current segment
    End {
        /// Segment ID
        id: &'a str,
    },
    /// MSH delimiters detected
    Delimiters {
        /// The parsed delimiters
        delimiters: Delimiters,
    },
    /// End of the message
    EndOfMessage,
}

/// State of the streaming parser
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    /// Ready to start parsing
    Initial,
    /// Parsing the MSH segment (special handling)
    InMsh,
    /// Parsing a regular segment
    InSegment,
    /// Between segments
    BetweenSegments,
    /// Emitting end event
    EmitEnd,
    /// Reached end of input
    Done,
}

/// Streaming parser for HL7 messages
///
/// Parses HL7 messages incrementally, emitting events for each segment and field.
pub struct StreamingParser<'a> {
    state: ParserState,
    current_line: usize,
    current_segment_id: Option<&'a str>,
    current_field_index: usize,
    delimiters: Option<Delimiters>,
    lines: Vec<&'a str>,
    line_index: usize,
    field_position: usize,    // Position within current line (after segment ID)
    msh_sep: Option<&'a str>, // MSH-1 value slice (byte-accurate for MSH-2 offsets)
    msh_enc: Option<&'a str>, // MSH-2 value slice
}

impl<'a> StreamingParser<'a> {
    /// Create a new streaming parser for the given input
    pub fn new(input: &'a str) -> Self {
        // Pre-split into lines for easier processing (parity with the
        // non-streaming parser, which splits on both \r and \n)
        let lines: Vec<&str> = input.split('\r').flat_map(|s| s.split('\n')).collect();

        Self {
            state: ParserState::Initial,
            current_line: 1,
            current_segment_id: None,
            current_field_index: 0,
            delimiters: None,
            lines,
            line_index: 0,
            field_position: 0,
            msh_sep: None,
            msh_enc: None,
        }
    }

    /// Get the next event from the parser
    pub fn next_event(&mut self) -> Option<Result<SegmentEvent<'a>>> {
        match self.state {
            ParserState::Initial => self.parse_initial(),
            ParserState::InMsh => self.parse_msh_field(),
            ParserState::InSegment => self.parse_segment_field(),
            ParserState::BetweenSegments => self.parse_next_segment(),
            ParserState::EmitEnd => self.emit_end(),
            ParserState::Done => None,
        }
    }

    /// Parse the initial state (expecting MSH)
    fn parse_initial(&mut self) -> Option<Result<SegmentEvent<'a>>> {
        // Skip empty lines at the start
        while self.line_index < self.lines.len() {
            let line = self.lines[self.line_index].trim();
            if !line.is_empty() {
                break;
            }
            self.line_index += 1;
            self.current_line += 1;
        }

        if self.line_index >= self.lines.len() {
            self.state = ParserState::Done;
            return Some(Ok(SegmentEvent::EndOfMessage));
        }

        let line = self.lines[self.line_index];

        // Must start with MSH
        if !line.starts_with("MSH") {
            return Some(Err(Error::parse_at(
                "Message must start with MSH segment",
                ErrorLocation::new().line(self.current_line),
            )));
        }

        // Extract delimiters from MSH-1 and MSH-2. Eight bytes
        // ("MSH|^~\\&") is a complete bare header, like strict.
        if line.len() < 8 {
            return Some(Err(Error::parse_at(
                "MSH segment too short for delimiters",
                ErrorLocation::new().line(self.current_line),
            )));
        }

        let after_msh = match line.get(3..) {
            Some(s) => s,
            None => {
                return Some(Err(Error::parse_at(
                    "MSH segment too short for delimiters",
                    ErrorLocation::new().line(self.current_line),
                )));
            }
        };
        let field_sep = match after_msh.chars().next() {
            Some(c) => c,
            None => {
                return Some(Err(Error::parse_at(
                    "MSH segment too short for delimiters",
                    ErrorLocation::new().line(self.current_line),
                )));
            }
        };
        let sep_len = field_sep.len_utf8();
        let enc_start = 3 + sep_len;
        let after_sep = after_msh.strip_prefix(field_sep).unwrap_or("");
        let encoding_chars = match after_sep.get(..4) {
            Some(s) => s,
            None => {
                return Some(Err(Error::parse_at(
                    "MSH segment has invalid encoding characters",
                    ErrorLocation::new().line(self.current_line),
                )));
            }
        };
        let msh_sep = after_msh.get(..sep_len).unwrap_or("");

        let delimiters = match Delimiters::from_encoding_characters(field_sep, encoding_chars) {
            Ok(delimiters) => delimiters,
            Err(e) => {
                return Some(Err(Error::parse_at(
                    e.to_string(),
                    ErrorLocation::new().line(self.current_line),
                )));
            }
        };

        self.delimiters = Some(delimiters);
        self.current_segment_id = Some("MSH");
        self.current_field_index = 0;
        self.field_position = enc_start + 4; // After MSH + field sep + encoding chars
        self.msh_sep = Some(msh_sep);
        self.msh_enc = Some(encoding_chars);
        self.state = ParserState::InMsh;

        // First emit the delimiters event
        Some(Ok(SegmentEvent::Delimiters { delimiters }))
    }

    /// Parse MSH fields (special handling for field separator)
    fn parse_msh_field(&mut self) -> Option<Result<SegmentEvent<'a>>> {
        let line = self.lines[self.line_index];
        let (Some(delims), Some(msh_sep), Some(msh_enc)) =
            (self.delimiters.as_ref(), self.msh_sep, self.msh_enc)
        else {
            return Some(Err(Error::parse_at(
                "Streaming parser in invalid state",
                ErrorLocation::new().line(self.current_line),
            )));
        };

        // First call after delimiters: emit Start event
        if self.current_field_index == 0 {
            self.current_field_index = 1;
            return Some(Ok(SegmentEvent::Start {
                id: "MSH",
                line: self.current_line,
            }));
        }

        // MSH-1: Field separator
        if self.current_field_index == 1 {
            self.current_field_index = 2;
            return Some(Ok(SegmentEvent::Field {
                index: 1,
                value: msh_sep, // The field separator character
            }));
        }

        // MSH-2: Encoding characters
        if self.current_field_index == 2 {
            self.current_field_index = 3;
            return Some(Ok(SegmentEvent::Field {
                index: 2,
                value: msh_enc, // The encoding characters
            }));
        }

        // Remaining MSH fields
        if self.field_position >= line.len() {
            // Trailing separator yields one more empty field,
            // matching strict split on content-bearing lines.
            if self.field_position == line.len()
                && self.current_field_index >= 3
                && line.trim_end().ends_with(delims.field_separator)
            {
                self.current_field_index += 1;
                let field_index = self.current_field_index;
                self.field_position = line.len() + 1;
                return Some(Ok(SegmentEvent::Field {
                    index: field_index,
                    value: "",
                }));
            }
            // End of MSH segment
            self.state = ParserState::BetweenSegments;
            self.line_index += 1;
            self.current_line += 1;
            return Some(Ok(SegmentEvent::End { id: "MSH" }));
        }

        let remaining = match line.get(self.field_position..) {
            Some(remaining) => remaining,
            None => {
                return Some(Err(Error::parse_at(
                    "Invalid field position in MSH segment",
                    ErrorLocation::new().line(self.current_line),
                )));
            }
        };

        // After a separator-terminated field the position is already at
        // the next field, so only the first remaining field (MSH-3)
        // strips a leading separator; stripping again would swallow
        // empty fields and shift later fields left.
        let sep_len = delims.field_separator.len_utf8();
        let remaining = if self.current_field_index == 3 {
            if let Some(rest) = remaining.strip_prefix(delims.field_separator) {
                self.field_position += sep_len;
                rest
            } else {
                remaining
            }
        } else {
            remaining
        };

        if remaining.is_empty() {
            // Line ends right after a separator ("MSH|^~\&|"): strict
            // split yields one empty MSH-3, so emit it here.
            if line.trim_end().ends_with(delims.field_separator) {
                let field_index = self.current_field_index;
                self.current_field_index += 1;
                self.field_position = line.len() + 1;
                return Some(Ok(SegmentEvent::Field {
                    index: field_index,
                    value: "",
                }));
            }
            self.state = ParserState::BetweenSegments;
            self.line_index += 1;
            self.current_line += 1;
            return Some(Ok(SegmentEvent::End { id: "MSH" }));
        }

        // Find the next field separator
        if let Some(sep_pos) = remaining.find(delims.field_separator) {
            let field_value = &remaining[..sep_pos];
            let field_index = self.current_field_index;
            self.current_field_index += 1;
            self.field_position += sep_pos + sep_len;

            Some(Ok(SegmentEvent::Field {
                index: field_index,
                value: field_value,
            }))
        } else {
            // Last field
            let field_index = self.current_field_index;
            self.current_field_index += 1;
            self.state = ParserState::EmitEnd;

            Some(Ok(SegmentEvent::Field {
                index: field_index,
                value: remaining.trim_end(),
            }))
        }
    }

    /// Emit end event and transition to next state
    fn emit_end(&mut self) -> Option<Result<SegmentEvent<'a>>> {
        let id = self.current_segment_id.unwrap_or("???");
        self.state = ParserState::BetweenSegments;
        self.line_index += 1;
        self.current_line += 1;
        Some(Ok(SegmentEvent::End { id }))
    }

    /// Parse the next segment after between-segments state
    fn parse_next_segment(&mut self) -> Option<Result<SegmentEvent<'a>>> {
        // Skip empty lines
        while self.line_index < self.lines.len() {
            let line = self.lines[self.line_index].trim();
            if !line.is_empty() {
                break;
            }
            self.line_index += 1;
            self.current_line += 1;
        }

        if self.line_index >= self.lines.len() {
            self.state = ParserState::Done;
            return Some(Ok(SegmentEvent::EndOfMessage));
        }

        let line = self.lines[self.line_index];
        if line.trim().is_empty() {
            self.line_index += 1;
            self.current_line += 1;
            return self.parse_next_segment();
        }

        // Extract segment ID with strict parity: exactly 3 ASCII
        // alphanumerics followed by the field separator or end of line.
        let delims = self.delimiters.as_ref()?;
        let mut id_len = 0usize;
        let mut id_end = 0usize;
        for c in line.chars().take_while(|c| c.is_ascii_alphanumeric()) {
            id_len += 1;
            id_end += c.len_utf8();
        }

        if id_len == 0 {
            return Some(Err(Error::parse_at(
                "Empty segment ID",
                ErrorLocation::new().line(self.current_line),
            )));
        }

        let segment_id = &line[..id_end];
        if id_len != 3 {
            return Some(Err(Error::parse_at(
                format!(
                    "Segment ID must be 3 characters, got {} ('{}')",
                    id_len, segment_id
                ),
                ErrorLocation::new().line(self.current_line),
            )));
        }

        let after_id = &line[id_end..];
        if !(after_id.is_empty() || after_id.starts_with(delims.field_separator)) {
            return Some(Err(Error::parse_at(
                format!(
                    "Expected field separator after segment ID, got '{}'",
                    after_id.chars().next().unwrap_or(' ')
                ),
                ErrorLocation::new().line(self.current_line),
            )));
        }

        if segment_id == "MSH" {
            // Non-first MSH keeps MSH-1/2 handling: MSH-1/2 come
            // from the message delimiters and content starts
            // after the encoding characters.
            let sep_len = delims.field_separator.len_utf8();
            self.current_segment_id = Some(segment_id);
            self.current_field_index = 1;
            self.field_position = id_end + sep_len + 4;
            self.state = ParserState::InMsh;
            return Some(Ok(SegmentEvent::Start {
                id: segment_id,
                line: self.current_line,
            }));
        }

        self.current_segment_id = Some(segment_id);
        self.current_field_index = 0;
        self.field_position = id_end;
        self.state = ParserState::InSegment;

        Some(Ok(SegmentEvent::Start {
            id: segment_id,
            line: self.current_line,
        }))
    }

    /// Parse fields within a regular segment
    fn parse_segment_field(&mut self) -> Option<Result<SegmentEvent<'a>>> {
        let line = self.lines[self.line_index];
        let delims = self.delimiters.as_ref()?;
        let segment_id = self.current_segment_id?;

        // Check if we've gone past the end of the line
        if self.field_position >= line.len() {
            // A trailing separator yields one more empty field,
            // like strict split ("PID|1|2|" has 3 fields).
            if self.field_position == line.len()
                && self.current_field_index > 0
                && line.trim_end().ends_with(delims.field_separator)
            {
                self.current_field_index += 1;
                let field_index = self.current_field_index;
                self.field_position = line.len() + 1;
                return Some(Ok(SegmentEvent::Field {
                    index: field_index,
                    value: "",
                }));
            }
            self.state = ParserState::BetweenSegments;
            self.line_index += 1;
            self.current_line += 1;
            return Some(Ok(SegmentEvent::End { id: segment_id }));
        }

        let remaining = match line.get(self.field_position..) {
            Some(remaining) => remaining,
            None => {
                return Some(Err(Error::parse_at(
                    "Invalid field position in segment",
                    ErrorLocation::new().line(self.current_line),
                )));
            }
        };

        // After a separator-terminated field the position is already at
        // the next field, so only the first field strips a leading
        // separator; stripping again would swallow empty fields and
        // shift later fields left.
        let remaining = if self.current_field_index == 0 {
            if let Some(rest) = remaining.strip_prefix(delims.field_separator) {
                self.field_position += delims.field_separator.len_utf8();
                self.current_field_index += 1;
                rest
            } else {
                remaining
            }
        } else {
            self.current_field_index += 1;
            remaining
        };

        if remaining.is_empty() {
            // Line ends right after a separator ("PID|"): strict
            // split yields one empty field, so emit it here.
            if line.trim_end().ends_with(delims.field_separator) {
                let field_index = self.current_field_index;
                self.field_position = line.len() + 1;
                return Some(Ok(SegmentEvent::Field {
                    index: field_index,
                    value: "",
                }));
            }
            self.state = ParserState::BetweenSegments;
            self.line_index += 1;
            self.current_line += 1;
            return Some(Ok(SegmentEvent::End { id: segment_id }));
        }

        // Find next field separator
        if let Some(sep_pos) = remaining.find(delims.field_separator) {
            let field_value = &remaining[..sep_pos];
            let field_index = self.current_field_index;
            self.field_position += sep_pos + delims.field_separator.len_utf8();

            Some(Ok(SegmentEvent::Field {
                index: field_index,
                value: field_value,
            }))
        } else {
            // Last field, possibly empty when whitespace-only.
            let field_index = self.current_field_index;
            self.state = ParserState::EmitEnd;

            Some(Ok(SegmentEvent::Field {
                index: field_index,
                value: remaining.trim_end(),
            }))
        }
    }

    /// Get the current delimiters (available after parsing MSH)
    pub fn delimiters(&self) -> Option<&Delimiters> {
        self.delimiters.as_ref()
    }

    /// Get the current line number
    pub fn current_line(&self) -> usize {
        self.current_line
    }

    /// Check if parsing is complete
    pub fn is_done(&self) -> bool {
        self.state == ParserState::Done
    }
}

/// Builder for constructing messages from streaming events
pub struct StreamingMessageBuilder {
    message: Message,
    current_segment: Option<Segment>,
    delimiters: Option<Delimiters>,
}

impl StreamingMessageBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            message: Message::new(),
            current_segment: None,
            delimiters: None,
        }
    }

    /// Process an event from the streaming parser
    pub fn process_event(&mut self, event: SegmentEvent<'_>) -> Result<()> {
        match event {
            SegmentEvent::Delimiters { delimiters } => {
                self.delimiters = Some(delimiters);
                self.message.delimiters = delimiters;
            }
            SegmentEvent::Start { id, .. } => {
                // Finish previous segment if any
                if let Some(seg) = self.current_segment.take() {
                    self.message.add_segment(seg);
                }
                self.current_segment = Some(Segment::new(id));
            }
            SegmentEvent::Field { index, value } => {
                // Parse field and add to segment
                let delims = self.delimiters;
                if let (Some(seg), Some(delims)) = (&mut self.current_segment, delims.as_ref()) {
                    // MSH-1/MSH-2 hold raw delimiter characters, never
                    // escape-encoded data (mirrors the regular parser).
                    let field = if seg.id == "MSH" && index <= 2 {
                        Field::from_value(value)
                    } else {
                        Self::parse_field_static(value, delims)?
                    };
                    seg.add_field(field);
                }
            }
            SegmentEvent::End { .. } => {
                // Segment will be added when next segment starts or at EndOfMessage
            }
            SegmentEvent::EndOfMessage => {
                if let Some(seg) = self.current_segment.take() {
                    self.message.add_segment(seg);
                }
            }
        }
        Ok(())
    }

    /// Parse a field value into a Field structure (static version for borrow checker)
    ///
    /// Escape sequences are decoded, matching the regular parser.
    fn parse_field_static(value: &str, delims: &Delimiters) -> Result<Field> {
        let mut field = Field::new();

        // Split by repetition separator
        let repetitions: Vec<&str> = value.split(delims.repetition_separator).collect();

        for rep_value in repetitions {
            let mut repetition = Repetition::new();

            // Split by component separator
            let components: Vec<&str> = rep_value.split(delims.component_separator).collect();

            for comp_value in components {
                let mut component = Component::new();

                // Split by subcomponent separator
                let subcomponents: Vec<&str> =
                    comp_value.split(delims.subcomponent_separator).collect();

                for sub_value in subcomponents {
                    let decoded = Encoding::decode(sub_value, delims)?;
                    component.add_subcomponent(SubComponent::new(decoded));
                }

                repetition.add_component(component);
            }

            field.add_repetition(repetition);
        }

        Ok(field)
    }

    /// Finish building and return the message
    pub fn finish(mut self) -> Message {
        if let Some(seg) = self.current_segment.take() {
            self.message.add_segment(seg);
        }
        self.message
    }
}

impl Default for StreamingMessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse an HL7 message using streaming parser
///
/// This is a convenience function that uses the streaming parser internally
/// but returns a complete Message object.
pub fn parse_streaming(input: &str) -> Result<Message> {
    let mut parser = StreamingParser::new(input);
    let mut builder = StreamingMessageBuilder::new();

    while let Some(event_result) = parser.next_event() {
        let event = event_result?;
        builder.process_event(event)?;
    }

    let message = builder.finish();
    if message.segments.is_empty() {
        return Err(Error::parse("Message must start with MSH segment"));
    }
    Ok(message)
}

/// Segment callback for streaming processing
pub trait SegmentHandler {
    /// Called when a segment is completely parsed
    fn handle_segment(&mut self, segment: &Segment) -> Result<()>;

    /// Called when parsing is complete
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Process a message with a segment handler (callback-based streaming)
pub fn process_with_handler<H: SegmentHandler>(input: &str, handler: &mut H) -> Result<()> {
    let mut parser = StreamingParser::new(input);
    let mut current_segment: Option<Segment> = None;
    let mut delimiters: Option<Delimiters> = None;

    while let Some(event_result) = parser.next_event() {
        let event = event_result?;

        match event {
            SegmentEvent::Delimiters { delimiters: d } => {
                delimiters = Some(d);
            }
            SegmentEvent::Start { id, .. } => {
                // Handle previous segment if any
                if let Some(seg) = current_segment.take() {
                    handler.handle_segment(&seg)?;
                }
                current_segment = Some(Segment::new(id));
            }
            SegmentEvent::Field { index, value } => {
                if let (Some(seg), Some(delims)) = (&mut current_segment, &delimiters) {
                    // MSH-1/MSH-2 hold raw delimiter characters, never
                    // escape-encoded data (mirrors the regular parser).
                    let field = if seg.id == "MSH" && index <= 2 {
                        Field::from_value(value)
                    } else {
                        parse_field_with_delimiters(value, delims)?
                    };
                    seg.add_field(field);
                }
            }
            SegmentEvent::End { .. } => {
                // Segment will be handled when next segment starts
            }
            SegmentEvent::EndOfMessage => {
                if let Some(seg) = current_segment.take() {
                    handler.handle_segment(&seg)?;
                }
                handler.finish()?;
            }
        }
    }

    Ok(())
}

/// Parse a field value with given delimiters
///
/// Escape sequences are decoded, matching the regular parser.
fn parse_field_with_delimiters(value: &str, delims: &Delimiters) -> Result<Field> {
    let mut field = Field::new();

    let repetitions: Vec<&str> = value.split(delims.repetition_separator).collect();

    for rep_value in repetitions {
        let mut repetition = Repetition::new();
        let components: Vec<&str> = rep_value.split(delims.component_separator).collect();

        for comp_value in components {
            let mut component = Component::new();
            let subcomponents: Vec<&str> =
                comp_value.split(delims.subcomponent_separator).collect();

            for sub_value in subcomponents {
                let decoded = Encoding::decode(sub_value, delims)?;
                component.add_subcomponent(SubComponent::new(decoded));
            }

            repetition.add_component(component);
        }

        field.add_repetition(repetition);
    }

    Ok(field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_parser_basic() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001||DOE^JOHN\r";

        let mut parser = StreamingParser::new(hl7);
        let mut events = Vec::new();

        while let Some(event) = parser.next_event() {
            events.push(event.unwrap());
        }

        // Check we got the right events
        assert!(matches!(events[0], SegmentEvent::Delimiters { .. }));
        assert!(matches!(events[1], SegmentEvent::Start { id: "MSH", .. }));
    }

    #[test]
    fn test_streaming_parser_delimiters() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r";

        let mut parser = StreamingParser::new(hl7);

        // First event should be delimiters
        let event = parser.next_event().unwrap().unwrap();
        if let SegmentEvent::Delimiters { delimiters } = event {
            assert_eq!(delimiters.field_separator, '|');
            assert_eq!(delimiters.component_separator, '^');
            assert_eq!(delimiters.repetition_separator, '~');
            assert_eq!(delimiters.escape_character, '\\');
            assert_eq!(delimiters.subcomponent_separator, '&');
        } else {
            panic!("Expected Delimiters event");
        }
    }

    #[test]
    fn test_streaming_message_builder() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001||DOE^JOHN\r";

        let message = parse_streaming(hl7).unwrap();

        assert_eq!(message.segments.len(), 2);
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "PID");
    }

    #[test]
    fn test_streaming_parser_multiple_segments() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\rPID|1||PAT001\rOBR|1||ORDER001\rOBX|1|NM|WBC||7.5\rOBX|2|NM|RBC||4.5\r";

        let message = parse_streaming(hl7).unwrap();

        assert_eq!(message.segments.len(), 5);
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "PID");
        assert_eq!(message.segments[2].id, "OBR");
        assert_eq!(message.segments[3].id, "OBX");
        assert_eq!(message.segments[4].id, "OBX");
    }

    #[test]
    fn test_segment_handler() {
        struct CountingHandler {
            segment_count: usize,
            segment_ids: Vec<String>,
        }

        impl SegmentHandler for CountingHandler {
            fn handle_segment(&mut self, segment: &Segment) -> Result<()> {
                self.segment_count += 1;
                self.segment_ids.push(segment.id.clone());
                Ok(())
            }
        }

        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001\rPV1|1|I|ICU\r";

        let mut handler = CountingHandler {
            segment_count: 0,
            segment_ids: Vec::new(),
        };

        process_with_handler(hl7, &mut handler).unwrap();

        assert_eq!(handler.segment_count, 3);
        assert_eq!(handler.segment_ids, vec!["MSH", "PID", "PV1"]);
    }

    #[test]
    fn test_parse_streaming_equivalence() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001||DOE^JOHN\r";

        let streaming = parse_streaming(hl7).unwrap();
        let regular = crate::parse_message(hl7).unwrap();

        // Both should produce equivalent messages
        assert_eq!(streaming.segments.len(), regular.segments.len());
        assert_eq!(streaming.segments[0].id, regular.segments[0].id);
        assert_eq!(streaming.segments[1].id, regular.segments[1].id);
    }

    #[test]
    fn test_empty_fields() {
        let hl7 = "MSH|^~\\&|APP|FAC||||||P|2.5\r";

        let message = parse_streaming(hl7).unwrap();

        assert_eq!(message.segments.len(), 1);
        // Should have multiple fields including empty ones
        assert!(message.segments[0].fields.len() > 3);
    }

    #[test]
    fn test_components_and_subcomponents() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01^ADT_A01|123|P|2.5\r";

        let message = parse_streaming(hl7).unwrap();

        // Find the field containing ADT^A01^ADT_A01 (should have 3 components)
        let msh = &message.segments[0];
        // The streaming parser adds fields as it parses them
        // Look for any field that has 3+ components
        let has_components = msh.fields.iter().any(|f| {
            f.get_repetition(0)
                .map(|rep| rep.components.len() >= 3)
                .unwrap_or(false)
        });

        assert!(
            has_components,
            "Message should have a field with 3+ components"
        );
    }

    /// Drive the parser to completion, returning the first error if any.
    fn drain_to_end(parser: &mut StreamingParser<'_>) -> Result<()> {
        while let Some(event) = parser.next_event() {
            event?;
        }
        Ok(())
    }

    #[test]
    fn test_streaming_short_msh_returns_error() {
        for input in ["M", "MS", "MSH", "MSH|", "MSH|^~\\"] {
            let mut parser = StreamingParser::new(input);
            let mut saw_error = false;
            while let Some(event) = parser.next_event() {
                if event.is_err() {
                    saw_error = true;
                    break;
                }
            }
            assert!(saw_error, "input {input:?} should error, not panic");
        }
    }

    #[test]
    fn test_streaming_non_msh_start_returns_error() {
        let mut parser = StreamingParser::new("PID|1||PAT001");
        let event = parser.next_event().unwrap();
        assert!(event.is_err());
    }

    #[test]
    fn test_streaming_empty_input_ends_cleanly() {
        let mut parser = StreamingParser::new("");
        let event = parser.next_event().unwrap().unwrap();
        assert!(matches!(event, SegmentEvent::EndOfMessage));
        assert!(parser.next_event().is_none());
    }

    #[test]
    fn test_parse_streaming_rejects_empty_input() {
        // No segments means no MSH: error like parse_message does.
        for input in ["", "   ", "  \r\n  \r\n"] {
            let err = parse_streaming(input).unwrap_err().to_string();
            assert!(err.contains("must start with MSH"), "{input:?}: {err}");
        }
    }

    #[test]
    fn test_streaming_non_ascii_header_returns_error_without_panic() {
        // Each input must produce an error (or clean end), never a panic.
        for input in [
            "MSHé^~\\&|APP\r",
            "MSH|é~\\&|APP\r",
            "MSH|^é\\&|APP\r",
            "MSH|^~é&|APP\r",
            "MSHéééééééééé\r",
            "MSH|^~\\&|APP\rPéD|1|2\r",
            "MSH|^~\\&|APP\ra€|1|2\r",
            "MSH|^~\\&|APP\r€€€|1\r",
        ] {
            let mut parser = StreamingParser::new(input);
            let _ = drain_to_end(&mut parser);
        }
    }

    #[test]
    fn test_streaming_malformed_segments_do_not_panic() {
        for input in [
            "MSH|^~\\&|APP\r|\r",
            "MSH|^~\\&|APP\rAB\r",
            "MSH|^~\\&|APP\rA\r",
            "MSH|^~\\&|APP\r\r\rPID|1\r",
            "MSH|^~\\&\r",
            "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5",
        ] {
            let mut parser = StreamingParser::new(input);
            let _ = drain_to_end(&mut parser);
        }
    }

    #[test]
    fn test_streaming_non_ascii_field_values_still_parse() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1|PAT001|Müller^Jürg\r";
        let message = parse_streaming(hl7).unwrap();
        assert_eq!(message.segments.len(), 2);
        let pid = &message.segments[1];
        assert_eq!(pid.id, "PID");
        assert_eq!(pid.get_field_value(1), Some("1"));
        assert_eq!(pid.get_field_value(2), Some("PAT001"));
        assert_eq!(pid.get_field_value(3), Some("Müller"));
    }

    #[test]
    fn test_streaming_invalid_segment_id_returns_error() {
        let mut parser = StreamingParser::new("MSH|^~\\&|APP\rA\r");
        let mut saw_error = false;
        while let Some(event) = parser.next_event() {
            if event.is_err() {
                saw_error = true;
                break;
            }
        }
        assert!(saw_error);
    }

    #[test]
    fn test_streaming_rejects_non_3char_segment_id_like_strict() {
        // Strict parity: IDs that are not exactly 3 characters error.
        for line in ["ZABC|1|2", "AB|1|2", "A|1|2"] {
            let hl7 = format!("MSH|^~\\&|APP\r{line}\r");
            assert!(crate::parse_message(&hl7).is_err(), "{line}: strict");
            assert!(parse_streaming(&hl7).is_err(), "{line}: streaming");
        }
        // A 3-character ID still parses.
        let ok = "MSH|^~\\&|APP\rPID|1||PAT001\r";
        assert!(parse_streaming(ok).is_ok());
    }

    #[test]
    fn test_msh_state_without_delimiters_returns_error() {
        // A parser forced into MSH-field state without delimiters must
        // report an invalid-state error instead of panicking.
        let mut parser = StreamingParser::new("MSH|^~\\&|APP\r");
        parser.state = ParserState::InMsh;
        let event = parser.next_event().expect("expected an event");
        assert!(event.is_err());
    }

    #[test]
    fn test_streaming_splits_lf_separated_segments() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\n\
                   PID|1||PAT001||DOE^JOHN\n\
                   PV1|1|I|ICU";
        let message = parse_streaming(hl7).unwrap();
        let ids: Vec<&str> = message.segments.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["MSH", "PID", "PV1"]);

        // Parity with the non-streaming parser.
        let regular = crate::parse_message(hl7).unwrap();
        assert_eq!(message.segments.len(), regular.segments.len());
    }

    #[test]
    fn test_streaming_splits_crlf_separated_segments() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\n\
                   PID|1|PAT001\r\n";
        let message = parse_streaming(hl7).unwrap();
        let ids: Vec<&str> = message.segments.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, vec!["MSH", "PID"]);
        assert_eq!(message.segments[1].get_field_value(2), Some("PAT001"));
    }

    #[test]
    fn test_streaming_decodes_escape_sequences() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1|PAT001|Test\\F\\Value\r";
        let message = parse_streaming(hl7).unwrap();
        assert_eq!(message.segments[1].get_field_value(3), Some("Test|Value"));

        // Parity with the non-streaming parser.
        let regular = crate::parse_message(hl7).unwrap();
        assert_eq!(regular.segments[1].get_field_value(3), Some("Test|Value"));
    }

    #[test]
    fn test_streaming_rejects_invalid_escape_sequences() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1|Bad\\Q\\Value\r";
        assert!(crate::parse_message(hl7).is_err());
        assert!(parse_streaming(hl7).is_err());
    }

    #[test]
    fn test_handler_path_decodes_escape_sequences() {
        struct Capture {
            fields: Vec<String>,
        }
        impl SegmentHandler for Capture {
            fn handle_segment(&mut self, segment: &Segment) -> Result<()> {
                for field in &segment.fields {
                    self.fields.push(field.value().unwrap_or("").to_string());
                }
                Ok(())
            }
        }

        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1|Test\\F\\Value\r";
        let mut handler = Capture { fields: Vec::new() };
        process_with_handler(hl7, &mut handler).unwrap();
        assert!(handler.fields.iter().any(|f| f == "Test|Value"));
        assert!(!handler.fields.iter().any(|f| f.contains("\\F\\")));
    }

    #[test]
    fn test_streaming_preserves_empty_fields_like_strict() {
        // Empty wire positions must not shift later fields left.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|MSG001|P|2.5\r\
                   PID|1||PAT001||DOE^JOHN\r";
        let streaming = parse_streaming(hl7).unwrap();
        let regular = crate::parse_message(hl7).unwrap();

        let msh = &streaming.segments[0];
        assert_eq!(msh.get_field_value(9), Some("ADT"));
        assert_eq!(
            streaming.get_message_type(),
            Some(("ADT".to_string(), "A01".to_string()))
        );
        let pid = &streaming.segments[1];
        assert_eq!(pid.get_field_value(3), Some("PAT001"));
        assert_eq!(pid.get_field_value(5), Some("DOE"));

        assert_eq!(msh.fields.len(), regular.segments[0].fields.len());
        assert_eq!(pid.fields.len(), regular.segments[1].fields.len());
    }

    #[test]
    fn test_streaming_validates_encoding_characters() {
        // Duplicate delimiters and short (multibyte) encoding characters
        // must error, matching the non-streaming parser.
        for hl7 in ["MSH|^^^^|APP\rPID|1\r", "MSH|é~\\&|APP\rPID|1\r"] {
            assert!(crate::parse_message(hl7).is_err(), "regular parses {hl7:?}");
            assert!(parse_streaming(hl7).is_err(), "streaming parses {hl7:?}");
        }
    }

    #[test]
    fn test_streaming_second_msh_keeps_alignment() {
        let hl7 = "MSH|^~\\&|APP1|FAC1|R|D|20240101||ADT^A01|123|P|2.5\r\
                   PID|1||PAT001\r\
                   MSH|^~\\&|APP2|FAC2|R|D|20240102||ADT^A01|124|P|2.5\r";
        let message = parse_streaming(hl7).unwrap();
        let second = message
            .segments
            .iter()
            .filter(|s| s.id == "MSH")
            .nth(1)
            .unwrap();
        assert_eq!(second.get_field_value(3), Some("APP2"));
        assert_eq!(second.get_field_value(9), Some("ADT"));
        assert!(second.encode(&message.delimiters).contains("APP2"));
    }

    #[test]
    fn test_streaming_trailing_delimiter_matches_strict() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1|2|\r";
        let streaming = parse_streaming(hl7).unwrap();
        let regular = crate::parse_message(hl7).unwrap();
        let pid_s = streaming.segments.iter().find(|s| s.id == "PID").unwrap();
        let pid_r = regular.segments.iter().find(|s| s.id == "PID").unwrap();
        assert_eq!(pid_s.fields.len(), 3);
        assert_eq!(pid_s.fields.len(), pid_r.fields.len());

        let hl7 = "MSH|^~\\&|APP|FAC|\rPID|1\r";
        let streaming = parse_streaming(hl7).unwrap();
        let regular = crate::parse_message(hl7).unwrap();
        assert_eq!(
            streaming.segments[0].fields.len(),
            regular.segments[0].fields.len()
        );
    }

    #[test]
    fn test_streaming_accepts_bare_eight_byte_msh() {
        let bare = "MSH|^~\\&";
        assert_eq!(bare.len(), 8);
        let streaming = parse_streaming(bare).unwrap();
        let regular = crate::parse_message(bare).unwrap();
        assert_eq!(streaming.segments.len(), 1);
        assert_eq!(
            streaming.segments[0].fields.len(),
            regular.segments[0].fields.len()
        );
    }

    #[test]
    fn test_streaming_msh_explicit_empty_msh3_is_some_empty() {
        // 'MSH|^~\&|' carries an explicit empty MSH-3, like any other
        // trailing empty field; only the bare header has no MSH-3.
        let streaming = parse_streaming("MSH|^~\\&|\r").unwrap();
        assert_eq!(streaming.segments[0].get_field_value(3), Some(""));
        let bare = parse_streaming("MSH|^~\\&").unwrap();
        assert_eq!(bare.segments[0].get_field(3), None);
    }
}
