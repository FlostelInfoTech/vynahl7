//! Segment group navigation for complex HL7 messages
//!
//! HL7 messages organize segments into logical groups. For example, in an ORU_R01 message:
//! - PATIENT group: PID, PV1
//! - ORDER_OBSERVATION group: OBR with associated OBX segments
//! - SPECIMEN group: SPM with associated OBX segments
//!
//! This module provides navigation and iteration over these segment groups.
//!
//! # Examples
//!
//! ```rust
//! use vyna_hl7_terser::{Terser, SegmentGroup, GroupNavigator};
//! use vyna_hl7_parser::parse_message;
//!
//! # fn main() -> vyna_hl7_core::Result<()> {
//! let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
//! PID|1||PAT001||DOE^JOHN
//! OBR|1||ORDER001|CBC^Complete Blood Count
//! OBX|1|NM|WBC||7.5|10*3/uL
//! OBX|2|NM|RBC||4.5|10*6/uL
//! OBR|2||ORDER002|BMP^Basic Metabolic Panel
//! OBX|1|NM|GLU||98|mg/dL
//! OBX|2|NM|NA||140|mmol/L";
//!
//! let message = parse_message(hl7)?;
//! let navigator = GroupNavigator::new(&message);
//!
//! // Navigate ORDER_OBSERVATION groups (OBR + associated OBX segments)
//! let order_groups = navigator.order_observations();
//! assert_eq!(order_groups.len(), 2);
//!
//! // First order group has 2 OBX segments
//! assert_eq!(order_groups[0].children.len(), 2);
//! # Ok(())
//! # }
//! ```

use vyna_hl7_core::{message::Message, segment::Segment};

/// A segment group containing a header segment and child segments
#[derive(Debug, Clone)]
pub struct SegmentGroup<'a> {
    /// The header/anchor segment of the group (e.g., OBR for ORDER_OBSERVATION)
    pub header: &'a Segment,
    /// The index of the header segment in the message
    pub header_index: usize,
    /// Child segments that belong to this group (e.g., OBX segments under OBR)
    pub children: Vec<&'a Segment>,
    /// Indices of child segments in the message
    pub children_indices: Vec<usize>,
}

impl<'a> SegmentGroup<'a> {
    /// Create a new segment group
    pub fn new(header: &'a Segment, header_index: usize) -> Self {
        Self {
            header,
            header_index,
            children: Vec::new(),
            children_indices: Vec::new(),
        }
    }

    /// Add a child segment to the group
    pub fn add_child(&mut self, segment: &'a Segment, index: usize) {
        self.children.push(segment);
        self.children_indices.push(index);
    }

    /// Get the header segment ID
    pub fn header_id(&self) -> &str {
        &self.header.id
    }

    /// Get the number of child segments
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Check if the group has any children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Get a child segment by index (0-based)
    pub fn get_child(&self, index: usize) -> Option<&'a Segment> {
        self.children.get(index).copied()
    }

    /// Iterate over child segments
    pub fn iter_children(&self) -> impl Iterator<Item = &'a Segment> {
        self.children.iter().copied()
    }

    /// Get field value from the header segment
    pub fn header_field(&self, field_index: usize) -> Option<&str> {
        self.header.get_field(field_index)?.value()
    }

    /// Get field values from all children for a specific field
    pub fn child_field_values(&self, field_index: usize) -> Vec<Option<&str>> {
        self.children
            .iter()
            .map(|seg| seg.get_field(field_index).and_then(|f| f.value()))
            .collect()
    }
}

/// Known segment group patterns in HL7 messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupPattern {
    /// ORDER_OBSERVATION: OBR as header with OBX children
    OrderObservation,
    /// PATIENT: PID as header with optional PD1, PV1, PV2
    Patient,
    /// SPECIMEN: SPM as header with OBX children
    Specimen,
    /// INSURANCE: IN1 as header with IN2, IN3
    Insurance,
    /// ORDER: ORC as header with associated segments
    Order,
    /// TIMING: TQ1 as header with TQ2 children
    Timing,
    /// PROCEDURE: PR1 as header
    Procedure,
    /// DIAGNOSIS: DG1 as header
    Diagnosis,
    /// Custom group with user-defined header and child segment IDs
    Custom,
}

/// Configuration for a segment group pattern
#[derive(Debug, Clone)]
pub struct GroupConfig {
    /// Header segment ID (the anchor of the group)
    pub header_id: String,
    /// Allowed child segment IDs
    pub child_ids: Vec<String>,
    /// Segment IDs that terminate the group (start a new group)
    pub terminating_ids: Vec<String>,
}

