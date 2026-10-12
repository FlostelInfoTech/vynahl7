//! Query result parsing for RSP (Response) messages
//!
//! This module provides utilities for parsing RSP messages and extracting query results.
//! It handles QAK (Query Acknowledgment) parsing and provides convenient access to query
//! response metadata.

use vyna_hl7_core::{error::{Error, Result}, message::Message};
use crate::Terser;

/// Query response status codes from HL7 Table 0208
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResponseStatus {
    /// OK - Data found, no errors
    Ok,
    /// NF - No data found, no errors
    NoDataFound,
    /// AE - Application error
    ApplicationError,
    /// AR - Application reject
    ApplicationReject,
    /// TM - Too much data found
    TooMuchData,
    /// PD - Protected data
    ProtectedData,
    /// Unknown status code
    Unknown(String),
}

impl std::str::FromStr for QueryResponseStatus {
    type Err = std::convert::Infallible;

    fn from_str(code: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self::from_str(code))
    }
}

impl QueryResponseStatus {
    /// Parse a query response status code from a string
    ///
    /// Note: This method is kept for backward compatibility.
    /// Consider using the `FromStr` trait implementation instead.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(code: &str) -> Self {
        match code {
            "OK" => QueryResponseStatus::Ok,
            "NF" => QueryResponseStatus::NoDataFound,
            "AE" => QueryResponseStatus::ApplicationError,
            "AR" => QueryResponseStatus::ApplicationReject,
            "TM" => QueryResponseStatus::TooMuchData,
            "PD" => QueryResponseStatus::ProtectedData,
            _ => QueryResponseStatus::Unknown(code.to_string()),
        }
    }

    /// Convert to HL7 code string
    pub fn as_str(&self) -> &str {
        match self {
            QueryResponseStatus::Ok => "OK",
            QueryResponseStatus::NoDataFound => "NF",
            QueryResponseStatus::ApplicationError => "AE",
            QueryResponseStatus::ApplicationReject => "AR",
            QueryResponseStatus::TooMuchData => "TM",
            QueryResponseStatus::ProtectedData => "PD",
            QueryResponseStatus::Unknown(code) => code,
        }
    }

    /// Check if the status indicates success (OK or NF)
    pub fn is_success(&self) -> bool {
        matches!(self, QueryResponseStatus::Ok | QueryResponseStatus::NoDataFound)
    }

    /// Check if the status indicates an error
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            QueryResponseStatus::ApplicationError | QueryResponseStatus::ApplicationReject
        )
    }
}

/// Query acknowledgment information from QAK segment
#[derive(Debug, Clone)]
pub struct QueryAcknowledgment {
    /// Query tag (QAK-1) - matches QPD-2 from the query
    pub query_tag: String,
    /// Query response status (QAK-2)
    pub status: QueryResponseStatus,
    /// Message query name (QAK-3)
    pub query_name: Option<String>,
    /// Total number of matching records (QAK-4)
    pub hit_count_total: Option<u32>,
    /// Number of records in this response (QAK-5)
    pub this_payload: Option<u32>,
    /// Number of records not yet sent (QAK-6)
    pub hits_remaining: Option<u32>,
}

impl QueryAcknowledgment {
    /// Check if there is more data available (pagination)
    pub fn has_more_data(&self) -> bool {
        self.hits_remaining.unwrap_or(0) > 0
    }

    /// Check if all data has been sent
    pub fn is_complete(&self) -> bool {
        self.hits_remaining.unwrap_or(0) == 0
    }

    /// Get the total number of records found
    pub fn total_records(&self) -> u32 {
        self.hit_count_total.unwrap_or(0)
    }

    /// Get the number of records in this response
    pub fn records_in_response(&self) -> u32 {
        self.this_payload.unwrap_or(0)
    }
}

/// Parser for extracting query results from RSP messages
pub struct QueryResultParser<'a> {
    message: &'a Message,
    terser: Terser<'a>,
}

impl<'a> QueryResultParser<'a> {
    /// Create a new query result parser for an RSP message
    pub fn new(message: &'a Message) -> Self {
        let terser = Terser::new(message);
        Self { message, terser }
    }

