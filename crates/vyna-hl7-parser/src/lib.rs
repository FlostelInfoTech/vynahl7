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

use config::is_known_segment_id;
pub use config::{ParseResult, ParseWarning, ParserConfig, WarningCode};
pub use streaming::{
    SegmentEvent, SegmentHandler, StreamingMessageBuilder, StreamingParser, parse_streaming,
    parse_streaming_with_config, process_with_handler, process_with_handler_with_config,
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

    // Skip leading blank lines before delimiter extraction so a
    // whitespace-only first line does not fail when blank lines are
    // skipped anyway (mirrors StreamingParser::parse_initial).
    let input = if config.skip_blank_lines {
        skip_leading_blank_lines(input)
    } else {
        input
    };

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
        warnings.push(ParseWarning {
            location: config.max_segments,
            message: format!(
                "Message truncated from {} to {} segments",
                segment_strings.len(),
                config.max_segments
            ),
            code: WarningCode::SegmentsTruncated,
        });
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
        } else if is_msh_segment(seg_str) {
            // Any MSH uses the MSH-1/2 path so fields stay aligned.
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

/// Skip leading blank (whitespace-only) lines.
///
/// Returns the input unchanged when every line is blank so the caller
/// still reports the real problem instead of an empty slice.
fn skip_leading_blank_lines(input: &str) -> &str {
    let mut offset = 0;
    for line in input.split(['\r', '\n']) {
        if !line.trim().is_empty() {
            return &input[offset..];
        }
        offset += line.len() + 1;
    }
    input
}

/// Extract delimiters with configuration options
///
/// Delimiters are ASCII-only: the byte-oriented header offsets and the
/// MLLP framing around them cannot address multibyte separators. Both
/// parse paths reject non-ASCII MSH-1/MSH-2 alike.
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

    if !field_sep.is_ascii() || !encoding_chars.bytes().all(|b| b.is_ascii()) {
        return Err(Error::parse("MSH segment has invalid encoding characters"));
    }

    // In lenient mode, allow non-standard encoding characters
    if encoding_chars.len() < 4 && !config.allow_non_standard_encoding_chars {
        return Err(Error::parse(format!(
            "Invalid encoding characters (expected 4, got {})",
            encoding_chars.len()
        )));
    }

    // Lenient mode tolerates short MSH-2 runs: missing trailing
    // positions fall back to the standard defaults. A wholly absent
    // MSH-2 still errors.
    if config.allow_non_standard_encoding_chars
        && !encoding_chars.is_empty()
        && encoding_chars.chars().count() < 4
    {
        let defaults: Vec<char> = Delimiters::default()
            .encoding_characters()
            .chars()
            .collect();
        let mut padded: Vec<char> = encoding_chars.chars().collect();
        while padded.len() < 4 {
            padded.push(defaults[padded.len()]);
        }
        let padded: String = padded.into_iter().collect();
        return Delimiters::from_encoding_characters(field_sep, &padded);
    }

    Delimiters::from_encoding_characters(field_sep, encoding_chars)
}

/// True when a segment line carries the MSH id (exactly "MSH").
fn is_msh_segment(seg_str: &str) -> bool {
    if !seg_str.starts_with("MSH") {
        return false;
    }
    if seg_str.len() == 3 {
        return true;
    }
    // "MSH" is 3 ASCII bytes, so byte 3 is a char boundary.
    !seg_str[3..]
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
}

/// Cap an echoed id at 32 chars plus length; short ids stay verbatim.
/// Shared with the streaming path so both truncate alike.
pub(crate) fn truncate_id(value: &str) -> String {
    const MAX: usize = 32;
    let count = value.chars().count();
    if count <= MAX {
        value.to_string()
    } else {
        let head: String = value.chars().take(MAX).collect();
        format!("{head}... ({} chars total)", count)
    }
}

