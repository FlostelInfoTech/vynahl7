//! Conformance profile data structures

pub mod parser;

use crate::error::Result;
use vyna_hl7_core::Version;

/// Root conformance profile structure
#[derive(Debug, Clone)]
pub struct ConformanceProfile {
    /// Profile metadata
    pub metadata: ProfileMetadata,
    /// Message profile (segments and fields)
    pub message: MessageProfile,
}

impl ConformanceProfile {
    /// Create a new conformance profile
    pub fn new(metadata: ProfileMetadata, message: MessageProfile) -> Self {
        Self { metadata, message }
    }

    /// Validate the profile structure itself
    ///
    /// Checks that the profile is well-formed before it can be used to validate messages.
    /// This validates metadata, message structure, segment definitions, and field constraints.
    pub fn validate(&self) -> Result<()> {
        // Validate metadata
        if self.metadata.name.trim().is_empty() {
            return Err(crate::error::ConformanceError::InvalidProfile(
                "Profile name must not be empty".to_string(),
            ));
        }
        if self.metadata.version.trim().is_empty() {
            return Err(crate::error::ConformanceError::InvalidProfile(
                "Profile version must not be empty".to_string(),
            ));
        }

        // Validate message structure
        if self.message.message_type.trim().is_empty() {
            return Err(crate::error::ConformanceError::InvalidProfile(
                "Message type must not be empty".to_string(),
            ));
        }
        if self.message.trigger_event.trim().is_empty() {
            return Err(crate::error::ConformanceError::InvalidProfile(
                "Trigger event must not be empty".to_string(),
            ));
        }
        if self.message.segments.is_empty() {
            return Err(crate::error::ConformanceError::InvalidProfile(
                "Profile must define at least one segment".to_string(),
            ));
        }

        // Validate each segment
        for segment in &self.message.segments {
            // Segment name must be a valid 3-character identifier
            if segment.name.trim().is_empty() {
                return Err(crate::error::ConformanceError::InvalidProfile(
                    "Segment name must not be empty".to_string(),
                ));
            }
            if segment.name.len() != 3 || !segment.name.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err(crate::error::ConformanceError::InvalidProfile(
                    format!("Invalid segment ID '{}': must be exactly 3 alphanumeric characters", segment.name),
                ));
            }

            // Validate field profiles
            for field in &segment.fields {
                if field.position == 0 {
                    return Err(crate::error::ConformanceError::InvalidProfile(
                        format!("Field position must be >= 1 in segment '{}'", segment.name),
                    ));
                }

                if let Some(length) = field.length
                    && length == 0 {
                        return Err(crate::error::ConformanceError::InvalidProfile(
                            format!("Field {}-{} length must be > 0", segment.name, field.position),
                        ));
                    }

                // Validate component profiles if present
                if let Some(ref components) = field.components {
                    for component in components {
                        if component.position == 0 {
                            return Err(crate::error::ConformanceError::InvalidProfile(
                                format!(
                                    "Component position must be >= 1 in field {}-{}",
                                    segment.name, field.position
                                ),
                            ));
                        }
                        if let Some(length) = component.length
                            && length == 0 {
                                return Err(crate::error::ConformanceError::InvalidProfile(
                                    format!(
                                        "Component {}-{}.{} length must be > 0",
                                        segment.name, field.position, component.position
                                    ),
                                ));
                            }
                    }
                }

                // Validate value set binding if present
                if let Some(ref vs) = field.value_set
                    && vs.value_set_id.trim().is_empty() {
                        return Err(crate::error::ConformanceError::InvalidProfile(
                            format!(
                                "Value set ID must not be empty for field {}-{}",
                                segment.name, field.position
                            ),
                        ));
                    }

                // Validate predicate condition if conditional
                if let ConditionalUsage::Conditional(ref predicate) = field.usage
                    && predicate.condition.trim().is_empty() {
                        return Err(crate::error::ConformanceError::InvalidProfile(
                            format!(
                                "Predicate condition must not be empty for conditional field {}-{}",
                                segment.name, field.position
                            ),
                        ));
                    }
            }
        }

        // Validate co-constraints if present
        if let Some(ref constraints) = self.message.co_constraints {
            for constraint in constraints {
                if constraint.id.trim().is_empty() {
                    return Err(crate::error::ConformanceError::InvalidProfile(
                        "Co-constraint ID must not be empty".to_string(),
                    ));
                }
                if constraint.condition.trim().is_empty() {
                    return Err(crate::error::ConformanceError::InvalidProfile(
                        format!("Co-constraint '{}' condition must not be empty", constraint.id),
                    ));
                }
            }
        }

