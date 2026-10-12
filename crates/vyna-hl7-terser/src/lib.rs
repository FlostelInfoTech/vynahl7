//! Terser API for HL7 messages
//!
//! The Terser provides a convenient API for accessing HL7 message fields
//! using path notation, similar to HAPI's Terser in Java.
//!
//! Path format examples:
//! - `PID-5-1` - PID segment, field 5, component 1
//! - `PID-5-1-2` - PID segment, field 5, component 1, subcomponent 2
//! - `OBX(2)-5` - Second OBX segment, field 5
//! - `PID-11(2)-1` - PID segment, field 11, second repetition, component 1

mod path;
pub mod cache;
pub mod bulk;
pub mod groups;
pub mod iterator;
pub mod query;
pub mod query_result;

use vyna_hl7_core::{
    error::{Error, Result},
    field::Field,
    message::Message,
    segment::Segment,
};

pub use bulk::BulkTerser;
pub use cache::CachedTerser;
pub use groups::{GroupConfig, GroupIterator, GroupNavigator, GroupPattern, SegmentGroup};
pub use query::TerserQuery;
pub use query_result::{QueryAcknowledgment, QueryResponseStatus, QueryResultParser};
use path::TerserPath;

/// Terser for accessing HL7 message fields using path notation
pub struct Terser<'a> {
    pub(crate) message: &'a Message,
}

impl<'a> Terser<'a> {
    /// Create a new Terser for the given message
    pub fn new(message: &'a Message) -> Self {
        Self { message }
    }

    /// Get a value using path notation
    ///
    /// Examples:
    /// - `get("PID-5-1")` - Get PID field 5, component 1
    /// - `get("OBX(2)-5")` - Get second OBX segment, field 5
    /// - `get("PID-11(2)-1")` - Get PID field 11, repetition 2, component 1
    pub fn get(&self, path: &str) -> Result<Option<&str>> {
        let parsed_path = TerserPath::parse(path)?;

        // Find the segment
        let segments = self.message.get_segments_by_id(&parsed_path.segment_id);

        if segments.is_empty() {
            return Ok(None);
        }

        let segment = segments.get(parsed_path.segment_index)
            .ok_or_else(|| Error::terser_path(format!(
                "Segment index {} out of bounds for {}",
                parsed_path.segment_index, parsed_path.segment_id
            )))?;

        // Get the field
        let field = match segment.get_field(parsed_path.field_index) {
            Some(f) => f,
            None => return Ok(None),
        };

        // Navigate to the value
        let value = self.get_field_value(
            field,
            parsed_path.repetition_index,
            parsed_path.component_index,
            parsed_path.subcomponent_index,
        );

        Ok(value)
    }

    /// Get a field value at the specified indices
    ///
    /// Note: Component and subcomponent indices are 1-based (HL7 standard notation)
    /// but internally converted to 0-based for array access.
    fn get_field_value<'b>(
        &self,
        field: &'b Field,
        rep_idx: usize,
        comp_idx: Option<usize>,
        sub_idx: Option<usize>,
    ) -> Option<&'b str> {
        let repetition = field.get_repetition(rep_idx)?;

        match (comp_idx, sub_idx) {
            (None, None) => {
                // Just the field value
                repetition.value()
            }
            (Some(c_idx), None) => {
                // Component value (convert 1-based HL7 to 0-based internal)
                if c_idx == 0 {
                    return None; // Invalid: HL7 uses 1-based indexing
                }
                repetition.get_component(c_idx - 1)?.value()
            }
            (Some(c_idx), Some(s_idx)) => {
                // Subcomponent value (convert 1-based HL7 to 0-based internal)
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
}

/// Mutable terser for modifying message values
pub struct TerserMut<'a> {
    message: &'a mut Message,
}

impl<'a> TerserMut<'a> {
    /// Create a new mutable Terser
    pub fn new(message: &'a mut Message) -> Self {
        Self { message }
    }

