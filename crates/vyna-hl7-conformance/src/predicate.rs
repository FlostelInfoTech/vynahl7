//! Predicate parsing and evaluation for conditional usage
//!
//! This module provides parsing and evaluation of conditional predicates used in
//! conformance profiles. Predicates allow fields to have conditional requirements
//! based on the values of other fields in the message.
//!
//! ## Supported Condition Types
//!
//! - **IS VALUED**: Check if a field has a non-empty value
//! - **IS NOT VALUED**: Check if a field is empty or missing
//! - **Equality**: Compare field value to literal (e.g., "PID-8 = 'M'")
//! - **Comparison**: Numeric comparisons (>, <, >=, <=, !=)
//! - **Boolean Logic**: AND, OR operators for complex conditions
//!
//! ## Example Conditions
//!
//! ```text
//! PID-8 IS VALUED
//! PID-8 = 'M'
//! PID-7 IS NOT VALUED
//! PID-8 IS VALUED AND PV1-2 = 'I'
//! ```

use crate::error::{ConformanceError, Result};
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Condition expression that can be evaluated against a message
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// Field has a non-empty value (e.g., "PID-8 IS VALUED")
    IsValued(String),
    /// Field is empty or missing (e.g., "PID-8 IS NOT VALUED")
    IsNotValued(String),
    /// Field equals literal value (e.g., "PID-8 = 'M'")
    Equals(String, String),
    /// Field does not equal literal value
    NotEquals(String, String),
    /// Numeric greater than comparison
    GreaterThan(String, f64),
    /// Numeric less than comparison
    LessThan(String, f64),
    /// Numeric greater than or equal comparison
    GreaterThanOrEqual(String, f64),
    /// Numeric less than or equal comparison
    LessThanOrEqual(String, f64),
    /// Logical AND of two conditions
    And(Box<Condition>, Box<Condition>),
    /// Logical OR of two conditions
    Or(Box<Condition>, Box<Condition>),
    /// Logical NOT of a condition
    Not(Box<Condition>),
}

impl Condition {
    /// Evaluate this condition against a message
    pub fn evaluate(&self, message: &Message) -> bool {
        let terser = Terser::new(message);
        self.evaluate_with_terser(&terser)
    }

