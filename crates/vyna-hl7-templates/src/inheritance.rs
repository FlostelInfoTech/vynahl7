//! Template inheritance support.

use crate::{Error, MessageTemplate, Result, SegmentTemplate};
use std::collections::HashMap;

/// Template resolver for handling inheritance
pub struct TemplateResolver {
    /// Template registry
    templates: HashMap<String, MessageTemplate>,
}

impl TemplateResolver {
    /// Create a new template resolver
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Register a template
    pub fn register(&mut self, template: MessageTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Register multiple templates
    pub fn register_all(&mut self, templates: Vec<MessageTemplate>) {
        for template in templates {
            self.register(template);
        }
    }

    /// Resolve a template with inheritance
    ///
    /// This method recursively resolves template inheritance, merging segments
    /// and fields from base templates.
    pub fn resolve(&self, name: &str) -> Result<MessageTemplate> {
        self.resolve_internal(name, &mut Vec::new())
    }

    /// Internal resolve method with cycle detection
    fn resolve_internal(
        &self,
        name: &str,
        visited: &mut Vec<String>,
    ) -> Result<MessageTemplate> {
        // Check for circular dependency
        if visited.contains(&name.to_string()) {
            return Err(Error::inheritance(format!(
                "Circular template inheritance detected: {} -> {}",
                visited.join(" -> "),
                name
            )));
        }

        // Get the template
        let template = self
            .templates
            .get(name)
            .ok_or_else(|| Error::not_found(format!("Template '{}' not found", name)))?;

        // If no base template, return as-is
        let Some(base_name) = &template.extends else {
            return Ok(template.clone());
        };

        // Track this template in the visit chain
        visited.push(name.to_string());

        // Recursively resolve the base template
        let base = self.resolve_internal(base_name, visited)?;

        // Merge the base with this template
        let merged = Self::merge_templates(&base, template)?;

        visited.pop();

        Ok(merged)
    }

    /// Merge two templates (child overrides base)
    fn merge_templates(base: &MessageTemplate, child: &MessageTemplate) -> Result<MessageTemplate> {
        let mut merged = base.clone();

        // Child's metadata overrides base (except name if different)
        if child.name != base.name {
            merged.name = child.name.clone();
        }
        if child.description.is_some() {
            merged.description = child.description.clone();
        }

        // Child's version, message_type, and trigger_event override base
        merged.version = child.version.clone();
        merged.message_type = child.message_type.clone();
        merged.trigger_event = child.trigger_event.clone();

        // Merge variables (child overrides base)
        if let Some(base_vars) = &base.variables {
            let mut vars = base_vars.clone();
            if let Some(child_vars) = &child.variables {
                vars.extend(child_vars.clone());
            }
            merged.variables = Some(vars);
        } else if child.variables.is_some() {
            merged.variables = child.variables.clone();
        }

        // Merge segments (child segments override base segments with same ID)
        merged.segments = Self::merge_segments(&base.segments, &child.segments);

        // Clear the extends reference in merged template
        merged.extends = None;

        Ok(merged)
    }

    /// Merge segment lists (child overrides base for matching IDs)
    fn merge_segments(
        base_segments: &[SegmentTemplate],
        child_segments: &[SegmentTemplate],
    ) -> Vec<SegmentTemplate> {
        let mut merged = base_segments.to_vec();

        for child_seg in child_segments {
            // Find if this segment already exists in base
            if let Some(pos) = merged.iter().position(|s| s.id == child_seg.id) {
                // Override the base segment
                merged[pos] = Self::merge_segment_templates(&merged[pos], child_seg);
            } else {
                // Add new segment
                merged.push(child_seg.clone());
            }
        }

        merged
    }

    /// Merge two segment templates (child overrides base)
    fn merge_segment_templates(
        base: &SegmentTemplate,
        child: &SegmentTemplate,
    ) -> SegmentTemplate {
        let mut merged = base.clone();

        // Child properties override
        merged.required = child.required;
        merged.repeating = child.repeating;

        if child.description.is_some() {
            merged.description = child.description.clone();
        }

        // Merge fields
        if let Some(child_fields) = &child.fields {
            let mut fields = base.fields.clone().unwrap_or_default();
            for (pos, field) in child_fields {
                fields.insert(*pos, field.clone());
            }
            merged.fields = Some(fields);
        }

        merged
    }
}

impl Default for TemplateResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FieldTemplate;

    #[test]
    fn test_simple_inheritance() {
        let mut resolver = TemplateResolver::new();

        // Base template
        let base = MessageTemplate::new("BaseADT", "2.5", "ADT", "A01")
            .with_segment(SegmentTemplate::new("MSH").required())
            .with_segment(SegmentTemplate::new("PID").required());

        // Child template that extends base
        let child = MessageTemplate::new("ExtendedADT", "2.5", "ADT", "A01")
            .with_extends("BaseADT")
            .with_segment(SegmentTemplate::new("PV1").required());

        resolver.register(base);
        resolver.register(child);

        let resolved = resolver.resolve("ExtendedADT").unwrap();

        // Should have all segments: MSH, PID (from base), PV1 (from child)
        assert_eq!(resolved.segments.len(), 3);
        assert!(resolved.segments.iter().any(|s| s.id == "MSH"));
        assert!(resolved.segments.iter().any(|s| s.id == "PID"));
        assert!(resolved.segments.iter().any(|s| s.id == "PV1"));
    }