        Ok(())
    }
}

/// Profile metadata
#[derive(Debug, Clone)]
pub struct ProfileMetadata {
    /// Profile name
    pub name: String,
    /// Profile version
    pub version: String,
    /// Organization name
    pub organization: Option<String>,
    /// HL7 version this profile is based on
    pub hl7_version: Version,
    /// Creation/modification date
    pub date: Option<String>,
    /// Profile description
    pub description: Option<String>,
}

impl ProfileMetadata {
    /// Create new profile metadata
    pub fn new(name: String, version: String, hl7_version: Version) -> Self {
        Self {
            name,
            version,
            organization: None,
            hl7_version,
            date: None,
            description: None,
        }
    }
}

/// Message profile with segment structure
#[derive(Debug, Clone)]
pub struct MessageProfile {
    /// Message type (e.g., "ADT")
    pub message_type: String,
    /// Trigger event (e.g., "A01")
    pub trigger_event: String,
    /// Segment profiles in order
    pub segments: Vec<SegmentProfile>,
    /// Co-constraints (Phase 2)
    pub co_constraints: Option<Vec<CoConstraint>>,
}

impl MessageProfile {
    /// Create new message profile
    pub fn new(message_type: String, trigger_event: String) -> Self {
        Self {
            message_type,
            trigger_event,
            segments: Vec::new(),
            co_constraints: None,
        }
    }

    /// Add a segment profile
    pub fn add_segment(&mut self, segment: SegmentProfile) {
        self.segments.push(segment);
    }

    /// Add co-constraints
    pub fn with_co_constraints(mut self, co_constraints: Vec<CoConstraint>) -> Self {
        self.co_constraints = Some(co_constraints);
        self
    }
}

/// Segment profile with field constraints
#[derive(Debug, Clone)]
pub struct SegmentProfile {
    /// Segment ID (e.g., "MSH", "PID")
    pub name: String,
    /// Long name/description
    pub long_name: Option<String>,
    /// Usage code (R, RE, O, X)
    pub usage: Usage,
    /// Cardinality (min/max occurrences)
    pub cardinality: Cardinality,
    /// Field profiles
    pub fields: Vec<FieldProfile>,
}

impl SegmentProfile {
    /// Create new segment profile
    pub fn new(name: String, usage: Usage, cardinality: Cardinality) -> Self {
        Self {
            name,
            long_name: None,
            usage,
            cardinality,
            fields: Vec::new(),
        }
    }

    /// Add a field profile
    pub fn add_field(&mut self, field: FieldProfile) {
        self.fields.push(field);
    }

    /// Get field by position
    pub fn get_field(&self, position: usize) -> Option<&FieldProfile> {
        self.fields.iter().find(|f| f.position == position)
    }
}

/// Component profile for composite fields (Phase 2)
#[derive(Debug, Clone)]
pub struct ComponentProfile {
    /// Component position (1-based)
    pub position: usize,
    /// Component name
    pub name: Option<String>,
    /// Usage code (R, RE, O, X, C)
    pub usage: ConditionalUsage,
    /// Data type
    pub datatype: Option<String>,
    /// Maximum length
    pub length: Option<usize>,
    /// HL7 table ID
    pub table_id: Option<String>,
}

impl ComponentProfile {
    /// Create new component profile
    pub fn new(position: usize, usage: ConditionalUsage) -> Self {
        Self {
            position,
            name: None,
            usage,
            datatype: None,
            length: None,
            table_id: None,
        }
    }
}

/// Conditional usage with predicate support (Phase 2)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalUsage {
    /// R - Required
    Required,
    /// RE - Required if Known
    RequiredIfKnown,
    /// O - Optional
    Optional,
    /// X - Not Used
    NotUsed,
    /// C - Conditional based on predicate
    Conditional(Predicate),
}

impl ConditionalUsage {
    /// Convert from basic Usage
    pub fn from_usage(usage: Usage) -> Self {
        match usage {
            Usage::Required => ConditionalUsage::Required,
            Usage::RequiredIfKnown => ConditionalUsage::RequiredIfKnown,
            Usage::Optional => ConditionalUsage::Optional,
            Usage::NotUsed => ConditionalUsage::NotUsed,
        }
    }

