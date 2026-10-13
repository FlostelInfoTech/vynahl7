//! Conditional queries for filtering and finding segments
//!
//! This module provides the `TerserQuery` which allows finding segments
//! based on conditions and extracting field values conditionally.

use crate::Terser;
use vyna_hl7_core::{Field, Message, Segment};

/// Check whether any value held by `field` equals `value`.
///
/// Every repetition, component, and subcomponent is searched, so a
/// match in a later repetition (e.g. `AAA~GLU`) or a later component
/// (e.g. `XXX^GLU`) is found instead of silently missed.
fn field_contains(field: &Field, value: &str) -> bool {
    field.repetitions.iter().any(|rep| {
        rep.components
            .iter()
            .any(|comp| comp.subcomponents.iter().any(|sub| sub.as_str() == value))
    })
}

/// Query interface for conditional field access
///
/// # Examples
///
/// ```
/// use vyna_hl7_terser::TerserQuery;
/// use vyna_hl7_parser::parse_message;
///
/// # fn main() -> vyna_hl7_core::Result<()> {
/// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
/// OBX|1|NM|GLU||98|mg/dL
/// OBX|2|NM|NA||140|mmol/L
/// OBX|3|NM|K||4.2|mmol/L";
///
/// let message = parse_message(hl7)?;
/// let query = TerserQuery::new(&message);
///
/// // Find all OBX segments where observation value > 100
/// let high_values = query.find_segments("OBX", |seg| {
///     seg.get_field(5)
///         .and_then(|f| f.value())
///         .and_then(|v| v.parse::<f64>().ok())
///         .map(|n| n > 100.0)
///         .unwrap_or(false)
/// });
///
/// assert_eq!(high_values.len(), 1);
/// # Ok(())
/// # }
/// ```
pub struct TerserQuery<'a> {
    terser: Terser<'a>,
}

impl<'a> TerserQuery<'a> {
    /// Create a new query interface for the message
    pub fn new(message: &'a Message) -> Self {
        Self {
            terser: Terser::new(message),
        }
    }

