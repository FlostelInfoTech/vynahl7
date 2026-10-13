//! XML decoder for HL7 v2.x messages
//!
//! Decodes HL7 XML format into vyna_hl7_core::Message structures.

use crate::error::{XmlError, XmlResult};
use quick_xml::Reader;
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use std::collections::BTreeMap;
use vyna_hl7_core::field::{Component, Field, Repetition, SubComponent};
use vyna_hl7_core::message::Message;
use vyna_hl7_core::segment::Segment;

/// Configuration for XML decoding
#[derive(Debug, Clone)]
pub struct XmlDecoderConfig {
    /// Tolerate non-standard or misplaced elements by skipping them.
    ///
    /// When false, unknown element names, field elements outside their
    /// own segment, component/subcomponent numbers that disagree with
    /// the enclosing element, truncated input, stray text, extra root
    /// elements, and mixed content are errors instead of being skipped.
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

/// An open element on the decoder stack.
enum Frame {
    /// Message wrapper (e.g. `ADT_A01`, `HL7Message`): carries no data.
    Root,
    /// Element skipped in lenient mode, along with its whole subtree.
    Ignored,
    /// A segment element; completed fields accumulate beside the stack.
    Segment { id: String },
    /// A field element; each one contributes exactly one repetition.
    Field {
        number: usize,
        components: BTreeMap<usize, Component>,
        text: String,
    },
    /// A component element.
    Component {
        number: usize,
        subcomponents: BTreeMap<usize, String>,
        text: String,
    },
    /// A subcomponent element.
    Subcomponent { number: usize, text: String },
}

/// Maximum field, component, or subcomponent number accepted in an
/// element name (e.g. `PID.5`, `PID.5.1`, `PID.5.1.2`).
///
/// Numbers drive allocation: fields extend the segment vec, and component
/// positions fill placeholder vecs. Real HL7 segments stay far below this;
/// anything beyond it is rejected before any allocation happens.
const MAX_ELEMENT_NUMBER: usize = 1000;

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
    ///
    /// Mirrors the shapes `XmlEncoder` emits: each `SEG.F` element
    /// contributes one field repetition, `SEG.F.C` (or `TYPE.C`) children
    /// become components, and `SEG.F.C.S` grandchildren become
    /// subcomponents. Numbered positions (including gaps) are preserved.
    pub fn decode(&self, xml: &str) -> XmlResult<Message> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(self.config.strip_whitespace);

