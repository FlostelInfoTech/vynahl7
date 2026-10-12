//! Schema loader for HL7 message definitions
//!
//! This module provides functionality to load message schemas from JSON files
//! embedded in the binary at compile time.

use crate::MessageSchema;
use vyna_hl7_core::{error::Result, Version};

/// Load a schema for a specific message type and version
pub fn load_schema(version: Version, message_type: &str, trigger_event: &str) -> Result<MessageSchema> {
    let version_str = match version {
        // Legacy versions fall back to v2.3 schemas
        Version::V2_1 | Version::V2_2 => "v2_3",
        Version::V2_3 | Version::V2_3_1 => "v2_3",
        Version::V2_4 => "v2_4",
        Version::V2_5 | Version::V2_5_1 => "v2_5",
        Version::V2_6 => "v2_6",
        Version::V2_7 | Version::V2_7_1 => "v2_7",
        Version::V2_8 | Version::V2_8_1 | Version::V2_8_2 => "v2_8",
    };

    // Construct the schema key
    let schema_key = if trigger_event.is_empty() {
        message_type.to_string()
    } else {
        format!("{}_{}", message_type, trigger_event)
    };

    // Try to load embedded schema
    match load_embedded_schema(version_str, &schema_key) {
        Some(schema) => Ok(schema),
        None => Err(vyna_hl7_core::error::Error::Validation(format!(
            "Schema not found for {} {} version {}",
            message_type, trigger_event, version.as_str()
        ))),
    }
}