    /// Get access to the underlying message
    fn message(&self) -> &'a Message {
        self.terser.message
    }

    /// Get a field value only if a condition is met
    ///
    /// The condition function receives a reference to a Terser for convenience.
    ///
    /// # Panics
    ///
    /// Panics when `path` is malformed (or the segment index is out
    /// of range) instead of masking the error as `None`: `None` means
    /// the condition failed or the addressed value is absent. Use
    /// [`Terser::get`](crate::Terser::get) for fallible access.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
    /// PID|1||PAT001||DOE^JOHN||19800101|M";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Get patient name only if gender is Male
    /// let name = query.get_if("PID-5-1", |terser| {
    ///     terser.get("PID-8").ok().flatten() == Some("M")
    /// });
    ///
    /// assert_eq!(name, Some("DOE"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_if(&self, path: &str, condition: impl Fn(&Terser) -> bool) -> Option<&str> {
        if condition(&self.terser) {
            match self.terser.get(path) {
                Ok(value) => value,
                Err(err) => panic!("TerserQuery::get_if: invalid path {path:?}: {err}"),
            }
        } else {
            None
        }
    }

    /// Find all segments matching a predicate
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140
    /// OBX|3|NM|K||4.2";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Find OBX segments with numeric values > 100
    /// let segments = query.find_segments("OBX", |seg| {
    ///     seg.get_field(5)
    ///         .and_then(|f| f.value())
    ///         .and_then(|v| v.parse::<f64>().ok())
    ///         .map(|n| n > 100.0)
    ///         .unwrap_or(false)
    /// });
    ///
    /// assert_eq!(segments.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_segments(
        &self,
        segment_id: &str,
        predicate: impl Fn(&Segment) -> bool,
    ) -> Vec<&Segment> {
        self.message()
            .get_segments_by_id(segment_id)
            .iter()
            .filter(|seg| predicate(seg))
            .copied()
            .collect()
    }

    /// Find repeating segments where a specific field equals a value
    ///
    /// Every repetition, component, and subcomponent of the field is
    /// searched: a match anywhere in the field selects the segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140
    /// OBX|3|NM|GLU||102";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Find all OBX segments where observation ID (field 3) = "GLU"
    /// let glucose_obs = query.filter_repeating("OBX", 3, "GLU");
    ///
    /// assert_eq!(glucose_obs.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter_repeating(&self, segment_id: &str, field: usize, value: &str) -> Vec<&Segment> {
        self.message()
            .get_segments_by_id(segment_id)
            .iter()
            .filter(|seg| {
                seg.get_field(field)
                    .map(|f| field_contains(f, value))
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    }

    /// Find the first segment where a field matches a value
    ///
    /// Every repetition, component, and subcomponent of the field is
    /// searched: a match anywhere in the field selects the segment.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140
    /// OBX|3|NM|K||4.2";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Find first OBX where observation ID = "NA"
    /// let na_obs = query.find_first("OBX", 3, "NA");
    ///
    /// assert!(na_obs.is_some());
    /// assert_eq!(na_obs.unwrap().get_field(5).and_then(|f| f.value()), Some("140"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_first(&self, segment_id: &str, field: usize, value: &str) -> Option<&Segment> {
        self.message()
            .get_segments_by_id(segment_id)
            .iter()
            .find(|seg| {
                seg.get_field(field)
                    .map(|f| field_contains(f, value))
                    .unwrap_or(false)
            })
            .copied()
    }

    /// Filter segments by component value
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|CE|GLU^Glucose||98^mg/dL
    /// OBX|2|CE|NA^Sodium||140^mmol/L
    /// OBX|3|CE|GLU^Glucose||102^mg/dL";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Find OBX segments where observation code (field 3, component 1) = "GLU"
    /// let glucose_obs = query.filter_by_component("OBX", 3, 1, "GLU");
    ///
    /// assert_eq!(glucose_obs.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn filter_by_component(
        &self,
        segment_id: &str,
        field: usize,
        component: usize,
        value: &str,
    ) -> Vec<&Segment> {
        self.message()
            .get_segments_by_id(segment_id)
            .iter()
            .filter(|seg| {
                seg.get_field(field)
                    .and_then(|f| f.get_repetition(0))
                    .and_then(|r| {
                        if component > 0 {
                            r.get_component(component - 1)
                        } else {
                            None
                        }
                    })
                    .and_then(|c| c.value())
                    .map(|v| v == value)
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    }

    /// Get values from all matching segments
    ///
    /// Segment matching searches every repetition, component, and
    /// subcomponent of the filter field (see [`filter_repeating`](Self::filter_repeating)).
    /// The value collected from each matching segment is the result
    /// field's primary value: the first component of its first
    /// repetition.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140
    /// OBX|3|NM|GLU||102";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Get all glucose values
    /// let glucose_values = query.get_values_where("OBX", 3, "GLU", 5);
    ///
    /// assert_eq!(glucose_values, vec!["98", "102"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_values_where(
        &self,
        segment_id: &str,
        filter_field: usize,
        filter_value: &str,
        result_field: usize,
    ) -> Vec<&str> {
        self.filter_repeating(segment_id, filter_field, filter_value)
            .iter()
            .filter_map(|seg| seg.get_field(result_field).and_then(|f| f.value()))
            .collect()
    }

    /// Count segments matching a condition
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140
    /// OBX|3|NM|K||4.2
    /// OBX|4|NM|CL||102";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Count observations with value > 100
    /// let count = query.count_where("OBX", |seg| {
    ///     seg.get_field(5)
    ///         .and_then(|f| f.value())
    ///         .and_then(|v| v.parse::<f64>().ok())
    ///         .map(|n| n > 100.0)
    ///         .unwrap_or(false)
    /// });
    ///
    /// assert_eq!(count, 2); // 140 and 102
    /// # Ok(())
    /// # }
    /// ```
    pub fn count_where(&self, segment_id: &str, predicate: impl Fn(&Segment) -> bool) -> usize {
        self.find_segments(segment_id, predicate).len()
    }

    /// Check if any segment matches a condition
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Check if any observation has value > 100
    /// let has_high_value = query.any_match("OBX", |seg| {
    ///     seg.get_field(5)
    ///         .and_then(|f| f.value())
    ///         .and_then(|v| v.parse::<f64>().ok())
    ///         .map(|n| n > 100.0)
    ///         .unwrap_or(false)
    /// });
    ///
    /// assert!(has_high_value);
    /// # Ok(())
    /// # }
    /// ```
    pub fn any_match(&self, segment_id: &str, predicate: impl Fn(&Segment) -> bool) -> bool {
        self.message()
            .get_segments_by_id(segment_id)
            .iter()
            .any(|seg| predicate(seg))
    }

    /// Check if all segments match a condition
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::TerserQuery;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140";
    ///
    /// let message = parse_message(hl7)?;
    /// let query = TerserQuery::new(&message);
    ///
    /// // Check if all observations have numeric values
    /// let all_numeric = query.all_match("OBX", |seg| {
    ///     seg.get_field(5)
    ///         .and_then(|f| f.value())
    ///         .and_then(|v| v.parse::<f64>().ok())
    ///         .is_some()
    /// });
    ///
    /// assert!(all_numeric);
    /// # Ok(())
    /// # }
    /// ```
    pub fn all_match(&self, segment_id: &str, predicate: impl Fn(&Segment) -> bool) -> bool {
        let segments = self.message().get_segments_by_id(segment_id);
        !segments.is_empty() && segments.iter().all(|seg| predicate(seg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_find_segments() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let high_values = query.find_segments("OBX", |seg| {
            seg.get_field(5)
                .and_then(|f| f.value())
                .and_then(|v| v.parse::<f64>().ok())
                .map(|n| n > 100.0)
                .unwrap_or(false)
        });

        assert_eq!(high_values.len(), 1);
    }

    #[test]
    fn test_filter_repeating() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|GLU||102";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let glucose_obs = query.filter_repeating("OBX", 3, "GLU");

        assert_eq!(glucose_obs.len(), 2);
    }

    #[test]
    fn test_find_first() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let na_obs = query.find_first("OBX", 3, "NA");

        assert!(na_obs.is_some());
        assert_eq!(
            na_obs.unwrap().get_field(5).and_then(|f| f.value()),
            Some("140")
        );
    }

    #[test]
    fn test_filter_by_component() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|CE|GLU^Glucose||98^mg/dL
OBX|2|CE|NA^Sodium||140^mmol/L
OBX|3|CE|GLU^Glucose||102^mg/dL";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let glucose_obs = query.filter_by_component("OBX", 3, 1, "GLU");

        assert_eq!(glucose_obs.len(), 2);
    }

    #[test]
    fn test_get_values_where() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|GLU||102";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let glucose_values = query.get_values_where("OBX", 3, "GLU", 5);

        assert_eq!(glucose_values, vec!["98", "102"]);
    }

    #[test]
    fn test_count_where() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2
