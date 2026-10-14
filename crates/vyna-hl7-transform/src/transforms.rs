//! Built-in transformation functions
//!
//! This module provides common transformation functions that can be used
//! with transformation rules.

use crate::error::{Error, Result};
use crate::rule::TransformContext;
use chrono::NaiveDateTime;

/// Convert value to uppercase
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new();
/// let result = transforms::uppercase("smith", &ctx).unwrap();
/// assert_eq!(result, "SMITH");
/// ```
pub fn uppercase(value: &str, _ctx: &TransformContext) -> Result<String> {
    Ok(value.to_uppercase())
}

/// Convert value to lowercase
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new();
/// let result = transforms::lowercase("SMITH", &ctx).unwrap();
/// assert_eq!(result, "smith");
/// ```
pub fn lowercase(value: &str, _ctx: &TransformContext) -> Result<String> {
    Ok(value.to_lowercase())
}

/// Trim whitespace from both ends of the value
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new();
/// let result = transforms::trim("  SMITH  ", &ctx).unwrap();
/// assert_eq!(result, "SMITH");
/// ```
pub fn trim(value: &str, _ctx: &TransformContext) -> Result<String> {
    Ok(value.trim().to_string())
}

/// Trim leading whitespace
pub fn trim_start(value: &str, _ctx: &TransformContext) -> Result<String> {
    Ok(value.trim_start().to_string())
}

/// Trim trailing whitespace
pub fn trim_end(value: &str, _ctx: &TransformContext) -> Result<String> {
    Ok(value.trim_end().to_string())
}

/// Remove all whitespace from the value
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new();
/// let result = transforms::remove_whitespace("S M I T H", &ctx).unwrap();
/// assert_eq!(result, "SMITH");
/// ```
pub fn remove_whitespace(value: &str, _ctx: &TransformContext) -> Result<String> {
    Ok(value.chars().filter(|c| !c.is_whitespace()).collect())
}

/// Get a substring of the value
///
/// Note: This function expects the context to contain "start" and optionally "length" keys.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("start".to_string(), "0".to_string())
///     .add_data("length".to_string(), "3".to_string());
///
/// let result = transforms::substring("SMITH", &ctx).unwrap();
/// assert_eq!(result, "SMI");
/// ```
pub fn substring(value: &str, ctx: &TransformContext) -> Result<String> {
    let raw_start = ctx
        .get_data("start")
        .ok_or_else(|| Error::transform_fn("substring requires 'start' in context"))?;
    let start = raw_start
        .parse::<usize>()
        .map_err(|_| Error::transform_fn("substring requires 'start' to be a number"))?;

    let length = match ctx.get_data("length") {
        None => None,
        Some(raw) => Some(raw.parse::<usize>().map_err(|_| {
            Error::transform_fn("substring requires 'length' to be a number when present")
        })?),
    };

    let chars: Vec<char> = value.chars().collect();

    if start >= chars.len() {
        return Ok(String::new());
    }

    let end = if let Some(len) = length {
        std::cmp::min(start.saturating_add(len), chars.len())
    } else {
        chars.len()
    };

    Ok(chars[start..end].iter().collect())
}

