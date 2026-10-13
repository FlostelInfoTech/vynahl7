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
//!
//! Every index in path notation is 1-based (HL7 standard): `OBX(1)` is the
//! first OBX segment and `PID-5(1)` is the first field repetition. The one
//! exception is [`Terser::iter_repetitions`], whose `segment_index`
//! argument is a 0-based position, not a parsed path.

pub mod bulk;
pub mod cache;
pub mod groups;
pub mod iterator;
mod path;
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
use path::TerserPath;
pub use query::TerserQuery;
pub use query_result::{QueryAcknowledgment, QueryResponseStatus, QueryResultParser};

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
    ///
    /// # Missing values vs errors
    ///
    /// Returns `Ok(None)` when the segment id is absent from the message or
    /// the addressed field, repetition, component, or subcomponent is
    /// absent. Returns `Err` when the path is malformed or the segment
    /// index exceeds the number of occurrences of that segment id (e.g.
    /// `OBX(5)` when the message holds two OBX segments).
    /// [`CachedTerser::get`](crate::CachedTerser::get) shares this contract.
    ///
    /// # Whole-field reads yield the first component
    ///
    /// A path without a component (e.g. `PID-5`) returns the first
    /// component of the addressed repetition (`DOE` for `DOE^JOHN^A`,
    /// `ADT` for `ADT^A01`), not the composite text. Use explicit
    /// component paths (`PID-5-2`, `MSH-9-2`) for the remaining parts.
    ///
    /// Whole-field `set` is asymmetric by provenance:
    /// [`TerserMut::set`](crate::TerserMut::set) stores the value
    /// literally as a single component, so delimiter characters in a
    /// `set` value read back whole here (`PID-5` returns `DOE^JOHN^A`)
    /// while the same characters in a parsed message are structure
    /// (`PID-5` returns `DOE`, `PID-5-2` returns `JOHN`).
    ///
    /// Component-level `set` is likewise literal: `set("PID-5-1",
    /// "a&b")` stores one subcomponent, so `PID-5-1` reads `a&b` whole
    /// while `PID-5-1-2` stays absent (parsed `a&b` instead yields
    /// `b` there).
    pub fn get(&self, path: &str) -> Result<Option<&str>> {
        let parsed_path = TerserPath::parse(path)?;

        // Find the segment
        let segments = self.message.get_segments_by_id(&parsed_path.segment_id);

        if segments.is_empty() {
            return Ok(None);
        }

        let segment = segments.get(parsed_path.segment_index).ok_or_else(|| {
            Error::terser_path(format!(
                "Segment index {} out of bounds for {}",
                parsed_path.segment_index, parsed_path.segment_id
            ))
        })?;

        // Get the field
        let field = match segment.get_field(parsed_path.field_index) {
            Some(f) => f,
            None => return Ok(None),
        };

        // Navigate to the value
        let value = path::resolve_field_value(
            field,
            parsed_path.repetition_index,
            parsed_path.component_index,
            parsed_path.subcomponent_index,
        );

        Ok(value)
    }
}