    /// Get the basic usage (for backwards compatibility)
    pub fn as_usage(&self) -> Option<Usage> {
        match self {
            ConditionalUsage::Required => Some(Usage::Required),
            ConditionalUsage::RequiredIfKnown => Some(Usage::RequiredIfKnown),
            ConditionalUsage::Optional => Some(Usage::Optional),
            ConditionalUsage::NotUsed => Some(Usage::NotUsed),
            ConditionalUsage::Conditional(_) => None,
        }
    }

    /// Check if this is a conditional usage
    pub fn is_conditional(&self) -> bool {
        matches!(self, ConditionalUsage::Conditional(_))
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            ConditionalUsage::Required => "R",
            ConditionalUsage::RequiredIfKnown => "RE",
            ConditionalUsage::Optional => "O",
            ConditionalUsage::NotUsed => "X",
            ConditionalUsage::Conditional(_) => "C",
        }
    }
}

/// Predicate for conditional usage evaluation (Phase 2)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Predicate {
    /// Condition expression (e.g., "PID-8 IS VALUED")
    pub condition: String,
    /// Usage when condition is true
    pub true_usage: Usage,
    /// Usage when condition is false
    pub false_usage: Usage,
    /// Optional description
    pub description: Option<String>,
}

impl Predicate {
    /// Create new predicate
    pub fn new(condition: String, true_usage: Usage, false_usage: Usage) -> Self {
        Self {
            condition,
            true_usage,
            false_usage,
            description: None,
        }
    }

    /// Add description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

/// Value set binding (Phase 2)
#[derive(Debug, Clone)]
pub struct ValueSetBinding {
    /// Value set ID
    pub value_set_id: String,
    /// Binding strength
    pub strength: BindingStrength,
}

impl ValueSetBinding {
    /// Create new value set binding
    pub fn new(value_set_id: String, strength: BindingStrength) -> Self {
        Self {
            value_set_id,
            strength,
        }
    }
}

/// Binding strength for value sets (Phase 2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStrength {
    /// Required - Must use value from set
    Required,
    /// Extensible - Should use value from set, but may extend
    Extensible,
    /// Preferred - Preferred to use value from set
    Preferred,
    /// Example - Example values only
    Example,
}

impl BindingStrength {
    /// Parse from string
    ///
    /// Note: This method is kept for backward compatibility.
    /// Consider using the `FromStr` trait implementation instead.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "REQUIRED" | "R" => Ok(BindingStrength::Required),
            "EXTENSIBLE" | "E" => Ok(BindingStrength::Extensible),
            "PREFERRED" | "P" => Ok(BindingStrength::Preferred),
            "EXAMPLE" | "X" => Ok(BindingStrength::Example),
            _ => Err(crate::error::ConformanceError::InvalidBindingStrength(
                s.to_string(),
            )),
        }
    }
}

impl std::str::FromStr for BindingStrength {
    type Err = crate::error::ConformanceError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

/// Co-constraint for cross-field validation (Phase 2)
#[derive(Debug, Clone)]
pub struct CoConstraint {
    /// Constraint ID
    pub id: String,
    /// Description
    pub description: String,
    /// Condition expression
    pub condition: String,
}

impl CoConstraint {
    /// Create new co-constraint
    pub fn new(id: String, description: String, condition: String) -> Self {
        Self {
            id,
            description,
            condition,
        }
    }
}

/// Field profile with constraints
#[derive(Debug, Clone)]
pub struct FieldProfile {
    /// Field position (1-based)
    pub position: usize,
    /// Field name
    pub name: Option<String>,
    /// Usage code (R, RE, O, X, C)
    pub usage: ConditionalUsage,
    /// Cardinality (min/max occurrences for repeating fields)
    pub cardinality: Cardinality,
    /// Data type (e.g., "ST", "CX", "TS")
    pub datatype: Option<String>,
    /// Maximum length
    pub length: Option<usize>,
    /// HL7 table ID (e.g., "0001")
    pub table_id: Option<String>,
    /// Component profiles (for composite fields)
    pub components: Option<Vec<ComponentProfile>>,
    /// Value set binding
    pub value_set: Option<ValueSetBinding>,
    /// HL7 item number (e.g., "00108"); identifies the data item, not its position
    pub item_no: Option<String>,
}

impl FieldProfile {
    /// Create new field profile
    pub fn new(position: usize, usage: Usage, cardinality: Cardinality) -> Self {
        Self {
            position,
            name: None,
            usage: ConditionalUsage::from_usage(usage),
            cardinality,
            datatype: None,
            length: None,
            table_id: None,
            components: None,
            value_set: None,
            item_no: None,
        }
    }

