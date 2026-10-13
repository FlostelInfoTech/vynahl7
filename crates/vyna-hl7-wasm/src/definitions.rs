//! HL7 schema definition bindings for WebAssembly
//!
//! This module exposes VynaHL7's HL7 schema database to JavaScript, enabling
//! features like message browsers, field reference lookups, and vocabulary
//! table exploration in the frontend.

use serde::Serialize;
use std::collections::HashMap;
use vyna_hl7_core::Version;
use vyna_hl7_validator::{SegmentDefinition, TableRegistry, list_available_schemas, load_schema};
use wasm_bindgen::prelude::*;

/// Supported HL7 versions exposed to JavaScript
const SUPPORTED_VERSIONS: &[&str] = &[
    "2.1", "2.2", "2.3", "2.3.1", "2.4", "2.5", "2.5.1", "2.6", "2.7", "2.7.1", "2.8", "2.8.1",
    "2.8.2",
];

/// Validate a version string without building a `JsValue`, so native
/// tests and non-wasm callers (e.g. `create_message`) can share the
/// supported-version check. Returns the parsed [`Version`].
pub(crate) fn validate_version_string(version: &str) -> Result<Version, String> {
    match version {
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
        _ => Err(format!(
            "Unsupported HL7 version: '{}'. Supported versions: {}",
            version,
            SUPPORTED_VERSIONS.join(", ")
        )),
    }
}

