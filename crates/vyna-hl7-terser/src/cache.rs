//! Caching layer for Terser to improve performance of repeated field access
//!
//! This module provides a cache that stores parsed terser paths and field locations
//! to avoid repeated path parsing and segment lookups.

use crate::path::TerserPath;
use std::collections::HashMap;
use vyna_hl7_core::{
    error::{Error, Result},
    message::Message,
};

/// A cache entry for a terser path lookup
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The parsed path
    path: TerserPath,
    /// The actual segment index in the message
    segment_index: usize,
}

/// A caching terser that stores parsed paths and segment locations
pub struct CachedTerser<'a> {
    message: &'a Message,
    /// Cache of path string -> cache entry
    cache: HashMap<String, CacheEntry>,
}

impl<'a> CachedTerser<'a> {
    /// Create a new cached terser
    pub fn new(message: &'a Message) -> Self {
        Self {
            message,
            cache: HashMap::new(),
        }
    }

    /// Create a cached terser with pre-allocated capacity
    pub fn with_capacity(message: &'a Message, capacity: usize) -> Self {
        Self {
            message,
            cache: HashMap::with_capacity(capacity),
        }
    }

    /// Get a value using path notation with caching
    ///
    /// The first access to a path will parse and cache it. Subsequent accesses
    /// will use the cached parsed path, significantly improving performance.
    ///
    /// # Missing values vs errors
    ///
    /// Shares [`Terser::get`](crate::Terser::get)'s contract exactly:
    /// returns `Ok(None)` when the segment id is absent from the message or
    /// the addressed field, repetition, component, or subcomponent is
    /// absent; returns `Err` when the path is malformed or the segment
    /// index exceeds the number of occurrences of that segment id.
    pub fn get(&mut self, path: &str) -> Result<Option<&str>> {
        // Check if path is already cached
        let entry = if let Some(cached) = self.cache.get(path) {
            cached.clone() // Clone the cached entry
        } else {
            // Not in cache - parse and store
            let parsed_path = TerserPath::parse(path)?;

            // Find the segment
            let segments = self.message.get_segments_by_id(&parsed_path.segment_id);
            if segments.is_empty() {
                return Ok(None);
            }

            let actual_index = self
                .message
                .segments
                .iter()
                .enumerate()
                .filter(|(_, s)| s.id == parsed_path.segment_id)
                .nth(parsed_path.segment_index)
                .map(|(idx, _)| idx);

            if let Some(segment_index) = actual_index {
                // Cache the entry
                let entry = CacheEntry {
                    path: parsed_path,
                    segment_index,
                };
                self.cache.insert(path.to_string(), entry.clone());
                entry
            } else {
                // Same contract as Terser::get: an out-of-range segment
                // index is an error, not a missing value.
                return Err(Error::terser_path(format!(
                    "Segment index {} out of bounds for {}",
                    parsed_path.segment_index, parsed_path.segment_id
                )));
            }
        };

        // Get the value using the cached entry
        let segment = &self.message.segments[entry.segment_index];
        let field = match segment.get_field(entry.path.field_index) {
            Some(f) => f,
            None => return Ok(None),
        };

        Ok(self.get_field_value(
            field,
            entry.path.repetition_index,
            entry.path.component_index,
            entry.path.subcomponent_index,
        ))
    }

