//! XML encoder for HL7 v2.x messages
//!
//! Encodes HL7 messages to XML format according to HL7 XML Encoding Rules.

use crate::error::XmlResult;
use crate::{HL7_V2_XML_NAMESPACE, XML_DECLARATION};
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use std::io::Cursor;
use vyna_hl7_core::field::{Component, Field, Repetition};
use vyna_hl7_core::message::Message;
use vyna_hl7_core::segment::Segment;

/// Configuration for XML encoding
#[derive(Debug, Clone)]
pub struct XmlEncoderConfig {
    /// Include XML declaration at the start
    pub include_declaration: bool,

    /// Include the HL7 v2 XML namespace
    pub include_namespace: bool,

    /// Pretty print with indentation
    pub pretty_print: bool,

    /// Indentation unit for pretty printing (default: `"  "`).
    ///
    /// The first byte is the indent character and the string length is
    /// the repeat count, so `"    "` indents four spaces per level and
    /// `"\t"` indents one tab.
    pub indent: String,

    /// Include empty fields as empty elements.
    ///
    /// Repetitions are always emitted (empty ones as `<SEG.F/>`):
    /// repetition position is order-based, so skipping one would shift
    /// every later repetition on decode.
    pub include_empty_fields: bool,

    /// Escape special characters (`<`, `>`, `&`, ...) in text content.
    ///
    /// When false, values are written verbatim and must already be
    /// valid XML content.
    pub escape_text: bool,
}

impl Default for XmlEncoderConfig {
    fn default() -> Self {
        Self {
            include_declaration: true,
            include_namespace: false,
            pretty_print: false,
            indent: "  ".to_string(),
            include_empty_fields: false,
            escape_text: true,
        }
    }
}

/// XML encoder for HL7 v2.x messages
pub struct XmlEncoder {
    config: XmlEncoderConfig,
}

impl XmlEncoder {
    /// Create a new XML encoder with default configuration
    pub fn new() -> Self {
        Self {
            config: XmlEncoderConfig::default(),
        }
    }

    /// Create an XML encoder with custom configuration
    pub fn with_config(config: XmlEncoderConfig) -> Self {
        Self { config }
    }

    /// Encode a message to XML
    pub fn encode(&self, message: &Message) -> XmlResult<String> {
        let mut result = String::new();

        // Write XML declaration
        if self.config.include_declaration {
            result.push_str(XML_DECLARATION);
            if self.config.pretty_print {
                result.push('\n');
            }
        }

        // Encode the message body
        let mut buffer = Vec::new();
        {
            let mut writer = if self.config.pretty_print {
                let (indent_char, indent_size) = Self::indent_params(&self.config.indent);
                Writer::new_with_indent(Cursor::new(&mut buffer), indent_char, indent_size)
            } else {
                Writer::new(Cursor::new(&mut buffer))
            };

            // Determine message type for root element
            let root_name = self.get_message_type_name(message);

            // Write root element
            let mut root = BytesStart::new(&root_name);
            if self.config.include_namespace {
                root.push_attribute(("xmlns", HL7_V2_XML_NAMESPACE));
            }
            writer.write_event(Event::Start(root))?;

            // Write segments
            for segment in &message.segments {
                self.write_segment(&mut writer, segment)?;
            }

            // Close root element
            writer.write_event(Event::End(BytesEnd::new(&root_name)))?;
        }

        // Append the XML body to result
        result.push_str(&String::from_utf8(buffer)?);

        Ok(result)
    }

    /// Derive quick-xml's (indent character, repeat count) from the
    /// configured indentation unit.
    fn indent_params(indent: &str) -> (u8, usize) {
        let char = indent.bytes().next().unwrap_or(b' ');
        (char, indent.len().max(1))
    }

