//! Structural diff of two HL7 messages.
//!
//! [`diff_messages`] walks both messages positionally (segments aligned by
//! occurrence within each segment id, then fields, repetitions, components,
//! subcomponents) and reports leaf-level differences with terser-style paths:
//!
//! ```text
//! PID-5-2              field 5, component 2
//! OBX(2)-5             second OBX, field 5
//! PID-11[2]-1          field 11, repetition 2, component 1
//! ```
//!
//! Repetition indexes (`[n]`, 1-based) appear only when either side repeats
//! the field. Segment occurrence (`(n)`, 1-based) appears only when either
//! side repeats the segment. Single-component values report at the field
//! path (`PID-1`, not `PID-1-1`). Empty and missing values compare equal,
//! so trailing empty fields never produce noise.
//!
//! The diff is positional: reordered segments or repetitions show up as
//! changed values, not moves.

use crate::delimiters::Delimiters;
use crate::field::{Component, Field, Repetition};
use crate::message::Message;
use crate::segment::Segment;

/// How one path differs between two messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    /// Present only in the new message.
    Added,
    /// Present only in the old message.
    Removed,
    /// Present in both with different values.
    Changed,
}

/// One leaf-level difference between two messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// Terser-style path (`PID-5-2`, `OBX(2)-5`).
    pub path: String,
    /// How it differs.
    pub kind: DiffKind,
    /// Old value (`None` for [`DiffKind::Added`]).
    pub old: Option<String>,
    /// New value (`None` for [`DiffKind::Removed`]).
    pub new: Option<String>,
}

/// Diff two messages. Returns an empty vec when they are equal.
pub fn diff_messages(old: &Message, new: &Message) -> Vec<Difference> {
    diff_messages_ignoring(old, new, &[])
}

/// Diff two messages, skipping `ignore` paths.
///
/// An ignore entry suppresses its exact path and everything beneath it, so
/// ignoring `PID-5` also ignores `PID-5-1`. Typical volatile paths: `MSH-7`
/// (timestamp) and `MSH-10` (control id).
pub fn diff_messages_ignoring(old: &Message, new: &Message, ignore: &[String]) -> Vec<Difference> {
    if old == new {
        return Vec::new();
    }
    let mut diffs = Vec::new();
    let mut ids: Vec<&str> = Vec::new();
    for message in [old, new] {
        for segment in &message.segments {
            if !ids.contains(&segment.id.as_str()) {
                ids.push(segment.id.as_str());
            }
        }
    }
    for id in ids {
        let old_segs: Vec<&Segment> = old.segments.iter().filter(|s| s.id == id).collect();
        let new_segs: Vec<&Segment> = new.segments.iter().filter(|s| s.id == id).collect();
        let occurrences = old_segs.len().max(new_segs.len());
        for i in 0..occurrences {
            let base = if occurrences > 1 {
                format!("{id}({})", i + 1)
            } else {
                id.to_string()
            };
            match (old_segs.get(i), new_segs.get(i)) {
                (Some(o), Some(n)) => diff_segment(o, n, &base, &mut diffs, old, new),
                (Some(o), None) => diffs.push(Difference {
                    path: base,
                    kind: DiffKind::Removed,
                    old: Some(o.encode(&old.delimiters)),
                    new: None,
                }),
                (None, Some(n)) => diffs.push(Difference {
                    path: base,
                    kind: DiffKind::Added,
                    old: None,
                    new: Some(n.encode(&new.delimiters)),
                }),
                (None, None) => unreachable!("occurrence index out of range"),
            }
        }
    }
    if ignore.is_empty() {
        diffs
    } else {
        diffs
            .into_iter()
            .filter(|d| !is_ignored(&d.path, ignore))
            .collect()
    }
}

fn is_ignored(path: &str, ignore: &[String]) -> bool {
    ignore.iter().any(|prefix| {
        path.len() >= prefix.len()
            && path.starts_with(prefix.as_str())
            && path
                .as_bytes()
                .get(prefix.len())
                .is_none_or(|b| matches!(b, b'-' | b'(' | b'['))
    })
}

fn diff_segment(
    old: &Segment,
    new: &Segment,
    base: &str,
    diffs: &mut Vec<Difference>,
    old_msg: &Message,
    new_msg: &Message,
) {
    let fields = old.fields.len().max(new.fields.len());
    for i in 0..fields {
        let path = format!("{base}-{}", i + 1);
        match (old.fields.get(i), new.fields.get(i)) {
            (Some(o), Some(n)) => diff_field(o, n, &path, diffs, old_msg, new_msg),
            (Some(o), None) if !o.is_empty() => diffs.push(Difference {
                path,
                kind: DiffKind::Removed,
                old: Some(o.encode(&old_msg.delimiters)),
                new: None,
            }),
            (None, Some(n)) if !n.is_empty() => diffs.push(Difference {
                path,
                kind: DiffKind::Added,
                old: None,
                new: Some(n.encode(&new_msg.delimiters)),
            }),
            _ => {}
        }
    }
}

