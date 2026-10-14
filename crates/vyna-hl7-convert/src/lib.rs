//! Schema-driven cross-version conversion for HL7 v2 messages.
//!
//! [`convert`] rewrites a message from one HL7 version to another using the
//! validator's per-version schemas, reporting everything that could not be
//! carried over. Fields convert by structural rules, not hand-written maps:
//!
//! - Same field number in both versions: copied, even across cosmetic
//!   renames (number wins).
//! - Number missing in the target: moved to the target field with the same
//!   name when one exists, otherwise dropped and reported.
//! - A move whose target already holds the segment's own copied value
//!   appends as extra repetitions so both values survive.
//! - Segments unknown to the *source* schema (Z-segments and local
//!   extensions) pass through verbatim in both directions.
//! - Fields unknown to the *source* schema pass through verbatim too;
//!   only source-known-but-unmappable fields are dropped and reported.
//! - Segments known to the source but absent in the target are dropped and
//!   reported.
//! - MSH-12 is stamped with the target version.
//!
//! ```rust
//! use vyna_hl7_convert::convert;
//! use vyna_hl7_core::Version;
//! use vyna_hl7_parser::parse_message;
//!
//! let message = parse_message(
//!     "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7",
//! )
//! .unwrap();
//! let (converted, report) = convert(&message, Version::V2_5, Version::V2_3).unwrap();
//! assert!(report.dropped_fields.is_empty());
//! assert_eq!(
//!     converted.get_segment_by_id("MSH").unwrap().get_field_value(12),
//!     Some("2.3")
//! );
//! ```

use std::collections::HashMap;
use thiserror::Error;
use vyna_hl7_core::{Field, Message, Segment, Version};
use vyna_hl7_validator::{MessageSchema, SegmentDefinition, load_schema};

/// Conversion failure.
#[derive(Debug, Error)]
pub enum ConvertError {
    /// MSH-9 is missing, so the message type is unknown.
    #[error("cannot convert a message without MSH-9")]
    MissingMessageType,
    /// The version pair is refused (e.g. a legacy fallback schema would
    /// be stamped with the wrong version).
    #[error("unsupported conversion: {0}")]
    Unsupported(String),
    /// A version schema could not be loaded; the source chain is kept.
    #[error("{0}")]
    Schema(#[from] vyna_hl7_core::Error),
}

/// What happened during a conversion, for audit and follow-up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversionReport {
    /// Segments dropped as unknown in the target, e.g. `OBX[1]`.
    pub dropped_segments: Vec<String>,
    /// Fields dropped as unmappable, e.g. `PID[0]-33`.
    pub dropped_fields: Vec<String>,
    /// Fields moved by name to another number, e.g. `PID[0]-5->6`.
    pub moved_fields: Vec<String>,
    /// Target-required segments/fields absent afterwards, e.g. `ERR[0]-3`.
    pub missing_required: Vec<String>,
}

impl ConversionReport {
    /// True when nothing was dropped, moved, or left missing.
    pub fn is_clean(&self) -> bool {
        self.dropped_segments.is_empty()
            && self.dropped_fields.is_empty()
            && self.moved_fields.is_empty()
            && self.missing_required.is_empty()
    }
}

/// Convert `message` from one HL7 version to another.
///
/// The message type and trigger event come from MSH-9; both versions need
/// validator schemas for that message.
///
/// Length limits (`max_length`) from either schema are not enforced:
/// over-long values pass through verbatim instead of being truncated
/// or dropped. Validate the result with `vyna_hl7_validator` to check
/// lengths.
pub fn convert(
    message: &Message,
    from: Version,
    to: Version,
) -> Result<(Message, ConversionReport), ConvertError> {
    let (msg_type, trigger) = message_type_of(message)?;
    let from_schema = version_schema(from, &msg_type, &trigger, "source")?;
    let to_schema = version_schema(to, &msg_type, &trigger, "target")?;
    Ok(convert_with_schemas(message, &from_schema, &to_schema, to))
}

