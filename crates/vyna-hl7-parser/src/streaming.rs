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

/// Floor a byte index down to a char boundary so slicing never panics.
fn floor_char_boundary(s: &str, mut end: usize) -> usize {
    end = end.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    end
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
        // Pre-split into lines for easier processing
        let lines: Vec<&str> = input.split('\r').collect();

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

        // Extract delimiters from MSH-1 and MSH-2
        if line.len() < 9 {
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

        let delimiters = Delimiters {
            field_separator: field_sep,
            component_separator: encoding_chars.chars().next().unwrap_or('^'),
            repetition_separator: encoding_chars.chars().nth(1).unwrap_or('~'),
            escape_character: encoding_chars.chars().nth(2).unwrap_or('\\'),
            subcomponent_separator: encoding_chars.chars().nth(3).unwrap_or('&'),
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
        let delims = self.delimiters.as_ref().unwrap();
        let (Some(msh_sep), Some(msh_enc)) = (self.msh_sep, self.msh_enc) else {
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

        // Skip leading field separator
        let sep_len = delims.field_separator.len_utf8();
        let remaining = if let Some(rest) = remaining.strip_prefix(delims.field_separator) {
            self.field_position += sep_len;
            rest
        } else {
            remaining
        };

        if remaining.is_empty() {
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

        // Extract segment ID (first 3 bytes typically, floored to a char boundary)
        let delims = self.delimiters.as_ref()?;
        let seg_end = line.find(delims.field_separator).unwrap_or(line.len());
        let id_end = floor_char_boundary(line, seg_end.min(3));
        let segment_id = &line[..id_end];

        // Validate segment ID (should be 3 uppercase letters typically)
        if segment_id.len() < 2 {
            return Some(Err(Error::parse_at(
                format!("Invalid segment ID: {}", segment_id),
                ErrorLocation::new().line(self.current_line),
            )));
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

        // Skip leading field separator
        let remaining = if let Some(rest) = remaining.strip_prefix(delims.field_separator) {
            self.field_position += delims.field_separator.len_utf8();
            self.current_field_index += 1;
            rest
        } else {
            remaining
        };

        if remaining.is_empty() {
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
            // Last field
            let field_index = self.current_field_index;
            self.state = ParserState::EmitEnd;

            let trimmed = remaining.trim_end();
            if trimmed.is_empty() {
                return self.emit_end();
            }

            Some(Ok(SegmentEvent::Field {
                index: field_index,
                value: trimmed,
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
            SegmentEvent::Field { index: _, value } => {
                // Parse field and add to segment
                let delims = self.delimiters;
                if let (Some(seg), Some(delims)) = (&mut self.current_segment, delims.as_ref()) {
                    let field = Self::parse_field_static(value, delims);
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
    fn parse_field_static(value: &str, delims: &Delimiters) -> Field {
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
                    component.add_subcomponent(SubComponent::new(sub_value));
                }

                repetition.add_component(component);
            }

            field.add_repetition(repetition);
        }

        field
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

    Ok(builder.finish())
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
            SegmentEvent::Field { value, .. } => {
                if let (Some(seg), Some(delims)) = (&mut current_segment, &delimiters) {
                    let field = parse_field_with_delimiters(value, delims);
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
fn parse_field_with_delimiters(value: &str, delims: &Delimiters) -> Field {
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
                component.add_subcomponent(SubComponent::new(sub_value));
            }

            repetition.add_component(component);
        }

        field.add_repetition(repetition);
    }

    field
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
}