fn diff_field(
    old: &Field,
    new: &Field,
    base: &str,
    diffs: &mut Vec<Difference>,
    old_msg: &Message,
    new_msg: &Message,
) {
    // Fast path: single repetition on both sides needs no repetition index.
    if old.repetitions.len() <= 1 && new.repetitions.len() <= 1 {
        match (old.repetitions.first(), new.repetitions.first()) {
            (Some(o), Some(n)) => diff_repetition(o, n, base, diffs),
            (Some(o), None) if !o.is_empty() => diffs.push(removed(base, o, &old_msg.delimiters)),
            (None, Some(n)) if !n.is_empty() => diffs.push(added(base, n, &new_msg.delimiters)),
            _ => {}
        }
        return;
    }
    let reps = old.repetitions.len().max(new.repetitions.len());
    for i in 0..reps {
        let path = format!("{base}[{}]", i + 1);
        match (old.repetitions.get(i), new.repetitions.get(i)) {
            (Some(o), Some(n)) => diff_repetition(o, n, &path, diffs),
            (Some(o), None) if !o.is_empty() => {
                diffs.push(removed(&path, o, &old_msg.delimiters));
            }
            (None, Some(n)) if !n.is_empty() => {
                diffs.push(added(&path, n, &new_msg.delimiters));
            }
            _ => {}
        }
    }
}

fn removed(path: &str, repetition: &Repetition, delimiters: &Delimiters) -> Difference {
    Difference {
        path: path.to_string(),
        kind: DiffKind::Removed,
        old: Some(repetition.encode(delimiters)),
        new: None,
    }
}

fn added(path: &str, repetition: &Repetition, delimiters: &Delimiters) -> Difference {
    Difference {
        path: path.to_string(),
        kind: DiffKind::Added,
        old: None,
        new: Some(repetition.encode(delimiters)),
    }
}

fn diff_repetition(old: &Repetition, new: &Repetition, base: &str, diffs: &mut Vec<Difference>) {
    // Scalar fast path: single-component, single-subcomponent values compare
    // at the field (or repetition) path instead of a redundant `-1` suffix.
    if let (Some(o), Some(n)) = (single_component(old), single_component(new))
        && o.subcomponents.len() <= 1
        && n.subcomponents.len() <= 1
    {
        let ov = o.subcomponents.first().map(|s| s.as_str()).unwrap_or("");
        let nv = n.subcomponents.first().map(|s| s.as_str()).unwrap_or("");
        if ov != nv {
            diffs.push(Difference {
                path: base.to_string(),
                kind: DiffKind::Changed,
                old: Some(ov.to_string()),
                new: Some(nv.to_string()),
            });
        }
        return;
    }
    let components = old.components.len().max(new.components.len());
    for i in 0..components {
        let path = format!("{base}-{}", i + 1);
        match (old.components.get(i), new.components.get(i)) {
            (Some(o), Some(n)) => diff_component(o, n, &path, diffs),
            (Some(o), None) if !o.is_empty() => diffs.push(Difference {
                path,
                kind: DiffKind::Removed,
                old: Some(component_text(o)),
                new: None,
            }),
            (None, Some(n)) if !n.is_empty() => diffs.push(Difference {
                path,
                kind: DiffKind::Added,
                old: None,
                new: Some(component_text(n)),
            }),
            _ => {}
        }
    }
}

/// The single component of a repetition, if it has at most one.
fn single_component(repetition: &Repetition) -> Option<&Component> {
    if repetition.components.len() <= 1 {
        repetition.components.first()
    } else {
        None
    }
}

fn diff_component(old: &Component, new: &Component, base: &str, diffs: &mut Vec<Difference>) {
    if old.subcomponents.len() <= 1 && new.subcomponents.len() <= 1 {
        let o = old.subcomponents.first().map(|s| s.as_str()).unwrap_or("");
        let n = new.subcomponents.first().map(|s| s.as_str()).unwrap_or("");
        if o != n {
            diffs.push(Difference {
                path: base.to_string(),
                kind: DiffKind::Changed,
                old: Some(o.to_string()),
                new: Some(n.to_string()),
            });
        }
        return;
    }
    let subs = old.subcomponents.len().max(new.subcomponents.len());
    for i in 0..subs {
        let path = format!("{base}-{}", i + 1);
        match (old.subcomponents.get(i), new.subcomponents.get(i)) {
            (Some(o), Some(n)) if o.as_str() != n.as_str() => diffs.push(Difference {
                path,
                kind: DiffKind::Changed,
                old: Some(o.as_str().to_string()),
                new: Some(n.as_str().to_string()),
            }),
            (Some(o), None) if !o.as_str().is_empty() => diffs.push(Difference {
                path,
                kind: DiffKind::Removed,
                old: Some(o.as_str().to_string()),
                new: None,
            }),
            (None, Some(n)) if !n.as_str().is_empty() => diffs.push(Difference {
                path,
                kind: DiffKind::Added,
                old: None,
                new: Some(n.as_str().to_string()),
            }),
            _ => {}
        }
    }
}