/// Parse MSH segment with configuration
fn parse_msh_segment_with_config(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
) -> Result<(Segment, Vec<ParseWarning>)> {
    let mut warnings = Vec::new();

    if !input.starts_with("MSH") {
        return Err(Error::parse("MSH segment must start with 'MSH'"));
    }

    let mut segment = Segment::new("MSH");

    // Add MSH-1 (field separator)
    segment.add_field(Field::from_value(delimiters.field_separator.to_string()));

    // Add MSH-2 (encoding characters)
    segment.add_field(Field::from_value(delimiters.encoding_characters()));

    // Parse the rest of the fields. Only the bare header (no bytes
    // past the encoding characters) has no content fields;
    // 'MSH|^~\&|' falls through so the empty remainder yields one
    // explicit empty MSH-3. The byte after the encoding characters
    // must be the field separator: slicing past anything else would
    // silently drop a byte (e.g. 'MSH|^~\&XAPP' lost the 'X').
    let header_end =
        3 + delimiters.field_separator.len_utf8() + delimiters.encoding_characters().len();
    if input.len() < header_end {
        return Ok((segment, warnings));
    }
    let after_header = match input.get(header_end..) {
        Some(after_header) => after_header,
        None => return Err(Error::parse("MSH segment has invalid encoding characters")),
    };
    if after_header.is_empty() {
        return Ok((segment, warnings));
    }
    let rest = match after_header.strip_prefix(delimiters.field_separator) {
        Some(rest) => rest,
        None => {
            return Err(Error::parse(
                "expected field separator after encoding characters",
            ));
        }
    };
    let field_strings: Vec<&str> = rest.split(delimiters.field_separator).collect();

    for field_str in field_strings {
        let field = parse_field_with_config(field_str, delimiters, config, &mut warnings)?;
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
            id_len,
            truncate_id(segment_id)
        )));
    }

    if id_len != 3 && config.allow_non_standard_segment_ids {
        warnings.push(ParseWarning {
            location: 0,
            message: format!("Non-standard segment ID length: {}", id_len),
            code: WarningCode::NonStandardSegmentId,
        });
    }

    if config.validate_segment_ids && !is_known_segment_id(segment_id) {
        warnings.push(ParseWarning {
            location: 0,
            message: format!("Unknown segment ID: {}", truncate_id(segment_id)),
            code: WarningCode::UnknownSegmentId,
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
                warnings.push(ParseWarning {
                    location: 0,
                    message: format!(
                        "Skipped segment content: expected field separator \
                         after segment ID, got '{}'",
                        next_char.unwrap_or(' ')
                    ),
                    code: WarningCode::SkippedSegment,
                });
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
        let field = parse_field_with_config(field_str, delimiters, config, &mut warnings)?;
        segment.add_field(field);
    }

    Ok((segment, warnings))
}

/// Parse a field with configuration
fn parse_field_with_config(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
    warnings: &mut Vec<ParseWarning>,
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
            warnings.push(ParseWarning {
                location: 0,
                message: format!(
                    "Field truncated from {} to {} repetitions",
                    repetition_strings.len(),
                    config.max_repetitions
                ),
                code: WarningCode::RepetitionsTruncated,
            });
            &repetition_strings[..config.max_repetitions]
        } else {
            &repetition_strings[..]
        };

    // Pre-allocate repetitions vector
    let mut repetitions = Vec::with_capacity(repetition_strings.len());

    for rep_str in repetition_strings {
        // Apply max field length (floored to a char boundary)
        let rep_str = if config.max_field_length > 0 && rep_str.len() > config.max_field_length {
            warnings.push(ParseWarning {
                location: 0,
                message: format!(
                    "Field truncated from {} to {} bytes",
                    rep_str.len(),
                    config.max_field_length
                ),
                code: WarningCode::FieldTruncated,
            });
            truncate_at_char_boundary(rep_str, config.max_field_length)
        } else {
            rep_str
        };

        repetitions.push(parse_repetition(rep_str, delimiters, config, warnings)?);
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
    parse_field_with_config(input, delimiters, &ParserConfig::strict(), &mut Vec::new())
}

/// Parse a repetition (can contain components)
fn parse_repetition(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
    warnings: &mut Vec<ParseWarning>,
) -> Result<Repetition> {
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
        components.push(parse_component(comp_str, delimiters, config, warnings)?);
    }

    repetition.components = components;
    Ok(repetition)
}

/// Parse a component (can contain subcomponents)
fn parse_component(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
    warnings: &mut Vec<ParseWarning>,
) -> Result<Component> {
    let mut component = Component::new();

    if input.is_empty() {
        component.add_subcomponent(SubComponent::new(""));
        return Ok(component);
    }

    // Fast path: no subcomponents (most common case)
    if !input.contains(delimiters.subcomponent_separator) {
        component.add_subcomponent(parse_subcomponent(input, delimiters, config, warnings)?);
        return Ok(component);
    }

    // Pre-allocate based on delimiter count
    let sub_count = input.matches(delimiters.subcomponent_separator).count() + 1;
    let mut subcomponents = Vec::with_capacity(sub_count);

    for sub_str in input.split(delimiters.subcomponent_separator) {
        subcomponents.push(parse_subcomponent(sub_str, delimiters, config, warnings)?);
    }

    component.subcomponents = subcomponents;
    Ok(component)
}

