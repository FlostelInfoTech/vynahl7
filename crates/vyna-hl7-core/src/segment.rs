//! HL7 segment structures

use crate::delimiters::Delimiters;
use crate::error::{Error, Result};
use crate::field::{Component, Field, Repetition};

/// An HL7 segment
///
/// A segment consists of:
/// - A 3-character segment ID (e.g., "MSH", "PID", "OBX")
/// - Multiple fields separated by the field separator
///
/// Note: For MSH segments, the encoding is special:
/// - MSH|^~\&|... (field separator and encoding characters come first)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Segment ID (3 characters, e.g., "MSH", "PID")
    pub id: String,
    /// Fields in the segment (excluding the segment ID)
    pub fields: Vec<Field>,
}

impl Segment {
    /// Create a new segment with the given ID
    pub fn new<S: Into<String>>(id: S) -> Self {
        let id = id.into();
        Self {
            id,
            fields: Vec::new(),
        }
    }

    /// Add a field to the segment
    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }

    /// Get a field by index (1-based, as per HL7 convention)
    ///
    /// Note: Field 0 is the segment ID itself
    pub fn get_field(&self, index: usize) -> Option<&Field> {
        if index == 0 {
            None // Field 0 is the segment ID, not a regular field
        } else {
            self.fields.get(index - 1)
        }
    }

    /// Get a mutable field by index (1-based)
    pub fn get_field_mut(&mut self, index: usize) -> Option<&mut Field> {
        if index == 0 {
            None
        } else {
            self.fields.get_mut(index - 1)
        }
    }

    /// Set a field value at the given index (1-based)
    ///
    /// This will extend the fields vector if necessary
    pub fn set_field(&mut self, index: usize, field: Field) -> Result<()> {
        if index == 0 {
            return Err(Error::InvalidFieldAccess(
                "Cannot set field 0 (segment ID)".to_string(),
            ));
        }

        let field_index = index - 1;

        // Extend fields vector if necessary
        while self.fields.len() <= field_index {
            self.fields.push(Field::new());
        }

        self.fields[field_index] = field;
        Ok(())
    }

    /// Get a field value as a string (convenience method)
    pub fn get_field_value(&self, index: usize) -> Option<&str> {
        self.get_field(index).and_then(|f| f.value())
    }

    /// Set a field from a simple string value
    pub fn set_field_value<S: Into<String>>(&mut self, index: usize, value: S) -> Result<()> {
        self.set_field(index, Field::from_value(value))
    }

    /// Set a component value within a field
    ///
    /// # Arguments
    /// * `field_index` - 1-based field index
    /// * `rep_index` - 0-based repetition index
    /// * `comp_index` - 0-based component index
    /// * `value` - The value to set
    pub fn set_component<S: Into<String>>(
        &mut self,
        field_index: usize,
        rep_index: usize,
        comp_index: usize,
        value: S,
    ) -> Result<()> {
        if field_index == 0 {
            return Err(Error::InvalidFieldAccess(
                "Cannot set field 0 (segment ID)".to_string(),
            ));
        }

        let field_idx = field_index - 1;

        // Extend fields if necessary
        while self.fields.len() <= field_idx {
            self.fields.push(Field::new());
        }

        let field = &mut self.fields[field_idx];

        // Extend repetitions if necessary
        while field.repetitions.len() <= rep_index {
            field.repetitions.push(Repetition::new());
        }

        let repetition = &mut field.repetitions[rep_index];

        // Extend components if necessary
        while repetition.components.len() <= comp_index {
            repetition.components.push(Component::new());
        }

        repetition.components[comp_index] = Component::from_value(value);
        Ok(())
    }

    /// Get the number of fields (excluding segment ID)
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Encode the segment to HL7 format
    pub fn encode(&self, delimiters: &Delimiters) -> String {
        let mut result = self.id.clone();

        // Special handling for MSH, FHS, and BHS segments
        // These segments have the same structure: field 1 is field separator, field 2 is encoding characters
        if self.id == "MSH" || self.id == "FHS" || self.id == "BHS" {
            result.push(delimiters.field_separator);
            result.push_str(&delimiters.encoding_characters());

            // Fields start from field 3 (after separator and encoding chars)
            // Skip first 2 fields which are the delimiter definitions
            for field in self.fields.iter().skip(2) {
                result.push(delimiters.field_separator);
                result.push_str(&field.encode(delimiters));
            }
        } else {
            // Regular segments
            for field in &self.fields {
                result.push(delimiters.field_separator);
                result.push_str(&field.encode(delimiters));
            }
        }

        result
    }

    /// Validate segment ID (must be 3 alphanumeric characters)
    pub fn validate_id(&self) -> Result<()> {
        if self.id.len() != 3 {
            return Err(Error::InvalidSegment(format!(
                "Segment ID must be 3 characters, got: {}",
                self.id
            )));
        }

        if !self.id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(Error::InvalidSegment(format!(
                "Segment ID must be alphanumeric, got: {}",
                self.id
            )));
        }

        Ok(())
    }
}

