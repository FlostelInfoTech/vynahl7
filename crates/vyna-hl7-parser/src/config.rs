//! Parser configuration for lenient/strict parsing modes
//!
//! This module provides configurable parsing behavior to handle real-world
//! HL7 messages that may not strictly conform to the specification.
//!
//! # Overview
//!
//! In production environments, HL7 messages often have issues:
//! - Trailing delimiters at the end of segments
//! - Non-standard segment IDs (2 or 4+ characters)
//! - Missing or malformed encoding characters
//! - Invalid escape sequences
//!
//! The `ParserConfig` allows you to choose between strict (spec-compliant)
//! and lenient (real-world tolerant) parsing.
//!
//! # Examples
//!
//! ```rust
//! use vyna_hl7_parser::{ParserConfig, parse_message_with_config};
//!
//! // Strict mode (default) - fails on non-compliant messages
//! let strict = ParserConfig::strict();
//!
//! // Lenient mode - tolerates common real-world deviations
//! let lenient = ParserConfig::lenient();
//!
//! // Custom configuration
//! let custom = ParserConfig::new()
//!     .allow_trailing_delimiters(true)
//!     .allow_non_standard_segment_ids(true)
//!     .strip_trailing_whitespace(true);
//!
//! let message_text = "MSH|^~\\&|App|Fac|||20240315||ADT^A01|123|P|2.5|\r";
//! let result = parse_message_with_config(message_text, &lenient);
//! ```

/// Configuration options for the HL7 parser
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Allow trailing field delimiters at the end of segments
    ///
    /// When true, "PID|1|2|3|" is equivalent to "PID|1|2|3"
    /// Default: false (strict)
    pub allow_trailing_delimiters: bool,

    /// Allow segment IDs that are not exactly 3 characters
    ///
    /// Standard HL7 segment IDs are 3 characters (e.g., MSH, PID, OBX).
    /// Some systems use 2-character IDs or Z-segments with more characters.
    /// Default: false (strict)
    pub allow_non_standard_segment_ids: bool,

    /// Strip trailing whitespace from segment lines
    ///
    /// When true, "PID|1|2  \r" is parsed as "PID|1|2"
    /// Default: true
    pub strip_trailing_whitespace: bool,

    /// Strip leading whitespace from segment lines
    ///
    /// When true, "  PID|1|2" is parsed as "PID|1|2"
    /// Default: false
    pub strip_leading_whitespace: bool,

    /// Allow non-standard encoding characters in MSH-2
    ///
    /// Standard encoding characters are "^~\&" (4 characters).
    /// Some systems may use different or fewer characters.
    /// Default: false (strict)
    pub allow_non_standard_encoding_chars: bool,

    /// Preserve invalid escape sequences as literal text
    ///
    /// When true, "\X" (invalid escape) is kept as "\X" and reported with
    /// an `InvalidEscape` warning
    /// When false, invalid escapes cause an error
    /// Default: false (strict)
    pub preserve_invalid_escapes: bool,

    /// Allow empty segment ID
    ///
    /// When true, lines starting with "|" are skipped
    /// When false, such lines cause an error
    /// Default: false (strict)
    pub allow_empty_segment_id: bool,

    /// Skip blank lines in the message
    ///
    /// When true, blank lines between segments are ignored
    /// When false, blank lines may cause errors
    /// Default: true
    pub skip_blank_lines: bool,

    /// Maximum field length (0 = unlimited)
    ///
    /// Truncate fields exceeding this length. Useful for preventing
    /// memory issues with malformed messages.
    /// Default: 0 (unlimited)
    pub max_field_length: usize,

    /// Maximum number of repetitions per field (0 = unlimited)
    ///
    /// Default: 0 (unlimited)
    pub max_repetitions: usize,

    /// Maximum number of segments per message (0 = unlimited)
    ///
    /// Default: 0 (unlimited)
    pub max_segments: usize,

    /// Continue parsing after encountering an error
    ///
    /// When true, errors are collected but parsing continues
    /// When false, parsing stops at the first error
    /// Default: false
    pub continue_on_error: bool,

    /// Validate segment IDs against known HL7 segments
    ///
    /// When true, unknown segment IDs cause a warning (not error) with
    /// an `UnknownSegmentId` code; parsing still succeeds
    /// When false, any 3-character ID is accepted
    /// Default: false
    pub validate_segment_ids: bool,
}