/// Load the schema covering `version`, refusing the loader's legacy
/// fallbacks: conversion must not silently use a v2.3 structure while
/// stamping MSH-12 with 2.1 or 2.2.
fn version_schema(
    version: Version,
    msg_type: &str,
    trigger: &str,
    role: &str,
) -> Result<MessageSchema, ConvertError> {
    let schema = load_schema(version, msg_type, trigger)?;
    if schema.version != schema_version(version) {
        return Err(ConvertError::Unsupported(format!(
            "{msg_type}^{trigger} has no {} {role} schema",
            version.as_str()
        )));
    }
    Ok(schema)
}

/// Schema-directory version backing a `Version` (patch releases share dirs).
fn schema_version(version: Version) -> &'static str {
    match version {
        Version::V2_1 => "2.1",
        Version::V2_2 => "2.2",
        Version::V2_3 | Version::V2_3_1 => "2.3",
        Version::V2_4 => "2.4",
        Version::V2_5 | Version::V2_5_1 => "2.5",
        Version::V2_6 => "2.6",
        Version::V2_7 | Version::V2_7_1 => "2.7",
        Version::V2_8 | Version::V2_8_1 | Version::V2_8_2 => "2.8",
    }
}

/// Convert using explicit schemas (the engine behind [`convert`]).
///
/// Exposed so callers can convert with custom or patched schemas; the
/// report and MSH-12 stamp behave exactly as in [`convert`]. Like
/// [`convert`], this does not enforce `max_length` limits: over-long
/// values pass through verbatim.
pub fn convert_with_schemas(
    message: &Message,
    from_schema: &MessageSchema,
    to_schema: &MessageSchema,
    to: Version,
) -> (Message, ConversionReport) {
    let mut report = ConversionReport::default();
    let mut out = Message::new();
    out.delimiters = message.delimiters;

    let mut occurrences: HashMap<&str, usize> = HashMap::new();
    for segment in &message.segments {
        let occurrence = occurrences.entry(segment.id.as_str()).or_insert(0);
        let here = *occurrence;
        *occurrence += 1;

        let from_def = from_schema.segments.get(&segment.id);
        let Some(to_def) = to_schema.segments.get(&segment.id) else {
            if from_def.is_none() {
                // Unknown to the source schema: local extension, keep it.
                out.add_segment(segment.clone());
            } else {
                report
                    .dropped_segments
                    .push(format!("{}[{here}]", segment.id));
            }
            continue;
        };
        if from_def.is_none() {
            // Known in the target but not the source schema: keep verbatim
            // rather than guess a field mapping.
            out.add_segment(segment.clone());
            continue;
        }

        let mut converted = Segment::new(&segment.id);
        // Pass 1: copy same-number fields so a segment's own values land
        // first; moves merge into them in pass 2 instead of colliding.
        let mut pending: Vec<(usize, &Field)> = Vec::new();
        let mut field_number = 1;
        while let Some(field) = segment.get_field(field_number) {
            if to_def.fields.contains_key(&field_number) {
                // Same number on both sides: copy, tolerating renames.
                // Empty fields copy too so positions never shift.
                let _ = converted.set_field(field_number, field.clone());
            } else {
                pending.push((field_number, field));
            }
            field_number += 1;
        }
        // Pass 2: keep source-unknown fields verbatim (local
        // extensions, mirroring the segment rule), move by name, or
        // drop with a report.
        for (number, field) in pending {
            if from_def.is_some_and(|def| !def.fields.contains_key(&number)) {
                // Unknown to the source schema: keep it verbatim, even
                // when empty, so positions never shift.
                let _ = converted.set_field(number, field.clone());
            } else if !field_has_data(field) {
                // No data to lose: skip silently.
            } else if let Some(moved_to) = find_same_name(from_def, to_def, number)
                && moved_to != 0
            // field 0 is the segment id: unmappable
            {
                merge_moved_field(&mut converted, moved_to, field);
                report
                    .moved_fields
                    .push(format!("{}[{here}]-{number}->{moved_to}", segment.id));
            } else {
                report
                    .dropped_fields
                    .push(format!("{}[{here}]-{number}", segment.id));
            }
        }
        out.add_segment(converted);
    }

    // Stamp the target version.
    if let Some(msh) = out.segments.iter_mut().find(|s| s.id == "MSH") {
        let _ = msh.set_field_value(12, to.as_str());
    }

    // Required items missing after conversion.
    let mut out_occurrences: HashMap<&str, usize> = HashMap::new();
    for segment in &out.segments {
        let occurrence = out_occurrences.entry(segment.id.as_str()).or_insert(0);
        let here = *occurrence;
        *occurrence += 1;
        let Some(to_def) = to_schema.segments.get(&segment.id) else {
            continue;
        };
        for (number, def) in &to_def.fields {
            if !def.required {
                continue;
            }
            // Whole-field emptiness: a field valued only past
            // component 1 (e.g. PID-5 "^John") is still present.
            let present = segment.get_field(*number).is_some_and(|f| !f.is_empty());
            if !present {
                report
                    .missing_required
                    .push(format!("{}[{here}]-{number}", segment.id));
            }
        }
    }
    for (id, def) in &to_schema.segments {
        if def.required && !out_occurrences.contains_key(id.as_str()) {
            report.missing_required.push(id.clone());
        }
    }

    (out, report)
}