impl GroupConfig {
    /// Create a new group configuration
    pub fn new(header_id: &str) -> Self {
        Self {
            header_id: header_id.to_string(),
            child_ids: Vec::new(),
            terminating_ids: Vec::new(),
        }
    }

    /// Add allowed child segment IDs
    pub fn with_children(mut self, children: &[&str]) -> Self {
        self.child_ids = children.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add terminating segment IDs
    pub fn with_terminators(mut self, terminators: &[&str]) -> Self {
        self.terminating_ids = terminators.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Get the ORDER_OBSERVATION group configuration
    pub fn order_observation() -> Self {
        Self::new("OBR")
            .with_children(&["OBX", "NTE", "TCD", "SID"])
            .with_terminators(&["OBR", "ORC", "SPM", "PID", "MSH"])
    }

    /// Get the PATIENT group configuration
    pub fn patient() -> Self {
        Self::new("PID")
            .with_children(&[
                "PD1", "PV1", "PV2", "NK1", "AL1", "DB1", "DRG", "GT1", "IN1", "IN2",
            ])
            .with_terminators(&["PID", "OBR", "ORC", "MSH"])
    }

    /// Get the SPECIMEN group configuration
    pub fn specimen() -> Self {
        Self::new("SPM")
            .with_children(&["OBX", "NTE", "SAC"])
            .with_terminators(&["SPM", "OBR", "ORC", "PID", "MSH"])
    }

    /// Get the INSURANCE group configuration
    pub fn insurance() -> Self {
        Self::new("IN1")
            .with_children(&["IN2", "IN3"])
            .with_terminators(&["IN1", "GT1", "OBR", "ORC", "PID", "MSH"])
    }

    /// Get the ORDER group configuration
    pub fn order() -> Self {
        Self::new("ORC")
            .with_children(&[
                "OBR", "RXO", "RXE", "RXD", "RXG", "RXA", "TQ1", "TQ2", "OBX", "NTE",
            ])
            .with_terminators(&["ORC", "PID", "MSH"])
    }

    /// Check if a segment ID is a valid child
    pub fn is_child(&self, segment_id: &str) -> bool {
        self.child_ids.iter().any(|id| id == segment_id)
    }

    /// Check if a segment ID terminates the group
    pub fn is_terminator(&self, segment_id: &str) -> bool {
        self.terminating_ids.iter().any(|id| id == segment_id)
    }
}

/// Navigator for traversing segment groups in an HL7 message
pub struct GroupNavigator<'a> {
    message: &'a Message,
}

impl<'a> GroupNavigator<'a> {
    /// Create a new group navigator for a message
    pub fn new(message: &'a Message) -> Self {
        Self { message }
    }

    /// Extract groups based on a configuration
    pub fn extract_groups(&self, config: &GroupConfig) -> Vec<SegmentGroup<'a>> {
        let mut groups = Vec::new();
        let mut current_group: Option<SegmentGroup<'a>> = None;

        for (idx, segment) in self.message.segments.iter().enumerate() {
            if segment.id == config.header_id {
                // Save previous group if exists
                if let Some(group) = current_group.take() {
                    groups.push(group);
                }
                // Start new group
                current_group = Some(SegmentGroup::new(segment, idx));
            } else if let Some(ref mut group) = current_group {
                if config.is_child(&segment.id) {
                    group.add_child(segment, idx);
                } else if config.is_terminator(&segment.id) {
                    // Save current group and stop
                    groups.push(current_group.take().unwrap());
                }
            }
        }

        // Don't forget the last group
        if let Some(group) = current_group {
            groups.push(group);
        }

        groups
    }