/// Standard HL7 v2.x segment IDs (all versions through 2.8).
///
/// Used by `ParserConfig::validate_segment_ids`: any parsed segment ID
/// not in this table produces an `UnknownSegmentId` warning. Site-defined
/// Z-segments are intentionally not listed and warn when validation is on.
pub(crate) const KNOWN_SEGMENT_IDS: &[&str] = &[
    "ABS", "ACC", "ADD", "ADJ", "AFF", "AIG", "AIL", "AIP", "AIS", "AL1", "APR", "ARQ", "ARV",
    "AUT", "BHS", "BLC", "BPO", "BPX", "BTS", "BTX", "CDM", "CM0", "CM1", "CM2", "CNS", "CON",
    "CSP", "CSR", "CSS", "CTD", "CTI", "DB1", "DG1", "DON", "DRG", "DSC", "DSP", "ECD", "ECR",
    "EDU", "EQL", "EQU", "ERQ", "ERR", "EVN", "FAC", "FHS", "FT1", "FTS", "GOL", "GP1", "GP2",
    "GT1", "IAM", "IN1", "IN2", "IN3", "INV", "IPC", "ISD", "LAN", "LCH", "LDP", "LOC", "LRL",
    "MFA", "MFE", "MFI", "MRG", "MSA", "MSH", "NCK", "NDS", "NK1", "NPU", "NSC", "NST", "NTE",
    "OBR", "OBX", "ODS", "ODT", "OH1", "OH2", "OH3", "OH4", "OM1", "OM2", "OM3", "OM4", "OM5",
    "OM6", "OM7", "ORC", "ORG", "OVR", "PCR", "PD1", "PDC", "PDA", "PEO", "PES", "PID", "PR1",
    "PRA", "PRB", "PRC", "PRD", "PRT", "PSG", "PSH", "PTH", "PV1", "PV2", "PYE", "QAK", "QID",
    "QPD", "QRD", "QRF", "QRI", "RCP", "RDF", "RDT", "REL", "RF1", "RGS", "RMI", "ROL", "RQ1",
    "RQD", "RXA", "RXC", "RXD", "RXE", "RXG", "RXO", "RXR", "SAC", "SCH", "SDD", "SFT", "SID",
    "SPM", "STF", "TAX", "TCD", "TQ1", "TQ2", "TXA", "UAC", "UB1", "UB2", "URD", "URS", "VAR",
];

/// Check whether `id` is a standard HL7 segment ID.
pub(crate) fn is_known_segment_id(id: &str) -> bool {
    KNOWN_SEGMENT_IDS.contains(&id)
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self::strict()
    }
}

impl ParserConfig {
    /// Create a new parser config with default strict settings
    pub fn new() -> Self {
        Self::strict()
    }

    /// Create a strict parser configuration
    ///
    /// This is the default and enforces HL7 specification compliance.
    pub fn strict() -> Self {
        Self {
            allow_trailing_delimiters: false,
            allow_non_standard_segment_ids: false,
            strip_trailing_whitespace: true,
            strip_leading_whitespace: false,
            allow_non_standard_encoding_chars: false,
            preserve_invalid_escapes: false,
            allow_empty_segment_id: false,
            skip_blank_lines: true,
            max_field_length: 0,
            max_repetitions: 0,
            max_segments: 0,
            continue_on_error: false,
            validate_segment_ids: false,
        }
    }

