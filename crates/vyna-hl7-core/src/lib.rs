//! Core data structures and types for HL7 v2.x message processing
//!
//! This crate provides the foundational types used throughout the vyna_hl7 library:
//! - Message structure hierarchy (Message, Segment, Field, Component, Subcomponent)
//! - Batch and file structures for high-volume processing
//! - Message builders for creating HL7 messages programmatically
//! - Encoding characters and delimiters
//! - Error types
//! - Common traits

pub mod batch;
pub mod builders;
pub mod charset;
pub mod delimiters;
pub mod diff;
pub mod encoding;
pub mod error;
pub mod field;
pub mod message;
pub mod segment;
pub mod types;

pub use batch::{Batch, BatchHeader, BatchTrailer, File, FileHeader, FileTrailer};
pub use charset::CharacterSet;
pub use delimiters::Delimiters;
pub use encoding::Encoding;
pub use error::{Error, Result};
pub use field::{Component, Field, Repetition, SubComponent};
pub use message::Message;
pub use segment::Segment;

use std::str::FromStr;

/// HL7 version enum
///
/// Supports HL7 v2.x versions from 2.1 through 2.8.2.
/// This aligns with HL7apy's version coverage for maximum compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Version {
    /// HL7 v2.1 (1990) - First public version
    V2_1,
    /// HL7 v2.2 (1994)
    V2_2,
    /// HL7 v2.3 (1997)
    V2_3,
    /// HL7 v2.3.1 (1999)
    V2_3_1,
    /// HL7 v2.4 (2000)
    V2_4,
    /// HL7 v2.5 (2003)
    V2_5,
    /// HL7 v2.5.1 (2007)
    V2_5_1,
    /// HL7 v2.6 (2007)
    V2_6,
    /// HL7 v2.7 (2011)
    V2_7,
    /// HL7 v2.7.1 (2012)
    V2_7_1,
    /// HL7 v2.8 (2014)
    V2_8,
    /// HL7 v2.8.1 (2016)
    V2_8_1,
    /// HL7 v2.8.2 (2019)
    V2_8_2,
}

impl Version {
    /// Parse version from string (e.g., "2.5" or "2.5.1")
    ///
    /// Note: This method is kept for backward compatibility.
    /// Consider using the `FromStr` trait implementation instead.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "2.1" => Some(Version::V2_1),
            "2.2" => Some(Version::V2_2),
            "2.3" => Some(Version::V2_3),
            "2.3.1" => Some(Version::V2_3_1),
            "2.4" => Some(Version::V2_4),
            "2.5" => Some(Version::V2_5),
            "2.5.1" => Some(Version::V2_5_1),
            "2.6" => Some(Version::V2_6),
            "2.7" => Some(Version::V2_7),
            "2.7.1" => Some(Version::V2_7_1),
            "2.8" => Some(Version::V2_8),
            "2.8.1" => Some(Version::V2_8_1),
            "2.8.2" => Some(Version::V2_8_2),
            _ => None,
        }
    }

    /// Get version as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Version::V2_1 => "2.1",
            Version::V2_2 => "2.2",
            Version::V2_3 => "2.3",
            Version::V2_3_1 => "2.3.1",
            Version::V2_4 => "2.4",
            Version::V2_5 => "2.5",
            Version::V2_5_1 => "2.5.1",
            Version::V2_6 => "2.6",
            Version::V2_7 => "2.7",
            Version::V2_7_1 => "2.7.1",
            Version::V2_8 => "2.8",
            Version::V2_8_1 => "2.8.1",
            Version::V2_8_2 => "2.8.2",
        }
    }

    /// Get the major version number
    pub fn major(&self) -> u8 {
        2
    }

    /// Get the minor version number
    pub fn minor(&self) -> u8 {
        match self {
            Version::V2_1 => 1,
            Version::V2_2 => 2,
            Version::V2_3 | Version::V2_3_1 => 3,
            Version::V2_4 => 4,
            Version::V2_5 | Version::V2_5_1 => 5,
            Version::V2_6 => 6,
            Version::V2_7 | Version::V2_7_1 => 7,
            Version::V2_8 | Version::V2_8_1 | Version::V2_8_2 => 8,
        }
    }

    /// Get the patch version number (if any)
    pub fn patch(&self) -> Option<u8> {
        match self {
            Version::V2_3_1 | Version::V2_5_1 | Version::V2_7_1 | Version::V2_8_1 => Some(1),
            Version::V2_8_2 => Some(2),
            _ => None,
        }
    }

    /// Check if this version supports a feature introduced in a specific version
    ///
    /// Returns true if self >= min_version
    pub fn supports(&self, min_version: Version) -> bool {
        *self >= min_version
    }

    /// Check if this is a legacy version (v2.1 or v2.2)
    ///
    /// Legacy versions have limited segment/field definitions and may
    /// require special handling.
    pub fn is_legacy(&self) -> bool {
        matches!(self, Version::V2_1 | Version::V2_2)
    }

    /// Get all supported versions
    pub fn all() -> &'static [Version] {
        &[
            Version::V2_1,
            Version::V2_2,
            Version::V2_3,
            Version::V2_3_1,
            Version::V2_4,
            Version::V2_5,
            Version::V2_5_1,
            Version::V2_6,
            Version::V2_7,
            Version::V2_7_1,
            Version::V2_8,
            Version::V2_8_1,
            Version::V2_8_2,
        ]
    }

    /// Get the latest stable version
    pub fn latest() -> Version {
        Version::V2_8_2
    }

    /// Get the most commonly used version (v2.5.1)
    pub fn common() -> Version {
        Version::V2_5_1
    }
}