/// Message type and trigger event from MSH-9, tolerating trigger-less
/// types like `ACK`.
fn message_type_of(message: &Message) -> Result<(String, String), ConvertError> {
    message
        .get_msh()
        .and_then(|msh| msh.get_field(9))
        .and_then(|field| field.get_repetition(0))
        .and_then(|rep| {
            let msg_type = rep.get_component(0)?.value()?.to_string();
            let trigger = rep
                .get_component(1)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            Some((msg_type, trigger))
        })
        .ok_or(ConvertError::MissingMessageType)
}

/// True when any repetition carries a non-empty value.
fn field_has_data(field: &vyna_hl7_core::Field) -> bool {
    field
        .repetitions
        .iter()
        .any(|r| r.full_value().is_some_and(|v| !v.is_empty()))
}

/// Place a moved field, appending as extra repetitions when the target
/// slot already holds the segment's own value so neither value is lost.
///
/// Choice: keep both values (own value first, moved value appended)
/// rather than drop either; the move stays recorded in `moved_fields`,
/// so no separate collision report is needed.
fn merge_moved_field(converted: &mut Segment, number: usize, field: &Field) {
    let occupied = converted.get_field(number).is_some_and(field_has_data);
    if !occupied {
        let _ = converted.set_field(number, field.clone());
        return;
    }
    if let Some(existing) = converted.get_field_mut(number) {
        for repetition in &field.repetitions {
            existing.add_repetition(repetition.clone());
        }
    }
}

