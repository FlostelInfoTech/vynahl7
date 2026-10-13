//! HL7 character set support (MSH-18, HL7 table 0211)
//!
//! HL7 messages declare their wire encoding in MSH-18. [`CharacterSet`] maps
//! that declaration to a text encoding so inbound wire bytes can be decoded
//! and outbound messages encoded in anything from ASCII to ISO-8859-16:
//!
//! ```rust
//! use vyna_hl7_core::charset::CharacterSet;
//!
//! // ISO-8859-1 bytes for "M\xfcller" with MSH-18 = 8859/1.
//! let wire: &[u8] = b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1||1||M\xfcller||19800101|M\r";
//! let text = CharacterSet::decode_message(wire).unwrap();
//! assert!(text.contains("M\u{fc}ller"));
//! ```
//!
//! Conversion is strict: malformed input and unmappable characters are
//! errors, never silent replacements.

use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};

/// Suffix listing every supported MSH-18 value, shared by error messages.
const SUPPORTED_SETS: &str = "supported: ASCII, UNICODE UTF-8, 8859/1-8859/11, 8859/13-8859/16";

/// HL7 character set from MSH-18 (HL7 table 0211).
///
/// Only single-byte encodings plus UTF-8 are supported. `UNICODE` (UTF-16)
/// is rejected because MLLP framing is byte-oriented and cannot frame
/// UTF-16 payloads; peers must send `UNICODE UTF-8` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharacterSet {
    /// 7-bit ASCII (`ASCII`).
    Ascii,
    /// UTF-8 (`UNICODE UTF-8`). Also the default when MSH-18 is empty.
    Utf8,
    /// ISO-8859-N single-byte encoding (`8859/N`).
    ///
    /// `N` must be 1-11 or 13-16; anything else fails conversion with an
    /// unsupported-character-set error instead of panicking.
    Iso8859(u8),
}

impl CharacterSet {
    /// Parse an MSH-18 character set declaration.
    ///
    /// Matching is case-insensitive, surrounding whitespace is ignored, and
    /// only the first repetition is used when MSH-18 repeats. An empty value
    /// means the sender declared nothing and returns the default (UTF-8).
    /// Common aliases (`UTF-8`, `ISO-8859-1`, `WINDOWS-1252`, ...) are
    /// accepted alongside the canonical table 0211 values.
    pub fn from_msh18(value: &str) -> Result<Self> {
        let first = value.split('~').next().unwrap_or("").trim();
        if first.is_empty() {
            return Ok(Self::default());
        }
        let upper = first.to_ascii_uppercase();
        let words: Vec<&str> = upper.split_whitespace().collect();
        let normalized = words.join(" ");
        match normalized.as_str() {
            "ASCII" => Ok(Self::Ascii),
            "UNICODE UTF-8" | "UTF-8" | "UTF8" => Ok(Self::Utf8),
            "UNICODE" => Err(Error::Decoding(
                "character set 'UNICODE' (UTF-16) is not supported; use 'UNICODE UTF-8' or a single-byte encoding instead"
                    .to_string(),
            )),
            "ISO IR87" | "ISO IR159" => Err(Error::Decoding(format!(
                "character set '{first}' is not supported ({SUPPORTED_SETS})"
            ))),
            "WINDOWS-1252" | "WINDOWS1252" => Ok(Self::Iso8859(1)),
            _ => {
                if let Some(number) = parse_8859_number(&normalized) {
                    return match number {
                        1..=11 | 13..=16 => Ok(Self::Iso8859(number)),
                        12 => Err(Error::Decoding(
                            "character set '8859/12' was withdrawn and has no registered encoding"
                                .to_string(),
                        )),
                        _ => Err(Error::Decoding(format!(
                            "unsupported character set '{first}' in MSH-18 ({SUPPORTED_SETS})"
                        ))),
                    };
                }
                Err(Error::Decoding(format!(
                    "unsupported character set '{first}' in MSH-18 ({SUPPORTED_SETS})"
                )))
            }
        }
    }

