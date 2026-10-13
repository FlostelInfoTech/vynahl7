//! Field iteration for repeating segments
//!
//! This module provides iterators for walking through field values across
//! multiple segments or repetitions.

use crate::Terser;
use vyna_hl7_core::Message;

/// Iterator over field values from repeating segments
///
/// # Examples
///
/// ```
/// use vyna_hl7_terser::Terser;
/// use vyna_hl7_parser::parse_message;
///
/// # fn main() -> vyna_hl7_core::Result<()> {
/// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
/// OBX|1|NM|GLU||98|mg/dL
/// OBX|2|NM|NA||140|mmol/L
/// OBX|3|NM|K||4.2|mmol/L";
///
/// let message = parse_message(hl7)?;
/// let terser = Terser::new(&message);
///
/// // Iterate over all observation values (OBX-5)
/// let values: Vec<&str> = terser.iter_field("OBX", 5).collect();
///
/// assert_eq!(values, vec!["98", "140", "4.2"]);
/// # Ok(())
/// # }
/// ```
pub struct FieldIterator<'a> {
    message: &'a Message,
    segment_id: String,
    field_index: usize,
    current_segment: usize,
}

impl<'a> FieldIterator<'a> {
    /// Create a new field iterator
    pub(crate) fn new(message: &'a Message, segment_id: &str, field_index: usize) -> Self {
        Self {
            message,
            segment_id: segment_id.to_string(),
            field_index,
            current_segment: 0,
        }
    }
}

impl<'a> Iterator for FieldIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let segments = self.message.get_segments_by_id(&self.segment_id);

        while self.current_segment < segments.len() {
            let segment = &segments[self.current_segment];
            self.current_segment += 1;

            if let Some(field) = segment.get_field(self.field_index)
                && let Some(value) = field.value()
            {
                // Skip empty values
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }

        None
    }
}

/// Iterator over component values from a specific field
///
/// # Examples
///
/// ```
/// use vyna_hl7_terser::Terser;
/// use vyna_hl7_parser::parse_message;
///
/// # fn main() -> vyna_hl7_core::Result<()> {
/// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
/// OBX|1|CE|GLU||98^mg/dL^ISO
/// OBX|2|CE|NA||140^mmol/L^ISO
/// OBX|3|CE|K||4.2^mmol/L^ISO";
///
/// let message = parse_message(hl7)?;
/// let terser = Terser::new(&message);
///
/// // Iterate over second component (units) of all OBX-5 fields
/// let units: Vec<&str> = terser.iter_component("OBX", 5, 2).collect();
///
/// assert_eq!(units, vec!["mg/dL", "mmol/L", "mmol/L"]);
/// # Ok(())
/// # }
/// ```
pub struct ComponentIterator<'a> {
    message: &'a Message,
    segment_id: String,
    field_index: usize,
    component_index: usize,
    current_segment: usize,
}

impl<'a> ComponentIterator<'a> {
    /// Create a new component iterator
    pub(crate) fn new(
        message: &'a Message,
        segment_id: &str,
        field_index: usize,
        component_index: usize,
    ) -> Self {
        Self {
            message,
            segment_id: segment_id.to_string(),
            field_index,
            component_index,
            current_segment: 0,
        }
    }
}

impl<'a> Iterator for ComponentIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let segments = self.message.get_segments_by_id(&self.segment_id);

        while self.current_segment < segments.len() {
            let segment = &segments[self.current_segment];
            self.current_segment += 1;

            if let Some(field) = segment.get_field(self.field_index)
                && let Some(repetition) = field.get_repetition(0)
            {
                // Component indices are 1-based in HL7, convert to 0-based
                if self.component_index > 0
                    && let Some(component) = repetition.get_component(self.component_index - 1)
                    && let Some(value) = component.value()
                {
                    // Skip empty values
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
        }

        None
    }
}

