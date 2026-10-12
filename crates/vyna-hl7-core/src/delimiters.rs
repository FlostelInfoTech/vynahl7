//! HL7 delimiter and encoding character handling

use crate::error::{Error, Result};

/// Default HL7 delimiters as defined in the standard
pub const DEFAULT_FIELD_SEPARATOR: char = '|';
pub const DEFAULT_COMPONENT_SEPARATOR: char = '^';
pub const DEFAULT_REPETITION_SEPARATOR: char = '~';
pub const DEFAULT_ESCAPE_CHARACTER: char = '\\';
pub const DEFAULT_SUBCOMPONENT_SEPARATOR: char = '&';

/// HL7 message delimiters and encoding characters
///
/// These characters define how HL7 messages are structured:
/// - Field separator: `|` (separates fields in a segment)
/// - Component separator: `^` (separates components within a field)
/// - Repetition separator: `~` (separates repeated fields)
/// - Escape character: `\` (used for escape sequences)
/// - Subcomponent separator: `&` (separates subcomponents within a component)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delimiters {
    pub field_separator: char,
    pub component_separator: char,
    pub repetition_separator: char,
    pub escape_character: char,
    pub subcomponent_separator: char,
}

impl Default for Delimiters {
    fn default() -> Self {
        Self {
            field_separator: DEFAULT_FIELD_SEPARATOR,
            component_separator: DEFAULT_COMPONENT_SEPARATOR,
            repetition_separator: DEFAULT_REPETITION_SEPARATOR,
            escape_character: DEFAULT_ESCAPE_CHARACTER,
            subcomponent_separator: DEFAULT_SUBCOMPONENT_SEPARATOR,
        }
    }
}

impl Delimiters {
    /// Create new delimiters with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create delimiters from MSH-2 encoding characters
    ///
    /// MSH-2 contains 4 characters in order:
    /// 1. Component separator (^)
    /// 2. Repetition separator (~)
    /// 3. Escape character (\)
    /// 4. Subcomponent separator (&)
    ///
    /// The field separator is always from MSH-1 (typically |)
    pub fn from_encoding_characters(field_sep: char, encoding_chars: &str) -> Result<Self> {
        if encoding_chars.len() != 4 {
            return Err(Error::InvalidDelimiters(format!(
                "Encoding characters must be exactly 4 characters, got {}",
                encoding_chars.len()
            )));
        }

        let chars: Vec<char> = encoding_chars.chars().collect();

        // Validate that all delimiters are unique
        let delims = Self {
            field_separator: field_sep,
            component_separator: chars[0],
            repetition_separator: chars[1],
            escape_character: chars[2],
            subcomponent_separator: chars[3],
        };

        delims.validate()?;
        Ok(delims)
    }

    /// Get encoding characters as a string (MSH-2 format)
    pub fn encoding_characters(&self) -> String {
        format!(
            "{}{}{}{}",
            self.component_separator,
            self.repetition_separator,
            self.escape_character,
            self.subcomponent_separator
        )
    }

    /// Validate that all delimiters are unique
    pub fn validate(&self) -> Result<()> {
        let chars = [self.field_separator,
            self.component_separator,
            self.repetition_separator,
            self.escape_character,
            self.subcomponent_separator];

        for (i, &c1) in chars.iter().enumerate() {
            for (j, &c2) in chars.iter().enumerate() {
                if i != j && c1 == c2 {
                    return Err(Error::InvalidDelimiters(format!(
                        "Duplicate delimiter character: '{}'",
                        c1
                    )));
                }
            }
        }

        Ok(())
    }

    /// Check if a character is a delimiter
    pub fn is_delimiter(&self, c: char) -> bool {
        c == self.field_separator
            || c == self.component_separator
            || c == self.repetition_separator
            || c == self.subcomponent_separator
    }

    /// Check if a character is the escape character
    pub fn is_escape(&self, c: char) -> bool {
        c == self.escape_character
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_delimiters() {
        let delims = Delimiters::default();
        assert_eq!(delims.field_separator, '|');
        assert_eq!(delims.component_separator, '^');
        assert_eq!(delims.repetition_separator, '~');
        assert_eq!(delims.escape_character, '\\');
        assert_eq!(delims.subcomponent_separator, '&');
    }

    #[test]
    fn test_from_encoding_characters() {
        let delims = Delimiters::from_encoding_characters('|', "^~\\&").unwrap();
        assert_eq!(delims.field_separator, '|');
        assert_eq!(delims.component_separator, '^');
        assert_eq!(delims.repetition_separator, '~');
        assert_eq!(delims.escape_character, '\\');
        assert_eq!(delims.subcomponent_separator, '&');
    }

    #[test]
    fn test_encoding_characters() {
        let delims = Delimiters::default();
        assert_eq!(delims.encoding_characters(), "^~\\&");
    }

    #[test]
    fn test_invalid_length() {
        let result = Delimiters::from_encoding_characters('|', "^~\\");
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_delimiters() {
        let result = Delimiters::from_encoding_characters('|', "^^^^");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_delimiter() {
        let delims = Delimiters::default();
        assert!(delims.is_delimiter('|'));
        assert!(delims.is_delimiter('^'));
        assert!(delims.is_delimiter('~'));
        assert!(delims.is_delimiter('&'));
        assert!(!delims.is_delimiter('\\'));
        assert!(!delims.is_delimiter('A'));
    }

    #[test]
    fn test_is_escape() {
        let delims = Delimiters::default();
        assert!(delims.is_escape('\\'));
        assert!(!delims.is_escape('|'));
        assert!(!delims.is_escape('A'));
    }
}