/// Parse a subcomponent (decode escape sequences)
fn parse_subcomponent(
    input: &str,
    delimiters: &Delimiters,
    config: &ParserConfig,
    warnings: &mut Vec<ParseWarning>,
) -> Result<SubComponent> {
    if input.is_empty() {
        return Ok(SubComponent::new(""));
    }

    // Fast path: skip decode when no escape sequences present (~95% of values)
    if !input.contains(delimiters.escape_character) {
        return Ok(SubComponent::new(input));
    }

    match Encoding::decode(input, delimiters) {
        Ok(decoded) => Ok(SubComponent::new(decoded)),
        Err(e) if config.preserve_invalid_escapes => {
            warnings.push(ParseWarning {
                location: 0,
                message: format!("Invalid escape sequence preserved as literal text: {e}"),
                code: WarningCode::InvalidEscape,
            });
            Ok(SubComponent::new(input))
        }
        Err(e) => Err(e),
    }
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
        creation_datetime: parse_datetime_field("FHS-7", segment.get_field_value(7))?,
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
        creation_datetime: parse_datetime_field("BHS-7", segment.get_field_value(7))?,
        security: segment.get_field_value(8).map(String::from),
        batch_name_id_type: segment.get_field_value(9).map(String::from),
        comment: segment.get_field_value(10).map(String::from),
        control_id: segment.get_field_value(11).map(String::from),
        reference_control_id: segment.get_field_value(12).map(String::from),
        sending_network_address: segment.get_field_value(13).map(String::from),
        receiving_network_address: segment.get_field_value(14).map(String::from),
    })
}

/// Parse an NM-typed trailer count (BTS-1, FTS-1)
///
/// Absent or blank counts mean "no count given" (`None`, so the trailer
/// check is skipped); a present but non-numeric count is a parse error,
/// never a silent skip.
fn parse_trailer_count(label: &str, value: Option<&str>) -> Result<Option<usize>> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(text) => text.parse::<usize>().map(Some).map_err(|_| {
            Error::parse(format!(
                "Invalid {label}: '{}' is not a number",
                truncate_id(text)
            ))
        }),
    }
}

/// Parse FTS (File Trailer Segment)
fn parse_fts_segment(input: &str, delimiters: &Delimiters) -> Result<FileTrailer> {
    let segment = parse_segment(input, delimiters)?;

    if segment.id != "FTS" {
        return Err(Error::parse("Expected FTS segment"));
    }

    let batch_count = parse_trailer_count("FTS-1 batch count", segment.get_field_value(1))?;

    let comment = segment.get_field_value(2).map(String::from);

    Ok(FileTrailer {
        batch_count,
        comment,
    })
}

/// HL7 NM (Numeric) grammar check: optional sign, digits, at most
/// one decimal point, at least one digit.
///
/// Deliberately not `f64` parsing, which accepts `NaN`, infinities,
/// and exponents that the NM grammar forbids.
fn is_valid_nm(value: &str) -> bool {
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    if digits.is_empty() {
        return false;
    }
    let mut seen_dot = false;
    let mut seen_digit = false;
    for c in digits.chars() {
        if c == '.' {
            if seen_dot {
                return false;
            }
            seen_dot = true;
        } else if c.is_ascii_digit() {
            seen_digit = true;
        } else {
            return false;
        }
    }
    seen_digit
}