    /// Create with conditional usage
    pub fn with_conditional_usage(
        position: usize,
        usage: ConditionalUsage,
        cardinality: Cardinality,
    ) -> Self {
        Self {
            position,
            name: None,
            usage,
            cardinality,
            datatype: None,
            length: None,
            table_id: None,
            components: None,
            value_set: None,
            item_no: None,
        }
    }

    /// Add component profiles
    pub fn with_components(mut self, components: Vec<ComponentProfile>) -> Self {
        self.components = Some(components);
        self
    }

    /// Add value set binding
    pub fn with_value_set(mut self, value_set: ValueSetBinding) -> Self {
        self.value_set = Some(value_set);
        self
    }
}

/// Usage codes (governs expected application behavior)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    /// R - Required: Must be present, error if missing
    Required,
    /// RE - Required if Known: Must send if known, no error if unknown
    RequiredIfKnown,
    /// O - Optional: May be present
    Optional,
    /// X - Not Used: Must not be present
    NotUsed,
}

impl Usage {
    /// Parse usage code from string
    ///
    /// Note: This method is kept for backward compatibility.
    /// Consider using the `FromStr` trait implementation instead.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "R" => Ok(Usage::Required),
            "RE" => Ok(Usage::RequiredIfKnown),
            "O" => Ok(Usage::Optional),
            "X" => Ok(Usage::NotUsed),
            _ => Err(crate::error::ConformanceError::InvalidUsage(s.to_string())),
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Usage::Required => "R",
            Usage::RequiredIfKnown => "RE",
            Usage::Optional => "O",
            Usage::NotUsed => "X",
        }
    }

    /// Check if the element is required (R or RE)
    pub fn is_required(&self) -> bool {
        matches!(self, Usage::Required | Usage::RequiredIfKnown)
    }

    /// Check if the element must not be present
    pub fn is_not_used(&self) -> bool {
        matches!(self, Usage::NotUsed)
    }
}

impl std::str::FromStr for Usage {
    type Err = crate::error::ConformanceError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_str(s)
    }
}

/// Cardinality specification (min/max occurrences)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cardinality {
    /// Minimum occurrences
    pub min: usize,
    /// Maximum occurrences (None = unbounded)
    pub max: Option<usize>,
}

impl Cardinality {
    /// Create new cardinality
    pub fn new(min: usize, max: Option<usize>) -> Result<Self> {
        if let Some(max_val) = max
            && min > max_val {
                return Err(crate::error::ConformanceError::InvalidCardinality(
                    format!("min ({}) > max ({})", min, max_val),
                ));
            }
        Ok(Self { min, max })
    }

    /// Create cardinality from [min..max] notation
    pub fn from_range(min: usize, max: Option<usize>) -> Result<Self> {
        Self::new(min, max)
    }

    /// Exactly one occurrence [1..1]
    pub fn one() -> Self {
        Self { min: 1, max: Some(1) }
    }

    /// Zero or one occurrence [0..1]
    pub fn zero_or_one() -> Self {
        Self { min: 0, max: Some(1) }
    }

    /// Zero or more occurrences [0..*]
    pub fn zero_or_more() -> Self {
        Self { min: 0, max: None }
    }

    /// One or more occurrences [1..*]
    pub fn one_or_more() -> Self {
        Self { min: 1, max: None }
    }

    /// Check if a count satisfies this cardinality
    pub fn satisfies(&self, count: usize) -> bool {
        if count < self.min {
            return false;
        }
        if let Some(max) = self.max
            && count > max {
                return false;
            }
        true
    }

}

impl std::fmt::Display for Cardinality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.max {
            Some(max) => write!(f, "[{}..{}]", self.min, max),
            None => write!(f, "[{}..*]", self.min),
        }
    }
}