/// Maximum 1-based index [`TerserMut::set`] accepts in any single
/// dimension (segment occurrence, field, repetition, component,
/// subcomponent).
///
/// `set` gap-fills missing structure, so an unbounded index would allocate
/// millions of empty values; anything beyond this cap is rejected with an
/// error instead. Reads via [`Terser::get`] are unaffected.
const MAX_SET_INDEX: usize = 10_000;

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
    ///
    /// Missing structure is created on demand so the value always lands in
    /// the addressed occurrence: absent segment occurrences are gap-filled
    /// with empty segments (inserted after the last existing occurrence, or
    /// appended when the segment id is new), and missing fields,
    /// repetitions, components, and subcomponents are gap-filled with empty
    /// values. A `set` followed by a `get` of the same path always reads
    /// the value back; existing occurrences are never overwritten by an
    /// out-of-range write.
    ///
    /// A whole-field path (e.g. `PID-5`, with no component) replaces
    /// the addressed repetition's entire content with `value` as a
    /// single literal component: trailing components from a previous
    /// composite value are cleared, and delimiter characters in
    /// `value` are stored literally, not parsed as structure. Address
    /// components explicitly (`PID-5-1`) to change one component
    /// while keeping the rest.
    ///
    /// A component path (e.g. `PID-5-1`) likewise stores `value`
    /// literally as a single subcomponent: `&` in the value is text,
    /// not structure, so `set("PID-5-1", "a&b")` reads back whole via
    /// `PID-5-1` while `PID-5-1-2` stays absent. Address subcomponents
    /// explicitly (`PID-5-1-2`) to write them as structure.
    ///
    /// Returns [`Error::terser_path`](Error::terser_path) when the path is
    /// malformed, uses a 0 index (all path indices are 1-based),
    /// addresses an index beyond `MAX_SET_INDEX` (10,000), creates a
    /// segment id that is not 3 ASCII alphanumeric characters, or
    /// writes field 1 or 2 of `MSH`, `FHS`, or `BHS`: those hold the
    /// message delimiters, which live on the message, not the field,
    /// and encode would silently drop anything written there.
    pub fn set(&mut self, path: &str, value: &str) -> Result<()> {
        let parsed_path = TerserPath::parse(path)?;

        // MSH/FHS/BHS fields 1 and 2 are the message delimiters: encode
        // derives them from Delimiters and skips the stored fields, so a
        // write here would vanish. Reject it instead of dropping it.
        if matches!(parsed_path.segment_id.as_str(), "MSH" | "FHS" | "BHS")
            && matches!(parsed_path.field_index, 1 | 2)
        {
            return Err(Error::terser_path(format!(
                "Cannot set {}-{}: fields 1 and 2 of {} hold the message \
                 delimiters, which are message-level, not field values",
                parsed_path.segment_id, parsed_path.field_index, parsed_path.segment_id
            )));
        }

        // Bound gap-filling before mutating anything: every dimension is
        // compared as a 1-based occurrence count, so no partial structure
        // is created when the write is rejected.
        if parsed_path.segment_index + 1 > MAX_SET_INDEX
            || parsed_path.field_index > MAX_SET_INDEX
            || parsed_path.repetition_index + 1 > MAX_SET_INDEX
            || parsed_path.component_index.unwrap_or(0) > MAX_SET_INDEX
            || parsed_path.subcomponent_index.unwrap_or(0) > MAX_SET_INDEX
        {
            return Err(Error::terser_path(format!(
                "Set index exceeds gap-fill cap of {MAX_SET_INDEX}: {path}"
            )));
        }

        // Reject 0 indices before mutating anything: every path index is
        // 1-based, and the checks inside the setters run only after
        // gap-filling has already appended segments or fields.
        if parsed_path.field_index == 0
            || parsed_path.component_index == Some(0)
            || parsed_path.subcomponent_index == Some(0)
        {
            return Err(Error::terser_path(
                "Invalid index 0: HL7 uses 1-based indexing",
            ));
        }
        if parsed_path.component_index.is_none() && parsed_path.subcomponent_index.is_some() {
            return Err(Error::terser_path(
                "Cannot set subcomponent without component index",
            ));
        }

        // Find or create the segment
        let segment_index =
            self.ensure_segment(&parsed_path.segment_id, parsed_path.segment_index)?;

        // Ensure field exists and set the value
        {
            let segment = &mut self.message.segments[segment_index];

            // Ensure field exists
            while segment.fields.len() < parsed_path.field_index {
                segment.add_field(Field::new());
            }

            let field = segment
                .get_field_mut(parsed_path.field_index)
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

    /// Ensure the `index`-th (0-based internal) occurrence exists
    ///
    /// Returns the message position of exactly that occurrence. Missing
    /// occurrences are gap-filled with empty segments inserted after the
    /// last existing occurrence (appended when the id is new), so an
    /// out-of-range write never lands in a different occurrence.
    fn ensure_segment(&mut self, segment_id: &str, index: usize) -> Result<usize> {
        let mut positions = Vec::new();
        for (i, seg) in self.message.segments.iter().enumerate() {
            if seg.id == segment_id {
                positions.push(i);
            }
        }

        if let Some(&pos) = positions.get(index) {
            return Ok(pos);
        }

        // A new occurrence must carry a valid segment id (3 ASCII
        // alnum, like Segment::validate_id): without this, set("AB-5")
        // appends a segment that fails validation. Checked before any
        // insertion, so a rejection leaves the message untouched. Reads
        // stay lenient (Terser::get returns Ok(None) for absent ids).
        if segment_id.len() != 3 || !segment_id.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(Error::terser_path(format!(
                "Invalid segment id '{segment_id}': must be 3 ASCII \
                 alphanumeric characters",
            )));
        }

        // Gap-fill: create every missing occurrence up to `index`.
        let missing = index + 1 - positions.len();
        let mut insert_at = positions
            .last()
            .map(|pos| pos + 1)
            .unwrap_or(self.message.segments.len());
        for _ in 0..missing {
            self.message
                .segments
                .insert(insert_at, Segment::new(segment_id));
            insert_at += 1;
        }
        Ok(insert_at - 1)
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

        let repetition = field
            .get_repetition_mut(rep_idx)
            .ok_or_else(|| Error::terser_path("Failed to get repetition"))?;

        match (comp_idx, sub_idx) {
            (None, None) => {
                // Set entire field value, replacing all components. A
                // whole-field value is literal text stored as the single
                // component, so trailing components from a previous
                // composite value are cleared rather than left stale.
                repetition.components = vec![Component::from_value(value)];
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
        let hl7 =
            "MSH|^~\\&|HIS|HOSPITAL|EMR|CLINIC|20250115143025||ADT^A01^ADT_A01|MSG12345|P|2.5\r";
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
        let hl7 =
            "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1||PAT001||DOE^JOHN^A^^^DR\r";
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
    fn test_set_gap_fills_missing_segment_occurrences() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|GLU||98\rOBX|2|NM|NA||140\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        // OBX(5) does not exist yet: the write must land in the 5th OBX,
        // gap-filling OBX(3) and OBX(4) — never in an existing occurrence.
        terser.set("OBX(5)-5", "4.2").unwrap();

        let read_terser = Terser::new(&message);
        assert_eq!(read_terser.get("OBX(5)-5").unwrap(), Some("4.2"));
        assert_eq!(read_terser.get("OBX(1)-5").unwrap(), Some("98"));
        assert_eq!(read_terser.get("OBX(2)-5").unwrap(), Some("140"));
        assert_eq!(message.get_segments_by_id("OBX").len(), 5);
    }

    #[test]
    fn test_set_appends_next_segment_occurrence() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|GLU||98\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        terser.set("OBX(2)-5", "140").unwrap();

        let read_terser = Terser::new(&message);
        assert_eq!(read_terser.get("OBX(1)-5").unwrap(), Some("98"));
        assert_eq!(read_terser.get("OBX(2)-5").unwrap(), Some("140"));
        assert_eq!(message.get_segments_by_id("OBX").len(), 2);
    }

    #[test]
    fn test_set_existing_segment_occurrence_only() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|GLU||98\rOBX|2|NM|NA||140\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        terser.set("OBX(1)-5", "7.5").unwrap();

        let read_terser = Terser::new(&message);
        assert_eq!(read_terser.get("OBX(1)-5").unwrap(), Some("7.5"));
        assert_eq!(read_terser.get("OBX(2)-5").unwrap(), Some("140"));
        assert_eq!(message.get_segments_by_id("OBX").len(), 2);
    }

    #[test]
    fn test_get_missing_vs_out_of_range_contract() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|GLU||98\r";
        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // Absent segment id or field: Ok(None).
        assert_eq!(terser.get("ZZZ-1").unwrap(), None);
        assert_eq!(terser.get("OBX-99").unwrap(), None);
        // Out-of-range segment index: Err.
        assert!(terser.get("OBX(2)-5").is_err());
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

    #[test]
    fn test_set_whole_field_clears_trailing_components() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1||PAT001||DOE^JOHN\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        terser.set("PID-5", "SMITH").unwrap();

        let read_terser = Terser::new(&message);
        assert_eq!(read_terser.get("PID-5").unwrap(), Some("SMITH"));
        assert_eq!(read_terser.get("PID-5-2").unwrap(), None);
        let pid = &message.get_segments_by_id("PID")[0];
        let rep = pid.get_field(5).unwrap().get_repetition(0).unwrap();
        assert_eq!(rep.components.len(), 1);
        assert!(!message.encode().contains("SMITH^JOHN"));
    }

    #[test]
    fn test_set_rejects_huge_segment_gap() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|GLU||98\r";
        let mut message = parse_message(hl7).unwrap();
        {
            let mut terser = TerserMut::new(&mut message);
            assert!(terser.set("OBX(1000000)-5", "4.2").is_err());
        }

        // Rejected before mutating: no gap-filled segments.
        assert_eq!(message.get_segments_by_id("OBX").len(), 1);
    }

    #[test]
    fn test_set_rejects_huge_field_repetition_component_gaps() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1\r";
        let mut message = parse_message(hl7).unwrap();
        {
            let mut terser = TerserMut::new(&mut message);
            assert!(terser.set("PID-999999999-1", "X").is_err());
            assert!(terser.set("PID-5(100001)-1", "X").is_err());
            assert!(terser.set("PID-5-100001", "X").is_err());
            assert!(terser.set("PID-5-1-100001", "X").is_err());
        }

        // Rejected before mutating: PID still holds its single field.
        assert_eq!(message.get_segments_by_id("PID")[0].fields.len(), 1);
    }

    #[test]
    fn test_get_whole_field_yields_first_component_parsed() {
        // Whole-field reads return the first component, not composite
        // text: PID-5 "DOE^JOHN^A" reads as "DOE", MSH-9 "ADT^A01" as
        // "ADT". Use explicit component paths for the rest.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1||PAT001||DOE^JOHN^A\r";
        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);
        assert_eq!(terser.get("PID-5").unwrap(), Some("DOE"));
        assert_eq!(terser.get("MSH-9").unwrap(), Some("ADT"));
        assert_eq!(terser.get("PID-5-2").unwrap(), Some("JOHN"));
        assert_eq!(terser.get("MSH-9-2").unwrap(), Some("A01"));
    }

    #[test]
    fn test_cached_get_whole_field_yields_first_component() {
        // CachedTerser shares Terser::get's first-component contract.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r\
                   PID|1||PAT001||DOE^JOHN^A\r";
        let message = parse_message(hl7).unwrap();
        let mut cached = CachedTerser::new(&message);
        assert_eq!(cached.get("PID-5").unwrap(), Some("DOE"));
        assert_eq!(cached.get("MSH-9").unwrap(), Some("ADT"));
    }

    #[test]
    fn test_get_whole_field_set_provenance_is_literal() {
        // set() stores whole-field values literally as one component:
        // delimiters in the value are text, not structure, so the
        // same characters read back whole here but split in a parsed
        // message. This pins the set/parse provenance asymmetry.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1\r";
        let mut message = parse_message(hl7).unwrap();
        TerserMut::new(&mut message)
            .set("PID-5", "DOE^JOHN^A")
            .unwrap();
        let terser = Terser::new(&message);
        assert_eq!(terser.get("PID-5").unwrap(), Some("DOE^JOHN^A"));
        assert_eq!(terser.get("PID-5-2").unwrap(), None);
    }

    #[test]
    fn test_component_set_stores_subcomponent_separator_literally() {
        // Component-level set stores `&` literally as one subcomponent
        // (like whole-field set stores `^` literally): the value reads
        // back whole, with no second subcomponent.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1\r";
        let mut message = parse_message(hl7).unwrap();
        TerserMut::new(&mut message).set("PID-5-1", "a&b").unwrap();
        let terser = Terser::new(&message);
        assert_eq!(terser.get("PID-5-1").unwrap(), Some("a&b"));
        assert_eq!(terser.get("PID-5-1-2").unwrap(), None);
        assert!(message.encode().contains("a\\T\\b"));
    }

    #[test]
    fn test_set_rejects_empty_segment_id() {
        // An empty segment id is a malformed path, not a new segment:
        // no garbage segment may be appended.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r";
        let mut message = parse_message(hl7).unwrap();
        let before = message.segments.len();
        assert!(TerserMut::new(&mut message).set("-5", "x").is_err());
        assert!(TerserMut::new(&mut message).set("   -5", "x").is_err());
        assert_eq!(message.segments.len(), before);
    }

    #[test]
    fn test_set_rejects_segment_id_with_whitespace() {
        // "PID " would create a segment failing validate_id (3 alnum).
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r";
        let mut message = parse_message(hl7).unwrap();
        let before = message.segments.len();
        assert!(TerserMut::new(&mut message).set("PID -5", "x").is_err());
        assert_eq!(message.segments.len(), before);
    }

    #[test]
    fn test_set_rejects_invalid_segment_id_without_mutation() {
        // A new occurrence must carry a valid segment id (3 ASCII
        // alnum): creating "AB" would fail Segment::validate_id.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r";
        let mut message = parse_message(hl7).unwrap();
        let before = message.segments.len();
        for path in ["AB-5", "AB(2)-5", "ABCD-5", "A!-5"] {
            assert!(
                TerserMut::new(&mut message).set(path, "x").is_err(),
                "{path} must fail"
            );
        }
        assert_eq!(message.segments.len(), before);
        assert!(message.get_segments_by_id("AB").is_empty());

        // Reads stay lenient: an absent id is Ok(None), not an error.
        let terser = Terser::new(&message);
        assert_eq!(terser.get("AB-5").unwrap(), None);

        // A valid new id still creates its occurrence.
        TerserMut::new(&mut message).set("ZZZ-5", "x").unwrap();
        assert_eq!(Terser::new(&message).get("ZZZ-5").unwrap(), Some("x"));
    }

    #[test]
    fn test_set_rejects_delimiter_fields_on_msh_fhs_bhs() {
        // MSH/FHS/BHS fields 1 and 2 are the message delimiters: encode
        // derives them from Delimiters and drops anything written here,
        // so writes are rejected instead of silently lost.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\r";
        let mut message = parse_message(hl7).unwrap();
        {
            let mut terser = TerserMut::new(&mut message);
            for path in [
                "MSH-1", "MSH-2", "MSH-2-1", "FHS-1", "FHS-2", "BHS-1", "BHS-2",
            ] {
                assert!(terser.set(path, "@@@@").is_err(), "{path} must fail");
            }
            // Control: neighboring fields still write.
            terser.set("MSH-3", "APP2").unwrap();
        }
        let terser = Terser::new(&message);
        assert_eq!(terser.get("MSH-3").unwrap(), Some("APP2"));
        assert!(message.encode().starts_with("MSH|^~\\&|"));
    }

    #[test]
    fn test_set_zero_index_rejected_without_mutation() {
        // 0 indices are 1-based violations: the write must fail before
        // gap-filling appends segments or fields.
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ADT^A01|123|P|2.5\rPID|1\r";
        let mut message = parse_message(hl7).unwrap();
        let segs_before = message.segments.len();
        let pid_fields_before = message.get_segments_by_id("PID")[0].fields.len();
        assert!(TerserMut::new(&mut message).set("ZZZ-0", "x").is_err());
        assert_eq!(message.segments.len(), segs_before);
        assert!(message.get_segments_by_id("ZZZ").is_empty());
        for path in ["PID-9-0", "PID-5-0", "PID-5-1-0"] {
            assert!(
                TerserMut::new(&mut message).set(path, "x").is_err(),
                "{path} must fail"
            );
        }
        assert_eq!(message.segments.len(), segs_before);
        assert_eq!(
            message.get_segments_by_id("PID")[0].fields.len(),
            pid_fields_before
        );
    }

    #[test]
    fn test_set_within_cap_still_roundtrips() {
        let hl7 = "MSH|^~\\&|APP|FAC|||20250115||ORU^R01|123|P|2.5\r\
                   OBX|1|NM|GLU||98\r";
        let mut message = parse_message(hl7).unwrap();
        let mut terser = TerserMut::new(&mut message);

        terser.set("OBX(100)-5", "4.2").unwrap();

        let read_terser = Terser::new(&message);
        assert_eq!(read_terser.get("OBX(100)-5").unwrap(), Some("4.2"));
        assert_eq!(message.get_segments_by_id("OBX").len(), 100);
    }
}
