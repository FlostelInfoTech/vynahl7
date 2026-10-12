//! Safe Harbor de-identification for HL7 v2 messages.
//!
//! [`Redactor`] rewrites protected health information (PHI) in place using
//! explicit, auditable rules. [`Redactor::safe_harbor`] ships a profile
//! covering the HIPAA Safe Harbor identifiers across the common segments
//! (names, addresses, telecom, dates, IDs, account and visit numbers).
//!
//! ```rust
//! use vyna_hl7_parser::parse_message;
//! use vyna_hl7_redact::Redactor;
//!
//! let mut message = parse_message(
//!     "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5\rPID|1||7||Smith^John||19800101|M",
//! )
//! .unwrap();
//! let redactor = Redactor::safe_harbor(b"site-secret", 30);
//! let report = redactor.redact(&mut message).unwrap();
//! assert!(!report.is_empty());
//! let pid = message.get_segment_by_id("PID").unwrap();
//! assert!(pid.get_field_value(5).is_none()); // name gone
//! assert_eq!(pid.get_field_value(7), Some("19800131")); // DOB shifted +30d
//! ```
//!
//! ## Design notes
//!
//! - Rules apply to **every** matching repetition: whole-field rules run per
//!   repetition, component rules run on that component of every repetition.
//! - Rules only touch values that exist; absent fields are left (and
//!   reported as) untouched, and no structure is ever created.
//! - [`Strategy::Hash`] yields 12 hex characters of HMAC-SHA256, so the same
//!   key preserves linkage across messages while staying short enough for
//!   identifier fields.
//! - [`Strategy::DateShift`] preserves the time-of-day and any trailing
//!   precision; values it cannot parse are removed rather than left in place.
//! - Free-text segments (NTE, OBX-5) are deliberately **not** in the default
//!   profile: blunt removal would destroy clinical notes, and real scrubbing
//!   needs NLP. Add explicit rules if your policy requires it.

use chrono::{Duration, NaiveDate};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;
use vyna_hl7_core::{Field, Message, Repetition, Segment};

/// Redaction failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RedactError {
    /// A [`Strategy::Hash`] rule ran without an HMAC key configured.
    #[error("hash strategy requires an HMAC key")]
    MissingHmacKey,
}

/// How one value is redacted.
#[derive(Debug, Clone, PartialEq)]
pub enum Strategy {
    /// Delete the value (whole field removed; component emptied).
    Remove,
    /// Replace with `***`.
    Mask,
    /// Replace with a fixed string.
    Fixed(String),
    /// Replace with 12 hex chars of HMAC-SHA256 (requires a key).
    Hash,
    /// Shift the date part by the configured days, keeping time-of-day.
    DateShift,
    /// Keep the first `n` characters (e.g. ZIP-3 generalization).
    Truncate(usize),
}

impl Strategy {
    fn name(&self) -> &'static str {
        match self {
            Strategy::Remove => "Remove",
            Strategy::Mask => "Mask",
            Strategy::Fixed(_) => "Fixed",
            Strategy::Hash => "Hash",
            Strategy::DateShift => "DateShift",
            Strategy::Truncate(_) => "Truncate",
        }
    }
}

/// One redaction rule: a field (or component) plus its strategy.
///
/// `segment` is a 3-letter ID or `"*"` for every segment; `field` and
/// `component` are 1-based. A rule with `component: None` rewrites each
/// repetition's whole value; otherwise it rewrites that component of every
/// repetition.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// Segment ID or `"*"`.
    pub segment: String,
    /// 1-based field number.
    pub field: usize,
    /// 1-based component number, or `None` for the whole field.
    pub component: Option<usize>,
    /// How to rewrite the value.
    pub strategy: Strategy,
}

impl Rule {
    /// Whole-field rule.
    pub fn field(segment: &str, field: usize, strategy: Strategy) -> Self {
        Self {
            segment: segment.to_string(),
            field,
            component: None,
            strategy,
        }
    }

    /// Component rule.
    pub fn component(segment: &str, field: usize, component: usize, strategy: Strategy) -> Self {
        Self {
            segment: segment.to_string(),
            field,
            component: Some(component),
            strategy,
        }
    }
}

/// One applied redaction, for audit trails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redaction {
    /// Where the value was, e.g. `PID[0]-5` or `OBX[1]-14`.
    pub path: String,
    /// Strategy name that was applied.
    pub strategy: &'static str,
}

/// De-identifies messages according to a rule list.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    rules: Vec<Rule>,
    date_shift_days: i64,
    hmac_key: Option<Vec<u8>>,
}

