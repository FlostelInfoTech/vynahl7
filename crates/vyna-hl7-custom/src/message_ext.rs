//! Message extensions for custom Z-segment support
//!
//! This module provides extension methods for `vyna_hl7_core::Message` that enable
//! extracting and working with custom Z-segments defined using the z_segment! macro.

use crate::error::Result;
use crate::segment::CustomSegment;
use vyna_hl7_core::message::Message;

/// Extension trait for Message to support custom Z-segments
///
/// This trait provides methods to extract typed custom segments from parsed messages.
///
/// # Example
///
/// ```rust,ignore
/// use vyna_hl7_custom::{MessageExt, z_segment};
/// use vyna_hl7_parser::parse_message;
///
/// z_segment! {
///     ZPV,
///     id = "ZPV",
///     fields = {
///         1 => visit_type: String,
///         2 => visit_number: String,
///     }
/// }
///
/// let message = parse_message(msg_str)?;
///
/// // Extract a single custom segment
/// if let Some(zpv) = message.get_custom_segment::<ZPV>()? {
///     println!("Visit type: {}", zpv.visit_type);
/// }
///
/// // Extract all occurrences
/// let all_zpvs = message.get_custom_segments::<ZPV>()?;
/// ```
pub trait MessageExt {
    /// Extract the first occurrence of a custom segment by type
    ///
    /// # Returns
    ///
    /// - `Ok(Some(T))` if the segment is found and parses successfully
    /// - `Ok(None)` if no segment with this ID is found
    /// - `Err(...)` if parsing or validation fails
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(zpv) = message.get_custom_segment::<ZPV>()? {
    ///     println!("Visit: {}", zpv.visit_number);
    /// }
    /// ```
    fn get_custom_segment<T: CustomSegment>(&self) -> Result<Option<T>>;

    /// Extract all occurrences of a custom segment by type
    ///
    /// # Returns
    ///
    /// A vector of parsed custom segments. Returns an empty vector if none are found.
    ///
    /// # Errors
    ///
    /// Returns an error if any segment fails to parse or validate
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zpvs = message.get_custom_segments::<ZPV>()?;
    /// for zpv in zpvs {
    ///     println!("Visit: {}", zpv.visit_number);
    /// }
    /// ```
    fn get_custom_segments<T: CustomSegment>(&self) -> Result<Vec<T>>;

    /// Check if a message contains a custom segment of the specified type
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if message.has_custom_segment::<ZPV>() {
    ///     println!("Message contains ZPV segment");
    /// }
    /// ```
    fn has_custom_segment<T: CustomSegment>(&self) -> bool;

    /// Replace or add a custom segment to the message
    ///
    /// If a segment with the same ID already exists, the first occurrence is replaced.
    /// Otherwise, the segment is added to the message.
    ///
    /// # Arguments
    ///
    /// * `segment` - The custom segment to add or replace
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zpv = ZPV::builder()
    ///     .visit_type("OUTPATIENT")
    ///     .visit_number("V12345")
    ///     .build()?;
    ///
    /// message.set_custom_segment(zpv)?;
    /// ```
    fn set_custom_segment<T: CustomSegment>(&mut self, segment: T) -> Result<()>;

    /// Add a custom segment to the message
    ///
    /// Unlike `set_custom_segment`, this always adds a new segment even if one
    /// with the same ID already exists.
    ///
    /// # Arguments
    ///
    /// * `segment` - The custom segment to add
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let zpv = ZPV::builder()
    ///     .visit_type("INPATIENT")
    ///     .visit_number("V67890")
    ///     .build()?;
    ///
    /// message.add_custom_segment(zpv);
    /// ```
    fn add_custom_segment<T: CustomSegment>(&mut self, segment: T);

    /// Remove all custom segments of the specified type from the message
    ///
    /// # Returns
    ///
    /// The number of segments removed
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let removed = message.remove_custom_segments::<ZPV>();
    /// println!("Removed {} ZPV segments", removed);
    /// ```
    fn remove_custom_segments<T: CustomSegment>(&mut self) -> usize;
}

impl MessageExt for Message {
    fn get_custom_segment<T: CustomSegment>(&self) -> Result<Option<T>> {
        let segment_id = T::segment_id();

        // Find the first segment with matching ID
        for segment in &self.segments {
            if segment.id == segment_id {
                let custom = T::from_segment(segment)?;
                custom.validate()?;
                return Ok(Some(custom));
            }
        }

        Ok(None)
    }

    fn get_custom_segments<T: CustomSegment>(&self) -> Result<Vec<T>> {
        let segment_id = T::segment_id();
        let mut results = Vec::new();

        for segment in &self.segments {
            if segment.id == segment_id {
                let custom = T::from_segment(segment)?;
                custom.validate()?;
                results.push(custom);
            }
        }

        Ok(results)
    }

    fn has_custom_segment<T: CustomSegment>(&self) -> bool {
        let segment_id = T::segment_id();
        self.segments.iter().any(|s| s.id == segment_id)
    }

    fn set_custom_segment<T: CustomSegment>(&mut self, segment: T) -> Result<()> {
        segment.validate()?;
        let segment_id = T::segment_id();
        let core_segment = segment.to_segment();

        // Find and replace the first occurrence
        for existing in &mut self.segments {
            if existing.id == segment_id {
                *existing = core_segment;
                return Ok(());
            }
        }

        // If not found, add it
        self.segments.push(core_segment);
        Ok(())
    }

    fn add_custom_segment<T: CustomSegment>(&mut self, segment: T) {
        self.segments.push(segment.to_segment());
    }

    fn remove_custom_segments<T: CustomSegment>(&mut self) -> usize {
        let segment_id = T::segment_id();
        let original_len = self.segments.len();
        self.segments.retain(|s| s.id != segment_id);
        original_len - self.segments.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::z_segment;
    use vyna_hl7_core::{Message, Segment};

    z_segment! {
        TestZEX,
        id = "ZEX",
        fields = {
            1 => test_field: String,
            2 => optional_field: Option<String>,
        }
    }

    fn create_test_message() -> Message {
        let mut msg = Message::new();

        // Add MSH
        let mut msh = Segment::new("MSH");
        msh.set_field_value(1, "|").unwrap();
        msh.set_field_value(2, "^~\\&").unwrap();
        msg.add_segment(msh);

        // Add PID
        msg.add_segment(Segment::new("PID"));

        // Add ZEX
        let mut zex = Segment::new("ZEX");
        zex.set_field_value(1, "TestValue").unwrap();
        zex.set_field_value(2, "OptionalValue").unwrap();
        msg.add_segment(zex);

        msg
    }

    #[test]
    fn test_get_custom_segment() {
        let msg = create_test_message();

        let zex = msg.get_custom_segment::<TestZEX>().unwrap();
        assert!(zex.is_some());

        let zex = zex.unwrap();
        assert_eq!(zex.test_field, "TestValue");
        assert_eq!(zex.optional_field, Some("OptionalValue".to_string()));
    }

    #[test]
    fn test_get_custom_segment_not_found() {
        let mut msg = Message::new();
        let msh = Segment::new("MSH");
        msg.add_segment(msh);

        let zex = msg.get_custom_segment::<TestZEX>().unwrap();
        assert!(zex.is_none());
    }

    #[test]
    fn test_get_custom_segments() {
        let mut msg = create_test_message();

        // Add another ZEX
        let mut zex2 = Segment::new("ZEX");
        zex2.set_field_value(1, "SecondValue").unwrap();
        msg.add_segment(zex2);

        let zexs = msg.get_custom_segments::<TestZEX>().unwrap();
        assert_eq!(zexs.len(), 2);
        assert_eq!(zexs[0].test_field, "TestValue");
        assert_eq!(zexs[1].test_field, "SecondValue");
    }

    #[test]
    fn test_has_custom_segment() {
        let msg = create_test_message();
        assert!(msg.has_custom_segment::<TestZEX>());

        let mut msg2 = Message::new();
        msg2.add_segment(Segment::new("MSH"));
        assert!(!msg2.has_custom_segment::<TestZEX>());
    }

    #[test]
    fn test_set_custom_segment_replace() {
        let mut msg = create_test_message();

        let new_zex = TestZEX {
            test_field: "ReplacedValue".to_string(),
            optional_field: None,
        };

        msg.set_custom_segment(new_zex).unwrap();

        let retrieved = msg.get_custom_segment::<TestZEX>().unwrap().unwrap();
        assert_eq!(retrieved.test_field, "ReplacedValue");
        assert_eq!(retrieved.optional_field, None);

        // Should still only have one ZEX segment
        assert_eq!(msg.get_custom_segments::<TestZEX>().unwrap().len(), 1);
    }

    #[test]
    fn test_set_custom_segment_add() {
        let mut msg = Message::new();
        msg.add_segment(Segment::new("MSH"));

        let zex = TestZEX {
            test_field: "NewValue".to_string(),
            optional_field: Some("Optional".to_string()),
        };

        msg.set_custom_segment(zex).unwrap();

        assert!(msg.has_custom_segment::<TestZEX>());
        let retrieved = msg.get_custom_segment::<TestZEX>().unwrap().unwrap();
        assert_eq!(retrieved.test_field, "NewValue");
    }

    #[test]
    fn test_add_custom_segment() {
        let mut msg = create_test_message();

        let new_zex = TestZEX {
            test_field: "AdditionalValue".to_string(),
            optional_field: None,
        };

        msg.add_custom_segment(new_zex);

        // Should now have 2 ZEX segments
        let zexs = msg.get_custom_segments::<TestZEX>().unwrap();
        assert_eq!(zexs.len(), 2);
    }

    #[test]
    fn test_remove_custom_segments() {
        let mut msg = create_test_message();

        // Add another ZEX
        let mut zex2 = Segment::new("ZEX");
        zex2.set_field_value(1, "SecondValue").unwrap();
        msg.add_segment(zex2);

        assert_eq!(msg.get_custom_segments::<TestZEX>().unwrap().len(), 2);

        let removed = msg.remove_custom_segments::<TestZEX>();
        assert_eq!(removed, 2);
        assert!(!msg.has_custom_segment::<TestZEX>());
    }
}
