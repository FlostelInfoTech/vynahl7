//! Bulk field extraction for efficient access to multiple fields at once.
//!
//! This module provides the `BulkTerser` which allows extracting multiple
//! field values in a single operation, as well as pattern-based extraction
//! for repeating segments.

use crate::Terser;
use std::collections::HashMap;

/// Bulk terser for extracting multiple fields efficiently
///
/// # Examples
///
/// ```
/// use vyna_hl7_terser::BulkTerser;
/// use vyna_hl7_parser::parse_message;
///
/// # fn main() -> vyna_hl7_core::Result<()> {
/// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
/// PID|1||PAT001||DOE^JOHN^A||19800101|M";
///
/// let message = parse_message(hl7)?;
/// let bulk = BulkTerser::new(&message);
///
/// // Extract multiple fields at once
/// let paths = vec!["PID-5-1", "PID-5-2", "PID-7", "PID-8"];
/// let values = bulk.get_multiple(&paths)?;
///
/// assert_eq!(values.get("PID-5-1"), Some(&Some("DOE")));
/// assert_eq!(values.get("PID-5-2"), Some(&Some("JOHN")));
/// assert_eq!(values.get("PID-7"), Some(&Some("19800101")));
/// assert_eq!(values.get("PID-8"), Some(&Some("M")));
/// # Ok(())
/// # }
/// ```
pub struct BulkTerser<'a> {
    terser: Terser<'a>,
}

impl<'a> BulkTerser<'a> {
    /// Create a new bulk terser for the given message
    pub fn new(message: &'a vyna_hl7_core::Message) -> Self {
        Self {
            terser: Terser::new(message),
        }
    }

    /// Extract multiple field values at once
    ///
    /// Returns a HashMap where keys are the paths and values are the extracted field values.
    /// If a path is invalid or the field doesn't exist, the value will be None.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::BulkTerser;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
    /// PID|1||PAT001||DOE^JOHN||19800101|M";
    ///
    /// let message = parse_message(hl7)?;
    /// let bulk = BulkTerser::new(&message);
    ///
    /// let paths = vec!["PID-5-1", "PID-5-2", "PID-7", "PID-8"];
    /// let values = bulk.get_multiple(&paths)?;
    ///
    /// // All requested fields are in the result
    /// assert_eq!(values.len(), 4);
    /// assert_eq!(values.get("PID-5-1"), Some(&Some("DOE")));
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_multiple(
        &self,
        paths: &[&str],
    ) -> vyna_hl7_core::Result<HashMap<String, Option<&str>>> {
        let mut result = HashMap::with_capacity(paths.len());

        for path in paths {
            let value = self.terser.get(path)?;
            result.insert(path.to_string(), value);
        }

        Ok(result)
    }

