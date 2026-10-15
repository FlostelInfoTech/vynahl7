//! Cross-field validation patterns
//!
//! This module provides common cross-field validation patterns for creating
//! validation rules that check relationships between multiple fields.
//!
//! ## Examples
//!
//! ```
//! use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
//! use vyna_hl7_validator::rules::RuleSeverity;
//!
//! // If PID-8 is 'M', then PID-19 must be valued
//! let rule = CrossFieldValidator::if_then(
//!     "male_needs_ssn",
//!     "Male patients must have SSN",
//!     RuleSeverity::Warning,
//!     "PID-8", "M",
//!     "PID-19",
//! );
//!
//! // PID-11 and PID-12 are mutually exclusive
//! let rule2 = CrossFieldValidator::mutually_exclusive(
//!     "address_xor",
//!     "Cannot have both home and work address",
//!     RuleSeverity::Error,
//!     vec!["PID-11", "PID-12"],
//! );
//! ```

use super::{RuleSeverity, ValidationRule};
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

/// Cross-field validation pattern builder
pub struct CrossFieldValidator;

/// Read the addressed value from every occurrence of an unindexed
/// segment path (`SEG-...`), one entry per occurrence in message
/// order. Returns `None` for indexed paths (`SEG(n)-...`) and paths
/// without a field part, so callers fall back to a single `get`.
///
/// Iteration is bounded by the occurrence count: it cannot stop on
/// an out-of-range error because `get` returns `Ok(None)` — not an
/// error — when the segment id is absent entirely.
fn occurrence_values<'a>(
    msg: &Message,
    terser: &'a Terser<'a>,
    field: &str,
) -> Option<Vec<Option<&'a str>>> {
    let (seg, rest) = field.split_once('-')?;
    if seg.is_empty() || rest.is_empty() || seg.contains('(') {
        return None;
    }
    let count = msg.get_segments_by_id(seg).len();
    let mut values = Vec::with_capacity(count);
    for index in 1..=count {
        match terser.get(&format!("{seg}({index})-{rest}")) {
            Ok(value) => values.push(value),
            Err(_) => break,
        }
    }
    Some(values)
}

impl CrossFieldValidator {
    /// Create an if-then rule: if field A equals value, then field B must be valued
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
    /// use vyna_hl7_validator::rules::RuleSeverity;
    ///
    /// // If patient is male (PID-8 = 'M'), then SSN (PID-19) must be provided
    /// let rule = CrossFieldValidator::if_then(
    ///     "male_patient_ssn",
    ///     "Male patients must have SSN",
    ///     RuleSeverity::Warning,
    ///     "PID-8", "M",
    ///     "PID-19",
    /// );
    /// ```
    pub fn if_then(
        name: impl Into<String>,
        description: impl Into<String>,
        severity: RuleSeverity,
        condition_field: impl Into<String>,
        condition_value: impl Into<String>,
        then_field: impl Into<String>,
    ) -> ValidationRule {
        let condition_field = condition_field.into();
        let condition_value = condition_value.into();
        let then_field = then_field.into();

        ValidationRule::new(name, description, severity).with_condition(move |msg| {
            let terser = Terser::new(msg);

            // Check if condition field equals the specified value
            if let Ok(Some(field_value)) = terser.get(&condition_field)
                && field_value.trim() == condition_value.trim()
            {
                // If condition is true, check that then_field is valued
                if let Ok(Some(then_value)) = terser.get(&then_field) {
                    return !then_value.trim().is_empty();
                }
                return false; // then_field not found or empty
            }

            // Condition not met, so rule passes
            true
        })
    }

    /// Create a mutually exclusive rule: fields cannot both be valued
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
    /// use vyna_hl7_validator::rules::RuleSeverity;
    ///
    /// // Patient can have either home address (PID-11) or work address (PID-12), but not both
    /// let rule = CrossFieldValidator::mutually_exclusive(
    ///     "address_exclusive",
    ///     "Home and work address are mutually exclusive",
    ///     RuleSeverity::Error,
    ///     vec!["PID-11", "PID-12"],
    /// );
    /// ```
    pub fn mutually_exclusive(
        name: impl Into<String>,
        description: impl Into<String>,
        severity: RuleSeverity,
        fields: Vec<impl Into<String>>,
    ) -> ValidationRule {
        let fields: Vec<String> = fields.into_iter().map(|f| f.into()).collect();

        ValidationRule::new(name, description, severity).with_condition(move |msg| {
            let terser = Terser::new(msg);
            let mut valued_count = 0;

            for field in &fields {
                if let Ok(Some(value)) = terser.get(field)
                    && !value.trim().is_empty()
                {
                    valued_count += 1;
                }
            }

            // Pass if at most one field is valued
            valued_count <= 1
        })
    }

