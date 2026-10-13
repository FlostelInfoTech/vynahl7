//! HL7 message parser
//!
//! This crate provides parsing functionality for HL7 v2.x messages.
//!
//! # Parsing Modes
//!
//! The parser supports both strict and lenient parsing modes:
//!
//! - **Strict mode** (default): Enforces HL7 specification compliance
//! - **Lenient mode**: Tolerates common real-world deviations
//!
//! See [`ParserConfig`] for configuration options.

mod config;
mod optimized;
pub mod streaming;

pub use config::{ParseResult, ParseWarning, ParserConfig, WarningCode};
pub use streaming::{
    SegmentEvent, SegmentHandler, StreamingMessageBuilder, StreamingParser, parse_streaming,
    process_with_handler,
};

use chrono::NaiveDateTime;
use vyna_hl7_core::{
    batch::{Batch, BatchHeader, BatchTrailer, File, FileHeader, FileTrailer},
    charset::CharacterSet,
    delimiters::Delimiters,
    encoding::Encoding,
    error::{Error, Result},
    field::{Component, Field, Repetition, SubComponent},
    message::Message,
    segment::Segment,
};

/// Parse a complete HL7 message with default strict configuration
pub fn parse_message(input: &str) -> Result<Message> {
    parse_message_with_config(input, &ParserConfig::strict()).map(|r| r.value)
}

/// Parse a complete HL7 message from raw wire bytes with default strict configuration
///
/// The bytes are decoded using the character set declared in MSH-18 (HL7
/// table 0211), defaulting to UTF-8 when MSH-18 is empty. Use this for
/// inbound data; use [`parse_message`] when the text is already decoded.
pub fn parse_message_bytes(input: &[u8]) -> Result<Message> {
    parse_message_bytes_with_config(input, &ParserConfig::strict()).map(|r| r.value)
}

/// Parse a complete HL7 message from raw wire bytes with custom configuration
///
/// The bytes are decoded using the character set declared in MSH-18 (HL7
/// table 0211), defaulting to UTF-8 when MSH-18 is empty.
pub fn parse_message_bytes_with_config(
    input: &[u8],
    config: &ParserConfig,
) -> Result<ParseResult<Message>> {
    let text = CharacterSet::decode_message(input)?;
    parse_message_with_config(&text, config)
}

/// Parse a complete HL7 message with custom configuration
///
/// This function allows specifying parsing options for handling non-compliant
/// messages. Use [`ParserConfig::lenient()`] for tolerant parsing.
///
/// # Example
///
/// ```rust
/// use vyna_hl7_parser::{parse_message_with_config, ParserConfig};
///
/// // Parse with lenient configuration
/// let config = ParserConfig::lenient();
/// let result = parse_message_with_config(
///     "MSH|^~\\&|App|Fac|||20240315||ADT^A01|123|P|2.5|",
///     &config
/// );
///
/// if let Ok(parsed) = result {
///     println!("Parsed message with {} warnings", parsed.warning_count());
/// }
/// ```
pub fn parse_message_with_config(
    input: &str,
    config: &ParserConfig,
) -> Result<ParseResult<Message>> {
    // Note: ParseWarning and WarningCode are available in config module for future warning support

    let mut warnings = Vec::new();

    // Handle whitespace based on config
    let input = if config.strip_trailing_whitespace {
        input.trim_end()
    } else {
        input
    };

    let input = if config.strip_leading_whitespace {
        input.trim_start()
    } else {
        input
    };

    let input = input.trim_matches(|c| c == '\r' || c == '\n');

    // Extract delimiters from MSH segment
    let delimiters = extract_delimiters_with_config(input, config)?;

    // Split message into segments (by \r or \n)
    let mut segment_strings: Vec<&str> = input.split('\r').flat_map(|s| s.split('\n')).collect();

    // Apply config-based filtering
    segment_strings = segment_strings
        .into_iter()
        .filter_map(|s| {
            let s = if config.strip_trailing_whitespace {
                s.trim_end()
            } else {
                s
            };
            let s = if config.strip_leading_whitespace {
                s.trim_start()
            } else {
                s
            };

            if s.is_empty() {
                if config.skip_blank_lines {
                    None
                } else {
                    Some(s)
                }
            } else {
                Some(s)
            }
        })
        .collect();

    if segment_strings.is_empty() {
        return Err(Error::parse("Empty message"));
    }

    // Check segment limit
    if config.max_segments > 0 && segment_strings.len() > config.max_segments {
        if !config.continue_on_error {
            return Err(Error::parse(format!(
                "Message exceeds maximum segment count: {} > {}",
                segment_strings.len(),
                config.max_segments
            )));
        }
        segment_strings.truncate(config.max_segments);
    }

    let mut message = Message::with_delimiters(delimiters);

    for (idx, seg_str) in segment_strings.iter().enumerate() {
        // Handle trailing delimiters
        let seg_str = if config.allow_trailing_delimiters {
            seg_str.trim_end_matches(delimiters.field_separator)
        } else {
            seg_str
        };

        let segment_result = if idx == 0 {
            parse_msh_segment_with_config(seg_str, &delimiters, config)
        } else {
            parse_segment_with_config(seg_str, &delimiters, config)
        };

        match segment_result {
            Ok((segment, seg_warnings)) => {
                message.add_segment(segment);
                warnings.extend(seg_warnings);
            }
            Err(e) => {
                if config.continue_on_error {
                    warnings.push(ParseWarning {
                        location: idx,
                        message: format!("Skipped segment {idx}: {e}"),
                        code: WarningCode::SkippedSegment,
                    });
                } else {
                    return Err(e);
                }
            }
        }
    }

    Ok(ParseResult {
        value: message,
        warnings,
    })
}