    /// Extract all values matching a glob pattern
    ///
    /// Supports patterns with wildcards for segment indices:
    /// - `OBX(*)-5` - Get field 5 from all OBX segments
    /// - `PID-11(*)-1` - Get component 1 from all repetitions of PID field 11
    ///
    /// Returns a vector of tuples (resolved_path, value) for all matching fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::BulkTerser;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98|mg/dL
    /// OBX|2|NM|NA||140|mmol/L
    /// OBX|3|NM|K||4.2|mmol/L";
    ///
    /// let message = parse_message(hl7)?;
    /// let bulk = BulkTerser::new(&message);
    ///
    /// // Get observation values from all OBX segments
    /// let values = bulk.get_pattern("OBX(*)-5")?;
    ///
    /// assert_eq!(values.len(), 3);
    /// assert_eq!(values[0].0, "OBX(1)-5");
    /// assert_eq!(values[0].1, "98");
    /// assert_eq!(values[1].0, "OBX(2)-5");
    /// assert_eq!(values[1].1, "140");
    /// assert_eq!(values[2].0, "OBX(3)-5");
    /// assert_eq!(values[2].1, "4.2");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_pattern(&self, pattern: &str) -> vyna_hl7_core::Result<Vec<(String, &str)>> {
        // Parse the pattern to identify segment wildcard or field repetition wildcard
        if let Some(wildcard_pos) = pattern.find("(*)") {
            // Determine if it's a segment wildcard or field repetition wildcard
            let before_wildcard = &pattern[..wildcard_pos];

            // Check if this is a segment wildcard (e.g., "OBX(*)-5" or "NK1(*)-2")
            // Segment wildcard: no hyphens before the wildcard, only segment ID
            if !before_wildcard.contains('-')
                && before_wildcard
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            {
                self.expand_segment_wildcard(pattern, before_wildcard)
            } else {
                // Field repetition wildcard (e.g., "PID-11(*)-1")
                self.expand_field_repetition_wildcard(pattern)
            }
        } else {
            Err(vyna_hl7_core::Error::terser_path(
                "Pattern must contain (*) wildcard".to_string(),
            ))
        }
    }

    /// Expand segment wildcard pattern (e.g., "OBX(*)-5")
    fn expand_segment_wildcard(
        &self,
        pattern: &str,
        segment_id: &str,
    ) -> vyna_hl7_core::Result<Vec<(String, &str)>> {
        let after_wildcard = &pattern[segment_id.len() + 3..]; // Skip "SEG(*)"
        let mut results = Vec::new();

        // Get all segments with this ID
        let segments = self.terser.message.get_segments_by_id(segment_id);

        // Extract value from each segment
        for (idx, _segment) in segments.iter().enumerate() {
            let segment_index = idx + 1; // 1-based indexing
            let resolved_path = format!("{}({}){}", segment_id, segment_index, after_wildcard);

            if let Some(value) = self.terser.get(&resolved_path)? {
                results.push((resolved_path, value));
            }
        }

        Ok(results)
    }

    /// Expand field repetition wildcard pattern (e.g., "PID-11(*)-1")
    fn expand_field_repetition_wildcard(
        &self,
        pattern: &str,
    ) -> vyna_hl7_core::Result<Vec<(String, &str)>> {
        // Parse the pattern to extract segment, field, and component parts
        // Pattern format: "SEGMENT-FIELD(*)-COMPONENT"
        let parts: Vec<&str> = pattern.split('-').collect();

        if parts.len() < 3 {
            return Err(vyna_hl7_core::Error::terser_path(
                "Invalid field repetition pattern".to_string(),
            ));
        }

        let segment_part = parts[0]; // e.g., "PID"
        // e.g., "11" from "11(*)".
        let field_part = parts[1].strip_suffix("(*)").unwrap_or(parts[1]);
        let after_wildcard = &parts[2..].join("-"); // e.g., "1" or "1-2"

        // Extract segment ID and optional index (1-based in patterns).
        let (segment_id, segment_idx) = if let Some(paren_pos) = segment_part.find('(') {
            let id = &segment_part[..paren_pos];
            let idx_str = segment_part
                .get(paren_pos + 1..)
                .and_then(|s| s.strip_suffix(')'));
            let idx_str = idx_str.ok_or_else(|| {
                vyna_hl7_core::Error::terser_path(
                    "Invalid segment index: missing closing parenthesis".to_string(),
                )
            })?;
            let idx: usize = idx_str.parse().map_err(|_| {
                vyna_hl7_core::Error::terser_path("Invalid segment index".to_string())
            })?;
            if idx == 0 {
                return Err(vyna_hl7_core::Error::terser_path(
                    "Invalid segment index 0: patterns use 1-based indexing".to_string(),
                ));
            }
            (id, idx - 1) // Convert to 0-based
        } else {
            (segment_part, 0)
        };

        let field_idx: usize = field_part
            .parse()
            .map_err(|_| vyna_hl7_core::Error::terser_path("Invalid field index".to_string()))?;

        let mut results = Vec::new();

        // Get the segment
        let segments = self.terser.message.get_segments_by_id(segment_id);
        if segments.is_empty() || segment_idx >= segments.len() {
            return Ok(results);
        }

        let segment = &segments[segment_idx];

        // Get the field
        if let Some(field) = segment.get_field(field_idx) {
            // Iterate over all repetitions
            for (rep_idx, _rep) in field.repetitions.iter().enumerate() {
                // Terser paths are 1-based; rep_idx is the 0-based Vec position.
                let resolved_path = if segment_idx == 0 {
                    format!(
                        "{}-{}({})-{}",
                        segment_id,
                        field_idx,
                        rep_idx + 1,
                        after_wildcard
                    )
                } else {
                    format!(
                        "{}({})-{}({})-{}",
                        segment_id,
                        segment_idx + 1,
                        field_idx,
                        rep_idx + 1,
                        after_wildcard
                    )
                };

                if let Some(value) = self.terser.get(&resolved_path)? {
                    results.push((resolved_path, value));
                }
            }
        }

        Ok(results)
    }

    /// Get all values for a specific field across all instances of a repeating segment
    ///
    /// This is a convenience method equivalent to `get_pattern("SEGMENT(*)-FIELD")`.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::BulkTerser;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140
    /// OBX|3|NM|K||4.2";
    ///
    /// let message = parse_message(hl7)?;
    /// let bulk = BulkTerser::new(&message);
    ///
    /// // Get all observation values (field 5) from OBX segments
    /// let values = bulk.get_all_from_segments("OBX", 5)?;
    ///
    /// assert_eq!(values, vec!["98", "140", "4.2"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_all_from_segments(
        &self,
        segment_id: &str,
        field: usize,
    ) -> vyna_hl7_core::Result<Vec<&str>> {
        let pattern = format!("{}(*)-{}", segment_id, field);
        let results = self.get_pattern(&pattern)?;
        Ok(results.into_iter().map(|(_, value)| value).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_get_multiple_basic() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN^A||19800101|M";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        let paths = vec!["PID-5-1", "PID-5-2", "PID-7", "PID-8"];
        let values = bulk.get_multiple(&paths).unwrap();

        assert_eq!(values.len(), 4);
        assert_eq!(values.get("PID-5-1"), Some(&Some("DOE")));
        assert_eq!(values.get("PID-5-2"), Some(&Some("JOHN")));
        assert_eq!(values.get("PID-7"), Some(&Some("19800101")));
        assert_eq!(values.get("PID-8"), Some(&Some("M")));
    }

    #[test]
    fn test_get_multiple_with_missing_fields() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        let paths = vec!["PID-5-1", "PID-7", "PID-99"];
        let values = bulk.get_multiple(&paths).unwrap();

        assert_eq!(values.len(), 3);
        assert_eq!(values.get("PID-5-1"), Some(&Some("DOE")));
        assert_eq!(values.get("PID-7"), Some(&None)); // Field exists but empty
        assert_eq!(values.get("PID-99"), Some(&None)); // Field doesn't exist
    }

    #[test]
    fn test_get_pattern_segment_wildcard() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98|mg/dL
