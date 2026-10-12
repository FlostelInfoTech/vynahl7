//! XML decoder for HL7 v2.x messages
//!
//! Decodes HL7 XML format into vyna_hl7_core::Message structures.

use crate::error::{XmlError, XmlResult};
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::Reader;
use vyna_hl7_core::field::{Component, Field, Repetition};
use vyna_hl7_core::message::Message;
use vyna_hl7_core::segment::Segment;
use std::collections::HashMap;

/// Configuration for XML decoding
#[derive(Debug, Clone)]
pub struct XmlDecoderConfig {
    /// Allow non-standard element names
    pub lenient: bool,

    /// Strip whitespace from text content
    pub strip_whitespace: bool,

    /// Maximum depth of XML nesting
    pub max_depth: usize,
}

impl Default for XmlDecoderConfig {
    fn default() -> Self {
        Self {
            lenient: true,
            strip_whitespace: true,
            max_depth: 10,
        }
    }
}

/// XML decoder for HL7 v2.x messages
pub struct XmlDecoder {
    config: XmlDecoderConfig,
}

impl XmlDecoder {
    /// Create a new XML decoder with default configuration
    pub fn new() -> Self {
        Self {
            config: XmlDecoderConfig::default(),
        }
    }

    /// Create an XML decoder with custom configuration
    pub fn with_config(config: XmlDecoderConfig) -> Self {
        Self { config }
    }

    /// Decode XML into a Message
    pub fn decode(&self, xml: &str) -> XmlResult<Message> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(self.config.strip_whitespace);

        let mut message = Message::new();
        let mut current_segment: Option<Segment> = None;
        let mut current_field_num: Option<usize> = None;
        let mut current_component_num: Option<usize> = None;
        let mut _current_subcomponent_num: Option<usize> = None;
        let mut element_stack: Vec<String> = Vec::new();
        let mut pending_fields: HashMap<usize, Field> = HashMap::new();
        let mut pending_components: HashMap<(usize, usize), String> = HashMap::new();
        let mut text_buffer = String::new();
        let mut depth = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    if depth > self.config.max_depth {
                        return Err(XmlError::InvalidStructure(format!(
                            "XML nesting depth exceeds maximum of {}",
                            self.config.max_depth
                        )));
                    }

                    let name = e.name().as_ref().to_string();
                    element_stack.push(name.clone());

                    // Check if this is a segment (3 uppercase letters)
                    if self.is_segment_name(&name) && depth <= 2 {
                        // Save previous segment if any
                        if let Some(mut seg) = current_segment.take() {
                            self.apply_pending_fields(&mut seg, &pending_fields);
                            message.add_segment(seg);
                            pending_fields.clear();
                            pending_components.clear();
                        }
                        current_segment = Some(Segment::new(&name));
                        current_field_num = None;
                        current_component_num = None;
                    } else if let Some((seg_id, field_num)) = self.parse_field_name(&name) {
                        // This is a field element like MSH.3 or PID.5
                        if let Some(ref seg) = current_segment
                            && seg.id == seg_id {
                                current_field_num = Some(field_num);
                                current_component_num = None;
                            }
                    } else if let Some((_, comp_num)) =
                        self.parse_component_name(&name, current_field_num)
                    {
                        // This is a component element
                        current_component_num = Some(comp_num);
                        _current_subcomponent_num = None;
                    } else if let Some(sub_num) =
                        self.parse_subcomponent_name(&name, current_component_num)
                    {
                        // This is a subcomponent element
                        _current_subcomponent_num = Some(sub_num);
                    }