/// Common segment types
pub mod types {
    /// Message Header segment
    pub const MSH: &str = "MSH";

    /// Patient Identification segment
    pub const PID: &str = "PID";

    /// Patient Visit segment
    pub const PV1: &str = "PV1";

    /// Observation Request segment
    pub const OBR: &str = "OBR";

    /// Observation/Result segment
    pub const OBX: &str = "OBX";

    /// Error segment
    pub const ERR: &str = "ERR";

    /// Message Acknowledgment segment
    pub const MSA: &str = "MSA";

    /// Next of Kin segment
    pub const NK1: &str = "NK1";

    /// Insurance segment
    pub const IN1: &str = "IN1";

    /// Additional demographics segment
    pub const PD1: &str = "PD1";

    /// Common Order segment
    pub const ORC: &str = "ORC";

    /// Diagnosis segment
    pub const DG1: &str = "DG1";

    /// Allergy Information segment
    pub const AL1: &str = "AL1";

    /// Notes and Comments segment
    pub const NTE: &str = "NTE";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_segment() {
        let segment = Segment::new("PID");
        assert_eq!(segment.id, "PID");
        assert_eq!(segment.fields.len(), 0);
    }

    #[test]
    fn test_add_field() {
        let mut segment = Segment::new("PID");
        segment.add_field(Field::from_value("12345"));
        assert_eq!(segment.fields.len(), 1);
    }

    #[test]
    fn test_get_field() {
        let mut segment = Segment::new("PID");
        segment.add_field(Field::from_value("12345"));

        assert_eq!(segment.get_field(1).unwrap().value(), Some("12345"));
        assert!(segment.get_field(0).is_none());
        assert!(segment.get_field(2).is_none());
    }

    #[test]
    fn test_set_field() {
        let mut segment = Segment::new("PID");
        segment.set_field_value(1, "12345").unwrap();
        segment.set_field_value(3, "Smith").unwrap();

        assert_eq!(segment.get_field_value(1), Some("12345"));
        assert_eq!(segment.get_field_value(3), Some("Smith"));
        // Field 2 should be empty but present
        assert_eq!(segment.fields.len(), 3);
    }

    #[test]
    fn test_encode_regular_segment() {
        let delims = Delimiters::default();
        let mut segment = Segment::new("PID");
        segment.add_field(Field::from_value("1"));
        segment.add_field(Field::from_value("12345"));
        segment.add_field(Field::from_value("Smith^John"));

        let encoded = segment.encode(&delims);
        assert!(encoded.starts_with("PID|"));
    }

    #[test]
    fn test_encode_msh_segment() {
        let delims = Delimiters::default();
        let mut segment = Segment::new("MSH");
        segment.add_field(Field::from_value("^~\\&")); // encoding characters
        segment.add_field(Field::from_value("SendingApp"));
        segment.add_field(Field::from_value("ReceivingApp"));

        let encoded = segment.encode(&delims);
        assert!(encoded.starts_with("MSH|^~\\&|"));
    }

    #[test]
    fn test_validate_id() {
        let valid = Segment::new("PID");
        assert!(valid.validate_id().is_ok());

        let invalid_length = Segment::new("PI");
        assert!(invalid_length.validate_id().is_err());

        let invalid_chars = Segment::new("PI!");
        assert!(invalid_chars.validate_id().is_err());
    }
}
