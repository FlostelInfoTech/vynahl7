//! HL7 data type definitions
//!
//! This module defines common HL7 data types as specified in the standard.
//! These types provide semantic meaning to field values.

use crate::error::Error;
use chrono::{NaiveDate, NaiveDateTime};
use std::str::FromStr;

/// HL7 data type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// String (ST) - general string data
    ST,
    /// Text (TX) - text data
    TX,
    /// Formatted Text (FT) - formatted text
    FT,
    /// Numeric (NM) - numeric values
    NM,
    /// Sequence ID (SI) - sequence identifier
    SI,
    /// Date (DT) - date in YYYYMMDD format
    DT,
    /// Time (TM) - time in HHMMSS format
    TM,
    /// Date/Time (DTM) - timestamp in YYYYMMDDHHMMSS format
    DTM,
    /// Timestamp (TS) - timestamp with precision
    TS,
    /// Coded Element (CE) - coded value
    CE,
    /// Coded With Exceptions (CWE) - coded value with exceptions
    CWE,
    /// Identifier (ID) - coded identifier
    ID,
    /// Person Name (XPN) - extended person name
    XPN,
    /// Extended Address (XAD) - extended address
    XAD,
    /// Extended Telecommunication (XTN) - phone/email
    XTN,
    /// Composite ID (CX) - composite ID with assigning authority
    CX,
    /// Extended Composite ID (EI) - extended entity identifier
    EI,
    /// Hierarchic Designator (HD) - hierarchic designator
    HD,
    /// Message Type (MSG) - message type
    MSG,
    /// Processing Type (PT) - processing type
    PT,
    /// Coded Value (CNE) - coded with no exceptions
    CNE,
    /// Numeric Array (NA) - array of numeric values
    NA,
    /// Coded Value for HL7 Defined Tables (IS)
    IS,
    /// Extended Composite ID Number and Name for Persons (XCN)
    XCN,
    /// Person Location (PL)
    PL,
    /// Version Identifier (VID)
    VID,
    /// Composite Price (CP)
    CP,
    /// Composite Quantity with Units (CQ)
    CQ,
    /// Date/Time Range (DR)
    DR,
    /// Entity Identifier Pair (EIP)
    EIP,
    /// Error Location and Description (ELD)
    ELD,
    /// Timing/Quantity (TQ)
    TQ,
    /// Variable Data Type (Varies) - context-dependent
    Varies,
    /// Structured Numeric (SN)
    SN,
    /// Extended Composite Name and ID for Organizations (XON)
    XON,
    /// Family Name (FN) - subcomponent of XPN/XCN
    FN,
    /// Money (MO)
    MO,
    /// Coded with Exceptions (CF)
    CF,
    /// Day Range (DIN)
    DIN,
    /// Discharge to Location (DLD)
    DLD,
    /// Financial Class (FC)
    FC,
    /// Job Code/Class (JCC)
    JCC,
    /// Person Name (PN) - legacy pre-2.3 person name
    PN,
    /// Address (AD) - legacy pre-2.3 address
    AD,
    /// Telephone Number (TN) - legacy pre-2.3 telecom
    TN,
    /// Composite ID and Name (CN) - legacy pre-2.3 person ID
    CN,
    /// Composite ID (CK) - legacy pre-2.3 composite ID
    CK,
    /// Repeat Interval (RI)
    RI,
    /// Composite (CM) - legacy generic composite
    CM,
}

