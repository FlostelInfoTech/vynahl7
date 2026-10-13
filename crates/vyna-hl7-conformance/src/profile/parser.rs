//! XML conformance profile parser

use crate::error::{ConformanceError, Result};
use crate::profile::{
    Cardinality, ConformanceProfile, FieldProfile, MessageProfile, ProfileMetadata, SegmentProfile,
    Usage,
};
use quick_xml::events::Event;
use quick_xml::Reader;
use vyna_hl7_core::Version;
use std::io::BufRead;
use std::path::Path;

/// Parser for HL7 conformance profile XML files
pub struct ProfileParser;

impl ProfileParser {
    /// Parse a conformance profile from an XML file
    pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ConformanceProfile> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_xml(&content)
    }

    /// Parse a conformance profile from XML string
    pub fn parse_xml(xml: &str) -> Result<ConformanceProfile> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut hl7_version = None;
        let mut metadata: Option<ProfileMetadata> = None;
        let mut message: Option<MessageProfile> = None;
        let mut msg_type = None;
        let mut event_type = None;

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) if e.name().as_ref() == "Segment" => {
                    // A self-closing Segment has no child Fields: build it
                    // from attributes alone so it cannot consume siblings.
                    if message.is_none() {
                        message = Some(MessageProfile::new(
                            msg_type.clone().unwrap_or_default(),
                            event_type.clone().unwrap_or_default(),
                        ));
                    }
                    let segment = Self::parse_empty_segment_element(e)?;
                    if let Some(ref mut msg) = message {
                        msg.add_segment(segment);
                    }
                }
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    match e.name().as_ref() {
                        "HL7v2xConformanceProfile" => {
                            // Parse root element attributes
                            for attr in e.attributes() {
                                let attr = attr.map_err(|e| {
                                    ConformanceError::ParseError(format!("Invalid attribute: {}", e))
                                })?;
                                if attr.key.as_ref() == "HL7Version" {
                                    let version_str = attr.value;
                                    hl7_version = Some(Self::parse_version(&version_str)?);
                                }
                            }
                        }
                        "HL7v2xStaticDef" => {
                            // Parse static definition attributes
                            for attr in e.attributes() {
                                let attr = attr.map_err(|e| {
                                    ConformanceError::ParseError(format!("Invalid attribute: {}", e))
                                })?;
                                match attr.key.as_ref() {
                                    "MsgType" => {
                                        msg_type = Some(attr.value.to_string());
                                    }
                                    "EventType" => {
                                        event_type = Some(attr.value.to_string());
                                    }
                                    _ => {}
                                }
                            }
                        }
                        "MetaData" => {
                            metadata = Some(Self::parse_metadata_element(&reader, e, hl7_version)?);
                        }
                        "Segment" => {
                            if message.is_none() {
                                message = Some(MessageProfile::new(
                                    msg_type.clone().unwrap_or_default(),
                                    event_type.clone().unwrap_or_default(),
                                ));
                            }
                            let segment = Self::parse_segment_element(&mut reader, e)?;
                            if let Some(ref mut msg) = message {
                                msg.add_segment(segment);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ConformanceError::XmlError(e)),
                _ => {}
            }
            buf.clear();
        }

        let metadata = metadata.ok_or_else(|| {
            ConformanceError::MissingElement("MetaData element not found".to_string())
        })?;
        let message = message.ok_or_else(|| {
            ConformanceError::MissingElement("No segments found in profile".to_string())
        })?;

        Ok(ConformanceProfile::new(metadata, message))
    }

    /// Parse version string to Version enum
    fn parse_version(version_str: &str) -> Result<Version> {
        match version_str {
            "2.1" => Ok(Version::V2_1),
            "2.2" => Ok(Version::V2_2),
            "2.3" => Ok(Version::V2_3),
            "2.3.1" => Ok(Version::V2_3_1),
            "2.4" => Ok(Version::V2_4),
            "2.5" => Ok(Version::V2_5),
            "2.5.1" => Ok(Version::V2_5_1),
            "2.6" => Ok(Version::V2_6),
            "2.7" => Ok(Version::V2_7),
            "2.7.1" => Ok(Version::V2_7_1),
            "2.8" => Ok(Version::V2_8),
            "2.8.1" => Ok(Version::V2_8_1),
            "2.8.2" => Ok(Version::V2_8_2),
            _ => Err(ConformanceError::ParseError(format!(
                "Unsupported HL7 version: {} (supported: 2.1 through 2.8.2)",
                version_str
            ))),
        }
    }

    /// Parse MetaData element
    fn parse_metadata_element<R: BufRead>(
        _reader: &Reader<R>,
        element: &quick_xml::events::BytesStart,
        hl7_version: Option<Version>,
    ) -> Result<ProfileMetadata> {
        let mut name = None;
        let mut org_name = None;
        let mut version = None;

        for attr in element.attributes() {
            let attr = attr.map_err(|e| {
                ConformanceError::ParseError(format!("Invalid attribute: {}", e))
            })?;
            match attr.key.as_ref() {
                "Name" => name = Some(attr.value.to_string()),
                "OrgName" => org_name = Some(attr.value.to_string()),
                "Version" => version = Some(attr.value.to_string()),
                _ => {}
            }
        }

        let name = name.ok_or_else(|| {
            ConformanceError::MissingElement("MetaData Name attribute required".to_string())
        })?;
        let version = version.ok_or_else(|| {
            ConformanceError::MissingElement("MetaData Version attribute required".to_string())
        })?;
        let hl7_version = hl7_version.ok_or_else(|| {
            ConformanceError::MissingElement("HL7Version not specified".to_string())
        })?;

        let mut metadata = ProfileMetadata::new(name, version, hl7_version);
        metadata.organization = org_name;

        Ok(metadata)
    }

    /// Parse a self-closing Segment element (attributes only, no Fields)
    fn parse_empty_segment_element(
        element: &quick_xml::events::BytesStart,
    ) -> Result<SegmentProfile> {
        let mut name = None;
        let mut long_name = None;
        let mut usage = None;
        let mut min = None;
        let mut max = None;

        // Parse attributes
        for attr in element.attributes() {
            let attr = attr.map_err(|e| {
                ConformanceError::ParseError(format!("Invalid attribute: {}", e))
            })?;
            match attr.key.as_ref() {
                "Name" => name = Some(attr.value.to_string()),
                "LongName" => long_name = Some(attr.value.to_string()),
                "Usage" => usage = Some(attr.value.to_string()),
                "Min" => min = Some(attr.value.to_string()),
                "Max" => max = Some(attr.value.to_string()),
                _ => {}
            }
        }

        let name = name.ok_or_else(|| {
            ConformanceError::MissingElement("Segment Name attribute required".to_string())
        })?;
        let usage_str = usage.ok_or_else(|| {
            ConformanceError::MissingElement("Segment Usage attribute required".to_string())
        })?;
        let min_str = min.ok_or_else(|| {
            ConformanceError::MissingElement("Segment Min attribute required".to_string())
        })?;
        let max_str = max.ok_or_else(|| {
            ConformanceError::MissingElement("Segment Max attribute required".to_string())
        })?;

        let usage = Usage::from_str(&usage_str)?;
        let cardinality = Self::parse_cardinality_from_min_max(&min_str, &max_str)?;

        let mut segment = SegmentProfile::new(name, usage, cardinality);
        segment.long_name = long_name;

        Ok(segment)
    }

    /// Parse Segment element
    fn parse_segment_element<R: BufRead>(
        reader: &mut Reader<R>,
        element: &quick_xml::events::BytesStart,
    ) -> Result<SegmentProfile> {
        let mut segment = Self::parse_empty_segment_element(element)?;

        // Parse child Field elements; position is the 1-based ordinal of the
        // Field element within its segment, not the HL7 ItemNo (which is a
        // global data-item identifier, e.g. PID-5 is item 00108).
        let mut buf = Vec::new();
        let mut depth = 1;
        let mut position = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    if e.name().as_ref() == "Field" {
                        position += 1;
                        let field = Self::parse_field_element(e, position)?;
                        segment.add_field(field);
                    }
                    depth += 1;
                }
                Ok(Event::Empty(ref e))
                    // Handle self-closing tags like <Field ... />
                    if e.name().as_ref() == "Field" => {
                        position += 1;
                        let field = Self::parse_field_element(e, position)?;
                        segment.add_field(field);
                    }
                Ok(Event::End(ref e)) => {
                    depth -= 1;
                    if depth == 0 && e.name().as_ref() == "Segment" {
                        break;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ConformanceError::XmlError(e)),
                _ => {}
            }
            buf.clear();
        }

        Ok(segment)
    }

    /// Parse Field element
    fn parse_field_element(
        element: &quick_xml::events::BytesStart,
        position: usize,
    ) -> Result<FieldProfile> {
        let mut name = None;
        let mut usage = None;
        let mut min = None;
        let mut max = None;
        let mut datatype = None;
        let mut length = None;
        let mut item_no = None;

        for attr in element.attributes() {
            let attr = attr.map_err(|e| {
                ConformanceError::ParseError(format!("Invalid attribute: {}", e))
            })?;
            match attr.key.as_ref() {
                "Name" => name = Some(attr.value.to_string()),
                "Usage" => usage = Some(attr.value.to_string()),
                "Min" => min = Some(attr.value.to_string()),
                "Max" => max = Some(attr.value.to_string()),
                "Datatype" => datatype = Some(attr.value.to_string()),
                "Length" => length = Some(attr.value.to_string()),
                "ItemNo" => item_no = Some(attr.value.to_string()),
                _ => {}
            }
        }

        let usage_str = usage.ok_or_else(|| {
            ConformanceError::MissingElement("Field Usage attribute required".to_string())
        })?;
        let min_str = min.ok_or_else(|| {
            ConformanceError::MissingElement("Field Min attribute required".to_string())
        })?;
        let max_str = max.ok_or_else(|| {
            ConformanceError::MissingElement("Field Max attribute required".to_string())
        })?;

        let usage = Usage::from_str(&usage_str)?;
        let cardinality = Self::parse_cardinality_from_min_max(&min_str, &max_str)?;

        let mut field = FieldProfile::new(position, usage, cardinality);
        field.name = name;
        field.datatype = datatype;
        field.length = length.and_then(|l| l.parse::<usize>().ok());
        field.item_no = item_no;

        Ok(field)
    }

    /// Parse cardinality from Min and Max attributes
    fn parse_cardinality_from_min_max(min_str: &str, max_str: &str) -> Result<Cardinality> {
        let min = min_str.parse::<usize>().map_err(|_| {
            ConformanceError::InvalidCardinality(format!("Invalid min value: {}", min_str))
        })?;

        let max = if max_str == "*" {
            None
        } else {
            Some(max_str.parse::<usize>().map_err(|_| {
                ConformanceError::InvalidCardinality(format!("Invalid max value: {}", max_str))
            })?)
        };

        Cardinality::new(min, max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConditionalUsage;

    #[test]
    fn test_parse_version() {
        assert_eq!(
            ProfileParser::parse_version("2.5").unwrap(),
            Version::V2_5
        );
        assert_eq!(
            ProfileParser::parse_version("2.3").unwrap(),
            Version::V2_3
        );
        assert!(ProfileParser::parse_version("9.9").is_err());
        // v2.1 and v2.8 are now supported
        assert_eq!(
            ProfileParser::parse_version("2.1").unwrap(),
            Version::V2_1
        );
        assert_eq!(
            ProfileParser::parse_version("2.8").unwrap(),
            Version::V2_8
        );
    }

    #[test]
    fn test_parse_cardinality_from_min_max() {
        let card = ProfileParser::parse_cardinality_from_min_max("1", "1").unwrap();
        assert_eq!(card.min, 1);
        assert_eq!(card.max, Some(1));

        let card = ProfileParser::parse_cardinality_from_min_max("0", "*").unwrap();
        assert_eq!(card.min, 0);
        assert_eq!(card.max, None);

        let card = ProfileParser::parse_cardinality_from_min_max("2", "5").unwrap();
        assert_eq!(card.min, 2);
        assert_eq!(card.max, Some(5));
    }

    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<HL7v2xConformanceProfile HL7Version="2.5" ProfileType="HL7">
  <HL7v2xStaticDef MsgType="ADT" EventType="A01" MsgStructID="ADT_A01">
    <MetaData Name="Test Profile" OrgName="Test Org" Version="1.0.0" Status="ACTIVE"/>
    <Segment Name="MSH" LongName="Message Header" Usage="R" Min="1" Max="1">
      <Field Name="Field Separator" Usage="R" Min="1" Max="1" Datatype="ST" Length="1" ItemNo="00001"/>
      <Field Name="Encoding Characters" Usage="R" Min="1" Max="1" Datatype="ST" Length="4" ItemNo="00002"/>
    </Segment>
    <Segment Name="PID" LongName="Patient Identification" Usage="R" Min="1" Max="1">
      <Field Name="Patient ID" Usage="R" Min="1" Max="1" Datatype="CX" Length="20" ItemNo="00003"/>
    </Segment>
  </HL7v2xStaticDef>
</HL7v2xConformanceProfile>"#;

        let profile = ProfileParser::parse_xml(xml).unwrap();

        // Check metadata
        assert_eq!(profile.metadata.name, "Test Profile");
        assert_eq!(profile.metadata.version, "1.0.0");
        assert_eq!(profile.metadata.hl7_version, Version::V2_5);
        assert_eq!(profile.metadata.organization, Some("Test Org".to_string()));

        // Check message profile
        assert_eq!(profile.message.message_type, "ADT");
        assert_eq!(profile.message.trigger_event, "A01");
        assert_eq!(profile.message.segments.len(), 2);

        // Check first segment (MSH)
        let msh = &profile.message.segments[0];
        assert_eq!(msh.name, "MSH");
        assert_eq!(msh.long_name, Some("Message Header".to_string()));
        assert_eq!(msh.usage, Usage::Required);
        assert_eq!(msh.cardinality.min, 1);
        assert_eq!(msh.cardinality.max, Some(1));
        assert_eq!(msh.fields.len(), 2);

        // Check first field of MSH
        let field1 = &msh.fields[0];
        assert_eq!(field1.position, 1);
        assert_eq!(field1.name, Some("Field Separator".to_string()));
        assert_eq!(field1.usage, ConditionalUsage::Required);
        assert_eq!(field1.datatype, Some("ST".to_string()));
        assert_eq!(field1.length, Some(1));

        // Check second segment (PID)
        let pid = &profile.message.segments[1];
        assert_eq!(pid.name, "PID");
        assert_eq!(pid.fields.len(), 1);
        // Position is the 1-based ordinal within the segment; the HL7 item
        // number (00003) is preserved separately and is not a position.
        assert_eq!(pid.fields[0].position, 1);
        assert_eq!(pid.fields[0].item_no, Some("00003".to_string()));
        assert_eq!(msh.fields[1].position, 2);
        assert_eq!(msh.fields[1].item_no, Some("00002".to_string()));
    }

    #[test]
    fn test_self_closing_segment_keeps_siblings() {
        // A self-closing <Segment .../> has no children: it must not
        // consume the following sibling Segment's Field elements.
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<HL7v2xConformanceProfile HL7Version="2.5" ProfileType="HL7">
  <HL7v2xStaticDef MsgType="ADT" EventType="A01" MsgStructID="ADT_A01">
    <MetaData Name="Test Profile" OrgName="Test Org" Version="1.0.0" Status="ACTIVE"/>
    <Segment Name="MSH" LongName="Message Header" Usage="R" Min="1" Max="1"/>
    <Segment Name="PID" LongName="Patient Identification" Usage="R" Min="1" Max="1">
      <Field Name="Patient ID" Usage="R" Min="1" Max="1" Datatype="CX" Length="20" ItemNo="00003"/>
    </Segment>
  </HL7v2xStaticDef>
</HL7v2xConformanceProfile>"#;

        let profile = ProfileParser::parse_xml(xml).unwrap();
        assert_eq!(profile.message.segments.len(), 2);
        assert_eq!(profile.message.segments[0].name, "MSH");
        assert!(profile.message.segments[0].fields.is_empty());
        assert_eq!(profile.message.segments[1].name, "PID");
        assert_eq!(profile.message.segments[1].fields.len(), 1);
        assert_eq!(profile.message.segments[1].fields[0].position, 1);
    }

    #[test]
    fn test_parse_file() {
        // Test parsing the sample XML file
        let profile = ProfileParser::parse_file("profiles/sample_adt_a01.xml").unwrap();

        assert_eq!(profile.metadata.name, "Sample ADT A01 Profile");
        assert_eq!(profile.metadata.organization, Some("VynaHL7 Project".to_string()));
        assert_eq!(profile.metadata.hl7_version, Version::V2_5);

        assert_eq!(profile.message.message_type, "ADT");
        assert_eq!(profile.message.trigger_event, "A01");

        // Should have MSH, EVN, PID, NK1, PV1
        assert_eq!(profile.message.segments.len(), 5);

        // Verify segment names
        assert_eq!(profile.message.segments[0].name, "MSH");
        assert_eq!(profile.message.segments[1].name, "EVN");
        assert_eq!(profile.message.segments[2].name, "PID");
        assert_eq!(profile.message.segments[3].name, "NK1");
        assert_eq!(profile.message.segments[4].name, "PV1");

        // Verify NK1 is optional with unbounded max
        let nk1 = &profile.message.segments[3];
        assert_eq!(nk1.usage, Usage::Optional);
        assert_eq!(nk1.cardinality.min, 0);
        assert_eq!(nk1.cardinality.max, None); // Unbounded (*)
    }
}
