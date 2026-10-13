//! Declarative YAML mapping from HL7 v2 to FHIR JSON.
//!
//! Code converters cover the common resources, but every interface has local
//! twists. Mappings express the same conversion as data: terser paths on the
//! HL7 side, JSON paths on the FHIR side, with value translation, formatting,
//! templating, and per-segment repetition.
//!
//! ```yaml
//! resourceType: Condition
//! fields:
//!   - target: code.coding[0].code
//!     source: PRB-3
//!   - target: subject.reference
//!     source: PID-3
//!     template: "Patient/{value}"
//!   - target: clinicalStatus.coding[0].code
//!     constant: active
//! ```
//!
//! ```rust
//! use vyna_hl7_fhir::mapping::{Mapping, MappingEngine};
//! use vyna_hl7_parser::parse_message;
//!
//! let mapping = Mapping::from_yaml(vyna_hl7_fhir::mapping::mappings::CONDITION).unwrap();
//! let message = parse_message("MSH|^~\\&|A|B|C|D|20240101||PPR^PC1|1|P|2.5\rPID|1||7\rPRB|AD|20240101|38341003^HTN^SCT|P1").unwrap();
//! let resources = MappingEngine::apply(&mapping, &message).unwrap();
//! assert_eq!(resources[0]["resourceType"], "Condition");
//! ```
//!
//! ## Rule reference
//!
//! - `target` (required): FHIR JSON path, e.g. `code.coding[0].code`.
//! - `source`: terser path, e.g. `PID-5-1` or `OBX(2)-5`. A rule is skipped
//!   when its source is absent or empty.
//! - `constant`: literal value used instead of a source.
//! - `map`: translate the value through a table; unmapped values fall back
//!   to `default` when given, otherwise pass through unchanged.
//! - `format`: `date` (`YYYYMMDD` -> `YYYY-MM-DD`) or `datetime`
//!   (`YYYYMMDDHHMMSS` -> ISO 8601, date-only when short).
//! - `template`: wrap the value, e.g. `"Patient/{value}"`.
//! - `when`: terser path that must resolve to a non-empty value, else the
//!   rule is skipped.
//! - `repeat: {segment: XXX}`: emit one resource per segment, rewriting
//!   sources on that segment to the current repetition (`OBX-5` reads the
//!   Nth OBX); sources on other segments stay global.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;

use crate::error::{ConversionError, ConversionResult};

/// Shipped example mappings, embedded at compile time.
pub mod mappings {
    /// PRB/DG1 problem list entry -> Condition (mirrors `ConditionConverter`).
    pub const CONDITION: &str = include_str!("../mappings/condition.yaml");
    /// AL1 allergy list entry -> AllergyIntolerance (mirrors
    /// `AllergyIntoleranceConverter` for a single repetition).
    pub const ALLERGY_INTOLERANCE: &str = include_str!("../mappings/allergy_intolerance.yaml");
}

/// A parsed declarative mapping: one FHIR resource type from one message.
#[derive(Debug, Clone, Deserialize)]
pub struct Mapping {
    /// FHIR resource type, written to `resourceType`.
    #[serde(rename = "resourceType")]
    pub resource_type: String,
    /// Emit one resource per segment with this ID instead of a single one.
    #[serde(default)]
    pub repeat: Option<RepeatSpec>,
    /// Field rules applied in order.
    #[serde(default)]
    pub fields: Vec<FieldRule>,
}

/// Per-segment repetition: one resource per matching segment.
#[derive(Debug, Clone, Deserialize)]
pub struct RepeatSpec {
    /// Segment ID to repeat over, e.g. `OBX`.
    pub segment: String,
}

/// Value formats applied after code translation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueFormat {
    /// `YYYYMMDD` -> `YYYY-MM-DD`.
    Date,
    /// `YYYYMMDDHHMMSS` -> ISO 8601 (`YYYY-MM-DD` when date-only).
    DateTime,
}