        let mut message = Message::new();
        let mut current_segment: Option<Segment> = None;
        let mut fields: BTreeMap<usize, Field> = BTreeMap::new();
        let mut stack: Vec<Frame> = Vec::new();
        let mut seen_root = false;
        let mut depth = 0;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    self.check_depth(depth)?;
                    let name = e.name().as_ref().to_string();
                    let frame = self.classify_start(
                        &name,
                        depth,
                        &stack,
                        current_segment.as_ref(),
                        seen_root,
                    )?;
                    if matches!(frame, Frame::Root) {
                        seen_root = true;
                    }
                    Self::open_frame(
                        frame,
                        &mut stack,
                        &mut current_segment,
                        &mut fields,
                        &mut message,
                    );
                }
                Ok(Event::Empty(e)) => {
                    depth += 1;
                    self.check_depth(depth)?;
                    let name = e.name().as_ref().to_string();
                    let frame = self.classify_start(
                        &name,
                        depth,
                        &stack,
                        current_segment.as_ref(),
                        seen_root,
                    )?;
                    if matches!(frame, Frame::Root) {
                        seen_root = true;
                    }
                    // Self-closing: open, then immediately close.
                    Self::open_frame(
                        frame,
                        &mut stack,
                        &mut current_segment,
                        &mut fields,
                        &mut message,
                    );
                    if let Some(frame) = stack.pop() {
                        self.close_frame(
                            frame,
                            &mut stack,
                            &mut current_segment,
                            &mut fields,
                            &mut message,
                        )?;
                    }
                    depth -= 1;
                }
                Ok(Event::End(_)) => {
                    if let Some(frame) = stack.pop() {
                        self.close_frame(
                            frame,
                            &mut stack,
                            &mut current_segment,
                            &mut fields,
                            &mut message,
                        )?;
                    }
                    depth -= 1;
                }
                Ok(Event::Text(e)) => {
                    let raw = e.into_inner();
                    let text = unescape(&raw)?;
                    let trimmed = if self.config.strip_whitespace {
                        text.trim()
                    } else {
                        &text
                    };
                    if !trimmed.is_empty() {
                        self.push_text(&mut stack, trimmed)?;
                    }
                }
                Ok(Event::CData(e)) => {
                    let text = e.into_inner();
                    self.push_text(&mut stack, &text)?;
                }
                Ok(Event::GeneralRef(e)) => {
                    // Entity references arrive as their own events
                    // (e.g. `&amp;` between two Text events); resolve
                    // them so values like `A&amp;B` survive intact.
                    let name = e.into_inner();
                    let entity = format!("&{name};");
                    let resolved = unescape(&entity)?;
                    self.push_text(&mut stack, &resolved)?;
                }
                Ok(Event::Eof) => break,
                Ok(_) => {} // Ignore other events
                Err(e) => return Err(XmlError::QuickXml(e)),
            }
        }

        // Elements left open at EOF mean truncated input: strict
        // rejects it, lenient folds what arrived (historical behavior).
        if !stack.is_empty() {
            self.check_context("truncated XML: unclosed element(s) at end of input".to_string())?;
        }
        while let Some(frame) = stack.pop() {
            self.close_frame(
                frame,
                &mut stack,
                &mut current_segment,
                &mut fields,
                &mut message,
            )?;
        }
        Self::flush_segment(&mut current_segment, &mut fields, &mut message);

        Ok(message)
    }

    /// Reject or skip a non-conforming element depending on `lenient`.
    fn misplaced(&self, what: String) -> XmlResult<Frame> {
        if self.config.lenient {
            Ok(Frame::Ignored)
        } else {
            Err(XmlError::InvalidStructure(what))
        }
    }

    /// Decide what an opening element represents.
    fn classify_start(
        &self,
        name: &str,
        depth: usize,
        stack: &[Frame],
        current_segment: Option<&Segment>,
        seen_root: bool,
    ) -> XmlResult<Frame> {
        // Inside skipped content: skip the whole subtree.
        if matches!(stack.last(), Some(Frame::Ignored)) {
            return Ok(Frame::Ignored);
        }

        // A segment element starts a new segment.
        if self.is_segment_name(name) && depth <= 2 {
            return Ok(Frame::Segment {
                id: name.to_string(),
            });
        }

        // A field element contributes one repetition to its segment field.
        if let Some((seg_id, field_num)) = self.parse_field_name(name)
            && current_segment.is_some_and(|seg| seg.id == seg_id)
        {
            // A 2-part name for another segment (e.g. `XPN.1` used as a
            // datatype tag) falls through to component matching below.
            if !matches!(stack.last(), Some(Frame::Segment { .. })) || field_num == 0 {
                return self.misplaced(format!("field element <{name}> outside <{seg_id}>"));
            }
            Self::check_element_number(name, field_num)?;
            return Ok(Frame::Field {
                number: field_num,
                components: BTreeMap::new(),
                text: String::new(),
            });
        }

        // A component element: SEG.F.C or the datatype shorthand TYPE.C.
        if let Some(parent_field) = Self::parent_field_number(stack)
            && let Some((field_num, comp_num)) = self.parse_component_name(name, Some(parent_field))
        {
            if comp_num == 0 {
                return self.misplaced(format!("component element <{name}> has number 0"));
            }
            Self::check_element_number(name, comp_num)?;
            if field_num != parent_field
                && let Err(e) = self.check_context(format!(
                    "component element <{name}> nested in field {parent_field}"
                ))
            {
                return Err(e);
            }
            return Ok(Frame::Component {
                number: comp_num,
                subcomponents: BTreeMap::new(),
                text: String::new(),
            });
        }

        // A subcomponent element: SEG.F.C.S.
        if let Some((parent_field, parent_comp)) = Self::parent_component_numbers(stack)
            && let Some((field_num, comp_num, sub_num)) = self.parse_subcomponent_name(name)
        {
            if sub_num == 0 {
                return self.misplaced(format!("subcomponent element <{name}> has number 0"));
            }
            Self::check_element_number(name, sub_num)?;
            if (field_num != parent_field || comp_num != parent_comp)
                && let Err(e) = self.check_context(format!(
                    "subcomponent element <{name}> nested in field \
                     {parent_field} component {parent_comp}"
                ))
            {
                return Err(e);
            }
            return Ok(Frame::Subcomponent {
                number: sub_num,
                text: String::new(),
            });
        }

        // The depth-1 wrapper is the message root; anything deeper that
        // reached this point is unknown. Dotted names always denote
        // fields/components, never the root.
        if depth == 1 && stack.is_empty() && !name.contains('.') {
            if seen_root {
                // A second top-level wrapper: strict rejects it,
                // lenient keeps merging what follows (historical).
                self.check_context(format!("second root element <{name}>"))?;
            }
            return Ok(Frame::Root);
        }
        self.misplaced(format!("unknown element <{name}>"))
    }

    /// Reject a context mismatch in strict mode, tolerate it otherwise.
    fn check_context(&self, what: String) -> XmlResult<()> {
        if self.config.lenient {
            Ok(())
        } else {
            Err(XmlError::InvalidStructure(what))
        }
    }

    /// Field number of the enclosing field element, if any.
    fn parent_field_number(stack: &[Frame]) -> Option<usize> {
        match stack.last() {
            Some(Frame::Field { number, .. }) => Some(*number),
            _ => None,
        }
    }

    /// (field, component) numbers enclosing a subcomponent element.
    fn parent_component_numbers(stack: &[Frame]) -> Option<(usize, usize)> {
        let [
            ..,
            Frame::Field { number: field, .. },
            Frame::Component { number: comp, .. },
        ] = stack
        else {
            return None;
        };
        Some((*field, *comp))
    }

    /// Push a frame, starting a new segment when one opens.
    fn open_frame(
        frame: Frame,
        stack: &mut Vec<Frame>,
        current_segment: &mut Option<Segment>,
        fields: &mut BTreeMap<usize, Field>,
        message: &mut Message,
    ) {
        if let Frame::Segment { ref id } = frame {
            // Save any previous segment left open (malformed XML).
            Self::flush_segment(current_segment, fields, message);
            *current_segment = Some(Segment::new(id));
        }
        stack.push(frame);
    }

    /// Append text to the innermost element that carries a value.
    ///
    /// Text outside field/component/subcomponent elements is dropped;
    /// in strict mode that is an error unless the text is only
    /// whitespace (pretty-print formatting).
    fn push_text(&self, stack: &mut [Frame], text: &str) -> XmlResult<()> {
        match stack.last_mut() {
            Some(Frame::Field { text: buf, .. })
            | Some(Frame::Component { text: buf, .. })
            | Some(Frame::Subcomponent { text: buf, .. }) => {
                buf.push_str(text);
                Ok(())
            }
            _ if text.trim().is_empty() => Ok(()),
            _ => self.check_context(format!("text outside any value element: {text:?}")),
        }
    }

    /// Close a frame, folding its content into its parent.
    fn close_frame(
        &self,
        frame: Frame,
        stack: &mut [Frame],
        current_segment: &mut Option<Segment>,
        fields: &mut BTreeMap<usize, Field>,
        message: &mut Message,
    ) -> XmlResult<()> {
        match frame {
            Frame::Root | Frame::Ignored => {}
            Frame::Segment { .. } => {
                Self::flush_segment(current_segment, fields, message);
            }
            Frame::Field {
                number,
                components,
                text,
            } => {
                // Mixed content (text plus component children) drops
                // the text; strict rejects it, lenient keeps the
                // components (historical).
                if !components.is_empty() && !text.trim().is_empty() {
                    self.check_context(format!(
                        "field {number} mixes text with component elements"
                    ))?;
                }
                let repetition = Self::build_repetition(components, &text);
                if let Some(field) = fields.get_mut(&number) {
                    field.add_repetition(repetition);
                } else {
                    let mut field = Field::new();
                    field.add_repetition(repetition);
                    fields.insert(number, field);
                }
            }
            Frame::Component {
                number,
                subcomponents,
                text,
            } => {
                // Mixed content (text plus subcomponent children)
                // drops the text; strict rejects it, lenient keeps the
                // subcomponents (historical).
                if !subcomponents.is_empty() && !text.trim().is_empty() {
                    self.check_context(format!(
                        "component {number} mixes text with subcomponent elements"
                    ))?;
                }
                let component = Self::build_component(subcomponents, &text);
                match stack.last_mut() {
                    Some(Frame::Field { components, .. }) => {
                        components.insert(number, component);
                    }
                    _ => {
                        return Err(XmlError::InvalidStructure(format!(
                            "component {number} has no enclosing field"
                        )));
                    }
                }
            }
            Frame::Subcomponent { number, text } => match stack.last_mut() {
                Some(Frame::Component { subcomponents, .. }) => {
                    subcomponents.insert(number, text);
                }
                _ => {
                    return Err(XmlError::InvalidStructure(format!(
                        "subcomponent {number} has no enclosing component"
                    )));
                }
            },
        }
        Ok(())
    }

    /// Move the open segment and its completed fields into the message.
    fn flush_segment(
        current_segment: &mut Option<Segment>,
        fields: &mut BTreeMap<usize, Field>,
        message: &mut Message,
    ) {
        if let Some(mut segment) = current_segment.take() {
            for (number, field) in fields.iter() {
                let _ = segment.set_field(*number, field.clone());
            }
            fields.clear();
            message.add_segment(segment);
        }
    }

    /// Build one repetition: positioned components win, else simple text.
    fn build_repetition(components: BTreeMap<usize, Component>, text: &str) -> Repetition {
        if components.is_empty() {
            return Repetition::from_value(text);
        }
        let max = components.keys().max().copied().unwrap_or(0);
        let mut repetition = Repetition::new();
        for _ in 0..max {
            repetition.add_component(Component::new());
        }
        for (number, component) in components {
            if number > 0 && number <= max {
                repetition.components[number - 1] = component;
            }
        }
        repetition
    }

    /// Build one component: positioned subcomponents win, else simple text.
    fn build_component(subcomponents: BTreeMap<usize, String>, text: &str) -> Component {
        if subcomponents.is_empty() {
            return Component::from_value(text);
        }
        let max = subcomponents.keys().max().copied().unwrap_or(0);
        let mut component = Component::new();
        for _ in 0..max {
            component.add_subcomponent(SubComponent::new(""));
        }
        for (number, value) in subcomponents {
            if number > 0 && number <= max {
                component.subcomponents[number - 1] = SubComponent::new(value);
            }
        }
        component
    }

    /// Reject element numbers beyond [`MAX_ELEMENT_NUMBER`].
    ///
    /// This is a hard error in both modes: lenient decoding must still
    /// refuse allocations no real message needs.
    fn check_element_number(name: &str, number: usize) -> XmlResult<()> {
        if number > MAX_ELEMENT_NUMBER {
            return Err(XmlError::InvalidStructure(format!(
                "element <{name}> number {number} exceeds maximum of {MAX_ELEMENT_NUMBER}"
            )));
        }
        Ok(())
    }

    /// Reject input nested deeper than the configured maximum.
    fn check_depth(&self, depth: usize) -> XmlResult<()> {
        if depth > self.config.max_depth {
            return Err(XmlError::InvalidStructure(format!(
                "XML nesting depth exceeds maximum of {}",
                self.config.max_depth
            )));
        }
        Ok(())
    }

    /// Check if a name looks like a segment ID (3 uppercase letters)
    fn is_segment_name(&self, name: &str) -> bool {
        name.len() == 3 && name.chars().all(|c| c.is_ascii_uppercase())
    }

    /// Parse a field element name like "MSH.3" or "PID.5"
    fn parse_field_name(&self, name: &str) -> Option<(String, usize)> {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() == 2
            && self.is_segment_name(parts[0])
            && let Ok(field_num) = parts[1].parse::<usize>()
        {
            return Some((parts[0].to_string(), field_num));
        }
        None
    }

    /// Parse a component element name
    fn parse_component_name(&self, name: &str, field_num: Option<usize>) -> Option<(usize, usize)> {
        let parts: Vec<&str> = name.split('.').collect();

        // Format: SEG.FIELD.COMPONENT (e.g., PID.5.1)
        if parts.len() == 3
            && let (Ok(f), Ok(c)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>())
        {
            return Some((f, c));
        }

        // Format: DATATYPE.COMPONENT (e.g., XPN.1, MSG.1)
        if parts.len() == 2
            && let Ok(c) = parts[1].parse::<usize>()
            && let Some(fn_num) = field_num
        {
            return Some((fn_num, c));
        }

        None
    }

    /// Parse a subcomponent element name (SEG.FIELD.COMPONENT.SUBCOMPONENT)
    fn parse_subcomponent_name(&self, name: &str) -> Option<(usize, usize, usize)> {
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() == 4
            && let (Ok(f), Ok(c), Ok(s)) = (
                parts[1].parse::<usize>(),
                parts[2].parse::<usize>(),
                parts[3].parse::<usize>(),
            )
        {
            return Some((f, c, s));
        }
        None
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

    #[test]
    fn test_decode_repeated_field_elements() {
        let xml = r#"
            <PID>
                <PID.11>Addr1</PID.11>
                <PID.11>Addr2</PID.11>
            </PID>
        "#;
        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();
        let field = message.segments[0].get_field(11).unwrap();
        assert_eq!(field.repetitions.len(), 2);
        assert_eq!(field.repetitions[0].value(), Some("Addr1"));
        assert_eq!(field.repetitions[1].value(), Some("Addr2"));
    }

    #[test]
    fn test_decode_four_part_subcomponent_names() {
        let xml = r#"
            <PID>
                <PID.5>
                    <PID.5.1>
                        <PID.5.1.1>Smith</PID.5.1.1>
                        <PID.5.1.2>Johnny</PID.5.1.2>
                    </PID.5.1>
                    <PID.5.2>John</PID.5.2>
                </PID.5>
            </PID>
        "#;
        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();
        let field = message.segments[0].get_field(5).unwrap();
        let rep = &field.repetitions[0];
        assert_eq!(rep.components.len(), 2);
        assert_eq!(rep.components[0].subcomponents.len(), 2);
        assert_eq!(rep.components[0].subcomponents[0].as_str(), "Smith");
        assert_eq!(rep.components[0].subcomponents[1].as_str(), "Johnny");
        assert_eq!(rep.components[1].value(), Some("John"));
    }

    #[test]
    fn test_decode_sparse_positions_preserved() {
        // Gaps in the numbering decode to empty placeholders.
        let xml = r#"
            <PID>
                <PID.5>
                    <PID.5.1>A</PID.5.1>
                    <PID.5.3>C</PID.5.3>
                </PID.5>
            </PID>
        "#;
        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();
        let rep = &message.segments[0].get_field(5).unwrap().repetitions[0];
        assert_eq!(rep.components.len(), 3);
        assert_eq!(rep.components[0].value(), Some("A"));
        assert!(rep.components[1].is_empty());
        assert_eq!(rep.components[2].value(), Some("C"));
    }

    #[test]
    fn test_decode_empty_elements() {
        let xml = r#"<PID><PID.3/></PID>"#;
        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();
        let field = message.segments[0].get_field(3).unwrap();
        assert_eq!(field.repetitions.len(), 1);
        assert!(field.repetitions[0].is_empty());
    }

    #[test]
    fn test_lenient_skips_unknown_elements() {
        let xml = r#"
            <PID>
                <PID.1>1</PID.1>
                <Mystery>unknown</Mystery>
            </PID>
        "#;
        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();
        assert_eq!(message.segments[0].get_field_value(1), Some("1"));
    }

    #[test]
    fn test_strict_rejects_unknown_elements() {
        let xml = r#"<PID><PID.1>1</PID.1><Mystery>unknown</Mystery></PID>"#;
        let decoder = XmlDecoder::with_config(XmlDecoderConfig {
            lenient: false,
            ..Default::default()
        });
        let err = decoder.decode(xml).unwrap_err();
        assert!(matches!(err, XmlError::InvalidStructure(_)));
    }

    #[test]
    fn test_misplaced_field_element() {
        // MSH.3 nested in PID: skipped when lenient, an error when strict,
        // and never funneled into another field.
        let xml = r#"<PID><PID.1>1</PID.1><MSH.3>APP</MSH.3></PID>"#;
        let lenient = XmlDecoder::new();
        let message = lenient.decode(xml).unwrap();
        assert_eq!(message.segments[0].get_field_value(1), Some("1"));
        assert_eq!(message.segments[0].get_field(3), None);

        let strict = XmlDecoder::with_config(XmlDecoderConfig {
            lenient: false,
            ..Default::default()
        });
        assert!(strict.decode(xml).is_err());
    }

    #[test]
    fn test_max_depth_enforced() {
        let xml = r#"<PID><PID.5><PID.5.1>x</PID.5.1></PID.5></PID>"#;
        let decoder = XmlDecoder::with_config(XmlDecoderConfig {
            max_depth: 2,
            ..Default::default()
        });
        assert!(decoder.decode(xml).is_err());
    }

    fn strict_decoder() -> XmlDecoder {
        XmlDecoder::with_config(XmlDecoderConfig {
            lenient: false,
            ..Default::default()
        })
    }

    #[test]
    fn test_strict_rejects_truncated_xml() {
        // Unclosed elements at EOF: strict errors, lenient folds what
        // arrived (historical behavior).
        for xml in ["<PID><PID.1>1", "<PID><PID.1>1</PID.1>"] {
            let err = strict_decoder().decode(xml).unwrap_err();
            assert!(
                matches!(err, XmlError::InvalidStructure(_)),
                "expected InvalidStructure for {xml:?}, got {err:?}"
            );
            let message = XmlDecoder::new().decode(xml).unwrap();
            assert_eq!(message.segments[0].get_field_value(1), Some("1"));
        }
    }

    #[test]
    fn test_strict_rejects_stray_text() {
        // Non-whitespace text outside any value element would be
        // silently dropped: strict errors, lenient drops it.
        let xml = r#"<PID>junk<PID.1>1</PID.1></PID>"#;
        let err = strict_decoder().decode(xml).unwrap_err();
        assert!(matches!(err, XmlError::InvalidStructure(_)));

        let message = XmlDecoder::new().decode(xml).unwrap();
        assert_eq!(message.segments[0].get_field_value(1), Some("1"));

        // Whitespace-only text is formatting, accepted even in strict
        // mode (also with whitespace stripping disabled).
        let pretty = "<PID>\n  <PID.1>1</PID.1>\n</PID>";
        assert!(strict_decoder().decode(pretty).is_ok());
        let strict_no_strip = XmlDecoder::with_config(XmlDecoderConfig {
            lenient: false,
            strip_whitespace: false,
            ..Default::default()
        });
        assert!(strict_no_strip.decode(pretty).is_ok());
    }

    #[test]
    fn test_strict_rejects_second_root() {
        let xml = "<ROOT><PID><PID.1>1</PID.1></PID></ROOT><JUNK/>";
        let err = strict_decoder().decode(xml).unwrap_err();
        assert!(matches!(err, XmlError::InvalidStructure(_)));

        // Lenient keeps merging what follows the first root.
        let message = XmlDecoder::new().decode(xml).unwrap();
        assert_eq!(message.segments[0].get_field_value(1), Some("1"));
    }

    #[test]
    fn test_strict_rejects_mixed_content() {
        // Text alongside component children is dropped (the positioned
        // children win): strict errors, lenient keeps the children.
        let field_mixed = "<PID><PID.5>direct<XPN.1>Smith</XPN.1></PID.5></PID>";
        assert!(strict_decoder().decode(field_mixed).is_err());
        let message = XmlDecoder::new().decode(field_mixed).unwrap();
        let rep = &message.segments[0].get_field(5).unwrap().repetitions[0];
        assert_eq!(rep.components[0].value(), Some("Smith"));

        let comp_mixed = "<PID><PID.5><PID.5.1>direct\
                          <PID.5.1.1>x</PID.5.1.1></PID.5.1></PID.5></PID>";
        assert!(strict_decoder().decode(comp_mixed).is_err());
        assert!(XmlDecoder::new().decode(comp_mixed).is_ok());
    }

    #[test]
    fn test_rejects_huge_element_numbers() {
        // Element numbers drive allocation (field/component/subcomponent
        // vecs), so absurd numbers must error instead of allocating.
        let cases = [
            "<PID><PID.200000>x</PID.200000></PID>",
            "<PID><PID.5><PID.5.200000>x</PID.5.200000></PID.5></PID>",
            "<PID><PID.5><PID.5.1><PID.5.1.200000>x</PID.5.1.200000>\
             </PID.5.1></PID.5></PID>",
        ];
        for xml in cases {
            for decoder in [&XmlDecoder::new(), &strict_decoder()] {
                let err = decoder.decode(xml).unwrap_err();
                assert!(
                    matches!(err, XmlError::InvalidStructure(_)),
                    "expected InvalidStructure for {xml:?}, got {err:?}"
                );
            }
        }
    }

    #[test]
    fn test_decode_entity_references() {
        let xml = r#"<PID><PID.3>A&amp;B</PID.3></PID>"#;
        let decoder = XmlDecoder::new();
        let message = decoder.decode(xml).unwrap();
        assert_eq!(message.segments[0].get_field_value(3), Some("A&B"));

        // An unresolvable entity keeps its source chain.
        let bad = r#"<PID><PID.3>&bogus;</PID.3></PID>"#;
        let err = decoder.decode(bad).unwrap_err();
        assert!(matches!(err, XmlError::Unescape(_)));
        let source: &dyn std::error::Error = &err;
        assert!(source.source().is_some());
    }
}