    /// Detect the character set of raw wire bytes from MSH-18.
    ///
    /// Returns the default (UTF-8) when the bytes do not start with an MSH
    /// segment or MSH-18 is empty. A UTF-16 byte-order mark is rejected
    /// because UTF-16 cannot be framed by MLLP.
    pub fn detect(bytes: &[u8]) -> Result<Self> {
        if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
            return Err(Error::Decoding(
                "message appears to be UTF-16 encoded (byte-order mark found); UTF-16 is not supported, send as UNICODE UTF-8 instead"
                    .to_string(),
            ));
        }
        match extract_msh18(bytes) {
            Some(value) => Self::from_msh18(value),
            None => Ok(Self::default()),
        }
    }

    /// Decode raw wire bytes to text using the MSH-18 declaration.
    ///
    /// This is [`detect`](Self::detect) followed by [`decode`](Self::decode), plus
    /// tolerance for a leading UTF-8 byte-order mark when the declared (or
    /// default) character set is UTF-8.
    pub fn decode_message(bytes: &[u8]) -> Result<String> {
        let charset = Self::detect(bytes)?;
        let body = match charset {
            Self::Utf8 => bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes),
            _ => bytes,
        };
        charset.decode(body)
    }

    /// Decode bytes in this character set. Malformed input is an error.
    pub fn decode(self, bytes: &[u8]) -> Result<String> {
        match self {
            Self::Ascii => decode_ascii(bytes),
            Self::Utf8 => std::str::from_utf8(bytes).map(str::to_string).map_err(|e| {
                Error::Decoding(format!(
                    "message bytes are not valid UTF-8: {e}; declare the sender's character set in MSH-18"
                ))
            }),
            Self::Iso8859(number) => {
                let encoding = single_byte_encoding(number).ok_or_else(|| {
                    Error::Decoding(unsupported_8859_message(number))
                })?;
                encoding
                    .decode_without_bom_handling_and_without_replacement(bytes)
                    .map(|text| text.into_owned())
                    .ok_or_else(|| {
                        Error::Decoding(format!(
                            "message bytes cannot be decoded as 8859/{number}"
                        ))
                    })
            }
        }
    }

    /// Encode text to wire bytes in this character set.
    ///
    /// Characters that have no representation in the target set are an
    /// error; nothing is silently replaced or dropped.
    pub fn encode(self, text: &str) -> Result<Vec<u8>> {
        match self {
            Self::Ascii => {
                if let Some(offender) = text.chars().find(|c| !c.is_ascii()) {
                    return Err(Error::Encoding(format!(
                        "character '{offender}' (U+{:04X}) cannot be encoded in ASCII",
                        offender as u32
                    )));
                }
                Ok(text.as_bytes().to_vec())
            }
            Self::Utf8 => Ok(text.as_bytes().to_vec()),
            Self::Iso8859(number) => {
                let encoding = single_byte_encoding(number)
                    .ok_or_else(|| Error::Encoding(unsupported_8859_message(number)))?;
                let (bytes, _, had_errors) = encoding.encode(text);
                if !had_errors {
                    return Ok(bytes.into_owned());
                }
                match Self::Iso8859(number).find_unmappable(text) {
                    Some(offender) => Err(Error::Encoding(format!(
                        "character '{offender}' (U+{:04X}) cannot be encoded in 8859/{number}",
                        offender as u32
                    ))),
                    None => Err(Error::Encoding(format!(
                        "message text cannot be encoded in 8859/{number}"
                    ))),
                }
            }
        }
    }

    /// First character with no representation in this character set, if any.
    ///
    /// Returns `None` for UTF-8 (everything is representable) and for
    /// 8859 parts with no registered encoding.
    pub fn find_unmappable(self, text: &str) -> Option<char> {
        match self {
            Self::Ascii => text.chars().find(|c| !c.is_ascii()),
            Self::Utf8 => None,
            Self::Iso8859(number) => {
                let encoding = single_byte_encoding(number)?;
                let mut buffer = [0u8; 4];
                text.chars().find(|c| {
                    let encoded = c.encode_utf8(&mut buffer);
                    let (_, _, failed) = encoding.encode(encoded);
                    failed
                })
            }
        }
    }

    /// Canonical HL7 table 0211 value for MSH-18.
    pub fn msh18(self) -> String {
        match self {
            Self::Ascii => "ASCII".to_string(),
            Self::Utf8 => "UNICODE UTF-8".to_string(),
            Self::Iso8859(number) => format!("8859/{number}"),
        }
    }

    /// IANA character set name, for MIME parameters such as HTTP
    /// `Content-Type: ...; charset=iso-8859-1`.
    pub fn iana_name(self) -> String {
        match self {
            Self::Ascii => "us-ascii".to_string(),
            Self::Utf8 => "utf-8".to_string(),
            Self::Iso8859(number) => format!("iso-8859-{number}"),
        }
    }

    /// Parse an IANA character set name from a MIME `charset` parameter.
    ///
    /// Matching is case-insensitive and surrounding quotes are ignored, so
    /// both `charset=iso-8859-1` and `charset="ISO-8859-1"` work. Names
    /// outside the supported set are an error.
    pub fn from_iana(value: &str) -> Result<Self> {
        let unquoted = value.trim().trim_matches('"').trim();
        if unquoted.eq_ignore_ascii_case("us-ascii") || unquoted.eq_ignore_ascii_case("usascii") {
            return Ok(Self::Ascii);
        }
        Self::from_msh18(unquoted).map_err(|_| {
            Error::Decoding(format!(
                "unsupported IANA character set '{value}' ({SUPPORTED_SETS})"
            ))
        })
    }
}