/// One mapping rule: a value source and its FHIR target path.
#[derive(Debug, Clone, Deserialize)]
pub struct FieldRule {
    /// FHIR JSON path, e.g. `code.coding[0].code`.
    pub target: String,
    /// Terser path to read, e.g. `PID-5-1`.
    #[serde(default)]
    pub source: Option<String>,
    /// Literal value used when no source is given.
    #[serde(default)]
    pub constant: Option<Value>,
    /// Translate the value through this table.
    #[serde(default)]
    pub map: Option<HashMap<String, String>>,
    /// Fallback when `map` has no entry (otherwise the value passes through).
    #[serde(default)]
    pub default: Option<String>,
    /// Reformat the value (`date` or `datetime`).
    #[serde(default)]
    pub format: Option<ValueFormat>,
    /// Wrap the value; `{value}` is replaced, e.g. `"Patient/{value}"`.
    #[serde(default)]
    pub template: Option<String>,
    /// Only apply when this terser path resolves to a non-empty value.
    #[serde(default)]
    pub when: Option<String>,
    /// Second terser path tried when `source` resolves to nothing.
    #[serde(default)]
    pub fallback_source: Option<String>,
}

/// One step of a parsed target path: an object key with an optional index.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PathStep {
    key: String,
    index: Option<usize>,
}

impl Mapping {
    /// Parse and validate a mapping from YAML text.
    pub fn from_yaml(yaml: &str) -> ConversionResult<Self> {
        let mapping: Mapping = serde_yaml::from_str(yaml)
            .map_err(|e| ConversionError::Mapping(format!("invalid mapping YAML: {e}")))?;
        mapping.validate()?;
        Ok(mapping)
    }

    /// Check shape, targets, and terser paths without a message.
    pub fn validate(&self) -> ConversionResult<()> {
        if self.resource_type.trim().is_empty() {
            return Err(ConversionError::Mapping(
                "mapping must declare a resourceType".to_string(),
            ));
        }
        if let Some(repeat) = &self.repeat
            && !is_segment_id(&repeat.segment)
        {
            return Err(ConversionError::Mapping(format!(
                "invalid repeat segment '{}'",
                repeat.segment
            )));
        }
        for (i, rule) in self.fields.iter().enumerate() {
            parse_target(&rule.target)
                .map_err(|e| ConversionError::Mapping(format!("field {i} ({e})")))?;
            match (&rule.source, &rule.constant) {
                (Some(_), Some(_)) => {
                    return Err(ConversionError::Mapping(format!(
                        "field {i} ('{}'): source and constant are mutually exclusive",
                        rule.target
                    )));
                }
                (None, None) => {
                    return Err(ConversionError::Mapping(format!(
                        "field {i} ('{}'): either source or constant is required",
                        rule.target
                    )));
                }
                _ => {}
            }
            for (name, path) in [
                ("source", &rule.source),
                ("when", &rule.when),
                ("fallback_source", &rule.fallback_source),
            ] {
                if let Some(path) = path
                    && !is_terser_path(path)
                {
                    return Err(ConversionError::Mapping(format!(
                        "field {i} ('{}'): invalid terser {name} '{path}'",
                        rule.target
                    )));
                }
            }
        }
        Ok(())
    }
}