    /// Parse the QAK (Query Acknowledgment) segment
    ///
    /// Returns the query acknowledgment information or an error if QAK is missing or invalid.
    pub fn parse_acknowledgment(&self) -> Result<QueryAcknowledgment> {
        // Check if QAK segment exists
        if self.message.get_segments_by_id("QAK").is_empty() {
            return Err(Error::InvalidSegment("QAK segment not found".to_string()));
        }

        // QAK-1: Query Tag
        let query_tag = self.terser.get("QAK-1")?
            .ok_or_else(|| Error::MissingRequiredField("QAK-1".to_string()))?
            .to_string();

        // QAK-2: Query Response Status
        let status_code = self.terser.get("QAK-2")?
            .unwrap_or("OK");
        let status = QueryResponseStatus::from_str(status_code);

        // QAK-3: Message Query Name
        let query_name = self.terser.get("QAK-3")?
            .map(|s| s.to_string());

        // QAK-4: Hit Count Total
        let hit_count_total = self.terser.get("QAK-4")?
            .and_then(|s| s.parse::<u32>().ok());

        // QAK-5: This Payload
        let this_payload = self.terser.get("QAK-5")?
            .and_then(|s| s.parse::<u32>().ok());

        // QAK-6: Hits Remaining
        let hits_remaining = self.terser.get("QAK-6")?
            .and_then(|s| s.parse::<u32>().ok());

        Ok(QueryAcknowledgment {
            query_tag,
            status,
            query_name,
            hit_count_total,
            this_payload,
            hits_remaining,
        })
    }

    /// Get the message acknowledgment code (MSA-1)
    ///
    /// Returns AA (Application Accept), AE (Application Error), or AR (Application Reject)
    pub fn get_acknowledgment_code(&self) -> Result<&str> {
        self.terser.get("MSA-1")?
            .ok_or_else(|| Error::MissingRequiredField("MSA-1".to_string()))
    }

    /// Get the message control ID being responded to (MSA-2)
    pub fn get_message_control_id(&self) -> Result<&str> {
        self.terser.get("MSA-2")?
            .ok_or_else(|| Error::MissingRequiredField("MSA-2".to_string()))
    }

    /// Get all data segments (everything after QPD)
    ///
    /// Returns segment IDs and their segments. Common segments in RSP messages include:
    /// PID (patient demographics), ORC (order), RXA (administration), OBX (observation)
    pub fn get_data_segments(&self) -> Vec<&vyna_hl7_core::segment::Segment> {
        // Find the index of QPD segment
        let qpd_index = self.message.segments
            .iter()
            .position(|s| s.id == "QPD");

        if let Some(idx) = qpd_index {
            // Return all segments after QPD
            self.message.segments[idx + 1..]
                .iter()
                .filter(|s| !matches!(s.id.as_str(), "DSC")) // Exclude continuation pointer
                .collect()
        } else {
            // If no QPD, return segments after QAK
            let qak_index = self.message.segments
                .iter()
                .position(|s| s.id == "QAK");

            if let Some(idx) = qak_index {
                self.message.segments[idx + 1..]
                    .iter()
                    .filter(|s| !matches!(s.id.as_str(), "DSC" | "QPD"))
                    .collect()
            } else {
                Vec::new()
            }
        }
    }

    /// Get the continuation pointer (DSC-1) if present
    ///
    /// Returns the continuation token for retrieving the next page of results, or None if
    /// all data has been sent.
    pub fn get_continuation_pointer(&self) -> Result<Option<&str>> {
        if self.message.get_segments_by_id("DSC").is_empty() {
            return Ok(None);
        }

        self.terser.get("DSC-1")
    }

    /// Check if the query was successful
    ///
    /// Returns true if MSA-1 is "AA" and QAK-2 is "OK" or "NF"
    pub fn is_successful(&self) -> bool {
        let msa_ok = self.get_acknowledgment_code()
            .map(|code| code == "AA")
            .unwrap_or(false);

        let qak_ok = self.parse_acknowledgment()
            .map(|ack| ack.status.is_success())
            .unwrap_or(false);

        msa_ok && qak_ok
    }