    /// Get all ORDER_OBSERVATION groups (OBR with associated OBX segments)
    pub fn order_observations(&self) -> Vec<SegmentGroup<'a>> {
        self.extract_groups(&GroupConfig::order_observation())
    }

    /// Get all PATIENT groups (PID with associated segments)
    pub fn patients(&self) -> Vec<SegmentGroup<'a>> {
        self.extract_groups(&GroupConfig::patient())
    }

    /// Get all SPECIMEN groups (SPM with associated OBX segments)
    pub fn specimens(&self) -> Vec<SegmentGroup<'a>> {
        self.extract_groups(&GroupConfig::specimen())
    }

    /// Get all INSURANCE groups (IN1 with associated IN2/IN3)
    pub fn insurance_groups(&self) -> Vec<SegmentGroup<'a>> {
        self.extract_groups(&GroupConfig::insurance())
    }

    /// Get all ORDER groups (ORC with associated segments)
    pub fn orders(&self) -> Vec<SegmentGroup<'a>> {
        self.extract_groups(&GroupConfig::order())
    }

    /// Extract custom groups with a user-defined configuration
    pub fn custom_groups(&self, header_id: &str, child_ids: &[&str]) -> Vec<SegmentGroup<'a>> {
        let config = GroupConfig::new(header_id).with_children(child_ids);
        self.extract_groups(&config)
    }

    /// Find a specific group by header field value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use vyna_hl7_terser::GroupNavigator;
    /// use vyna_hl7_parser::parse_message;
    ///
    /// # fn main() -> vyna_hl7_core::Result<()> {
    /// let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
    /// OBR|1||ORDER001|CBC
    /// OBX|1|NM|WBC||7.5
    /// OBR|2||ORDER002|BMP
    /// OBX|1|NM|GLU||98";
    ///
    /// let message = parse_message(hl7)?;
    /// let navigator = GroupNavigator::new(&message);
    ///
    /// // Find the ORDER_OBSERVATION group with filler order number "ORDER002"
    /// let config = vyna_hl7_terser::GroupConfig::order_observation();
    /// let group = navigator.find_group_by_field(&config, 3, "ORDER002");
    /// assert!(group.is_some());
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_group_by_field(
        &self,
        config: &GroupConfig,
        field_index: usize,
        value: &str,
    ) -> Option<SegmentGroup<'a>> {
        self.extract_groups(config)
            .into_iter()
            .find(|group| group.header_field(field_index) == Some(value))
    }

    /// Count segments of a specific type within groups
    pub fn count_segments_in_groups(&self, config: &GroupConfig, segment_id: &str) -> Vec<usize> {
        self.extract_groups(config)
            .iter()
            .map(|group| {
                group
                    .children
                    .iter()
                    .filter(|seg| seg.id == segment_id)
                    .count()
            })
            .collect()
    }
}

/// Iterator over segment groups
///
/// Group membership is resolved once at construction; the iterator yields
/// the groups found in the message at that point.
pub struct GroupIterator<'a> {
    current_index: usize,
    groups: Vec<SegmentGroup<'a>>,
    // Phantom to tie lifetime
    _phantom: std::marker::PhantomData<&'a Message>,
}

impl<'a> GroupIterator<'a> {
    /// Create a new group iterator
    ///
    /// Extracts the groups matching `config` from `message` immediately;
    /// iteration then walks that snapshot in message order.
    pub fn new(message: &'a Message, config: GroupConfig) -> Self {
        let navigator = GroupNavigator::new(message);
        let groups = navigator.extract_groups(&config);
        Self {
            current_index: 0,
            groups,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> Iterator for GroupIterator<'a> {
    type Item = SegmentGroup<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.groups.len() {
            let group = self.groups[self.current_index].clone();
            self.current_index += 1;
            Some(group)
        } else {
            None
        }
    }
}

/// Extension trait to add group navigation to Terser
impl<'a> crate::Terser<'a> {
    /// Create a group navigator for this message
    pub fn group_navigator(&self) -> GroupNavigator<'a> {
        GroupNavigator::new(self.message)
    }