OBX|4|NM|CL||102";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let count = query.count_where("OBX", |seg| {
            seg.get_field(5)
                .and_then(|f| f.value())
                .and_then(|v| v.parse::<f64>().ok())
                .map(|n| n > 100.0)
                .unwrap_or(false)
        });

        assert_eq!(count, 2);
    }

    #[test]
    fn test_any_match() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let has_high_value = query.any_match("OBX", |seg| {
            seg.get_field(5)
                .and_then(|f| f.value())
                .and_then(|v| v.parse::<f64>().ok())
                .map(|n| n > 100.0)
                .unwrap_or(false)
        });

        assert!(has_high_value);

        let has_negative = query.any_match("OBX", |seg| {
            seg.get_field(5)
                .and_then(|f| f.value())
                .and_then(|v| v.parse::<f64>().ok())
                .map(|n| n < 0.0)
                .unwrap_or(false)
        });

        assert!(!has_negative);
    }

    #[test]
    fn test_all_match() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let all_numeric = query.all_match("OBX", |seg| {
            seg.get_field(5)
                .and_then(|f| f.value())
                .and_then(|v| v.parse::<f64>().ok())
                .is_some()
        });

        assert!(all_numeric);

        let all_high = query.all_match("OBX", |seg| {
            seg.get_field(5)
                .and_then(|f| f.value())
                .and_then(|v| v.parse::<f64>().ok())
                .map(|n| n > 100.0)
                .unwrap_or(false)
        });

        assert!(!all_high); // 98 is not > 100
    }

    #[test]
    fn test_get_if() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        // Get patient name only if gender is Male
        let name = query.get_if("PID-5-1", |terser| {
            terser.get("PID-8").ok().flatten() == Some("M")
        });

        assert_eq!(name, Some("DOE"));

        // Try with a false condition
        let name_female = query.get_if("PID-5-1", |terser| {
            terser.get("PID-8").ok().flatten() == Some("F")
        });

        assert_eq!(name_female, None);
    }

    #[test]
    #[should_panic(expected = "invalid path")]
    fn test_get_if_propagates_malformed_path() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        // A malformed path panics like Terser::get errors, instead of
        // masking as None.
        let _ = query.get_if("PID-5-1-2-3-4", |_| true);
    }

    #[test]
    fn test_get_if_none_for_absent_or_failed_condition() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        // None is kept only for absent values and failed conditions.
        assert_eq!(query.get_if("PID-99", |_| true), None);
        assert_eq!(query.get_if("PID-5-1", |_| false), None);
        // A false condition short-circuits without touching the path.
        assert_eq!(query.get_if("PID-5-1-2-3-4", |_| false), None);
    }

    #[test]
    fn test_filter_repeating_searches_all_repetitions_and_components() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|AAA~GLU||98\r\
                   OBX|2|NM|XXX^GLU||140\r\
                   OBX|3|NM|K||4.2\r";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        // GLU sits in a later repetition (OBX-1) and a later
        // component (OBX-2): both must match.
        let hits = query.filter_repeating("OBX", 3, "GLU");
        assert_eq!(hits.len(), 2);

        // First-position matches keep working.
        assert_eq!(query.filter_repeating("OBX", 3, "K").len(), 1);
        assert!(query.filter_repeating("OBX", 3, "ZZZ").is_empty());
    }

    #[test]
    fn test_find_first_searches_all_repetitions_and_components() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|AAA~GLU||98\r\
                   OBX|2|NM|XXX^GLU||140\r";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        let found = query.find_first("OBX", 3, "GLU").unwrap();
        assert_eq!(found.get_field(5).and_then(|f| f.value()), Some("98"));
        assert!(query.find_first("OBX", 3, "ZZZ").is_none());
    }

    #[test]
    fn test_get_values_where_matches_beyond_first_value() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|AAA~GLU||98\r\
                   OBX|2|NM|NA||140\r";

        let message = parse_message(hl7).unwrap();
        let query = TerserQuery::new(&message);

        // Matched via the second repetition; the collected value is
        // the result field's primary value.
        assert_eq!(query.get_values_where("OBX", 3, "GLU", 5), vec!["98"]);
    }
}
