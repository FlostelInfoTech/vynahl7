//! Error types for XML encoding/decoding

use thiserror::Error;

/// Result type for XML operations
pub type XmlResult<T> = Result<T, XmlError>;

/// Errors that can occur during XML encoding/decoding
#[derive(Error, Debug)]
pub enum XmlError {
    /// Error parsing XML structure
    #[error("XML parsing error: {0}")]
    XmlParse(String),

    /// Error parsing HL7 content from XML
    #[error("HL7 parsing error: {0}")]
    ParseError(String),

    /// Error parsing HL7 content, preserving the underlying cause
    #[error("HL7 parsing error: {0}")]
    Parse(#[from] vyna_hl7_core::Error),

    /// Error encoding to XML
    #[error("XML encoding error: {0}")]
    EncodingError(String),

    /// Encoded output was not valid UTF-8
    #[error("XML UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Error resolving an XML entity reference
    #[error("XML entity error: {0}")]
    Unescape(#[from] quick_xml::escape::EscapeError),

    /// Invalid XML structure for HL7
    #[error("Invalid HL7 XML structure: {0}")]
    InvalidStructure(String),

    /// Missing required element
    #[error("Missing required XML element: {0}")]
    MissingElement(String),

    /// Invalid field reference
    #[error("Invalid field reference: {0}")]
    InvalidFieldReference(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// quick-xml error
    #[error("XML error: {0}")]
    QuickXml(#[from] quick_xml::Error),

    /// quick-xml attribute error
    #[error("XML attribute error: {0}")]
    AttrError(#[from] quick_xml::events::attributes::AttrError),
}