impl FromStr for Version {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::from_str(s).ok_or_else(|| Error::UnsupportedVersion(format!("Unknown HL7 version: {}", s)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(Version::from_str("2.5"), Some(Version::V2_5));
        assert_eq!(Version::from_str("2.7.1"), Some(Version::V2_7_1));
        assert_eq!(Version::from_str("invalid"), None);
    }

    #[test]
    fn test_version_as_str() {
        assert_eq!(Version::V2_5.as_str(), "2.5");
        assert_eq!(Version::V2_7_1.as_str(), "2.7.1");
    }

    #[test]
    fn test_legacy_versions() {
        assert_eq!(Version::from_str("2.1"), Some(Version::V2_1));
        assert_eq!(Version::from_str("2.2"), Some(Version::V2_2));
        assert!(Version::V2_1.is_legacy());
        assert!(Version::V2_2.is_legacy());
        assert!(!Version::V2_3.is_legacy());
    }

    #[test]
    fn test_new_versions() {
        assert_eq!(Version::from_str("2.8"), Some(Version::V2_8));
        assert_eq!(Version::from_str("2.8.1"), Some(Version::V2_8_1));
        assert_eq!(Version::from_str("2.8.2"), Some(Version::V2_8_2));
        assert_eq!(Version::V2_8.as_str(), "2.8");
        assert_eq!(Version::V2_8_2.as_str(), "2.8.2");
    }

    #[test]
    fn test_version_components() {
        assert_eq!(Version::V2_5_1.major(), 2);
        assert_eq!(Version::V2_5_1.minor(), 5);
        assert_eq!(Version::V2_5_1.patch(), Some(1));
        assert_eq!(Version::V2_5.patch(), None);
        assert_eq!(Version::V2_8_2.patch(), Some(2));
    }

    #[test]
    fn test_version_ordering() {
        assert!(Version::V2_1 < Version::V2_8_2);
        assert!(Version::V2_5 < Version::V2_5_1);
        assert!(Version::V2_7_1 < Version::V2_8);
    }

    #[test]
    fn test_version_supports() {
        assert!(Version::V2_8.supports(Version::V2_5));
        assert!(!Version::V2_3.supports(Version::V2_5));
        assert!(Version::V2_5.supports(Version::V2_5));
    }

    #[test]
    fn test_version_all() {
        let all = Version::all();
        assert_eq!(all.len(), 13);
        assert_eq!(all[0], Version::V2_1);
        assert_eq!(all[12], Version::V2_8_2);
    }
}