/// Extract delimiters with configuration options
fn extract_delimiters_with_config(input: &str, config: &ParserConfig) -> Result<Delimiters> {
    if !input.starts_with("MSH") {
        return Err(Error::parse("Message must start with MSH segment"));
    }

    if input.len() < 8 {
        // In lenient mode, allow short encoding characters
        if !config.allow_non_standard_encoding_chars {
            return Err(Error::parse("MSH segment too short"));
        }
    }

    let field_sep = match input.as_bytes().get(3) {
        Some(&b) => b as char,
        None => return Err(Error::parse("MSH segment too short")),
    };
    let encoding_chars = if input.len() >= 8 {
        input.get(4..8)
    } else {
        input.get(4..)
    };
    let encoding_chars = match encoding_chars {
        Some(s) => s,
        None => return Err(Error::parse("MSH segment has invalid encoding characters")),
    };

    // In lenient mode, allow non-standard encoding characters
    if encoding_chars.len() < 4 && !config.allow_non_standard_encoding_chars {
        return Err(Error::parse(format!(
            "Invalid encoding characters (expected 4, got {})",
            encoding_chars.len()
        )));
    }

    Delimiters::from_encoding_characters(field_sep, encoding_chars)
}

/// Parse MSH segment with configuration
fn parse_msh_segment_with_config(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
) -> Result<(Segment, Vec<ParseWarning>)> {
    let warnings = Vec::new();

    if !input.starts_with("MSH") {
        return Err(Error::parse("MSH segment must start with 'MSH'"));
    }

    let mut segment = Segment::new("MSH");

    // Add MSH-1 (field separator)
    segment.add_field(Field::from_value(delimiters.field_separator.to_string()));

    // Add MSH-2 (encoding characters)
    segment.add_field(Field::from_value(delimiters.encoding_characters()));

    // Parse the rest of the fields
    let field_start = 9;
    if input.len() <= field_start {
        return Ok((segment, warnings));
    }

    let rest = match input.get(field_start..) {
        Some(rest) => rest,
        None => return Err(Error::parse("MSH segment has invalid encoding characters")),
    };
    let field_strings: Vec<&str> = rest.split(delimiters.field_separator).collect();

    for field_str in field_strings {
        let field = parse_field_with_config(field_str, delimiters, config)?;
        segment.add_field(field);
    }

    Ok((segment, warnings))
}

/// Parse a regular segment with configuration
fn parse_segment_with_config(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
) -> Result<(Segment, Vec<ParseWarning>)> {
    let mut warnings = Vec::new();

    // Handle segment ID length (byte end, safe for non-ASCII input)
    let mut id_len = 0usize;
    let mut id_end = 0usize;
    for c in input.chars().take_while(|c| c.is_ascii_alphanumeric()) {
        id_len += 1;
        id_end += c.len_utf8();
    }

    if id_len == 0 {
        if config.allow_empty_segment_id {
            warnings.push(ParseWarning {
                location: 0,
                message: "Empty segment ID".to_string(),
                code: WarningCode::EmptySegment,
            });
            // Return empty segment
            return Ok((Segment::new(""), warnings));
        } else {
            return Err(Error::parse("Empty segment ID"));
        }
    }

    let segment_id = match input.get(..id_end) {
        Some(id) => id,
        None => return Err(Error::parse("Invalid segment ID")),
    };

    if id_len != 3 && !config.allow_non_standard_segment_ids {
        return Err(Error::parse(format!(
            "Segment ID must be 3 characters, got {} ('{}')",
            id_len, segment_id
        )));
    }

    if id_len != 3 && config.allow_non_standard_segment_ids {
        warnings.push(ParseWarning {
            location: 0,
            message: format!("Non-standard segment ID length: {}", id_len),
            code: WarningCode::NonStandardSegmentId,
        });
    }

    let mut segment = Segment::new(segment_id);

    // Check for field separator
    let after_id = input.get(id_end..).unwrap_or("");
    if after_id.is_empty() {
        return Ok((segment, warnings));
    }

    let next_char = after_id.chars().next();
    let rest = match after_id.strip_prefix(delimiters.field_separator) {
        Some(rest) => rest,
        None => {
            if config.continue_on_error {
                return Ok((segment, warnings));
            }
            return Err(Error::parse(format!(
                "Expected field separator after segment ID, got '{}'",
                next_char.unwrap_or(' ')
            )));
        }
    };
    let field_strings: Vec<&str> = rest.split(delimiters.field_separator).collect();

    for field_str in field_strings {
        let field = parse_field_with_config(field_str, delimiters, config)?;
        segment.add_field(field);
    }

    Ok((segment, warnings))
}

/// Parse a field with configuration
fn parse_field_with_config(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
) -> Result<Field> {
    let mut field = Field::new();

    let repetition_strings: Vec<&str> = if input.is_empty() {
        vec![""]
    } else {
        input.split(delimiters.repetition_separator).collect()
    };

    // Apply max repetitions limit
    let repetition_strings =
        if config.max_repetitions > 0 && repetition_strings.len() > config.max_repetitions {
            &repetition_strings[..config.max_repetitions]
        } else {
            &repetition_strings[..]
        };

    // Pre-allocate repetitions vector
    let mut repetitions = Vec::with_capacity(repetition_strings.len());

    for rep_str in repetition_strings {
        // Apply max field length (floored to a char boundary)
        let rep_str = if config.max_field_length > 0 && rep_str.len() > config.max_field_length {
            truncate_at_char_boundary(rep_str, config.max_field_length)
        } else {
            rep_str
        };

        repetitions.push(parse_repetition(rep_str, delimiters)?);
    }

    field.repetitions = repetitions;
    Ok(field)
}

/// Truncate a string to at most `max_bytes` bytes without splitting a character.
fn truncate_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