/// True for 3-character HL7 segment IDs (uppercase by convention).
fn is_segment_id(id: &str) -> bool {
    id.len() == 3
        && id
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

/// True for the terser subset mappings support:
/// `SEG-1`, `SEG(2)-1`, `SEG-1-2`, `SEG-1-2-3`, `SEG-11(1)-1`.
fn is_terser_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.len() < 4
        || !bytes[..3]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return false;
    }
    let mut rest = &path[3..];
    // Optional (segment index).
    if let Some(after) = rest.strip_prefix('(') {
        let Some((digits, tail)) = after.split_once(')') else {
            return false;
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        rest = tail;
    }
    // Required -field.
    let Some(tail) = rest.strip_prefix('-') else {
        return false;
    };
    let (digits, tail) = split_digits(tail);
    if digits.is_empty() {
        return false;
    }
    rest = tail;
    // Optional (repetition).
    if let Some(after) = rest.strip_prefix('(') {
        let Some((digits, tail)) = after.split_once(')') else {
            return false;
        };
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        rest = tail;
    }
    // Up to two -component parts.
    for _ in 0..2 {
        let Some(tail) = rest.strip_prefix('-') else {
            return rest.is_empty();
        };
        let (digits, tail) = split_digits(tail);
        if digits.is_empty() {
            return false;
        }
        rest = tail;
    }
    rest.is_empty()
}

/// Split leading ASCII digits from the rest.
fn split_digits(text: &str) -> (&str, &str) {
    let end = text.bytes().take_while(|b| b.is_ascii_digit()).count();
    text.split_at(end)
}

/// Parse a target path into steps, e.g. `a.b[0].c`.
fn parse_target(target: &str) -> Result<Vec<PathStep>, String> {
    if target.is_empty() {
        return Err("empty target".to_string());
    }
    let mut steps = Vec::new();
    for part in target.split('.') {
        if part.is_empty() {
            return Err(format!("empty path element in '{target}'"));
        }
        let (key, index) = match part.split_once('[') {
            None => (part, None),
            Some((key, rest)) => {
                let digits = rest
                    .strip_suffix(']')
                    .ok_or_else(|| format!("unbalanced '[' in '{target}'"))?;
                if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(format!("bad index in '{target}'"));
                }
                let index: usize = digits
                    .parse()
                    .map_err(|_| format!("bad index in '{target}'"))?;
                (key, Some(index))
            }
        };
        if key.is_empty()
            || !key
                .bytes()
                .next()
                .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
            || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            return Err(format!("bad key '{key}' in '{target}'"));
        }
        steps.push(PathStep {
            key: key.to_string(),
            index,
        });
    }
    Ok(steps)
}

/// The mapping engine: applies a [`Mapping`] to a message.
pub struct MappingEngine;

impl MappingEngine {
    /// Apply a mapping, returning one value per repetition (exactly one
    /// without `repeat`, none when the repeat segment is absent).
    pub fn apply(mapping: &Mapping, message: &Message) -> ConversionResult<Vec<Value>> {
        let terser = Terser::new(message);
        let repetitions = match &mapping.repeat {
            None => 1,
            Some(repeat) => message
                .segments
                .iter()
                .filter(|s| s.id == repeat.segment)
                .count(),
        };
        let mut resources = Vec::with_capacity(repetitions.max(1));
        for index in 0..repetitions {
            resources.push(Self::apply_once(mapping, &terser, index)?);
        }
        Ok(resources)
    }

    fn apply_once(
        mapping: &Mapping,
        terser: &Terser,
        repetition: usize,
    ) -> ConversionResult<Value> {
        let mut root = Value::Object(Default::default());
        root["resourceType"] = Value::String(mapping.resource_type.clone());

        for rule in &mapping.fields {
            if let Some(when) = &rule.when
                && Self::resolve(terser, mapping, when, repetition).is_none()
            {
                continue;
            }
            let Some(value) = Self::rule_value(rule, mapping, terser, repetition)? else {
                continue;
            };
            // Targets were validated at load; parse cannot fail here.
            let steps = parse_target(&rule.target).map_err(ConversionError::Mapping)?;
            Self::assign(&mut root, &steps, value, &rule.target)?;
        }
        Ok(root)
    }