/// Iterator over field repetitions
///
/// # Examples
///
/// ```
/// use vyna_hl7_terser::Terser;
/// use vyna_hl7_parser::parse_message;
///
/// # fn main() -> vyna_hl7_core::Result<()> {
/// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
/// PID|1||PAT001||DOE^JOHN||19800101|M|||123 Main St~456 Oak Ave";
///
/// let message = parse_message(hl7)?;
/// let terser = Terser::new(&message);
///
/// // Iterate over all address repetitions in PID-11 (segment index 0 = first PID)
/// let addresses: Vec<&str> = terser.iter_repetitions("PID", 11, 0).collect();
///
/// assert_eq!(addresses.len(), 2);
/// assert_eq!(addresses[0], "123 Main St");
/// assert_eq!(addresses[1], "456 Oak Ave");
/// # Ok(())
/// # }
/// ```
pub struct RepetitionIterator<'a> {
    message: &'a Message,
    segment_id: String,
    field_index: usize,
    segment_index: usize,
    current_repetition: usize,
}

impl<'a> RepetitionIterator<'a> {
    /// Create a new repetition iterator
    pub(crate) fn new(
        message: &'a Message,
        segment_id: &str,
        field_index: usize,
        segment_index: usize,
    ) -> Self {
        Self {
            message,
            segment_id: segment_id.to_string(),
            field_index,
            segment_index,
            current_repetition: 0,
        }
    }
}

impl<'a> Iterator for RepetitionIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let segments = self.message.get_segments_by_id(&self.segment_id);

        if self.segment_index >= segments.len() {
            return None;
        }

        let segment = &segments[self.segment_index];

        if let Some(field) = segment.get_field(self.field_index) {
            while self.current_repetition < field.repetitions.len() {
                let rep_idx = self.current_repetition;
                self.current_repetition += 1;

                if let Some(repetition) = field.get_repetition(rep_idx)
                    && let Some(value) = repetition.value()
                {
                    // Skip empty values
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
        }

        None
    }
}

/// Extension trait for Terser to add iterator methods
impl<'a> Terser<'a> {
    /// Create an iterator over a field from all instances of a repeating segment
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::Terser;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|NM|GLU||98
    /// OBX|2|NM|NA||140
    /// OBX|3|NM|K||4.2";
    ///
    /// let message = parse_message(hl7)?;
    /// let terser = Terser::new(&message);
    ///
    /// // Iterate over all OBX-5 values
    /// let values: Vec<&str> = terser.iter_field("OBX", 5).collect();
    ///
    /// assert_eq!(values, vec!["98", "140", "4.2"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn iter_field(&self, segment_id: &str, field: usize) -> FieldIterator<'a> {
        FieldIterator::new(self.message, segment_id, field)
    }

    /// Create an iterator over a specific component from all instances of a repeating segment
    ///
    /// Component index is 1-based (HL7 standard).
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::Terser;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBX|1|CE|GLU||98^mg/dL
    /// OBX|2|CE|NA||140^mmol/L";
    ///
    /// let message = parse_message(hl7)?;
    /// let terser = Terser::new(&message);
    ///
    /// // Get all units (component 2) from OBX-5
    /// let units: Vec<&str> = terser.iter_component("OBX", 5, 2).collect();
    ///
    /// assert_eq!(units, vec!["mg/dL", "mmol/L"]);
    /// # Ok(())
    /// # }
    /// ```
    pub fn iter_component(
        &self,
        segment_id: &str,
        field: usize,
        component: usize,
    ) -> ComponentIterator<'a> {
        ComponentIterator::new(self.message, segment_id, field, component)
    }

    /// Create an iterator over repetitions of a field within a single segment
    ///
    /// `segment_index` is a 0-based position into the segments with the
    /// given id, so `0` selects the first one and an out-of-range position
    /// yields no items. Note this differs from path notation, where every
    /// index (`OBX(1)`, `PID-5(1)`, components, subcomponents) is 1-based:
    /// this argument is a direct position, not a parsed path.
    ///
    /// # Examples
    ///
    /// ```
    /// use vyna_hl7_terser::Terser;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
    /// PID|1||PAT001||DOE^JOHN||19800101|M|||123 Main St~456 Oak Ave~789 Pine Rd";
    ///
    /// let message = parse_message(hl7)?;
    /// let terser = Terser::new(&message);
    ///
    /// // Iterate over all address repetitions in PID-11 (segment index 0 = first PID)
    /// let addresses: Vec<&str> = terser.iter_repetitions("PID", 11, 0).collect();
    ///
    /// assert_eq!(addresses.len(), 3);
    /// assert_eq!(addresses[0], "123 Main St");
    /// assert_eq!(addresses[1], "456 Oak Ave");
    /// assert_eq!(addresses[2], "789 Pine Rd");
    /// # Ok(())
    /// # }
    /// ```
    pub fn iter_repetitions(
        &self,
        segment_id: &str,
        field: usize,
        segment_index: usize,
    ) -> RepetitionIterator<'a> {
        RepetitionIterator::new(self.message, segment_id, field, segment_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_field_iterator_basic() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98|mg/dL
OBX|2|NM|NA||140|mmol/L
OBX|3|NM|K||4.2|mmol/L";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        let values: Vec<&str> = terser.iter_field("OBX", 5).collect();

        assert_eq!(values, vec!["98", "140", "4.2"]);
    }

    #[test]
    fn test_field_iterator_empty() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // No OBX segments
        let values: Vec<&str> = terser.iter_field("OBX", 5).collect();

        assert_eq!(values.len(), 0);
    }

    #[test]
    fn test_component_iterator() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|CE|GLU||98^mg/dL^ISO