    /// Set a value using path notation
    pub fn set(&mut self, path: &str, value: &str) -> Result<()> {
        let parsed_path = TerserPath::parse(path)?;

        // Find or create the segment
        let segment_index = self.ensure_segment(&parsed_path.segment_id, parsed_path.segment_index)?;

        // Ensure field exists and set the value
        {
            let segment = &mut self.message.segments[segment_index];

            // Ensure field exists
            while segment.fields.len() < parsed_path.field_index {
                segment.add_field(Field::new());
            }

            let field = segment.get_field_mut(parsed_path.field_index)
                .ok_or_else(|| Error::terser_path("Failed to get field"))?;

            // Set the value
            Self::set_field_value_static(
                field,
                value,
                parsed_path.repetition_index,
                parsed_path.component_index,
                parsed_path.subcomponent_index,
            )?;
        }

        Ok(())
    }

    /// Ensure segment exists at the given index
    fn ensure_segment(&mut self, segment_id: &str, index: usize) -> Result<usize> {
        let mut current_index = 0;
        let mut actual_index = None;

        for (i, seg) in self.message.segments.iter().enumerate() {
            if seg.id == segment_id {
                if current_index == index {
                    actual_index = Some(i);
                    break;
                }
                current_index += 1;
            }
        }

        if let Some(idx) = actual_index {
            Ok(idx)
        } else {
            // Create new segment
            let new_segment = Segment::new(segment_id);
            self.message.add_segment(new_segment);
            Ok(self.message.segments.len() - 1)
        }
    }