    #[test]
    fn test_override_segment() {
        let mut resolver = TemplateResolver::new();

        // Base with PID segment
        let mut base = MessageTemplate::new("BaseADT", "2.5", "ADT", "A01");
        let mut pid_base = SegmentTemplate::new("PID");
        pid_base.add_field(3, FieldTemplate::new().with_placeholder("{{base_id}}"));
        base.add_segment(pid_base);

        // Child overrides PID segment
        let mut child = MessageTemplate::new("ExtendedADT", "2.5", "ADT", "A01")
            .with_extends("BaseADT");
        let mut pid_child = SegmentTemplate::new("PID").required();
        pid_child.add_field(3, FieldTemplate::new().required().with_placeholder("{{child_id}}"));
        child.add_segment(pid_child);

        resolver.register(base);
        resolver.register(child);

        let resolved = resolver.resolve("ExtendedADT").unwrap();

        // Should have PID from child (required, with child placeholder)
        let pid = resolved.segments.iter().find(|s| s.id == "PID").unwrap();
        assert!(pid.required);

        let field3 = pid.fields.as_ref().unwrap().get(&3).unwrap();
        assert_eq!(field3.placeholder, Some("{{child_id}}".to_string()));
    }

    #[test]
    fn test_merge_variables() {
        let mut resolver = TemplateResolver::new();

        // Base with some variables
        let mut base = MessageTemplate::new("BaseADT", "2.5", "ADT", "A01");
        base.add_variable("app", "BaseApp");
        base.add_variable("facility", "BaseFacility");

        // Child overrides one variable and adds another
        let mut child = MessageTemplate::new("ExtendedADT", "2.5", "ADT", "A01")
            .with_extends("BaseADT");
        child.add_variable("facility", "ChildFacility"); // Override
        child.add_variable("location", "ChildLocation"); // New

        resolver.register(base);
        resolver.register(child);

        let resolved = resolver.resolve("ExtendedADT").unwrap();

        let vars = resolved.variables.as_ref().unwrap();
        assert_eq!(vars.get("app"), Some(&"BaseApp".to_string())); // From base
        assert_eq!(vars.get("facility"), Some(&"ChildFacility".to_string())); // Overridden
        assert_eq!(vars.get("location"), Some(&"ChildLocation".to_string())); // New
    }

    #[test]
    fn test_multi_level_inheritance() {
        let mut resolver = TemplateResolver::new();

        // Grandparent
        let grandparent = MessageTemplate::new("GrandparentADT", "2.5", "ADT", "A01")
            .with_segment(SegmentTemplate::new("MSH").required());

        // Parent extends grandparent
        let parent = MessageTemplate::new("ParentADT", "2.5", "ADT", "A01")
            .with_extends("GrandparentADT")
            .with_segment(SegmentTemplate::new("PID").required());

        // Child extends parent
        let child = MessageTemplate::new("ChildADT", "2.5", "ADT", "A01")
            .with_extends("ParentADT")
            .with_segment(SegmentTemplate::new("PV1").required());

        resolver.register(grandparent);
        resolver.register(parent);
        resolver.register(child);

        let resolved = resolver.resolve("ChildADT").unwrap();

        // Should have all segments: MSH, PID, PV1
        assert_eq!(resolved.segments.len(), 3);
        assert!(resolved.segments.iter().any(|s| s.id == "MSH"));
        assert!(resolved.segments.iter().any(|s| s.id == "PID"));
        assert!(resolved.segments.iter().any(|s| s.id == "PV1"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut resolver = TemplateResolver::new();

        // Template A extends B
        let template_a = MessageTemplate::new("A", "2.5", "ADT", "A01")
            .with_extends("B");

        // Template B extends A (circular!)
        let template_b = MessageTemplate::new("B", "2.5", "ADT", "A01")
            .with_extends("A");

        resolver.register(template_a);
        resolver.register(template_b);

        let result = resolver.resolve("A");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Circular"));
    }

    #[test]
    fn test_missing_base_template() {
        let mut resolver = TemplateResolver::new();

        let child = MessageTemplate::new("Child", "2.5", "ADT", "A01")
            .with_extends("NonExistent");

        resolver.register(child);

        let result = resolver.resolve("Child");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_resolve_without_inheritance() {
        let mut resolver = TemplateResolver::new();

        let template = MessageTemplate::new("Standalone", "2.5", "ADT", "A01")
            .with_segment(SegmentTemplate::new("MSH").required());

        resolver.register(template.clone());

        let resolved = resolver.resolve("Standalone").unwrap();

        // Should be identical to original
        assert_eq!(resolved.name, template.name);
        assert_eq!(resolved.segments.len(), template.segments.len());
    }

    #[test]
    fn test_register_all() {
        let mut resolver = TemplateResolver::new();

        let templates = vec![
            MessageTemplate::new("T1", "2.5", "ADT", "A01"),
            MessageTemplate::new("T2", "2.5", "ADT", "A02"),
            MessageTemplate::new("T3", "2.5", "ADT", "A03"),
        ];

        resolver.register_all(templates);

        assert!(resolver.resolve("T1").is_ok());
        assert!(resolver.resolve("T2").is_ok());
        assert!(resolver.resolve("T3").is_ok());
    }
}