impl Redactor {
    /// Empty redactor; add rules with the `with_*` builders.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one rule.
    pub fn with_rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Append several rules.
    pub fn with_rules(mut self, rules: Vec<Rule>) -> Self {
        self.rules.extend(rules);
        self
    }

    /// Days added by [`Strategy::DateShift`] (may be negative).
    pub fn with_date_shift_days(mut self, days: i64) -> Self {
        self.date_shift_days = days;
        self
    }

    /// HMAC key for [`Strategy::Hash`].
    pub fn with_hmac_key(mut self, key: &[u8]) -> Self {
        self.hmac_key = Some(key.to_vec());
        self
    }

    /// Append the Safe Harbor profile rules (see module docs).
    pub fn with_safe_harbor_rules(mut self) -> Self {
        self.rules.extend(safe_harbor_rules());
        self
    }

    /// Redactor with the full Safe Harbor profile.
    pub fn safe_harbor(hmac_key: &[u8], date_shift_days: i64) -> Self {
        Self::new()
            .with_hmac_key(hmac_key)
            .with_date_shift_days(date_shift_days)
            .with_safe_harbor_rules()
    }

    /// Redact `message` in place, returning what was changed.
    pub fn redact(&self, message: &mut Message) -> Result<Vec<Redaction>, RedactError> {
        if self.rules.iter().any(|r| r.strategy == Strategy::Hash) && self.hmac_key.is_none() {
            return Err(RedactError::MissingHmacKey);
        }
        let mut report = Vec::new();
        // Occurrence counter per segment ID for stable audit paths.
        let mut occurrences: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for index in 0..message.segments.len() {
            let segment_id = message.segments[index].id.clone();
            let here = occurrences.get(&segment_id).copied().unwrap_or(0);
            occurrences.insert(segment_id, here + 1);
            let segment = &mut message.segments[index];
            for rule in &self.rules {
                if rule.segment != "*" && rule.segment != segment.id {
                    continue;
                }
                match rule.component {
                    None => {
                        if self.redact_field(segment, rule)? {
                            report.push(Redaction {
                                path: format!("{}[{here}]-{}", segment.id, rule.field),
                                strategy: rule.strategy.name(),
                            });
                        }
                    }
                    Some(component) => {
                        if self.redact_component(segment, rule.field, component, &rule.strategy)? {
                            report.push(Redaction {
                                path: format!("{}[{here}]-{}-{component}", segment.id, rule.field),
                                strategy: rule.strategy.name(),
                            });
                        }
                    }
                }
            }
        }
        Ok(report)
    }

    /// Rewrite each repetition's whole value; true when anything changed.
    fn redact_field(&self, segment: &mut Segment, rule: &Rule) -> Result<bool, RedactError> {
        let Some(field) = segment.get_field(rule.field) else {
            return Ok(false);
        };
        let values: Vec<Option<String>> =
            field.repetitions.iter().map(|r| r.full_value()).collect();
        if values.iter().all(|v| v.as_deref().unwrap_or("").is_empty()) {
            return Ok(false);
        }
        if rule.strategy == Strategy::Remove {
            let _ = segment.set_field(rule.field, Field::new());
            return Ok(true);
        }
        let mut rebuilt = Field::new();
        for value in &values {
            let text = value.as_deref().unwrap_or("");
            if text.is_empty() {
                rebuilt.add_repetition(Repetition::new());
            } else {
                let replacement = self.transform(text, &rule.strategy);
                if replacement.is_empty() {
                    rebuilt.add_repetition(Repetition::new());
                } else {
                    rebuilt.add_repetition(Repetition::from_value(replacement));
                }
            }
        }
        let _ = segment.set_field(rule.field, rebuilt);
        Ok(true)
    }

    /// Rewrite one component of every repetition; true when anything changed.
    fn redact_component(
        &self,
        segment: &mut Segment,
        field_index: usize,
        component: usize,
        strategy: &Strategy,
    ) -> Result<bool, RedactError> {
        if component == 0 {
            return Ok(false);
        }
        let Some(field) = segment.get_field(field_index) else {
            return Ok(false);
        };
        // Collect first so the immutable borrow ends before mutation.
        let mut targets: Vec<(usize, String)> = Vec::new();
        for (rep_index, repetition) in field.repetitions.iter().enumerate() {
            if let Some(comp) = repetition.components.get(component - 1) {
                let text: String = comp
                    .subcomponents
                    .iter()
                    .map(|s| s.value.as_str())
                    .collect::<Vec<_>>()
                    .join("&");
                if !text.is_empty() {
                    targets.push((rep_index, text));
                }
            }
        }
        if targets.is_empty() {
            return Ok(false);
        }
        for (rep_index, text) in targets {
            let replacement = self.transform(&text, strategy);
            let _ = segment.set_component(field_index, rep_index, component - 1, replacement);
        }
        Ok(true)
    }

