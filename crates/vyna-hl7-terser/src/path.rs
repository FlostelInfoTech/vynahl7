//! Terser path parsing module
//!
//! This module handles parsing of terser path strings like "PID-5-1" or "OBX(2)-3-1"

use vyna_hl7_core::error::{Error, Result};
use vyna_hl7_core::field::Field;

/// Read one value out of a parsed field.
///
/// Repetition selection is 0-based (internal position); component and
/// subcomponent selections are 1-based (HL7 notation), so `Some(0)` and a
/// subcomponent without a component match nothing. Shared by [`crate::Terser`]
/// and [`crate::CachedTerser`] so both keep the same contract.
#[inline]
pub(crate) fn resolve_field_value(
    field: &Field,
    rep_idx: usize,
    comp_idx: Option<usize>,
    sub_idx: Option<usize>,
) -> Option<&str> {
    let repetition = field.get_repetition(rep_idx)?;

    match (comp_idx, sub_idx) {
        (None, None) => repetition.value(),
        (Some(c_idx), None) => {
            if c_idx == 0 {
                return None; // Invalid: HL7 uses 1-based indexing
            }
            repetition.get_component(c_idx - 1)?.value()
        }
        (Some(c_idx), Some(s_idx)) => {
            if c_idx == 0 || s_idx == 0 {
                return None; // Invalid: HL7 uses 1-based indexing
            }
            repetition
                .get_component(c_idx - 1)?
                .get_subcomponent(s_idx - 1)?
                .as_str()
                .into()
        }
        (None, Some(_)) => {
            // Invalid: can't have subcomponent without component
            None
        }
    }
}

/// Parsed terser path
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerserPath {
    pub segment_id: String,
    pub segment_index: usize,
    pub field_index: usize,
    pub repetition_index: usize,
    pub component_index: Option<usize>,
    pub subcomponent_index: Option<usize>,
}

impl TerserPath {
    /// Parse a terser path string
    ///
    /// Format: SEGMENT[(index)]-FIELD[(rep)]-COMPONENT-SUBCOMPONENT
    ///
    /// Examples:
    /// - PID-5 (field 5 of first PID)
    /// - PID-5-1 (field 5, component 1)
    /// - PID-5-1-2 (field 5, component 1, subcomponent 2)
    /// - OBX(2)-5 (field 5 of second OBX, 1-indexed)
    /// - PID-11(1)-1 (field 11, first repetition, component 1)
    pub fn parse(path: &str) -> Result<Self> {
        let parts: Vec<&str> = path.split('-').collect();

        if parts.is_empty() {
            return Err(Error::terser_path("Empty path"));
        }

        // Parse segment part (e.g., "PID" or "OBX(2)")
        let (segment_id, segment_index) = Self::parse_segment_part(parts[0])?;

        // An empty segment id is malformed, not a wildcard: without
        // this, set("-5", x) would append a garbage segment with id "".
        if segment_id.trim().is_empty() {
            return Err(Error::terser_path("Empty segment id"));
        }
        // Interior whitespace is malformed too: without this, set("PID
        // -5", x) would create segment id "PID ", which fails validate_id.
        if segment_id.chars().any(|c| c.is_whitespace()) {
            return Err(Error::terser_path(
                "Invalid segment id: must not contain whitespace",
            ));
        }

        // Parse field part (e.g., "5" or "11(1)")
        if parts.len() < 2 {
            return Err(Error::terser_path("Missing field index"));
        }

        let (field_index, repetition_index) = Self::parse_field_part(parts[1])?;

        // Parse component index (optional)
        let component_index = if parts.len() >= 3 {
            Some(Self::parse_index(parts[2])?)
        } else {
            None
        };

        // Parse subcomponent index (optional)
        let subcomponent_index = if parts.len() >= 4 {
            Some(Self::parse_index(parts[3])?)
        } else {
            None
        };

        // Anything beyond SEGMENT-FIELD-COMPONENT-SUBCOMPONENT is
        // malformed: silently dropping it would address the wrong value.
        if parts.len() > 4 {
            return Err(Error::terser_path("Too many path parts"));
        }

        Ok(TerserPath {
            segment_id,
            segment_index,
            field_index,
            repetition_index,
            component_index,
            subcomponent_index,
        })
    }