/// Load an embedded schema from compile-time included JSON
fn load_embedded_schema(version: &str, schema_key: &str) -> Option<MessageSchema> {
    // Include schemas at compile time
    match (version, schema_key) {
        // V2.3 schemas - ADT messages
        ("v2_3", "ADT_A01") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A01.json")),
        ("v2_3", "ADT_A02") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A02.json")),
        ("v2_3", "ADT_A03") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A03.json")),
        ("v2_3", "ADT_A04") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A04.json")),
        ("v2_3", "ADT_A05") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A05.json")),
        ("v2_3", "ADT_A06") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A06.json")),
        ("v2_3", "ADT_A07") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A07.json")),
        ("v2_3", "ADT_A08") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A08.json")),
        ("v2_3", "ADT_A09") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A09.json")),
        ("v2_3", "ADT_A10") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A10.json")),
        ("v2_3", "ADT_A11") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A11.json")),
        ("v2_3", "ADT_A12") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A12.json")),
        ("v2_3", "ADT_A13") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A13.json")),
        ("v2_3", "ADT_A17") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A17.json")),
        ("v2_3", "ADT_A28") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A28.json")),
        ("v2_3", "ADT_A31") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A31.json")),
        ("v2_3", "ADT_A34") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A34.json")),
        ("v2_3", "ADT_A40") => parse_schema_json(include_str!("../schemas/v2_3/ADT_A40.json")),
        // V2.3 - Other message types
        ("v2_3", "ORU_R01") => parse_schema_json(include_str!("../schemas/v2_3/ORU_R01.json")),
        ("v2_3", "ORM_O01") => parse_schema_json(include_str!("../schemas/v2_3/ORM_O01.json")),
        ("v2_3", "ACK") => parse_schema_json(include_str!("../schemas/v2_3/ACK.json")),
        ("v2_3", "SIU_S12") => parse_schema_json(include_str!("../schemas/v2_3/SIU_S12.json")),
        ("v2_3", "SIU_S13") => parse_schema_json(include_str!("../schemas/v2_3/SIU_S13.json")),
        ("v2_3", "SIU_S14") => parse_schema_json(include_str!("../schemas/v2_3/SIU_S14.json")),
        ("v2_3", "SIU_S15") => parse_schema_json(include_str!("../schemas/v2_3/SIU_S15.json")),
        ("v2_3", "MDM_T01") => parse_schema_json(include_str!("../schemas/v2_3/MDM_T01.json")),
        ("v2_3", "MDM_T02") => parse_schema_json(include_str!("../schemas/v2_3/MDM_T02.json")),
        ("v2_3", "MDM_T04") => parse_schema_json(include_str!("../schemas/v2_3/MDM_T04.json")),
        ("v2_3", "DFT_P03") => parse_schema_json(include_str!("../schemas/v2_3/DFT_P03.json")),
        ("v2_3", "DFT_P11") => parse_schema_json(include_str!("../schemas/v2_3/DFT_P11.json")),
        ("v2_3", "QRY_A19") => parse_schema_json(include_str!("../schemas/v2_3/QRY_A19.json")),
        ("v2_3", "QRY_Q01") => parse_schema_json(include_str!("../schemas/v2_3/QRY_Q01.json")),
        ("v2_3", "QRY_Q02") => parse_schema_json(include_str!("../schemas/v2_3/QRY_Q02.json")),
        ("v2_3", "BAR_P01") => parse_schema_json(include_str!("../schemas/v2_3/BAR_P01.json")),
        ("v2_3", "BAR_P02") => parse_schema_json(include_str!("../schemas/v2_3/BAR_P02.json")),
        ("v2_3", "RDE_O11") => parse_schema_json(include_str!("../schemas/v2_3/RDE_O11.json")),
        ("v2_3", "RAS_O17") => parse_schema_json(include_str!("../schemas/v2_3/RAS_O17.json")),
        ("v2_3", "MFN_M01") => parse_schema_json(include_str!("../schemas/v2_3/MFN_M01.json")),
        ("v2_3", "RDS_O13") => parse_schema_json(include_str!("../schemas/v2_3/RDS_O13.json")),
        ("v2_3", "RGV_O15") => parse_schema_json(include_str!("../schemas/v2_3/RGV_O15.json")),
        ("v2_3", "RRD_O14") => parse_schema_json(include_str!("../schemas/v2_3/RRD_O14.json")),
        ("v2_3", "RRA_O18") => parse_schema_json(include_str!("../schemas/v2_3/RRA_O18.json")),
        ("v2_3", "OUL_R21") => parse_schema_json(include_str!("../schemas/v2_3/OUL_R21.json")),
        ("v2_3", "OML_O21") => parse_schema_json(include_str!("../schemas/v2_3/OML_O21.json")),
        ("v2_3", "VXU_V04") => parse_schema_json(include_str!("../schemas/v2_3/VXU_V04.json")),
        ("v2_3", "PPR_PC1") => parse_schema_json(include_str!("../schemas/v2_3/PPR_PC1.json")),
        ("v2_3", "OMP_O09") => parse_schema_json(include_str!("../schemas/v2_3/OMP_O09.json")),

        // V2.4 schemas - ADT messages
        ("v2_4", "ADT_A01") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A01.json")),
        ("v2_4", "ADT_A02") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A02.json")),
        ("v2_4", "ADT_A03") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A03.json")),
        ("v2_4", "ADT_A04") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A04.json")),
        ("v2_4", "ADT_A05") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A05.json")),
        ("v2_4", "ADT_A06") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A06.json")),
        ("v2_4", "ADT_A07") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A07.json")),
        ("v2_4", "ADT_A08") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A08.json")),
        ("v2_4", "ADT_A09") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A09.json")),
        ("v2_4", "ADT_A10") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A10.json")),
        ("v2_4", "ADT_A11") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A11.json")),
        ("v2_4", "ADT_A12") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A12.json")),
        ("v2_4", "ADT_A13") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A13.json")),
        ("v2_4", "ADT_A17") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A17.json")),
        ("v2_4", "ADT_A28") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A28.json")),
        ("v2_4", "ADT_A31") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A31.json")),
        ("v2_4", "ADT_A34") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A34.json")),
        ("v2_4", "ADT_A40") => parse_schema_json(include_str!("../schemas/v2_4/ADT_A40.json")),
        // V2.4 - Other message types
        ("v2_4", "ORU_R01") => parse_schema_json(include_str!("../schemas/v2_4/ORU_R01.json")),
        ("v2_4", "ORM_O01") => parse_schema_json(include_str!("../schemas/v2_4/ORM_O01.json")),
        ("v2_4", "ACK") => parse_schema_json(include_str!("../schemas/v2_4/ACK.json")),
        ("v2_4", "SIU_S12") => parse_schema_json(include_str!("../schemas/v2_4/SIU_S12.json")),
        ("v2_4", "SIU_S13") => parse_schema_json(include_str!("../schemas/v2_4/SIU_S13.json")),
        ("v2_4", "SIU_S14") => parse_schema_json(include_str!("../schemas/v2_4/SIU_S14.json")),
        ("v2_4", "SIU_S15") => parse_schema_json(include_str!("../schemas/v2_4/SIU_S15.json")),
        ("v2_4", "MDM_T01") => parse_schema_json(include_str!("../schemas/v2_4/MDM_T01.json")),
        ("v2_4", "MDM_T02") => parse_schema_json(include_str!("../schemas/v2_4/MDM_T02.json")),
        ("v2_4", "MDM_T04") => parse_schema_json(include_str!("../schemas/v2_4/MDM_T04.json")),
        ("v2_4", "DFT_P03") => parse_schema_json(include_str!("../schemas/v2_4/DFT_P03.json")),
        ("v2_4", "DFT_P11") => parse_schema_json(include_str!("../schemas/v2_4/DFT_P11.json")),
        ("v2_4", "QRY_A19") => parse_schema_json(include_str!("../schemas/v2_4/QRY_A19.json")),
        ("v2_4", "QRY_Q01") => parse_schema_json(include_str!("../schemas/v2_4/QRY_Q01.json")),
        ("v2_4", "QRY_Q02") => parse_schema_json(include_str!("../schemas/v2_4/QRY_Q02.json")),
        ("v2_4", "BAR_P01") => parse_schema_json(include_str!("../schemas/v2_4/BAR_P01.json")),
        ("v2_4", "BAR_P02") => parse_schema_json(include_str!("../schemas/v2_4/BAR_P02.json")),
        ("v2_4", "RDE_O11") => parse_schema_json(include_str!("../schemas/v2_4/RDE_O11.json")),
        ("v2_4", "RAS_O17") => parse_schema_json(include_str!("../schemas/v2_4/RAS_O17.json")),
        ("v2_4", "MFN_M01") => parse_schema_json(include_str!("../schemas/v2_4/MFN_M01.json")),
        ("v2_4", "RDS_O13") => parse_schema_json(include_str!("../schemas/v2_4/RDS_O13.json")),
        ("v2_4", "RGV_O15") => parse_schema_json(include_str!("../schemas/v2_4/RGV_O15.json")),
        ("v2_4", "RRD_O14") => parse_schema_json(include_str!("../schemas/v2_4/RRD_O14.json")),
        ("v2_4", "RRA_O18") => parse_schema_json(include_str!("../schemas/v2_4/RRA_O18.json")),
        ("v2_4", "OUL_R21") => parse_schema_json(include_str!("../schemas/v2_4/OUL_R21.json")),
        ("v2_4", "OML_O21") => parse_schema_json(include_str!("../schemas/v2_4/OML_O21.json")),
        ("v2_4", "VXU_V04") => parse_schema_json(include_str!("../schemas/v2_4/VXU_V04.json")),
        ("v2_4", "PPR_PC1") => parse_schema_json(include_str!("../schemas/v2_4/PPR_PC1.json")),
        ("v2_4", "OMP_O09") => parse_schema_json(include_str!("../schemas/v2_4/OMP_O09.json")),

        // V2.5 schemas - ADT messages
        ("v2_5", "ADT_A01") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A01.json")),
        ("v2_5", "ADT_A02") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A02.json")),
        ("v2_5", "ADT_A03") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A03.json")),
        ("v2_5", "ADT_A04") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A04.json")),
        ("v2_5", "ADT_A05") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A05.json")),
        ("v2_5", "ADT_A06") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A06.json")),
        ("v2_5", "ADT_A07") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A07.json")),
        ("v2_5", "ADT_A08") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A08.json")),
        ("v2_5", "ADT_A09") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A09.json")),
        ("v2_5", "ADT_A10") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A10.json")),
        ("v2_5", "ADT_A11") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A11.json")),
        ("v2_5", "ADT_A12") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A12.json")),
        ("v2_5", "ADT_A13") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A13.json")),
        ("v2_5", "ADT_A17") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A17.json")),
        ("v2_5", "ADT_A28") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A28.json")),
        ("v2_5", "ADT_A31") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A31.json")),
        ("v2_5", "ADT_A34") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A34.json")),
        ("v2_5", "ADT_A40") => parse_schema_json(include_str!("../schemas/v2_5/ADT_A40.json")),
        // V2.5 - Other message types
        ("v2_5", "ORU_R01") => parse_schema_json(include_str!("../schemas/v2_5/ORU_R01.json")),
        ("v2_5", "ORM_O01") => parse_schema_json(include_str!("../schemas/v2_5/ORM_O01.json")),
        ("v2_5", "ACK") => parse_schema_json(include_str!("../schemas/v2_5/ACK.json")),
        ("v2_5", "SIU_S12") => parse_schema_json(include_str!("../schemas/v2_5/SIU_S12.json")),
        ("v2_5", "SIU_S13") => parse_schema_json(include_str!("../schemas/v2_5/SIU_S13.json")),
        ("v2_5", "SIU_S14") => parse_schema_json(include_str!("../schemas/v2_5/SIU_S14.json")),
        ("v2_5", "SIU_S15") => parse_schema_json(include_str!("../schemas/v2_5/SIU_S15.json")),
        ("v2_5", "MDM_T01") => parse_schema_json(include_str!("../schemas/v2_5/MDM_T01.json")),
        ("v2_5", "MDM_T02") => parse_schema_json(include_str!("../schemas/v2_5/MDM_T02.json")),
        ("v2_5", "MDM_T04") => parse_schema_json(include_str!("../schemas/v2_5/MDM_T04.json")),
        ("v2_5", "DFT_P03") => parse_schema_json(include_str!("../schemas/v2_5/DFT_P03.json")),
        ("v2_5", "DFT_P11") => parse_schema_json(include_str!("../schemas/v2_5/DFT_P11.json")),
        ("v2_5", "QRY_A19") => parse_schema_json(include_str!("../schemas/v2_5/QRY_A19.json")),
        ("v2_5", "QRY_Q01") => parse_schema_json(include_str!("../schemas/v2_5/QRY_Q01.json")),
        ("v2_5", "QRY_Q02") => parse_schema_json(include_str!("../schemas/v2_5/QRY_Q02.json")),
        ("v2_5", "BAR_P01") => parse_schema_json(include_str!("../schemas/v2_5/BAR_P01.json")),
        ("v2_5", "BAR_P02") => parse_schema_json(include_str!("../schemas/v2_5/BAR_P02.json")),
        ("v2_5", "RDE_O11") => parse_schema_json(include_str!("../schemas/v2_5/RDE_O11.json")),
        ("v2_5", "RAS_O17") => parse_schema_json(include_str!("../schemas/v2_5/RAS_O17.json")),
        ("v2_5", "MFN_M01") => parse_schema_json(include_str!("../schemas/v2_5/MFN_M01.json")),
        ("v2_5", "RDS_O13") => parse_schema_json(include_str!("../schemas/v2_5/RDS_O13.json")),
        ("v2_5", "RGV_O15") => parse_schema_json(include_str!("../schemas/v2_5/RGV_O15.json")),
        ("v2_5", "RRD_O14") => parse_schema_json(include_str!("../schemas/v2_5/RRD_O14.json")),
        ("v2_5", "RRA_O18") => parse_schema_json(include_str!("../schemas/v2_5/RRA_O18.json")),
        ("v2_5", "OUL_R21") => parse_schema_json(include_str!("../schemas/v2_5/OUL_R21.json")),
        ("v2_5", "OML_O21") => parse_schema_json(include_str!("../schemas/v2_5/OML_O21.json")),
        ("v2_5", "VXU_V04") => parse_schema_json(include_str!("../schemas/v2_5/VXU_V04.json")),
        ("v2_5", "PPR_PC1") => parse_schema_json(include_str!("../schemas/v2_5/PPR_PC1.json")),
        ("v2_5", "OMP_O09") => parse_schema_json(include_str!("../schemas/v2_5/OMP_O09.json")),

        // V2.6 schemas - ADT messages
        ("v2_6", "ADT_A01") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A01.json")),
        ("v2_6", "ADT_A02") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A02.json")),
        ("v2_6", "ADT_A03") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A03.json")),
        ("v2_6", "ADT_A04") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A04.json")),
        ("v2_6", "ADT_A05") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A05.json")),
        ("v2_6", "ADT_A06") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A06.json")),
        ("v2_6", "ADT_A07") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A07.json")),
        ("v2_6", "ADT_A08") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A08.json")),
        ("v2_6", "ADT_A09") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A09.json")),
        ("v2_6", "ADT_A10") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A10.json")),
        ("v2_6", "ADT_A11") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A11.json")),
        ("v2_6", "ADT_A12") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A12.json")),
        ("v2_6", "ADT_A13") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A13.json")),
        ("v2_6", "ADT_A17") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A17.json")),
        ("v2_6", "ADT_A28") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A28.json")),
        ("v2_6", "ADT_A31") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A31.json")),
        ("v2_6", "ADT_A34") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A34.json")),
        ("v2_6", "ADT_A40") => parse_schema_json(include_str!("../schemas/v2_6/ADT_A40.json")),
        // V2.6 - Other message types
        ("v2_6", "ORU_R01") => parse_schema_json(include_str!("../schemas/v2_6/ORU_R01.json")),
        ("v2_6", "ORM_O01") => parse_schema_json(include_str!("../schemas/v2_6/ORM_O01.json")),
        ("v2_6", "ACK") => parse_schema_json(include_str!("../schemas/v2_6/ACK.json")),
        ("v2_6", "SIU_S12") => parse_schema_json(include_str!("../schemas/v2_6/SIU_S12.json")),
        ("v2_6", "SIU_S13") => parse_schema_json(include_str!("../schemas/v2_6/SIU_S13.json")),
        ("v2_6", "SIU_S14") => parse_schema_json(include_str!("../schemas/v2_6/SIU_S14.json")),
        ("v2_6", "SIU_S15") => parse_schema_json(include_str!("../schemas/v2_6/SIU_S15.json")),
        ("v2_6", "MDM_T01") => parse_schema_json(include_str!("../schemas/v2_6/MDM_T01.json")),
        ("v2_6", "MDM_T02") => parse_schema_json(include_str!("../schemas/v2_6/MDM_T02.json")),
        ("v2_6", "MDM_T04") => parse_schema_json(include_str!("../schemas/v2_6/MDM_T04.json")),
        ("v2_6", "DFT_P03") => parse_schema_json(include_str!("../schemas/v2_6/DFT_P03.json")),
        ("v2_6", "DFT_P11") => parse_schema_json(include_str!("../schemas/v2_6/DFT_P11.json")),
        ("v2_6", "QRY_A19") => parse_schema_json(include_str!("../schemas/v2_6/QRY_A19.json")),
        ("v2_6", "QRY_Q01") => parse_schema_json(include_str!("../schemas/v2_6/QRY_Q01.json")),
        ("v2_6", "QRY_Q02") => parse_schema_json(include_str!("../schemas/v2_6/QRY_Q02.json")),
        ("v2_6", "BAR_P01") => parse_schema_json(include_str!("../schemas/v2_6/BAR_P01.json")),
        ("v2_6", "BAR_P02") => parse_schema_json(include_str!("../schemas/v2_6/BAR_P02.json")),
        ("v2_6", "RDE_O11") => parse_schema_json(include_str!("../schemas/v2_6/RDE_O11.json")),
        ("v2_6", "RAS_O17") => parse_schema_json(include_str!("../schemas/v2_6/RAS_O17.json")),
        ("v2_6", "MFN_M01") => parse_schema_json(include_str!("../schemas/v2_6/MFN_M01.json")),
        ("v2_6", "RDS_O13") => parse_schema_json(include_str!("../schemas/v2_6/RDS_O13.json")),
        ("v2_6", "RGV_O15") => parse_schema_json(include_str!("../schemas/v2_6/RGV_O15.json")),
        ("v2_6", "RRD_O14") => parse_schema_json(include_str!("../schemas/v2_6/RRD_O14.json")),
        ("v2_6", "RRA_O18") => parse_schema_json(include_str!("../schemas/v2_6/RRA_O18.json")),
        ("v2_6", "OUL_R21") => parse_schema_json(include_str!("../schemas/v2_6/OUL_R21.json")),
        ("v2_6", "OML_O21") => parse_schema_json(include_str!("../schemas/v2_6/OML_O21.json")),
        ("v2_6", "VXU_V04") => parse_schema_json(include_str!("../schemas/v2_6/VXU_V04.json")),
        ("v2_6", "PPR_PC1") => parse_schema_json(include_str!("../schemas/v2_6/PPR_PC1.json")),
        ("v2_6", "OMP_O09") => parse_schema_json(include_str!("../schemas/v2_6/OMP_O09.json")),

        // V2.7 schemas - ADT messages
        ("v2_7", "ADT_A01") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A01.json")),
        ("v2_7", "ADT_A02") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A02.json")),
        ("v2_7", "ADT_A03") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A03.json")),
        ("v2_7", "ADT_A04") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A04.json")),
        ("v2_7", "ADT_A05") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A05.json")),
        ("v2_7", "ADT_A06") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A06.json")),
        ("v2_7", "ADT_A07") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A07.json")),
        ("v2_7", "ADT_A08") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A08.json")),
        ("v2_7", "ADT_A09") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A09.json")),
        ("v2_7", "ADT_A10") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A10.json")),
        ("v2_7", "ADT_A11") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A11.json")),
        ("v2_7", "ADT_A12") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A12.json")),
        ("v2_7", "ADT_A13") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A13.json")),
        ("v2_7", "ADT_A17") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A17.json")),
        ("v2_7", "ADT_A28") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A28.json")),
        ("v2_7", "ADT_A31") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A31.json")),
        ("v2_7", "ADT_A34") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A34.json")),
        ("v2_7", "ADT_A40") => parse_schema_json(include_str!("../schemas/v2_7/ADT_A40.json")),
        // V2.7 - Other message types
        ("v2_7", "ORU_R01") => parse_schema_json(include_str!("../schemas/v2_7/ORU_R01.json")),
        ("v2_7", "ORM_O01") => parse_schema_json(include_str!("../schemas/v2_7/ORM_O01.json")),
        ("v2_7", "ACK") => parse_schema_json(include_str!("../schemas/v2_7/ACK.json")),
        ("v2_7", "SIU_S12") => parse_schema_json(include_str!("../schemas/v2_7/SIU_S12.json")),
        ("v2_7", "SIU_S13") => parse_schema_json(include_str!("../schemas/v2_7/SIU_S13.json")),
        ("v2_7", "SIU_S14") => parse_schema_json(include_str!("../schemas/v2_7/SIU_S14.json")),
        ("v2_7", "SIU_S15") => parse_schema_json(include_str!("../schemas/v2_7/SIU_S15.json")),
        ("v2_7", "MDM_T01") => parse_schema_json(include_str!("../schemas/v2_7/MDM_T01.json")),
        ("v2_7", "MDM_T02") => parse_schema_json(include_str!("../schemas/v2_7/MDM_T02.json")),
        ("v2_7", "MDM_T04") => parse_schema_json(include_str!("../schemas/v2_7/MDM_T04.json")),
        ("v2_7", "DFT_P03") => parse_schema_json(include_str!("../schemas/v2_7/DFT_P03.json")),
        ("v2_7", "DFT_P11") => parse_schema_json(include_str!("../schemas/v2_7/DFT_P11.json")),
        ("v2_7", "QRY_A19") => parse_schema_json(include_str!("../schemas/v2_7/QRY_A19.json")),
        ("v2_7", "QRY_Q01") => parse_schema_json(include_str!("../schemas/v2_7/QRY_Q01.json")),
        ("v2_7", "QRY_Q02") => parse_schema_json(include_str!("../schemas/v2_7/QRY_Q02.json")),
        ("v2_7", "BAR_P01") => parse_schema_json(include_str!("../schemas/v2_7/BAR_P01.json")),
        ("v2_7", "BAR_P02") => parse_schema_json(include_str!("../schemas/v2_7/BAR_P02.json")),
        ("v2_7", "RDE_O11") => parse_schema_json(include_str!("../schemas/v2_7/RDE_O11.json")),
        ("v2_7", "RAS_O17") => parse_schema_json(include_str!("../schemas/v2_7/RAS_O17.json")),
        ("v2_7", "MFN_M01") => parse_schema_json(include_str!("../schemas/v2_7/MFN_M01.json")),
        ("v2_7", "RDS_O13") => parse_schema_json(include_str!("../schemas/v2_7/RDS_O13.json")),
        ("v2_7", "RGV_O15") => parse_schema_json(include_str!("../schemas/v2_7/RGV_O15.json")),
        ("v2_7", "RRD_O14") => parse_schema_json(include_str!("../schemas/v2_7/RRD_O14.json")),
        ("v2_7", "RRA_O18") => parse_schema_json(include_str!("../schemas/v2_7/RRA_O18.json")),
        ("v2_7", "OUL_R21") => parse_schema_json(include_str!("../schemas/v2_7/OUL_R21.json")),
        ("v2_7", "OML_O21") => parse_schema_json(include_str!("../schemas/v2_7/OML_O21.json")),
        ("v2_7", "VXU_V04") => parse_schema_json(include_str!("../schemas/v2_7/VXU_V04.json")),
        ("v2_7", "PPR_PC1") => parse_schema_json(include_str!("../schemas/v2_7/PPR_PC1.json")),
        ("v2_7", "OMP_O09") => parse_schema_json(include_str!("../schemas/v2_7/OMP_O09.json")),

        // V2.8 schemas - ADT messages
        ("v2_8", "ADT_A01") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A01.json")),
        ("v2_8", "ADT_A02") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A02.json")),
        ("v2_8", "ADT_A03") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A03.json")),
        ("v2_8", "ADT_A04") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A04.json")),
        ("v2_8", "ADT_A05") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A05.json")),
        ("v2_8", "ADT_A06") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A06.json")),
        ("v2_8", "ADT_A07") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A07.json")),
        ("v2_8", "ADT_A08") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A08.json")),
        ("v2_8", "ADT_A09") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A09.json")),
        ("v2_8", "ADT_A10") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A10.json")),
        ("v2_8", "ADT_A11") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A11.json")),
        ("v2_8", "ADT_A12") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A12.json")),
        ("v2_8", "ADT_A13") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A13.json")),
        ("v2_8", "ADT_A17") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A17.json")),
        ("v2_8", "ADT_A28") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A28.json")),
        ("v2_8", "ADT_A31") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A31.json")),
        ("v2_8", "ADT_A34") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A34.json")),
        ("v2_8", "ADT_A40") => parse_schema_json(include_str!("../schemas/v2_8/ADT_A40.json")),
        // V2.8 - Other message types
        ("v2_8", "ORU_R01") => parse_schema_json(include_str!("../schemas/v2_8/ORU_R01.json")),
        ("v2_8", "ORM_O01") => parse_schema_json(include_str!("../schemas/v2_8/ORM_O01.json")),
        ("v2_8", "ACK") => parse_schema_json(include_str!("../schemas/v2_8/ACK.json")),
        ("v2_8", "SIU_S12") => parse_schema_json(include_str!("../schemas/v2_8/SIU_S12.json")),
        ("v2_8", "SIU_S13") => parse_schema_json(include_str!("../schemas/v2_8/SIU_S13.json")),
        ("v2_8", "SIU_S14") => parse_schema_json(include_str!("../schemas/v2_8/SIU_S14.json")),
        ("v2_8", "SIU_S15") => parse_schema_json(include_str!("../schemas/v2_8/SIU_S15.json")),
        ("v2_8", "MDM_T01") => parse_schema_json(include_str!("../schemas/v2_8/MDM_T01.json")),
        ("v2_8", "MDM_T02") => parse_schema_json(include_str!("../schemas/v2_8/MDM_T02.json")),
        ("v2_8", "MDM_T04") => parse_schema_json(include_str!("../schemas/v2_8/MDM_T04.json")),
        ("v2_8", "DFT_P03") => parse_schema_json(include_str!("../schemas/v2_8/DFT_P03.json")),
        ("v2_8", "DFT_P11") => parse_schema_json(include_str!("../schemas/v2_8/DFT_P11.json")),
        ("v2_8", "QRY_A19") => parse_schema_json(include_str!("../schemas/v2_8/QRY_A19.json")),
        ("v2_8", "QRY_Q01") => parse_schema_json(include_str!("../schemas/v2_8/QRY_Q01.json")),
        ("v2_8", "QRY_Q02") => parse_schema_json(include_str!("../schemas/v2_8/QRY_Q02.json")),
        ("v2_8", "BAR_P01") => parse_schema_json(include_str!("../schemas/v2_8/BAR_P01.json")),
        ("v2_8", "BAR_P02") => parse_schema_json(include_str!("../schemas/v2_8/BAR_P02.json")),
        ("v2_8", "RDE_O11") => parse_schema_json(include_str!("../schemas/v2_8/RDE_O11.json")),
        ("v2_8", "RAS_O17") => parse_schema_json(include_str!("../schemas/v2_8/RAS_O17.json")),
        ("v2_8", "MFN_M01") => parse_schema_json(include_str!("../schemas/v2_8/MFN_M01.json")),
        ("v2_8", "RDS_O13") => parse_schema_json(include_str!("../schemas/v2_8/RDS_O13.json")),
        ("v2_8", "RGV_O15") => parse_schema_json(include_str!("../schemas/v2_8/RGV_O15.json")),
        ("v2_8", "RRD_O14") => parse_schema_json(include_str!("../schemas/v2_8/RRD_O14.json")),
        ("v2_8", "RRA_O18") => parse_schema_json(include_str!("../schemas/v2_8/RRA_O18.json")),
        ("v2_8", "OUL_R21") => parse_schema_json(include_str!("../schemas/v2_8/OUL_R21.json")),
        ("v2_8", "OML_O21") => parse_schema_json(include_str!("../schemas/v2_8/OML_O21.json")),
        ("v2_8", "VXU_V04") => parse_schema_json(include_str!("../schemas/v2_8/VXU_V04.json")),
        ("v2_8", "PPR_PC1") => parse_schema_json(include_str!("../schemas/v2_8/PPR_PC1.json")),
        ("v2_8", "OMP_O09") => parse_schema_json(include_str!("../schemas/v2_8/OMP_O09.json")),

        _ => None,
    }
}