    /// Get error text if the query failed
    ///
    /// Checks MSA-3 (text message) for error details
    pub fn get_error_text(&self) -> Result<Option<&str>> {
        self.terser.get("MSA-3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::builders::rsp::RspK22Builder;
    use vyna_hl7_core::{Version, field::Field, segment::Segment};
    use vyna_hl7_parser::parse_message;

    #[test]
    fn test_query_response_status_parsing() {
        assert_eq!(QueryResponseStatus::from_str("OK"), QueryResponseStatus::Ok);
        assert_eq!(QueryResponseStatus::from_str("NF"), QueryResponseStatus::NoDataFound);
        assert_eq!(QueryResponseStatus::from_str("AE"), QueryResponseStatus::ApplicationError);
        assert_eq!(QueryResponseStatus::from_str("AR"), QueryResponseStatus::ApplicationReject);
        assert_eq!(QueryResponseStatus::from_str("TM"), QueryResponseStatus::TooMuchData);
        assert_eq!(QueryResponseStatus::from_str("PD"), QueryResponseStatus::ProtectedData);

        assert!(QueryResponseStatus::Ok.is_success());
        assert!(QueryResponseStatus::NoDataFound.is_success());
        assert!(QueryResponseStatus::ApplicationError.is_error());
        assert!(!QueryResponseStatus::TooMuchData.is_error());
    }

    #[test]
    fn test_parse_successful_response() {
        let message = RspK22Builder::new(Version::V2_5_1)
            .sending_application("HOSPMPI")
            .receiving_application("CLINREG")
            .in_response_to("Q-001")
            .query_tag("987654321")
            .query_name("Q22^Find Candidates^HL7")
            .query_response_status("OK")
            .hit_counts(50, 10, 40)
            .build()
            .unwrap();

        let parser = QueryResultParser::new(&message);

        assert!(parser.is_successful());

        let ack = parser.parse_acknowledgment().unwrap();
        assert_eq!(ack.query_tag, "987654321");
        assert_eq!(ack.status, QueryResponseStatus::Ok);
        assert_eq!(ack.hit_count_total, Some(50));
        assert_eq!(ack.this_payload, Some(10));
        assert_eq!(ack.hits_remaining, Some(40));
        assert!(ack.has_more_data());
        assert!(!ack.is_complete());
    }

    #[test]
    fn test_parse_no_data_response() {
        let message = RspK22Builder::new(Version::V2_5_1)
            .sending_application("HOSPMPI")
            .receiving_application("CLINREG")
            .in_response_to("Q-002")
            .query_tag("123456")
            .query_response_status("NF")
            .hit_count(0)
            .build()
            .unwrap();

        let parser = QueryResultParser::new(&message);

        assert!(parser.is_successful());

        let ack = parser.parse_acknowledgment().unwrap();
        assert_eq!(ack.status, QueryResponseStatus::NoDataFound);
        assert_eq!(ack.total_records(), 0);
        assert!(!ack.has_more_data());
        assert!(ack.is_complete());
    }

    #[test]
    fn test_parse_with_data_segments() {
        let mut pid1 = Segment::new("PID");
        pid1.add_field(Field::from_value("1"));
        pid1.add_field(Field::from_value(""));
        pid1.add_field(Field::from_value("12345^^^MRN"));

        let mut pid2 = Segment::new("PID");
        pid2.add_field(Field::from_value("2"));
        pid2.add_field(Field::from_value(""));
        pid2.add_field(Field::from_value("67890^^^MRN"));

        let message = RspK22Builder::new(Version::V2_5_1)
            .sending_application("HOSPMPI")
            .receiving_application("CLINREG")
            .in_response_to("Q-003")
            .query_tag("ABC123")
            .query_response_status("OK")
            .hit_count(2)
            .add_segment(pid1)
            .add_segment(pid2)
            .build()
            .unwrap();

        let parser = QueryResultParser::new(&message);

        let data_segments = parser.get_data_segments();
        assert_eq!(data_segments.len(), 2);
        assert_eq!(data_segments[0].id, "PID");
        assert_eq!(data_segments[1].id, "PID");
    }

    #[test]
    fn test_parse_real_rsp_message() {
        let hl7 = "MSH|^~\\&|HOSPMPI|HOSP|CLINREG|WESTCLIN|20231115120001||RSP^K22|RSP-001|D|2.5.1\r\
                   MSA|AA|Q-20231115-045\r\
                   QAK|987654321|OK|Q22^Find Candidates^HL7|2|2|0\r\
                   QPD|Q22^Find Candidates^HL7|987654321|@PID.5.1^SMITH\r\
                   PID|1||1001^^^MPI^MR||SMITH^JOHN^A||19800515|M\r\
                   PID|2||1002^^^MPI^MR||SMITH^MARY^B||19850822|F";

        let message = parse_message(hl7).unwrap();
        let parser = QueryResultParser::new(&message);

        assert!(parser.is_successful());
        assert_eq!(parser.get_acknowledgment_code().unwrap(), "AA");
        assert_eq!(parser.get_message_control_id().unwrap(), "Q-20231115-045");

        let ack = parser.parse_acknowledgment().unwrap();
        assert_eq!(ack.query_tag, "987654321");
        assert_eq!(ack.status, QueryResponseStatus::Ok);
        assert_eq!(ack.total_records(), 2);
        assert_eq!(ack.records_in_response(), 2);
        assert!(ack.is_complete());

        let data_segments = parser.get_data_segments();
        assert_eq!(data_segments.len(), 2);
        assert!(data_segments.iter().all(|s| s.id == "PID"));
    }
}