/// Convert HL7 date format (YYYYMMDD) to another format
///
/// Note: This function expects the context to contain a "format" key.
/// Supported formats: "YYYY-MM-DD", "MM/DD/YYYY", "DD/MM/YYYY", "YYYYMMDD"
///
/// Documented time tails (HH, HHMM, HHMMSS) are accepted but must hold
/// valid clock readings; out-of-range tails are rejected like
/// [`format_datetime`] rejects them.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("format".to_string(), "YYYY-MM-DD".to_string());
///
/// let result = transforms::format_date("20240315", &ctx).unwrap();
/// assert_eq!(result, "2024-03-15");
/// ```
pub fn format_date(value: &str, ctx: &TransformContext) -> Result<String> {
    let format = ctx
        .get_data("format")
        .ok_or_else(|| Error::transform_fn("format_date requires 'format' in context"))?;

    // Parse HL7 date (YYYYMMDD or YYYYMMDDHHMM or YYYYMMDDHHMMSS).
    // .get() keeps non-ASCII input from panicking on a split boundary.
    if value.len() < 8 {
        return Err(Error::transform_fn(
            "Date must be at least 8 characters (YYYYMMDD)",
        ));
    }
    let head = value
        .get(..8)
        .ok_or_else(|| Error::transform_fn(format!("Invalid date format: {value}")))?;
    if !head.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::transform_fn(format!("Invalid date format: {value}")));
    }
    // Whole-value validation like `format_datetime`: only documented
    // numeric time tails (HH, HHMM, HHMMSS) may follow the date, so
    // trailing garbage can never be silently dropped. Byte 8 is a char
    // boundary (the `.get(..8)` above succeeded), so slicing is safe.
    let tail = &value[8..];
    if !tail.bytes().all(|b| b.is_ascii_digit()) || !matches!(tail.len(), 0 | 2 | 4 | 6) {
        return Err(Error::transform_fn(format!("Invalid date format: {value}")));
    }
    // Time tails must hold valid clock readings like `format_datetime`:
    // an out-of-range tail (hour 99, minute 60) is rejected instead of
    // silently dropped with the date part returned. Slicing is safe:
    // the tail is all ASCII digits, so every byte is a char boundary.
    if !tail.is_empty() {
        // Byte math is total here: every byte is an ASCII digit and the
        // length is 2, 4, or 6, so no index or subtraction can fail.
        let bytes = tail.as_bytes();
        let two = |i: usize| (bytes[i] - b'0') as u32 * 10 + (bytes[i + 1] - b'0') as u32;
        let hour = two(0);
        let minute = if tail.len() >= 4 { two(2) } else { 0 };
        let second = if tail.len() == 6 { two(4) } else { 0 };
        if chrono::NaiveTime::from_hms_opt(hour, minute, second).is_none() {
            return Err(Error::transform_fn(format!("Invalid date format: {value}")));
        }
    }
    let date = chrono::NaiveDate::parse_from_str(head, "%Y%m%d")
        .map_err(|e| Error::transform_fn(format!("Invalid date format: {}", e)))?;

    // Format according to requested format
    let result = match format.as_str() {
        "YYYY-MM-DD" => date.format("%Y-%m-%d").to_string(),
        "MM/DD/YYYY" => date.format("%m/%d/%Y").to_string(),
        "DD/MM/YYYY" => date.format("%d/%m/%Y").to_string(),
        "YYYYMMDD" => date.format("%Y%m%d").to_string(),
        _ => {
            return Err(Error::transform_fn(format!(
                "Unsupported date format: {}",
                format
            )));
        }
    };

    Ok(result)
}

/// Convert HL7 datetime format to another format
///
/// Note: This function expects the context to contain a "format" key.
/// Supported formats: "YYYY-MM-DD HH:MM:SS", "ISO8601", "YYYYMMDDHHMMSS"
///
/// HL7 DTM carries variable precision: date (8 digits), hour (10), minute
/// (12), seconds (14). Shorter, ragged (9/11/13), overlong, or non-numeric
/// values are rejected, as are trailing fractional seconds or timezone
/// offsets, which this function does not interpret.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("format".to_string(), "YYYY-MM-DD HH:MM:SS".to_string());
///
/// let result = transforms::format_datetime("20240315123045", &ctx).unwrap();
/// assert_eq!(result, "2024-03-15 12:30:45");
/// ```
pub fn format_datetime(value: &str, ctx: &TransformContext) -> Result<String> {
    let format = ctx
        .get_data("format")
        .ok_or_else(|| Error::transform_fn("format_datetime requires 'format' in context"))?;

    // Whole-value validation (no slicing, so non-ASCII input cannot panic
    // on a split boundary): every precision band parses its own time
    // fields, and anything past them is trailing garbage, rejected
    // instead of silently truncated.
    if value.len() < 8 {
        return Err(Error::transform_fn(
            "Datetime must be at least 8 characters (YYYYMMDD)",
        ));
    }
    if !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::transform_fn(format!(
            "Invalid datetime format: {value}"
        )));
    }
    let datetime = match value.len() {
        8 => {
            // Date only, default time to 00:00:00
            let date = chrono::NaiveDate::parse_from_str(value, "%Y%m%d")
                .map_err(|e| Error::transform_fn(format!("Invalid date format: {}", e)))?;
            date.and_hms_opt(0, 0, 0)
                .ok_or_else(|| Error::transform_fn("Failed to create datetime"))?
        }
        // Hour/minute precision: chrono cannot build a NaiveDateTime
        // from a bare "%Y%m%d%H", so the date and time fields parse
        // separately (slicing is safe: the value is all ASCII digits).
        10 => {
            let date = chrono::NaiveDate::parse_from_str(&value[..8], "%Y%m%d")
                .map_err(|e| Error::transform_fn(format!("Invalid date format: {}", e)))?;
            let hour: u32 = value[8..10]
                .parse()
                .map_err(|e| Error::transform_fn(format!("Invalid datetime format: {e}")))?;
            date.and_hms_opt(hour, 0, 0)
                .ok_or_else(|| Error::transform_fn(format!("Invalid datetime format: {value}")))?
        }
        12 => {
            let date = chrono::NaiveDate::parse_from_str(&value[..8], "%Y%m%d")
                .map_err(|e| Error::transform_fn(format!("Invalid date format: {}", e)))?;
            let hour: u32 = value[8..10]
                .parse()
                .map_err(|e| Error::transform_fn(format!("Invalid datetime format: {e}")))?;
            let minute: u32 = value[10..12]
                .parse()
                .map_err(|e| Error::transform_fn(format!("Invalid datetime format: {e}")))?;
            date.and_hms_opt(hour, minute, 0)
                .ok_or_else(|| Error::transform_fn(format!("Invalid datetime format: {value}")))?
        }
        14 => NaiveDateTime::parse_from_str(value, "%Y%m%d%H%M%S")
            .map_err(|e| Error::transform_fn(format!("Invalid datetime format: {}", e)))?,
        _ => {
            return Err(Error::transform_fn(format!(
                "Invalid datetime format: {value}"
            )));
        }
    };

    // Format according to requested format
    let result = match format.as_str() {
        "YYYY-MM-DD HH:MM:SS" => datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
        "ISO8601" => datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
        "YYYYMMDDHHMMSS" => datetime.format("%Y%m%d%H%M%S").to_string(),
        _ => {
            return Err(Error::transform_fn(format!(
                "Unsupported datetime format: {}",
                format
            )));
        }
    };

    Ok(result)
}