/// Parse a regular segment
fn parse_segment(input: &str, delimiters: &Delimiters) -> Result<Segment> {
    if input.len() < 3 {
        return Err(Error::parse("Segment too short"));
    }

    let segment_id = match input.get(0..3) {
        Some(id) => id,
        None => return Err(Error::parse("Invalid segment ID")),
    };
    let mut segment = Segment::new(segment_id);

    let after_id = input.get(3..).unwrap_or("");
    if after_id.is_empty() {
        return Ok(segment);
    }

    // Check for field separator after segment ID
    let rest = match after_id.strip_prefix(delimiters.field_separator) {
        Some(rest) => rest,
        None => {
            return Err(Error::parse(format!(
                "Expected field separator after segment ID, got '{}'",
                after_id.chars().next().unwrap_or(' ')
            )));
        }
    };
    let field_strings: Vec<&str> = rest.split(delimiters.field_separator).collect();

    for field_str in field_strings {
        let field = parse_field(field_str, delimiters)?;
        segment.add_field(field);
    }

    Ok(segment)
}

/// Parse a field (can contain repetitions)
fn parse_field(input: &str, delimiters: &Delimiters) -> Result<Field> {
    parse_field_with_config(input, delimiters, &ParserConfig::strict())
}

/// Parse a repetition (can contain components)
fn parse_repetition(input: &str, delimiters: &Delimiters) -> Result<Repetition> {
    let mut repetition = Repetition::new();

    if input.is_empty() {
        let mut comp = Component::new();
        comp.add_subcomponent(SubComponent::new(""));
        repetition.add_component(comp);
        return Ok(repetition);
    }

    // Pre-allocate based on delimiter count
    let comp_count = input.matches(delimiters.component_separator).count() + 1;
    let mut components = Vec::with_capacity(comp_count);

    for comp_str in input.split(delimiters.component_separator) {
        components.push(parse_component(comp_str, delimiters)?);
    }

    repetition.components = components;
    Ok(repetition)
}

/// Parse a component (can contain subcomponents)
fn parse_component(input: &str, delimiters: &Delimiters) -> Result<Component> {
    let mut component = Component::new();

    if input.is_empty() {
        component.add_subcomponent(SubComponent::new(""));
        return Ok(component);
    }

    // Fast path: no subcomponents (most common case)
    if !input.contains(delimiters.subcomponent_separator) {
        component.add_subcomponent(parse_subcomponent(input, delimiters)?);
        return Ok(component);
    }

    // Pre-allocate based on delimiter count
    let sub_count = input.matches(delimiters.subcomponent_separator).count() + 1;
    let mut subcomponents = Vec::with_capacity(sub_count);

    for sub_str in input.split(delimiters.subcomponent_separator) {
        subcomponents.push(parse_subcomponent(sub_str, delimiters)?);
    }

    component.subcomponents = subcomponents;
    Ok(component)
}

/// Parse a subcomponent (decode escape sequences)
fn parse_subcomponent(input: &str, delimiters: &Delimiters) -> Result<SubComponent> {
    if input.is_empty() {
        return Ok(SubComponent::new(""));
    }

    // Fast path: skip decode when no escape sequences present (~95% of values)
    if !input.contains(delimiters.escape_character) {
        return Ok(SubComponent::new(input));
    }

    let decoded = Encoding::decode(input, delimiters)?;
    Ok(SubComponent::new(decoded))
}

/// Parse a complete HL7 batch message
///
/// A batch message consists of:
/// - BHS (Batch Header Segment)
/// - One or more complete messages (each starting with MSH)
/// - BTS (Batch Trailer Segment)
pub fn parse_batch(input: &str) -> Result<Batch> {
    let input = input.trim();

    if !input.starts_with("BHS") {
        return Err(Error::parse("Batch must start with BHS segment"));
    }

    // Split into segments
    let segment_strings: Vec<&str> = input
        .split('\r')
        .flat_map(|s| s.split('\n'))
        .filter(|s| !s.is_empty())
        .collect();

    if segment_strings.len() < 2 {
        return Err(Error::parse(
            "Batch must have at least BHS and BTS segments",
        ));
    }

    // Extract delimiters from BHS segment (same format as MSH)
    let delimiters = extract_delimiters_from_bhs(segment_strings[0])?;

    // Parse BHS (first segment)
    let header = parse_bhs_segment(segment_strings[0], &delimiters)?;

    // Find BTS segment (last segment)
    let last_idx = segment_strings.len() - 1;
    if !segment_strings[last_idx].starts_with("BTS") {
        return Err(Error::parse("Batch must end with BTS segment"));
    }
    let trailer = parse_bts_segment(segment_strings[last_idx], &delimiters)?;

    // Parse messages between BHS and BTS
    let mut messages = Vec::new();
    let mut current_message_lines = Vec::new();

    for line in &segment_strings[1..last_idx] {
        if line.starts_with("MSH") {
            // Start of a new message - save previous if exists
            if !current_message_lines.is_empty() {
                let msg_text = current_message_lines.join("\r");
                messages.push(parse_message(&msg_text)?);
                current_message_lines.clear();
            }
        }
        current_message_lines.push(*line);
    }

    // Don't forget the last message
    if !current_message_lines.is_empty() {
        let msg_text = current_message_lines.join("\r");
        messages.push(parse_message(&msg_text)?);
    }

    let batch = Batch {
        header,
        messages,
        trailer,
    };

    // Validate the batch
    batch.validate()?;

    Ok(batch)
}