    /// Create a lenient parser configuration
    ///
    /// This configuration tolerates common real-world deviations from
    /// the HL7 specification. Use this for parsing messages from systems
    /// known to produce non-compliant output.
    pub fn lenient() -> Self {
        Self {
            allow_trailing_delimiters: true,
            allow_non_standard_segment_ids: true,
            strip_trailing_whitespace: true,
            strip_leading_whitespace: true,
            allow_non_standard_encoding_chars: true,
            preserve_invalid_escapes: true,
            allow_empty_segment_id: true,
            skip_blank_lines: true,
            max_field_length: 0,
            max_repetitions: 0,
            max_segments: 0,
            continue_on_error: true,
            validate_segment_ids: false,
        }
    }

    /// Set whether to allow trailing delimiters
    pub fn allow_trailing_delimiters(mut self, allow: bool) -> Self {
        self.allow_trailing_delimiters = allow;
        self
    }

    /// Set whether to allow non-standard segment IDs
    pub fn allow_non_standard_segment_ids(mut self, allow: bool) -> Self {
        self.allow_non_standard_segment_ids = allow;
        self
    }

    /// Set whether to strip trailing whitespace
    pub fn strip_trailing_whitespace(mut self, strip: bool) -> Self {
        self.strip_trailing_whitespace = strip;
        self
    }

    /// Set whether to strip leading whitespace
    pub fn strip_leading_whitespace(mut self, strip: bool) -> Self {
        self.strip_leading_whitespace = strip;
        self
    }

    /// Set whether to allow non-standard encoding characters
    pub fn allow_non_standard_encoding_chars(mut self, allow: bool) -> Self {
        self.allow_non_standard_encoding_chars = allow;
        self
    }

    /// Set whether to preserve invalid escape sequences
    pub fn preserve_invalid_escapes(mut self, preserve: bool) -> Self {
        self.preserve_invalid_escapes = preserve;
        self
    }

    /// Set whether to allow empty segment IDs
    pub fn allow_empty_segment_id(mut self, allow: bool) -> Self {
        self.allow_empty_segment_id = allow;
        self
    }

    /// Set whether to skip blank lines
    pub fn skip_blank_lines(mut self, skip: bool) -> Self {
        self.skip_blank_lines = skip;
        self
    }

    /// Set maximum field length (0 = unlimited)
    pub fn max_field_length(mut self, max: usize) -> Self {
        self.max_field_length = max;
        self
    }

    /// Set maximum repetitions per field (0 = unlimited)
    pub fn max_repetitions(mut self, max: usize) -> Self {
        self.max_repetitions = max;
        self
    }

    /// Set maximum segments per message (0 = unlimited)
    pub fn max_segments(mut self, max: usize) -> Self {
        self.max_segments = max;
        self
    }

    /// Set whether to continue parsing after errors
    pub fn continue_on_error(mut self, continue_on: bool) -> Self {
        self.continue_on_error = continue_on;
        self
    }

    /// Set whether to validate segment IDs
    pub fn validate_segment_ids(mut self, validate: bool) -> Self {
        self.validate_segment_ids = validate;
        self
    }
}

/// Warning generated during lenient parsing
#[derive(Debug, Clone)]
pub struct ParseWarning {
    /// Line or segment index where warning occurred
    pub location: usize,
    /// Warning message
    pub message: String,
    /// Warning code
    pub code: WarningCode,
}

/// Warning codes for parsing issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    /// Trailing delimiter was removed
    TrailingDelimiter,
    /// Non-standard segment ID was accepted
    NonStandardSegmentId,
    /// Whitespace was stripped
    WhitespaceStripped,
    /// Invalid escape sequence was preserved
    InvalidEscape,
    /// Empty segment was skipped
    EmptySegment,
    /// Blank line was skipped
    BlankLine,
    /// Field was truncated
    FieldTruncated,
    /// Repetitions were truncated
    RepetitionsTruncated,
    /// Segments beyond `max_segments` were dropped
    SegmentsTruncated,
    /// Unknown segment ID
    UnknownSegmentId,
    /// Segment skipped after a parse error with continue_on_error
    SkippedSegment,
}