    /// Parse segment part (e.g., "PID" or "OBX(2)")
    /// Note: Uses 1-based indexing in API (OBX(1) = first, OBX(2) = second)
    /// but converts to 0-based internally for array access
    fn parse_segment_part(part: &str) -> Result<(String, usize)> {
        if let Some(paren_pos) = part.find('(') {
            let id = part[..paren_pos].to_string();
            let index_str = &part[paren_pos + 1..];

            if let Some(close_paren) = index_str.find(')') {
                // Reject trailing garbage such as "OBX(2)x": only the
                // index may sit between the parentheses.
                if !index_str[close_paren + 1..].is_empty() {
                    return Err(Error::terser_path(
                        "Trailing characters after segment index",
                    ));
                }
                let index = index_str[..close_paren]
                    .parse::<usize>()
                    .map_err(|_| Error::terser_path("Invalid segment index"))?;

                // Validate 1-based index (must be > 0)
                if index == 0 {
                    return Err(Error::terser_path(
                        "Invalid segment index 0: HL7 uses 1-based indexing (use 1 for first segment)",
                    ));
                }

                // Convert 1-based to 0-based for internal use
                Ok((id, index - 1))
            } else {
                Err(Error::terser_path("Missing closing parenthesis"))
            }
        } else {
            // No index specified = first segment (internal index 0)
            Ok((part.to_string(), 0))
        }
    }

    /// Parse field part (e.g., "5" or "11(1)")
    /// Note: Uses 1-based indexing in API (PID-5(1) = first repetition)
    /// but converts to 0-based internally for array access, mirroring
    /// segment indices.
    fn parse_field_part(part: &str) -> Result<(usize, usize)> {
        if let Some(paren_pos) = part.find('(') {
            let field_str = &part[..paren_pos];
            let rep_str = &part[paren_pos + 1..];

            let field_index = Self::parse_field_index(field_str)?;

            if let Some(close_paren) = rep_str.find(')') {
                // Reject trailing garbage such as "5(2)x": only the
                // index may sit between the parentheses.
                if !rep_str[close_paren + 1..].is_empty() {
                    return Err(Error::terser_path(
                        "Trailing characters after repetition index",
                    ));
                }
                let rep_index = rep_str[..close_paren]
                    .parse::<usize>()
                    .map_err(|_| Error::terser_path("Invalid repetition index"))?;

                // Validate 1-based index (must be > 0)
                if rep_index == 0 {
                    return Err(Error::terser_path(
                        "Invalid repetition index 0: HL7 uses 1-based indexing (use 1 for first repetition)",
                    ));
                }

                // Convert 1-based to 0-based for internal use
                Ok((field_index, rep_index - 1))
            } else {
                Err(Error::terser_path("Missing closing parenthesis"))
            }
        } else {
            let field_index = Self::parse_field_index(part)?;
            // No repetition specified = first repetition (internal index 0)
            Ok((field_index, 0))
        }
    }

    /// Parse a 1-based field number; field 0 is the segment id, never a field
    fn parse_field_index(field_str: &str) -> Result<usize> {
        let field_index = field_str
            .parse::<usize>()
            .map_err(|_| Error::terser_path("Invalid field index"))?;
        if field_index == 0 {
            return Err(Error::terser_path(
                "Invalid field index 0: HL7 uses 1-based indexing (field 0 is the segment id)",
            ));
        }
        Ok(field_index)
    }