/// Target field number holding the same name as source `field_number`.
fn find_same_name(
    from_def: Option<&SegmentDefinition>,
    to_def: &SegmentDefinition,
    field_number: usize,
) -> Option<usize> {
    let name = &from_def?.fields.get(&field_number)?.name;
    to_def
        .fields
        .iter()
        .find(|(_, def)| &def.name == name)
        .map(|(number, _)| *number)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;
    use vyna_hl7_validator::FieldDefinition;

    /// Schema-compliant ADT_A01 with v2.5-only fields populated.
    fn adt_25() -> Message {
        let mut message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rEVN||20240101\rPID|1||7||Smith^John\rPV1|1|O\rOBX|1|NM|WBC||8.5\r",
        )
        .unwrap();
        let msh = message.segments.iter_mut().find(|s| s.id == "MSH").unwrap();
        let _ = msh.set_field_value(21, "PIDprofile");
        let pid = message.segments.iter_mut().find(|s| s.id == "PID").unwrap();
        let _ = pid.set_field_value(33, "20240101120000");
        let obx = message.segments.iter_mut().find(|s| s.id == "OBX").unwrap();
        let _ = obx.set_field_value(11, "F");
        let _ = obx.set_field_value(18, "EQP1");
        message
    }

    fn field(name: &str, required: bool) -> FieldDefinition {
        FieldDefinition {
            name: name.to_string(),
            data_type: "ST".to_string(),
            required,
            repeating: false,
            max_length: None,
            table_id: None,
            components: None,
        }
    }

    fn schema(fields: &[(usize, &str, bool)]) -> MessageSchema {
        MessageSchema {
            message_type: "TST".to_string(),
            trigger_event: "T01".to_string(),
            version: "2.5".to_string(),
            segments: HashMap::from([(
                "PID".to_string(),
                SegmentDefinition {
                    name: "Patient Identification".to_string(),
                    required: true,
                    repeating: false,
                    fields: fields
                        .iter()
                        .map(|(n, name, req)| (*n, field(name, *req)))
                        .collect(),
                },
            )]),
        }
    }

    #[test]
    fn same_version_conversion_is_clean() {
        let message = adt_25();
        let (converted, report) = convert(&message, Version::V2_5, Version::V2_5).unwrap();

        assert!(report.is_clean());
        assert_eq!(message.encode(), converted.encode());
    }

    #[test]
    fn downgrade_drops_newer_fields_and_restamps() {
        // MSH-21, PID-33 and OBX-18 exist in v2.5 but not v2.3.
        let message = adt_25();

        let (converted, report) = convert(&message, Version::V2_5, Version::V2_3).unwrap();

        assert_eq!(
            report.dropped_fields,
            vec!["MSH[0]-21", "PID[0]-33", "OBX[0]-18"]
        );
        assert!(report.dropped_segments.is_empty());
        assert_eq!(
            converted
                .get_segment_by_id("MSH")
                .unwrap()
                .get_field_value(12),
            Some("2.3")
        );
        // Shared fields survive.
        assert_eq!(
            converted
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(3),
            Some("7")
        );
    }

    #[test]
    fn upgrade_keeps_everything_and_restamps() {
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.3\rPID|1||7\r").unwrap();
        let (converted, report) = convert(&message, Version::V2_3, Version::V2_5).unwrap();

        assert!(report.dropped_fields.is_empty());
        assert_eq!(
            converted
                .get_segment_by_id("MSH")
                .unwrap()
                .get_field_value(12),
            Some("2.5")
        );
    }

    #[test]
    fn custom_segments_pass_through_verbatim() {
        let message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\rZCU|1|local-data\r",
        )
        .unwrap();
        let (converted, report) = convert(&message, Version::V2_5, Version::V2_3).unwrap();

        // Nothing dropped or moved (the message is not schema-complete, so
        // missing_required is out of scope for this assertion).
        assert!(report.dropped_segments.is_empty());
        assert!(report.dropped_fields.is_empty());
        assert!(report.moved_fields.is_empty());
        assert_eq!(
            converted
                .get_segment_by_id("ZCU")
                .unwrap()
                .get_field_value(2),
            Some("local-data")
        );
    }

    #[test]
    fn missing_required_target_fields_are_reported() {
        // ERR-3/ERR-4 are required in v2.8 ACK but absent in v2.3.
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ACK|1|P|2.3\rMSA|AA|1\rERR||\r").unwrap();
        let (converted, report) = convert(&message, Version::V2_3, Version::V2_8).unwrap();

        assert_eq!(
            converted
                .get_segment_by_id("MSH")
                .unwrap()
                .get_field_value(12),
            Some("2.8")
        );
        assert!(report.missing_required.contains(&"ERR[0]-3".to_string()));
        assert!(report.missing_required.contains(&"ERR[0]-4".to_string()));
    }

    #[test]
    fn unknown_message_or_version_is_an_error() {
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ZZZ^Z99|1|P|2.5\rPID|1||7\r").unwrap();
        // Unknown messages fail in the schema loader, keeping the cause.
        let err = convert(&message, Version::V2_5, Version::V2_3).unwrap_err();
        assert!(matches!(err, ConvertError::Schema(_)));
        let source: &dyn std::error::Error = &err;
        assert!(source.source().is_some());

        // v2.1 has real schemas for covered messages: ADT converts.
        let adt = parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\r").unwrap();
        let (converted, _report) = convert(&adt, Version::V2_5, Version::V2_1).unwrap();
        assert_eq!(
            converted
                .get_segment_by_id("MSH")
                .unwrap()
                .get_field_value(12),
            Some("2.1")
        );

        // VXU did not exist in 2.1: the v2.3 fallback must be refused.
        let vxu = parse_message("MSH|^~\\&|A|B|C|D|20240101||VXU^V04|1|P|2.5\rPID|1||7\r").unwrap();
        assert!(matches!(
            convert(&vxu, Version::V2_5, Version::V2_1),
            Err(ConvertError::Unsupported(_))
        ));
    }

    #[test]
    fn target_length_limits_are_not_enforced() {
        // PID-3 allows 250 chars in the v2.3 schema; a longer value must
        // still pass through verbatim instead of being truncated or
        // dropped. Length checks belong to the validator.
        let long = "X".repeat(300);
        let er7 = format!("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||{long}\r");
        let message = parse_message(&er7).unwrap();
        let (converted, report) = convert(&message, Version::V2_5, Version::V2_3).unwrap();
        assert!(report.dropped_fields.is_empty());
        assert_eq!(
            converted
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(3),
            Some(long.as_str())
        );
    }

    #[test]
    fn moved_fields_follow_the_name() {
        // Synthetic schemas: "Nickname" lives at 2 in source, 4 in target.
        let from = schema(&[(1, "Set ID", false), (2, "Nickname", false)]);
        let to = schema(&[(1, "Set ID", false), (4, "Nickname", false)]);
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||TST^T01|1|P|2.5\rPID|1|Bobby\r").unwrap();

        let (converted, report) = convert_with_schemas(&message, &from, &to, Version::V2_5);

        assert_eq!(report.moved_fields, vec!["PID[0]-2->4"]);
        assert!(report.dropped_fields.is_empty());
        assert_eq!(
            converted
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(4),
            Some("Bobby")
        );
    }

    #[test]
    fn move_to_field_zero_is_reported_dropped() {
        // Synthetic schemas: "Nickname" lives at 2 in source, 0 in target.
        // Field 0 cannot hold a value, so the field must be reported
        // dropped rather than moved-into-the-void.
        let from = schema(&[(1, "Set ID", false), (2, "Nickname", false)]);
        let to = schema(&[(1, "Set ID", false), (0, "Nickname", false)]);
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||TST^T01|1|P|2.5\rPID|1|Bobby\r").unwrap();

        let (converted, report) = convert_with_schemas(&message, &from, &to, Version::V2_5);

        assert!(report.moved_fields.is_empty());
        assert_eq!(report.dropped_fields, vec!["PID[0]-2"]);
        assert_eq!(
            converted
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(2),
            None
        );
    }

    #[test]
    fn move_into_occupied_target_appends_repetition() {
        // Target field 2 already holds the segment's own value when the
        // move from field 6 lands: both values must survive.
        let from = schema(&[
            (1, "Set ID", false),
            (2, "Given", false),
            (6, "Nick", false),
        ]);
        let to = schema(&[(1, "Set ID", false), (2, "Nick", false)]);
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||TST^T01|1|P|2.5\rPID|1|OwnTwo||||Bobby\r")
                .unwrap();

        let (converted, report) = convert_with_schemas(&message, &from, &to, Version::V2_5);

        assert_eq!(report.moved_fields, vec!["PID[0]-6->2"]);
        let field = converted
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(2)
            .unwrap();
        let values: Vec<&str> = field.repetitions.iter().filter_map(|r| r.value()).collect();
        assert_eq!(values, vec!["OwnTwo", "Bobby"]);
    }

    #[test]
    fn earlier_move_survives_later_same_number_copy() {
        // The move from field 2 lands before field 4's own value is
        // copied: the moved value must not be overwritten.
        let from = schema(&[(1, "Set ID", false), (2, "Nick", false), (4, "Own", false)]);
        let to = schema(&[(1, "Set ID", false), (4, "Nick", false)]);
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||TST^T01|1|P|2.5\rPID|1|Bobby||OwnValue\r")
                .unwrap();

        let (converted, report) = convert_with_schemas(&message, &from, &to, Version::V2_5);

        assert_eq!(report.moved_fields, vec!["PID[0]-2->4"]);
        let field = converted
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(4)
            .unwrap();
        let values: Vec<&str> = field.repetitions.iter().filter_map(|r| r.value()).collect();
        assert_eq!(values, vec!["OwnValue", "Bobby"]);
    }

    #[test]
    fn required_field_valued_past_component_one_is_present() {
        // PID-5 "^John" values only component 2: whole-field emptiness
        // must see it, or the field is wrongly reported missing.
        let from = schema(&[(1, "Set ID", false), (5, "Patient Name", false)]);
        let to = schema(&[(1, "Set ID", false), (5, "Patient Name", true)]);
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||TST^T01|1|P|2.5\rPID|1||||^John\r").unwrap();

        let (_converted, report) = convert_with_schemas(&message, &from, &to, Version::V2_5);

        assert!(
            report.missing_required.is_empty(),
            "unexpected missing: {:?}",
            report.missing_required
        );

        // A genuinely absent required field is still reported.
        let empty = parse_message("MSH|^~\\&|A|B|C|D|20240101||TST^T01|1|P|2.5\rPID|1\r").unwrap();
        let (_converted, report) = convert_with_schemas(&empty, &from, &to, Version::V2_5);
        assert_eq!(report.missing_required, vec!["PID[0]-5"]);
    }

    #[test]
    fn same_version_conversion_keeps_schema_unknown_fields() {
        // PID-50 is outside the v2.5 schema (1..=39): a local
        // extension, so identity conversion must keep it verbatim.
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\r").unwrap();
        let pid = message.segments.iter_mut().find(|s| s.id == "PID").unwrap();
        let _ = pid.set_field_value(50, "LOCALDATA");

        let (converted, report) = convert(&message, Version::V2_5, Version::V2_5).unwrap();

        // The minimal message legitimately reports missing required
        // items, but nothing may be dropped or moved.
        assert!(report.dropped_fields.is_empty(), "unexpected: {report:?}");
        assert!(report.dropped_segments.is_empty(), "unexpected: {report:?}");
        assert!(report.moved_fields.is_empty(), "unexpected: {report:?}");
        assert_eq!(
            converted
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(50),
            Some("LOCALDATA")
        );
        assert_eq!(message.encode(), converted.encode());

        // Cross-version conversion keeps source-unknown fields too.
        let (down, down_report) = convert(&message, Version::V2_5, Version::V2_3).unwrap();
        assert_eq!(
            down.get_segment_by_id("PID").unwrap().get_field_value(50),
            Some("LOCALDATA")
        );
        assert!(
            !down_report.dropped_fields.iter().any(|d| d == "PID[0]-50"),
            "unexpected report: {down_report:?}"
        );
    }

    #[test]
    fn renames_at_the_same_number_copy_silently() {
        // Same number, different name: number wins, nothing reported.
        let from = schema(&[(1, "Old Name", false)]);
        let to = schema(&[(1, "New Name", false)]);
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||TST^T01|1|P|2.5\rPID|1\r").unwrap();

        let (converted, report) = convert_with_schemas(&message, &from, &to, Version::V2_5);

        assert!(report.is_clean());
        assert_eq!(
            converted
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(1),
            Some("1")
        );
    }
}