    /// Apply a strategy to one non-empty value.
    fn transform(&self, value: &str, strategy: &Strategy) -> String {
        match strategy {
            Strategy::Remove => String::new(),
            Strategy::Mask => "***".to_string(),
            Strategy::Fixed(fixed) => fixed.clone(),
            Strategy::Hash => self.hash(value),
            Strategy::DateShift => shift_date(value, self.date_shift_days).unwrap_or_default(),
            Strategy::Truncate(n) => value.chars().take(*n).collect(),
        }
    }

    /// 12 hex chars of HMAC-SHA256 (key presence checked in `redact`).
    fn hash(&self, value: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.hmac_key.as_deref().unwrap_or_default())
            .expect("HMAC accepts any key length");
        mac.update(value.as_bytes());
        let digest = mac.finalize().into_bytes();
        let mut hex = String::with_capacity(24);
        for byte in digest.iter().take(6) {
            use std::fmt::Write;
            write!(hex, "{byte:02x}").expect("writing to String cannot fail");
        }
        hex
    }
}

/// Shift the `YYYYMMDD` date part by `days`, preserving the remainder.
/// Returns `None` when the value has no parseable date part.
fn shift_date(value: &str, days: i64) -> Option<String> {
    if value.len() < 8 || !value[..8].bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let date = NaiveDate::parse_from_str(&value[..8], "%Y%m%d").ok()?;
    let shifted = date + Duration::days(days);
    Some(format!("{}{}", shifted.format("%Y%m%d"), &value[8..]))
}