/// Parse a complete HL7 file message
///
/// A file message consists of:
/// - FHS (File Header Segment)
/// - One or more batches (each with BHS...BTS)
/// - FTS (File Trailer Segment)
pub fn parse_file(input: &str) -> Result<File> {
    let input = input.trim();

    if !input.starts_with("FHS") {
        return Err(Error::parse("File must start with FHS segment"));
    }

    // Split into segments
    let segment_strings: Vec<&str> = input
        .split('\r')
        .flat_map(|s| s.split('\n'))
        .filter(|s| !s.is_empty())
        .collect();

    if segment_strings.len() < 2 {
        return Err(Error::parse("File must have at least FHS and FTS segments"));
    }

    // Extract delimiters from FHS segment
    let delimiters = extract_delimiters_from_fhs(segment_strings[0])?;

    // Parse FHS (first segment)
    let header = parse_fhs_segment(segment_strings[0], &delimiters)?;

    // Find FTS segment (last segment)
    let last_idx = segment_strings.len() - 1;
    if !segment_strings[last_idx].starts_with("FTS") {
        return Err(Error::parse("File must end with FTS segment"));
    }
    let trailer = parse_fts_segment(segment_strings[last_idx], &delimiters)?;

    // Parse batches between FHS and FTS
    let mut batches = Vec::new();
    let mut current_batch_lines = Vec::new();

    for line in &segment_strings[1..last_idx] {
        if line.starts_with("BHS") {
            // Start of a new batch - save previous if exists
            if !current_batch_lines.is_empty() {
                let batch_text = current_batch_lines.join("\r");
                batches.push(parse_batch(&batch_text)?);
                current_batch_lines.clear();
            }
        }
        current_batch_lines.push(*line);
    }

    // Don't forget the last batch
    if !current_batch_lines.is_empty() {
        let batch_text = current_batch_lines.join("\r");
        batches.push(parse_batch(&batch_text)?);
    }

    let file = File {
        header,
        batches,
        trailer,
    };

    // Validate the file
    file.validate()?;

    Ok(file)
}

/// Extract delimiters from a segment with MSH-like structure (MSH, BHS, FHS)
///
/// All three segment types use the same encoding: position 3 is the field separator,
/// positions 4-7 are encoding characters (e.g., `^~\&`).
fn extract_delimiters_from_header(expected_prefix: &str, input: &str) -> Result<Delimiters> {
    if !input.starts_with(expected_prefix) {
        return Err(Error::parse(format!(
            "Expected {} segment",
            expected_prefix
        )));
    }

    if input.len() < 8 {
        return Err(Error::parse(format!(
            "{} segment too short",
            expected_prefix
        )));
    }

    let field_sep = input.as_bytes()[3] as char;
    let encoding_chars = match input.get(4..8) {
        Some(chars) => chars,
        None => {
            return Err(Error::parse(format!(
                "{} segment has invalid encoding characters",
                expected_prefix
            )));
        }
    };

    Delimiters::from_encoding_characters(field_sep, encoding_chars)
}

/// Extract delimiters from BHS segment (same format as MSH)
fn extract_delimiters_from_bhs(input: &str) -> Result<Delimiters> {
    extract_delimiters_from_header("BHS", input)
}

/// Extract delimiters from FHS segment (same format as MSH)
fn extract_delimiters_from_fhs(input: &str) -> Result<Delimiters> {
    extract_delimiters_from_header("FHS", input)
}

/// Parse FHS (File Header Segment)
fn parse_fhs_segment(input: &str, delimiters: &Delimiters) -> Result<FileHeader> {
    let segment = parse_segment_like_msh("FHS", input, delimiters)?;

    Ok(FileHeader {
        field_separator: delimiters.field_separator,
        encoding_characters: delimiters.encoding_characters(),
        sending_application: segment.get_field_value(3).map(String::from),
        sending_facility: segment.get_field_value(4).map(String::from),
        receiving_application: segment.get_field_value(5).map(String::from),
        receiving_facility: segment.get_field_value(6).map(String::from),
        creation_datetime: parse_datetime_field(segment.get_field_value(7)),
        security: segment.get_field_value(8).map(String::from),
        file_name_id: segment.get_field_value(9).map(String::from),
        comment: segment.get_field_value(10).map(String::from),
        control_id: segment.get_field_value(11).map(String::from),
        reference_control_id: segment.get_field_value(12).map(String::from),
        sending_network_address: segment.get_field_value(13).map(String::from),
        receiving_network_address: segment.get_field_value(14).map(String::from),
    })
}

/// Parse BHS (Batch Header Segment)
fn parse_bhs_segment(input: &str, delimiters: &Delimiters) -> Result<BatchHeader> {
    let segment = parse_segment_like_msh("BHS", input, delimiters)?;

    Ok(BatchHeader {
        field_separator: delimiters.field_separator,
        encoding_characters: delimiters.encoding_characters(),
        sending_application: segment.get_field_value(3).map(String::from),
        sending_facility: segment.get_field_value(4).map(String::from),
        receiving_application: segment.get_field_value(5).map(String::from),
        receiving_facility: segment.get_field_value(6).map(String::from),
        creation_datetime: parse_datetime_field(segment.get_field_value(7)),
        security: segment.get_field_value(8).map(String::from),
        batch_name_id_type: segment.get_field_value(9).map(String::from),
        comment: segment.get_field_value(10).map(String::from),
        control_id: segment.get_field_value(11).map(String::from),
        reference_control_id: segment.get_field_value(12).map(String::from),
        sending_network_address: segment.get_field_value(13).map(String::from),
        receiving_network_address: segment.get_field_value(14).map(String::from),
    })
}

/// Parse FTS (File Trailer Segment)
fn parse_fts_segment(input: &str, delimiters: &Delimiters) -> Result<FileTrailer> {
    let segment = parse_segment(input, delimiters)?;

    if segment.id != "FTS" {
        return Err(Error::parse("Expected FTS segment"));
    }

    let batch_count = segment
        .get_field_value(1)
        .and_then(|s| s.parse::<usize>().ok());

    let comment = segment.get_field_value(2).map(String::from);

    Ok(FileTrailer {
        batch_count,
        comment,
    })
}