    /// Evaluate one rule to a JSON value, or `None` to skip it.
    fn rule_value(
        rule: &FieldRule,
        mapping: &Mapping,
        terser: &Terser,
        repetition: usize,
    ) -> ConversionResult<Option<Value>> {
        let mut value = match (&rule.source, &rule.constant) {
            (Some(source), _) => {
                let resolved = Self::resolve(terser, mapping, source, repetition).or_else(|| {
                    rule.fallback_source
                        .as_ref()
                        .and_then(|f| Self::resolve(terser, mapping, f, repetition))
                });
                match resolved {
                    Some(text) => Value::String(text),
                    None => return Ok(None),
                }
            }
            (None, Some(constant)) => constant.clone(),
            (None, None) => return Ok(None),
        };

        if let Some(map) = &rule.map
            && let Value::String(text) = &value
        {
            match map.get(text) {
                Some(mapped) => value = Value::String(mapped.clone()),
                None => {
                    if let Some(default) = &rule.default {
                        value = Value::String(default.clone());
                    }
                }
            }
        }

        if let Some(format) = rule.format
            && let Value::String(text) = &value
        {
            let formatted = match format {
                ValueFormat::Date => format_date(text),
                ValueFormat::DateTime => format_datetime(text),
            }
            .ok_or_else(|| {
                ConversionError::InvalidFormat(
                    rule.target.clone(),
                    mapping.resource_type.clone(),
                    format!("cannot format '{text}'"),
                )
            })?;
            value = Value::String(formatted);
        }

        if let Some(template) = &rule.template
            && let Value::String(text) = &value
        {
            value = Value::String(template.replace("{value}", text));
        }

        Ok(Some(value))
    }

    /// Resolve a terser path for one repetition: `None` when absent or empty.
    /// Out-of-range and empty results mean "no data" and skip the rule; the
    /// path shape itself was validated when the mapping loaded.
    fn resolve(
        terser: &Terser,
        mapping: &Mapping,
        path: &str,
        repetition: usize,
    ) -> Option<String> {
        let rewritten = Self::rewrite_repeat(mapping, path, repetition);
        let value = terser.get(&rewritten).ok().flatten()?;
        if value.is_empty() {
            return None;
        }
        Some(value.to_string())
    }

    /// Rewrite sources on the repeat segment to the current repetition:
    /// `OBX-5` becomes `OBX(2)-5`. Explicitly indexed paths are left alone.
    fn rewrite_repeat(mapping: &Mapping, path: &str, repetition: usize) -> String {
        let Some(repeat) = &mapping.repeat else {
            return path.to_string();
        };
        let dash = path.find('-');
        let paren = path.find('(');
        let head = &path[..dash.unwrap_or(path.len())];
        // Already indexed, or a different segment: leave alone.
        if paren.is_some_and(|p| dash.is_none_or(|d| p < d)) || head != repeat.segment {
            return path.to_string();
        }
        match dash {
            Some(d) => format!("{}({}){}", head, repetition + 1, &path[d..]),
            None => path.to_string(),
        }
    }

    /// Assign a value at a parsed target path, creating objects and arrays.
    fn assign(
        root: &mut Value,
        steps: &[PathStep],
        value: Value,
        target: &str,
    ) -> ConversionResult<()> {
        Self::assign_at(root, steps, value, target)
    }

    fn assign_at(
        node: &mut Value,
        steps: &[PathStep],
        value: Value,
        target: &str,
    ) -> ConversionResult<()> {
        let conflict = || ConversionError::Mapping(format!("target conflict at '{target}'"));
        let step = &steps[0];
        let last = steps.len() == 1;
        let slot = node
            .as_object_mut()
            .ok_or_else(conflict)?
            .entry(step.key.clone())
            .or_insert(Value::Null);
        match step.index {
            None if last => {
                if !slot.is_null() {
                    return Err(ConversionError::Mapping(format!(
                        "target '{target}' is written twice"
                    )));
                }
                *slot = value;
                Ok(())
            }
            None => {
                if slot.is_null() {
                    *slot = Value::Object(Default::default());
                }
                if !slot.is_object() {
                    return Err(conflict());
                }
                Self::assign_at(slot, &steps[1..], value, target)
            }
            Some(index) => {
                if slot.is_null() {
                    *slot = Value::Array(Vec::new());
                }
                let array = slot.as_array_mut().ok_or_else(conflict)?;
                while array.len() <= index {
                    array.push(Value::Null);
                }
                if last {
                    if !array[index].is_null() {
                        return Err(ConversionError::Mapping(format!(
                            "target '{target}' is written twice"
                        )));
                    }
                    array[index] = value;
                    return Ok(());
                }
                if array[index].is_null() {
                    array[index] = Value::Object(Default::default());
                }
                if !array[index].is_object() {
                    return Err(conflict());
                }
                Self::assign_at(&mut array[index], &steps[1..], value, target)
            }
        }
    }
}