    /// Evaluate using an existing Terser instance
    pub fn evaluate_with_terser(&self, terser: &Terser) -> bool {
        match self {
            Condition::IsValued(path) => {
                if let Ok(Some(value)) = terser.get(path) {
                    !value.trim().is_empty()
                } else {
                    false
                }
            }
            Condition::IsNotValued(path) => {
                if let Ok(Some(value)) = terser.get(path) {
                    value.trim().is_empty()
                } else {
                    true
                }
            }
            Condition::Equals(path, expected) => {
                if let Ok(Some(value)) = terser.get(path) {
                    value.trim() == expected.trim()
                } else {
                    false
                }
            }
            Condition::NotEquals(path, expected) => {
                if let Ok(Some(value)) = terser.get(path) {
                    value.trim() != expected.trim()
                } else {
                    true
                }
            }
            Condition::GreaterThan(path, threshold) => {
                if let Ok(Some(value)) = terser.get(path) {
                    if let Ok(num) = value.trim().parse::<f64>() {
                        num > *threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Condition::LessThan(path, threshold) => {
                if let Ok(Some(value)) = terser.get(path) {
                    if let Ok(num) = value.trim().parse::<f64>() {
                        num < *threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Condition::GreaterThanOrEqual(path, threshold) => {
                if let Ok(Some(value)) = terser.get(path) {
                    if let Ok(num) = value.trim().parse::<f64>() {
                        num >= *threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Condition::LessThanOrEqual(path, threshold) => {
                if let Ok(Some(value)) = terser.get(path) {
                    if let Ok(num) = value.trim().parse::<f64>() {
                        num <= *threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            Condition::And(left, right) => {
                left.evaluate_with_terser(terser) && right.evaluate_with_terser(terser)
            }
            Condition::Or(left, right) => {
                left.evaluate_with_terser(terser) || right.evaluate_with_terser(terser)
            }
            Condition::Not(inner) => !inner.evaluate_with_terser(terser),
        }
    }
}

/// Parser for predicate condition expressions
pub struct PredicateParser;

/// Maximum nesting depth for NOT/AND/OR recursion.
///
/// The parser recurses per operator; without a cap, hostile profile
/// text with thousands of nested operators could exhaust the stack.
const MAX_PREDICATE_DEPTH: usize = 64;

impl PredicateParser {
    /// Parse a condition string into a Condition
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_conformance::predicate::{PredicateParser, Condition};
    ///
    /// let cond = PredicateParser::parse("PID-8 IS VALUED").unwrap();
    /// assert_eq!(cond, Condition::IsValued("PID-8".to_string()));
    ///
    /// let cond = PredicateParser::parse("PID-8 = 'M'").unwrap();
    /// assert_eq!(cond, Condition::Equals("PID-8".to_string(), "M".to_string()));
    /// ```
    pub fn parse(input: &str) -> Result<Condition> {
        Self::parse_with_depth(input, 0)
    }

    fn parse_with_depth(input: &str, depth: usize) -> Result<Condition> {
        if depth > MAX_PREDICATE_DEPTH {
            return Err(ConformanceError::InvalidPredicate(format!(
                "predicate nesting exceeds limit {MAX_PREDICATE_DEPTH}"
            )));
        }
        let trimmed = input.trim();

        // Handle OR (lowest precedence)
        if let Some(or_pos) = Self::find_operator(trimmed, " OR ") {
            let left = Self::parse_with_depth(&trimmed[..or_pos], depth + 1)?;
            let right = Self::parse_with_depth(&trimmed[or_pos + 4..], depth + 1)?;
            return Ok(Condition::Or(Box::new(left), Box::new(right)));
        }

        // Handle AND (higher precedence than OR)
        if let Some(and_pos) = Self::find_operator(trimmed, " AND ") {
            let left = Self::parse_with_depth(&trimmed[..and_pos], depth + 1)?;
            let right = Self::parse_with_depth(&trimmed[and_pos + 5..], depth + 1)?;
            return Ok(Condition::And(Box::new(left), Box::new(right)));
        }

        // Handle NOT (byte-compared: the match pins ASCII bytes, so
        // slicing at 4 is always a character boundary).
        if trimmed.len() >= 4 && trimmed.as_bytes()[..4].eq_ignore_ascii_case(b"NOT ") {
            let inner = Self::parse_with_depth(&trimmed[4..], depth + 1)?;
            return Ok(Condition::Not(Box::new(inner)));
        }

        // Handle IS VALUED (quote-aware: a literal such as
        // 'X IS VALUED' must not match).
        if let Some(pos) = Self::find_operator(trimmed, " IS VALUED") {
            let path = trimmed[..pos].trim().to_string();
            Self::validate_path(&path)?;
            return Ok(Condition::IsValued(path));
        }

        // Handle IS NOT VALUED
        if let Some(pos) = Self::find_operator(trimmed, " IS NOT VALUED") {
            let path = trimmed[..pos].trim().to_string();
            Self::validate_path(&path)?;
            return Ok(Condition::IsNotValued(path));
        }

        // Handle comparison operators
        if let Some((path, op, value)) = Self::parse_comparison(trimmed)? {
            Self::validate_path(&path)?;
            match op.as_str() {
                "=" => Ok(Condition::Equals(path, value)),
                "!=" | "<>" => Ok(Condition::NotEquals(path, value)),
                ">" => {
                    let num = Self::parse_number(&value)?;
                    Ok(Condition::GreaterThan(path, num))
                }
                "<" => {
                    let num = Self::parse_number(&value)?;
                    Ok(Condition::LessThan(path, num))
                }
                ">=" => {
                    let num = Self::parse_number(&value)?;
                    Ok(Condition::GreaterThanOrEqual(path, num))
                }
                "<=" => {
                    let num = Self::parse_number(&value)?;
                    Ok(Condition::LessThanOrEqual(path, num))
                }
                _ => Err(ConformanceError::InvalidPredicate(format!(
                    "Unknown operator: {}",
                    op
                ))),
            }
        } else {
            Err(ConformanceError::InvalidPredicate(format!(
                "Unable to parse condition: {}",
                trimmed
            )))
        }
    }

    /// Find operator position, ignoring quoted strings
    ///
    /// The operator is matched ASCII case-insensitively against the
    /// original bytes: indexing an uppercased copy would panic (or split
    /// at the wrong place) whenever a non-ASCII character maps to a
    /// different number of bytes. The returned index is always a
    /// character boundary and safe to slice with. Both single and double
    /// quotes open a literal region (a quote only closes its own kind).
    fn find_operator(input: &str, operator: &str) -> Option<usize> {
        let bytes = input.as_bytes();
        let needle = operator.as_bytes();
        let mut quote: Option<u8> = None;

        for (i, _) in input.char_indices() {
            let b = bytes[i];
            match quote {
                Some(q) if b == q => quote = None,
                None if b == b'\'' || b == b'"' => quote = Some(b),
                _ => {}
            }

            if quote.is_none()
                && bytes.len() - i >= needle.len()
                && bytes[i..i + needle.len()].eq_ignore_ascii_case(needle)
            {
                return Some(i);
            }
        }

        None
    }

    /// Parse comparison expression (path op value)
    fn parse_comparison(input: &str) -> Result<Option<(String, String, String)>> {
        // Try operators in order of length (to match >= before =, etc.).
        // Operators inside quoted literals are ignored: without that,
        // "PID-8 = 'a>=b'" would split on the quoted ">=".
        let operators = [">=", "<=", "!=", "<>", "=", ">", "<"];

        for op in &operators {
            if let Some(op_pos) = Self::find_operator(input, op) {
                let path = input[..op_pos].trim().to_string();
                let value_part = input[op_pos + op.len()..].trim();

                // Remove quotes if present (single or double). A lone
                // quote (length 1) is kept literally: stripping it would
                // slice [1..0] and panic.
                let value = if value_part.len() >= 2
                    && ((value_part.starts_with('\'') && value_part.ends_with('\''))
                        || (value_part.starts_with('"') && value_part.ends_with('"')))
                {
                    value_part[1..value_part.len() - 1].to_string()
                } else {
                    value_part.to_string()
                };

                return Ok(Some((path, op.to_string(), value)));
            }
        }

        Ok(None)
    }

    /// Validate that a path follows Terser notation
    ///
    /// Mirrors the terser path grammar (`SEGMENT[(n)]-FIELD[(n)]` with
    /// optional `-COMPONENT` and `-SUBCOMPONENT`), plus an uppercase
    /// segment ID: terser matching is case-sensitive, so a lowercase
    /// path would parse yet silently mis-evaluate.
    fn validate_path(path: &str) -> Result<()> {
        if path.is_empty() {
            return Err(ConformanceError::InvalidPredicate(
                "Empty field path".to_string(),
            ));
        }

        // Terser paths never contain whitespace, operators, or quotes;
        // their presence means the condition split misfired.
        if path
            .chars()
            .any(|c| c.is_whitespace() || "=><!'\"".contains(c))
        {
            return Err(ConformanceError::InvalidPredicate(format!(
                "Invalid field path: {}",
                path
            )));
        }

        let parts: Vec<&str> = path.split('-').collect();
        if parts.len() < 2 || parts.len() > 4 {
            return Err(ConformanceError::InvalidPredicate(format!(
                "Invalid field path: {}",
                path
            )));
        }
        let (segment_id, _) = Self::split_indexed(parts[0], path)?;
        if segment_id.is_empty()
            || !segment_id
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        {
            return Err(ConformanceError::InvalidPredicate(format!(
                "Invalid segment ID in path: {}",
                path
            )));
        }
        let (field, _) = Self::split_indexed(parts[1], path)?;
        if field.parse::<usize>().is_err() {
            return Err(ConformanceError::InvalidPredicate(format!(
                "Invalid field index in path: {}",
                path
            )));
        }
        for part in &parts[2..] {
            if part.parse::<usize>().is_err() {
                return Err(ConformanceError::InvalidPredicate(format!(
                    "Invalid component index in path: {}",
                    path
                )));
            }
        }

        Ok(())
    }

    /// Split `BASE` or `BASE(n)` into its base and optional 1-based index.
    ///
    /// Mirrors the terser parser: the index must be numeric and >= 1,
    /// and nothing may trail the closing parenthesis.
    fn split_indexed<'a>(part: &'a str, path: &str) -> Result<(&'a str, Option<usize>)> {
        let Some(open) = part.find('(') else {
            if part.contains(')') {
                return Err(ConformanceError::InvalidPredicate(format!(
                    "Invalid field path: {}",
                    path
                )));
            }
            return Ok((part, None));
        };
        // `(` and `)` are ASCII, so these slices are char boundaries.
        let base = &part[..open];
        let rest = &part[open + 1..];
        let Some(close) = rest.find(')') else {
            return Err(ConformanceError::InvalidPredicate(format!(
                "Invalid field path: {}",
                path
            )));
        };
        if !rest[close + 1..].is_empty() {
            return Err(ConformanceError::InvalidPredicate(format!(
                "Invalid field path: {}",
                path
            )));
        }
        let index = rest[..close].parse::<usize>().map_err(|_| {
            ConformanceError::InvalidPredicate(format!("Invalid field path: {}", path))
        })?;
        if index == 0 {
            return Err(ConformanceError::InvalidPredicate(format!(
                "Invalid field path (index 0 is 1-based): {}",
                path
            )));
        }
        Ok((base, Some(index)))
    }

    /// Parse a string as a number
    fn parse_number(s: &str) -> Result<f64> {
        s.parse::<f64>()
            .map_err(|_| ConformanceError::InvalidPredicate(format!("Invalid number: {}", s)))
    }
}

/// Evaluator for predicate-based conditional usage
pub struct PredicateEvaluator;

impl PredicateEvaluator {
    /// Evaluate a predicate's condition and return the appropriate usage
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_conformance::predicate::PredicateEvaluator;
    /// use vyna_hl7_conformance::profile::{Predicate, Usage};
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let message = parse_message("MSH|^~\\&|...\rPID|||12345||Doe^John||M|||...")?;
    ///
    /// let predicate = Predicate::new(
    ///     "PID-8 IS VALUED".to_string(),
    ///     Usage::Required,
    ///     Usage::Optional,
    /// );
    ///
    /// let usage = PredicateEvaluator::evaluate(&predicate, &message)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn evaluate(
        predicate: &crate::profile::Predicate,
        message: &Message,
    ) -> Result<crate::profile::Usage> {
        let condition = PredicateParser::parse(&predicate.condition)?;
        let result = condition.evaluate(message);

        Ok(if result {
            predicate.true_usage
        } else {
            predicate.false_usage
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::{Component, Field, Repetition, Segment};

    fn create_test_message() -> Message {
        let mut message = Message::default();

        // Add MSH segment
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|")); // MSH-1
        msh.fields.push(Field::from_value("^~\\&")); // MSH-2
        message.segments.push(msh);

        // Add PID segment
        let mut pid = Segment::new("PID");
        pid.fields.push(Field::new()); // PID-1 (empty)
        pid.fields.push(Field::new()); // PID-2 (empty)
        pid.fields.push(Field::from_value("12345")); // PID-3
        pid.fields.push(Field::new()); // PID-4 (empty)

        // PID-5: Patient Name with components
        let mut name_field = Field::new();
        let mut name_rep = Repetition::new();
        name_rep.components.push(Component::from_value("Doe"));
        name_rep.components.push(Component::from_value("John"));
        name_field.add_repetition(name_rep);
        pid.fields.push(name_field); // PID-5

        pid.fields.push(Field::new()); // PID-6 (empty)
        pid.fields.push(Field::from_value("19900101")); // PID-7: DOB
        pid.fields.push(Field::from_value("M")); // PID-8: Sex
        message.segments.push(pid);

        message
    }

    #[test]
    fn test_parse_is_valued() {
        let result = PredicateParser::parse("PID-8 IS VALUED");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Condition::IsValued("PID-8".to_string()));
    }

    #[test]
    fn test_parse_is_not_valued() {
        let result = PredicateParser::parse("PID-7 IS NOT VALUED");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Condition::IsNotValued("PID-7".to_string()));
    }

    #[test]
    fn test_parse_equals() {
        let result = PredicateParser::parse("PID-8 = 'M'");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Condition::Equals("PID-8".to_string(), "M".to_string())
        );
    }

    #[test]
    fn test_parse_not_equals() {
        let result = PredicateParser::parse("PID-8 != 'F'");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Condition::NotEquals("PID-8".to_string(), "F".to_string())
        );
    }

    #[test]
    fn test_parse_greater_than() {
        let result = PredicateParser::parse("PID-7 > 18");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            Condition::GreaterThan("PID-7".to_string(), 18.0)
        );
    }

    #[test]
    fn test_parse_and() {
        let result = PredicateParser::parse("PID-8 IS VALUED AND PID-7 IS VALUED");
        assert!(result.is_ok());
        match result.unwrap() {
            Condition::And(left, right) => {
                assert_eq!(*left, Condition::IsValued("PID-8".to_string()));
                assert_eq!(*right, Condition::IsValued("PID-7".to_string()));
            }
            _ => panic!("Expected And condition"),
        }
    }

    #[test]
    fn test_parse_or() {
        let result = PredicateParser::parse("PID-8 = 'M' OR PID-8 = 'F'");
        assert!(result.is_ok());
        match result.unwrap() {
            Condition::Or(left, right) => {
                assert_eq!(
                    *left,
                    Condition::Equals("PID-8".to_string(), "M".to_string())
                );
                assert_eq!(
                    *right,
                    Condition::Equals("PID-8".to_string(), "F".to_string())
                );
            }
            _ => panic!("Expected Or condition"),
        }
    }

    #[test]
    fn test_parse_not() {
        let result = PredicateParser::parse("NOT PID-8 IS VALUED");
        assert!(result.is_ok());
        match result.unwrap() {
            Condition::Not(inner) => {
                assert_eq!(*inner, Condition::IsValued("PID-8".to_string()));
            }
            _ => panic!("Expected Not condition"),
        }
    }

    #[test]
    fn test_evaluate_is_valued() {
        let message = create_test_message();
        let condition = Condition::IsValued("PID-8".to_string());
        assert!(condition.evaluate(&message));

        let condition = Condition::IsValued("PID-4".to_string());
        assert!(!condition.evaluate(&message));
    }

    #[test]
    fn test_evaluate_is_not_valued() {
        let message = create_test_message();
        let condition = Condition::IsNotValued("PID-4".to_string());
        assert!(condition.evaluate(&message));

        let condition = Condition::IsNotValued("PID-8".to_string());
        assert!(!condition.evaluate(&message));
    }

    #[test]
    fn test_evaluate_equals() {
        let message = create_test_message();
        let condition = Condition::Equals("PID-8".to_string(), "M".to_string());
        assert!(condition.evaluate(&message));

        let condition = Condition::Equals("PID-8".to_string(), "F".to_string());
        assert!(!condition.evaluate(&message));
    }

    #[test]
    fn test_evaluate_and() {
        let message = create_test_message();
        let condition = Condition::And(
            Box::new(Condition::IsValued("PID-8".to_string())),
            Box::new(Condition::Equals("PID-8".to_string(), "M".to_string())),
        );
        assert!(condition.evaluate(&message));

        let condition = Condition::And(
            Box::new(Condition::IsValued("PID-8".to_string())),
            Box::new(Condition::Equals("PID-8".to_string(), "F".to_string())),
        );
        assert!(!condition.evaluate(&message));
    }

    #[test]
    fn test_evaluate_or() {
        let message = create_test_message();
        let condition = Condition::Or(
            Box::new(Condition::Equals("PID-8".to_string(), "M".to_string())),
            Box::new(Condition::Equals("PID-8".to_string(), "F".to_string())),
        );
        assert!(condition.evaluate(&message));

        let condition = Condition::Or(
            Box::new(Condition::Equals("PID-8".to_string(), "X".to_string())),
            Box::new(Condition::Equals("PID-8".to_string(), "U".to_string())),
        );
        assert!(!condition.evaluate(&message));
    }

    #[test]
    fn test_evaluate_complex() {
        let message = create_test_message();
        // (PID-8 = 'M' OR PID-8 = 'F') AND PID-7 IS VALUED
        let condition = Condition::And(
            Box::new(Condition::Or(
                Box::new(Condition::Equals("PID-8".to_string(), "M".to_string())),
                Box::new(Condition::Equals("PID-8".to_string(), "F".to_string())),
            )),
            Box::new(Condition::IsValued("PID-7".to_string())),
        );
        assert!(condition.evaluate(&message));
    }

    #[test]
    fn test_invalid_path() {
        let result = PredicateParser::parse("INVALID IS VALUED");
        assert!(result.is_err());
    }

    #[test]
    fn test_lowercase_segment_id_is_rejected() {
        // Terser matching is case-sensitive: a lowercase path parses
        // but silently mis-evaluates, so reject it at parse time.
        assert!(PredicateParser::parse("pid-8 IS VALUED").is_err());
        assert!(PredicateParser::parse("pid-8 IS NOT VALUED").is_err());
        assert!(PredicateParser::parse("pid-8 = 'M'").is_err());
        assert!(PredicateParser::parse("PID-8 IS VALUED").is_ok());
    }

    #[test]
    fn test_malformed_terser_paths_are_rejected() {
        // Full terser shape: SEGMENT[(n)]-FIELD[(n)][-COMP][-SUB].
        assert!(PredicateParser::parse("PID-5-1-2-3 IS VALUED").is_err());
        assert!(PredicateParser::parse("OBX(0)-5 IS VALUED").is_err());
        assert!(PredicateParser::parse("PID-11(0)-1 IS VALUED").is_err());
        assert!(PredicateParser::parse("PID-X IS VALUED").is_err());
        assert!(PredicateParser::parse("PID-5-x IS VALUED").is_err());
        assert!(PredicateParser::parse("OBX(2)-5 IS VALUED").is_ok());
        assert!(PredicateParser::parse("PID-11(2)-1 IS VALUED").is_ok());
        assert!(PredicateParser::parse("PID-5-1-2 IS VALUED").is_ok());
    }

    #[test]
    fn test_invalid_expression() {
        let result = PredicateParser::parse("some random text");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_non_ascii_literal_does_not_panic() {
        // U+017F (2 bytes) uppercases to "S" (1 byte), shifting every
        // byte index after it; the parser must not panic on it.
        let result = PredicateParser::parse("PID-8 = 'ſ' AND PID-7 IS VALUED");
        assert!(result.is_ok());
        match result.unwrap() {
            Condition::And(left, right) => {
                assert_eq!(
                    *left,
                    Condition::Equals("PID-8".to_string(), "ſ".to_string())
                );
                assert_eq!(*right, Condition::IsValued("PID-7".to_string()));
            }
            _ => panic!("Expected And condition"),
        }
    }

    #[test]
    fn test_parse_multibyte_before_is_valued_does_not_panic() {
        let result = PredicateParser::parse("PID-8 = 'Müller' OR PID-8 = 'ß'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_lone_quote_does_not_panic() {
        let result = PredicateParser::parse("PID-8 = '");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_byte_shifting_case_map_does_not_panic() {
        // 'ſ' (2 bytes) uppercases to "S" (1 byte), shifting every
        // later byte index; slicing the uppercased copy panicked here.
        // The path is invalid (no dash), so an error — not a panic — is
        // the correct outcome.
        let result = PredicateParser::parse("ſé = 'M'");
        assert!(result.is_err());
    }

    #[test]
    fn test_quoted_is_valued_literal_is_not_a_keyword() {
        // The keyword search must skip quoted literals: this is an
        // equality test against the literal 'X IS VALUED', not IsValued.
        let parsed = PredicateParser::parse("PID-8 = 'X IS VALUED'").unwrap();
        assert!(matches!(
            parsed,
            Condition::Equals(path, value)
                if path == "PID-8" && value == "X IS VALUED"
        ));
    }

    #[test]
    fn test_quoted_comparison_operators_do_not_split() {
        // Operators inside quotes belong to the literal, not the syntax.
        let parsed = PredicateParser::parse("PID-8 = 'a>=b'").unwrap();
        assert!(matches!(
            parsed,
            Condition::Equals(path, value) if path == "PID-8" && value == "a>=b"
        ));
        let parsed = PredicateParser::parse("PID-8 != \"x<y\"").unwrap();
        assert!(matches!(
            parsed,
            Condition::NotEquals(path, value) if path == "PID-8" && value == "x<y"
        ));
    }

    #[test]
    fn test_malformed_path_with_operators_is_rejected() {
        assert!(PredicateParser::parse("PID-8 = 'X' IS VALUED").is_err());
    }

    #[test]
    fn test_deeply_nested_not_is_rejected() {
        let mut input = String::new();
        for _ in 0..(MAX_PREDICATE_DEPTH + 10) {
            input.push_str("NOT ");
        }
        input.push_str("PID-8 IS VALUED");
        let err = PredicateParser::parse(&input).unwrap_err();
        assert!(err.to_string().contains("exceeds limit"), "{err}");
    }

    #[test]
    fn test_deeply_nested_and_is_rejected() {
        let mut input = String::from("PID-8 IS VALUED");
        for _ in 0..(MAX_PREDICATE_DEPTH + 10) {
            input = format!("{input} AND PID-8 IS VALUED");
        }
        let err = PredicateParser::parse(&input).unwrap_err();
        assert!(err.to_string().contains("exceeds limit"), "{err}");
    }

    #[test]
    fn test_shallow_nesting_still_parses() {
        let parsed =
            PredicateParser::parse("NOT PID-8 IS VALUED AND PID-7 IS VALUED OR PID-8 = 'M'")
                .unwrap();
        assert!(matches!(parsed, Condition::Or(..)));
    }
}