/// Parse BTS (Batch Trailer Segment)
fn parse_bts_segment(input: &str, delimiters: &Delimiters) -> Result<BatchTrailer> {
    let segment = parse_segment(input, delimiters)?;

    if segment.id != "BTS" {
        return Err(Error::parse("Expected BTS segment"));
    }

    let message_count = segment
        .get_field_value(1)
        .and_then(|s| s.parse::<usize>().ok());

    let comment = segment.get_field_value(2).map(String::from);

    let totals = if let Some(totals_str) = segment.get_field_value(3) {
        totals_str
            .split(delimiters.repetition_separator)
            .filter_map(|s| s.parse::<f64>().ok())
            .collect()
    } else {
        Vec::new()
    };

    Ok(BatchTrailer {
        message_count,
        comment,
        totals,
    })
}

/// Helper to parse segments with MSH-like structure (FHS/BHS)
///
/// These segments have the same special structure as MSH where field 1 is the field separator
/// and field 2 is the encoding characters
fn parse_segment_like_msh(
    segment_type: &str,
    input: &str,
    delimiters: &Delimiters,
) -> Result<Segment> {
    if !input.starts_with(segment_type) {
        return Err(Error::parse(format!(
            "{} segment must start with '{}'",
            segment_type, segment_type
        )));
    }

    let mut segment = Segment::new(segment_type);

    // Add field 1 (field separator)
    segment.add_field(Field::from_value(delimiters.field_separator.to_string()));

    // Add field 2 (encoding characters)
    segment.add_field(Field::from_value(delimiters.encoding_characters()));

    // Parse the rest of the fields starting after "XXX|^~\&"
    let field_start = 9; // "XXX|^~\&|".len()
    if input.len() <= field_start {
        return Ok(segment);
    }

    let rest = match input.get(field_start..) {
        Some(rest) => rest,
        None => {
            return Err(Error::parse(format!(
                "{} segment has invalid encoding characters",
                segment_type
            )));
        }
    };
    let field_strings: Vec<&str> = rest.split(delimiters.field_separator).collect();

    // Add remaining fields
    for field_str in field_strings {
        let field = parse_field(field_str, delimiters)?;
        segment.add_field(field);
    }

    Ok(segment)
}