/// Parse BTS (Batch Trailer Segment)
fn parse_bts_segment(input: &str, delimiters: &Delimiters) -> Result<BatchTrailer> {
    let segment = parse_segment(input, delimiters)?;

    if segment.id != "BTS" {
        return Err(Error::parse("Expected BTS segment"));
    }

    let message_count = parse_trailer_count("BTS-1 message count", segment.get_field_value(1))?;

    let comment = segment.get_field_value(2).map(String::from);

    // BTS-3 is a repeating field: collect every component of every
    // repetition. Blank entries carry no total; a present but
    // non-numeric entry is a parse error, never a silent drop.
    let mut totals = Vec::new();
    if let Some(field) = segment.get_field(3) {
        for repetition in &field.repetitions {
            for component in &repetition.components {
                let text = component.value().map(str::trim).unwrap_or("");
                if text.is_empty() {
                    continue;
                }
                if !is_valid_nm(text) {
                    return Err(Error::parse(format!(
                        "Invalid BTS-3 batch total: '{}' is not a number",
                        truncate_id(text)
                    )));
                }
                let total = text.parse::<f64>().map_err(|_| {
                    Error::parse(format!(
                        "Invalid BTS-3 batch total: '{}' is not a number",
                        truncate_id(text)
                    ))
                })?;
                totals.push(total);
            }
        }
    }

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

    // Parse the rest of the fields starting after "XXX|^~\&". The
    // byte after the encoding characters must be the field
    // separator: slicing past anything else would silently drop it.
    let header_end =
        3 + delimiters.field_separator.len_utf8() + delimiters.encoding_characters().len();
    if input.len() <= header_end {
        return Ok(segment);
    }

    let after_header = match input.get(header_end..) {
        Some(after_header) => after_header,
        None => {
            return Err(Error::parse(format!(
                "{} segment has invalid encoding characters",
                segment_type
            )));
        }
    };
    let rest = match after_header.strip_prefix(delimiters.field_separator) {
        Some(rest) => rest,
        None => {
            return Err(Error::parse(format!(
                "expected field separator after encoding characters in {} segment",
                segment_type
            )));
        }
    };
    if rest.is_empty() {
        // A lone trailing separator ("BHS|^~\&|") carries no content
        // fields, matching the historical boundary.
        return Ok(segment);
    }
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
///
/// A present but unparseable value is a parse error naming the field;
/// only absent or empty values yield `None`, so bad header datetimes
/// can no longer vanish into an empty re-encoded field. A `.SSSS`
/// fraction on a 14-digit head folds into nanoseconds.
fn parse_datetime_field(label: &str, value: Option<&str>) -> Result<Option<NaiveDateTime>> {
    let Some(s) = value else {
        return Ok(None);
    };
    if s.is_empty() {
        return Ok(None);
    }

    // Strip only a validated trailing [+/-]ZZZZ offset (we keep a
    // NaiveDateTime); any other '+'/'-' or a malformed offset is an
    // error, never silent truncation.
    let head = strip_validated_offset(label, s)?;

    // An optional fractional-seconds suffix (documented [.SSSS]) is
    // allowed only on 14-digit heads and folds into nanoseconds,
    // mirroring core types::parse_timestamp.
    let (digits, nanos) = match head.split_once('.') {
        None => (head, 0),
        Some((d, frac)) => {
            if d.len() != 14 || frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
                return Err(invalid_datetime(label, s));
            }
            // ASCII-only, so byte slicing is a char boundary; digits
            // beyond nanosecond precision are truncated.
            let head9 = &frac[..frac.len().min(9)];
            let mut nanos = head9.parse::<u32>().unwrap_or(u32::MAX);
            for _ in head9.len()..9 {
                nanos *= 10;
            }
            (d, nanos)
        }
    };

    // Exact-length digit heads: 14/12/10 keep their time, 8 is
    // date-only at midnight. Anything else (trailing garbage, short
    // heads, non-digits) is an error.
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid_datetime(label, s));
    }
    // The digits are ASCII (checked above), so these slices
    // are char boundaries and the parses cannot fail; out-of-range
    // values fall out through from_ymd_opt/from_hms_nano_opt.
    let num = |at: usize| digits[at..at + 2].parse::<u32>().unwrap_or(u32::MAX);
    let (hour, minute, second) = match digits.len() {
        14 => (num(8), num(10), num(12)),
        12 => (num(8), num(10), 0),
        10 => (num(8), 0, 0),
        8 => (0, 0, 0),
        _ => return Err(invalid_datetime(label, s)),
    };
    let year = digits[0..4].parse::<i32>().unwrap_or(-1);
    let date = chrono::NaiveDate::from_ymd_opt(year, num(4), num(6));
    let time = chrono::NaiveTime::from_hms_nano_opt(hour, minute, second, nanos);
    match (date, time) {
        (Some(d), Some(t)) => Ok(Some(NaiveDateTime::new(d, t))),
        _ => Err(invalid_datetime(label, s)),
    }
}

/// Split a trailing `[+/-ZZZZ]` offset, returning the head. A present
/// offset must be exactly a sign plus 4 in-range digits; any other
/// `+`/`-` is an error (mirrors core `split_offset`).
fn strip_validated_offset<'a>(label: &str, s: &'a str) -> Result<&'a str> {
    let Some(at) = s.find(['+', '-']) else {
        return Ok(s);
    };
    let (head, offset) = s.split_at(at);
    if head.is_empty() {
        return Err(invalid_datetime(label, s));
    }
    // u32::parse accepts a leading '+', so digit shape is checked
    // explicitly first (mirrors core split_offset); offset[1..] is a
    // char boundary: the sign is one ASCII byte.
    let digits = &offset[1..];
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid_datetime(label, s));
    }
    let n: u32 = digits.parse().unwrap_or(u32::MAX);
    if n / 100 > 23 || n % 100 > 59 {
        return Err(invalid_datetime(label, s));
    }
    Ok(head)
}