                    text_buffer.clear();
                }
                Ok(Event::End(e)) => {
                    let name = e.name().as_ref().to_string();

                    // Process text content if any
                    if !text_buffer.is_empty()
                        && let Some(field_num) = current_field_num {
                            if let Some(comp_num) = current_component_num {
                                // Store component value
                                pending_components
                                    .insert((field_num, comp_num), text_buffer.clone());
                            } else {
                                // Store simple field value
                                let field = Field::from_value(&text_buffer);
                                pending_fields.insert(field_num, field);
                            }
                        }

                    // Handle segment closure
                    if self.is_segment_name(&name) {
                        if let Some(mut seg) = current_segment.take() {
                            self.apply_pending_fields(&mut seg, &pending_fields);
                            self.apply_pending_components(&mut seg, &pending_components);
                            message.add_segment(seg);
                            pending_fields.clear();
                            pending_components.clear();
                        }
                    } else if self.parse_field_name(&name).is_some() {
                        // Field closed - apply any pending components to this field
                        if let Some(field_num) = current_field_num
                            && !pending_components.is_empty() {
                                // Build field from components
                                let field =
                                    self.build_field_from_components(&pending_components, field_num);
                                pending_fields.insert(field_num, field);
                                // Clear only components for this field
                                pending_components
                                    .retain(|(f, _), _| *f != field_num);
                            }
                        current_field_num = None;
                        current_component_num = None;
                    } else if self.parse_component_name(&name, current_field_num).is_some() {
                        current_component_num = None;
                    }

                    element_stack.pop();
                    text_buffer.clear();
                    depth -= 1;
                }
                Ok(Event::Text(e)) => {
                    let raw = e.into_inner();
                    let text = unescape(&raw).map_err(|e| XmlError::XmlParse(e.to_string()))?;
                    let trimmed = if self.config.strip_whitespace {
                        text.trim()
                    } else {
                        &text
                    };
                    if !trimmed.is_empty() {
                        text_buffer.push_str(trimmed);
                    }
                }
                Ok(Event::CData(e)) => {
                    let text = e.into_inner();
                    text_buffer.push_str(&text);
                }
                Ok(Event::Eof) => break,
                Ok(_) => {} // Ignore other events
                Err(e) => return Err(XmlError::XmlParse(e.to_string())),
            }
        }

        // Handle any remaining segment
        if let Some(mut seg) = current_segment.take() {
            self.apply_pending_fields(&mut seg, &pending_fields);
            self.apply_pending_components(&mut seg, &pending_components);
            message.add_segment(seg);
        }

        Ok(message)
    }

    /// Check if a name looks like a segment ID (3 uppercase letters)
    fn is_segment_name(&self, name: &str) -> bool {
        name.len() == 3 && name.chars().all(|c| c.is_ascii_uppercase())
    }

    /// Parse a field element name like "MSH.3" or "PID.5"
    fn parse_field_name(&self, name: &str) -> Option<(String, usize)> {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() == 2 && self.is_segment_name(parts[0])
            && let Ok(field_num) = parts[1].parse::<usize>() {
                return Some((parts[0].to_string(), field_num));
            }
        None
    }

    /// Parse a component element name
    fn parse_component_name(&self, name: &str, field_num: Option<usize>) -> Option<(usize, usize)> {
        let parts: Vec<&str> = name.split('.').collect();

        // Format: SEG.FIELD.COMPONENT (e.g., PID.5.1)
        if parts.len() == 3
            && let (Ok(f), Ok(c)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>()) {
                return Some((f, c));
            }

        // Format: DATATYPE.COMPONENT (e.g., XPN.1, MSG.1)
        if parts.len() == 2
            && let Ok(c) = parts[1].parse::<usize>()
                && let Some(fn_num) = field_num {
                    return Some((fn_num, c));
                }

        None
    }

    /// Parse a subcomponent element name
    fn parse_subcomponent_name(&self, name: &str, _comp_num: Option<usize>) -> Option<usize> {
        let parts: Vec<&str> = name.split('.').collect();
        // Subcomponents would be SEG.FIELD.COMPONENT.SUBCOMPONENT
        if parts.len() == 4
            && let Ok(s) = parts[3].parse::<usize>() {
                return Some(s);
            }
        None
    }

    /// Apply pending field values to a segment
    fn apply_pending_fields(&self, segment: &mut Segment, fields: &HashMap<usize, Field>) {
        for (&field_num, field) in fields {
            let _ = segment.set_field(field_num, field.clone());
        }
    }

    /// Apply pending component values to a segment
    fn apply_pending_components(
        &self,
        segment: &mut Segment,
        components: &HashMap<(usize, usize), String>,
    ) {
        // Group by field number
        let mut fields: HashMap<usize, Vec<(usize, String)>> = HashMap::new();
        for (&(field_num, comp_num), value) in components {
            fields
                .entry(field_num)
                .or_default()
                .push((comp_num, value.clone()));
        }

        // Apply each field's components
        for (field_num, comps) in fields {
            let field = self.build_field_from_component_list(&comps);
            let _ = segment.set_field(field_num, field);
        }
    }

    /// Build a field from a list of components
    fn build_field_from_component_list(&self, components: &[(usize, String)]) -> Field {
        let max_comp = components.iter().map(|(c, _)| *c).max().unwrap_or(0);

        let mut repetition = Repetition::new();

        // Pre-fill with empty components
        for _ in 0..max_comp {
            repetition.add_component(Component::new());
        }

        // Set component values
        for (comp_num, value) in components {
            if *comp_num > 0 && *comp_num <= repetition.components.len() {
                repetition.components[*comp_num - 1] = Component::from_value(value);
            }
        }

        let mut field = Field::new();
        field.add_repetition(repetition);
        field
    }

    /// Build a field from pending components
    fn build_field_from_components(
        &self,
        components: &HashMap<(usize, usize), String>,
        field_num: usize,
    ) -> Field {
        let field_comps: Vec<(usize, String)> = components
            .iter()
            .filter(|((f, _), _)| *f == field_num)
            .map(|((_, c), v)| (*c, v.clone()))
            .collect();

        self.build_field_from_component_list(&field_comps)
    }
}