/// Replace all occurrences of a pattern with a replacement string
///
/// Note: This function expects the context to contain "pattern" and "replacement" keys.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("pattern".to_string(), " ".to_string())
///     .add_data("replacement".to_string(), "-".to_string());
///
/// let result = transforms::replace("JOHN DOE", &ctx).unwrap();
/// assert_eq!(result, "JOHN-DOE");
/// ```
pub fn replace(value: &str, ctx: &TransformContext) -> Result<String> {
    let pattern = ctx
        .get_data("pattern")
        .ok_or_else(|| Error::transform_fn("replace requires 'pattern' in context"))?;
    let replacement = ctx
        .get_data("replacement")
        .ok_or_else(|| Error::transform_fn("replace requires 'replacement' in context"))?;

    Ok(value.replace(pattern, replacement))
}

/// Replace using a regular expression
///
/// Note: This function expects the context to contain "regex" and "replacement" keys.
///
/// The replacement string follows the `regex` crate's expansion syntax:
/// `$1` inserts the first capture group, `$name` a named group, and `$$`
/// a literal dollar sign. An unescaped `$N` with no such group expands
/// to empty, so spell a literal dollar before digits as `$$` (`"$$5"`
/// writes "$5", while `"$5"` writes nothing). For a fully literal
/// replacement with no expansion, use [`replace`] instead.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("regex".to_string(), r"\d+".to_string())
///     .add_data("replacement".to_string(), "XXX".to_string());
///
/// let result = transforms::regex_replace("ID-12345", &ctx).unwrap();
/// assert_eq!(result, "ID-XXX");
/// ```
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// // Capture groups reorder; $$ escapes a literal dollar.
/// let ctx = TransformContext::new()
///     .add_data("regex".to_string(), r"(\w+)-(\d+)".to_string())
///     .add_data("replacement".to_string(), "$2-$1".to_string());
/// assert_eq!(transforms::regex_replace("ID-12345", &ctx).unwrap(), "12345-ID");
///
/// let ctx = TransformContext::new()
///     .add_data("regex".to_string(), r"\d+".to_string())
///     .add_data("replacement".to_string(), "$$5".to_string());
/// assert_eq!(transforms::regex_replace("ID-12345", &ctx).unwrap(), "ID-$5");
/// ```
pub fn regex_replace(value: &str, ctx: &TransformContext) -> Result<String> {
    let pattern = ctx
        .get_data("regex")
        .ok_or_else(|| Error::transform_fn("regex_replace requires 'regex' in context"))?;
    let replacement = ctx
        .get_data("replacement")
        .ok_or_else(|| Error::transform_fn("regex_replace requires 'replacement' in context"))?;

    let re = regex::Regex::new(pattern)
        .map_err(|e| Error::transform_fn(format!("Invalid regex: {}", e)))?;

    Ok(re.replace_all(value, replacement.as_str()).to_string())
}

