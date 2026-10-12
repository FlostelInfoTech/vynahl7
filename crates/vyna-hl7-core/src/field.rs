//! HL7 field, component, and subcomponent structures

use crate::encoding::Encoding;
use crate::delimiters::Delimiters;
use crate::error::Result;

/// A subcomponent within a component
///
/// This is the smallest unit in HL7 message hierarchy.
/// Example: In "Smith&John", "Smith" and "John" are subcomponents
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubComponent {
    pub value: String,
}

impl SubComponent {
    /// Create a new subcomponent
    pub fn new<S: Into<String>>(value: S) -> Self {
        Self {
            value: value.into(),
        }
    }

    /// Check if subcomponent is empty
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Get the raw value
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Encode the subcomponent value
    pub fn encode(&self, delimiters: &Delimiters) -> String {
        Encoding::encode(&self.value, delimiters)
    }

    /// Decode from string
    pub fn decode(value: &str, delimiters: &Delimiters) -> Result<Self> {
        Ok(Self {
            value: Encoding::decode(value, delimiters)?,
        })
    }
}

impl From<String> for SubComponent {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for SubComponent {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// A component within a field
///
/// Components can contain subcomponents separated by the subcomponent separator (&).
/// Example: In "Smith&John^MD", "Smith&John" is a component with two subcomponents
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub subcomponents: Vec<SubComponent>,
}

impl Component {
    /// Create a new component
    pub fn new() -> Self {
        Self {
            subcomponents: Vec::new(),
        }
    }

    /// Create a component from a single value
    pub fn from_value<S: Into<String>>(value: S) -> Self {
        Self {
            subcomponents: vec![SubComponent::new(value)],
        }
    }

    /// Add a subcomponent
    pub fn add_subcomponent(&mut self, subcomponent: SubComponent) {
        self.subcomponents.push(subcomponent);
    }

    /// Get a subcomponent by index (0-based)
    pub fn get_subcomponent(&self, index: usize) -> Option<&SubComponent> {
        self.subcomponents.get(index)
    }

    /// Get a mutable subcomponent by index (0-based)
    pub fn get_subcomponent_mut(&mut self, index: usize) -> Option<&mut SubComponent> {
        self.subcomponents.get_mut(index)
    }

    /// Check if component is empty
    pub fn is_empty(&self) -> bool {
        self.subcomponents.is_empty() || self.subcomponents.iter().all(|s| s.is_empty())
    }

    /// Get the first subcomponent value (most common case)
    pub fn value(&self) -> Option<&str> {
        self.subcomponents.first().map(|s| s.as_str())
    }

    /// Encode the component
    pub fn encode(&self, delimiters: &Delimiters) -> String {
        let sep = delimiters.subcomponent_separator;
        let mut result = String::new();
        for (i, sc) in self.subcomponents.iter().enumerate() {
            if i > 0 {
                result.push(sep);
            }
            result.push_str(&sc.encode(delimiters));
        }
        result
    }
}

impl Default for Component {
    fn default() -> Self {
        Self::new()
    }
}

/// A repetition of a field
///
/// Fields can repeat, with each repetition containing components.
/// Example: In "Value1~Value2~Value3", there are 3 repetitions
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repetition {
    pub components: Vec<Component>,
}

impl Repetition {
    /// Create a new repetition
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Create from a single value
    pub fn from_value<S: Into<String>>(value: S) -> Self {
        Self {
            components: vec![Component::from_value(value)],
        }
    }

    /// Add a component
    pub fn add_component(&mut self, component: Component) {
        self.components.push(component);
    }

    /// Get a component by index (0-based)
    pub fn get_component(&self, index: usize) -> Option<&Component> {
        self.components.get(index)
    }

    /// Get a mutable component by index (0-based)
    pub fn get_component_mut(&mut self, index: usize) -> Option<&mut Component> {
        self.components.get_mut(index)
    }

    /// Check if repetition is empty
    pub fn is_empty(&self) -> bool {
        self.components.is_empty() || self.components.iter().all(|c| c.is_empty())
    }

    /// Get the first component's value (most common case)
    pub fn value(&self) -> Option<&str> {
        self.components.first().and_then(|c| c.value())
    }

    /// Get the full value of all components joined by the component separator (^)
    ///
    /// This returns the complete field value including all components.
    /// For example, for MSH-9 with "ADT^A01", this returns "ADT^A01" instead of just "ADT".
    pub fn full_value(&self) -> Option<String> {
        if self.components.is_empty() {
            return None;
        }

        // Get values from all components, using empty string for None
        let values: Vec<&str> = self
            .components
            .iter()
            .map(|c| c.value().unwrap_or(""))
            .collect();

        // Trim trailing empty components
        let last_non_empty = values.iter().rposition(|v| !v.is_empty());
        match last_non_empty {
            Some(idx) => Some(values[..=idx].join("^")),
            None => self.value().map(|s| s.to_string()),
        }
    }