    /// Parse a simple numeric index
    fn parse_index(part: &str) -> Result<usize> {
        part.parse::<usize>()
            .map_err(|_| Error::terser_path("Invalid index"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_field() {
        let path = TerserPath::parse("PID-5").unwrap();
        assert_eq!(path.segment_id, "PID");
        assert_eq!(path.segment_index, 0);
        assert_eq!(path.field_index, 5);
        assert_eq!(path.repetition_index, 0);
        assert_eq!(path.component_index, None);
        assert_eq!(path.subcomponent_index, None);
    }

    #[test]
    fn test_parse_with_component() {
        let path = TerserPath::parse("PID-5-1").unwrap();
        assert_eq!(path.segment_id, "PID");
        assert_eq!(path.field_index, 5);
        assert_eq!(path.component_index, Some(1));
    }

    #[test]
    fn test_parse_with_subcomponent() {
        let path = TerserPath::parse("PID-5-1-2").unwrap();
        assert_eq!(path.component_index, Some(1));
        assert_eq!(path.subcomponent_index, Some(2));
    }

    #[test]
    fn test_parse_segment_index() {
        // OBX(2) = second OBX segment (1-based) = internal index 1 (0-based)
        let path = TerserPath::parse("OBX(2)-5").unwrap();
        assert_eq!(path.segment_id, "OBX");
        assert_eq!(path.segment_index, 1); // Internal 0-based index
        assert_eq!(path.field_index, 5);

        // OBX(1) = first OBX segment (1-based) = internal index 0 (0-based)
        let path = TerserPath::parse("OBX(1)-5").unwrap();
        assert_eq!(path.segment_index, 0); // Internal 0-based index

        // OBX without index = first OBX segment = internal index 0
        let path = TerserPath::parse("OBX-5").unwrap();
        assert_eq!(path.segment_index, 0); // Internal 0-based index
    }

    #[test]
    fn test_parse_segment_index_zero_is_invalid() {
        // OBX(0) should be invalid (HL7 uses 1-based indexing)
        let result = TerserPath::parse("OBX(0)-5");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_repetition_index() {
        // PID-11(1) = first repetition (1-based) = internal index 0 (0-based)
        let path = TerserPath::parse("PID-11(1)-1").unwrap();
        assert_eq!(path.field_index, 11);
        assert_eq!(path.repetition_index, 0); // Internal 0-based index
        assert_eq!(path.component_index, Some(1));

        // PID-11(2) = second repetition (1-based) = internal index 1
        let path = TerserPath::parse("PID-11(2)-1").unwrap();
        assert_eq!(path.repetition_index, 1); // Internal 0-based index

        // PID-11 without repetition = first repetition = internal index 0
        let path = TerserPath::parse("PID-11-1").unwrap();
        assert_eq!(path.repetition_index, 0); // Internal 0-based index
    }

    #[test]
    fn test_parse_repetition_index_zero_is_invalid() {
        // PID-11(0) should be invalid (HL7 uses 1-based indexing)
        let result = TerserPath::parse("PID-11(0)-1");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_field_index_zero_is_invalid() {
        // Field 0 is the segment id, never an addressable field.
        assert!(TerserPath::parse("PID-0").is_err());
        assert!(TerserPath::parse("PID-00").is_err());
        assert!(TerserPath::parse("PID-0(1)-1").is_err());
        assert!(TerserPath::parse("PID-0-1").is_err());
        // Nearby valid paths still parse.
        assert!(TerserPath::parse("PID-1").is_ok());
        assert!(TerserPath::parse("PID-10(1)-1").is_ok());
    }

    #[test]
    fn test_parse_rejects_trailing_garbage_after_segment_index() {
        assert!(TerserPath::parse("OBX(2)x-5").is_err());
        assert!(TerserPath::parse("OBX(2)(3)-5").is_err());
        assert!(TerserPath::parse("OBX(2) -5").is_err());
        // Clean indexed paths still parse.
        assert!(TerserPath::parse("OBX(2)-5").is_ok());
    }

    #[test]
    fn test_parse_rejects_trailing_garbage_after_repetition_index() {
        assert!(TerserPath::parse("PID-11(2)x-1").is_err());
        assert!(TerserPath::parse("PID-11(2)(3)-1").is_err());
        // Clean indexed paths still parse.
        assert!(TerserPath::parse("PID-11(2)-1").is_ok());
    }

    #[test]
    fn test_parse_rejects_empty_segment_id() {
        // An empty or whitespace-only segment id is malformed: without
        // this, TerserMut::set("-5", x) appends a garbage segment.
        assert!(TerserPath::parse("-5").is_err());
        assert!(TerserPath::parse("   -5").is_err());
        assert!(TerserPath::parse("(1)-5").is_err());
        assert!(TerserPath::parse("\t-5-1").is_err());
        // Ordinary ids still parse.
        assert!(TerserPath::parse("PID-5").is_ok());
    }

    #[test]
    fn test_parse_rejects_parts_beyond_subcomponent() {
        assert!(TerserPath::parse("PID-5-1-2-3").is_err());
        assert!(TerserPath::parse("PID-5-1-2-3-4").is_err());
        // A full four-part path still parses.
        assert!(TerserPath::parse("PID-5-1-2").is_ok());
    }

    #[test]
    fn test_parse_rejects_segment_id_with_whitespace() {
        // "PID " would create a segment failing validate_id (3 alnum).
        assert!(TerserPath::parse("PID -5").is_err());
        assert!(TerserPath::parse("P ID-5").is_err());
        assert!(TerserPath::parse("PID\t-5").is_err());
        assert!(TerserPath::parse("PID-5").is_ok());
    }
}
