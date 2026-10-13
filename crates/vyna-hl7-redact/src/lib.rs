//! Safe Harbor de-identification for HL7 v2 messages.
//!
//! [`Redactor`] rewrites protected health information (PHI) in place using
//! explicit, auditable rules. [`Redactor::safe_harbor`] ships a profile
//! covering the HIPAA Safe Harbor identifiers (names, addresses, telecom,
//! dates, IDs, account and visit numbers) across 34 segments: ACC, AL1,
//! BLG, CTD, DG1, EVN, FT1, GT1, IN1, IN2, IN3, MRG, MSH, NK1, OBR, OBX,
//! ORC, PD1, PDA, PID, PR1, PRB, PRD, PV1, ROL, RXA, RXD, RXE, RXO, SCH,
//! SPM, STF, TQ1, TXA.
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
//! - Free-text segments (NTE) and OBX-5 observation values are deliberately
//!   **not** in the default profile: blunt removal would destroy clinical
//!   notes, and real scrubbing needs NLP. Site-defined Z-segments are
//!   likewise **not** covered — their layout is site-specific, so the
//!   profile cannot know which fields hold PHI. Segments outside the
//!   34 listed above (e.g. ARQ, AUT, FAC, IAM, LAN, MFA, PRC, RFI, UB1,
//!   UB2, VAR) are also **not** covered. Add explicit rules if your
//!   policy requires them.

use chrono::{Duration, NaiveDate};
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use thiserror::Error;
use vyna_hl7_core::{Delimiters, Field, Message, Repetition, Segment};

/// Redaction failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RedactError {
    /// A [`Strategy::Hash`] rule ran without an HMAC key configured.
    #[error("hash strategy requires an HMAC key")]
    MissingHmacKey,
    /// A rule used field 0 or component 0 (numbering is 1-based).
    #[error("invalid rule: {0}")]
    InvalidRule(String),
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
    ///
    /// The hashed input is the value's wire-faithful ER7 encoding
    /// (subcomponents and escapes intact), so distinct identities
    /// never share a digest. Deterministic: same input, same digest.
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
    ///
    /// `segment` is matched exactly (case-sensitive); a typo or a
    /// lowercase ID matches nothing. `field` is 1-based: 0 is rejected
    /// by [`Redactor::redact`] with [`RedactError::InvalidRule`].
    pub fn field(segment: &str, field: usize, strategy: Strategy) -> Self {
        Self {
            segment: segment.to_string(),
            field,
            component: None,
            strategy,
        }
    }

    /// Component rule.
    ///
    /// `segment` is matched exactly (case-sensitive). `field` and
    /// `component` are 1-based: 0 is rejected by [`Redactor::redact`]
    /// with [`RedactError::InvalidRule`].
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
#[derive(Clone, Default)]
pub struct Redactor {
    rules: Vec<Rule>,
    date_shift_days: i64,
    hmac_key: Option<Vec<u8>>,
}