    /// Get all ORDER_OBSERVATION groups
    pub fn order_observations(&self) -> Vec<SegmentGroup<'a>> {
        self.group_navigator().order_observations()
    }

    /// Get all PATIENT groups
    pub fn patients(&self) -> Vec<SegmentGroup<'a>> {
        self.group_navigator().patients()
    }

    /// Iterate over ORDER_OBSERVATION groups
    pub fn iter_order_observations(&self) -> GroupIterator<'a> {
        GroupIterator::new(self.message, GroupConfig::order_observation())
    }

    /// Iterate over groups with custom configuration
    pub fn iter_groups(&self, config: GroupConfig) -> GroupIterator<'a> {
        GroupIterator::new(self.message, config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    fn sample_oru_message() -> Message {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
PID|1||PAT001||DOE^JOHN
OBR|1||ORDER001|CBC^Complete Blood Count
OBX|1|NM|WBC||7.5|10*3/uL
OBX|2|NM|RBC||4.5|10*6/uL
OBX|3|NM|HGB||14.0|g/dL
NTE|1||Normal range: 4.5-11.0
OBR|2||ORDER002|BMP^Basic Metabolic Panel
OBX|1|NM|GLU||98|mg/dL
OBX|2|NM|NA||140|mmol/L
OBX|3|NM|K||4.2|mmol/L";
        parse_message(hl7).unwrap()
    }

    #[test]
    fn test_order_observation_groups() {
        let message = sample_oru_message();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.order_observations();

        assert_eq!(groups.len(), 2);

        // First order has 3 OBX and 1 NTE
        assert_eq!(groups[0].header_id(), "OBR");
        assert_eq!(groups[0].child_count(), 4); // 3 OBX + 1 NTE

        // Second order has 3 OBX
        assert_eq!(groups[1].header_id(), "OBR");
        assert_eq!(groups[1].child_count(), 3);
    }

    #[test]
    fn test_group_header_field() {
        let message = sample_oru_message();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.order_observations();

        // OBR-3 is filler order number
        assert_eq!(groups[0].header_field(3), Some("ORDER001"));
        assert_eq!(groups[1].header_field(3), Some("ORDER002"));
    }

    #[test]
    fn test_group_child_field_values() {
        let message = sample_oru_message();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.order_observations();

        // Get OBX-5 (observation values) from first order group
        let values: Vec<_> = groups[0]
            .children
            .iter()
            .filter(|seg| seg.id == "OBX")
            .filter_map(|seg| seg.get_field(5).and_then(|f| f.value()))
            .collect();

        assert_eq!(values, vec!["7.5", "4.5", "14.0"]);
    }

    #[test]
    fn test_find_group_by_field() {
        let message = sample_oru_message();
        let navigator = GroupNavigator::new(&message);

        let config = GroupConfig::order_observation();

        // Find by OBR-3 (filler order number)
        let group = navigator.find_group_by_field(&config, 3, "ORDER002");
        assert!(group.is_some());

        let group = group.unwrap();
        assert_eq!(group.child_count(), 3);
    }

    #[test]
    fn test_patient_group() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN
PV1|1|I|ICU^001||||||SMITH^JANE";

        let message = parse_message(hl7).unwrap();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.patients();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].header_id(), "PID");
        assert_eq!(groups[0].child_count(), 1); // PV1
    }

    #[test]
    fn test_custom_groups() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
IN1|1|INS001|BCBS
IN2|1|Subscriber info
IN1|2|INS002|AETNA
IN2|1|More info
IN3|1|Authorization";

        let message = parse_message(hl7).unwrap();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.insurance_groups();

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].child_count(), 1); // IN2 only
        assert_eq!(groups[1].child_count(), 2); // IN2 + IN3
    }

    #[test]
    fn test_terser_extension() {
        let message = sample_oru_message();
        let terser = crate::Terser::new(&message);

        let groups = terser.order_observations();
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn test_group_iterator() {
        let message = sample_oru_message();
        let navigator = GroupNavigator::new(&message);

        let count = navigator.order_observations().len();
        assert_eq!(count, 2);

        // Using iterator
        let iter_count: usize =
            GroupIterator::new(&message, GroupConfig::order_observation()).count();
        assert_eq!(iter_count, 2);
    }

    #[test]
    fn test_iterate_group_children() {
        let message = sample_oru_message();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.order_observations();
        let first_group = &groups[0];

        // Iterate and collect OBX segment IDs
        let obx_count = first_group
            .iter_children()
            .filter(|seg| seg.id == "OBX")
            .count();

        assert_eq!(obx_count, 3);
    }

    #[test]
    fn test_count_segments_in_groups() {
        let message = sample_oru_message();
        let navigator = GroupNavigator::new(&message);

        let config = GroupConfig::order_observation();
        let obx_counts = navigator.count_segments_in_groups(&config, "OBX");

        assert_eq!(obx_counts, vec![3, 3]);
    }

    #[test]
    fn test_empty_message() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ADT^A01|123|P|2.5
PID|1||PAT001";

        let message = parse_message(hl7).unwrap();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.order_observations();
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_group_without_children() {
        let hl7 = r"MSH|^~\&|APP|FAC|||20250115||ORU^R01|123|P|2.5
OBR|1||ORDER001|CBC";

        let message = parse_message(hl7).unwrap();
        let navigator = GroupNavigator::new(&message);

        let groups = navigator.order_observations();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].child_count(), 0);
        assert!(!groups[0].has_children());
    }
}