    /// Get a field value at the specified indices
    ///
    /// Note: Component and subcomponent indices are 1-based (HL7 standard notation)
    /// but internally converted to 0-based for array access.
    #[inline]
    fn get_field_value<'b>(
        &self,
        field: &'b vyna_hl7_core::field::Field,
        rep_idx: usize,
        comp_idx: Option<usize>,
        sub_idx: Option<usize>,
    ) -> Option<&'b str> {
        let repetition = field.get_repetition(rep_idx)?;

        match (comp_idx, sub_idx) {
            (None, None) => repetition.value(),
            (Some(c_idx), None) => {
                // Convert 1-based HL7 to 0-based internal
                if c_idx == 0 {
                    return None; // Invalid: HL7 uses 1-based indexing
                }
                repetition.get_component(c_idx - 1)?.value()
            }
            (Some(c_idx), Some(s_idx)) => {
                // Convert 1-based HL7 to 0-based internal
                if c_idx == 0 || s_idx == 0 {
                    return None; // Invalid: HL7 uses 1-based indexing
                }
                repetition
                    .get_component(c_idx - 1)?
                    .get_subcomponent(s_idx - 1)?
                    .as_str()
                    .into()
            }
            (None, Some(_)) => None,
        }
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get the number of cached entries
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Pre-warm the cache with commonly accessed paths
    ///
    /// Returns an error if any path is malformed or addresses an
    /// out-of-range segment index; paths addressing absent segments or
    /// fields warm as `None` without error.
    pub fn warm_cache(&mut self, paths: &[&str]) -> Result<()> {
        for path in paths {
            let _ = self.get(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_cached_terser_basic() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();

        // First verify what regular Terser returns
        let regular_terser = crate::Terser::new(&message);
        let expected = regular_terser.get("PID-5").unwrap();

        let mut terser = CachedTerser::new(&message);

        // First access - not cached
        let name = terser.get("PID-5").unwrap();
        assert_eq!(name, expected);
        assert_eq!(terser.cache_size(), 1);

        // Second access - cached
        let name = terser.get("PID-5").unwrap();
        assert_eq!(name, expected);
        assert_eq!(terser.cache_size(), 1);
    }

    #[test]
    fn test_cached_terser_components() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();

        // Verify with regular Terser using 1-based indexing (HL7 standard)
        let regular_terser = crate::Terser::new(&message);
        let expected_family = regular_terser.get("PID-5-1").unwrap(); // Family name
        let expected_given = regular_terser.get("PID-5-2").unwrap(); // Given name

        let mut terser = CachedTerser::new(&message);

        let family = terser.get("PID-5-1").unwrap(); // 1-based: first component
        assert_eq!(family, expected_family);

        let given = terser.get("PID-5-2").unwrap(); // 1-based: second component
        assert_eq!(given, expected_given);

        assert_eq!(terser.cache_size(), 2);
    }

    #[test]
    fn test_out_of_range_segment_index_matches_terser() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
                   OBX|1|NM|GLU||98\r";

        let message = parse_message(hl7).unwrap();
        let terser = crate::Terser::new(&message);
        let mut cached = CachedTerser::new(&message);

        // Out-of-range segment index is an error in both implementations.
        assert!(terser.get("OBX(5)-5").is_err());
        assert!(cached.get("OBX(5)-5").is_err());

        // Absent segment id is Ok(None) in both implementations.
        assert_eq!(terser.get("ZZZ-1").unwrap(), None);
        assert_eq!(cached.get("ZZZ-1").unwrap(), None);

        // Absent field is Ok(None) in both implementations.
        assert_eq!(terser.get("OBX-99").unwrap(), None);
        assert_eq!(cached.get("OBX-99").unwrap(), None);

        // Absent component is Ok(None) in both implementations.
        assert_eq!(terser.get("OBX-5-9").unwrap(), None);
        assert_eq!(cached.get("OBX-5-9").unwrap(), None);
    }

    #[test]
    fn test_warm_cache() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123||DOE^JOHN||19800101|M";

        let message = parse_message(hl7).unwrap();
        let mut terser = CachedTerser::new(&message);

        // Use 1-based indexing (HL7 standard)
        terser
            .warm_cache(&["PID-5", "PID-5-1", "PID-5-2", "PID-7", "PID-8"])
            .unwrap();
        assert_eq!(terser.cache_size(), 5);

        // All subsequent accesses should be cached
        let _ = terser.get("PID-5").unwrap();
        assert_eq!(terser.cache_size(), 5);
    }
}