/// Shared "bad header datetime" error naming the field and value.
fn invalid_datetime(label: &str, s: &str) -> Error {
    Error::parse(format!(
        "Invalid {label} creation date/time: '{}' is not a valid date/time",
        truncate_id(s)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    use proptest::prelude::*;

    /// Field text without wire-structural characters.
    fn field_text() -> impl Strategy<Value = String> {
        prop::collection::vec(
            prop::sample::select(vec![
                'A', 'B', 'a', 'z', '0', '9', ' ', '.', '-', '%', '(', ')',
            ]),
            0..12,
        )
        .prop_map(|chars| chars.into_iter().collect())
    }

    proptest! {
        #[test]
        fn built_message_survives_encode_parse(
            pid_fields in prop::collection::vec(field_text(), 1..6).prop_filter(
                "final field must not end in whitespace (the default \
                 config strips trailing segment whitespace)",
                |fields| !fields.last().is_some_and(|f| f.ends_with(' ')),
            ),
        ) {
            // Build MSH + PID programmatically, then require the
            // encode -> parse -> encode fixed point.
            let mut msh = Segment::new_header("MSH");
            msh.add_field(Field::from_value("SEND"));
            msh.add_field(Field::from_value("SFAC"));
            msh.add_field(Field::from_value("RECV"));
            msh.add_field(Field::from_value("RFAC"));
            msh.add_field(Field::from_value("20240101120000"));
            msh.add_field(Field::from_value(""));
            msh.add_field(Field::from_delimited("ADT^A01"));
            msh.add_field(Field::from_value("MSG1"));
            msh.add_field(Field::from_value("P"));
            msh.add_field(Field::from_value("2.5"));
            let mut pid = Segment::new("PID");
            for value in &pid_fields {
                pid.add_field(Field::from_value(value));
            }
            let mut message = Message::new();
            message.add_segment(msh);
            message.add_segment(pid);

            let wire = message.encode();
            let parsed = parse_message(&wire).unwrap();
            prop_assert_eq!(parsed.encode(), wire);
            prop_assert_eq!(parsed, message);
        }
    }

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
    fn test_parse_bts_totals_collect_all_repetitions() {
        // BTS-3 is repeating: every repetition (and component) is a total.
        let delims = Delimiters::default();
        let trailer = parse_bts_segment("BTS|1|done|1.5~2.5", &delims).unwrap();
        assert_eq!(trailer.message_count, Some(1));
        assert_eq!(trailer.totals, vec![1.5, 2.5]);

        // Blank entries carry no total and do not error.
        let trailer = parse_bts_segment("BTS|1|done|", &delims).unwrap();
        assert!(trailer.totals.is_empty());
        let trailer = parse_bts_segment("BTS|1|done", &delims).unwrap();
        assert!(trailer.totals.is_empty());
    }

    #[test]
    fn test_parse_bts_rejects_non_numeric_counts_and_totals() {
        let delims = Delimiters::default();
        for bts in [
            "BTS|abc|done",
            "BTS|1|done|abc",
            "BTS|1|done|1.5~oops",
            "BTS|1.5|done",
        ] {
            let err = parse_bts_segment(bts, &delims).unwrap_err().to_string();
            assert!(err.contains("not a number"), "{bts}: {err}");
        }
        // Blank BTS-1 still means "no count given", not an error.
        let trailer = parse_bts_segment("BTS||done", &delims).unwrap();
        assert_eq!(trailer.message_count, None);
    }

    #[test]
    fn test_parse_fts_rejects_non_numeric_batch_count() {
        let delims = Delimiters::default();
        let err = parse_fts_segment("FTS|abc", &delims)
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a number"), "{err}");
        let trailer = parse_fts_segment("FTS|", &delims).unwrap();
        assert_eq!(trailer.batch_count, None);
    }

    #[test]
    fn test_msh_missing_separator_after_encoding_chars_errors() {
        // The byte after MSH-2 must be the field separator; slicing
        // past it silently dropped the 'X' (MSH-3 parsed as 'APP').
        let err = parse_message("MSH|^~\\&XAPP|FAC|\rPID|1")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("expected field separator after encoding characters"),
            "{err}"
        );
        // Bare headers and the trailing-separator form still parse.
        assert!(parse_message("MSH|^~\\&").is_ok());
        let msg = parse_message("MSH|^~\\&|").unwrap();
        assert_eq!(msg.segments[0].get_field_value(3), Some(""));
    }

    #[test]
    fn test_batch_header_missing_separator_after_encoding_chars_errors() {
        let delims = Delimiters::default();
        for (label, input) in [("BHS", "BHS|^~\\&XAPP|FAC"), ("FHS", "FHS|^~\\&XAPP|FAC")] {
            let err = parse_segment_like_msh(label, input, &delims)
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("expected field separator after encoding characters"),
                "{label}: {err}"
            );
        }
        // A lone trailing separator still carries no content fields.
        let segment = parse_segment_like_msh("BHS", "BHS|^~\\&|", &delims).unwrap();
        assert_eq!(segment.field_count(), 2);
    }

    #[test]
    fn test_trailer_count_echo_is_truncated() {
        let delims = Delimiters::default();
        let digits = "9".repeat(300);
        for input in [format!("BTS|{digits}"), format!("FTS|{digits}")] {
            let err = match &input[..3] {
                "BTS" => parse_bts_segment(&input, &delims).unwrap_err().to_string(),
                _ => parse_fts_segment(&input, &delims).unwrap_err().to_string(),
            };
            assert!(!err.contains(&digits), "must not echo full count");
            assert!(err.contains("300 chars total"), "{err}");
        }
        // Long BTS-3 echoes are capped as well.
        let junk = "z".repeat(300);
        let err = parse_bts_segment(&format!("BTS|1||{junk}"), &delims)
            .unwrap_err()
            .to_string();
        assert!(!err.contains(&junk), "must not echo full total");
        assert!(err.contains("300 chars total"), "{err}");
    }

    #[test]
    fn test_parse_bts_rejects_non_nm_float_spellings() {
        // f64 parses these, but the HL7 NM grammar forbids NaN,
        // infinities, and exponents.
        let delims = Delimiters::default();
        for total in ["NaN", "inf", "-inf", "1e5", "Infinity", "nan"] {
            let err = parse_bts_segment(&format!("BTS|1||{total}"), &delims)
                .unwrap_err()
                .to_string();
            assert!(err.contains("not a number"), "{total}: {err}");
        }
        // Genuine NM values still parse.
        let trailer = parse_bts_segment("BTS|1||-12.5~+3~0.25", &delims).unwrap();
        assert_eq!(trailer.totals, vec![-12.5, 3.0, 0.25]);
    }

    #[test]
    fn test_batch_header_garbage_datetime_errors() {
        let msh = "MSH|^~\\&|S|F|R|D|20240115||ADT^A01|1|P|2.5";
        let batch_text = format!("BHS|^~\\&|S|F|R|D|garbage-datetime||B1\r{msh}\rPID|1\rBTS|1");
        let err = parse_batch(&batch_text).unwrap_err().to_string();
        assert!(err.contains("BHS-7"), "{err}");
        assert!(err.contains("garbage-datetime"), "{err}");
        // Empty BHS-7 still means "no timestamp", not an error.
        let ok_text = format!("BHS|^~\\&|S|F|R|D|||B1\r{msh}\rPID|1\rBTS|1");
        let batch = parse_batch(&ok_text).unwrap();
        assert!(batch.header.creation_datetime.is_none());
    }

    #[test]
    fn test_multibyte_delimiters_rejected_by_both_paths() {
        // ASCII-only delimiters: strict and streaming agree.
        let input = "MSHé^~\\&éS";
        assert!(parse_message(input).is_err());
        assert!(parse_streaming(input).is_err());
    }

    #[test]
    fn test_strict_parse_rejects_unterminated_escape() {
        // Unterminated escapes error like other malformed escapes do.
        let err = parse_message("MSH|^~\\&|A|B|||20250115||ADT^A01|1|P|2.5\rPID|1||X\\X0D")
            .unwrap_err()
            .to_string();
        assert!(err.contains("Unterminated"), "{err}");
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
        let dt1 = parse_datetime_field("BHS-7", Some("20240315143000")).unwrap();
        assert!(dt1.is_some());
        let dt1 = dt1.unwrap();
        assert_eq!(dt1.year(), 2024);
        assert_eq!(dt1.month(), 3);
        assert_eq!(dt1.day(), 15);
        assert_eq!(dt1.hour(), 14);
        assert_eq!(dt1.minute(), 30);
        assert_eq!(dt1.second(), 0);

        // Date only
        let dt2 = parse_datetime_field("BHS-7", Some("20240315")).unwrap();
        assert!(dt2.is_some());
        let dt2 = dt2.unwrap();
        assert_eq!(dt2.year(), 2024);
        assert_eq!(dt2.month(), 3);
        assert_eq!(dt2.day(), 15);

        // Empty
        let dt3 = parse_datetime_field("BHS-7", Some("")).unwrap();
        assert!(dt3.is_none());

        // None
        let dt4 = parse_datetime_field("BHS-7", None).unwrap();
        assert!(dt4.is_none());

        // Garbage names the field instead of vanishing into None.
        let err = parse_datetime_field("BHS-7", Some("garbage-datetime"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("BHS-7"), "{err}");
        assert!(err.contains("garbage-datetime"), "{err}");
    }

    #[test]
    fn test_parse_datetime_field_validates_offsets_and_time() {
        // Malformed offsets are errors, never silent truncation.
        assert!(parse_datetime_field("BHS-7", Some("2024010112+100")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240101-99")).is_err());
        // Hour/minute precisions keep their time, not midnight.
        let dt = parse_datetime_field("BHS-7", Some("2024010112"))
            .unwrap()
            .unwrap();
        assert_eq!((dt.hour(), dt.minute()), (12, 0));
        let dt = parse_datetime_field("BHS-7", Some("202401011230"))
            .unwrap()
            .unwrap();
        assert_eq!((dt.hour(), dt.minute()), (12, 30));
        // Trailing garbage is an error at every precision.
        assert!(parse_datetime_field("BHS-7", Some("20240101120000XYZ")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240101XYZ")).is_err());
        // Controls: validated offsets parse, garbage is rejected.
        assert!(parse_datetime_field("BHS-7", Some("20240101120000+0800")).is_ok());
        assert!(parse_datetime_field("BHS-7", Some("20240101120000-0500")).is_ok());
        assert!(parse_datetime_field("BHS-7", Some("garbage-datetime")).is_err());
    }

    #[test]
    fn test_parse_datetime_field_accepts_fractional_seconds() {
        // Documented [.SSSS]: fraction folds into nanoseconds.
        let dt = parse_datetime_field("BHS-7", Some("20240101120000.123"))
            .unwrap()
            .unwrap();
        assert_eq!(dt.nanosecond(), 123_000_000);
        assert_eq!((dt.hour(), dt.minute(), dt.second()), (12, 0, 0));
        // Fraction plus validated offset.
        let dt = parse_datetime_field("BHS-7", Some("20240101120000.123+0800"))
            .unwrap()
            .unwrap();
        assert_eq!(dt.nanosecond(), 123_000_000);
        // Fraction only on 14-digit heads; malformed fractions rejected.
        assert!(parse_datetime_field("BHS-7", Some("202401011200.123")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240101120000.")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240101120000.12X")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240101120000.1.2")).is_err());
    }

    #[test]
    fn test_parse_datetime_field_rejects_double_sign_offset() {
        // u32::parse accepts a leading '+', so "++059" must be
        // rejected by an explicit all-digits check, not the parse.
        assert!(parse_datetime_field("BHS-7", Some("20240101120000++059")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240101120000+-059")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240101120000-+059")).is_err());
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

        let error = message.encode_bytes().unwrap_err().to_string();
        assert!(error.contains("U+00FC"), "{error}");
        assert!(!error.contains('ü'), "{error}");
    }

    #[test]
    fn test_lenient_short_input_returns_error_without_panic() {
        let lenient = ParserConfig::lenient();
        // "MSH|^~" is absent here on purpose: short but usable MSH-2
        // runs are padded under allow_non_standard_encoding_chars (see
        // test_lenient_short_encoding_chars_are_padded).
        for input in ["", "M", "MS", "MSH", "MSH|"] {
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
        let mut warnings = Vec::new();
        let field = parse_field_with_config("abcdéfgh", &delims, &config, &mut warnings).unwrap();
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
    fn test_non_ascii_datetime_field_returns_error_without_panic() {
        assert!(parse_datetime_field("BHS-7", Some("2024010é123456")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("20240é15")).is_err());
        assert!(parse_datetime_field("BHS-7", Some("éééééééééééééééé")).is_err());
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

    #[test]
    fn test_preserve_invalid_escapes_keeps_literal_text() {
        let config = ParserConfig::new().preserve_invalid_escapes(true);
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rPID|1||X\\Q\\Y";
        let result = parse_message_with_config(msg, &config).unwrap();
        let pid = result
            .value
            .segments
            .iter()
            .find(|s| s.id == "PID")
            .unwrap();
        assert_eq!(pid.get_field_value(3), Some("X\\Q\\Y"));
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::InvalidEscape)
        );
    }

    #[test]
    fn test_invalid_escape_errors_in_strict_mode() {
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rPID|1||X\\Q\\Y";
        assert!(parse_message_with_config(msg, &ParserConfig::strict()).is_err());
    }

    #[test]
    fn test_validate_segment_ids_warns_on_unknown_id() {
        let config = ParserConfig::new().validate_segment_ids(true);
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rXXX|1|2\rPID|1";
        let result = parse_message_with_config(msg, &config).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::UnknownSegmentId)
        );
    }

    #[test]
    fn test_validate_segment_ids_accepts_known_ids() {
        let config = ParserConfig::new().validate_segment_ids(true);
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rPID|1\rPV1|1|O";
        let result = parse_message_with_config(msg, &config).unwrap();
        assert!(
            !result
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::UnknownSegmentId)
        );
    }

    #[test]
    fn test_max_field_length_truncation_emits_warning() {
        let config = ParserConfig::new().max_field_length(2);
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rPID|1||ABCDE";
        let result = parse_message_with_config(msg, &config).unwrap();
        let pid = result
            .value
            .segments
            .iter()
            .find(|s| s.id == "PID")
            .unwrap();
        assert_eq!(pid.get_field_value(3), Some("AB"));
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::FieldTruncated)
        );
    }

    #[test]
    fn test_max_repetitions_truncation_emits_warning() {
        let config = ParserConfig::new().max_repetitions(2);
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rPID|1||a~b~c";
        let result = parse_message_with_config(msg, &config).unwrap();
        let pid = result
            .value
            .segments
            .iter()
            .find(|s| s.id == "PID")
            .unwrap();
        assert_eq!(pid.get_field(3).unwrap().repetitions.len(), 2);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::RepetitionsTruncated)
        );
    }

    #[test]
    fn test_lenient_short_encoding_chars_are_padded() {
        // allow_non_standard_encoding_chars tolerates short MSH-2 runs:
        // missing positions fall back to the standard defaults.
        let config = ParserConfig::lenient();
        for msh in ["MSH|^~", "MSH|^~\\"] {
            let result = parse_message_with_config(msh, &config);
            assert!(result.is_ok(), "{msh}: {:?}", result.err());
        }
        let result = parse_message_with_config("MSH|^~", &config).unwrap();
        assert_eq!(result.value.delimiters, Delimiters::default());
    }

    #[test]
    fn test_strict_short_encoding_chars_still_rejected() {
        let config = ParserConfig::strict();
        assert!(parse_message_with_config("MSH|^~", &config).is_err());
    }

    #[test]
    fn test_bad_separator_with_continue_on_error_warns_skipped_segment() {
        // A bad separator after the segment ID keeps the (empty) segment
        // under continue_on_error but must warn like other skips.
        let config = ParserConfig::strict().continue_on_error(true);
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rPID |1|DOE";
        let result = parse_message_with_config(msg, &config).unwrap();
        let pid = result
            .value
            .segments
            .iter()
            .find(|s| s.id == "PID")
            .unwrap();
        assert!(pid.fields.is_empty());
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::SkippedSegment),
            "warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_max_segments_truncation_emits_warning() {
        let config = ParserConfig::new().max_segments(2).continue_on_error(true);
        let msg = "MSH|^~\\&|S|F|R|RF|20240315||ADT^A01|1|P|2.5\rPID|1\rPV1|1|O";
        let result = parse_message_with_config(msg, &config).unwrap();
        assert_eq!(result.value.segments.len(), 2);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == WarningCode::SegmentsTruncated)
        );
    }

    #[test]
    fn test_second_msh_keeps_msh_field_alignment() {
        let msg = "MSH|^~\\&|APP1|FAC1|R|D|20240101||ADT^A01|123|P|2.5\r\
                   PID|1||PAT001\r\
                   MSH|^~\\&|APP2|FAC2|R|D|20240102||ADT^A01|124|P|2.5\r\
                   PID|1||PAT002\r";
        for config in [ParserConfig::strict(), ParserConfig::lenient()] {
            let message = parse_message_with_config(msg, &config).unwrap().value;
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
    }

    #[test]
    fn test_is_msh_segment_rejects_longer_ids() {
        assert!(is_msh_segment("MSH|^~\\&|APP"));
        assert!(is_msh_segment("MSH"));
        assert!(!is_msh_segment("MSHA|1"));
        assert!(!is_msh_segment("PID|1"));
    }

    #[test]
    fn test_long_segment_id_is_truncated_in_error() {
        let long = "A".repeat(66);
        let msg = format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r{long}");
        let err = parse_message(&msg).unwrap_err().to_string();
        assert!(!err.contains(&long), "{err}");
        assert!(err.contains("66"), "{err}");
        assert!(err.contains("chars total"), "{err}");
    }

    #[test]
    fn test_leading_blank_line_before_msh_is_skipped() {
        // A whitespace-only first line is blank and skipped when
        // skip_blank_lines is set, like the streaming parser does.
        let msg = "   \rMSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||PAT001\r";
        let message = parse_message_with_config(msg, &ParserConfig::strict())
            .unwrap()
            .value;
        assert_eq!(message.segments.len(), 2);
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "PID");
    }

    #[test]
    fn test_msh_explicit_empty_msh3_is_some_empty() {
        // 'MSH|^~\&|' carries an explicit empty MSH-3, like any other
        // trailing empty field; only the bare header has no MSH-3.
        let message = parse_message("MSH|^~\\&|").unwrap();
        assert_eq!(message.segments[0].get_field_value(3), Some(""));
        let bare = parse_message("MSH|^~\\&").unwrap();
        assert_eq!(bare.segments[0].get_field(3), None);
    }
}