/// The Safe Harbor profile: (segment, field, component, strategy).
fn safe_harbor_rules() -> Vec<Rule> {
    use Strategy::{DateShift, Hash, Remove, Truncate};
    // (segment, field, component)
    let hashed: &[(&str, usize)] = &[
        ("MSH", 10),
        ("PID", 2),
        ("PID", 3),
        ("PID", 4),
        ("PID", 18),
        ("PID", 21),
        ("PV1", 19),
        ("PV1", 50),
        ("ORC", 2),
        ("ORC", 3),
        ("OBR", 2),
        ("OBR", 3),
        ("GT1", 2),
        ("IN1", 36),
        ("MRG", 1),
        ("MRG", 2),
        ("MRG", 3),
        ("MRG", 4),
        ("MRG", 5),
        ("MRG", 6),
        ("SPM", 2),
    ];
    let removed: &[(&str, usize)] = &[
        ("PID", 5),
        ("PID", 6),
        ("PID", 9),
        ("PID", 13),
        ("PID", 14),
        ("PID", 19),
        ("PID", 20),
        ("PID", 23),
        ("PD1", 4),
        ("NK1", 2),
        ("NK1", 5),
        ("NK1", 6),
        ("PV1", 7),
        ("PV1", 8),
        ("PV1", 9),
        ("PV1", 17),
        ("ORC", 12),
        ("OBR", 16),
        ("OBX", 16),
        ("RXA", 10),
        ("RXO", 14),
        ("RXO", 15),
        ("GT1", 3),
        ("GT1", 4),
        ("GT1", 6),
        ("GT1", 7),
        ("GT1", 19),
        ("IN1", 16),
        ("MRG", 7),
    ];
    let shifted: &[(&str, usize)] = &[
        ("MSH", 7),
        ("EVN", 2),
        ("EVN", 6),
        ("PID", 7),
        ("PID", 29),
        ("PID", 33),
        ("PD1", 19),
        ("PV1", 44),
        ("PV1", 45),
        ("ORC", 9),
        ("ORC", 15),
        ("OBR", 7),
        ("OBR", 8),
        ("OBR", 22),
        ("OBX", 14),
        ("OBX", 19),
        ("TQ1", 7),
        ("TQ1", 8),
        ("RXA", 3),
        ("RXA", 4),
        ("RXA", 16),
        ("RXA", 22),
        ("PRB", 2),
        ("PRB", 7),
        ("PRB", 8),
        ("PRB", 9),
        ("PRB", 15),
        ("PRB", 16),
        ("PRB", 24),
        ("PRB", 25),
        ("DG1", 5),
        ("GT1", 8),
        ("IN1", 12),
        ("IN1", 13),
        ("IN1", 14),
        ("IN1", 15),
        ("AL1", 6),
        ("SPM", 17),
        ("SPM", 18),
    ];
    let mut rules = Vec::new();
    for (segment, field) in hashed {
        rules.push(Rule::field(segment, *field, Hash));
    }
    for (segment, field) in removed {
        rules.push(Rule::field(segment, *field, Remove));
    }
    for (segment, field) in shifted {
        rules.push(Rule::field(segment, *field, DateShift));
    }
    // Addresses: drop street/city/geo, generalize ZIP to 3 digits, keep
    // state/country/type. Applies to PID-11, NK1-4, GT1-5.
    for (segment, field) in [("PID", 11), ("NK1", 4), ("GT1", 5)] {
        for component in [1, 2, 3, 8, 9, 10] {
            rules.push(Rule::component(segment, field, component, Remove));
        }
        rules.push(Rule::component(segment, field, 5, Truncate(3)));
    }
    rules
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    fn sample() -> Message {
        parse_message(
            "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG001|P|2.5\r\
             EVN||20240101120000\r\
             PID|1||PAT123^^^HOSP^MR||Smith^John^A||19800101|M|||123 Main St^^Springfield^IL^62701^USA||||||||123-45-6789\r\
             PV1|1|I|WARD^^^FAC||||DOC123^House^Greg|||||||||||VISIT999\r\
             OBX|1|NM|WBC^White Count||8.5|K/uL|||||F|||20240101130000\r\
             DG1|1||I10^Hypertension^ICD10||20240101",
        )
        .unwrap()
    }

    fn hash_of(key: &[u8], value: &str) -> String {
        Redactor::new().with_hmac_key(key).hash(value)
    }

    #[test]
    fn hash_is_deterministic_keyed_and_short() {
        let a = hash_of(b"key-1", "PAT123");
        let b = hash_of(b"key-1", "PAT123");
        let c = hash_of(b"key-2", "PAT123");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 12);
        assert!(a.bytes().all(|ch| ch.is_ascii_hexdigit()));
    }

    #[test]
    fn shift_date_keeps_time_and_handles_edges() {
        assert_eq!(shift_date("19800101", 30).as_deref(), Some("19800131"));
        assert_eq!(
            shift_date("20240101120000", 1).as_deref(),
            Some("20240102120000")
        );
        assert_eq!(shift_date("20240228", 1).as_deref(), Some("20240229")); // leap day
        assert_eq!(shift_date("20240101", -1).as_deref(), Some("20231231"));
        assert_eq!(shift_date("2024", 5), None); // too short
        assert_eq!(shift_date("not-a-date", 5), None);
    }

    #[test]
    fn hash_rule_requires_a_key() {
        let redactor = Redactor::new().with_rule(Rule::field("PID", 3, Strategy::Hash));
        let mut message = sample();
        assert_eq!(
            redactor.redact(&mut message),
            Err(RedactError::MissingHmacKey)
        );
    }

    #[test]
    fn whole_field_hash_applies_per_repetition() {
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||AAA~BBB\r").unwrap();
        let redactor =
            Redactor::new()
                .with_hmac_key(b"k")
                .with_rule(Rule::field("PID", 3, Strategy::Hash));
        let report = redactor.redact(&mut message).unwrap();

        let field = message
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(3)
            .unwrap();
        assert_eq!(field.repetitions.len(), 2);
        let first = field.repetitions[0].full_value().unwrap();
        let second = field.repetitions[1].full_value().unwrap();
        assert_ne!(first, second);
        assert_eq!(first, hash_of(b"k", "AAA"));
        assert_eq!(
            report,
            vec![Redaction {
                path: "PID[0]-3".to_string(),
                strategy: "Hash",
            }]
        );
    }

    #[test]
    fn component_rule_applies_to_every_repetition() {
        let mut message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||||||||||^^^IL^62701~^^^IL^60601\r",
        )
        .unwrap();
        let redactor =
            Redactor::new().with_rule(Rule::component("PID", 11, 5, Strategy::Truncate(3)));
        redactor.redact(&mut message).unwrap();

        let field = message
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(11)
            .unwrap();
        let zips: Vec<_> = field
            .repetitions
            .iter()
            .map(|r| r.get_component(4).unwrap().value().unwrap().to_string())
            .collect();
        assert_eq!(zips, vec!["627".to_string(), "606".to_string()]);
    }

    #[test]
    fn wildcard_segment_and_fixed_mask_remove() {
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rNTE|1||note here\r")
                .unwrap();
        let redactor = Redactor::new()
            .with_rule(Rule::field("NTE", 1, Strategy::Fixed("9".to_string())))
            .with_rule(Rule::field("*", 3, Strategy::Mask));
        let report = redactor.redact(&mut message).unwrap();

        let nte = message.get_segment_by_id("NTE").unwrap();
        assert_eq!(nte.get_field_value(1), Some("9"));
        assert_eq!(nte.get_field_value(3), Some("***"));
        // MSH-3 ("A") is also field 3 of a segment: wildcard matched it too.
        assert_eq!(
            message.get_segment_by_id("MSH").unwrap().get_field_value(3),
            Some("***")
        );
        assert_eq!(report.len(), 3);
    }

    #[test]
    fn absent_fields_are_untouched_and_unreported() {
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\r").unwrap();
        let redactor = Redactor::safe_harbor(b"k", 7);
        let report = redactor.redact(&mut message).unwrap();

        // PID-3 etc. absent: no rule fired for them, no structure created.
        assert!(!report.iter().any(|r| r.path.starts_with("PID")));
        assert!(
            message
                .get_segment_by_id("PID")
                .unwrap()
                .get_field(3)
                .is_none()
        );
    }

    #[test]
    fn unparseable_dates_are_removed_not_kept() {
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||||||unknown\r")
                .unwrap();
        // PID-7 = "unknown": not a date, must not survive.
        let redactor = Redactor::new().with_rule(Rule::field("PID", 7, Strategy::DateShift));
        redactor.redact(&mut message).unwrap();
        assert!(
            message
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(7)
                .is_none()
        );
    }

    #[test]
    fn safe_harbor_profile_clears_phi_and_keeps_clinical_data() {
        let mut message = sample();
        let redactor = Redactor::safe_harbor(b"site-secret", 30);
        let report = redactor.redact(&mut message).unwrap();
        assert!(!report.is_empty());

        let pid = message.get_segment_by_id("PID").unwrap();
        assert!(pid.get_field_value(5).is_none()); // name
        assert!(pid.get_field_value(19).is_none()); // SSN
        assert_eq!(pid.get_field_value(7), Some("19800131")); // DOB +30d
        assert_eq!(pid.get_field_value(8), Some("M")); // sex kept
        let hashed_id = pid.get_field(3).unwrap().repetitions[0]
            .full_value()
            .unwrap();
        assert_eq!(hashed_id, hash_of(b"site-secret", "PAT123^^^HOSP^MR"));
        // Address: street/city gone, ZIP generalized, state kept.
        let addr = pid.get_field(11).unwrap().get_repetition(0).unwrap();
        assert_eq!(addr.get_component(0).unwrap().value(), Some(""));
        assert_eq!(addr.get_component(4).unwrap().value(), Some("627"));
        assert_eq!(addr.get_component(3).unwrap().value(), Some("IL"));

        let pv1 = message.get_segment_by_id("PV1").unwrap();
        assert!(pv1.get_field_value(7).is_none()); // attending doctor
        assert_eq!(pv1.get_field_value(2), Some("I")); // patient class kept
        assert_ne!(pv1.get_field_value(19), Some("VISIT999")); // visit number hashed

        // Clinical data intact, datetimes shifted.
        let obx = message.get_segment_by_id("OBX").unwrap();
        assert_eq!(obx.get_field_value(5), Some("8.5"));
        assert_eq!(obx.get_field_value(14), Some("20240131130000"));
        let dg1 = message.get_segment_by_id("DG1").unwrap();
        assert_eq!(dg1.get_field_value(3), Some("I10"));
        assert_eq!(dg1.get_field_value(5), Some("20240131"));

        let msh = message.get_segment_by_id("MSH").unwrap();
        assert_eq!(msh.get_field_value(7), Some("20240131120000"));
        assert_eq!(msh.get_field_value(10).unwrap().len(), 12); // control ID hashed

        // Deterministic linkage: same key, same pseudonyms.
        let mut again = sample();
        Redactor::safe_harbor(b"site-secret", 30)
            .redact(&mut again)
            .unwrap();
        let again_id = again
            .get_segment_by_id("PID")
            .unwrap()
            .get_field(3)
            .unwrap()
            .repetitions[0]
            .full_value()
            .unwrap();
        assert_eq!(hashed_id, again_id);

        // Report paths point at what changed.
        assert!(
            report
                .iter()
                .any(|r| r.path == "PID[0]-5" && r.strategy == "Remove")
        );
        assert!(
            report
                .iter()
                .any(|r| r.path == "PID[0]-3" && r.strategy == "Hash")
        );

        // Still a parseable message. (The parser materializes the emptied
        // field as Some(""), so assert emptiness rather than absence.)
        let encoded = message.encode();
        let reparsed = parse_message(&encoded).unwrap();
        assert!(
            reparsed
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(5)
                .unwrap_or("")
                .is_empty()
        );
    }
}