impl std::fmt::Display for WarningCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WarningCode::TrailingDelimiter => write!(f, "TRAILING_DELIMITER"),
            WarningCode::NonStandardSegmentId => write!(f, "NON_STANDARD_SEGMENT_ID"),
            WarningCode::WhitespaceStripped => write!(f, "WHITESPACE_STRIPPED"),
            WarningCode::InvalidEscape => write!(f, "INVALID_ESCAPE"),
            WarningCode::EmptySegment => write!(f, "EMPTY_SEGMENT"),
            WarningCode::BlankLine => write!(f, "BLANK_LINE"),
            WarningCode::FieldTruncated => write!(f, "FIELD_TRUNCATED"),
            WarningCode::RepetitionsTruncated => write!(f, "REPETITIONS_TRUNCATED"),
            WarningCode::SegmentsTruncated => write!(f, "SEGMENTS_TRUNCATED"),
            WarningCode::UnknownSegmentId => write!(f, "UNKNOWN_SEGMENT_ID"),
            WarningCode::SkippedSegment => write!(f, "SKIPPED_SEGMENT"),
        }
    }
}

/// Result of parsing with configuration, includes warnings
#[derive(Debug)]
pub struct ParseResult<T> {
    /// The parsed value (if successful)
    pub value: T,
    /// Warnings generated during parsing
    pub warnings: Vec<ParseWarning>,
}

impl<T> ParseResult<T> {
    /// Create a new parse result
    pub fn new(value: T) -> Self {
        Self {
            value,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: ParseWarning) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Add multiple warnings
    pub fn with_warnings(mut self, warnings: Vec<ParseWarning>) -> Self {
        self.warnings.extend(warnings);
        self
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get the number of warnings
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_config() {
        let config = ParserConfig::strict();

        assert!(!config.allow_trailing_delimiters);
        assert!(!config.allow_non_standard_segment_ids);
        assert!(config.strip_trailing_whitespace);
        assert!(!config.strip_leading_whitespace);
        assert!(!config.allow_non_standard_encoding_chars);
        assert!(!config.preserve_invalid_escapes);
        assert!(!config.continue_on_error);
    }

    #[test]
    fn test_lenient_config() {
        let config = ParserConfig::lenient();

        assert!(config.allow_trailing_delimiters);
        assert!(config.allow_non_standard_segment_ids);
        assert!(config.strip_trailing_whitespace);
        assert!(config.strip_leading_whitespace);
        assert!(config.allow_non_standard_encoding_chars);
        assert!(config.preserve_invalid_escapes);
        assert!(config.continue_on_error);
    }

    #[test]
    fn test_builder_pattern() {
        let config = ParserConfig::new()
            .allow_trailing_delimiters(true)
            .max_field_length(1000)
            .max_segments(100);

        assert!(config.allow_trailing_delimiters);
        assert_eq!(config.max_field_length, 1000);
        assert_eq!(config.max_segments, 100);
    }

    #[test]
    fn test_parse_result() {
        let result: ParseResult<String> = ParseResult::new("test".to_string());
        assert!(!result.has_warnings());
        assert_eq!(result.warning_count(), 0);

        let result = result.with_warning(ParseWarning {
            location: 0,
            message: "Test warning".to_string(),
            code: WarningCode::TrailingDelimiter,
        });

        assert!(result.has_warnings());
        assert_eq!(result.warning_count(), 1);
    }

    #[test]
    fn test_warning_code_display() {
        assert_eq!(
            format!("{}", WarningCode::TrailingDelimiter),
            "TRAILING_DELIMITER"
        );
        assert_eq!(
            format!("{}", WarningCode::NonStandardSegmentId),
            "NON_STANDARD_SEGMENT_ID"
        );
    }
}