/// Concatenate with a prefix
///
/// Note: This function expects the context to contain a "prefix" key.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("prefix".to_string(), "MRN-".to_string());
///
/// let result = transforms::prefix("12345", &ctx).unwrap();
/// assert_eq!(result, "MRN-12345");
/// ```
pub fn prefix(value: &str, ctx: &TransformContext) -> Result<String> {
    let prefix = ctx
        .get_data("prefix")
        .ok_or_else(|| Error::transform_fn("prefix requires 'prefix' in context"))?;

    Ok(format!("{}{}", prefix, value))
}

/// Concatenate with a suffix
///
/// Note: This function expects the context to contain a "suffix" key.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("suffix".to_string(), "@hospital.com".to_string());
///
/// let result = transforms::suffix("john.doe", &ctx).unwrap();
/// assert_eq!(result, "john.doe@hospital.com");
/// ```
pub fn suffix(value: &str, ctx: &TransformContext) -> Result<String> {
    let suffix = ctx
        .get_data("suffix")
        .ok_or_else(|| Error::transform_fn("suffix requires 'suffix' in context"))?;

    Ok(format!("{}{}", value, suffix))
}

/// Maximum padded length accepted by [`pad`].
///
/// The target length comes from caller-controlled context data, so an
/// unbounded value would allocate without limit. One million characters
/// is far beyond any real HL7 field while keeping one call under ~4 MB.
pub const MAX_PAD_LENGTH: usize = 1_000_000;

/// Pad the value to a specified length with a character
///
/// Note: This function expects the context to contain "length" and "pad_char" keys, and
/// optionally "side" ("left" or "right"). When "side" is absent it defaults to "left".
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("length".to_string(), "5".to_string())
///     .add_data("pad_char".to_string(), "0".to_string())
///     .add_data("side".to_string(), "left".to_string());
///
/// let result = transforms::pad("123", &ctx).unwrap();
/// assert_eq!(result, "00123");
/// ```
pub fn pad(value: &str, ctx: &TransformContext) -> Result<String> {
    let raw_length = ctx
        .get_data("length")
        .ok_or_else(|| Error::transform_fn("pad requires 'length' in context"))?;
    let length = raw_length
        .parse::<usize>()
        .map_err(|_| Error::transform_fn("pad requires 'length' to be a number"))?;
    if length > MAX_PAD_LENGTH {
        return Err(Error::transform_fn(format!(
            "pad length {length} exceeds maximum {MAX_PAD_LENGTH}"
        )));
    }

    let pad_char = ctx
        .get_data("pad_char")
        .and_then(|s| s.chars().next())
        .ok_or_else(|| Error::transform_fn("pad requires 'pad_char' in context"))?;

    let side = ctx.get_data("side").map(|s| s.as_str()).unwrap_or("left");
    // Validate before the length short-circuit: otherwise a config error
    // hides behind any value already long enough to skip padding.
    if !matches!(side, "left" | "right") {
        return Err(Error::transform_fn(format!("Invalid pad side: {}", side)));
    }

    // Compare in characters, not bytes: padding appends `char`s, so a
    // byte-length comparison under-pads multi-byte values.
    let value_chars = value.chars().count();
    if value_chars >= length {
        return Ok(value.to_string());
    }

    let padding = std::iter::repeat_n(pad_char, length - value_chars).collect::<String>();

    match side {
        "left" => Ok(format!("{}{}", padding, value)),
        "right" => Ok(format!("{}{}", value, padding)),
        _ => Err(Error::transform_fn(format!("Invalid pad side: {}", side))),
    }
}