    /// Build a text event, escaping unless configured otherwise.
    fn text_event<'a>(&self, value: &'a str) -> Event<'a> {
        if self.config.escape_text {
            Event::Text(BytesText::new(value))
        } else {
            Event::Text(BytesText::from_escaped(value))
        }
    }

    /// Get the message type name for the root element
    fn get_message_type_name(&self, message: &Message) -> String {
        // Try to extract from MSH-9. MSH-9 is composite, so the full
        // repetition value ("ADT^A01") is required: value() returns only
        // the first component ("ADT"), which would emit a root the
        // decoder reads back as a spurious segment.
        if let Some(msh) = message.segments.iter().find(|s| s.id == "MSH")
            && let Some(field9) = msh.get_field(9)
            && let Some(value) = field9.repetitions.first().and_then(|r| r.full_value())
        {
            // Parse ADT^A01 format
            let parts: Vec<&str> = value.split('^').collect();
            let candidate = if parts.len() >= 2 {
                let raw = format!("{}_{}", parts[0], parts[1]);
                sanitize_element_name(&raw)
            } else if !parts.is_empty() {
                sanitize_element_name(parts[0])
            } else {
                "HL7Message".to_string()
            };
            // A bare 3-uppercase-alnum root (a lone MSH-9 "ACK") is
            // indistinguishable from a segment element to the decoder
            // (see XmlDecoder::is_segment_name), which would read it back
            // as a spurious empty segment. Suffix it so the root stays a
            // wrapper and the message round-trips exactly.
            if candidate.len() == 3
                && candidate
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            {
                return format!("{candidate}_Message");
            }
            return candidate;
        }
        "HL7Message".to_string()
    }

    /// Write a segment to XML
    fn write_segment<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        segment: &Segment,
    ) -> XmlResult<()> {
        let segment_name = sanitize_element_name(&segment.id);
        writer.write_event(Event::Start(BytesStart::new(&segment_name)))?;

        // Write each field
        for (idx, field) in segment.fields.iter().enumerate() {
            let field_num = idx + 1;

            // Skip empty fields unless configured to include them
            if !self.config.include_empty_fields && field.is_empty() {
                continue;
            }

            self.write_field(writer, &segment_name, field_num, field)?;
        }

        writer.write_event(Event::End(BytesEnd::new(&segment_name)))?;
        Ok(())
    }

    /// Write a field to XML
    fn write_field<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        segment_id: &str,
        field_num: usize,
        field: &Field,
    ) -> XmlResult<()> {
        let field_name = format!("{}.{}", segment_id, field_num);

        // Every repetition gets its own element, even when empty:
        // repetition position is order-based (unlike numbered
        // fields/components), so skipping one would shift the rest.
        for repetition in field.repetitions.iter() {
            self.write_repetition(writer, &field_name, repetition)?;
        }

        Ok(())
    }

    /// Write a repetition to XML
    fn write_repetition<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        field_name: &str,
        repetition: &Repetition,
    ) -> XmlResult<()> {
        // Empty repetitions keep their position as empty elements.
        if repetition.is_empty() {
            writer.write_event(Event::Empty(BytesStart::new(field_name)))?;
            return Ok(());
        }

        // A lone single-valued component is written as simple text; a
        // lone component with subcomponents needs the wrapped form so
        // the subcomponents survive the round-trip.
        if repetition.components.len() == 1 && repetition.components[0].subcomponents.len() <= 1 {
            if let Some(value) = repetition.components[0].value()
                && !value.is_empty()
            {
                writer.write_event(Event::Start(BytesStart::new(field_name)))?;
                writer.write_event(self.text_event(value))?;
                writer.write_event(Event::End(BytesEnd::new(field_name)))?;
            }
        } else if !repetition.components.is_empty() {
            // Multiple components - wrap in field element with component children
            writer.write_event(Event::Start(BytesStart::new(field_name)))?;

            for (comp_idx, component) in repetition.components.iter().enumerate() {
                if component.is_empty() && !self.config.include_empty_fields {
                    continue;
                }

                self.write_component(writer, field_name, comp_idx + 1, component)?;
            }

            writer.write_event(Event::End(BytesEnd::new(field_name)))?;
        }

        Ok(())
    }

    /// Write a component to XML
    fn write_component<W: std::io::Write>(
        &self,
        writer: &mut Writer<W>,
        field_name: &str,
        comp_num: usize,
        component: &Component,
    ) -> XmlResult<()> {
        // Determine the component element name
        // For data types like XPN, XAD, use the data type prefix
        let comp_name = self.get_component_element_name(field_name, comp_num);

        if component.subcomponents.len() == 1 {
            // Simple component with value
            if let Some(value) = component.value()
                && !value.is_empty()
            {
                writer.write_event(Event::Start(BytesStart::new(&comp_name)))?;
                writer.write_event(self.text_event(value))?;
                writer.write_event(Event::End(BytesEnd::new(&comp_name)))?;
            }
        } else if component.subcomponents.len() > 1 {
            // Component with subcomponents
            writer.write_event(Event::Start(BytesStart::new(&comp_name)))?;

            for (sub_idx, subcomp) in component.subcomponents.iter().enumerate() {
                if subcomp.is_empty() && !self.config.include_empty_fields {
                    continue;
                }

                let sub_name = format!("{}.{}", comp_name, sub_idx + 1);
                writer.write_event(Event::Start(BytesStart::new(&sub_name)))?;
                writer.write_event(self.text_event(&subcomp.value))?;
                writer.write_event(Event::End(BytesEnd::new(&sub_name)))?;
            }

            writer.write_event(Event::End(BytesEnd::new(&comp_name)))?;
        }

        Ok(())
    }

    /// Get the element name for a component
    fn get_component_element_name(&self, field_name: &str, comp_num: usize) -> String {
        // Standard HL7 XML uses data type names for components
        // For now, use a generic format
        format!("{}.{}", field_name, comp_num)
    }
}