OBX|2|NM|NA||140|mmol/L
OBX|3|NM|K||4.2|mmol/L";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        let values = bulk.get_pattern("OBX(*)-5").unwrap();

        assert_eq!(values.len(), 3);
        assert_eq!(values[0].0, "OBX(1)-5");
        assert_eq!(values[0].1, "98");
        assert_eq!(values[1].0, "OBX(2)-5");
        assert_eq!(values[1].1, "140");
        assert_eq!(values[2].0, "OBX(3)-5");
        assert_eq!(values[2].1, "4.2");
    }

    #[test]
    fn test_get_pattern_segment_wildcard_with_component() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|CE|GLU||98^mg/dL
OBX|2|CE|NA||140^mmol/L
OBX|3|CE|K||4.2^mmol/L";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // Get first component of observation value
        let values = bulk.get_pattern("OBX(*)-5-1").unwrap();

        assert_eq!(values.len(), 3);
        assert_eq!(values[0].1, "98");
        assert_eq!(values[1].1, "140");
        assert_eq!(values[2].1, "4.2");

        // Get second component (units)
        let units = bulk.get_pattern("OBX(*)-5-2").unwrap();

        assert_eq!(units.len(), 3);
        assert_eq!(units[0].1, "mg/dL");
        assert_eq!(units[1].1, "mmol/L");
        assert_eq!(units[2].1, "mmol/L");
    }

    #[test]
    fn test_get_pattern_field_repetition_wildcard() {
        let hl7 =
            "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001||DOE^JOHN~SMITH^JANE\r";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // Repetition paths are 1-based: (1) is the first repetition.
        let values = bulk.get_pattern("PID-5(*)-1").unwrap();

        assert_eq!(values.len(), 2);
        assert_eq!(values[0].0, "PID-5(1)-1");
        assert_eq!(values[0].1, "DOE");
        assert_eq!(values[1].0, "PID-5(2)-1");
        assert_eq!(values[1].1, "SMITH");
    }

    #[test]
    fn test_get_pattern_field_wildcard_zero_segment_index_errors() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN~SMITH^JANE";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // Segment indices are 1-based; (0) must be an error, not a panic.
        assert!(bulk.get_pattern("PID(0)-5(*)-1").is_err());
    }

    #[test]
    fn test_get_pattern_field_wildcard_malformed_segment_errors() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN~SMITH^JANE";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // Malformed segment parts must be errors, not slice panics.
        for pattern in [
            "PID(-5(*)-1",
            "PID(2-5(*)-1",
            "PID()-5(*)-1",
            "PID(x)-5(*)-1",
            "PID(1-5(*)-1",
            "(-5(*)-1",
        ] {
            assert!(bulk.get_pattern(pattern).is_err(), "{pattern}");
        }
    }

    #[test]
    fn test_get_pattern_field_wildcard_indexed_segment() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1||PAT001||DOE^JOHN~SMITH^JANE\r\
                   PID|2||PAT002||WAYNE^BRUCE~KENT^CLARK\r";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // Indexed segment part selects the second PID (1-based).
        let values = bulk.get_pattern("PID(2)-5(*)-1").unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0].0, "PID(2)-5(1)-1");
        assert_eq!(values[0].1, "WAYNE");
        assert_eq!(values[1].0, "PID(2)-5(2)-1");
        assert_eq!(values[1].1, "KENT");
    }

    #[test]
    fn test_get_pattern_no_wildcard_error() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        let result = bulk.get_pattern("PID-5-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_pattern_empty_result() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // No OBX segments in the message
        let values = bulk.get_pattern("OBX(*)-5").unwrap();

        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_get_all_from_segments() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        let values = bulk.get_all_from_segments("OBX", 5).unwrap();

        assert_eq!(values, vec!["98", "140", "4.2"]);
    }

    #[test]
    fn test_get_all_from_segments_with_components() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