impl Default for CharacterSet {
    /// UTF-8: preserves the historical behavior of accepting UTF-8 text when
    /// the sender declares no character set in MSH-18.
    fn default() -> Self {
        Self::Utf8
    }
}

impl FromStr for CharacterSet {
    type Err = Error;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_msh18(value)
    }
}

impl fmt::Display for CharacterSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msh18())
    }
}

/// Parse the numeric part of an `8859/N` style label. Returns `None` when the
/// label has no recognized 8859 prefix or the suffix is not a number.
fn parse_8859_number(normalized: &str) -> Option<u8> {
    for prefix in ["8859/", "8859-", "ISO-8859-", "ISO_8859-"] {
        if let Some(number) = normalized.strip_prefix(prefix) {
            return number.parse().ok();
        }
    }
    None
}

/// Map an 8859 part number to its `encoding_rs` encoding.
///
/// Part 1 resolves to windows-1252 per the WHATWG Encoding Standard, which
/// unifies ISO-8859-1 with windows-1252 (a strict superset that also maps
/// 0x80-0x9F to printable characters such as the euro sign).
fn single_byte_encoding(number: u8) -> Option<&'static encoding_rs::Encoding> {
    use encoding_rs::{
        ISO_8859_2, ISO_8859_3, ISO_8859_4, ISO_8859_5, ISO_8859_6, ISO_8859_7, ISO_8859_8,
        ISO_8859_10, ISO_8859_13, ISO_8859_14, ISO_8859_15, ISO_8859_16, WINDOWS_874, WINDOWS_1252,
        WINDOWS_1254,
    };
    match number {
        1 => Some(WINDOWS_1252),
        2 => Some(ISO_8859_2),
        3 => Some(ISO_8859_3),
        4 => Some(ISO_8859_4),
        5 => Some(ISO_8859_5),
        6 => Some(ISO_8859_6),
        7 => Some(ISO_8859_7),
        8 => Some(ISO_8859_8),
        9 => Some(WINDOWS_1254),
        10 => Some(ISO_8859_10),
        11 => Some(WINDOWS_874),
        13 => Some(ISO_8859_13),
        14 => Some(ISO_8859_14),
        15 => Some(ISO_8859_15),
        16 => Some(ISO_8859_16),
        _ => None,
    }
}

/// Error message for an 8859 part with no registered encoding.
fn unsupported_8859_message(number: u8) -> String {
    if number == 12 {
        "character set '8859/12' was withdrawn and has no registered encoding".to_string()
    } else {
        format!("unsupported character set '8859/{number}' ({SUPPORTED_SETS})")
    }
}

/// Strict 7-bit ASCII decode with the offending byte in the error.
fn decode_ascii(bytes: &[u8]) -> Result<String> {
    if let Some((offset, byte)) = bytes
        .iter()
        .enumerate()
        .find(|(_, byte)| !byte.is_ascii())
        .map(|(index, byte)| (index, *byte))
    {
        return Err(Error::Decoding(format!(
            "message bytes are not valid ASCII (byte 0x{byte:02X} at offset {offset}); declare the sender's character set in MSH-18"
        )));
    }
    // Every byte is ASCII, so this conversion cannot fail.
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(text.to_string()),
        Err(e) => Err(Error::Decoding(format!(
            "message bytes are not valid ASCII: {e}"
        ))),
    }
}