/// Convert a version string like "2.5" to the vyna_hl7_core::Version enum
fn parse_version(version: &str) -> Result<Version, JsValue> {
    validate_version_string(version).map_err(|e| JsValue::from_str(&e))
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
/// A JavaScript array of version strings from `"2.1"` through `"2.8.2"`,
/// including the point releases core supports.
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
        // String keys: plain JS objects only support string keys, and
        // d.ts types this as `Record<string, FieldDefinition>`.
        fields: HashMap<String, JsFieldDefinition>,
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
                                idx.to_string(),
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

    // Plain nested objects (not ES Maps) with `null` for missing
    // `max_length`, per d.ts (`MessageSchema`).
    crate::to_json_jsvalue(&js_schema)
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
/// Message schemas searched first by [`get_segment_definition`], in
/// priority order. Well-known schemas likely to hold the segment with
/// the most complete field definitions come first.
const SEARCH_SCHEMAS: &[(&str, &str)] = &[
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
    ("ADT", "A40"), // Has MRG (patient identifier merges)
    ("PPR", "PC1"), // Has PRB, GOL, PTH, VAR (problem/goal/pathway)
];

/// Find a segment definition across message schemas (pure helper).
///
/// Searches [`SEARCH_SCHEMAS`] first, then every remaining schema for
/// the version, returning the first match. Returns `None` when no
/// schema for the version contains the segment.
fn find_segment_definition(version: Version, segment_id: &str) -> Option<SegmentDefinition> {
    for (msg_type, trigger) in SEARCH_SCHEMAS {
        if let Ok(schema) = load_schema(version, msg_type, trigger)
            && let Some(seg_def) = schema.segments.get(segment_id)
        {
            return Some(seg_def.clone());
        }
    }
    // Fallback: any other schema for the version, so segments added
    // to schemas outside the curated list stay reachable.
    for schema_name in list_available_schemas(version) {
        let (msg_type, trigger) = match schema_name.split_once('^') {
            Some((t, e)) => (t, e),
            None => (schema_name.as_str(), ""),
        };
        if SEARCH_SCHEMAS.contains(&(msg_type, trigger)) {
            continue;
        }
        if let Ok(schema) = load_schema(version, msg_type, trigger)
            && let Some(seg_def) = schema.segments.get(segment_id)
        {
            return Some(seg_def.clone());
        }
    }
    None
}

#[wasm_bindgen(js_name = getSegmentDefinition)]
pub fn get_segment_definition(version: &str, segment_id: &str) -> Result<JsValue, JsValue> {
    let v = parse_version(version)?;

    if let Some(seg_def) = find_segment_definition(v, segment_id) {
        return serialize_segment_definition(segment_id, &seg_def);
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
        // String keys: plain JS objects only support string keys, and
        // d.ts types this as `Record<string, FieldDefinition>`.
        fields: HashMap<String, JsFieldResult>,
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
                    idx.to_string(),
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

    // Plain nested objects (not ES Maps) with `null` for missing
    // `max_length`, per d.ts (`SegmentResult`).
    crate::to_json_jsvalue(&result)
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
/// // { table_id: "0001", name: "Administrative Sex", description: "...",
/// //   values: { F: { code: "F", description: "Female", deprecated: false }, ... } }
/// ```
#[wasm_bindgen(js_name = getVocabularyTable)]
pub fn get_vocabulary_table(table_id: &str) -> Result<JsValue, JsValue> {
    let registry = TableRegistry::new();

    match registry.get_table(table_id) {
        Some(table) => {
            // Plain `values` object (not ES Map), per d.ts.
            crate::to_json_jsvalue(table)
        }
        None => Err(JsValue::from_str(&format!(
            "Vocabulary table '{}' not found. Available tables include: 0001, 0002, 0004, 0007, 0061, 0063, 0078, 0085, 0103, 0119, 0201, 0203, 0301",
            table_id
        ))),
    }
}

/// Sort rank for search hits: segments (0) before fields (1).
/// `match_type` strings must not be compared directly: ascending
/// lexicographic order puts `"field"` before `"segment"`.
fn match_type_rank(match_type: &str) -> u8 {
    match match_type {
        "segment" => 0,
        _ => 1,
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
                && seen_segments.insert(seg_id.clone())
            {
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
        match_type_rank(&a.match_type)
            .cmp(&match_type_rank(&b.match_type))
            .then(a.segment_id.cmp(&b.segment_id))
            .then(a.field_index.cmp(&b.field_index))
    });

    // Limit to 50 results
    results.truncate(50);

    // `null` (not `undefined`) for missing `field_index`/`data_type`,
    // per d.ts (`SearchResult`).
    crate::to_json_jsvalue(&results)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    #[test]
    fn test_parse_version_valid() {
        // parse_version returns Result<Version, JsValue> but JsValue panics on
        // non-wasm32 targets. Test the underlying logic instead.
        let version_map: &[(&str, bool)] = &[
            ("2.1", true),
            ("2.2", true),
            ("2.3", true),
            ("2.3.1", true),
            ("2.4", true),
            ("2.5", true),
            ("2.5.1", true),
            ("2.6", true),
            ("2.7", true),
            ("2.7.1", true),
            ("2.8", true),
            ("2.8.1", true),
            ("2.8.2", true),
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
        assert_eq!(SUPPORTED_VERSIONS.len(), 13);
        assert!(SUPPORTED_VERSIONS.contains(&"2.1"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.2"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.3"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.3.1"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.5.1"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.7"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.7.1"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.8"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.8.1"));
        assert!(SUPPORTED_VERSIONS.contains(&"2.8.2"));
    }

    #[test]
    fn point_releases_are_listed_as_supported() {
        // Core supports 2.3.1/2.5.1/2.7.1/2.8.1/2.8.2 and the
        // schema loader maps them onto their major dirs, so the
        // WASM bindings must accept them too.
        for ver in ["2.3.1", "2.5.1", "2.7.1", "2.8.1", "2.8.2"] {
            assert!(
                SUPPORTED_VERSIONS.contains(&ver),
                "point release {} is missing",
                ver
            );
        }
    }

    #[test]
    fn validate_version_string_rejects_unknown() {
        let err = validate_version_string("9.9").unwrap_err();
        assert!(err.contains("Unsupported HL7 version: '9.9'"), "{err}");
        assert!(err.contains("2.5"), "{err}");
        assert_eq!(validate_version_string("2.5").unwrap(), Version::V2_5);
    }

    #[test]
    fn parse_version_accepts_point_releases() {
        // The Ok path builds no JsValue, so it runs on native targets.
        assert_eq!(parse_version("2.3.1").unwrap(), Version::V2_3_1);
        assert_eq!(parse_version("2.5.1").unwrap(), Version::V2_5_1);
        assert_eq!(parse_version("2.7.1").unwrap(), Version::V2_7_1);
        assert_eq!(parse_version("2.8.1").unwrap(), Version::V2_8_1);
        assert_eq!(parse_version("2.8.2").unwrap(), Version::V2_8_2);
        assert_eq!(parse_version("2.5").unwrap(), Version::V2_5);
    }

    #[test]
    fn segment_search_finds_merge_and_problem_segments() {
        for seg_id in ["MRG", "PRB", "GOL", "PTH", "VAR"] {
            assert!(
                find_segment_definition(Version::V2_5, seg_id).is_some(),
                "{seg_id} must resolve from its schema"
            );
        }
    }

    #[test]
    fn segment_search_keeps_curated_hits_and_misses_unknown() {
        // Previously covered segments still resolve first-match.
        for seg_id in ["MSH", "PID", "OBX", "RXA", "QRD"] {
            assert!(
                find_segment_definition(Version::V2_5, seg_id).is_some(),
                "{seg_id} must keep resolving"
            );
        }
        assert!(find_segment_definition(Version::V2_5, "ZZZ").is_none());
    }

    #[test]
    fn search_sort_places_segments_before_fields() {
        // Ascending lexicographic order puts "field" before
        // "segment", so the comparator must rank explicitly.
        assert!(match_type_rank("segment") < match_type_rank("field"));
        let mut hits = ["field", "segment", "field", "segment"];
        hits.sort_by_key(|m| match_type_rank(m));
        assert_eq!(hits, ["segment", "segment", "field", "field"]);
    }

    #[test]
    fn native_shapes_back_wasm_prefix_assertions() {
        // Locks the d.ts shapes the wasm prefix assertions below rely
        // on: `TableValue` keeps code/description/deprecated and PID-5
        // keeps name "Patient Name" with data type "XPN", so the wasm
        // `JSON.stringify` prefixes (no premature `}`) must hold.
        let registry = TableRegistry::new();
        let table = registry.get_table("0001").expect("table 0001 exists");
        let male = table.values.get("M").expect("M in table 0001");
        assert_eq!(male.code, "M");
        assert_eq!(male.description, "Male");
        assert!(!male.deprecated);
        let schema = load_schema(Version::V2_5, "ADT", "A01").expect("ADT_A01");
        let pid = schema.segments.get("PID").expect("PID in ADT_A01");
        let field5 = pid.fields.get(&5).expect("PID-5 exists");
        assert_eq!(field5.name, "Patient Name");
        assert_eq!(field5.data_type, "XPN");
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_vocabulary_table_serializes_values_as_object() {
        // Pre-fix `values` was an ES Map (`values["M"]` undefined,
        // stringified as `{}`); d.ts promises a `Record`. Prefix (no
        // closing brace): each value also carries `description` and
        // `deprecated` per d.ts `TableValue`.
        let json = crate::test_stringify(&get_vocabulary_table("0001").unwrap());
        assert!(
            json.contains("\"M\":{\"code\":\"M\""),
            "not an object: {json}"
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_message_definition_uses_string_field_keys() {
        // Pre-fix `segments`/`fields` were nested ES Maps (stringified
        // as `{}`); d.ts promises nested `Record`s with string keys.
        // Prefix (no closing brace): each field also carries
        // `dataType` etc. per d.ts `FieldDefinition`.
        let def = get_message_definition("2.5", "ADT", "A01").unwrap();
        let json = crate::test_stringify(&def);
        assert!(json.contains("\"PID\":"), "segments not an object: {json}");
        assert!(
            json.contains("\"5\":{\"name\":\"Patient Name\""),
            "bad keys: {json}"
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_segment_definition_uses_string_keys_and_nulls() {
        // Pre-fix `fields` was an ES Map and missing `max_length` was
        // `undefined` (dropped from JSON); d.ts wants string keys and
        // `maxLength: number | null` (PID-20 has no max length).
        // Prefix (no closing brace): each field also carries
        // `dataType` etc. per d.ts `FieldDefinition`.
        let def = get_segment_definition("2.5", "PID").unwrap();
        let json = crate::test_stringify(&def);
        assert!(
            json.contains("\"5\":{\"name\":\"Patient Name\""),
            "bad keys: {json}"
        );
        assert!(json.contains("\"20\":"), "PID-20 missing: {json}");
        assert!(
            json.contains("\"maxLength\":null"),
            "missing must be null: {json}"
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_search_definitions_use_null_for_missing() {
        // Pre-fix segment hits carried `fieldIndex`/`dataType` as
        // `undefined` (dropped from JSON); d.ts wants explicit `null`.
        let json = crate::test_stringify(&search_definitions("2.5", "patient").unwrap());
        assert!(
            json.contains("\"matchType\":\"segment\""),
            "no segment hit: {json}"
        );
        assert!(json.contains("\"fieldIndex\":null"), "must be null: {json}");
        assert!(json.contains("\"dataType\":null"), "must be null: {json}");
        assert!(
            json.contains("\"matchType\":\"field\""),
            "no field hit: {json}"
        );
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn test_search_definitions_sorts_segments_first() {
        // "patient" matches both the PID segment and several fields;
        // segment hits must sort before field hits (see rustdoc).
        let json = crate::test_stringify(&search_definitions("2.5", "patient").unwrap());
        let first_segment = json.find("\"matchType\":\"segment\"").expect("segment hit");
        let first_field = json.find("\"matchType\":\"field\"").expect("field hit");
        assert!(first_segment < first_field, "segments sort first: {json}");
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