    /// Create an "at least one" rule: at least one of the specified fields must be valued
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
    /// use vyna_hl7_validator::rules::RuleSeverity;
    ///
    /// // Patient must have at least one identifier
    /// let rule = CrossFieldValidator::at_least_one(
    ///     "patient_id_required",
    ///     "Patient must have at least one identifier",
    ///     RuleSeverity::Error,
    ///     vec!["PID-2", "PID-3", "PID-4"],
    /// );
    /// ```
    pub fn at_least_one(
        name: impl Into<String>,
        description: impl Into<String>,
        severity: RuleSeverity,
        fields: Vec<impl Into<String>>,
    ) -> ValidationRule {
        let fields: Vec<String> = fields.into_iter().map(|f| f.into()).collect();

        ValidationRule::new(name, description, severity).with_condition(move |msg| {
            let terser = Terser::new(msg);

            for field in &fields {
                if let Ok(Some(value)) = terser.get(field)
                    && !value.trim().is_empty()
                {
                    return true; // At least one field is valued
                }
            }

            false // None of the fields are valued
        })
    }

    /// Create an "all or none" rule: either all fields must be valued or none of them
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
    /// use vyna_hl7_validator::rules::RuleSeverity;
    ///
    /// // Address components must all be provided or all be empty
    /// let rule = CrossFieldValidator::all_or_none(
    ///     "complete_address",
    ///     "Address must be complete or empty",
    ///     RuleSeverity::Warning,
    ///     vec!["PID-11-1", "PID-11-3", "PID-11-4", "PID-11-5"],
    /// );
    /// ```
    pub fn all_or_none(
        name: impl Into<String>,
        description: impl Into<String>,
        severity: RuleSeverity,
        fields: Vec<impl Into<String>>,
    ) -> ValidationRule {
        let fields: Vec<String> = fields.into_iter().map(|f| f.into()).collect();

        ValidationRule::new(name, description, severity).with_condition(move |msg| {
            let terser = Terser::new(msg);
            let mut valued_count = 0;
            let mut empty_count = 0;

            for field in &fields {
                if let Ok(Some(value)) = terser.get(field) {
                    if value.trim().is_empty() {
                        empty_count += 1;
                    } else {
                        valued_count += 1;
                    }
                } else {
                    empty_count += 1;
                }
            }

            let total = fields.len();
            // Pass if all valued or all empty
            valued_count == total || empty_count == total
        })
    }

    /// Create a "field equals value" rule: check if a field equals a specific value
    ///
    /// Occurrence semantics: an unindexed segment path (`OBX-5`)
    /// requires every occurrence to be valued, and at least one
    /// occurrence to exist; an indexed path (`OBX(2)-5`) checks that
    /// occurrence only.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
    /// use vyna_hl7_validator::rules::RuleSeverity;
    ///
    /// // Patient gender must be specified
    /// let rule = CrossFieldValidator::field_valued(
    ///     "gender_required",
    ///     "Patient gender must be specified",
    ///     RuleSeverity::Error,
    ///     "PID-8",
    /// );
    /// ```
    pub fn field_valued(
        name: impl Into<String>,
        description: impl Into<String>,
        severity: RuleSeverity,
        field: impl Into<String>,
    ) -> ValidationRule {
        let field = field.into();

        ValidationRule::new(name, description, severity).with_condition(move |msg| {
            let terser = Terser::new(msg);
            if let Some(values) = occurrence_values(msg, &terser, &field) {
                return !values.is_empty()
                    && values
                        .iter()
                        .all(|v| matches!(v, Some(s) if !s.trim().is_empty()));
            }
            if let Ok(Some(value)) = terser.get(&field) {
                !value.trim().is_empty()
            } else {
                false
            }
        })
    }