/// Parse a JSON schema string into a MessageSchema
fn parse_schema_json(json: &str) -> Option<MessageSchema> {
    serde_json::from_str(json).ok()
}

/// Get a list of all available schemas
pub fn list_available_schemas(version: Version) -> Vec<String> {
    let schemas = vec![
        // ADT messages
        "ADT^A01".to_string(), "ADT^A02".to_string(), "ADT^A03".to_string(),
        "ADT^A04".to_string(), "ADT^A05".to_string(), "ADT^A06".to_string(),
        "ADT^A07".to_string(), "ADT^A08".to_string(), "ADT^A09".to_string(),
        "ADT^A10".to_string(), "ADT^A11".to_string(), "ADT^A12".to_string(),
        "ADT^A13".to_string(), "ADT^A17".to_string(), "ADT^A28".to_string(),
        "ADT^A31".to_string(), "ADT^A34".to_string(), "ADT^A40".to_string(),
        // Order messages
        "ORU^R01".to_string(),
        "ORM^O01".to_string(),
        // Acknowledgment
        "ACK".to_string(),
        // Scheduling
        "SIU^S12".to_string(), "SIU^S13".to_string(), "SIU^S14".to_string(), "SIU^S15".to_string(),
        // Medical Document Management
        "MDM^T01".to_string(), "MDM^T02".to_string(), "MDM^T04".to_string(),
        // Financial
        "DFT^P03".to_string(), "DFT^P11".to_string(),
        // Query
        "QRY^A19".to_string(), "QRY^Q01".to_string(), "QRY^Q02".to_string(),
        // Billing
        "BAR^P01".to_string(), "BAR^P02".to_string(),
        // Pharmacy
        "RDE^O11".to_string(), "RAS^O17".to_string(), "RDS^O13".to_string(),
        "RGV^O15".to_string(), "RRD^O14".to_string(), "RRA^O18".to_string(),
        // Laboratory
        "OUL^R21".to_string(), "OML^O21".to_string(),
        // Master File
        "MFN^M01".to_string(),
        // Immunization
        "VXU^V04".to_string(),
        // Patient Problem
        "PPR^PC1".to_string(),
        // Pharmacy Order
        "OMP^O09".to_string(),
    ];

    // All versions currently have the same set of schemas
    // Legacy versions (2.1, 2.2) use v2.3 schemas
    match version {
        Version::V2_1 | Version::V2_2 => schemas,
        Version::V2_3 | Version::V2_3_1 => schemas,
        Version::V2_4 => schemas,
        Version::V2_5 | Version::V2_5_1 => schemas,
        Version::V2_6 => schemas,
        Version::V2_7 | Version::V2_7_1 => schemas,
        Version::V2_8 | Version::V2_8_1 | Version::V2_8_2 => schemas,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_adt_a01_schema() {
        let schema = load_schema(Version::V2_5, "ADT", "A01").unwrap();
        assert_eq!(schema.message_type, "ADT");
        assert_eq!(schema.trigger_event, "A01");
        assert_eq!(schema.version, "2.5");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("PID"));
    }

    #[test]
    fn test_load_adt_a34_schema() {
        let schema = load_schema(Version::V2_5, "ADT", "A34").unwrap();
        assert_eq!(schema.message_type, "ADT");
        assert_eq!(schema.trigger_event, "A34");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("EVN"));
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("MRG"));
        assert!(schema.segments["MRG"].required);
        // A34 merges patient ID only: no visit segments.
        assert!(!schema.segments.contains_key("PV1"));
    }

    #[test]
    fn test_load_adt_a34_all_versions() {
        for version in [
            Version::V2_3,
            Version::V2_4,
            Version::V2_5,
            Version::V2_6,
            Version::V2_7,
            Version::V2_8,
        ] {
            let schema = load_schema(version, "ADT", "A34").unwrap();
            assert_eq!(schema.message_type, "ADT");
            assert_eq!(schema.trigger_event, "A34");
            assert!(schema.segments.contains_key("MRG"));
        }
        assert!(list_available_schemas(Version::V2_5).contains(&"ADT^A34".to_string()));
    }

    #[test]
    fn test_load_oru_r01_schema() {
        let schema = load_schema(Version::V2_5, "ORU", "R01").unwrap();
        assert_eq!(schema.message_type, "ORU");
        assert!(schema.segments.contains_key("OBR"));
        assert!(schema.segments.contains_key("OBX"));
    }

    #[test]
    fn test_load_ack_schema() {
        let schema = load_schema(Version::V2_5, "ACK", "").unwrap();
        assert_eq!(schema.message_type, "ACK");
        assert!(schema.segments.contains_key("MSA"));
    }

    #[test]
    fn test_load_nonexistent_schema() {
        let result = load_schema(Version::V2_5, "INVALID", "XXX");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_available_schemas() {
        let schemas = list_available_schemas(Version::V2_5);
        assert!(schemas.contains(&"ADT^A01".to_string()));
        assert!(schemas.contains(&"ORU^R01".to_string()));
        assert!(schemas.len() >= 3);
    }

    #[test]
    fn test_load_bar_p01_schema() {
        let schema = load_schema(Version::V2_5, "BAR", "P01").unwrap();
        assert_eq!(schema.message_type, "BAR");
        assert_eq!(schema.trigger_event, "P01");
        assert_eq!(schema.version, "2.5");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("EVN"));
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("PV1"));
    }

    #[test]
    fn test_load_rde_o11_schema() {
        let schema = load_schema(Version::V2_5, "RDE", "O11").unwrap();
        assert_eq!(schema.message_type, "RDE");
        assert_eq!(schema.trigger_event, "O11");
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("RXE"));
    }

    #[test]
    fn test_load_ras_o17_schema() {
        let schema = load_schema(Version::V2_5, "RAS", "O17").unwrap();
        assert_eq!(schema.message_type, "RAS");
        assert_eq!(schema.trigger_event, "O17");
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("RXA"));
    }

    #[test]
    fn test_load_mfn_m01_schema() {
        let schema = load_schema(Version::V2_5, "MFN", "M01").unwrap();
        assert_eq!(schema.message_type, "MFN");
        assert_eq!(schema.trigger_event, "M01");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("MFI"));
        assert!(schema.segments.contains_key("MFE"));
    }

    #[test]
    fn test_load_rds_o13_schema() {
        let schema = load_schema(Version::V2_5, "RDS", "O13").unwrap();
        assert_eq!(schema.message_type, "RDS");
        assert_eq!(schema.trigger_event, "O13");
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("RXD"));
    }

    #[test]
    fn test_load_new_schemas_all_versions() {
        // Test BAR_P01 across all versions
        for version in [Version::V2_3, Version::V2_4, Version::V2_5, Version::V2_6, Version::V2_7, Version::V2_8] {
            let schema = load_schema(version, "BAR", "P01").unwrap();
            assert_eq!(schema.message_type, "BAR");
            assert_eq!(schema.trigger_event, "P01");
        }

        // Test RDE_O11 across all versions
        for version in [Version::V2_3, Version::V2_4, Version::V2_5, Version::V2_6, Version::V2_7, Version::V2_8] {
            let schema = load_schema(version, "RDE", "O11").unwrap();
            assert_eq!(schema.message_type, "RDE");
            assert_eq!(schema.trigger_event, "O11");
        }
    }

    #[test]
    fn test_load_rgv_o15_schema() {
        let schema = load_schema(Version::V2_5, "RGV", "O15").unwrap();
        assert_eq!(schema.message_type, "RGV");
        assert_eq!(schema.trigger_event, "O15");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("RXG"));
    }

    #[test]
    fn test_load_rrd_o14_schema() {
        let schema = load_schema(Version::V2_5, "RRD", "O14").unwrap();
        assert_eq!(schema.message_type, "RRD");
        assert_eq!(schema.trigger_event, "O14");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("RXD"));
    }

    #[test]
    fn test_load_rra_o18_schema() {
        let schema = load_schema(Version::V2_5, "RRA", "O18").unwrap();
        assert_eq!(schema.message_type, "RRA");
        assert_eq!(schema.trigger_event, "O18");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("MSA"));
    }

    #[test]
    fn test_load_vxu_v04_schema() {
        let schema = load_schema(Version::V2_5, "VXU", "V04").unwrap();
        assert_eq!(schema.message_type, "VXU");
        assert_eq!(schema.trigger_event, "V04");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("RXA"));
        assert!(schema.segments.contains_key("RXR"));
        // PV1 is optional in VXU (unlike the pharmacy administration messages)
        assert!(!schema.segments["PV1"].required);
    }

    #[test]
    fn test_load_vxu_v04_all_versions() {
        for version in [
            Version::V2_3,
            Version::V2_4,
            Version::V2_5,
            Version::V2_6,
            Version::V2_7,
            Version::V2_8,
        ] {
            let schema = load_schema(version, "VXU", "V04").unwrap();
            assert_eq!(schema.message_type, "VXU");
            assert_eq!(schema.trigger_event, "V04");
            assert!(schema.segments.contains_key("RXA"));
        }
        assert!(list_available_schemas(Version::V2_5).contains(&"VXU^V04".to_string()));
    }

    #[test]
    fn test_load_ppr_pc1_schema() {
        let schema = load_schema(Version::V2_5, "PPR", "PC1").unwrap();
        assert_eq!(schema.message_type, "PPR");
        assert_eq!(schema.trigger_event, "PC1");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("PV1"));
        assert!(schema.segments.contains_key("PRB"));
        assert!(schema.segments.contains_key("VAR"));
        assert!(schema.segments["PRB"].required);
        assert!(schema.segments["PRB"].repeating);
    }

    #[test]
    fn test_load_ppr_pc1_all_versions() {
        for version in [
            Version::V2_3,
            Version::V2_4,
            Version::V2_5,
            Version::V2_6,
            Version::V2_7,
            Version::V2_8,
        ] {
            let schema = load_schema(version, "PPR", "PC1").unwrap();
            assert_eq!(schema.message_type, "PPR");
            assert_eq!(schema.trigger_event, "PC1");
            assert!(schema.segments.contains_key("PRB"));
        }
        assert!(list_available_schemas(Version::V2_5).contains(&"PPR^PC1".to_string()));
    }

    #[test]
    fn test_load_omp_o09_schema() {
        let schema = load_schema(Version::V2_5, "OMP", "O09").unwrap();
        assert_eq!(schema.message_type, "OMP");
        assert_eq!(schema.trigger_event, "O09");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("RXO"));
        assert!(schema.segments.contains_key("RXR"));
        assert!(schema.segments["RXO"].required);
        // RXR is required in OMP orders (unlike the administration messages)
        assert!(schema.segments["RXR"].required);
    }

    #[test]
    fn test_load_omp_o09_all_versions() {
        for version in [
            Version::V2_3,
            Version::V2_4,
            Version::V2_5,
            Version::V2_6,
            Version::V2_7,
            Version::V2_8,
        ] {
            let schema = load_schema(version, "OMP", "O09").unwrap();
            assert_eq!(schema.message_type, "OMP");
            assert_eq!(schema.trigger_event, "O09");
            assert!(schema.segments.contains_key("RXO"));
        }
        assert!(list_available_schemas(Version::V2_5).contains(&"OMP^O09".to_string()));
    }

    #[test]
    fn test_load_pharmacy_schemas_all_versions() {
        // Test RGV_O15 across all versions
        for version in [Version::V2_3, Version::V2_4, Version::V2_5, Version::V2_6, Version::V2_7, Version::V2_8] {
            let schema = load_schema(version, "RGV", "O15").unwrap();
            assert_eq!(schema.message_type, "RGV");
            assert_eq!(schema.trigger_event, "O15");
        }

        // Test RRD_O14 across all versions
        for version in [Version::V2_3, Version::V2_4, Version::V2_5, Version::V2_6, Version::V2_7, Version::V2_8] {
            let schema = load_schema(version, "RRD", "O14").unwrap();
            assert_eq!(schema.message_type, "RRD");
            assert_eq!(schema.trigger_event, "O14");
        }

        // Test RRA_O18 across all versions
        for version in [Version::V2_3, Version::V2_4, Version::V2_5, Version::V2_6, Version::V2_7, Version::V2_8] {
            let schema = load_schema(version, "RRA", "O18").unwrap();
            assert_eq!(schema.message_type, "RRA");
            assert_eq!(schema.trigger_event, "O18");
        }
    }

    #[test]
    fn test_load_oul_r21_schema() {
        let schema = load_schema(Version::V2_5, "OUL", "R21").unwrap();
        assert_eq!(schema.message_type, "OUL");
        assert_eq!(schema.trigger_event, "R21");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("OBR"));
        assert!(schema.segments.contains_key("OBX"));
        assert!(schema.segments.contains_key("SPM"));
    }

    #[test]
    fn test_load_oml_o21_schema() {
        let schema = load_schema(Version::V2_5, "OML", "O21").unwrap();
        assert_eq!(schema.message_type, "OML");
        assert_eq!(schema.trigger_event, "O21");
        assert!(schema.segments.contains_key("MSH"));
        assert!(schema.segments.contains_key("PID"));
        assert!(schema.segments.contains_key("ORC"));
        assert!(schema.segments.contains_key("OBR"));
        assert!(schema.segments.contains_key("SPM"));
    }

    #[test]
    fn test_load_laboratory_schemas_all_versions() {
        // Test OUL_R21 across all versions
        for version in [Version::V2_3, Version::V2_4, Version::V2_5, Version::V2_6, Version::V2_7, Version::V2_8] {
            let schema = load_schema(version, "OUL", "R21").unwrap();
            assert_eq!(schema.message_type, "OUL");
            assert_eq!(schema.trigger_event, "R21");
        }

        // Test OML_O21 across all versions
        for version in [Version::V2_3, Version::V2_4, Version::V2_5, Version::V2_6, Version::V2_7, Version::V2_8] {
            let schema = load_schema(version, "OML", "O21").unwrap();
            assert_eq!(schema.message_type, "OML");
            assert_eq!(schema.trigger_event, "O21");
        }
    }

    #[test]
    fn test_load_v28_schema_with_new_fields() {
        let schema = load_schema(Version::V2_8, "ADT", "A01").unwrap();
        assert_eq!(schema.version, "2.8");

        // v2.8 MSH should have 25 fields (4 new: MSH-22 through MSH-25)
        let msh = schema.segments.get("MSH").unwrap();
        assert!(msh.fields.contains_key(&22)); // Sending Responsible Organization
        assert!(msh.fields.contains_key(&25)); // Receiving Network Address

        // v2.8 timestamps should use DTM instead of TS
        assert_eq!(msh.fields.get(&7).unwrap().data_type, "DTM");

        // v2.8 should have completed CE->CWE migration
        let pid = schema.segments.get("PID").unwrap();
        assert_eq!(pid.fields.get(&10).unwrap().data_type, "CWE"); // Race
        assert_eq!(pid.fields.get(&15).unwrap().data_type, "CWE"); // Primary Language
    }

    #[test]
    fn test_load_v28_oru_with_obx_new_fields() {
        let schema = load_schema(Version::V2_8, "ORU", "R01").unwrap();
        let obx = schema.segments.get("OBX").unwrap();

        // OBX-20 through OBX-25 added in v2.8
        assert!(obx.fields.contains_key(&20)); // Observation Site
        assert!(obx.fields.contains_key(&23)); // Performing Organization Name
        assert!(obx.fields.contains_key(&25)); // Performing Organization Medical Director
        assert_eq!(obx.fields.len(), 25);
    }

    #[test]
    fn test_v28_fallback_for_v281_v282() {
        // v2.8.1 and v2.8.2 should use v2.8 schemas
        let schema_281 = load_schema(Version::V2_8_1, "ADT", "A01").unwrap();
        assert_eq!(schema_281.version, "2.8");

        let schema_282 = load_schema(Version::V2_8_2, "ADT", "A01").unwrap();
        assert_eq!(schema_282.version, "2.8");
    }
}