/// Return a default value if the input is empty
///
/// Note: This function expects the context to contain a "default" key.
///
/// When used as a [`TransformationRule`](crate::rule::TransformationRule)
/// transform, the rule must set `skip_if_empty(false)`: with the default
/// `skip_if_empty(true)` the rule returns its own default (or nothing)
/// for empty values without ever calling this function, so the default
/// here would silently never apply. For plain defaults prefer
/// [`with_default`](crate::rule::TransformationRule::with_default),
/// which applies under the default skip setting.
///
/// # Examples
///
/// ```rust
/// use vyna_hl7_transform::{transforms, rule::TransformContext};
///
/// let ctx = TransformContext::new()
///     .add_data("default".to_string(), "UNKNOWN".to_string());
///
/// let result = transforms::default_if_empty("", &ctx).unwrap();
/// assert_eq!(result, "UNKNOWN");
/// ```
pub fn default_if_empty(value: &str, ctx: &TransformContext) -> Result<String> {
    if value.is_empty() {
        let default = ctx
            .get_data("default")
            .ok_or_else(|| Error::transform_fn("default_if_empty requires 'default' in context"))?;
        Ok(default.clone())
    } else {
        Ok(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uppercase() {
        let ctx = TransformContext::new();
        assert_eq!(uppercase("smith", &ctx).unwrap(), "SMITH");
        assert_eq!(uppercase("SMITH", &ctx).unwrap(), "SMITH");
    }

    #[test]
    fn test_lowercase() {
        let ctx = TransformContext::new();
        assert_eq!(lowercase("SMITH", &ctx).unwrap(), "smith");
        assert_eq!(lowercase("smith", &ctx).unwrap(), "smith");
    }

    #[test]
    fn test_trim() {
        let ctx = TransformContext::new();
        assert_eq!(trim("  SMITH  ", &ctx).unwrap(), "SMITH");
        assert_eq!(trim("SMITH", &ctx).unwrap(), "SMITH");
    }

    #[test]
    fn test_remove_whitespace() {
        let ctx = TransformContext::new();
        assert_eq!(remove_whitespace("S M I T H", &ctx).unwrap(), "SMITH");
        assert_eq!(remove_whitespace("SMITH", &ctx).unwrap(), "SMITH");
    }

    #[test]
    fn test_substring() {
        let ctx = TransformContext::new()
            .add_data("start".to_string(), "0".to_string())
            .add_data("length".to_string(), "3".to_string());

        assert_eq!(substring("SMITH", &ctx).unwrap(), "SMI");

        let ctx = TransformContext::new()
            .add_data("start".to_string(), "2".to_string())
            .add_data("length".to_string(), "3".to_string());

        assert_eq!(substring("SMITH", &ctx).unwrap(), "ITH");

        let ctx = TransformContext::new().add_data("start".to_string(), "2".to_string());

        assert_eq!(substring("SMITH", &ctx).unwrap(), "ITH");
    }

    #[test]
    fn test_format_date() {
        let ctx = TransformContext::new().add_data("format".to_string(), "YYYY-MM-DD".to_string());

        assert_eq!(format_date("20240315", &ctx).unwrap(), "2024-03-15");

        let ctx = TransformContext::new().add_data("format".to_string(), "MM/DD/YYYY".to_string());

        assert_eq!(format_date("20240315", &ctx).unwrap(), "03/15/2024");

        let ctx = TransformContext::new().add_data("format".to_string(), "DD/MM/YYYY".to_string());

        assert_eq!(format_date("20240315", &ctx).unwrap(), "15/03/2024");

        // Raw HL7 output format (also listed in the docstring).
        let ctx = TransformContext::new().add_data("format".to_string(), "YYYYMMDD".to_string());

        assert_eq!(format_date("20240315", &ctx).unwrap(), "20240315");
    }

    #[test]
    fn test_format_datetime() {
        let ctx = TransformContext::new()
            .add_data("format".to_string(), "YYYY-MM-DD HH:MM:SS".to_string());

        assert_eq!(
            format_datetime("20240315123045", &ctx).unwrap(),
            "2024-03-15 12:30:45"
        );

        let ctx = TransformContext::new().add_data("format".to_string(), "ISO8601".to_string());

        assert_eq!(
            format_datetime("20240315123045", &ctx).unwrap(),
            "2024-03-15T12:30:45"
        );

        // Raw HL7 output format (also listed in the docstring).
        let ctx =
            TransformContext::new().add_data("format".to_string(), "YYYYMMDDHHMMSS".to_string());

        assert_eq!(
            format_datetime("20240315123045", &ctx).unwrap(),
            "20240315123045"
        );
    }

    #[test]
    fn test_replace() {
        let ctx = TransformContext::new()
            .add_data("pattern".to_string(), " ".to_string())
            .add_data("replacement".to_string(), "-".to_string());

        assert_eq!(replace("JOHN DOE", &ctx).unwrap(), "JOHN-DOE");
    }

    #[test]
    fn test_regex_replace() {
        let ctx = TransformContext::new()
            .add_data("regex".to_string(), r"\d+".to_string())
            .add_data("replacement".to_string(), "XXX".to_string());

        assert_eq!(regex_replace("ID-12345", &ctx).unwrap(), "ID-XXX");
    }

    #[test]
    fn test_prefix() {
        let ctx = TransformContext::new().add_data("prefix".to_string(), "MRN-".to_string());

        assert_eq!(prefix("12345", &ctx).unwrap(), "MRN-12345");
    }

    #[test]
    fn test_suffix() {
        let ctx =
            TransformContext::new().add_data("suffix".to_string(), "@hospital.com".to_string());

        assert_eq!(suffix("john.doe", &ctx).unwrap(), "john.doe@hospital.com");
    }

    #[test]
    fn test_pad() {
        let ctx = TransformContext::new()
            .add_data("length".to_string(), "5".to_string())
            .add_data("pad_char".to_string(), "0".to_string())
            .add_data("side".to_string(), "left".to_string());

        assert_eq!(pad("123", &ctx).unwrap(), "00123");

        let ctx = TransformContext::new()
            .add_data("length".to_string(), "5".to_string())
            .add_data("pad_char".to_string(), "0".to_string())
            .add_data("side".to_string(), "right".to_string());

        assert_eq!(pad("123", &ctx).unwrap(), "12300");
    }

    #[test]
    fn test_default_if_empty() {
        let ctx = TransformContext::new().add_data("default".to_string(), "UNKNOWN".to_string());

        assert_eq!(default_if_empty("", &ctx).unwrap(), "UNKNOWN");
        assert_eq!(default_if_empty("SMITH", &ctx).unwrap(), "SMITH");
    }

    #[test]
    fn test_default_if_empty_rule_wiring_requires_opt_out_of_skip() {
        use crate::rule::TransformationRule;

        let ctx = TransformContext::new().add_data("default".to_string(), "UNKNOWN".to_string());
        // Direct wiring: the function itself substitutes the default.
        assert_eq!(default_if_empty("", &ctx).unwrap(), "UNKNOWN");

        // Rule wiring with the default skip_if_empty(true) never calls
        // the function for empty values, so the target stays absent.
        let rule = TransformationRule::new("PID-5", "PID-5").with_transform(default_if_empty);
        assert_eq!(rule.apply("", &ctx).unwrap(), None);

        // With skip_if_empty(false) the function runs and the default
        // applies through the rule path too.
        let rule = TransformationRule::new("PID-5", "PID-5")
            .with_transform(default_if_empty)
            .skip_if_empty(false);
        assert_eq!(rule.apply("", &ctx).unwrap(), Some("UNKNOWN".to_string()));
    }

    #[test]
    fn test_default_if_empty_end_to_end_through_transformer() {
        use crate::MessageTransformer;
        use crate::rule::TransformationRule;
        use vyna_hl7_terser::Terser;

        // PID-5 is absent, so the rule reads an empty source value.
        let source = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|A|B|C|D|202401011200||ADT^A01|1|P|2.5\rPID|1||PATID",
        )
        .unwrap();

        // Default wiring: the empty source never reaches the function.
        let mut transformer = MessageTransformer::new();
        transformer.set_context_data("default".to_string(), "UNKNOWN".to_string());
        transformer
            .add_rule(TransformationRule::new("PID-5", "PID-5").with_transform(default_if_empty));
        let result = transformer.transform(&source).unwrap();
        assert!(
            Terser::new(&result)
                .get("PID-5")
                .unwrap()
                .is_none_or(|v| v.is_empty()),
            "default-skip wiring must leave the target absent"
        );

        // With skip_if_empty(false) the transformer path applies the default.
        let mut transformer = MessageTransformer::new();
        transformer.set_context_data("default".to_string(), "UNKNOWN".to_string());
        transformer.add_rule(
            TransformationRule::new("PID-5", "PID-5")
                .with_transform(default_if_empty)
                .skip_if_empty(false),
        );
        let result = transformer.transform(&source).unwrap();
        assert_eq!(Terser::new(&result).get("PID-5").unwrap(), Some("UNKNOWN"));
    }

    #[test]
    fn test_format_date_non_ascii_returns_error_without_panic() {
        let ctx = TransformContext::new().add_data("format".to_string(), "YYYY-MM-DD".to_string());

        // 12 bytes with a split boundary inside: slicing panicked here.
        assert!(format_date("1234567éXXXX", &ctx).is_err());
        assert_eq!(format_date("20240315", &ctx).unwrap(), "2024-03-15");
    }

    #[test]
    fn test_substring_huge_length_does_not_overflow() {
        let ctx = TransformContext::new()
            .add_data("start".to_string(), "2".to_string())
            .add_data("length".to_string(), usize::MAX.to_string());

        assert_eq!(substring("SMITH", &ctx).unwrap(), "ITH");
    }

    #[test]
    fn test_pad_counts_chars_not_bytes() {
        let ctx = TransformContext::new()
            .add_data("length".to_string(), "3".to_string())
            .add_data("pad_char".to_string(), "0".to_string())
            .add_data("side".to_string(), "left".to_string());

        // "éé" is 2 chars (4 bytes); padding to length 3 adds one char.
        assert_eq!(pad("éé", &ctx).unwrap(), "0éé");
    }

    #[test]
    fn test_pad_rejects_huge_length() {
        for length in [usize::MAX.to_string(), (MAX_PAD_LENGTH + 1).to_string()] {
            let ctx = TransformContext::new()
                .add_data("length".to_string(), length)
                .add_data("pad_char".to_string(), "0".to_string())
                .add_data("side".to_string(), "left".to_string());

            assert!(pad("123", &ctx).is_err());
        }
    }

    #[test]
    fn test_format_datetime_parses_hour_and_minute_precision() {
        let ctx = TransformContext::new().add_data("format".to_string(), "ISO8601".to_string());

        assert_eq!(
            format_datetime("2024031512", &ctx).unwrap(),
            "2024-03-15T12:00:00"
        );
        assert_eq!(
            format_datetime("202403151230", &ctx).unwrap(),
            "2024-03-15T12:30:00"
        );
        // Date-only still defaults the time to midnight.
        assert_eq!(
            format_datetime("20240315", &ctx).unwrap(),
            "2024-03-15T00:00:00"
        );
    }

    #[test]
    fn test_format_datetime_rejects_ragged_and_trailing_garbage() {
        let ctx = TransformContext::new().add_data("format".to_string(), "ISO8601".to_string());

        for bad in [
            "202403151",           // ragged: 9 chars
            "20240315123",         // ragged: 11 chars
            "2024031512304",       // ragged: 13 chars
            "20240315123045X",     // trailing garbage
            "20240315XYZ",         // trailing garbage on a date
            "2024031512304599",    // overlong digits
            "20240315123045+0000", // unsupported offset tail
            "2024031525",          // hour 25 out of range
            "202403151260",        // minute 60 out of range
        ] {
            assert!(
                format_datetime(bad, &ctx).is_err(),
                "{bad} must be rejected"
            );
        }
    }

    #[test]
    fn test_pad_rejects_invalid_side_for_long_values() {
        // The side must be validated before the length short-circuit:
        // a config error cannot hide behind a long-enough value.
        let ctx = TransformContext::new()
            .add_data("length".to_string(), "3".to_string())
            .add_data("pad_char".to_string(), "0".to_string())
            .add_data("side".to_string(), "sideways".to_string());

        assert!(pad("1", &ctx).is_err());
        assert!(pad("12345", &ctx).is_err());
    }

    #[test]
    fn test_format_date_rejects_trailing_garbage() {
        let ctx = TransformContext::new().add_data("format".to_string(), "YYYY-MM-DD".to_string());

        // Non-numeric tails, ragged and overlong digit tails are rejected,
        // matching format_datetime's trailing-garbage rejection.
        for bad in [
            "20240315XYZ",
            "20240315123045X",
            "20240315+0000",
            "202403151",
            "20240315123",
            "2024031512304",
            "2024031512304599",
        ] {
            assert!(format_date(bad, &ctx).is_err(), "{bad} must be rejected");
        }
        // Documented numeric time tails still parse to the date part.
        for good in ["20240315", "2024031512", "202403151230", "20240315123045"] {
            assert_eq!(format_date(good, &ctx).unwrap(), "2024-03-15");
        }
    }

    #[test]
    fn test_substring_rejects_unparseable_length() {
        // An invalid length must error like an invalid start, not fall
        // back to end-of-string by silently ignoring the bad value.
        let ctx = TransformContext::new()
            .add_data("start".to_string(), "0".to_string())
            .add_data("length".to_string(), "abc".to_string());

        assert!(substring("SMITH", &ctx).is_err());
    }

    #[test]
    fn test_format_datetime_non_ascii_returns_error_without_panic() {
        let ctx = TransformContext::new()
            .add_data("format".to_string(), "YYYY-MM-DD HH:MM:SS".to_string());

        // 15 bytes with byte 14 mid-character: slicing panicked here.
        assert!(format_datetime("1234567890123é", &ctx).is_err());
        assert!(format_datetime("2024031é", &ctx).is_err());
        assert_eq!(
            format_datetime("20240315123045", &ctx).unwrap(),
            "2024-03-15 12:30:45"
        );
    }

    #[test]
    fn test_regex_replace_expands_captures_and_escapes_dollar() {
        // $2/$1 reorder capture groups in the replacement.
        let ctx = TransformContext::new()
            .add_data("regex".to_string(), r"(\w+)-(\d+)".to_string())
            .add_data("replacement".to_string(), "$2-$1".to_string());
        assert_eq!(regex_replace("ID-12345", &ctx).unwrap(), "12345-ID");

        // $$ writes a literal dollar sign.
        let ctx = TransformContext::new()
            .add_data("regex".to_string(), r"\d+".to_string())
            .add_data("replacement".to_string(), "$$5".to_string());
        assert_eq!(regex_replace("ID-12345", &ctx).unwrap(), "ID-$5");

        // An unescaped $N with no such group expands to empty (regex
        // crate semantics), not to a literal "$5".
        let ctx = TransformContext::new()
            .add_data("regex".to_string(), r"\d+".to_string())
            .add_data("replacement".to_string(), "$5".to_string());
        assert_eq!(regex_replace("ID-12345", &ctx).unwrap(), "ID-");
    }

    #[test]
    fn test_format_date_rejects_out_of_range_time_tails() {
        let ctx = TransformContext::new().add_data("format".to_string(), "YYYY-MM-DD".to_string());

        // Out-of-range tails are rejected like format_datetime rejects
        // them, instead of silently dropped with the date returned.
        for bad in ["2024031599", "20240315259999", "20240315126061"] {
            assert!(format_date(bad, &ctx).is_err(), "{bad} must be rejected");
        }
        // Valid tails still parse to the date part.
        for good in ["2024031523", "202403151259", "20240315125959"] {
            assert_eq!(format_date(good, &ctx).unwrap(), "2024-03-15");
        }
    }

    #[test]
    fn test_pad_side_defaults_to_left_when_absent() {
        let ctx = TransformContext::new()
            .add_data("length".to_string(), "5".to_string())
            .add_data("pad_char".to_string(), "0".to_string());

        assert_eq!(pad("123", &ctx).unwrap(), "00123");
    }

    #[test]
    fn test_substring_error_names_present_but_invalid_start() {
        // A missing key keeps the "in context" error...
        let err = substring("SMITH", &TransformContext::new())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("requires 'start' in context"),
            "unexpected: {err}"
        );

        // ...while a present-but-invalid value names the real problem.
        let ctx = TransformContext::new().add_data("start".to_string(), "-1".to_string());
        let err = substring("SMITH", &ctx).unwrap_err().to_string();
        assert!(
            err.contains("requires 'start' to be a number"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn test_pad_error_names_present_but_invalid_length() {
        // A missing key keeps the "in context" error...
        let ctx = TransformContext::new().add_data("pad_char".to_string(), "0".to_string());
        let err = pad("123", &ctx).unwrap_err().to_string();
        assert!(
            err.contains("requires 'length' in context"),
            "unexpected: {err}"
        );

        // ...while a present-but-invalid value names the real problem.
        let ctx = TransformContext::new()
            .add_data("length".to_string(), "abc".to_string())
            .add_data("pad_char".to_string(), "0".to_string());
        let err = pad("123", &ctx).unwrap_err().to_string();
        assert!(
            err.contains("requires 'length' to be a number"),
            "unexpected: {err}"
        );
    }
}