/// `YYYYMMDD[HHMMSS...]` -> `YYYY-MM-DD`.
fn format_date(text: &str) -> Option<String> {
    // Byte-slice checks on `[u8]` never split a char boundary, and an
    // all-ASCII-digit prefix makes the `&str` slices below safe.
    let bytes = text.as_bytes();
    if bytes.len() >= 8 && bytes[..8].iter().all(|b| b.is_ascii_digit()) {
        return Some(format!("{}-{}-{}", &text[0..4], &text[4..6], &text[6..8]));
    }
    None
}

/// HL7 datetime -> FHIR ISO 8601 (date-only when short).
fn format_datetime(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.len() >= 14 && bytes[..14].iter().all(|b| b.is_ascii_digit()) {
        Some(format!(
            "{}-{}-{}T{}:{}:{}",
            &text[0..4],
            &text[4..6],
            &text[6..8],
            &text[8..10],
            &text[10..12],
            &text[12..14]
        ))
    } else {
        format_date(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::converters::allergy_intolerance::AllergyIntoleranceConverter;
    use crate::converters::condition::ConditionConverter;
    use vyna_hl7_parser::parse_message;

    const SAMPLE: &str = "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5\rPID|1||12345^^^MRN||Smith^John||19800101|M\rOBX|1|NM|WBC^White Blood Count||8.5||||||F\rOBX|2|NM|RBC^Red Blood Count||4.5||||||F\r";

    fn message() -> Message {
        parse_message(SAMPLE).unwrap()
    }

    #[test]
    fn format_helpers_reject_non_ascii_without_panic() {
        // Misaligned multi-byte chars used to panic on byte slicing.
        assert_eq!(format_date("123\u{e9}5678"), None);
        assert_eq!(format_date("19800101"), Some("1980-01-01".to_string()));
        assert_eq!(format_datetime("123\u{e9}56789012"), None);
        assert_eq!(
            format_datetime("20240315100000"),
            Some("2024-03-15T10:00:00".to_string())
        );
    }

    #[test]
    fn engine_copies_sources_and_constants() {
        let mapping = Mapping::from_yaml(
            r#"
resourceType: Patient
fields:
  - target: identifier[0].value
    source: PID-3
  - target: active
    constant: true
"#,
        )
        .unwrap();
        let out = MappingEngine::apply(&mapping, &message()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["resourceType"], "Patient");
        assert_eq!(out[0]["identifier"][0]["value"], "12345");
        assert_eq!(out[0]["active"], true);
    }

    #[test]
    fn engine_skips_absent_and_empty_sources() {
        let mapping = Mapping::from_yaml(
            r#"
resourceType: Patient
fields:
  - target: nickname
    source: PID-2
  - target: name[0].family
    source: PID-5-1
"#,
        )
        .unwrap();
        let out = MappingEngine::apply(&mapping, &message()).unwrap();
        assert!(out[0].get("nickname").is_none());
        assert_eq!(out[0]["name"][0]["family"], "Smith");
    }

    #[test]
    fn engine_applies_map_default_format_template_and_when() {
        let mapping = Mapping::from_yaml(
            r#"
resourceType: Patient
fields:
  - target: gender
    source: PID-8
    map: {M: male, F: female}
  - target: birthDate
    source: PID-7
    format: date
  - target: subject.reference
    source: PID-3
    template: "Patient/{value}"
  - target: telecom[0].value
    source: PID-13
    when: PID-13
"#,
        )
        .unwrap();
        let out = MappingEngine::apply(&mapping, &message()).unwrap();
        assert_eq!(out[0]["gender"], "male");
        assert_eq!(out[0]["birthDate"], "1980-01-01");
        assert_eq!(out[0]["subject"]["reference"], "Patient/12345");
        // PID-13 absent: gated rule skipped.
        assert!(out[0].get("telecom").is_none());
    }

    #[test]
    fn engine_map_falls_back_to_default_then_passthrough() {
        let yaml = r#"
resourceType: X
fields:
  - target: a
    source: PID-8
    map: {F: female}
    default: unknown
  - target: b
    source: PID-8
    map: {F: female}
"#;
        let mapping = Mapping::from_yaml(yaml).unwrap();
        let out = MappingEngine::apply(&mapping, &message()).unwrap();
        assert_eq!(out[0]["a"], "unknown");
        assert_eq!(out[0]["b"], "M");
    }

    #[test]
    fn engine_repeats_per_segment_with_global_paths() {
        let mapping = Mapping::from_yaml(
            r#"
resourceType: Observation
repeat: {segment: OBX}
fields:
  - target: code.coding[0].code
    source: OBX-3-1
  - target: subject.reference
    source: PID-3
    template: "Patient/{value}"
"#,
        )
        .unwrap();
        let out = MappingEngine::apply(&mapping, &message()).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["code"]["coding"][0]["code"], "WBC");
        assert_eq!(out[1]["code"]["coding"][0]["code"], "RBC");
        assert_eq!(out[0]["subject"]["reference"], "Patient/12345");
        assert_eq!(out[1]["subject"]["reference"], "Patient/12345");
    }

    #[test]
    fn engine_repeat_over_absent_segment_yields_nothing() {
        let mapping = Mapping::from_yaml(
            r#"
resourceType: Observation
repeat: {segment: OBR}
fields:
  - target: code.coding[0].code
    source: OBR-4-1
"#,
        )
        .unwrap();
        assert!(
            MappingEngine::apply(&mapping, &message())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn engine_rejects_bad_mappings_at_load() {
        // Not YAML-mapping shaped.
        assert!(Mapping::from_yaml("resourceType: [unclosed").is_err());
        // Missing resourceType.
        assert!(Mapping::from_yaml("fields: []").is_err());
        // Neither source nor constant.
        assert!(Mapping::from_yaml("resourceType: X\nfields:\n  - target: a\n").is_err());
        // Both source and constant.
        assert!(
            Mapping::from_yaml(
                "resourceType: X\nfields:\n  - target: a\n    source: PID-3\n    constant: 1\n"
            )
            .is_err()
        );
        // Bad target.
        assert!(
            Mapping::from_yaml("resourceType: X\nfields:\n  - target: a..b\n    constant: 1\n")
                .is_err()
        );
        // Bad terser path.
        assert!(
            Mapping::from_yaml("resourceType: X\nfields:\n  - target: a\n    source: PID3\n")
                .is_err()
        );
        // Unknown format.
        assert!(
            Mapping::from_yaml(
                "resourceType: X\nfields:\n  - target: a\n    source: PID-7\n    format: fuzzy\n"
            )
            .is_err()
        );
    }

    #[test]
    fn engine_reports_target_conflicts_and_bad_dates() {
        let conflict = Mapping::from_yaml(
            "resourceType: X\nfields:\n  - target: a\n    constant: 1\n  - target: a\n    constant: 2\n",
        )
        .unwrap();
        assert!(MappingEngine::apply(&conflict, &message()).is_err());

        let bad_date = Mapping::from_yaml(
            "resourceType: X\nfields:\n  - target: birthDate\n    source: PID-8\n    format: date\n",
        )
        .unwrap();
        // PID-8 is "M": not a date.
        assert!(MappingEngine::apply(&bad_date, &message()).is_err());
    }

    #[test]
    fn terser_path_validation_covers_supported_shapes() {
        for valid in [
            "PID-3",
            "PID-5-1",
            "PID-5-1-2",
            "OBX(2)-5",
            "PID-11(1)-1",
            "MSH-9-1",
        ] {
            assert!(is_terser_path(valid), "{valid}");
        }
        for invalid in [
            "",
            "PID",
            "PID3",
            "PID-",
            "PID-3-",
            "OBX(2-5",
            "OBX()-5",
            "pid-3",
            "PID-3-1-2-3",
        ] {
            assert!(!is_terser_path(invalid), "{invalid}");
        }
    }

    #[test]
    fn shipped_condition_mapping_matches_code_converter() {
        let message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||PPR^PC1|1|P|2.5\rPID|1||7\rPRB|AD|20240101|38341003^HTN^SCT|P1",
        )
        .unwrap();

        let mapping = Mapping::from_yaml(mappings::CONDITION).unwrap();
        let declarative = MappingEngine::apply(&mapping, &message).unwrap();
        let coded = ConditionConverter::convert_all(&message).unwrap();

        assert_eq!(declarative.len(), 1);
        assert_eq!(declarative[0], serde_json::to_value(&coded[0]).unwrap());
    }

    #[test]
    fn shipped_allergy_mapping_matches_code_converter() {
        let message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\rAL1|1|DA|1191^ASPIRIN|SV",
        )
        .unwrap();

        let mapping = Mapping::from_yaml(mappings::ALLERGY_INTOLERANCE).unwrap();
        let declarative = MappingEngine::apply(&mapping, &message).unwrap();
        let coded = AllergyIntoleranceConverter::convert_all(&message).unwrap();

        assert_eq!(declarative.len(), 1);
        assert_eq!(declarative[0], serde_json::to_value(&coded[0]).unwrap());
    }

    #[test]
    fn fallback_source_used_when_primary_resolves_nothing() {
        let message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\rAL1|1|DA|PENICILLIN|SV",
        )
        .unwrap();

        let mapping = Mapping::from_yaml(
            "resourceType: AllergyIntolerance\nfields:\n  - target: code.text\n    source: AL1-3-2\n    fallback_source: AL1-3\n",
        )
        .unwrap();
        let declarative = MappingEngine::apply(&mapping, &message).unwrap();
        assert_eq!(declarative[0]["code"]["text"], "PENICILLIN");
    }

    #[test]
    fn invalid_fallback_source_rejected_at_load() {
        let err = Mapping::from_yaml(
            "resourceType: X\nfields:\n  - target: a\n    source: PID-3\n    fallback_source: nope\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("fallback_source"), "{err}");
    }

    #[test]
    fn rewrite_repeat_indexes_only_matching_unindexed_paths() {
        let mapping = Mapping::from_yaml(
            "resourceType: X\nrepeat: {segment: OBX}\nfields:\n  - target: a\n    constant: 1\n",
        )
        .unwrap();
        assert_eq!(
            MappingEngine::rewrite_repeat(&mapping, "OBX-5", 1),
            "OBX(2)-5"
        );
        assert_eq!(
            MappingEngine::rewrite_repeat(&mapping, "OBX-11(1)-1", 0),
            "OBX(1)-11(1)-1"
        );
        // Other segments untouched.
        assert_eq!(MappingEngine::rewrite_repeat(&mapping, "PID-3", 1), "PID-3");
        // Explicit index wins.
        assert_eq!(
            MappingEngine::rewrite_repeat(&mapping, "OBX(1)-5", 1),
            "OBX(1)-5"
        );
    }
}