impl DataType {
    /// Get the data type code as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::ST => "ST",
            DataType::TX => "TX",
            DataType::FT => "FT",
            DataType::NM => "NM",
            DataType::SI => "SI",
            DataType::DT => "DT",
            DataType::TM => "TM",
            DataType::DTM => "DTM",
            DataType::TS => "TS",
            DataType::CE => "CE",
            DataType::CWE => "CWE",
            DataType::ID => "ID",
            DataType::XPN => "XPN",
            DataType::XAD => "XAD",
            DataType::XTN => "XTN",
            DataType::CX => "CX",
            DataType::EI => "EI",
            DataType::HD => "HD",
            DataType::MSG => "MSG",
            DataType::PT => "PT",
            DataType::CNE => "CNE",
            DataType::NA => "NA",
            DataType::IS => "IS",
            DataType::XCN => "XCN",
            DataType::PL => "PL",
            DataType::VID => "VID",
            DataType::CP => "CP",
            DataType::CQ => "CQ",
            DataType::DR => "DR",
            DataType::EIP => "EIP",
            DataType::ELD => "ELD",
            DataType::TQ => "TQ",
            DataType::Varies => "Varies",
            DataType::SN => "SN",
            DataType::XON => "XON",
            DataType::FN => "FN",
            DataType::MO => "MO",
            DataType::CF => "CF",
            DataType::DIN => "DIN",
            DataType::DLD => "DLD",
            DataType::FC => "FC",
            DataType::JCC => "JCC",
            DataType::PN => "PN",
            DataType::AD => "AD",
            DataType::TN => "TN",
            DataType::CN => "CN",
            DataType::CK => "CK",
            DataType::RI => "RI",
            DataType::CM => "CM",
        }
    }

    /// Parse a data type from a string
    ///
    /// Note: This method is kept for backward compatibility.
    /// Consider using the `FromStr` trait implementation instead.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ST" => Some(DataType::ST),
            "TX" => Some(DataType::TX),
            "FT" => Some(DataType::FT),
            "NM" => Some(DataType::NM),
            "SI" => Some(DataType::SI),
            "DT" => Some(DataType::DT),
            "TM" => Some(DataType::TM),
            "DTM" => Some(DataType::DTM),
            "TS" => Some(DataType::TS),
            "CE" => Some(DataType::CE),
            "CWE" => Some(DataType::CWE),
            "ID" => Some(DataType::ID),
            "XPN" => Some(DataType::XPN),
            "XAD" => Some(DataType::XAD),
            "XTN" => Some(DataType::XTN),
            "CX" => Some(DataType::CX),
            "EI" => Some(DataType::EI),
            "HD" => Some(DataType::HD),
            "MSG" => Some(DataType::MSG),
            "PT" => Some(DataType::PT),
            "CNE" => Some(DataType::CNE),
            "NA" => Some(DataType::NA),
            "IS" => Some(DataType::IS),
            "XCN" => Some(DataType::XCN),
            "PL" => Some(DataType::PL),
            "VID" => Some(DataType::VID),
            "CP" => Some(DataType::CP),
            "CQ" => Some(DataType::CQ),
            "DR" => Some(DataType::DR),
            "EIP" => Some(DataType::EIP),
            "ELD" => Some(DataType::ELD),
            "TQ" => Some(DataType::TQ),
            "Varies" | "varies" => Some(DataType::Varies),
            "SN" => Some(DataType::SN),
            "XON" => Some(DataType::XON),
            "FN" => Some(DataType::FN),
            "MO" => Some(DataType::MO),
            "CF" => Some(DataType::CF),
            "DIN" => Some(DataType::DIN),
            "DLD" => Some(DataType::DLD),
            "FC" => Some(DataType::FC),
            "JCC" => Some(DataType::JCC),
            "PN" => Some(DataType::PN),
            "AD" => Some(DataType::AD),
            "TN" => Some(DataType::TN),
            "CN" => Some(DataType::CN),
            "CK" => Some(DataType::CK),
            "RI" => Some(DataType::RI),
            "CM" => Some(DataType::CM),
            _ => None,
        }
    }
}

impl FromStr for DataType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| Error::parse(format!("Unknown data type: {}", s)))
    }
}

/// Parse HL7 date (DT) format: YYYYMMDD or YYYY or YYYYMM
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    match s.len() {
        4 => {
            // YYYY
            let year = s.parse::<i32>().ok()?;
            NaiveDate::from_ymd_opt(year, 1, 1)
        }
        6 => {
            // YYYYMM (.get: non-ASCII input yields None, never panics)
            let year = s.get(0..4)?.parse::<i32>().ok()?;
            let month = s.get(4..6)?.parse::<u32>().ok()?;
            NaiveDate::from_ymd_opt(year, month, 1)
        }
        8 => {
            // YYYYMMDD (.get: non-ASCII input yields None, never panics)
            let year = s.get(0..4)?.parse::<i32>().ok()?;
            let month = s.get(4..6)?.parse::<u32>().ok()?;
            let day = s.get(6..8)?.parse::<u32>().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)
        }
        _ => None,
    }
}

