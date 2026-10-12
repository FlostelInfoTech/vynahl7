//! HL7 escape sequence encoding and decoding

use crate::delimiters::Delimiters;
use crate::error::{Error, Result};

/// HL7 escape sequences
///
/// HL7 uses escape sequences to represent special characters:
/// - `\F\` - Field separator
/// - `\S\` - Component separator
/// - `\T\` - Subcomponent separator
/// - `\R\` - Repetition separator
/// - `\E\` - Escape character
/// - `\Xnn\` - Hexadecimal character (e.g., \X0D\ for carriage return)
/// - `\Znn...nn\` - Locally defined escape sequence
/// - `\.br\` - Line break (formatting)
/// - `\H\` and `\N\` - Highlight on/off (formatting)
pub struct Encoding;

impl Encoding {
    /// Encode a string by replacing special characters with escape sequences
    pub fn encode(input: &str, delimiters: &Delimiters) -> String {
        let mut result = String::with_capacity(input.len());

        for ch in input.chars() {
            if ch == delimiters.escape_character {
                result.push(delimiters.escape_character);
                result.push('E');
                result.push(delimiters.escape_character);
            } else if ch == delimiters.field_separator {
                result.push(delimiters.escape_character);
                result.push('F');
                result.push(delimiters.escape_character);
            } else if ch == delimiters.component_separator {
                result.push(delimiters.escape_character);
                result.push('S');
                result.push(delimiters.escape_character);
            } else if ch == delimiters.subcomponent_separator {
                result.push(delimiters.escape_character);
                result.push('T');
                result.push(delimiters.escape_character);
            } else if ch == delimiters.repetition_separator {
                result.push(delimiters.escape_character);
                result.push('R');
                result.push(delimiters.escape_character);
            } else if ch == '\r' {
                result.push(delimiters.escape_character);
                result.push_str("X0D");
                result.push(delimiters.escape_character);
            } else if ch == '\n' {
                result.push(delimiters.escape_character);
                result.push_str("X0A");
                result.push(delimiters.escape_character);
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Decode a string by replacing escape sequences with their actual characters
    pub fn decode(input: &str, delimiters: &Delimiters) -> Result<String> {
        // Fast path: no escape character means nothing to decode
        if !input.contains(delimiters.escape_character) {
            return Ok(input.to_string());
        }

        let mut result = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == delimiters.escape_character {
                // Look for the escape sequence
                let mut escape_seq = String::new();

                while let Some(&next_ch) = chars.peek() {
                    if next_ch == delimiters.escape_character {
                        chars.next(); // consume the closing escape character
                        break;
                    }
                    escape_seq.push(next_ch);
                    chars.next();
                }

                // Decode the escape sequence
                match escape_seq.as_str() {
                    "F" => result.push(delimiters.field_separator),
                    "S" => result.push(delimiters.component_separator),
                    "T" => result.push(delimiters.subcomponent_separator),
                    "R" => result.push(delimiters.repetition_separator),
                    "E" => result.push(delimiters.escape_character),
                    ".br" => result.push('\n'),
                    "H" => {}, // Highlight on - formatting, ignored
                    "N" => {}, // Highlight off - formatting, ignored
                    seq if seq.starts_with('X') => {
                        // Hexadecimal character
                        let hex = &seq[1..];
                        if let Ok(code) = u32::from_str_radix(hex, 16) {
                            if let Some(decoded_ch) = char::from_u32(code) {
                                result.push(decoded_ch);
                            } else {
                                return Err(Error::Decoding(format!(
                                    "Invalid hexadecimal escape sequence: \\{}\\",
                                    seq
                                )));
                            }
                        } else {
                            return Err(Error::Decoding(format!(
                                "Invalid hexadecimal escape sequence: \\{}\\",
                                seq
                            )));
                        }
                    }
                    seq if seq.starts_with('Z') => {
                        // Locally defined escape - preserve as-is
                        result.push(delimiters.escape_character);
                        result.push_str(seq);
                        result.push(delimiters.escape_character);
                    }
                    _ => {
                        return Err(Error::Decoding(format!(
                            "Unknown escape sequence: \\{}\\",
                            escape_seq
                        )));
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_delimiters() {
        let delims = Delimiters::default();

        assert_eq!(Encoding::encode("|", &delims), "\\F\\");
        assert_eq!(Encoding::encode("^", &delims), "\\S\\");
        assert_eq!(Encoding::encode("&", &delims), "\\T\\");
        assert_eq!(Encoding::encode("~", &delims), "\\R\\");
        assert_eq!(Encoding::encode("\\", &delims), "\\E\\");
    }

    #[test]
    fn test_encode_mixed() {
        let delims = Delimiters::default();
        assert_eq!(
            Encoding::encode("Test|Value^Component", &delims),
            "Test\\F\\Value\\S\\Component"
        );
    }

    #[test]
    fn test_encode_newlines() {
        let delims = Delimiters::default();
        assert_eq!(Encoding::encode("\r\n", &delims), "\\X0D\\\\X0A\\");
    }

    #[test]
    fn test_decode_delimiters() {
        let delims = Delimiters::default();

        assert_eq!(Encoding::decode("\\F\\", &delims).unwrap(), "|");
        assert_eq!(Encoding::decode("\\S\\", &delims).unwrap(), "^");
        assert_eq!(Encoding::decode("\\T\\", &delims).unwrap(), "&");
        assert_eq!(Encoding::decode("\\R\\", &delims).unwrap(), "~");
        assert_eq!(Encoding::decode("\\E\\", &delims).unwrap(), "\\");
    }

    #[test]
    fn test_decode_hexadecimal() {
        let delims = Delimiters::default();
        assert_eq!(Encoding::decode("\\X0D\\", &delims).unwrap(), "\r");
        assert_eq!(Encoding::decode("\\X0A\\", &delims).unwrap(), "\n");
        assert_eq!(Encoding::decode("\\X20\\", &delims).unwrap(), " ");
    }

    #[test]
    fn test_decode_line_break() {
        let delims = Delimiters::default();
        assert_eq!(Encoding::decode("\\.br\\", &delims).unwrap(), "\n");
    }

    #[test]
    fn test_decode_highlight() {
        let delims = Delimiters::default();
        // Highlighting sequences are ignored
        assert_eq!(Encoding::decode("\\H\\bold\\N\\", &delims).unwrap(), "bold");
    }

    #[test]
    fn test_roundtrip() {
        let delims = Delimiters::default();
        let original = "Test|Value^Component&Sub~Rep\\Escape";
        let encoded = Encoding::encode(original, &delims);
        let decoded = Encoding::decode(&encoded, &delims).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_invalid_escape_sequence() {
        let delims = Delimiters::default();
        let result = Encoding::decode("\\INVALID\\", &delims);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hex_sequence() {
        let delims = Delimiters::default();
        let result = Encoding::decode("\\XZZ\\", &delims);
        assert!(result.is_err());
    }
}