NK1|1|DOE^JANE|SPOUSE
NK1|2|SMITH^BOB|FATHER";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // Get all NK1-2 field values (first component - last names)
        // Note: Field access without component spec returns first component only
        let last_names = bulk.get_all_from_segments("NK1", 2).unwrap();

        assert_eq!(last_names.len(), 2);
        assert_eq!(last_names[0], "DOE");
        assert_eq!(last_names[1], "SMITH");

        // To get specific components, use pattern with component index
        let first_names = bulk.get_pattern("NK1(*)-2-2").unwrap();
        assert_eq!(first_names.len(), 2);
        assert_eq!(first_names[0].1, "JANE");
        assert_eq!(first_names[1].1, "BOB");
    }

    #[test]
    fn test_bulk_extraction_performance() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
PID|1||PAT001||DOE^JOHN^A||19800101|M|||123 Main St^^Boston^MA^02101||555-1234
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2
OBX|4|NM|CL||102
OBX|5|NM|CO2||24";

        let message = parse_message(hl7).unwrap();
        let bulk = BulkTerser::new(&message);

        // Extract many fields at once
        let paths = vec![
            "PID-5-1", "PID-5-2", "PID-7", "PID-8", "PID-11-1", "PID-11-3", "PID-11-4", "PID-11-5",
            "PID-13",
        ];

        let values = bulk.get_multiple(&paths).unwrap();

        assert_eq!(values.len(), 9);
        assert_eq!(values.get("PID-5-1"), Some(&Some("DOE")));
        assert_eq!(values.get("PID-11-3"), Some(&Some("Boston")));
    }
}