/// Parse an HL7 datetime field into NaiveDateTime
///
/// HL7 datetime formats:
/// - TS (v2.3-v2.5): YYYYMMDDHHMMSS[.SSSS][+/-ZZZZ]
/// - DTM (v2.6+): YYYY[MM[DD[HH[MM[SS[.S[S[S[S]]]]]]]]][+/-ZZZZ]
fn parse_datetime_field(value: Option<&str>) -> Option<NaiveDateTime> {
    value.and_then(|s| {
        if s.is_empty() {
            return None;
        }

        // Remove timezone offset if present (we use NaiveDateTime)
        let datetime_str = s
            .split('+')
            .next()
            .and_then(|s| s.split('-').next())
            .unwrap_or(s);

        // Try various datetime formats
        // Full: YYYYMMDDHHMMSS
        if datetime_str.len() >= 14
            && let Some(prefix) = datetime_str.get(..14)
            && let Ok(dt) = NaiveDateTime::parse_from_str(prefix, "%Y%m%d%H%M%S")
        {
            return Some(dt);
        }

        // Date only: YYYYMMDD
        if datetime_str.len() >= 8
            && let Some(prefix) = datetime_str.get(..8)
            && let Ok(date) = chrono::NaiveDate::parse_from_str(prefix, "%Y%m%d")
        {
            return date.and_hms_opt(0, 0, 0);
        }

        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_extract_delimiters() {
        let msh = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5";
        let delims = extract_delimiters_with_config(msh, &ParserConfig::strict()).unwrap();

        assert_eq!(delims.field_separator, '|');
        assert_eq!(delims.component_separator, '^');
        assert_eq!(delims.repetition_separator, '~');
        assert_eq!(delims.escape_character, '\\');
        assert_eq!(delims.subcomponent_separator, '&');
    }

    #[test]
    fn test_parse_msh_segment() {
        let msh = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac";
        let delims = Delimiters::default();
        let config = ParserConfig::strict();
        let (segment, _warnings) = parse_msh_segment_with_config(msh, &delims, &config).unwrap();

        assert_eq!(segment.id, "MSH");
        assert_eq!(segment.get_field_value(3), Some("SendApp"));
        assert_eq!(segment.get_field_value(4), Some("SendFac"));
    }

    #[test]
    fn test_parse_segment() {
        let delims = Delimiters::default();
        let pid = "PID|1|12345|67890^^^MRN|DOE^JOHN^A|";
        let segment = parse_segment(pid, &delims).unwrap();

        assert_eq!(segment.id, "PID");
        assert_eq!(segment.get_field_value(1), Some("1"));
        assert_eq!(segment.get_field_value(2), Some("12345"));
    }

    #[test]
    fn test_parse_field_with_components() {
        let delims = Delimiters::default();
        let field_str = "DOE^JOHN^A";
        let field = parse_field(field_str, &delims).unwrap();

        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.get_component(0).unwrap().value(), Some("DOE"));
        assert_eq!(rep.get_component(1).unwrap().value(), Some("JOHN"));
        assert_eq!(rep.get_component(2).unwrap().value(), Some("A"));
    }

    #[test]
    fn test_parse_field_with_subcomponents() {
        let delims = Delimiters::default();
        let field_str = "ID1&AssignAuth^ID2";
        let field = parse_field(field_str, &delims).unwrap();

        let rep = field.get_repetition(0).unwrap();
        let comp0 = rep.get_component(0).unwrap();
        assert_eq!(comp0.get_subcomponent(0).unwrap().as_str(), "ID1");
        assert_eq!(comp0.get_subcomponent(1).unwrap().as_str(), "AssignAuth");
    }

    #[test]
    fn test_parse_field_with_repetitions() {
        let delims = Delimiters::default();
        let field_str = "Value1~Value2~Value3";
        let field = parse_field(field_str, &delims).unwrap();

        assert_eq!(field.repetitions.len(), 3);
        assert_eq!(field.get_repetition(0).unwrap().value(), Some("Value1"));
        assert_eq!(field.get_repetition(1).unwrap().value(), Some("Value2"));
        assert_eq!(field.get_repetition(2).unwrap().value(), Some("Value3"));
    }

    #[test]
    fn test_parse_complete_message() {
        let msg = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|12345|P|2.5\r\
                   PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M\r\
                   PV1|1|I|Ward^Room^Bed";

        let parsed = parse_message(msg).unwrap();

        assert_eq!(parsed.segments.len(), 3);
        assert_eq!(parsed.get_msh().unwrap().id, "MSH");
        assert_eq!(parsed.get_sending_application(), Some("SendApp"));

        let pid = &parsed.segments[1];
        assert_eq!(pid.id, "PID");
        assert_eq!(pid.get_field_value(2), Some("12345"));
    }

    #[test]
    fn test_parse_with_escape_sequences() {
        let delims = Delimiters::default();
        let field_str = "Test\\F\\Value";
        let field = parse_field(field_str, &delims).unwrap();

        assert_eq!(field.value(), Some("Test|Value"));
    }

    #[test]
    fn test_parse_empty_fields() {
        let delims = Delimiters::default();
        let segment = "PID|1||3|4|5";
        let parsed = parse_segment(segment, &delims).unwrap();

        assert_eq!(parsed.get_field_value(1), Some("1"));
        assert_eq!(parsed.get_field_value(2), Some(""));
        assert_eq!(parsed.get_field_value(3), Some("3"));
    }

    #[test]
    fn test_parse_bhs_segment() {
        let bhs = "BHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||BATCH001||B12345";
        let delims = Delimiters::default();
        let header = parse_bhs_segment(bhs, &delims).unwrap();

        assert_eq!(header.sending_application, Some("SendApp".to_string()));
        assert_eq!(header.sending_facility, Some("SendFac".to_string()));
        assert_eq!(header.receiving_application, Some("RecApp".to_string()));
        assert_eq!(header.receiving_facility, Some("RecFac".to_string()));
        assert_eq!(header.batch_name_id_type, Some("BATCH001".to_string()));
        assert_eq!(header.control_id, Some("B12345".to_string()));
        assert!(header.creation_datetime.is_some());
    }

    #[test]
    fn test_parse_fhs_segment() {
        let fhs = "FHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||FILE001||F12345";
        let delims = Delimiters::default();
        let header = parse_fhs_segment(fhs, &delims).unwrap();

        assert_eq!(header.sending_application, Some("SendApp".to_string()));
        assert_eq!(header.sending_facility, Some("SendFac".to_string()));
        assert_eq!(header.file_name_id, Some("FILE001".to_string()));
        assert_eq!(header.control_id, Some("F12345".to_string()));
        assert!(header.creation_datetime.is_some());
    }

    #[test]
    fn test_parse_bts_segment() {
        let bts = "BTS|2|Batch complete";
        let delims = Delimiters::default();
        let trailer = parse_bts_segment(bts, &delims).unwrap();

        assert_eq!(trailer.message_count, Some(2));
        assert_eq!(trailer.comment, Some("Batch complete".to_string()));
    }

    #[test]
    fn test_parse_fts_segment() {
        let fts = "FTS|1|File complete";
        let delims = Delimiters::default();
        let trailer = parse_fts_segment(fts, &delims).unwrap();

        assert_eq!(trailer.batch_count, Some(1));
        assert_eq!(trailer.comment, Some("File complete".to_string()));
    }

    #[test]
    fn test_parse_batch() {
        let batch_text = "\
BHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||BATCH001||B12345
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG001|P|2.5
PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143100||ADT^A01|MSG002|P|2.5
PID|1|54321|09876^^^MRN|SMITH^JANE^B||19900202|F
BTS|2";

        let batch = parse_batch(batch_text).unwrap();

        assert_eq!(
            batch.header.sending_application,
            Some("SendApp".to_string())
        );
        assert_eq!(batch.header.control_id, Some("B12345".to_string()));
        assert_eq!(batch.messages.len(), 2);
        assert_eq!(batch.trailer.message_count, Some(2));

        // Check first message
        assert_eq!(batch.messages[0].get_sending_application(), Some("SendApp"));
        assert_eq!(batch.messages[0].segments.len(), 2); // MSH + PID

        // Check second message
        assert_eq!(batch.messages[1].segments.len(), 2); // MSH + PID
    }

    #[test]
    fn test_parse_batch_with_validation_error() {
        // BTS says 5 messages but only 2 are present
        let batch_text = "\
BHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||BATCH001||B12345
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG001|P|2.5
PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143100||ADT^A01|MSG002|P|2.5
PID|1|54321|09876^^^MRN|SMITH^JANE^B||19900202|F
BTS|5";

        let result = parse_batch(batch_text);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("message count mismatch")
        );
    }

    #[test]
    fn test_parse_file() {
        let file_text = "\
FHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||FILE001||F12345
BHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||BATCH001||B12345
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG001|P|2.5
PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M
BTS|1
BHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143100||BATCH002||B12346
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143100||ADT^A01|MSG002|P|2.5
PID|1|54321|09876^^^MRN|SMITH^JANE^B||19900202|F
BTS|1
FTS|2";

        let file = parse_file(file_text).unwrap();

        assert_eq!(file.header.sending_application, Some("SendApp".to_string()));
        assert_eq!(file.header.control_id, Some("F12345".to_string()));
        assert_eq!(file.batches.len(), 2);
        assert_eq!(file.trailer.batch_count, Some(2));

        // Check first batch
        assert_eq!(
            file.batches[0].header.control_id,
            Some("B12345".to_string())
        );
        assert_eq!(file.batches[0].messages.len(), 1);

        // Check second batch
        assert_eq!(
            file.batches[1].header.control_id,
            Some("B12346".to_string())
        );
        assert_eq!(file.batches[1].messages.len(), 1);
    }

    #[test]
    fn test_parse_file_with_validation_error() {
        // FTS says 5 batches but only 2 are present
        let file_text = "\
FHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||FILE001||F12345
BHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||BATCH001||B12345
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|MSG001|P|2.5
PID|1|12345|67890^^^MRN|DOE^JOHN^A||19800101|M
BTS|1
BHS|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143100||BATCH002||B12346
MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315143100||ADT^A01|MSG002|P|2.5
PID|1|54321|09876^^^MRN|SMITH^JANE^B||19900202|F
BTS|1
FTS|5";

        let result = parse_file(file_text);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("batch count mismatch")
        );
    }

    #[test]
    fn test_parse_datetime_field() {
        // Full timestamp
        let dt1 = parse_datetime_field(Some("20240315143000"));
        assert!(dt1.is_some());
        let dt1 = dt1.unwrap();
        assert_eq!(dt1.year(), 2024);
        assert_eq!(dt1.month(), 3);
        assert_eq!(dt1.day(), 15);
        assert_eq!(dt1.hour(), 14);
        assert_eq!(dt1.minute(), 30);
        assert_eq!(dt1.second(), 0);

        // Date only
        let dt2 = parse_datetime_field(Some("20240315"));
        assert!(dt2.is_some());
        let dt2 = dt2.unwrap();
        assert_eq!(dt2.year(), 2024);
        assert_eq!(dt2.month(), 3);
        assert_eq!(dt2.day(), 15);

        // Empty
        let dt3 = parse_datetime_field(Some(""));
        assert!(dt3.is_none());

        // None
        let dt4 = parse_datetime_field(None);
        assert!(dt4.is_none());
    }

    #[test]
    fn test_parse_batch_single_message() {
        let batch_text = "\
BHS|^~\\&|APP|FAC|||20240315||BATCH||B001
MSH|^~\\&|APP|FAC|SYS|HOSP|20240315||ADT^A01|MSG001|P|2.5
PID|1|12345
BTS|1";

        let batch = parse_batch(batch_text).unwrap();
        assert_eq!(batch.messages.len(), 1);
        assert_eq!(batch.trailer.message_count, Some(1));
    }

    #[test]
    fn test_extract_delimiters_from_bhs() {
        let bhs = "BHS|^~\\&|SendApp|SendFac";
        let delims = extract_delimiters_from_bhs(bhs).unwrap();

        assert_eq!(delims.field_separator, '|');
        assert_eq!(delims.component_separator, '^');
        assert_eq!(delims.repetition_separator, '~');
        assert_eq!(delims.escape_character, '\\');
        assert_eq!(delims.subcomponent_separator, '&');
    }

    #[test]
    fn test_extract_delimiters_from_fhs() {
        let fhs = "FHS|^~\\&|SendApp|SendFac";
        let delims = extract_delimiters_from_fhs(fhs).unwrap();

        assert_eq!(delims.field_separator, '|');
        assert_eq!(delims.component_separator, '^');
        assert_eq!(delims.repetition_separator, '~');
        assert_eq!(delims.escape_character, '\\');
        assert_eq!(delims.subcomponent_separator, '&');
    }

    #[test]
    fn test_parse_message_bytes_decodes_declared_charset() {
        let wire: &[u8] = b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1||12345^^^FAC^MR||M\xfcller^J\xfcrg||19800101|M\r";
        let message = parse_message_bytes(wire).unwrap();

        let pid = message.get_segment_by_id("PID").unwrap();
        let name = pid.get_field(5).unwrap().get_repetition(0).unwrap();
        assert_eq!(name.get_component(0).unwrap().value(), Some("Müller"));
        assert_eq!(name.get_component(1).unwrap().value(), Some("Jürg"));
        assert_eq!(
            message.get_character_set().unwrap(),
            CharacterSet::Iso8859(1)
        );
    }

    #[test]
    fn test_parse_message_bytes_decodes_declared_utf8() {
        let text = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||UNICODE UTF-8\rPID|1||12345^^^FAC^MR||Müller^Jörg||19800101|M\r";
        let message = parse_message_bytes(text.as_bytes()).unwrap();

        let pid = message.get_segment_by_id("PID").unwrap();
        let name = pid.get_field(5).unwrap().get_repetition(0).unwrap();
        assert_eq!(name.get_component(0).unwrap().value(), Some("Müller"));
        assert_eq!(name.get_component(1).unwrap().value(), Some("Jörg"));
    }

    #[test]
    fn test_parse_message_bytes_defaults_to_utf8_without_declaration() {
        let text = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1\rPID|1||12345^^^FAC^MR||Müller||19800101|M\r";
        let message = parse_message_bytes(text.as_bytes()).unwrap();

        let pid = message.get_segment_by_id("PID").unwrap();
        assert_eq!(pid.get_field_value(5), Some("Müller"));
    }

    #[test]
    fn test_parse_message_bytes_rejects_mismatched_charset() {
        // Latin-1 bytes declared as ASCII must fail, not mojibake.
        let wire: &[u8] = b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||ASCII\rPID|1||12345^^^FAC^MR||M\xfcller||19800101|M\r";
        let error = parse_message_bytes(wire).unwrap_err();
        assert!(error.to_string().contains("ASCII"));

        // Invalid UTF-8 without a declaration must fail with guidance.
        let wire: &[u8] = b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1\rPID|1||12345^^^FAC^MR||M\xfcller||19800101|M\r";
        let error = parse_message_bytes(wire).unwrap_err();
        assert!(error.to_string().contains("MSH-18"));
    }

    #[test]
    fn test_parse_message_bytes_round_trips_outbound_encoding() {
        let text = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1||12345^^^FAC^MR||Müller^Jörg||19800101|M\r";
        let message = parse_message(text).unwrap();

        let bytes = message.encode_bytes().unwrap();
        assert!(bytes.contains(&0xFC));
        let round_tripped = parse_message_bytes(&bytes).unwrap();
        assert_eq!(round_tripped, message);
    }

    #[test]
    fn test_parse_message_bytes_supports_multiple_hl7_versions() {
        for version in ["2.3", "2.5.1", "2.8.2"] {
            let mut wire = format!(
                "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|{version}||||||8859/1\rPID|1||12345^^^FAC^MR||M"
            )
            .into_bytes();
            wire.push(0xFC);
            wire.extend_from_slice(b"ller||19800101|M\r");

            let message = parse_message_bytes(&wire).unwrap();
            assert_eq!(message.get_version().unwrap().as_str(), version);
            assert_eq!(
                message.get_segment_by_id("PID").unwrap().get_field_value(5),
                Some("Müller")
            );
        }
    }

    #[test]
    fn test_encode_bytes_rejects_content_outside_declared_set() {
        let text = "MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||ASCII\rPID|1||12345^^^FAC^MR||Müller||19800101|M\r";
        let message = parse_message(text).unwrap();

        let error = message.encode_bytes().unwrap_err();
        assert!(error.to_string().contains('ü'));
    }

    #[test]
    fn test_lenient_short_input_returns_error_without_panic() {
        let lenient = ParserConfig::lenient();
        for input in ["", "M", "MS", "MSH", "MSH|", "MSH|^~"] {
            let result = parse_message_with_config(input, &lenient);
            assert!(result.is_err(), "input {input:?} should fail, not panic");
        }
    }

    #[test]
    fn test_strict_short_input_returns_error_without_panic() {
        let strict = ParserConfig::strict();
        for input in ["", "M", "MS", "MSH", "MSH|", "MSH|^~"] {
            let result = parse_message_with_config(input, &strict);
            assert!(result.is_err(), "input {input:?} should fail, not panic");
        }
    }

    #[test]
    fn test_non_ascii_delimiter_area_returns_error_without_panic() {
        let strict = ParserConfig::strict();
        let lenient = ParserConfig::lenient();
        for input in [
            "MSHé^~\\&|APP",
            "MSH|é~\\&|APP",
            "MSH|^é\\&|APP",
            "MSH|^~é&|APP",
            "MSH|^~\\é|APP",
        ] {
            assert!(
                parse_message_with_config(input, &strict).is_err(),
                "strict {input:?} should fail, not panic"
            );
            // Lenient mode must also never panic; result may vary.
            let _ = parse_message_with_config(input, &lenient);
        }
    }

    #[test]
    fn test_non_ascii_segment_id_returns_error_without_panic() {
        let delims = Delimiters::default();
        for segment in ["éXX|1|2", "PéD|1|2", "ab€|1|2"] {
            let result = parse_segment(segment, &delims);
            assert!(result.is_err(), "segment {segment:?} should fail");
        }
        // Boundary-valid but non-ASCII IDs must at least not panic.
        let _ = parse_segment("€|1", &delims);
        // Multibyte field separator must not panic the configured path.
        let multi = Delimiters {
            field_separator: 'é',
            ..Delimiters::default()
        };
        let (segment, _) =
            parse_segment_with_config("PIDé12é34", &multi, &ParserConfig::strict()).unwrap();
        assert_eq!(segment.get_field_value(1), Some("12"));
        assert_eq!(segment.get_field_value(2), Some("34"));
        // Non-ASCII via the configured path must not panic either.
        let strict = ParserConfig::strict();
        let lenient = ParserConfig::lenient();
        let msg = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPéD|1|2";
        assert!(parse_message_with_config(msg, &strict).is_err());
        let _ = parse_message_with_config(msg, &lenient);
    }

    #[test]
    fn test_max_field_length_with_multibyte_value_does_not_panic() {
        let config = ParserConfig::strict().max_field_length(5);
        let delims = Delimiters::default();
        // Byte 5 lands inside a multibyte character.
        let field = parse_field_with_config("abcdéfgh", &delims, &config).unwrap();
        assert_eq!(field.value(), Some("abcd"));
    }

    #[test]
    fn test_non_ascii_header_delimiters_return_error_without_panic() {
        assert!(extract_delimiters_from_bhs("BHSé^~\\&|A").is_err());
        assert!(extract_delimiters_from_bhs("BHS|^é\\&|A").is_err());
        assert!(extract_delimiters_from_fhs("FHSé^~\\&|A").is_err());
        assert!(extract_delimiters_from_fhs("FHS|^é\\&|A").is_err());
        assert!(extract_delimiters_from_bhs("BHS").is_err());
        assert!(extract_delimiters_from_fhs("FHS").is_err());
    }

    #[test]
    fn test_non_ascii_datetime_field_returns_none_without_panic() {
        assert_eq!(parse_datetime_field(Some("2024010é123456")), None);
        assert_eq!(parse_datetime_field(Some("20240é15")), None);
        assert_eq!(parse_datetime_field(Some("éééééééééééééééé")), None);
    }

    #[test]
    fn test_non_ascii_msh_body_does_not_panic() {
        let strict = ParserConfig::strict();
        let lenient = ParserConfig::lenient();
        // Multibyte content where MSH field parsing slices the header.
        for msg in ["MSH|^~\\é|A|B", "MSH|^~\\&ééééé", "MSH|^~\\&|é"] {
            let _ = parse_message_with_config(msg, &strict);
            let _ = parse_message_with_config(msg, &lenient);
        }
    }

    #[test]
    fn test_continue_on_error_surfaces_segment_errors() {
        let config = ParserConfig::strict().continue_on_error(true);
        let msg = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rAB|1|2\rPID|1||7\r";
        let result = parse_message_with_config(msg, &config).unwrap();

        // Parsing continues past the bad segment ...
        let ids: Vec<&str> = result
            .value
            .segments
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(ids, vec!["MSH", "PID"]);

        // ... but the skipped segment's error must be reported, not dropped.
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].location, 1);
        assert_eq!(result.warnings[0].code, WarningCode::SkippedSegment);
        assert!(result.warnings[0].message.contains("3 characters"));
    }
}