    /// Encode the repetition
    pub fn encode(&self, delimiters: &Delimiters) -> String {
        let sep = delimiters.component_separator;
        let mut result = String::new();
        for (i, c) in self.components.iter().enumerate() {
            if i > 0 {
                result.push(sep);
            }
            result.push_str(&c.encode(delimiters));
        }
        result
    }
}

impl Default for Repetition {
    fn default() -> Self {
        Self::new()
    }
}

/// A field within a segment
///
/// Fields can contain multiple repetitions, each with components and subcomponents.
/// The hierarchy is: Field -> Repetitions -> Components -> Subcomponents
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub repetitions: Vec<Repetition>,
}

impl Field {
    /// Create a new empty field
    pub fn new() -> Self {
        Self {
            repetitions: Vec::new(),
        }
    }

    /// Create from a single value
    pub fn from_value<S: Into<String>>(value: S) -> Self {
        Self {
            repetitions: vec![Repetition::from_value(value)],
        }
    }

    /// Add a repetition
    pub fn add_repetition(&mut self, repetition: Repetition) {
        self.repetitions.push(repetition);
    }

    /// Get a repetition by index (0-based)
    pub fn get_repetition(&self, index: usize) -> Option<&Repetition> {
        self.repetitions.get(index)
    }

    /// Get a mutable repetition by index (0-based)
    pub fn get_repetition_mut(&mut self, index: usize) -> Option<&mut Repetition> {
        self.repetitions.get_mut(index)
    }

    /// Check if field is empty
    pub fn is_empty(&self) -> bool {
        self.repetitions.is_empty() || self.repetitions.iter().all(|r| r.is_empty())
    }

    /// Get the first repetition's value (most common case)
    pub fn value(&self) -> Option<&str> {
        self.repetitions.first().and_then(|r| r.value())
    }

    /// Encode the field
    pub fn encode(&self, delimiters: &Delimiters) -> String {
        let sep = delimiters.repetition_separator;
        let mut result = String::new();
        for (i, r) in self.repetitions.iter().enumerate() {
            if i > 0 {
                result.push(sep);
            }
            result.push_str(&r.encode(delimiters));
        }
        result
    }

    /// Get component at path (e.g., \[0\]\[2\] for first repetition, third component)
    pub fn get_component(&self, rep_index: usize, comp_index: usize) -> Option<&Component> {
        self.get_repetition(rep_index)
            .and_then(|r| r.get_component(comp_index))
    }

    /// Get subcomponent at path
    pub fn get_subcomponent(
        &self,
        rep_index: usize,
        comp_index: usize,
        sub_index: usize,
    ) -> Option<&SubComponent> {
        self.get_component(rep_index, comp_index)
            .and_then(|c| c.get_subcomponent(sub_index))
    }
}

impl Default for Field {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcomponent() {
        let sc = SubComponent::new("Test");
        assert_eq!(sc.as_str(), "Test");
        assert!(!sc.is_empty());

        let empty = SubComponent::new("");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_component() {
        let mut comp = Component::new();
        comp.add_subcomponent(SubComponent::new("First"));
        comp.add_subcomponent(SubComponent::new("Second"));

        assert_eq!(comp.subcomponents.len(), 2);
        assert_eq!(comp.value(), Some("First"));
    }

    #[test]
    fn test_repetition() {
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Component1"));
        rep.add_component(Component::from_value("Component2"));

        assert_eq!(rep.components.len(), 2);
        assert_eq!(rep.value(), Some("Component1"));
    }

    #[test]
    fn test_field() {
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("Rep1"));
        field.add_repetition(Repetition::from_value("Rep2"));

        assert_eq!(field.repetitions.len(), 2);
        assert_eq!(field.value(), Some("Rep1"));
    }

    #[test]
    fn test_field_hierarchy() {
        let mut field = Field::new();
        let mut rep = Repetition::new();
        let mut comp = Component::new();
        comp.add_subcomponent(SubComponent::new("SubValue"));
        rep.add_component(comp);
        field.add_repetition(rep);

        let subcomp = field.get_subcomponent(0, 0, 0);
        assert_eq!(subcomp.map(|s| s.as_str()), Some("SubValue"));
    }

    #[test]
    fn test_encoding() {
        let delims = Delimiters::default();

        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Test|Value"));
        field.add_repetition(rep);

        let encoded = field.encode(&delims);
        assert!(encoded.contains("\\F\\"));
    }
}