    /// Set a field value at the specified indices
    ///
    /// Note: Component and subcomponent indices are 1-based (HL7 standard notation)
    /// but internally converted to 0-based for array access.
    fn set_field_value_static(
        field: &mut Field,
        value: &str,
        rep_idx: usize,
        comp_idx: Option<usize>,
        sub_idx: Option<usize>,
    ) -> Result<()> {
        use vyna_hl7_core::field::{Component, Repetition, SubComponent};

        // Ensure repetition exists
        while field.repetitions.len() <= rep_idx {
            field.add_repetition(Repetition::new());
        }

        let repetition = field.get_repetition_mut(rep_idx)
            .ok_or_else(|| Error::terser_path("Failed to get repetition"))?;

        match (comp_idx, sub_idx) {
            (None, None) => {
                // Set entire field value
                if repetition.components.is_empty() {
                    repetition.add_component(Component::from_value(value));
                } else {
                    repetition.components[0] = Component::from_value(value);
                }
            }
            (Some(c_idx), None) => {
                // Set component value (convert 1-based HL7 to 0-based internal)
                if c_idx == 0 {
                    return Err(Error::terser_path(
                        "Invalid component index 0: HL7 uses 1-based indexing",
                    ));
                }
                let internal_idx = c_idx - 1;
                while repetition.components.len() <= internal_idx {
                    repetition.add_component(Component::new());
                }

                repetition.components[internal_idx] = Component::from_value(value);
            }
            (Some(c_idx), Some(s_idx)) => {
                // Set subcomponent value (convert 1-based HL7 to 0-based internal)
                if c_idx == 0 || s_idx == 0 {
                    return Err(Error::terser_path(
                        "Invalid index 0: HL7 uses 1-based indexing",
                    ));
                }
                let internal_c_idx = c_idx - 1;
                let internal_s_idx = s_idx - 1;

                while repetition.components.len() <= internal_c_idx {
                    repetition.add_component(Component::new());
                }

                let component = &mut repetition.components[internal_c_idx];

                while component.subcomponents.len() <= internal_s_idx {
                    component.add_subcomponent(SubComponent::new(""));
                }

                component.subcomponents[internal_s_idx] = SubComponent::new(value);
            }
            (None, Some(_)) => {
                return Err(Error::terser_path(
                    "Cannot set subcomponent without component index",
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_parse_path_with_segment_index() {
        // OBX(2) = second OBX (1-based API) = internal index 1 (0-based)
        let path = TerserPath::parse("OBX(2)-5").unwrap();
        assert_eq!(path.segment_id, "OBX");
        assert_eq!(path.segment_index, 1); // Internal 0-based index
        assert_eq!(path.field_index, 5);
    }

    #[test]
    fn test_parse_path_with_repetition() {
        // PID-11(1) = first repetition (1-based API) = internal index 0
        let path = TerserPath::parse("PID-11(1)-1").unwrap();
        assert_eq!(path.field_index, 11);
        assert_eq!(path.repetition_index, 0); // Internal 0-based index
        assert_eq!(path.component_index, Some(1));
    }

    #[test]
    fn test_parse_invalid_path() {
        assert!(TerserPath::parse("").is_err());
        assert!(TerserPath::parse("PID").is_err());
        assert!(TerserPath::parse("PID-").is_err());
    }

    #[test]
    fn test_1_based_component_indexing() {
        // Standard HL7 message with components: ADT^A01^ADT_A01
        let hl7 = "MSH|^~\\&|HIS|HOSPITAL|EMR|CLINIC|20250115143025||ADT^A01^ADT_A01|MSG12345|P|2.5\r";
        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // MSH-9-1 should be first component (ADT)
        let msg_type = terser.get("MSH-9-1").unwrap();
        assert_eq!(msg_type, Some("ADT"));

        // MSH-9-2 should be second component (A01)
        let trigger = terser.get("MSH-9-2").unwrap();
        assert_eq!(trigger, Some("A01"));

        // MSH-9-3 should be third component (ADT_A01)
        let structure = terser.get("MSH-9-3").unwrap();
        assert_eq!(structure, Some("ADT_A01"));
    }

    #[test]
    fn test_1_based_patient_name_components() {
        // PID with patient name: DOE^JOHN^A^^^DR
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001||DOE^JOHN^A^^^DR\r";
        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // PID-5-1 = Family name (DOE)
        assert_eq!(terser.get("PID-5-1").unwrap(), Some("DOE"));

        // PID-5-2 = Given name (JOHN)
        assert_eq!(terser.get("PID-5-2").unwrap(), Some("JOHN"));

        // PID-5-3 = Middle name (A)
        assert_eq!(terser.get("PID-5-3").unwrap(), Some("A"));

        // PID-5-6 = Prefix (DR)
        assert_eq!(terser.get("PID-5-6").unwrap(), Some("DR"));
    }

    #[test]
    fn test_invalid_0_based_index_returns_none() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r";
        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // MSH-9-0 should be invalid (HL7 doesn't use 0-based)
        let result = terser.get("MSH-9-0").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_set_with_1_based_indexing() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        // Set patient name using 1-based component indices
        terser.set("PID-5-1", "SMITH").unwrap(); // Family name
        terser.set("PID-5-2", "JANE").unwrap(); // Given name
        terser.set("PID-5-3", "M").unwrap(); // Middle name

        // Verify with read terser
        let read_terser = Terser::new(&message);
        assert_eq!(read_terser.get("PID-5-1").unwrap(), Some("SMITH"));
        assert_eq!(read_terser.get("PID-5-2").unwrap(), Some("JANE"));
        assert_eq!(read_terser.get("PID-5-3").unwrap(), Some("M"));
    }

    #[test]
    fn test_set_invalid_0_index_returns_error() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        // Setting with index 0 should fail
        let result = terser.set("PID-5-0", "INVALID");
        assert!(result.is_err());
    }

    #[test]
    fn test_1_based_repetition_indexing() {
        let hl7 =
            "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001||DOE^JOHN~SMITH^JANE\r";
        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // No repetition specified = first repetition
        assert_eq!(terser.get("PID-5-1").unwrap(), Some("DOE"));

        // Explicit 1-based repetition indices
        assert_eq!(terser.get("PID-5(1)-1").unwrap(), Some("DOE"));
        assert_eq!(terser.get("PID-5(1)-2").unwrap(), Some("JOHN"));
        assert_eq!(terser.get("PID-5(2)-1").unwrap(), Some("SMITH"));
        assert_eq!(terser.get("PID-5(2)-2").unwrap(), Some("JANE"));

        // Repetition 0 is invalid, like segment index 0
        assert!(terser.get("PID-5(0)-1").is_err());
    }

    #[test]
    fn test_set_with_1_based_repetition_indexing() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        terser.set("PID-5(1)-1", "DOE").unwrap();
        terser.set("PID-5(2)-1", "SMITH").unwrap();
        assert!(terser.set("PID-5(0)-1", "INVALID").is_err());

        let read_terser = Terser::new(&message);
        assert_eq!(read_terser.get("PID-5(1)-1").unwrap(), Some("DOE"));
        assert_eq!(read_terser.get("PID-5(2)-1").unwrap(), Some("SMITH"));
    }
}
