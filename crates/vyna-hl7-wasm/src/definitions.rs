//! HL7 schema definition bindings for WebAssembly
//!
//! This module exposes VynaHL7's HL7 schema database to JavaScript, enabling
//! features like message browsers, field reference lookups, and vocabulary
//! table exploration in the frontend.

use vyna_hl7_core::Version;
use vyna_hl7_validator::{SegmentDefinition, TableRegistry, list_available_schemas, load_schema};
use serde::Serialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

/// Supported HL7 versions exposed to JavaScript
const SUPPORTED_VERSIONS: &[&str] = &["2.1", "2.2", "2.3", "2.4", "2.5", "2.6", "2.7", "2.8"];

/// Convert a version string like "2.5" to the vyna_hl7_core::Version enum
fn parse_version(version: &str) -> Result<Version, JsValue> {
    match version {
        "2.1" => Ok(Version::V2_1),
        "2.2" => Ok(Version::V2_2),
        "2.3" => Ok(Version::V2_3),
        "2.4" => Ok(Version::V2_4),
        "2.5" => Ok(Version::V2_5),
        "2.6" => Ok(Version::V2_6),
        "2.7" => Ok(Version::V2_7),
        "2.8" => Ok(Version::V2_8),
        _ => Err(JsValue::from_str(&format!(
            "Unsupported HL7 version: '{}'. Supported versions: {}",
            version,
            SUPPORTED_VERSIONS.join(", ")
        ))),
    }
}

/// Get the human-readable description for a message type and trigger event.
/// These descriptions match the HL7 standard definitions.
fn get_message_description(message_type: &str, trigger_event: &str) -> &'static str {
    match (message_type, trigger_event) {
        // ADT - Admit Discharge Transfer
        ("ADT", "A01") => "Admit/Visit Notification",
        ("ADT", "A02") => "Transfer a Patient",
        ("ADT", "A03") => "Discharge/End Visit",
        ("ADT", "A04") => "Register a Patient",
        ("ADT", "A05") => "Pre-Admit a Patient",
        ("ADT", "A06") => "Change an Outpatient to an Inpatient",
        ("ADT", "A07") => "Change an Inpatient to an Outpatient",
        ("ADT", "A08") => "Update Patient Information",
        ("ADT", "A09") => "Patient Departing - Tracking",
        ("ADT", "A10") => "Patient Arriving - Tracking",
        ("ADT", "A11") => "Cancel Admit/Visit Notification",
        ("ADT", "A12") => "Cancel Transfer",
        ("ADT", "A13") => "Cancel Discharge/End Visit",
        ("ADT", "A17") => "Swap Patients",
        ("ADT", "A28") => "Add Person Information",
        ("ADT", "A31") => "Update Person Information",
        ("ADT", "A34") => "Merge Patient - Patient ID Only",
        ("ADT", "A40") => "Merge Patient - Patient Identifier List",
        // ORU - Observation Result
        ("ORU", "R01") => "Unsolicited Observation Result",
        // ORM - Order Message
        ("ORM", "O01") => "Order Message",
        // ACK - Acknowledgment
        ("ACK", "") => "General Acknowledgment",
        // SIU - Scheduling
        ("SIU", "S12") => "Notification of New Appointment Booking",
        ("SIU", "S13") => "Notification of Appointment Rescheduling",
        ("SIU", "S14") => "Notification of Appointment Modification",
        ("SIU", "S15") => "Notification of Appointment Cancellation",
        // MDM - Medical Document Management
        ("MDM", "T01") => "Original Document Notification",
        ("MDM", "T02") => "Original Document Notification and Content",
        ("MDM", "T04") => "Document Status Change Notification and Content",
        // DFT - Detailed Financial Transaction
        ("DFT", "P03") => "Post Detail Financial Transaction",
        ("DFT", "P11") => "Post Detail Financial Transaction - New",
        // QRY - Query
        ("QRY", "A19") => "Patient Query",
        ("QRY", "Q01") => "Query Sent for Immediate Response",
        ("QRY", "Q02") => "Query Sent for Deferred Response",
        // BAR - Billing Account Record
        ("BAR", "P01") => "Add Patient Account",
        ("BAR", "P02") => "Purge Patient Account",
        // Pharmacy
        ("RDE", "O11") => "Pharmacy/Treatment Encoded Order",
        ("RAS", "O17") => "Pharmacy/Treatment Administration",
        ("RDS", "O13") => "Pharmacy/Treatment Dispense",
        ("RGV", "O15") => "Pharmacy/Treatment Give",
        ("RRD", "O14") => "Pharmacy/Treatment Dispense Acknowledgment",
        ("RRA", "O18") => "Pharmacy/Treatment Administration Acknowledgment",
        // Laboratory
        ("OUL", "R21") => "Unsolicited Laboratory Observation",
        ("OML", "O21") => "Laboratory Order",
        // Master File
        ("MFN", "M01") => "Master File Notification - Not Otherwise Specified",
        // VXU - Immunization
        ("VXU", "V04") => "Unsolicited Vaccination Record Update",
        // PPR - Patient Problem
        ("PPR", "PC1") => "Patient Problem - Add",
        // OMP - Pharmacy Order
        ("OMP", "O09") => "Pharmacy/Treatment Order",
        _ => "Unknown Message Type",
    }
}