// The HMAC key must never leak into logs via `{:?}`: report presence and
// length only, never bytes (mirrors the `FtpConfig`/`SftpAuth` redaction).
impl std::fmt::Debug for Redactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key = self
            .hmac_key
            .as_ref()
            .map(|key| format!("<redacted: {} bytes>", key.len()));
        f.debug_struct("Redactor")
            .field("rules", &self.rules)
            .field("date_shift_days", &self.date_shift_days)
            .field("hmac_key", &key)
            .finish()
    }
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

    /// HMAC key for [`Strategy::Hash`]. Empty keys carry no entropy and
    /// are treated as missing.
    pub fn with_hmac_key(mut self, key: &[u8]) -> Self {
        self.hmac_key = if key.is_empty() {
            None
        } else {
            Some(key.to_vec())
        };
        self
    }

    /// Append the Safe Harbor profile rules (see module docs).
    ///
    /// Free-text notes (NTE, OBX-5), site-defined Z-segments, and any
    /// segment outside the 34 covered ones are not covered; add explicit
    /// [`Rule`]s when your policy requires them.
    pub fn with_safe_harbor_rules(mut self) -> Self {
        self.rules.extend(safe_harbor_rules());
        self
    }

    /// Redactor with the full Safe Harbor profile.
    ///
    /// Covers the 34 segments listed in the module docs. Free-text notes
    /// (NTE, OBX-5), site-defined Z-segments, and uncovered segments are
    /// intentionally left untouched — add explicit [`Rule`]s for them
    /// when your policy requires it:
    ///
    /// ```rust
    /// use vyna_hl7_redact::{Redactor, Rule, Strategy};
    ///
    /// let _redactor = Redactor::safe_harbor(b"key", 30)
    ///     .with_rule(Rule::field("NTE", 3, Strategy::Remove));
    /// ```
    pub fn safe_harbor(hmac_key: &[u8], date_shift_days: i64) -> Self {
        Self::new()
            .with_hmac_key(hmac_key)
            .with_date_shift_days(date_shift_days)
            .with_safe_harbor_rules()
    }

    /// Redact `message` in place, returning what was changed.
    ///
    /// An empty report means no rule changed anything — either the
    /// message held no matching values (normal) or a rule never matched
    /// (e.g. a typo or lowercase segment ID, which matches nothing:
    /// matching is exact and case-sensitive). Use
    /// [`unmatched_rules`](Self::unmatched_rules) to tell the two apart.
    pub fn redact(&self, message: &mut Message) -> Result<Vec<Redaction>, RedactError> {
        let key_missing = self.hmac_key.as_ref().is_none_or(|key| key.is_empty());
        if self.rules.iter().any(|r| r.strategy == Strategy::Hash) && key_missing {
            return Err(RedactError::MissingHmacKey);
        }
        for rule in &self.rules {
            if rule.field == 0 {
                return Err(RedactError::InvalidRule(format!(
                    "field 0 in rule for segment '{}' (fields are 1-based)",
                    rule.segment
                )));
            }
            if rule.component == Some(0) {
                return Err(RedactError::InvalidRule(format!(
                    "component 0 in rule for segment '{}' (components are 1-based)",
                    rule.segment
                )));
            }
        }
        let mut report = Vec::new();
        // Copied up front: the loop below mutably borrows segments.
        let delimiters = message.delimiters;
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
                        if self.redact_field(segment, rule, &delimiters)? {
                            report.push(Redaction {
                                path: format!("{}[{here}]-{}", segment.id, rule.field),
                                strategy: rule.strategy.name(),
                            });
                        }
                    }
                    Some(component) => {
                        if self.redact_component(
                            segment,
                            rule.field,
                            component,
                            &rule.strategy,
                            &delimiters,
                        )? {
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

    /// Rules whose segment selector matched no segment in `message`.
    ///
    /// Segment matching is exact and case-sensitive, so a typo or a
    /// lowercase ID (e.g. `"pid"`) silently matches nothing and
    /// [`redact`](Self::redact) returns an empty report. Call this after
    /// a suspiciously empty report to tell "no PHI present" apart from
    /// "rule never matched". `"*"` rules match any segment, so they are
    /// only listed when the message has no segments at all. A rule is
    /// *not* listed when its segment is present but the field is absent
    /// — that is normal (rules only touch values that exist).
    pub fn unmatched_rules(&self, message: &Message) -> Vec<Rule> {
        self.rules
            .iter()
            .filter(|rule| {
                !message
                    .segments
                    .iter()
                    .any(|segment| rule.segment == "*" || rule.segment == segment.id)
            })
            .cloned()
            .collect()
    }

    /// Rewrite each repetition's whole value; true when anything changed.
    fn redact_field(
        &self,
        segment: &mut Segment,
        rule: &Rule,
        delimiters: &Delimiters,
    ) -> Result<bool, RedactError> {
        let Some(field) = segment.get_field(rule.field) else {
            return Ok(false);
        };
        // Hash input is the wire-faithful encoding (subcomponents and
        // escapes intact): the display join drops subcomponent content
        // and merges distinct identities. Other strategies keep the
        // display value so their output is unchanged.
        let values: Vec<String> = if rule.strategy == Strategy::Hash {
            field
                .repetitions
                .iter()
                .map(|r| {
                    if r.is_empty() {
                        String::new()
                    } else {
                        r.encode(delimiters)
                    }
                })
                .collect()
        } else {
            field
                .repetitions
                .iter()
                .map(|r| r.full_value().unwrap_or_default())
                .collect()
        };
        if values.iter().all(|v| v.is_empty()) {
            return Ok(false);
        }
        if rule.strategy == Strategy::Remove {
            let _ = segment.set_field(rule.field, Field::new());
            return Ok(true);
        }
        let mut rebuilt = Field::new();
        for value in &values {
            let text = value.as_str();
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
        delimiters: &Delimiters,
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
                // Hash input is the wire-faithful encoding: the naive
                // "&"-join cannot tell structured subcomponents from
                // literal "&" text. Other strategies keep the join.
                let text: String = if *strategy == Strategy::Hash {
                    if comp.is_empty() {
                        String::new()
                    } else {
                        comp.encode(delimiters)
                    }
                } else {
                    comp.subcomponents
                        .iter()
                        .map(|s| s.value.as_str())
                        .collect::<Vec<_>>()
                        .join("&")
                };
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
    // Byte-slice via `get` so short values yield `None` instead of
    // panicking; ASCII-only prefix means index 8 is a char boundary.
    let prefix = value.as_bytes().get(..8)?;
    if !prefix.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let date = NaiveDate::parse_from_str(&value[..8], "%Y%m%d").ok()?;
    // Checked arithmetic: extreme shifts overflow `Duration::days` or push
    // the date out of range, and both must remove the value, never panic.
    let shifted = date.checked_add_signed(Duration::try_days(days)?)?;
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
        ("PD1", 10),
        ("PV1", 5),
        ("PV1", 19),
        ("PV1", 50),
        ("ORC", 2),
        ("ORC", 3),
        ("ORC", 4),
        ("ORC", 8),
        ("OBR", 2),
        ("OBR", 3),
        ("OBR", 26),
        ("OBR", 29),
        ("GT1", 2),
        ("GT1", 29),
        ("IN1", 10),
        ("IN1", 28),
        ("IN1", 36),
        ("IN1", 49),
        ("NK1", 12),
        ("NK1", 33),
        ("BLG", 3),
        ("IN3", 2),
        ("MRG", 1),
        ("MRG", 2),
        ("MRG", 3),
        ("MRG", 4),
        ("MRG", 5),
        ("MRG", 6),
        ("SPM", 2),
        ("SPM", 3),
        ("STF", 1),
        ("STF", 2),
        ("STF", 14),
        ("STF", 22),
        ("PRD", 7),
        ("TXA", 12),
        ("TXA", 13),
        ("TXA", 14),
        ("TXA", 15),
        ("TXA", 16),
        ("IN2", 1),
        ("IN2", 6),
        ("IN2", 8),
        ("IN2", 10),
        ("IN2", 25),
        ("IN2", 26),
        ("IN2", 61),
        ("FT1", 2),
        ("FT1", 23),
        ("FT1", 30),
        ("FT1", 31),
        ("RXD", 7),
        ("RXE", 15),
        ("CTD", 7),
        ("SCH", 1),
        ("SCH", 2),
        ("SCH", 4),
        ("SCH", 23),
        ("SCH", 24),
        ("SCH", 26),
        ("SCH", 27),
        ("DG1", 20),
        ("ROL", 1),
        ("PR1", 19),
        ("PRB", 4),
        ("PRB", 5),
    ];
    let removed: &[(&str, usize)] = &[
        ("PID", 5),
        ("PID", 6),
        ("PID", 9),
        ("PID", 12),
        ("PID", 13),
        ("PID", 14),
        ("PID", 19),
        ("PID", 20),
        ("PID", 23),
        ("PD1", 4),
        ("NK1", 2),
        ("NK1", 5),
        ("NK1", 6),
        ("NK1", 26),
        ("NK1", 30),
        ("NK1", 31),
        ("NK1", 37),
        ("NK1", 38),
        ("PV1", 7),
        ("PV1", 8),
        ("PV1", 9),
        ("PV1", 17),
        ("PV1", 52),
        ("EVN", 5),
        ("ORC", 10),
        ("ORC", 11),
        ("ORC", 12),
        ("ORC", 14),
        ("ORC", 19),
        ("ORC", 23),
        ("OBR", 10),
        ("OBR", 16),
        ("OBR", 17),
        ("OBR", 28),
        ("OBR", 32),
        ("OBR", 33),
        ("OBR", 34),
        ("OBR", 35),
        ("OBX", 16),
        ("OBX", 25),
        ("RXA", 10),
        ("RXO", 14),
        ("RXO", 15),
        ("RXD", 10),
        ("RXE", 13),
        ("RXE", 14),
        ("GT1", 3),
        ("GT1", 4),
        ("GT1", 6),
        ("GT1", 7),
        ("GT1", 12),
        ("GT1", 16),
        ("GT1", 18),
        ("GT1", 19),
        ("GT1", 42),
        ("GT1", 45),
        ("GT1", 46),
        ("GT1", 56),
        ("IN1", 6),
        ("IN1", 7),
        ("IN1", 16),
        ("IN1", 30),
        ("IN1", 52),
        ("IN2", 2),
        ("IN2", 3),
        ("IN2", 7),
        ("IN2", 9),
        ("IN2", 22),
        ("IN2", 40),
        ("IN2", 49),
        ("IN2", 50),
        ("IN2", 52),
        ("IN2", 53),
        ("IN2", 58),
        ("IN2", 63),
        ("IN2", 64),
        ("SCH", 12),
        ("SCH", 13),
        ("SCH", 16),
        ("SCH", 17),
        ("SCH", 20),
        ("SCH", 21),
        ("ROL", 4),
        ("ROL", 12),
        ("PR1", 8),
        ("PR1", 11),
        ("PR1", 12),
        ("CTD", 2),
        ("CTD", 5),
        ("FT1", 4),
        ("FT1", 20),
        ("FT1", 21),
        ("FT1", 24),
        ("ACC", 7),
        ("ACC", 9),
        ("PDA", 5),
        ("PDA", 7),
        ("PDA", 8),
        ("IN3", 3),
        ("IN3", 8),
        ("IN3", 14),
        ("IN3", 15),
        ("IN3", 16),
        ("IN3", 19),
        ("IN3", 21),
        ("IN3", 25),
        ("MRG", 7),
        ("DG1", 16),
        ("STF", 3),
        ("STF", 10),
        ("STF", 15),
        ("STF", 34),
        ("PRD", 2),
        ("PRD", 5),
        ("PRD", 10),
        ("PRD", 13),
        ("TXA", 5),
        ("TXA", 9),
        ("TXA", 10),
        ("TXA", 11),
        ("TXA", 22),
        ("TXA", 23),
        ("SPM", 17),
    ];
    let shifted: &[(&str, usize)] = &[
        ("MSH", 7),
        ("EVN", 2),
        ("EVN", 3),
        ("EVN", 6),
        ("PID", 7),
        ("PID", 29),
        ("PID", 33),
        ("PD1", 13),
        ("PD1", 17),
        ("PD1", 18),
        ("PV1", 25),
        ("PV1", 30),
        ("PV1", 35),
        ("PV1", 44),
        ("PV1", 45),
        ("ORC", 9),
        ("ORC", 15),
        ("ORC", 27),
        ("OBR", 6),
        ("OBR", 7),
        ("OBR", 8),
        ("OBR", 14),
        ("OBR", 22),
        ("OBR", 36),
        ("OBX", 12),
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
        ("DG1", 5),
        ("DG1", 19),
        ("GT1", 8),
        ("GT1", 13),
        ("GT1", 14),
        ("GT1", 24),
        ("GT1", 31),
        ("GT1", 32),
        ("IN1", 12),
        ("IN1", 13),
        ("IN1", 18),
        ("IN1", 24),
        ("IN1", 26),
        ("IN1", 29),
        ("IN1", 51),
        ("NK1", 8),
        ("NK1", 9),
        ("NK1", 16),
        ("AL1", 6),
        ("SPM", 18),
        ("SPM", 19),
        ("STF", 6),
        ("STF", 12),
        ("STF", 13),
        ("STF", 24),
        ("STF", 25),
        ("STF", 26),
        ("STF", 31),
        ("STF", 35),
        ("PRD", 8),
        ("PRD", 9),
        ("TXA", 4),
        ("TXA", 6),
        ("TXA", 7),
        ("TXA", 8),
        ("PDA", 4),
        ("IN3", 6),
        ("IN3", 7),
        ("IN3", 9),
        ("IN3", 10),
        ("IN3", 13),
        ("IN3", 22),
        ("RXD", 3),
        ("RXD", 19),
        ("RXE", 18),
        ("RXE", 32),
        ("FT1", 5),
        ("ROL", 5),
        ("ROL", 6),
        ("PR1", 5),
        ("ACC", 1),
        ("IN2", 17),
        ("IN2", 44),
        ("IN2", 45),
        ("IN2", 55),
        ("IN2", 56),
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
    // state/country/type. Applies to PID-11, NK1-4/32, GT1-5/17,
    // STF-11, PRD-3/11, IN1-19/44, ORC-22/24, SCH-14/18, ROL-11,
    // CTD-3, RXD-31, RXE-41/43, ACC-11.
    for (segment, field) in [
        ("PID", 11),
        ("NK1", 4),
        ("NK1", 32),
        ("GT1", 5),
        ("GT1", 17),
        ("STF", 11),
        ("PRD", 3),
        ("PRD", 11),
        ("IN1", 19),
        ("IN1", 44),
        ("ORC", 22),
        ("ORC", 24),
        ("SCH", 14),
        ("SCH", 18),
        ("ROL", 11),
        ("CTD", 3),
        ("RXD", 31),
        ("RXE", 41),
        ("RXE", 43),
        ("ACC", 11),
    ] {
        for component in [1, 2, 3, 8, 9, 10] {
            rules.push(Rule::component(segment, field, component, Remove));
        }
        rules.push(Rule::component(segment, field, 5, Truncate(3)));
    }
    // Timing/quantity fields (TQ): components 4/5 are TS Start/End
    // Date-Time and shift while quantity/interval/priority are kept.
    for (segment, field) in [("ORC", 7), ("SCH", 11), ("RXE", 1), ("OBR", 27)] {
        rules.push(Rule::component(segment, field, 4, DateShift));
        rules.push(Rule::component(segment, field, 5, DateShift));
    }
    // IN1-14 Authorization Information (AUI): only component 2 (DT)
    // is a date; the authorization number and source are kept.
    rules.push(Rule::component("IN1", 14, 2, DateShift));
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
    fn extreme_date_shifts_remove_the_value_instead_of_panicking() {
        // `Duration::days(i64::MAX)` overflows and `date + days` panics
        // outside the representable range; both must yield `None` so the
        // value is removed, never a panic.
        assert_eq!(shift_date("20240101", i64::MAX), None);
        assert_eq!(shift_date("20240101", i64::MIN), None);
        assert_eq!(shift_date("2621431231", 1), None);
        // End to end: the overflowing value is removed, not kept.
        let mut message = parse_message(concat!(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r",
            "PID|1||||||20240101",
        ))
        .unwrap();
        Redactor::new()
            .with_date_shift_days(i64::MAX)
            .with_rule(Rule::field("PID", 7, Strategy::DateShift))
            .redact(&mut message)
            .unwrap();
        assert!(
            message
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(7)
                .is_none()
        );
    }

    #[test]
    fn unmatched_rules_lists_wildcard_only_for_empty_messages() {
        let redactor = Redactor::new().with_rule(Rule::field("*", 3, Strategy::Mask));

        // No segments at all: even "*" matches nothing.
        let empty = Message::new();
        assert_eq!(redactor.unmatched_rules(&empty).len(), 1);

        // Any segment satisfies a wildcard rule.
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\r").unwrap();
        assert!(redactor.unmatched_rules(&message).is_empty());
    }

    #[test]
    fn whole_field_remove_clears_every_repetition() {
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||AAA~BBB\r").unwrap();
        let redactor = Redactor::new().with_rule(Rule::field("PID", 3, Strategy::Remove));
        let report = redactor.redact(&mut message).unwrap();

        // One report entry for the field, both repetitions gone.
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].path, "PID[0]-3");
        let pid = message.get_segment_by_id("PID").unwrap();
        assert!(pid.get_field(3).unwrap().is_empty());
    }

    #[test]
    fn out_of_range_component_rule_is_a_quiet_noop() {
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||||Smith^John\r")
                .unwrap();
        // PID-5 has 2 components; component 9 exists nowhere.
        let redactor = Redactor::new().with_rule(Rule::component("PID", 5, 9, Strategy::Remove));
        let report = redactor.redact(&mut message).unwrap();

        assert!(report.is_empty());
        let pid = message.get_segment_by_id("PID").unwrap();
        let value = pid.get_field(5).unwrap().repetitions[0]
            .full_value()
            .unwrap();
        assert_eq!(value, "Smith^John");
    }

    #[test]
    fn unmatched_rules_flags_typos_but_not_absent_fields() {
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\r").unwrap();

        // Lowercase segment ID: redact succeeds with an empty report.
        let redactor = Redactor::new().with_rule(Rule::field("pid", 5, Strategy::Remove));
        let mut message = message.clone();
        assert!(redactor.redact(&mut message).unwrap().is_empty());
        // ...but the rule is reported as never matched.
        assert_eq!(redactor.unmatched_rules(&message).len(), 1);

        // Correct segment, absent field: empty report is normal, and the
        // rule is *not* flagged.
        let redactor = Redactor::new().with_rule(Rule::field("PID", 5, Strategy::Remove));
        assert!(redactor.redact(&mut message).unwrap().is_empty());
        assert!(redactor.unmatched_rules(&message).is_empty());
    }

    #[test]
    fn zero_field_or_component_is_rejected() {
        let mut message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1\r").unwrap();
        let err = Redactor::new()
            .with_rule(Rule::field("PID", 0, Strategy::Remove))
            .redact(&mut message)
            .unwrap_err();
        assert!(matches!(err, RedactError::InvalidRule(_)));

        let err = Redactor::new()
            .with_rule(Rule::component("PID", 5, 0, Strategy::Remove))
            .redact(&mut message)
            .unwrap_err();
        assert!(matches!(err, RedactError::InvalidRule(_)));
    }

    #[test]
    fn safe_harbor_leaves_free_text_and_z_segments_untouched() {
        let mut message = parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r\
             NTE|1||Patient called from 555-1234\r\
             OBX|1|TX|NOTE^Note||Patient lives alone||||||F\r\
             ZPV|1|SECRET",
        )
        .unwrap();
        Redactor::safe_harbor(b"k", 7).redact(&mut message).unwrap();

        // Documented gap: blunt removal would destroy clinical notes, and
        // Z-segment layouts are site-specific. Add explicit rules when
        // policy requires them.
        let nte = message.get_segment_by_id("NTE").unwrap();
        assert_eq!(nte.get_field_value(3), Some("Patient called from 555-1234"));
        let obx = message.get_segment_by_id("OBX").unwrap();
        assert_eq!(obx.get_field_value(5), Some("Patient lives alone"));
        let zpv = message.get_segment_by_id("ZPV").unwrap();
        assert_eq!(zpv.get_field_value(2), Some("SECRET"));
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

    #[test]
    fn date_shift_never_panics_on_short_or_multibyte_values() {
        // Byte 8 lands inside 'é': `len() >= 8`, but `[..8]` would panic.
        assert_eq!(shift_date("1234567é8900", 30), None);
        assert_eq!(shift_date("2024010é1", 30), None);
        assert_eq!(shift_date("éééééééé", 30), None);
        assert_eq!(shift_date("", 30), None);
        assert_eq!(shift_date("2024", 30), None);
        // End to end: the unparseable value is removed, not kept.
        let mut message = parse_message(concat!(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\r",
            "PID|1||||||1234567é8900",
        ))
        .unwrap();
        Redactor::new()
            .with_rule(Rule::field("PID", 7, Strategy::DateShift))
            .redact(&mut message)
            .unwrap();
        assert!(
            message
                .get_segment_by_id("PID")
                .unwrap()
                .get_field_value(7)
                .is_none()
        );
    }

    #[test]
    fn safe_harbor_redacts_provider_and_staff_identifiers() {
        let mut message = parse_message(concat!(
            "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG001|P|2.5\r",
            "DG1|1||I10^Hypertension^ICD10||20240101|||||||||||",
            "DOC999^House^Greg\r",
            "STF|KEY1|STAFF42|Nightingale^Florence|N|F|19700515|A|FT|",
            "MED|555-0100|10 Ward Way^^Springfield^IL^62701^USA|20200101|",
            "20250101|BACKUP1|nurse@example.com\r",
            "PRD|PP^Primary Care|House^Greg|",
            "221 B St^^Princeton^NJ^08540||555-0200||NPI1234567|",
            "20200101|20250101|Princeton Hospital|",
            "1 Hospital Dr^^Princeton^NJ^08540||555-0300\r",
            "TXA|1|DS||20240101100000|DOC1^Author^Amy|20240101110000|",
            "20240102120000|20240103120000|DOC2^Orig^Olive|",
            "DOC3^Auth^Al|TR1^Trans^Tina|DOCNUM1|DOCNUM0|PLAC1|",
            "FILL1|file001.pdf",
        ))
        .unwrap();
        let key = b"site-secret";
        Redactor::safe_harbor(key, 30).redact(&mut message).unwrap();

        // Diagnosing clinician gone; code kept, date shifted.
        let dg1 = message.get_segment_by_id("DG1").unwrap();
        assert!(dg1.get_field_value(16).is_none());
        assert_eq!(dg1.get_field_value(3), Some("I10"));
        assert_eq!(dg1.get_field_value(5), Some("20240131"));

        // Staff IDs hashed, name/phone/email removed, dates shifted,
        // address generalized, sex kept.
        let stf = message.get_segment_by_id("STF").unwrap();
        let expect_key1 = hash_of(key, "KEY1");
        assert_eq!(stf.get_field_value(1), Some(expect_key1.as_str()));
        let expect_staff = hash_of(key, "STAFF42");
        assert_eq!(stf.get_field_value(2), Some(expect_staff.as_str()));
        assert!(stf.get_field_value(3).is_none());
        assert_eq!(stf.get_field_value(5), Some("F"));
        assert_eq!(stf.get_field_value(6), Some("19700614"));
        assert!(stf.get_field_value(10).is_none());
        assert_eq!(stf.get_field_value(12), Some("20200131"));
        assert_eq!(stf.get_field_value(13), Some("20250131"));
        let expect_backup = hash_of(key, "BACKUP1");
        assert_eq!(stf.get_field_value(14), Some(expect_backup.as_str()));
        assert!(stf.get_field_value(15).is_none());
        let addr = stf.get_field(11).unwrap().get_repetition(0).unwrap();
        assert_eq!(addr.get_component(0).unwrap().value(), Some(""));
        assert_eq!(addr.get_component(4).unwrap().value(), Some("627"));
        assert_eq!(addr.get_component(3).unwrap().value(), Some("IL"));

        // Provider name/telecom/org removed, IDs hashed, dates shifted.
        let prd = message.get_segment_by_id("PRD").unwrap();
        assert_eq!(prd.get_field_value(1), Some("PP")); // role kept
        assert!(prd.get_field_value(2).is_none());
        assert!(prd.get_field_value(5).is_none());
        let expect_npi = hash_of(key, "NPI1234567");
        assert_eq!(prd.get_field_value(7), Some(expect_npi.as_str()));
        assert_eq!(prd.get_field_value(8), Some("20200131"));
        assert_eq!(prd.get_field_value(9), Some("20250131"));
        assert!(prd.get_field_value(10).is_none());
        assert!(prd.get_field_value(13).is_none());
        let addr = prd.get_field(3).unwrap().get_repetition(0).unwrap();
        assert_eq!(addr.get_component(0).unwrap().value(), Some(""));
        assert_eq!(addr.get_component(4).unwrap().value(), Some("085"));
        let org = prd.get_field(11).unwrap().get_repetition(0).unwrap();
        assert_eq!(org.get_component(0).unwrap().value(), Some(""));
        assert_eq!(org.get_component(4).unwrap().value(), Some("085"));

        // Transcription authors removed, dates shifted, numbers hashed.
        let txa = message.get_segment_by_id("TXA").unwrap();
        assert_eq!(txa.get_field_value(1), Some("1"));
        assert_eq!(txa.get_field_value(2), Some("DS"));
        assert_eq!(txa.get_field_value(4), Some("20240131100000"));
        assert!(txa.get_field_value(5).is_none());
        assert_eq!(txa.get_field_value(6), Some("20240131110000"));
        assert_eq!(txa.get_field_value(7), Some("20240201120000"));
        assert_eq!(txa.get_field_value(8), Some("20240202120000"));
        assert!(txa.get_field_value(9).is_none());
        assert!(txa.get_field_value(10).is_none());
        assert!(txa.get_field_value(11).is_none());
        for (field, original) in [
            (12, "DOCNUM1"),
            (13, "DOCNUM0"),
            (14, "PLAC1"),
            (15, "FILL1"),
            (16, "file001.pdf"),
        ] {
            let expected = hash_of(key, original);
            assert_eq!(
                txa.get_field_value(field),
                Some(expected.as_str()),
                "TXA-{field} must be hashed"
            );
        }
    }

    #[test]
    fn empty_hmac_key_is_rejected_like_a_missing_key() {
        let mut message = sample();
        let redactor =
            Redactor::new()
                .with_hmac_key(b"")
                .with_rule(Rule::field("PID", 3, Strategy::Hash));
        assert_eq!(
            redactor.redact(&mut message),
            Err(RedactError::MissingHmacKey)
        );

        let mut message = sample();
        let redactor = Redactor::safe_harbor(b"", 7);
        assert_eq!(
            redactor.redact(&mut message),
            Err(RedactError::MissingHmacKey)
        );
    }

    #[test]
    fn debug_redacts_hmac_key_bytes() {
        let key = b"super-secret-hmac-key";
        let redactor = Redactor::safe_harbor(key, 30);
        let debug = format!("{redactor:?}");
        // Raw key bytes must never appear (`Vec<u8>` debug prints them).
        assert!(!debug.contains(&format!("{:?}", key.to_vec())));
        assert!(debug.contains("redacted"));
        assert!(debug.contains(&key.len().to_string()));
        // A missing key reports None.
        assert!(format!("{:?}", Redactor::new()).contains("None"));
    }

    /// Build `ID|f1|f2|...` with 1-based field numbers for tests.
    fn seg(id: &str, fields: &[(usize, &str)]) -> String {
        let max = fields.iter().map(|(n, _)| *n).max().unwrap_or(0);
        let mut parts = vec![String::new(); max];
        for (n, val) in fields {
            parts[n - 1] = val.to_string();
        }
        format!("{}|{}", id, parts.join("|"))
    }

    /// Parse MSH plus `segments`, redact with Safe Harbor, return it.
    fn redact_segments(segments: &[String]) -> Message {
        let mut text = String::from("MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|MSG001|P|2.5");
        for s in segments {
            text.push('\r');
            text.push_str(s);
        }
        let mut message = parse_message(&text).unwrap();
        Redactor::safe_harbor(b"site-secret", 30)
            .redact(&mut message)
            .unwrap();
        message
    }

    /// Full `^`-joined value of a field's first repetition (or "").
    fn full_value_of(segment: &Segment, field: usize) -> String {
        segment
            .get_field(field)
            .and_then(|f| f.repetitions.first())
            .and_then(|r| r.full_value())
            .unwrap_or_default()
    }

    /// An XAD field was generalized: street/city gone, ZIP-3 kept.
    fn assert_address_generalized(segment: &Segment, field: usize, zip3: &str) {
        let addr = segment.get_field(field).unwrap().get_repetition(0).unwrap();
        assert_eq!(addr.get_component(0).unwrap().value(), Some(""));
        assert_eq!(addr.get_component(2).unwrap().value(), Some(""));
        assert_eq!(addr.get_component(4).unwrap().value(), Some(zip3));
    }

    #[test]
    fn safe_harbor_redacts_guarantor_insured_kin_identifiers() {
        let key = b"site-secret";
        let message = redact_segments(&[
            seg(
                "GT1",
                &[
                    (12, "123-45-6789"),
                    (16, "Employer^Acme"),
                    (17, "789 Corp Ave^^NYC^NY^10001^USA"),
                    (18, "555-0200"),
                    (29, "EMPID456"),
                    (42, "Mother^Maiden"),
                    (45, "Contact^Cathy"),
                    (46, "555-0300"),
                    (56, "Boston"),
                ],
            ),
            seg(
                "IN1",
                &[
                    (18, "19800101"),
                    (19, "123 Main St^^Springfield^IL^62701^USA"),
                    (49, "INS999"),
                    (52, "Springfield"),
                ],
            ),
            seg(
                "NK1",
                &[
                    (16, "19800101"),
                    (26, "Maiden^Mary"),
                    (30, "Contact^Carl"),
                    (31, "555-0100"),
                    (32, "456 Oak St^^Chicago^IL^60601^USA"),
                    (33, "NKID123"),
                    (37, "987-65-4321"),
                    (38, "Chicago"),
                ],
            ),
            seg("PV1", &[(5, "PREADMIT1")]),
            seg("PD1", &[(10, "DUP123")]),
        ]);

        // Guarantor SSN, employer identity, contacts, birth place gone.
        let gt1 = message.get_segment_by_id("GT1").unwrap();
        for field in [12, 16, 18, 42, 45, 46, 56] {
            assert!(gt1.get_field_value(field).is_none(), "GT1-{field}");
        }
        assert_address_generalized(gt1, 17, "100");
        assert_eq!(full_value_of(gt1, 29), hash_of(key, "EMPID456"));

        // Insured DOB shifted, address generalized, ID hashed.
        let in1 = message.get_segment_by_id("IN1").unwrap();
        assert_eq!(in1.get_field_value(18), Some("19800131"));
        assert_address_generalized(in1, 19, "627");
        assert_eq!(full_value_of(in1, 49), hash_of(key, "INS999"));
        assert!(in1.get_field_value(52).is_none());

        // Next of kin DOB shifted, names/phones/SSN gone, IDs hashed.
        let nk1 = message.get_segment_by_id("NK1").unwrap();
        assert_eq!(nk1.get_field_value(16), Some("19800131"));
        for field in [26, 30, 31, 37, 38] {
            assert!(nk1.get_field_value(field).is_none(), "NK1-{field}");
        }
        assert_address_generalized(nk1, 32, "606");
        assert_eq!(full_value_of(nk1, 33), hash_of(key, "NKID123"));

        // Preadmit and duplicate-patient IDs hashed, not kept.
        let pv1 = message.get_segment_by_id("PV1").unwrap();
        assert_eq!(full_value_of(pv1, 5), hash_of(key, "PREADMIT1"));
        let pd1 = message.get_segment_by_id("PD1").unwrap();
        assert_eq!(full_value_of(pd1, 10), hash_of(key, "DUP123"));
    }

    #[test]
    fn safe_harbor_redacts_staff_provider_callback_identity() {
        let message = redact_segments(&[
            seg(
                "ORC",
                &[
                    (10, "EN1^Enter^Eve"),
                    (11, "VF1^Verify^Victor"),
                    (14, "555-0400"),
                    (19, "AC1^Action^Amy"),
                    (22, "100 Hosp Rd^^Denver^CO^80201^USA"),
                    (23, "555-0500"),
                    (24, "200 Doc Ln^^Denver^CO^80202^USA"),
                ],
            ),
            seg("EVN", &[(5, "OP123^Operator^Olivia")]),
            seg(
                "OBR",
                &[
                    (10, "COL1^Collector^Cal"),
                    (17, "555-0600"),
                    (28, "CC1^Copy^Cora"),
                ],
            ),
            seg("PV1", &[(52, "OTH1^Other^Owen")]),
            seg("IN1", &[(30, "VER1^Verifier^Vera")]),
            seg("TXA", &[(23, "DIST1^Dist^Dan")]),
        ]);

        let orc = message.get_segment_by_id("ORC").unwrap();
        for field in [10, 11, 14, 19, 23] {
            assert!(orc.get_field_value(field).is_none(), "ORC-{field}");
        }
        assert_address_generalized(orc, 22, "802");
        assert_address_generalized(orc, 24, "802");

        let evn = message.get_segment_by_id("EVN").unwrap();
        assert!(evn.get_field_value(5).is_none());

        let obr = message.get_segment_by_id("OBR").unwrap();
        for field in [10, 17, 28] {
            assert!(obr.get_field_value(field).is_none(), "OBR-{field}");
        }

        let pv1 = message.get_segment_by_id("PV1").unwrap();
        assert!(pv1.get_field_value(52).is_none());
        let in1 = message.get_segment_by_id("IN1").unwrap();
        assert!(in1.get_field_value(30).is_none());
        let txa = message.get_segment_by_id("TXA").unwrap();
        assert!(txa.get_field_value(23).is_none());
    }

    #[test]
    fn safe_harbor_covers_schedule_role_procedure_pharmacy_segments() {
        let key = b"site-secret";
        let message = redact_segments(&[
            seg("IN2", &[(2, "111-22-3333")]),
            seg(
                "SCH",
                &[
                    (12, "PL1^Placer^Pete"),
                    (13, "555-0700"),
                    (14, "300 Sched St^^Austin^TX^78701^USA"),
                    (16, "FL1^Filler^Fiona"),
                    (17, "555-0800"),
                    (18, "400 Fill Ave^^Austin^TX^78702^USA"),
                ],
            ),
            seg(
                "ROL",
                &[
                    (4, "RP1^Role^Rita"),
                    (11, "500 Role Rd^^Seattle^WA^98101^USA"),
                    (12, "555-0900"),
                ],
            ),
            seg(
                "PR1",
                &[
                    (8, "AN1^Anes^Anna"),
                    (11, "SU1^Surg^Sam"),
                    (12, "PP1^Proc^Paul"),
                ],
            ),
            seg(
                "CTD",
                &[
                    (2, "Contact^Chris"),
                    (3, "600 Contact Ct^^Miami^FL^33101^USA"),
                    (5, "555-1000"),
                ],
            ),
            seg(
                "RXD",
                &[
                    (10, "DP1^Dispense^Dave"),
                    (31, "700 Pharm Pl^^Boston^MA^02101^USA"),
                ],
            ),
            seg(
                "RXE",
                &[
                    (13, "DEA123^Doc^Dan"),
                    (14, "PH1^Pharm^Phil"),
                    (41, "800 Disp Dr^^Boston^MA^02102^USA"),
                    (43, "900 Deliver Dr^^Boston^MA^02103^USA"),
                ],
            ),
            seg(
                "FT1",
                &[
                    (20, "PB1^Perf^Pat"),
                    (21, "OB1^Order^Omar"),
                    (24, "EB1^Enter^Ed"),
                ],
            ),
            seg("BLG", &[(3, "ACCT777")]),
            seg(
                "ACC",
                &[
                    (7, "EN2^Enter^Erin"),
                    (11, "1000 Acc Ave^^Dallas^TX^75201^USA"),
                ],
            ),
        ]);

        let in2 = message.get_segment_by_id("IN2").unwrap();
        assert!(in2.get_field_value(2).is_none());

        let sch = message.get_segment_by_id("SCH").unwrap();
        for field in [12, 13, 16, 17] {
            assert!(sch.get_field_value(field).is_none(), "SCH-{field}");
        }
        assert_address_generalized(sch, 14, "787");
        assert_address_generalized(sch, 18, "787");

        let rol = message.get_segment_by_id("ROL").unwrap();
        assert!(rol.get_field_value(4).is_none());
        assert!(rol.get_field_value(12).is_none());
        assert_address_generalized(rol, 11, "981");

        let pr1 = message.get_segment_by_id("PR1").unwrap();
        for field in [8, 11, 12] {
            assert!(pr1.get_field_value(field).is_none(), "PR1-{field}");
        }

        let ctd = message.get_segment_by_id("CTD").unwrap();
        assert!(ctd.get_field_value(2).is_none());
        assert!(ctd.get_field_value(5).is_none());
        assert_address_generalized(ctd, 3, "331");

        let rxd = message.get_segment_by_id("RXD").unwrap();
        assert!(rxd.get_field_value(10).is_none());
        assert_address_generalized(rxd, 31, "021");

        let rxe = message.get_segment_by_id("RXE").unwrap();
        assert!(rxe.get_field_value(13).is_none());
        assert!(rxe.get_field_value(14).is_none());
        assert_address_generalized(rxe, 41, "021");
        assert_address_generalized(rxe, 43, "021");

        let ft1 = message.get_segment_by_id("FT1").unwrap();
        for field in [20, 21, 24] {
            assert!(ft1.get_field_value(field).is_none(), "FT1-{field}");
        }

        let blg = message.get_segment_by_id("BLG").unwrap();
        assert_eq!(full_value_of(blg, 3), hash_of(key, "ACCT777"));

        let acc = message.get_segment_by_id("ACC").unwrap();
        assert!(acc.get_field_value(7).is_none());
        assert_address_generalized(acc, 11, "752");
    }

    #[test]
    fn safe_harbor_redacts_oru_provider_fields_and_county() {
        let message = redact_segments(&[
            seg("PID", &[(5, "Smith^John"), (12, "06001")]),
            seg(
                "OBR",
                &[
                    (32, "PRI^Principal^Pete"),
                    (33, "AST^Assistant^Amy"),
                    (34, "TEC^Tech^Tina"),
                    (35, "TRN^Trans^Trent"),
                ],
            ),
            seg("OBX", &[(25, "DIR^Director^Dan")]),
        ]);

        // OBR-32..35 (NDL interpreters/tech/transcriptionist) and OBX-25
        // (XCN medical director) hold person names; PID-12 is a county
        // code, a Safe Harbor geographic identifier. None may survive.
        let pid = message.get_segment_by_id("PID").unwrap();
        assert!(pid.get_field_value(5).is_none());
        assert!(pid.get_field_value(12).is_none(), "PID-12 county survives");
        let obr = message.get_segment_by_id("OBR").unwrap();
        for field in [32, 33, 34, 35] {
            assert!(obr.get_field_value(field).is_none(), "OBR-{field} survives");
        }
        let obx = message.get_segment_by_id("OBX").unwrap();
        assert!(obx.get_field_value(25).is_none(), "OBX-25 survives");
    }

    #[test]
    fn safe_harbor_redacts_audited_person_name_fields() {
        // XCN/XPN audit over the 34 covered segments: every person-name
        // field the profile missed (field numbers cross-checked against
        // crates/vyna-hl7-validator/schemas).
        let message = redact_segments(&[
            seg("IN1", &[(6, "Contact^Casey")]),
            seg(
                "IN2",
                &[
                    (3, "EMP1^Employer^Ed"),
                    (7, "Case^Medicaid"),
                    (9, "Sponsor^Sam"),
                    (22, "Approver^Amy"),
                    (40, "Maiden^Mary"),
                    (49, "Contact^Carl"),
                    (52, "Kin^Kate"),
                ],
            ),
            seg("SCH", &[(20, "ENT1^Entered^Eve")]),
        ]);

        let in1 = message.get_segment_by_id("IN1").unwrap();
        assert!(in1.get_field_value(6).is_none());
        let in2 = message.get_segment_by_id("IN2").unwrap();
        for field in [3, 7, 9, 22, 40, 49, 52] {
            assert!(in2.get_field_value(field).is_none(), "IN2-{field} survives");
        }
        let sch = message.get_segment_by_id("SCH").unwrap();
        assert!(sch.get_field_value(20).is_none());
    }

    #[test]
    fn safe_harbor_covers_death_and_certification_segments() {
        let key = b"site-secret";
        let message = redact_segments(&[
            seg(
                "PDA",
                &[
                    (4, "20240101120000"),
                    (5, "DC1^Cert^Cid"),
                    (7, "20240101^20240102"),
                    (8, "AU1^Autopsy^Al"),
                ],
            ),
            seg(
                "IN3",
                &[
                    (2, "CERT999"),
                    (3, "CB1^Cert^Cory"),
                    (6, "20240101"),
                    (7, "20240102"),
                    (8, "OP2^Oper^Odin"),
                    (9, "20240103"),
                    (10, "20240203"),
                    (13, "20240104"),
                    (14, "RV1^Review^Ruth"),
                    (15, "Cert Contact"),
                    (16, "555-1100"),
                    (19, "555-1200"),
                    (21, "Case Manager"),
                    (22, "20240105"),
                    (25, "SO1^Second^Sid"),
                ],
            ),
        ]);

        // Death certificate date shifted; certifier, autopsy range and
        // performer removed (the DR range is removed, not half-shifted).
        let pda = message.get_segment_by_id("PDA").unwrap();
        assert_eq!(pda.get_field_value(4), Some("20240131120000"));
        for field in [5, 7, 8] {
            assert!(pda.get_field_value(field).is_none(), "PDA-{field}");
        }

        // Certification number hashed, dates shifted, contacts gone.
        let in3 = message.get_segment_by_id("IN3").unwrap();
        assert_eq!(full_value_of(in3, 2), hash_of(key, "CERT999"));
        assert_eq!(in3.get_field_value(6), Some("20240131"));
        assert_eq!(in3.get_field_value(7), Some("20240201"));
        assert_eq!(in3.get_field_value(9), Some("20240202"));
        assert_eq!(in3.get_field_value(10), Some("20240304"));
        assert_eq!(in3.get_field_value(13), Some("20240203"));
        assert_eq!(in3.get_field_value(22), Some("20240204"));
        for field in [3, 8, 14, 15, 16, 19, 21, 25] {
            assert!(in3.get_field_value(field).is_none(), "IN3-{field}");
        }
    }

    #[test]
    fn safe_harbor_removes_insurance_and_schedule_telecom() {
        // XTN audit over the covered segments: every phone field the
        // profile missed (numbers and datatypes cross-checked against
        // crates/vyna-hl7-validator/schemas).
        let message = redact_segments(&[
            seg(
                "IN2",
                &[
                    (50, "555-1111"),
                    (53, "555-2222"),
                    (58, "555-3333"),
                    (63, "555-4444"),
                    (64, "555-5555"),
                ],
            ),
            seg("SCH", &[(21, "555-6666")]),
        ]);

        let in2 = message.get_segment_by_id("IN2").unwrap();
        for field in [50, 53, 58, 63, 64] {
            assert!(in2.get_field_value(field).is_none(), "IN2-{field} survives");
        }
        let sch = message.get_segment_by_id("SCH").unwrap();
        assert!(sch.get_field_value(21).is_none(), "SCH-21 survives");
    }

    #[test]
    fn safe_harbor_hashes_insurance_order_and_document_ids() {
        // EI/CX/ST identifier audit: every ID field the profile missed
        // (numbers, names and datatypes cross-checked against
        // crates/vyna-hl7-validator/schemas). TXA-22 (PPN person stamp)
        // and ACC-9 (person name) hold person identity directly, so they
        // are removed instead of hashed.
        let key = b"site-secret";
        let message = redact_segments(&[
            seg(
                "IN2",
                &[
                    (1, "EMP1"),
                    (6, "MEDICARE123"),
                    (8, "MCD1"),
                    (10, "MIL1"),
                    (25, "PAY1"),
                    (26, "SUB1"),
                    (61, "MEM1"),
                ],
            ),
            seg("IN1", &[(10, "GRP1"), (28, "PAC1")]),
            seg("NK1", &[(12, "NKEMP1")]),
            seg(
                "FT1",
                &[(2, "TXN1"), (23, "FILL1"), (30, "PAYREF1"), (31, "5")],
            ),
            seg("RXD", &[(7, "RXN1")]),
            seg("RXE", &[(15, "RXN2")]),
            seg("CTD", &[(7, "CID1")]),
            seg(
                "SCH",
                &[
                    (1, "PA1"),
                    (2, "FA1"),
                    (4, "PG1"),
                    (23, "PPA1"),
                    (24, "PFA1"),
                    (26, "PON1"),
                    (27, "FON1"),
                ],
            ),
            seg("ORC", &[(4, "OG1")]),
            seg("DG1", &[(20, "DIAG1")]),
            seg("ROL", &[(1, "ROLE1")]),
            seg("PR1", &[(19, "PROC1")]),
            seg("SPM", &[(3, "SPP1")]),
            seg("TXA", &[(22, "DOC1^Auth^Amy")]),
            seg("ACC", &[(9, "John Doe")]),
        ]);

        for (segment, field, original) in [
            ("IN2", 1, "EMP1"),
            ("IN2", 6, "MEDICARE123"),
            ("IN2", 8, "MCD1"),
            ("IN2", 10, "MIL1"),
            ("IN2", 25, "PAY1"),
            ("IN2", 26, "SUB1"),
            ("IN2", 61, "MEM1"),
            ("IN1", 10, "GRP1"),
            ("IN1", 28, "PAC1"),
            ("NK1", 12, "NKEMP1"),
            ("FT1", 2, "TXN1"),
            ("FT1", 23, "FILL1"),
            ("FT1", 30, "PAYREF1"),
            ("FT1", 31, "5"),
            ("RXD", 7, "RXN1"),
            ("RXE", 15, "RXN2"),
            ("CTD", 7, "CID1"),
            ("SCH", 1, "PA1"),
            ("SCH", 2, "FA1"),
            ("SCH", 4, "PG1"),
            ("SCH", 23, "PPA1"),
            ("SCH", 24, "PFA1"),
            ("SCH", 26, "PON1"),
            ("SCH", 27, "FON1"),
            ("ORC", 4, "OG1"),
            ("DG1", 20, "DIAG1"),
            ("ROL", 1, "ROLE1"),
            ("PR1", 19, "PROC1"),
            ("SPM", 3, "SPP1"),
        ] {
            let segment_id = segment;
            let segment = message.get_segment_by_id(segment).unwrap();
            assert_eq!(
                full_value_of(segment, field),
                hash_of(key, original),
                "{segment_id}-{field} survives"
            );
        }
        let txa = message.get_segment_by_id("TXA").unwrap();
        assert!(txa.get_field_value(22).is_none(), "TXA-22 survives");
        let acc = message.get_segment_by_id("ACC").unwrap();
        assert!(acc.get_field_value(9).is_none(), "ACC-9 survives");
    }

    #[test]
    fn safe_harbor_shifts_remaining_clinical_and_coverage_dates() {
        // TS/DT/DTM audit: every date field the profile missed (numbers
        // and datatypes cross-checked against
        // crates/vyna-hl7-validator/schemas). IN1-44 is an employer
        // address (generalized like the other XAD fields); FT1-4 is a DR
        // range (removed like PDA-7, not half-shifted).
        let message = redact_segments(&[
            seg(
                "OBR",
                &[(6, "20240101"), (14, "20240101"), (36, "20240101")],
            ),
            seg("PR1", &[(5, "20240101")]),
            seg("RXD", &[(3, "20240101"), (19, "20240101")]),
            seg("RXE", &[(18, "20240101"), (32, "20240101")]),
            seg("EVN", &[(3, "20240101")]),
            seg("ACC", &[(1, "20240101")]),
            seg("DG1", &[(19, "20240101")]),
            seg(
                "GT1",
                &[
                    (13, "20240101"),
                    (14, "20240101"),
                    (24, "20240101"),
                    (31, "20240101"),
                    (32, "20240101"),
                ],
            ),
            seg(
                "IN1",
                &[
                    (24, "20240101"),
                    (26, "20240101"),
                    (29, "20240101"),
                    (51, "20240101"),
                    (44, "123 Main St^^Springfield^IL^62701^USA"),
                ],
            ),
            seg(
                "IN2",
                &[
                    (17, "20240101"),
                    (44, "20240101"),
                    (45, "20240101"),
                    (55, "20240101"),
                    (56, "20240101"),
                ],
            ),
            seg("NK1", &[(8, "20240101"), (9, "20240101")]),
            seg("ORC", &[(27, "20240101")]),
            seg(
                "PD1",
                &[(13, "20240101"), (17, "20240101"), (18, "20240101")],
            ),
            seg(
                "PV1",
                &[(25, "20240101"), (30, "20240101"), (35, "20240101")],
            ),
            seg("ROL", &[(5, "20240101"), (6, "20240101")]),
            seg("FT1", &[(5, "20240101"), (4, "20240101^20240102")]),
            seg("SPM", &[(19, "20240101")]),
            seg("OBX", &[(12, "20240101")]),
        ]);

        for (segment, field) in [
            ("OBR", 6),
            ("OBR", 14),
            ("OBR", 36),
            ("PR1", 5),
            ("RXD", 3),
            ("RXE", 18),
            ("RXE", 32),
            ("EVN", 3),
            ("ACC", 1),
            ("DG1", 19),
            ("GT1", 13),
            ("GT1", 14),
            ("GT1", 24),
            ("GT1", 31),
            ("GT1", 32),
            ("IN1", 24),
            ("IN1", 26),
            ("IN1", 29),
            ("IN1", 51),
            ("IN2", 17),
            ("IN2", 44),
            ("IN2", 45),
            ("IN2", 55),
            ("IN2", 56),
            ("NK1", 8),
            ("NK1", 9),
            ("ORC", 27),
            ("PD1", 13),
            ("PD1", 17),
            ("PD1", 18),
            ("PV1", 25),
            ("PV1", 30),
            ("PV1", 35),
            ("ROL", 5),
            ("ROL", 6),
            ("FT1", 5),
            ("SPM", 19),
            ("RXD", 19),
            ("OBX", 12),
        ] {
            let segment_id = segment;
            let segment = message.get_segment_by_id(segment).unwrap();
            assert_eq!(
                segment.get_field_value(field),
                Some("20240131"),
                "{segment_id}-{field} not shifted"
            );
        }
        let in1 = message.get_segment_by_id("IN1").unwrap();
        assert_address_generalized(in1, 44, "627");
        let ft1 = message.get_segment_by_id("FT1").unwrap();
        assert!(ft1.get_field_value(4).is_none(), "FT1-4 DR survives");
    }

    #[test]
    fn safe_harbor_keeps_pd1_military_branch() {
        // PD1-19 is Military Branch (IS/CWE), not a date: a DateShift
        // rule silently removed it while the real PD1 dates leaked.
        let message = redact_segments(&[seg("PD1", &[(19, "ARMY")])]);
        let pd1 = message.get_segment_by_id("PD1").unwrap();
        assert_eq!(pd1.get_field_value(19), Some("ARMY"));
    }

    #[test]
    fn safe_harbor_removes_specimen_collection_range() {
        // SPM-17 is a DR range: shifting its joined text corrupted the
        // wire form ('^' escaped, range end untouched), so like PDA-7 it
        // is removed instead of shifted.
        let message = redact_segments(&[seg("SPM", &[(17, "20240101^20240102")])]);
        let spm = message.get_segment_by_id("SPM").unwrap();
        assert!(spm.get_field_value(17).is_none(), "SPM-17 DR survives");
        assert!(
            !message.encode().contains("\\S\\"),
            "escape-corrupted range: {}",
            message.encode()
        );
    }

    #[test]
    fn safe_harbor_hashes_orc_parent_numbers() {
        // ORC-8 Parent (EIP) carries the same placer/filler numbers
        // hashed in ORC-2/ORC-3; leaving it verbatim leaks them.
        let key = b"site-secret";
        let message = redact_segments(&[seg("ORC", &[(8, "PLACER99^FILLER99")])]);
        let orc = message.get_segment_by_id("ORC").unwrap();
        assert_eq!(
            orc.get_field_value(8),
            Some(hash_of(key, "PLACER99^FILLER99").as_str()),
            "ORC-8 parent numbers survive verbatim"
        );
    }

    #[test]
    fn safe_harbor_hashes_obr_parent_numbers() {
        // OBR-26 Parent Result (PRL) and OBR-29 Parent (EIP) carry
        // parent placer/filler numbers like OBR-2/OBR-3.
        let key = b"site-secret";
        let message = redact_segments(&[seg(
            "OBR",
            &[(26, "WBC^PLACER88^FILLER88"), (29, "PLACER77^FILLER77")],
        )]);
        let obr = message.get_segment_by_id("OBR").unwrap();
        assert_eq!(
            obr.get_field_value(26),
            Some(hash_of(key, "WBC^PLACER88^FILLER88").as_str()),
            "OBR-26 parent result survives verbatim"
        );
        assert_eq!(
            obr.get_field_value(29),
            Some(hash_of(key, "PLACER77^FILLER77").as_str()),
            "OBR-29 parent numbers survive verbatim"
        );
    }

    #[test]
    fn safe_harbor_hashes_problem_instance_ids() {
        // PRB-4 Problem Instance ID (EI) and PRB-5 Episode of Care ID
        // (EI) are identifiers like DG1-20/PR1-19.
        let key = b"site-secret";
        let message = redact_segments(&[seg("PRB", &[(4, "PROBINST1"), (5, "EPISODE1")])]);
        let prb = message.get_segment_by_id("PRB").unwrap();
        assert_eq!(
            prb.get_field_value(4),
            Some(hash_of(key, "PROBINST1").as_str()),
            "PRB-4 instance id survives verbatim"
        );
        assert_eq!(
            prb.get_field_value(5),
            Some(hash_of(key, "EPISODE1").as_str()),
            "PRB-5 episode id survives verbatim"
        );
    }

    #[test]
    fn safe_harbor_redacts_staff_license_and_review_dates() {
        // STF-22 Driver's License Number (DLN) is an explicit Safe
        // Harbor identifier; STF-24/25/26/31/35 are dates (DT/TS) and
        // STF-34 an Institution Relationship Period (DR) removed like
        // PDA-7/SPM-17/FT1-4. (STF is absent from our validator
        // schemas; numbering follows the HL7 v2.5 STF segment.)
        let key = b"site-secret";
        let message = redact_segments(&[seg(
            "STF",
            &[
                (22, "DL1234567"),
                (24, "20250101"),
                (25, "20240101"),
                (26, "20260101"),
                (31, "20240315120000"),
                (34, "20240101^20241231"),
                (35, "20250201"),
            ],
        )]);
        let stf = message.get_segment_by_id("STF").unwrap();
        assert_eq!(
            stf.get_field_value(22),
            Some(hash_of(key, "DL1234567").as_str()),
            "STF-22 license survives verbatim"
        );
        for (field, expected) in [
            (24, "20250131"),
            (25, "20240131"),
            (26, "20260131"),
            (31, "20240414120000"),
            (35, "20250303"),
        ] {
            assert_eq!(
                stf.get_field_value(field),
                Some(expected),
                "STF-{field} not shifted"
            );
        }
        assert!(stf.get_field_value(34).is_none(), "STF-34 DR survives");
    }

    #[test]
    fn safe_harbor_keeps_prb_awareness_and_sensitivity() {
        // PRB-24 Family Awareness (ST) and PRB-25 Security/Sensitivity
        // (CE) are not dates: DateShift deleted them (unparseable yields
        // None) though Safe Harbor does not cover them.
        let message = redact_segments(&[seg("PRB", &[(24, "Y"), (25, "R")])]);
        let prb = message.get_segment_by_id("PRB").unwrap();
        assert_eq!(prb.get_field_value(24), Some("Y"));
        assert_eq!(prb.get_field_value(25), Some("R"));
    }

    #[test]
    fn safe_harbor_keeps_in1_plan_type_and_shifts_auth_date() {
        // IN1-14 Authorization Information (AUI) is not a date as a
        // whole: only its component 2 (DT) shifts, the number and
        // source stay. IN1-15 Plan Type (IS) is kept.
        let message =
            redact_segments(&[seg("IN1", &[(14, "AUTH456^20240101^SRC"), (15, "PTYPE")])]);
        let in1 = message.get_segment_by_id("IN1").unwrap();
        assert_eq!(
            full_value_of(in1, 14),
            "AUTH456^20240131^SRC",
            "IN1-14-2 auth date not shifted"
        );
        assert_eq!(in1.get_field_value(15), Some("PTYPE"));
    }

    #[test]
    fn safe_harbor_shifts_timing_quantity_datetimes() {
        // ORC-7, SCH-11, RXE-1, OBR-27 are TQ: components 4/5 are TS
        // Start/End Date-Time and must shift while the rest is kept.
        let timing = "1^once^^20240101120000^20240101130000";
        let shifted = "1^once^^20240131120000^20240131130000";
        let message = redact_segments(&[
            seg("ORC", &[(7, timing)]),
            seg("SCH", &[(11, timing)]),
            seg("RXE", &[(1, timing)]),
            seg("OBR", &[(27, timing)]),
        ]);
        for (segment, field) in [("ORC", 7), ("SCH", 11), ("RXE", 1), ("OBR", 27)] {
            let seg = message.get_segment_by_id(segment).unwrap();
            assert_eq!(
                full_value_of(seg, field),
                shifted,
                "{segment}-{field} TQ datetimes not shifted"
            );
        }
    }

    #[test]
    fn safe_harbor_prd_audit_matches_v2plus_reference() {
        // PRD audit: field names, numbers and datatypes verified against
        // the HL7 v2+ PRD segment reference (PRD is absent from our
        // validator schemas, so nothing else cross-checks it). Every
        // person-identity field has its profile-consistent rule: names
        // (2 XPN), telecom (5, 13 XTN) and org name (10 XON) removed;
        // addresses (3, 11 XAD) generalized; identifiers (7 PLN) hashed;
        // role dates (8, 9 DTM) shifted. Coded fields (1, 6, 14 CWE) and
        // facility locations (4, 12 PL, like the uncovered PV1-3) stay.
        let key = b"site-secret";
        let message = redact_segments(&[seg(
            "PRD",
            &[
                (1, "PP"),
                (2, "House^Greg"),
                (3, "221 B St^^Princeton^NJ^08540^USA"),
                (4, "ER^R1^B2^FAC"),
                (5, "555-0200"),
                (6, "PH"),
                (7, "NPI1234567"),
                (8, "20200101"),
                (9, "20250101"),
                (10, "Princeton Hospital"),
                (11, "1 Hospital Dr^^Princeton^NJ^08540^USA"),
                (12, "MAIN^WING^FAC"),
                (13, "555-0300"),
                (14, "EM"),
            ],
        )]);
        let prd = message.get_segment_by_id("PRD").unwrap();
        for field in [2, 5, 10, 13] {
            assert!(prd.get_field_value(field).is_none(), "PRD-{field} survives");
        }
        assert_address_generalized(prd, 3, "085");
        assert_address_generalized(prd, 11, "085");
        assert_eq!(
            full_value_of(prd, 7),
            hash_of(key, "NPI1234567"),
            "PRD-7 identifiers survive verbatim"
        );
        assert_eq!(prd.get_field_value(8), Some("20200131"));
        assert_eq!(prd.get_field_value(9), Some("20250131"));
        for (field, expected) in [(1, "PP"), (6, "PH"), (14, "EM")] {
            assert_eq!(
                prd.get_field_value(field),
                Some(expected),
                "PRD-{field} must be kept"
            );
        }
        assert_eq!(full_value_of(prd, 4), "ER^R1^B2^FAC");
        assert_eq!(full_value_of(prd, 12), "MAIN^WING^FAC");
    }

    #[test]
    fn whole_field_hash_distinguishes_subcomponent_content() {
        // "AAA&X" and "AAA&Y" share a first subcomponent but are
        // distinct identities: wire-faithful hashing must not merge
        // them (the old first-subcomponent join hashed both as "AAA").
        let digest = |value: &str| {
            let mut message = parse_message(&format!(
                "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||{value}\r"
            ))
            .unwrap();
            Redactor::new()
                .with_hmac_key(b"k")
                .with_rule(Rule::field("PID", 3, Strategy::Hash))
                .redact(&mut message)
                .unwrap();
            message
                .get_segment_by_id("PID")
                .unwrap()
                .get_field(3)
                .unwrap()
                .repetitions[0]
                .full_value()
                .unwrap()
        };
        assert_ne!(digest("AAA&X"), digest("AAA&Y"));
        // Deterministic: same input, same digest.
        assert_eq!(digest("AAA&X"), digest("AAA&X"));
        assert_eq!(digest("AAA&X"), hash_of(b"k", "AAA&X"));
    }

    #[test]
    fn component_hash_distinguishes_structure_from_literal_text() {
        // Two subcomponents ("A","B") vs one literal "A&B": the naive
        // "&"-join collides ("A&B" both); wire-faithful encoding
        // ("A&B" vs "A\T\B") must not.
        use vyna_hl7_core::{Component, SubComponent};
        let digest = |component: Component| {
            let mut message = Message::new();
            let mut pid = Segment::new("PID");
            let mut field = Field::new();
            let mut repetition = Repetition::new();
            repetition.add_component(component);
            field.add_repetition(repetition);
            pid.set_field(5, field).unwrap();
            message.add_segment(pid);
            Redactor::new()
                .with_hmac_key(b"k")
                .with_rule(Rule::component("PID", 5, 1, Strategy::Hash))
                .redact(&mut message)
                .unwrap();
            message
                .get_segment_by_id("PID")
                .unwrap()
                .get_field(5)
                .unwrap()
                .repetitions[0]
                .components[0]
                .value()
                .unwrap()
                .to_string()
        };
        let mut structured = Component::new();
        structured.add_subcomponent(SubComponent::new("A"));
        structured.add_subcomponent(SubComponent::new("B"));
        let literal = Component::from_value("A&B");
        assert_ne!(digest(structured), digest(literal));
    }

    #[test]
    fn safe_harbor_removes_in1_insurance_phone() {
        // IN1-7 Insurance Co Phone Number (XTN) must go like its
        // sibling telecom fields (IN1-16, IN2-58, ORC-23).
        let message = redact_segments(&[seg(
            "IN1",
            &[(7, "555-9999"), (16, "Insured^Ivy"), (30, "VER1^Vera")],
        )]);
        let in1 = message.get_segment_by_id("IN1").unwrap();
        assert!(in1.get_field_value(7).is_none(), "IN1-7 phone survives");
        assert!(in1.get_field_value(16).is_none());
        assert!(in1.get_field_value(30).is_none());
    }
}