/// Parse HL7 timestamp (TS/DTM) into a naive local datetime.
///
/// Accepted grammar: 8, 10, 12, or 14 digits (`YYYYMMDD[HH[MM[SS]]]`)
/// with an optional fractional-seconds suffix (`.S...`, folded into
/// nanoseconds). Values carrying a UTC offset (`+/-ZZZZ`) are refused
/// (`None`): a `NaiveDateTime` cannot represent an offset, and silently
/// dropping one shifts the instant by hours. For offset-carrying
/// values, use [`Hl7DateTime`](crate::datetime::Hl7DateTime) instead.
pub fn parse_timestamp(s: &str) -> Option<NaiveDateTime> {
    // A sign character can only start an offset suffix, which this
    // return type cannot represent.
    if s.contains(['+', '-']) {
        return None;
    }

    let (digits, fraction) = match s.split_once('.') {
        Some((head, tail)) => (head, Some(tail)),
        None => (s, None),
    };
    if !matches!(digits.len(), 8 | 10 | 12 | 14) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    let nanos = match fraction {
        None => 0,
        Some(frac) => {
            if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            // ASCII-only, so byte slicing is a char boundary.
            let head = &frac[..frac.len().min(9)];
            let mut nanos = head.parse::<u32>().ok()?;
            for _ in head.len()..9 {
                nanos *= 10;
            }
            nanos
        }
    };

    let year = digits.get(0..4)?.parse::<i32>().ok()?;
    let month = digits.get(4..6)?.parse::<u32>().ok()?;
    let day = digits.get(6..8)?.parse::<u32>().ok()?;

    let (hour, minute, second) = match digits.len() {
        14 => (
            digits.get(8..10)?.parse::<u32>().ok()?,
            digits.get(10..12)?.parse::<u32>().ok()?,
            digits.get(12..14)?.parse::<u32>().ok()?,
        ),
        12 => (
            digits.get(8..10)?.parse::<u32>().ok()?,
            digits.get(10..12)?.parse::<u32>().ok()?,
            0,
        ),
        10 => (digits.get(8..10)?.parse::<u32>().ok()?, 0, 0),
        _ => (0, 0, 0),
    };

    NaiveDate::from_ymd_opt(year, month, day)?.and_hms_nano_opt(hour, minute, second, nanos)
}

/// Format a date to HL7 DT format (YYYYMMDD)
pub fn format_date(date: &NaiveDate) -> String {
    date.format("%Y%m%d").to_string()
}

/// Format a timestamp to HL7 TS format (YYYYMMDDHHMMSS)
pub fn format_timestamp(dt: &NaiveDateTime) -> String {
    dt.format("%Y%m%d%H%M%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_type_string() {
        assert_eq!(DataType::ST.as_str(), "ST");
        assert_eq!(DataType::NM.as_str(), "NM");
        assert_eq!(DataType::TS.as_str(), "TS");
    }

    #[test]
    fn test_data_type_parse() {
        assert_eq!(DataType::from_str("ST"), Some(DataType::ST));
        assert_eq!(DataType::from_str("INVALID"), None);
    }

    #[test]
    fn test_parse_date() {
        assert_eq!(parse_date("20240315"), NaiveDate::from_ymd_opt(2024, 3, 15));
        assert_eq!(parse_date("202403"), NaiveDate::from_ymd_opt(2024, 3, 1));
        assert_eq!(parse_date("2024"), NaiveDate::from_ymd_opt(2024, 1, 1));
        assert_eq!(parse_date("invalid"), None);
    }

    #[test]
    fn test_parse_timestamp() {
        let ts = parse_timestamp("20240315143000").unwrap();
        assert_eq!(ts.format("%Y%m%d%H%M%S").to_string(), "20240315143000");

        let ts2 = parse_timestamp("20240315").unwrap();
        assert_eq!(ts2.format("%Y%m%d").to_string(), "20240315");
    }

    #[test]
    fn test_format_date() {
        let date = NaiveDate::from_ymd_opt(2024, 3, 15).unwrap();
        assert_eq!(format_date(&date), "20240315");
    }

    #[test]
    fn test_format_timestamp() {
        let dt = NaiveDate::from_ymd_opt(2024, 3, 15)
            .unwrap()
            .and_hms_opt(14, 30, 45)
            .unwrap();
        assert_eq!(format_timestamp(&dt), "20240315143045");
    }

    #[test]
    fn test_parse_date_non_ascii_returns_none_without_panic() {
        // 8/6 bytes but not on char boundaries: slicing panicked here.
        assert_eq!(parse_date("123456é"), None);
        assert_eq!(parse_date("1234é"), None);
        assert!(parse_date("20240315").is_some());
    }

    #[test]
    fn test_parse_timestamp_non_ascii_returns_none_without_panic() {
        assert_eq!(parse_timestamp("1234567é901234"), None);
        assert_eq!(parse_timestamp("2024é315143045"), None);
        assert!(parse_timestamp("20240315143045").is_some());
    }

    #[test]
    fn test_parse_timestamp_rejects_offsets() {
        // A NaiveDateTime cannot hold a UTC offset; silently dropping
        // +0800 shifts the instant by 8h, so offset values are refused.
        assert_eq!(parse_timestamp("20240101120000+0800"), None);
        assert_eq!(parse_timestamp("20240101120000.123-0530"), None);
    }

    #[test]
    fn test_parse_timestamp_fraction_and_trailing_junk() {
        let ts = parse_timestamp("20240101120000.5").unwrap();
        assert_eq!(
            ts.format("%Y%m%d%H%M%S%.3f").to_string(),
            "20240101120000.500"
        );
        // Digit runs must be exactly 8/10/12/14; trailing junk fails.
        assert_eq!(parse_timestamp("202401011"), None);
        assert_eq!(parse_timestamp("20240101120000XYZ"), None);
        assert_eq!(parse_timestamp("20240101120000+0800"), None);
    }
}
