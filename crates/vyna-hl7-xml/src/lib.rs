//! XML Encoding and Decoding for HL7 v2.x Messages
//!
//! This crate provides XML encoding and decoding capabilities for HL7 v2.x messages
//! according to the HL7 XML Encoding Rules.
//!
//! # Overview
//!
//! HL7 v2.x messages can be encoded in two formats:
//! - ER7 (Encoding Rules 7): The traditional pipe-delimited format
//! - XML: An XML-based format defined by HL7 International
//!
//! This crate enables:
//! - Parsing HL7 XML format into `vyna_hl7_core::Message`
//! - Encoding `vyna_hl7_core::Message` to HL7 XML format
//! - Bidirectional conversion between ER7 and XML formats
//!
//! # Examples
//!
//! ## Encoding a Message to XML
//!
//! ```ignore
//! use vyna_hl7_core::message::Message;
//! use vyna_hl7_xml::XmlEncoder;
//!
//! let message = Message::new();
//! let encoder = XmlEncoder::new();
//! let xml = encoder.encode(&message)?;
//! ```
//!
//! ## Decoding XML to a Message
//!
//! ```ignore
//! use vyna_hl7_xml::XmlDecoder;
//!
//! let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
//! <ADT_A01 xmlns="urn:hl7-org:v2xml">
//!     <MSH>...</MSH>
//! </ADT_A01>"#;
//!
//! let decoder = XmlDecoder::new();
//! let message = decoder.decode(xml)?;
//! ```

mod decoder;
mod encoder;
mod error;

pub use decoder::{XmlDecoder, XmlDecoderConfig};
pub use encoder::{XmlEncoder, XmlEncoderConfig};
pub use error::{XmlError, XmlResult};

/// The HL7 v2 XML namespace
pub const HL7_V2_XML_NAMESPACE: &str = "urn:hl7-org:v2xml";

/// The default XML declaration
pub const XML_DECLARATION: &str = r#"<?xml version="1.0" encoding="UTF-8"?>"#;

/// Convert an ER7 message to XML format
///
/// This is a convenience function that parses ER7 and encodes to XML.
/// Requires the `convert` feature.
///
/// # Example
///
/// ```ignore
/// use vyna_hl7_xml::er7_to_xml;
///
/// let er7 = "MSH|^~\\&|SENDING|FACILITY|...\rPID|1||...";
/// let xml = er7_to_xml(er7)?;
/// ```
#[cfg(feature = "convert")]
pub fn er7_to_xml(er7: &str) -> XmlResult<String> {
    let message = vyna_hl7_parser::parse(er7).map_err(|e| XmlError::ParseError(e.to_string()))?;
    let encoder = XmlEncoder::new();
    encoder.encode(&message)
}

/// Convert XML format to ER7
///
/// This is a convenience function that decodes XML and encodes to ER7.
///
/// # Example
///
/// ```ignore
/// use vyna_hl7_xml::xml_to_er7;
///
/// let xml = r#"<ADT_A01 xmlns="urn:hl7-org:v2xml">...</ADT_A01>"#;
/// let er7 = xml_to_er7(xml)?;
/// ```
pub fn xml_to_er7(xml: &str) -> XmlResult<String> {
    let decoder = XmlDecoder::new();
    let message = decoder.decode(xml)?;
    Ok(message.encode())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::message::Message;
    use vyna_hl7_core::segment::Segment;

    fn create_simple_message() -> Message {
        let mut message = Message::new();

        // Create MSH segment
        let mut msh = Segment::new("MSH");
        let _ = msh.set_field_value(3, "SENDING_APP");
        let _ = msh.set_field_value(4, "SENDING_FAC");
        let _ = msh.set_field_value(5, "RECEIVING_APP");
        let _ = msh.set_field_value(6, "RECEIVING_FAC");
        let _ = msh.set_field_value(7, "20240101120000");
        let _ = msh.set_field_value(9, "ADT^A01");
        let _ = msh.set_field_value(10, "MSG001");
        let _ = msh.set_field_value(11, "P");
        let _ = msh.set_field_value(12, "2.5");
        message.add_segment(msh);

        // Create PID segment
        let mut pid = Segment::new("PID");
        let _ = pid.set_field_value(1, "1");
        let _ = pid.set_field_value(3, "12345");
        let _ = pid.set_field_value(5, "Smith^John");
        let _ = pid.set_field_value(7, "19800101");
        let _ = pid.set_field_value(8, "M");
        message.add_segment(pid);

        message
    }

    #[test]
    fn test_encode_message_to_xml() {
        let message = create_simple_message();
        let encoder = XmlEncoder::new();
        let xml = encoder.encode(&message);

        assert!(xml.is_ok());
        let xml = xml.unwrap();

        assert!(xml.contains("<?xml version"));
        assert!(xml.contains("<MSH>"));
        assert!(xml.contains("<PID>"));
        assert!(xml.contains("</MSH>"));
        assert!(xml.contains("</PID>"));
        assert!(xml.contains("SENDING_APP"));
        assert!(xml.contains("Smith"));
    }

    #[test]
    fn test_encode_with_namespace() {
        let message = create_simple_message();
        let encoder = XmlEncoder::with_config(XmlEncoderConfig {
            include_namespace: true,
            pretty_print: true,
            ..Default::default()
        });
        let xml = encoder.encode(&message).unwrap();

        assert!(xml.contains("urn:hl7-org:v2xml"));
    }

    #[test]
    fn test_decode_simple_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ADT_A01>
    <MSH>
        <MSH.3>SENDING_APP</MSH.3>
        <MSH.4>SENDING_FAC</MSH.4>
        <MSH.5>RECEIVING_APP</MSH.5>
        <MSH.6>RECEIVING_FAC</MSH.6>
        <MSH.7>20240101120000</MSH.7>
        <MSH.9>
            <MSG.1>ADT</MSG.1>
            <MSG.2>A01</MSG.2>
        </MSH.9>
        <MSH.10>MSG001</MSH.10>
        <MSH.11>P</MSH.11>
        <MSH.12>2.5</MSH.12>
    </MSH>
    <PID>
        <PID.1>1</PID.1>
        <PID.3>12345</PID.3>
        <PID.5>
            <XPN.1>Smith</XPN.1>
            <XPN.2>John</XPN.2>
        </PID.5>
        <PID.7>19800101</PID.7>
        <PID.8>M</PID.8>
    </PID>
</ADT_A01>"#;

        let decoder = XmlDecoder::new();
        let result = decoder.decode(xml);

        assert!(result.is_ok());
        let message = result.unwrap();

        assert_eq!(message.segments.len(), 2);
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "PID");
    }

    #[test]
    fn test_roundtrip_xml() {
        let message = create_simple_message();

        // Encode to XML
        let encoder = XmlEncoder::new();
        let xml = encoder.encode(&message).unwrap();

        // Decode back
        let decoder = XmlDecoder::new();
        let decoded = decoder.decode(&xml).unwrap();

        // Compare segments
        assert_eq!(message.segments.len(), decoded.segments.len());

        for (orig, dec) in message.segments.iter().zip(decoded.segments.iter()) {
            assert_eq!(orig.id, dec.id);
        }
    }
}