impl Default for Cardinality {
    fn default() -> Self {
        Self::one()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_from_str() {
        assert_eq!(Usage::from_str("R").unwrap(), Usage::Required);
        assert_eq!(Usage::from_str("RE").unwrap(), Usage::RequiredIfKnown);
        assert_eq!(Usage::from_str("O").unwrap(), Usage::Optional);
        assert_eq!(Usage::from_str("X").unwrap(), Usage::NotUsed);
        assert_eq!(Usage::from_str("r").unwrap(), Usage::Required); // case insensitive
        assert!(Usage::from_str("INVALID").is_err());
    }

    #[test]
    fn test_usage_as_str() {
        assert_eq!(Usage::Required.as_str(), "R");
        assert_eq!(Usage::RequiredIfKnown.as_str(), "RE");
        assert_eq!(Usage::Optional.as_str(), "O");
        assert_eq!(Usage::NotUsed.as_str(), "X");
    }

    #[test]
    fn test_usage_is_required() {
        assert!(Usage::Required.is_required());
        assert!(Usage::RequiredIfKnown.is_required());
        assert!(!Usage::Optional.is_required());
        assert!(!Usage::NotUsed.is_required());
    }

    #[test]
    fn test_cardinality_creation() {
        let card = Cardinality::new(1, Some(1)).unwrap();
        assert_eq!(card.min, 1);
        assert_eq!(card.max, Some(1));

        let card = Cardinality::one();
        assert_eq!(card.min, 1);
        assert_eq!(card.max, Some(1));

        let card = Cardinality::zero_or_more();
        assert_eq!(card.min, 0);
        assert_eq!(card.max, None);
    }

    #[test]
    fn test_cardinality_satisfies() {
        let one = Cardinality::one();
        assert!(!one.satisfies(0));
        assert!(one.satisfies(1));
        assert!(!one.satisfies(2));

        let zero_or_more = Cardinality::zero_or_more();
        assert!(zero_or_more.satisfies(0));
        assert!(zero_or_more.satisfies(1));
        assert!(zero_or_more.satisfies(100));

        let range = Cardinality::new(2, Some(5)).unwrap();
        assert!(!range.satisfies(1));
        assert!(range.satisfies(2));
        assert!(range.satisfies(5));
        assert!(!range.satisfies(6));
    }

    #[test]
    fn test_cardinality_invalid() {
        let result = Cardinality::new(5, Some(3));
        assert!(result.is_err());
    }

    fn make_valid_profile() -> ConformanceProfile {
        let metadata = ProfileMetadata::new(
            "Test Profile".to_string(),
            "1.0".to_string(),
            Version::V2_5,
        );
        let mut message = MessageProfile::new("ADT".to_string(), "A01".to_string());
        let mut msh = SegmentProfile::new("MSH".to_string(), Usage::Required, Cardinality::one());
        msh.add_field(FieldProfile::new(1, Usage::Required, Cardinality::one()));
        message.add_segment(msh);
        ConformanceProfile::new(metadata, message)
    }

    #[test]
    fn test_validate_valid_profile() {
        let profile = make_valid_profile();
        assert!(profile.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let mut profile = make_valid_profile();
        profile.metadata.name = "".to_string();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_empty_version() {
        let mut profile = make_valid_profile();
        profile.metadata.version = "".to_string();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_empty_message_type() {
        let mut profile = make_valid_profile();
        profile.message.message_type = "".to_string();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_no_segments() {
        let mut profile = make_valid_profile();
        profile.message.segments.clear();
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_segment_id() {
        let mut profile = make_valid_profile();
        profile.message.segments[0].name = "AB".to_string(); // Too short
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_field_position_zero() {
        let mut profile = make_valid_profile();
        profile.message.segments[0].fields[0].position = 0;
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_field_zero_length() {
        let mut profile = make_valid_profile();
        profile.message.segments[0].fields[0].length = Some(0);
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_empty_predicate_condition() {
        let mut profile = make_valid_profile();
        profile.message.segments[0].fields[0].usage =
            ConditionalUsage::Conditional(Predicate::new(
                "".to_string(),
                Usage::Required,
                Usage::Optional,
            ));
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_empty_value_set_id() {
        let mut profile = make_valid_profile();
        profile.message.segments[0].fields[0].value_set = Some(ValueSetBinding::new(
            "".to_string(),
            BindingStrength::Required,
        ));
        assert!(profile.validate().is_err());
    }

    #[test]
    fn test_validate_empty_co_constraint_id() {
        let mut profile = make_valid_profile();
        profile.message.co_constraints = Some(vec![CoConstraint::new(
            "".to_string(),
            "desc".to_string(),
            "condition".to_string(),
        )]);
        assert!(profile.validate().is_err());
    }
}