/// Extract the raw MSH-18 value from wire bytes without decoding them.
///
/// Only ASCII bytes are inspected: field separators below 0x80 can never
/// occur inside multibyte sequences, so the scan is safe in every supported
/// encoding. Returns `None` when the bytes do not start with an MSH segment
/// or MSH-18 is absent or non-ASCII.
fn extract_msh18(bytes: &[u8]) -> Option<&str> {
    let scanned = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let end = scanned
        .iter()
        .position(|&byte| byte == b'\r' || byte == b'\n')
        .unwrap_or(scanned.len());
    let header = &scanned[..end];
    if !header.starts_with(b"MSH") {
        return None;
    }
    // MSH-1 is the separator itself, stored at index 3.
    let separator = *header.get(3)?;
    let mut fields = header.split(|&byte| byte == separator);
    fields.next()?;
    // MSH-2 is now at index 0, so MSH-18 is at index 16.
    let raw = fields.nth(16)?;
    std::str::from_utf8(raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn msh18_labels_parse_to_expected_sets() {
        let cases = [
            ("ASCII", CharacterSet::Ascii),
            ("ascii", CharacterSet::Ascii),
            ("  ASCII  ", CharacterSet::Ascii),
            ("UNICODE UTF-8", CharacterSet::Utf8),
            ("unicode utf-8", CharacterSet::Utf8),
            ("UTF-8", CharacterSet::Utf8),
            ("utf8", CharacterSet::Utf8),
            ("8859/1", CharacterSet::Iso8859(1)),
            ("8859/15", CharacterSet::Iso8859(15)),
            ("8859-2", CharacterSet::Iso8859(2)),
            ("ISO-8859-3", CharacterSet::Iso8859(3)),
            ("iso_8859-4", CharacterSet::Iso8859(4)),
            ("WINDOWS-1252", CharacterSet::Iso8859(1)),
            // First repetition wins; empty means undeclared.
            ("8859/1~8859/2", CharacterSet::Iso8859(1)),
            ("", CharacterSet::Utf8),
            ("   ", CharacterSet::Utf8),
        ];
        for (label, expected) in cases {
            assert_eq!(
                CharacterSet::from_msh18(label).unwrap(),
                expected,
                "label {label:?}"
            );
        }
    }

    #[test]
    fn unsupported_msh18_labels_are_rejected() {
        for label in [
            "FOO-99",
            "8859/12",
            "8859/0",
            "8859/17",
            "UNICODE",
            "ISO IR87",
            "ISO IR159",
            "UTF-16",
        ] {
            assert!(
                CharacterSet::from_msh18(label).is_err(),
                "label {label:?} must be rejected"
            );
        }
        let withdrawn = CharacterSet::from_msh18("8859/12").unwrap_err();
        assert!(withdrawn.to_string().contains("withdrawn"));
        let utf16 = CharacterSet::from_msh18("UNICODE").unwrap_err();
        assert!(utf16.to_string().contains("UTF-16"));
    }

    #[test]
    fn canonical_msh18_values_round_trip() {
        let sets = [
            CharacterSet::Ascii,
            CharacterSet::Utf8,
            CharacterSet::Iso8859(1),
            CharacterSet::Iso8859(2),
            CharacterSet::Iso8859(5),
            CharacterSet::Iso8859(9),
            CharacterSet::Iso8859(11),
            CharacterSet::Iso8859(15),
            CharacterSet::Iso8859(16),
        ];
        for set in sets {
            assert_eq!(CharacterSet::from_msh18(&set.msh18()).unwrap(), set);
            assert_eq!(set.to_string(), set.msh18());
        }
        assert_eq!(CharacterSet::Ascii.msh18(), "ASCII");
        assert_eq!(CharacterSet::Utf8.msh18(), "UNICODE UTF-8");
        assert_eq!(CharacterSet::Iso8859(1).msh18(), "8859/1");
    }

    #[test]
    fn invalid_part_numbers_fail_conversion_without_panicking() {
        for number in [0, 12, 17, 99, 255] {
            let set = CharacterSet::Iso8859(number);
            assert!(set.encode("plain").is_err());
            assert!(set.decode(b"plain").is_err());
        }
    }

    #[test]
    fn single_byte_encodings_round_trip() {
        let cases = [
            (CharacterSet::Ascii, "Muller 123"),
            (CharacterSet::Utf8, "Müller 北京 €"),
            (CharacterSet::Iso8859(1), "Müller € naïve"),
            (CharacterSet::Iso8859(2), "Łódź Ąćę"),
            (CharacterSet::Iso8859(3), "Ħaż-Żmien"),
            (CharacterSet::Iso8859(4), "Ēķu Āč"),
            (CharacterSet::Iso8859(5), "АБВ абв"),
            (CharacterSet::Iso8859(6), "ا ب ت"),
            (CharacterSet::Iso8859(7), "αβγ ΑΒΓ"),
            (CharacterSet::Iso8859(8), "אבג"),
            (CharacterSet::Iso8859(9), "ğüşöçı İĞ"),
            (CharacterSet::Iso8859(10), "Āčē Đš"),
            (CharacterSet::Iso8859(11), "กขค"),
            (CharacterSet::Iso8859(13), "ąčę ĄČĘ"),
            (CharacterSet::Iso8859(14), "ḃċḋ ĊḊ"),
            (CharacterSet::Iso8859(15), "Müller € ŠšŽž"),
            (CharacterSet::Iso8859(16), "ȘșȚț Ăâ"),
        ];
        for (set, text) in cases {
            let bytes = set.encode(text).unwrap();
            assert_eq!(set.decode(&bytes).unwrap(), text, "charset {set}");
        }
    }

    #[test]
    fn known_characters_map_to_expected_bytes() {
        // Spot checks cross-verified against Python's codecs module.
        let cases = [
            (CharacterSet::Iso8859(1), "é", vec![0xE9]),
            (CharacterSet::Iso8859(1), "€", vec![0x80]),
            (
                CharacterSet::Iso8859(2),
                "Łódź",
                vec![0xA3, 0xF3, 0x64, 0xBC],
            ),
            (CharacterSet::Iso8859(2), "Ą", vec![0xA1]),
            (CharacterSet::Iso8859(3), "é", vec![0xE9]),
            (CharacterSet::Iso8859(4), "é", vec![0xE9]),
            (CharacterSet::Iso8859(5), "А", vec![0xB0]),
            (CharacterSet::Iso8859(6), "ا", vec![0xC7]),
            (CharacterSet::Iso8859(7), "α", vec![0xE1]),
            (CharacterSet::Iso8859(8), "א", vec![0xE0]),
            (CharacterSet::Iso8859(9), "é", vec![0xE9]),
            (CharacterSet::Iso8859(10), "é", vec![0xE9]),
            (CharacterSet::Iso8859(11), "ก", vec![0xA1]),
            (CharacterSet::Iso8859(13), "é", vec![0xE9]),
            (CharacterSet::Iso8859(14), "é", vec![0xE9]),
            (CharacterSet::Iso8859(15), "€", vec![0xA4]),
            (CharacterSet::Iso8859(16), "é", vec![0xE9]),
            (CharacterSet::Utf8, "é", vec![0xC3, 0xA9]),
        ];
        for (set, text, expected) in cases {
            assert_eq!(set.encode(text).unwrap(), expected, "charset {set}");
            assert_eq!(set.decode(&expected).unwrap(), text, "charset {set}");
        }
    }

    #[test]
    fn unmappable_characters_are_errors_not_replacements() {
        let ascii = CharacterSet::Ascii.encode("Müller").unwrap_err();
        assert!(ascii.to_string().contains('ü'));
        let latin2 = CharacterSet::Iso8859(2).encode("price: €").unwrap_err();
        assert!(latin2.to_string().contains('€'));
        let latin1 = CharacterSet::Iso8859(1).encode("北京").unwrap_err();
        assert!(latin1.to_string().contains("U+5317"));
        // No silent numeric character references may leak into output.
        assert!(CharacterSet::Iso8859(2).encode("€").is_err());
    }

    #[test]
    fn malformed_bytes_are_errors_not_replacements() {
        let ascii = CharacterSet::Ascii.decode(b"abc\xE9").unwrap_err();
        assert!(ascii.to_string().contains("0xE9"));
        let utf8 = CharacterSet::Utf8.decode(b"abc\xE9").unwrap_err();
        assert!(utf8.to_string().contains("UTF-8"));
        assert!(CharacterSet::Utf8.decode(b"\xFF\xFE").is_err());
        // Stray UTF-8 continuation bytes stay invalid even as ASCII.
        assert!(CharacterSet::Ascii.decode("é".as_bytes()).is_err());
    }

    #[test]
    fn detect_reads_msh18_from_wire_bytes() {
        let latin1 =
            b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1";
        assert_eq!(
            CharacterSet::detect(latin1).unwrap(),
            CharacterSet::Iso8859(1)
        );
        let utf8 = b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||UNICODE UTF-8\rPID|1";
        assert_eq!(CharacterSet::detect(utf8).unwrap(), CharacterSet::Utf8);
        // Empty or missing MSH-18 falls back to the UTF-8 default.
        let empty = b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1\rPID|1";
        assert_eq!(CharacterSet::detect(empty).unwrap(), CharacterSet::Utf8);
        let short = b"MSH|^~\\&|SND\rPID|1";
        assert_eq!(CharacterSet::detect(short).unwrap(), CharacterSet::Utf8);
        let not_msh = b"PID|1||12345\r";
        assert_eq!(CharacterSet::detect(not_msh).unwrap(), CharacterSet::Utf8);
    }

    #[test]
    fn detect_honors_custom_field_separators() {
        let wire =
            b"MSH#^~\\&#SND#FAC#RCV#RFAC#20240115120000##ADT^A01#1#P#2.5.1######8859/2\rPID#1";
        assert_eq!(
            CharacterSet::detect(wire).unwrap(),
            CharacterSet::Iso8859(2)
        );
    }

    #[test]
    fn detect_rejects_utf16_byte_order_marks() {
        for bom in [&[0xFF, 0xFE][..], &[0xFE, 0xFF][..]] {
            let mut wire = bom.to_vec();
            wire.extend_from_slice(b"MSH|^~\\&|SND\r");
            let error = CharacterSet::detect(&wire).unwrap_err();
            assert!(error.to_string().contains("UTF-16"));
        }
    }

    #[test]
    fn decode_message_tolerates_utf8_byte_order_mark() {
        let mut wire = vec![0xEF, 0xBB, 0xBF];
        wire.extend_from_slice(
            b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1\rPID|1",
        );
        let text = CharacterSet::decode_message(&wire).unwrap();
        assert!(text.starts_with("MSH|"));
        // The declared set still wins when a BOM is present.
        let mut latin1 = vec![0xEF, 0xBB, 0xBF];
        latin1.extend_from_slice(
            b"MSH|^~\\&|SND|FAC|RCV|RFAC|20240115120000||ADT^A01|1|P|2.5.1||||||8859/1\rPID|1",
        );
        assert_eq!(
            CharacterSet::detect(&latin1).unwrap(),
            CharacterSet::Iso8859(1)
        );
    }

    #[test]
    fn default_character_set_is_utf8() {
        assert_eq!(CharacterSet::default(), CharacterSet::Utf8);
    }

    #[test]
    fn iana_names_parse_and_round_trip() {
        let cases = [
            ("us-ascii", CharacterSet::Ascii),
            ("US-ASCII", CharacterSet::Ascii),
            ("utf-8", CharacterSet::Utf8),
            ("UTF-8", CharacterSet::Utf8),
            ("iso-8859-1", CharacterSet::Iso8859(1)),
            ("\"ISO-8859-15\"", CharacterSet::Iso8859(15)),
            ("windows-1252", CharacterSet::Iso8859(1)),
        ];
        for (name, expected) in cases {
            assert_eq!(CharacterSet::from_iana(name).unwrap(), expected);
        }
        assert!(CharacterSet::from_iana("gbk").is_err());
        assert!(CharacterSet::from_iana("utf-16").is_err());

        assert_eq!(CharacterSet::Ascii.iana_name(), "us-ascii");
        assert_eq!(CharacterSet::Utf8.iana_name(), "utf-8");
        assert_eq!(CharacterSet::Iso8859(1).iana_name(), "iso-8859-1");
        for set in [
            CharacterSet::Ascii,
            CharacterSet::Utf8,
            CharacterSet::Iso8859(2),
        ] {
            assert_eq!(CharacterSet::from_iana(&set.iana_name()).unwrap(), set);
        }
    }
}