impl Default for XmlDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_config_default() {
        let config = XmlDecoderConfig::default();
        assert!(config.lenient);
        assert!(config.strip_whitespace);
        assert_eq!(config.max_depth, 10);
    }

    #[test]
    fn test_is_segment_name() {
        let decoder = XmlDecoder::new();
        assert!(decoder.is_segment_name("MSH"));
        assert!(decoder.is_segment_name("PID"));
        assert!(decoder.is_segment_name("OBX"));
        assert!(!decoder.is_segment_name("msh"));
        assert!(!decoder.is_segment_name("MSH.3"));
        assert!(!decoder.is_segment_name("AB"));
        assert!(!decoder.is_segment_name("ABCD"));
    }

    #[test]
    fn test_parse_field_name() {
        let decoder = XmlDecoder::new();
        assert_eq!(
            decoder.parse_field_name("MSH.3"),
            Some(("MSH".to_string(), 3))
        );
        assert_eq!(
            decoder.parse_field_name("PID.5"),
            Some(("PID".to_string(), 5))
        );
        assert_eq!(decoder.parse_field_name("MSH"), None);
        assert_eq!(decoder.parse_field_name("invalid"), None);
    }

    #[test]
    fn test_decode_minimal_xml() {
        let xml = r#"<MSH><MSH.3>APP</MSH.3></MSH>"#;
        let decoder = XmlDecoder::new();
        let result = decoder.decode(xml);

        assert!(result.is_ok());
        let message = result.unwrap();
        assert_eq!(message.segments.len(), 1);
        assert_eq!(message.segments[0].id, "MSH");
    }

    #[test]
    fn test_decode_multiple_segments() {
        let xml = r#"
            <ROOT>
                <MSH><MSH.3>APP</MSH.3></MSH>
                <PID><PID.1>1</PID.1><PID.3>12345</PID.3></PID>
            </ROOT>
        "#;

        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();

        assert_eq!(message.segments.len(), 2);
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "PID");
    }

    #[test]
    fn test_decode_with_components() {
        let xml = r#"
            <PID>
                <PID.5>
                    <XPN.1>Smith</XPN.1>
                    <XPN.2>John</XPN.2>
                </PID.5>
            </PID>
        "#;

        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();

        assert_eq!(message.segments.len(), 1);
        let pid = &message.segments[0];
        assert_eq!(pid.id, "PID");

        // Check field 5 has components
        let field5 = pid.get_field(5);
        assert!(field5.is_some());
    }
}
