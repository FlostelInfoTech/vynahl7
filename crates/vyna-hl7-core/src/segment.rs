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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Create a header segment (`MSH`, `FHS`, or `BHS`) with the two
    /// delimiter placeholders already pushed
    ///
    /// The placeholders use the default HL7 separators; for custom
    /// delimiters use [`Segment::new_header_with`]. Content pushed after
    /// this lands in field 3+, so [`Segment::encode`] cannot drop it.
    pub fn new_header<S: Into<String>>(id: S) -> Self {
        Self::new_header_with(id, &Delimiters::default())
    }

    /// Create a header segment with delimiter placeholders for `delimiters`
    ///
    /// Same as [`Segment::new_header`], but stores `delimiters`' field
    /// separator and encoding characters as fields 1-2 so accessors like
    /// [`Segment::get_field`] read the effective separators back.
    pub fn new_header_with<S: Into<String>>(id: S, delimiters: &Delimiters) -> Self {
        let mut segment = Self::new(id);
        segment.add_field(Field::from_value(delimiters.field_separator.to_string()));
        segment.add_field(Field::from_value(delimiters.encoding_characters()));
        segment
    }

    /// Whether this segment uses header encoding (`MSH`, `FHS`, `BHS`)
    ///
    /// Header segments store two delimiter placeholders as fields 1-2
    /// that [`Segment::encode`] skips in favor of the separators from
    /// its [`Delimiters`] argument.
    pub fn is_header(&self) -> bool {
        self.id == "MSH" || self.id == "FHS" || self.id == "BHS"
    }

    /// Add a field to the segment
    ///
    /// For `MSH`, `FHS`, and `BHS` the first two pushed fields must be
    /// the delimiter placeholders (field 1: the field separator, field 2:
    /// the encoding characters); [`Segment::encode`] skips them and takes
    /// the separators from the [`Delimiters`] argument instead. Pushing
    /// content as the first or second field of these segments silently
    /// drops it on encode, so always push both placeholders first.
    pub fn add_field(&mut self, field: Field) {
        self.fields.push(field);
    }

    /// Get a field by index (1-based, as per HL7 convention)
    ///
    /// Note: Field 0 is the segment ID itself.
    ///
    /// For header segments (`MSH`, `FHS`, `BHS`) fields 1-2 hold the
    /// delimiter values literally: `get_field(1)` is the field separator
    /// (typically `"|"`) and `get_field(2)` the encoding characters.
    /// Field iteration that is not header-aware must special-case them.
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
    ///
    /// The value is stored literally as one component: delimiter characters
    /// (`~`, `^`, `&`) are data, not structure, and are read literally by
    /// validation and [`Message`](crate::message::Message) accessors. For
    /// composite values use [`Field::from_delimited`] via [`Segment::set_field`].
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
    ///
    /// For `MSH`, `FHS`, and `BHS` the first two stored fields are the
    /// delimiter placeholders (see [`Segment::add_field`]) and are skipped:
    /// the separators are written from `delimiters` instead, and content
    /// encoding starts at the third stored field.
    ///
    /// A header segment with fewer than two stored fields loses content
    /// silently here; prefer [`Segment::new_header`] when building and
    /// [`Segment::try_encode`] when the segment may be hand-assembled.
    pub fn encode(&self, delimiters: &Delimiters) -> String {
        // Header segments with missing placeholders keep the legacy skip
        // behavior here; try_encode() is the checked variant.
        self.encode_inner(delimiters)
    }

    /// Encode the segment, rejecting malformed header segments
    ///
    /// Same output as [`Segment::encode`], but returns
    /// [`Error::InvalidSegment`] when a header segment (`MSH`, `FHS`,
    /// `BHS`) has fewer than the two delimiter placeholder fields, since
    /// encoding it would silently drop content fields.
    pub fn try_encode(&self, delimiters: &Delimiters) -> Result<String> {
        if self.is_header() && self.fields.len() < 2 {
            return Err(Error::InvalidSegment(format!(
                "{} segment has {} stored field(s); header segments require \
                 the two delimiter placeholders (see Segment::new_header)",
                self.id,
                self.fields.len()
            )));
        }
        Ok(self.encode_inner(delimiters))
    }

    fn encode_inner(&self, delimiters: &Delimiters) -> String {
        let mut result = self.id.clone();

        // Special handling for MSH, FHS, and BHS segments
        // These segments have the same structure: field 1 is field separator, field 2 is encoding characters
        if self.is_header() {
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
                truncate_id(&self.id)
            )));
        }

        if !self.id.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(Error::InvalidSegment(format!(
                "Segment ID must be alphanumeric, got: {}",
                truncate_id(&self.id)
            )));
        }

        Ok(())
    }
}

/// Cap an echoed id at 32 chars plus length; short ids stay verbatim.
fn truncate_id(value: &str) -> String {
    const MAX: usize = 32;
    let count = value.chars().count();
    if count <= MAX {
        value.to_string()
    } else {
        let head: String = value.chars().take(MAX).collect();
        format!("{head}... ({} chars total)", count)
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
        let mut segment = Segment::new_header("MSH");
        segment.add_field(Field::from_value("SendingApp"));
        segment.add_field(Field::from_value("SendingFac"));
        segment.add_field(Field::from_value("ReceivingApp"));

        assert_eq!(
            segment.encode(&delims),
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp"
        );
    }

    #[test]
    fn test_new_header_pushes_placeholders() {
        let segment = Segment::new_header("MSH");
        assert!(segment.is_header());
        assert_eq!(segment.get_field_value(1), Some("|"));
        assert_eq!(segment.get_field_value(2), Some("^~\\&"));

        let custom = Delimiters::from_encoding_characters('!', ":;\\?").unwrap();
        let segment = Segment::new_header_with("FHS", &custom);
        assert!(segment.is_header());
        assert_eq!(segment.get_field_value(1), Some("!"));
        assert_eq!(segment.get_field_value(2), Some(":;\\?"));
        assert!(!Segment::new("PID").is_header());
    }

    #[test]
    fn test_try_encode_rejects_header_without_placeholders() {
        let delims = Delimiters::default();
        let mut segment = Segment::new("MSH");
        segment.add_field(Field::from_value("SendingApp"));

        let err = segment.try_encode(&delims).unwrap_err().to_string();
        assert!(err.contains("MSH"), "{err}");
        assert!(err.contains("new_header"), "{err}");

        // Regular segments are unaffected.
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        assert_eq!(pid.try_encode(&delims).unwrap(), "PID|1");
    }

    #[test]
    fn test_msh_encode_uses_two_placeholder_fields() {
        // MSH/FHS/BHS carry two placeholders (separator, encodings)
        // that encode() skips; content starts at the third field.
        let delims = Delimiters::default();
        let mut segment = Segment::new("MSH");
        segment.add_field(Field::from_value("|"));
        segment.add_field(Field::from_value("^~\\&"));
        segment.add_field(Field::from_value("SendingApp"));
        assert_eq!(segment.encode(&delims), "MSH|^~\\&|SendingApp");
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

    #[test]
    fn test_validate_id_truncates_long_echo() {
        let long = "B".repeat(200);
        let err = Segment::new(&long).validate_id().unwrap_err().to_string();
        assert!(!err.contains(&long), "must not echo full id: {err}");
        assert!(err.contains("200 chars total"), "{err}");
        assert!(err.contains(&"B".repeat(32)), "{err}");
        // Short ids stay verbatim.
        let err = Segment::new("PI").validate_id().unwrap_err().to_string();
        assert!(err.contains("PI"), "{err}");
    }
}