OBX|2|CE|NA||140^mmol/L^ISO
OBX|3|CE|K||4.2^mmol/L^ISO";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // Get all units (component 2)
        let units: Vec<&str> = terser.iter_component("OBX", 5, 2).collect();

        assert_eq!(units, vec!["mg/dL", "mmol/L", "mmol/L"]);

        // Get all coding systems (component 3)
        let systems: Vec<&str> = terser.iter_component("OBX", 5, 3).collect();

        assert_eq!(systems, vec!["ISO", "ISO", "ISO"]);
    }

    #[test]
    fn test_repetition_iterator() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN||19800101|M|||123 Main St~456 Oak Ave~789 Pine Rd";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // Iterate over all address repetitions in PID-11
        let addresses: Vec<&str> = terser.iter_repetitions("PID", 11, 0).collect();

        assert_eq!(addresses.len(), 3);
        assert_eq!(addresses[0], "123 Main St");
        assert_eq!(addresses[1], "456 Oak Ave");
        assert_eq!(addresses[2], "789 Pine Rd");
    }

    #[test]
    fn test_repetition_iterator_segment_index_is_zero_based() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN||19800101|M|||111 First St~222 Second St
PID|2||PAT002||SMITH^JANE||19900202|F|||333 Third Ave";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // segment_index 0 selects the first PID segment (0-based argument,
        // unlike 1-based path notation).
        let first: Vec<&str> = terser.iter_repetitions("PID", 11, 0).collect();
        assert_eq!(first, vec!["111 First St", "222 Second St"]);

        let second: Vec<&str> = terser.iter_repetitions("PID", 11, 1).collect();
        assert_eq!(second, vec!["333 Third Ave"]);

        // Out-of-range segment position yields no items.
        let none: Vec<&str> = terser.iter_repetitions("PID", 11, 5).collect();
        assert!(none.is_empty());
    }

    #[test]
    fn test_repetition_iterator_no_repetitions() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN||19800101|M|||123 Main St";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // Single address (no repetition separator)
        let addresses: Vec<&str> = terser.iter_repetitions("PID", 11, 0).collect();

        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0], "123 Main St");
    }

    #[test]
    fn test_iterator_with_missing_fields() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||
OBX|3|NM|K||4.2";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // Second OBX has empty value
        let values: Vec<&str> = terser.iter_field("OBX", 5).collect();

        // Should skip empty values
        assert_eq!(values, vec!["98", "4.2"]);
    }

    #[test]
    fn test_iterator_count() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2
OBX|4|NM|CL||102
OBX|5|NM|CO2||24";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        let count = terser.iter_field("OBX", 5).count();

        assert_eq!(count, 5);
    }

    #[test]
    fn test_iterator_filter() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // Filter values > 100
        let high_values: Vec<&str> = terser
            .iter_field("OBX", 5)
            .filter(|v| v.parse::<f64>().map(|n| n > 100.0).unwrap_or(false))
            .collect();

        assert_eq!(high_values, vec!["140"]);
    }

    #[test]
    fn test_iterator_map() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBX|1|NM|GLU||98
OBX|2|NM|NA||140
OBX|3|NM|K||4.2";

        let message = parse_message(hl7).unwrap();
        let terser = Terser::new(&message);

        // Parse all values as f64
        let values: Vec<f64> = terser
            .iter_field("OBX", 5)
            .filter_map(|v| v.parse().ok())
            .collect();

        assert_eq!(values, vec![98.0, 140.0, 4.2]);
    }
}