    /// Create a "field matches pattern" rule using simple pattern matching
    ///
    /// Occurrence semantics: an unindexed segment path (`OBX-2`)
    /// requires every valued occurrence to be in the set; absent or
    /// empty occurrences are OK. An indexed path (`OBX(2)-2`) checks
    /// that occurrence only.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
    /// use vyna_hl7_validator::rules::RuleSeverity;
    ///
    /// // Patient class must be valid
    /// let rule = CrossFieldValidator::field_in_set(
    ///     "valid_patient_class",
    ///     "Patient class must be I, O, or E",
    ///     RuleSeverity::Error,
    ///     "PV1-2",
    ///     vec!["I", "O", "E"],
    /// );
    /// ```
    pub fn field_in_set(
        name: impl Into<String>,
        description: impl Into<String>,
        severity: RuleSeverity,
        field: impl Into<String>,
        valid_values: Vec<impl Into<String>>,
    ) -> ValidationRule {
        let field = field.into();
        let valid_values: Vec<String> = valid_values.into_iter().map(|v| v.into()).collect();

        ValidationRule::new(name, description, severity).with_condition(move |msg| {
            let terser = Terser::new(msg);
            if let Some(values) = occurrence_values(msg, &terser, &field) {
                return values.iter().all(|v| match v {
                    None => true,
                    Some(s) if s.trim().is_empty() => true,
                    Some(s) => valid_values.iter().any(|x| x.as_str() == s.trim()),
                });
            }
            if let Ok(Some(value)) = terser.get(&field) {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return true; // Empty is OK (use field_valued for required checks)
                }
                valid_values.iter().any(|v| v.as_str() == trimmed)
            } else {
                true // Field not present is OK
            }
        })
    }

    /// Create a "dependent fields" rule: if field A is valued, then field B must also be valued
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_validator::rules::cross_field::CrossFieldValidator;
    /// use vyna_hl7_validator::rules::RuleSeverity;
    ///
    /// // If phone number is provided, phone type must also be provided
    /// let rule = CrossFieldValidator::dependent_fields(
    ///     "phone_requires_type",
    ///     "Phone number requires phone type",
    ///     RuleSeverity::Warning,
    ///     "PID-13",
    ///     "PID-13-2",
    /// );
    /// ```
    pub fn dependent_fields(
        name: impl Into<String>,
        description: impl Into<String>,
        severity: RuleSeverity,
        primary_field: impl Into<String>,
        dependent_field: impl Into<String>,
    ) -> ValidationRule {
        let primary_field = primary_field.into();
        let dependent_field = dependent_field.into();

        ValidationRule::new(name, description, severity).with_condition(move |msg| {
            let terser = Terser::new(msg);

            // Check if primary field is valued
            if let Ok(Some(primary_value)) = terser.get(&primary_field)
                && !primary_value.trim().is_empty()
            {
                // Primary is valued, check dependent
                if let Ok(Some(dependent_value)) = terser.get(&dependent_field) {
                    return !dependent_value.trim().is_empty();
                }
                return false; // Primary valued but dependent not
            }

            true // Primary not valued, so no dependency
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::{Field, Message, Segment};

    fn create_test_message() -> Message {
        let mut msg = Message::default();

        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msg.segments.push(msh);

        let mut pid = Segment::new("PID");
        pid.fields.push(Field::new()); // PID-1
        pid.fields.push(Field::from_value("123")); // PID-2
        pid.fields.push(Field::from_value("456")); // PID-3
        pid.fields.push(Field::new()); // PID-4
        pid.fields.push(Field::new()); // PID-5
        pid.fields.push(Field::new()); // PID-6
        pid.fields.push(Field::new()); // PID-7
        pid.fields.push(Field::from_value("M")); // PID-8
        msg.segments.push(pid);

        msg
    }

    #[test]
    fn test_if_then_passes() {
        let rule = CrossFieldValidator::if_then(
            "test",
            "Test if-then",
            RuleSeverity::Error,
            "PID-8",
            "M",
            "PID-2",
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_none()); // PID-8 = M and PID-2 is valued
    }

    #[test]
    fn test_if_then_fails() {
        let rule = CrossFieldValidator::if_then(
            "test",
            "Test if-then",
            RuleSeverity::Error,
            "PID-8",
            "M",
            "PID-7", // Empty field
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_some()); // PID-8 = M but PID-7 is empty
    }

    #[test]
    fn test_mutually_exclusive_passes() {
        let rule = CrossFieldValidator::mutually_exclusive(
            "test",
            "Test mutually exclusive",
            RuleSeverity::Error,
            vec!["PID-2", "PID-4"], // Only PID-2 is valued
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_none());
    }

    #[test]
    fn test_mutually_exclusive_fails() {
        let rule = CrossFieldValidator::mutually_exclusive(
            "test",
            "Test mutually exclusive",
            RuleSeverity::Error,
            vec!["PID-2", "PID-3"], // Both are valued
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_some());
    }

    #[test]
    fn test_at_least_one_passes() {
        let rule = CrossFieldValidator::at_least_one(
            "test",
            "Test at least one",
            RuleSeverity::Error,
            vec!["PID-2", "PID-3", "PID-4"],
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_none()); // PID-2 and PID-3 are valued
    }

    #[test]
    fn test_at_least_one_fails() {
        let rule = CrossFieldValidator::at_least_one(
            "test",
            "Test at least one",
            RuleSeverity::Error,
            vec!["PID-4", "PID-5", "PID-6"], // All empty
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_some());
    }

    #[test]
    fn test_field_valued_passes() {
        let rule = CrossFieldValidator::field_valued(
            "test",
            "Test field valued",
            RuleSeverity::Error,
            "PID-8",
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_none()); // PID-8 is valued
    }

    #[test]
    fn test_field_valued_fails() {
        let rule = CrossFieldValidator::field_valued(
            "test",
            "Test field valued",
            RuleSeverity::Error,
            "PID-7",
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_some()); // PID-7 is empty
    }

    #[test]
    fn test_field_in_set_passes() {
        let rule = CrossFieldValidator::field_in_set(
            "test",
            "Test field in set",
            RuleSeverity::Error,
            "PID-8",
            vec!["M", "F", "O"],
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_none()); // PID-8 = M is in set
    }

    #[test]
    fn test_field_in_set_fails() {
        let rule = CrossFieldValidator::field_in_set(
            "test",
            "Test field in set",
            RuleSeverity::Error,
            "PID-8",
            vec!["F", "O"], // M not in set
        );

        let msg = create_test_message();
        assert!(rule.evaluate(&msg).is_some());
    }

    fn create_obx_message(rows: &[(&str, &str, &str)]) -> Message {
        // Each row is (OBX-1 set ID, OBX-2 value type, OBX-5 value).
        let mut msg = Message::default();
        let mut msh = Segment::new("MSH");
        msh.fields.push(Field::from_value("|"));
        msg.segments.push(msh);
        for (set_id, value_type, value) in rows {
            let mut obx = Segment::new("OBX");
            obx.fields.push(Field::from_value(*set_id)); // OBX-1
            obx.fields.push(Field::from_value(*value_type)); // OBX-2
            obx.fields.push(Field::from_value("GLU")); // OBX-3
            obx.fields.push(Field::new()); // OBX-4
            obx.fields.push(Field::from_value(*value)); // OBX-5
            msg.segments.push(obx);
        }
        msg
    }

    #[test]
    fn test_field_in_set_checks_every_occurrence() {
        let rule = CrossFieldValidator::field_in_set(
            "obx_value_type_valid",
            "Value Type must be NM",
            RuleSeverity::Error,
            "OBX-2",
            vec!["NM"],
        );
        // First OBX is valid but the second is BOGUS: must fail.
        let msg = create_obx_message(&[("1", "NM", "98"), ("2", "BOGUS", "140")]);
        assert!(rule.evaluate(&msg).is_some());
        // All valid: must pass.
        let msg = create_obx_message(&[("1", "NM", "98"), ("2", "NM", "140")]);
        assert!(rule.evaluate(&msg).is_none());
    }

    #[test]
    fn test_field_valued_requires_every_occurrence() {
        let rule = CrossFieldValidator::field_valued(
            "obx_value_required",
            "Observation Value must be provided",
            RuleSeverity::Error,
            "OBX-5",
        );
        // Second OBX-5 is empty: must fail.
        let msg = create_obx_message(&[("1", "NM", "98"), ("2", "NM", "")]);
        assert!(rule.evaluate(&msg).is_some());
        // All valued: must pass.
        let msg = create_obx_message(&[("1", "NM", "98"), ("2", "NM", "140")]);
        assert!(rule.evaluate(&msg).is_none());
        // No OBX at all: must fail (at least one occurrence required).
        assert!(rule.evaluate(&create_test_message()).is_some());
    }

    #[test]
    fn test_indexed_paths_still_check_one_occurrence() {
        let msg = create_obx_message(&[("1", "NM", "98"), ("2", "BOGUS", "140")]);
        let first = CrossFieldValidator::field_in_set(
            "t",
            "t",
            RuleSeverity::Error,
            "OBX(1)-2",
            vec!["NM"],
        );
        assert!(first.evaluate(&msg).is_none());
        let second = CrossFieldValidator::field_in_set(
            "t",
            "t",
            RuleSeverity::Error,
            "OBX(2)-2",
            vec!["NM"],
        );
        assert!(second.evaluate(&msg).is_some());
    }

    #[test]
    fn test_field_in_set_absent_segment_still_passes() {
        let rule =
            CrossFieldValidator::field_in_set("t", "t", RuleSeverity::Error, "OBX-2", vec!["NM"]);
        assert!(rule.evaluate(&create_test_message()).is_none());
    }
}