/// Display text for a component: decoded subcomponents joined by `&`.
fn component_text(component: &Component) -> String {
    component
        .subcomponents
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::Field;

    fn pid(id: &str, family: &str, given: &str) -> Segment {
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        pid.add_field(Field::from_value(""));
        let mut id_field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(id));
        id_field.add_repetition(rep);
        pid.add_field(id_field);
        pid.add_field(Field::from_value(""));
        let mut name = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(family));
        rep.add_component(Component::from_value(given));
        name.add_repetition(rep);
        pid.add_field(name);
        pid
    }

    fn message_with(segments: Vec<Segment>) -> Message {
        let mut message = Message::new();
        for segment in segments {
            message.segments.push(segment);
        }
        message
    }

    #[test]
    fn identical_messages_have_no_diffs() {
        let message = message_with(vec![pid("7", "DOE", "JOHN")]);
        assert!(diff_messages(&message, &message.clone()).is_empty());
    }

    #[test]
    fn changed_component_reports_leaf_path() {
        let old = message_with(vec![pid("7", "DOE", "JOHN")]);
        let new = message_with(vec![pid("7", "DOE", "JANE")]);
        let diffs = diff_messages(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "PID-5-2");
        assert_eq!(diffs[0].kind, DiffKind::Changed);
        assert_eq!(diffs[0].old.as_deref(), Some("JOHN"));
        assert_eq!(diffs[0].new.as_deref(), Some("JANE"));
    }

    #[test]
    fn added_segment_reports_occurrence_path() {
        let mut old_seg = Segment::new("OBX");
        old_seg.add_field(Field::from_value("1"));
        let mut new_seg_a = Segment::new("OBX");
        new_seg_a.add_field(Field::from_value("1"));
        let mut new_seg_b = Segment::new("OBX");
        new_seg_b.add_field(Field::from_value("2"));
        let old = message_with(vec![old_seg]);
        let new = message_with(vec![new_seg_a, new_seg_b]);
        let diffs = diff_messages(&old, &new);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "OBX(2)");
        assert_eq!(diffs[0].kind, DiffKind::Added);
    }

    #[test]
    fn removed_field_and_empty_equals_missing() {
        let mut old_seg = Segment::new("PID");
        old_seg.add_field(Field::from_value("1"));
        old_seg.add_field(Field::from_value("gone"));
        let mut new_seg = Segment::new("PID");
        new_seg.add_field(Field::from_value("1"));
        new_seg.add_field(Field::from_value(""));
        // Empty new field vs missing: no diff...
        let mut no_field = Segment::new("PID");
        no_field.add_field(Field::from_value("1"));
        let diffs = diff_messages(
            &message_with(vec![new_seg.clone()]),
            &message_with(vec![no_field]),
        );
        assert!(diffs.is_empty());
        // ...but clearing a field reports as a change to empty...
        let diffs = diff_messages(
            &message_with(vec![old_seg.clone()]),
            &message_with(vec![new_seg]),
        );
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "PID-2");
        assert_eq!(diffs[0].kind, DiffKind::Changed);
        assert_eq!(diffs[0].old.as_deref(), Some("gone"));
        assert_eq!(diffs[0].new.as_deref(), Some(""));
        // ...while dropping it entirely reports as removed.
        let mut dropped = Segment::new("PID");
        dropped.add_field(Field::from_value("1"));
        let diffs = diff_messages(&message_with(vec![old_seg]), &message_with(vec![dropped]));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "PID-2");
        assert_eq!(diffs[0].kind, DiffKind::Removed);
    }

    #[test]
    fn repetitions_get_indexed_paths() {
        let mut old_field = Field::new();
        for value in ["A", "B"] {
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(value));
            old_field.add_repetition(rep);
        }
        let mut new_field = Field::new();
        for value in ["A", "C"] {
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(value));
            new_field.add_repetition(rep);
        }
        let mut old_seg = Segment::new("PID");
        old_seg.add_field(old_field);
        let mut new_seg = Segment::new("PID");
        new_seg.add_field(new_field);
        let diffs = diff_messages(&message_with(vec![old_seg]), &message_with(vec![new_seg]));
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "PID-1[2]");
        assert_eq!(diffs[0].kind, DiffKind::Changed);
    }

    #[test]
    fn ignore_suppresses_paths_and_children() {
        let old = message_with(vec![pid("7", "DOE", "JOHN")]);
        let new = message_with(vec![pid("8", "DOE", "JANE")]);
        let diffs = diff_messages_ignoring(&old, &new, &["PID-5".to_string()]);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].path, "PID-3");
        // Prefix boundary: PID-3 must not suppress PID-30.
        let diffs = diff_messages_ignoring(&old, &new, &["PID-3".to_string()]);
        assert!(diffs.iter().all(|d| d.path != "PID-3"));
    }
}