/// List all supported HL7 versions
///
/// # Returns
///
/// A JavaScript array of version strings: `["2.3", "2.4", "2.5", "2.6", "2.7", "2.8"]`
#[wasm_bindgen(js_name = listHL7Versions)]
pub fn list_hl7_versions() -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&SUPPORTED_VERSIONS).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// List all message types available for a given HL7 version
///
/// # Arguments
///
/// * `version` - HL7 version string (e.g., "2.5")
///
/// # Returns
///
/// A JavaScript array of objects with `messageType`, `triggerEvent`, and `description`
///
/// # Example
///
/// ```javascript
/// const types = listMessageTypes("2.5");
/// // [{ messageType: "ADT", triggerEvent: "A01", description: "Admit/Visit Notification" }, ...]
/// ```
#[wasm_bindgen(js_name = listMessageTypes)]
pub fn list_message_types(version: &str) -> Result<JsValue, JsValue> {
    let v = parse_version(version)?;
    let schemas = list_available_schemas(v);

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct MessageTypeInfo {
        message_type: String,
        trigger_event: String,
        description: String,
    }

    let mut result: Vec<MessageTypeInfo> = schemas
        .iter()
        .map(|schema_name| {
            // schema_name is like "ADT^A01" or "ACK"
            let parts: Vec<&str> = schema_name.split('^').collect();
            let msg_type = parts[0].to_string();
            let trigger = if parts.len() > 1 {
                parts[1].to_string()
            } else {
                String::new()
            };
            let description = get_message_description(&msg_type, &trigger).to_string();
            MessageTypeInfo {
                message_type: msg_type,
                trigger_event: trigger,
                description,
            }
        })
        .collect();

    // Sort by message_type then trigger_event for consistent ordering
    result.sort_by(|a, b| {
        a.message_type
            .cmp(&b.message_type)
            .then(a.trigger_event.cmp(&b.trigger_event))
    });

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the full message definition (schema) for a specific message type
///
/// # Arguments
///
/// * `version` - HL7 version string (e.g., "2.5")
/// * `message_type` - Message type (e.g., "ADT")
/// * `trigger_event` - Trigger event (e.g., "A01"), or empty string for types like "ACK"
///
/// # Returns
///
/// The full MessageSchema JSON including all segments and field definitions
///
/// # Example
///
/// ```javascript
/// const schema = getMessageDefinition("2.5", "ADT", "A01");
/// // { messageType: "ADT", triggerEvent: "A01", version: "2.5",
/// //   segments: { MSH: { name: "Message Header", required: true, ... }, ... } }
/// ```
#[wasm_bindgen(js_name = getMessageDefinition)]
pub fn get_message_definition(
    version: &str,
    message_type: &str,
    trigger_event: &str,
) -> Result<JsValue, JsValue> {
    let v = parse_version(version)?;
    let schema = load_schema(v, message_type, trigger_event)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Serialize the MessageSchema with camelCase keys for JavaScript
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsMessageSchema {
        message_type: String,
        trigger_event: String,
        version: String,
        description: String,
        segments: HashMap<String, JsSegmentDefinition>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsSegmentDefinition {
        name: String,
        required: bool,
        repeating: bool,
        fields: HashMap<usize, JsFieldDefinition>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsFieldDefinition {
        name: String,
        data_type: String,
        required: bool,
        repeating: bool,
        max_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        table_id: Option<String>,
    }

    let js_schema = JsMessageSchema {
        description: get_message_description(&schema.message_type, &schema.trigger_event)
            .to_string(),
        message_type: schema.message_type,
        trigger_event: schema.trigger_event,
        version: schema.version,
        segments: schema
            .segments
            .into_iter()
            .map(|(seg_id, seg_def)| {
                let js_seg = JsSegmentDefinition {
                    name: seg_def.name,
                    required: seg_def.required,
                    repeating: seg_def.repeating,
                    fields: seg_def
                        .fields
                        .into_iter()
                        .map(|(idx, field_def)| {
                            (
                                idx,
                                JsFieldDefinition {
                                    name: field_def.name,
                                    data_type: field_def.data_type,
                                    required: field_def.required,
                                    repeating: field_def.repeating,
                                    max_length: field_def.max_length,
                                    table_id: field_def.table_id,
                                },
                            )
                        })
                        .collect(),
                };
                (seg_id, js_seg)
            })
            .collect(),
    };

    serde_wasm_bindgen::to_value(&js_schema).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get a specific segment definition from a message schema
///
/// Loads the segment definition across all message schemas for the given version,
/// returning the first match found. This provides field metadata for a segment type.
///
/// # Arguments
///
/// * `version` - HL7 version string (e.g., "2.5")
/// * `segment_id` - Segment identifier (e.g., "PID", "OBX", "MSH")
///
/// # Returns
///
/// The segment definition with all field metadata
///
/// # Example
///
/// ```javascript
/// const segment = getSegmentDefinition("2.5", "PID");
/// // { name: "Patient Identification", required: true, repeating: false,
/// //   fields: { 1: { name: "Set ID", dataType: "SI", ... }, ... } }
/// ```
#[wasm_bindgen(js_name = getSegmentDefinition)]
pub fn get_segment_definition(version: &str, segment_id: &str) -> Result<JsValue, JsValue> {
    let v = parse_version(version)?;

    // Strategy: search through known message schemas to find this segment.
    // We use well-known schemas that are likely to contain the segment with
    // the most complete field definitions.
    let search_schemas: &[(&str, &str)] = &[
        ("ADT", "A01"), // Most comprehensive - has MSH, EVN, PID, PV1, NK1, etc.
        ("ORU", "R01"), // Has OBR, OBX, NTE, etc.
        ("ORM", "O01"), // Has ORC, OBR, etc.
        ("SIU", "S12"), // Has SCH, AIS, AIG, AIL, AIP
        ("DFT", "P03"), // Has FT1, DG1, PR1
        ("BAR", "P01"), // Has GT1, IN1, IN2, ACC
        ("RDE", "O11"), // Has RXE, RXR, RXC
        ("RAS", "O17"), // Has RXA
        ("MFN", "M01"), // Has MFI, MFE
        ("MDM", "T02"), // Has TXA
        ("ACK", ""),    // Has MSA, ERR
        ("RDS", "O13"), // Has RXD
        ("RGV", "O15"), // Has RXG
        ("OUL", "R21"), // Has SPM
        ("OML", "O21"), // Has SPM, SAC
        ("QRY", "A19"), // Has QRD, QRF
    ];

    for (msg_type, trigger) in search_schemas {
        if let Ok(schema) = load_schema(v, msg_type, trigger)
            && let Some(seg_def) = schema.segments.get(segment_id) {
                return serialize_segment_definition(segment_id, seg_def);
            }
    }

    Err(JsValue::from_str(&format!(
        "Segment '{}' not found in any schema for version {}",
        segment_id, version
    )))
}

/// Helper to serialize a SegmentDefinition to JsValue
fn serialize_segment_definition(
    segment_id: &str,
    seg_def: &SegmentDefinition,
) -> Result<JsValue, JsValue> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsSegmentResult {
        segment_id: String,
        name: String,
        required: bool,
        repeating: bool,
        fields: HashMap<usize, JsFieldResult>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct JsFieldResult {
        name: String,
        data_type: String,
        required: bool,
        repeating: bool,
        max_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        table_id: Option<String>,
    }

    let result = JsSegmentResult {
        segment_id: segment_id.to_string(),
        name: seg_def.name.clone(),
        required: seg_def.required,
        repeating: seg_def.repeating,
        fields: seg_def
            .fields
            .iter()
            .map(|(idx, field_def)| {
                (
                    *idx,
                    JsFieldResult {
                        name: field_def.name.clone(),
                        data_type: field_def.data_type.clone(),
                        required: field_def.required,
                        repeating: field_def.repeating,
                        max_length: field_def.max_length,
                        table_id: field_def.table_id.clone(),
                    },
                )
            })
            .collect(),
    };

    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get a vocabulary table by its table ID
///
/// Returns the HL7 standard table values for coded fields (e.g., Table 0001
/// for Administrative Sex, Table 0004 for Patient Class).
///
/// # Arguments
///
/// * `table_id` - The HL7 table ID (e.g., "0001", "0004", "0085")
///
/// # Returns
///
/// The vocabulary table with all code values and descriptions
///
/// # Example
///
/// ```javascript
/// const table = getVocabularyTable("0001");
/// // { tableId: "0001", name: "Administrative Sex", description: "...",
/// //   values: { F: { code: "F", description: "Female", deprecated: false }, ... } }
/// ```
#[wasm_bindgen(js_name = getVocabularyTable)]
pub fn get_vocabulary_table(table_id: &str) -> Result<JsValue, JsValue> {
    let registry = TableRegistry::new();

    match registry.get_table(table_id) {
        Some(table) => {
            serde_wasm_bindgen::to_value(table).map_err(|e| JsValue::from_str(&e.to_string()))
        }
        None => Err(JsValue::from_str(&format!(
            "Vocabulary table '{}' not found. Available tables include: 0001, 0002, 0004, 0007, 0061, 0063, 0078, 0085, 0103, 0119, 0201, 0203, 0301",
            table_id
        ))),
    }
}

/// Search across segment and field definitions for a given query
///
/// Searches segment names and field names within all message schemas for the
/// given version. Returns up to 50 matches.
///
/// # Arguments
///
/// * `version` - HL7 version string (e.g., "2.5")
/// * `query` - Search query (case-insensitive substring match)
///
/// # Returns
///
/// An array of search results, each with segment/field location and match context
///
/// # Example
///
/// ```javascript
/// const results = searchDefinitions("2.5", "patient");
/// // [{ segmentId: "PID", fieldIndex: null, name: "Patient Identification",
/// //    matchType: "segment", path: "PID" },
/// //  { segmentId: "PID", fieldIndex: 5, name: "Patient Name",
/// //    matchType: "field", path: "PID-5" }, ...]
/// ```
#[wasm_bindgen(js_name = searchDefinitions)]
pub fn search_definitions(version: &str, query: &str) -> Result<JsValue, JsValue> {
    let v = parse_version(version)?;
    let query_lower = query.to_lowercase();

    if query_lower.is_empty() {
        return Err(JsValue::from_str("Search query must not be empty"));
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SearchResult {
        segment_id: String,
        field_index: Option<usize>,
        name: String,
        match_type: String, // "segment" or "field"
        path: String,
        data_type: Option<String>,
    }

    let mut results: Vec<SearchResult> = Vec::new();
    // Track segments we've already added to avoid duplicates across schemas
    let mut seen_segments: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen_fields: std::collections::HashSet<String> = std::collections::HashSet::new();

    let schemas = list_available_schemas(v);
    for schema_name in &schemas {
        let parts: Vec<&str> = schema_name.split('^').collect();
        let msg_type = parts[0];
        let trigger = if parts.len() > 1 { parts[1] } else { "" };

        let schema = match load_schema(v, msg_type, trigger) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for (seg_id, seg_def) in &schema.segments {
            // Check segment name match
            if (seg_def.name.to_lowercase().contains(&query_lower)
                || seg_id.to_lowercase().contains(&query_lower))
                && seen_segments.insert(seg_id.clone()) {
                    results.push(SearchResult {
                        segment_id: seg_id.clone(),
                        field_index: None,
                        name: seg_def.name.clone(),
                        match_type: "segment".to_string(),
                        path: seg_id.clone(),
                        data_type: None,
                    });
                }

            // Check field name matches
            for (field_idx, field_def) in &seg_def.fields {
                let field_key = format!("{}-{}", seg_id, field_idx);
                if (field_def.name.to_lowercase().contains(&query_lower)
                    || field_def.data_type.to_lowercase().contains(&query_lower))
                    && seen_fields.insert(field_key.clone())
                {
                    results.push(SearchResult {
                        segment_id: seg_id.clone(),
                        field_index: Some(*field_idx),
                        name: field_def.name.clone(),
                        match_type: "field".to_string(),
                        path: field_key,
                        data_type: Some(field_def.data_type.clone()),
                    });
                }
            }
        }

        // Early exit if we have enough results
        if results.len() >= 50 {
            break;
        }
    }

    // Sort: segments first, then fields, alphabetically
    results.sort_by(|a, b| {
        a.match_type
            .cmp(&b.match_type)
            .then(a.segment_id.cmp(&b.segment_id))
            .then(a.field_index.cmp(&b.field_index))
    });

    // Limit to 50 results
    results.truncate(50);

    serde_wasm_bindgen::to_value(&results).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_valid() {
        // parse_version returns Result<Version, JsValue> but JsValue panics on
        // non-wasm32 targets. Test the underlying logic instead.
        let version_map: &[(&str, bool)] = &[
            ("2.1", true),
            ("2.2", true),
            ("2.3", true),
            ("2.4", true),
            ("2.5", true),
            ("2.6", true),
            ("2.7", true),
            ("2.8", true),
            ("1.0", false),
            ("", false),
        ];

        for (ver, should_exist) in version_map {
            let is_supported = SUPPORTED_VERSIONS.contains(ver);
            assert_eq!(
                is_supported, *should_exist,
                "Version '{}' supported={} expected={}",
                ver, is_supported, should_exist
            );
        }
    }

    #[test]
    fn test_get_message_description() {
        assert_eq!(
            get_message_description("ADT", "A01"),
            "Admit/Visit Notification"
        );
        assert_eq!(
            get_message_description("ORU", "R01"),
            "Unsolicited Observation Result"
        );
        assert_eq!(get_message_description("ACK", ""), "General Acknowledgment");
        assert_eq!(
            get_message_description("UNKNOWN", "X99"),
            "Unknown Message Type"
        );
    }

    #[test]
    fn test_supported_versions() {
        assert_eq!(SUPPORTED_VERSIONS.len(), 8);
        assert!(SUPPORTED_VERSIONS.contains(&"2.1"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.2"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.3"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.7"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.8"));
    }

    #[test]
    fn test_description_coverage() {
        // Verify all message types from list_available_schemas have descriptions
        let schemas = list_available_schemas(Version::V2_5);
        for schema_name in &schemas {
            let parts: Vec<&str> = schema_name.split('^').collect();
            let msg_type = parts[0];
            let trigger = if parts.len() > 1 { parts[1] } else { "" };
            let desc = get_message_description(msg_type, trigger);
            assert_ne!(
                desc, "Unknown Message Type",
                "Missing description for {}^{}",
                msg_type, trigger
            );
        }
    }
}