/// Replace characters illegal in XML names with `_`.
///
/// Element names derive from message content (MSH-9, segment IDs);
/// unsanitized values like `AD<T` would emit malformed XML.
fn sanitize_element_name(name: &str) -> String {
    if name.is_empty() {
        return "_".to_string();
    }
    let mut out = String::with_capacity(name.len() + 1);
    let mut chars = name.chars();
    let first = chars.next().unwrap_or('_');
    if first.is_ascii_alphabetic() || first == '_' {
        out.push(first);
    } else if first.is_ascii_alphanumeric() {
        out.push('_');
        out.push(first);
    } else {
        out.push('_');
    }
    for ch in chars {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

impl Default for XmlEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_config_default() {
        let config = XmlEncoderConfig::default();
        assert!(config.include_declaration);
        assert!(!config.include_namespace);
        assert!(!config.pretty_print);
    }

    #[test]
    fn test_encode_empty_message() {
        let message = Message::new();
        let encoder = XmlEncoder::new();
        let result = encoder.encode(&message);

        assert!(result.is_ok());
        let xml = result.unwrap();
        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<HL7Message"));
    }

    #[test]
    fn test_encode_with_segment() {
        let mut message = Message::new();
        let mut msh = Segment::new("MSH");
        let _ = msh.set_field_value(3, "APP");
        message.add_segment(msh);

        let encoder = XmlEncoder::new();
        let xml = encoder.encode(&message).unwrap();

        assert!(xml.contains("<MSH>"));
        assert!(xml.contains("APP"));
        assert!(xml.contains("</MSH>"));
    }

    #[test]
    fn test_indent_config_drives_pretty_printing() {
        let mut message = Message::new();
        let mut msh = Segment::new("MSH");
        let _ = msh.set_field_value(3, "APP");
        message.add_segment(msh);

        // Default: two spaces per level.
        let encoder = XmlEncoder::with_config(XmlEncoderConfig {
            pretty_print: true,
            ..Default::default()
        });
        let xml = encoder.encode(&message).unwrap();
        assert!(xml.contains("\n  <MSH>"));

        // Custom unit: four spaces per level.
        let encoder = XmlEncoder::with_config(XmlEncoderConfig {
            pretty_print: true,
            indent: "    ".to_string(),
            ..Default::default()
        });
        let xml = encoder.encode(&message).unwrap();
        assert!(xml.contains("\n    <MSH>"));
    }

    #[test]
    fn test_escape_text_config() {
        let mut message = Message::new();
        let mut pid = Segment::new("PID");
        let _ = pid.set_field_value(5, "A<B&C");
        message.add_segment(pid);

        let encoder = XmlEncoder::new();
        let xml = encoder.encode(&message).unwrap();
        assert!(xml.contains("A&lt;B&amp;C"));

        let encoder = XmlEncoder::with_config(XmlEncoderConfig {
            escape_text: false,
            ..Default::default()
        });
        let xml = encoder.encode(&message).unwrap();
        assert!(xml.contains("A<B&C"));
    }

    #[test]
    fn test_single_component_with_subcomponents() {
        use vyna_hl7_core::field::SubComponent;

        // A lone component carrying subcomponents must use the wrapped
        // form (4-part names), not collapse to its first value.
        let mut message = Message::new();
        let mut pid = Segment::new("PID");
        let mut field = Field::new();
        let mut repetition = Repetition::new();
        let mut component = Component::new();
        component.add_subcomponent(SubComponent::new("Smith"));
        component.add_subcomponent(SubComponent::new("Johnny"));
        repetition.add_component(component);
        field.add_repetition(repetition);
        let _ = pid.set_field(5, field);
        message.add_segment(pid);

        let encoder = XmlEncoder::new();
        let xml = encoder.encode(&message).unwrap();
        assert!(xml.contains("<PID.5.1.1>Smith</PID.5.1.1>"));
        assert!(xml.contains("<PID.5.1.2>Johnny</PID.5.1.2>"));
    }

    #[test]
    fn test_root_name_sanitizes_invalid_xml_chars() {
        let mut message = Message::new();
        let mut msh = Segment::new("MSH");
        let _ = msh.set_field_value(9, "AD<T^A01");
        message.add_segment(msh);

        let xml = XmlEncoder::new().encode(&message).unwrap();
        assert!(xml.contains("<AD_T_A01>"));
        assert!(xml.contains("</AD_T_A01>"));
        assert!(!xml.contains("<AD<T_A01>"));
    }

    #[test]
    fn test_segment_id_sanitizes_invalid_xml_chars() {
        let mut message = Message::new();
        message.add_segment(Segment::new("AD<T"));

        let xml = XmlEncoder::new().encode(&message).unwrap();
        assert!(xml.contains("<AD_T>"));
        assert!(!xml.contains("<AD<T>"));
    }

    #[test]
    fn test_bare_ack_root_round_trips_exactly() {
        use crate::XmlDecoder;

        let mut message = Message::new();
        let mut msh = Segment::new("MSH");
        let _ = msh.set_field_value(9, "ACK");
        message.add_segment(msh);
        let mut msa = Segment::new("MSA");
        let _ = msa.set_field_value(1, "AA");
        message.add_segment(msa);

        // A bare <ACK> root would decode as a spurious empty segment.
        let xml = XmlEncoder::new().encode(&message).unwrap();
        assert!(!xml.contains("<ACK>"), "bare root decodes as segment");
        assert!(xml.contains("<ACK_Message>"));

        let back = XmlDecoder::new().decode(&xml).unwrap();
        assert_eq!(back.segments.len(), 2);
        assert_eq!(back.segments[0].id, "MSH");
        assert_eq!(back.segments[1].id, "MSA");
        assert_eq!(back.segments[0].get_field_value(9), Some("ACK"));
        assert_eq!(back.segments[1].get_field_value(1), Some("AA"));
    }
}
