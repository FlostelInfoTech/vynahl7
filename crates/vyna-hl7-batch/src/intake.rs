//! Lenient parsing of inbound batch files.
//!
//! [`Intake::parse`] accepts a whole file drop (`FHS`...`FTS`), a single
//! batch (`BHS`...`BTS`), or a bare sequence of messages, and reports
//! per-message failures instead of rejecting the file because of one bad
//! message. Wrappers (`FHS`/`BHS`/`BTS`/`FTS`) are optional: missing trailers
//! simply leave the declared counts unknown.

use vyna_hl7_core::Message;

use crate::BatchError;

/// One message slot in an intake unit.
#[derive(Debug)]
pub struct IntakeMessage {
    /// 0-based position of the message within the intake unit.
    pub index: usize,
    /// MSH-10 when the MSH line was readable, even if the message failed.
    pub control_id: Option<String>,
    /// Parsed message, or the parse error text.
    pub result: Result<Message, String>,
}

/// One batch group in an intake unit.
#[derive(Debug)]
pub struct IntakeBatch {
    /// BHS-11 when a batch header was present.
    pub control_id: Option<String>,
    /// BTS-1 when a batch trailer was present.
    pub declared_message_count: Option<usize>,
    /// Messages in encounter order.
    pub messages: Vec<IntakeMessage>,
}

/// A leniently parsed inbound drop.
#[derive(Debug)]
pub struct Intake {
    /// FHS-11 when a file header was present.
    pub file_control_id: Option<String>,
    /// FTS-1 when a file trailer was present.
    pub declared_batch_count: Option<usize>,
    /// Batch groups in encounter order. Bare message sequences yield one
    /// group with no control id.
    pub batches: Vec<IntakeBatch>,
}

impl Intake {
    /// Parse an inbound drop. Never fails because of a bad message; message
    /// failures land in [`IntakeMessage::result`]. Only a structurally empty
    /// input is an error.
    pub fn parse(input: &str) -> Result<Self, BatchError> {
        let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);
        let lines: Vec<&str> = input
            .split('\r')
            .flat_map(|s| s.split('\n'))
            .map(str::trim_end)
            .filter(|s| !s.is_empty())
            .collect();
        if lines.is_empty() {
            return Err(BatchError::Invalid("empty input".to_string()));
        }

        let mut intake = Intake {
            file_control_id: None,
            declared_batch_count: None,
            batches: Vec::new(),
        };
        let mut index = 0usize;

        if lines[0].starts_with("FHS") {
            let (body, trailer) = match lines.last() {
                Some(last) if last.starts_with("FTS") => (&lines[1..lines.len() - 1], Some(*last)),
                _ => (&lines[1..], None),
            };
            intake.file_control_id = header_control_id(lines[0]);
            intake.declared_batch_count = trailer.and_then(trailer_count);
            split_groups(body, &mut intake.batches, &mut index);
        } else if lines[0].starts_with("BHS") {
            let (body, trailer) = match lines.last() {
                Some(last) if last.starts_with("BTS") && lines.len() > 1 => {
                    (&lines[1..lines.len() - 1], Some(*last))
                }
                _ => (&lines[1..], None),
            };
            let mut batch = IntakeBatch {
                control_id: header_control_id(lines[0]),
                declared_message_count: trailer.and_then(trailer_count),
                messages: Vec::new(),
            };
            split_messages(body, &mut batch.messages, &mut index);
            intake.batches.push(batch);
        } else {
            let mut batch = IntakeBatch {
                control_id: None,
                declared_message_count: None,
                messages: Vec::new(),
            };
            split_messages(&lines, &mut batch.messages, &mut index);
            intake.batches.push(batch);
        }

        Ok(intake)
    }

    /// Parse raw drop bytes as UTF-8.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, BatchError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| BatchError::Invalid(format!("drop is not valid UTF-8: {e}")))?;
        Self::parse(text)
    }

    /// Total message slots across all batches.
    pub fn message_count(&self) -> usize {
        self.batches.iter().map(|b| b.messages.len()).sum()
    }

    /// Successfully parsed messages, in encounter order.
    pub fn ok_messages(&self) -> Vec<&Message> {
        self.batches
            .iter()
            .flat_map(|b| b.messages.iter())
            .filter_map(|m| m.result.as_ref().ok())
            .collect()
    }

    /// Failed slots as `(index, control_id, error)`.
    pub fn errors(&self) -> Vec<(usize, Option<&str>, &str)> {
        self.batches
            .iter()
            .flat_map(|b| b.messages.iter())
            .filter_map(|m| match &m.result {
                Ok(_) => None,
                Err(e) => Some((m.index, m.control_id.as_deref(), e.as_str())),
            })
            .collect()
    }

    /// Number of successfully parsed messages.
    pub fn ok_count(&self) -> usize {
        self.ok_messages().len()
    }

    /// Number of failed message slots.
    pub fn error_count(&self) -> usize {
        self.errors().len()
    }

    /// Check declared trailer counts against actual content. Missing trailers
    /// are skipped, not errors.
    pub fn verify_counts(&self) -> Result<(), BatchError> {
        if let Some(declared) = self.declared_batch_count {
            let actual = self.batches.len();
            if declared != actual {
                return Err(BatchError::CountMismatch {
                    location: "FTS",
                    declared,
                    actual,
                });
            }
        }
        for batch in &self.batches {
            if let Some(declared) = batch.declared_message_count {
                let actual = batch.messages.len();
                if declared != actual {
                    return Err(BatchError::CountMismatch {
                        location: "BTS",
                        declared,
                        actual,
                    });
                }
            }
        }
        Ok(())
    }
}

/// Split file-body lines into `BHS`...`BTS` groups. Lines outside any group
/// (missing `BHS`) collect into an implicit group so nothing is silently lost.
fn split_groups(lines: &[&str], batches: &mut Vec<IntakeBatch>, index: &mut usize) {
    let mut current: Option<IntakeBatch> = None;
    let mut pending: Vec<&str> = Vec::new();

    let mut flush = |group: &mut Option<IntakeBatch>, pending: &mut Vec<&str>| {
        if let Some(mut batch) = group.take() {
            split_messages(pending, &mut batch.messages, index);
            pending.clear();
            batches.push(batch);
        } else if !pending.is_empty() {
            let mut batch = IntakeBatch {
                control_id: None,
                declared_message_count: None,
                messages: Vec::new(),
            };
            split_messages(pending, &mut batch.messages, index);
            pending.clear();
            batches.push(batch);
        }
    };

    for line in lines {
        if line.starts_with("BHS") {
            flush(&mut current, &mut pending);
            current = Some(IntakeBatch {
                control_id: header_control_id(line),
                declared_message_count: None,
                messages: Vec::new(),
            });
        } else if line.starts_with("BTS") {
            if let Some(batch) = current.as_mut() {
                batch.declared_message_count = trailer_count(line);
            }
            flush(&mut current, &mut pending);
        } else {
            pending.push(line);
        }
    }
    flush(&mut current, &mut pending);
}

/// Split group lines into messages on `MSH` boundaries. Stray non-`MSH` lines
/// outside any message become a single error slot each so senders can see them.
fn split_messages(lines: &[&str], out: &mut Vec<IntakeMessage>, index: &mut usize) {
    let mut chunk: Vec<&str> = Vec::new();
    let mut strays: Vec<&str> = Vec::new();

    for line in lines {
        if line.starts_with("MSH") {
            push_strays(out, index, &mut strays);
            push_chunk(out, index, &mut chunk);
            chunk.push(line);
        } else if chunk.is_empty() {
            strays.push(line);
        } else {
            chunk.push(line);
        }
    }
    push_strays(out, index, &mut strays);
    push_chunk(out, index, &mut chunk);
}

fn push_strays(out: &mut Vec<IntakeMessage>, index: &mut usize, strays: &mut Vec<&str>) {
    if strays.is_empty() {
        return;
    }
    out.push(IntakeMessage {
        index: *index,
        control_id: None,
        result: Err(format!(
            "expected MSH, found {} line(s) starting with '{}'",
            strays.len(),
            strays[0].chars().take(3).collect::<String>()
        )),
    });
    *index += 1;
    strays.clear();
}

fn push_chunk(out: &mut Vec<IntakeMessage>, index: &mut usize, chunk: &mut Vec<&str>) {
    if chunk.is_empty() {
        return;
    }
    let control_id = msh_control_id(chunk[0]);
    let result = vyna_hl7_parser::parse_message(&chunk.join("\r")).map_err(|e| e.to_string());
    out.push(IntakeMessage {
        index: *index,
        control_id,
        result,
    });
    *index += 1;
    chunk.clear();
}

/// FHS-11 / BHS-11 from a header line. Field 1 is the separator itself, so
/// field 11 is the 10th element after the segment id.
fn header_control_id(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    let sep = bytes[3] as char;
    line[4..]
        .split(sep)
        .nth(9)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// BTS-1 / FTS-1 from a trailer line.
fn trailer_count(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    let sep = bytes[3] as char;
    line[4..].split(sep).next()?.parse().ok()
}

/// MSH-10 from an MSH line. MSH-1 is the separator itself, so MSH-10 is the
/// 9th element after the segment id.
fn msh_control_id(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    if bytes.len() < 4 {
        return None;
    }
    let sep = bytes[3] as char;
    line[4..]
        .split(sep)
        .nth(8)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MSH1: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|MSG001|P|2.5";
    const MSH2: &str = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|MSG002|P|2.5";

    fn batch_text() -> String {
        format!("BHS|^~\\&|S|F|R|D|20240101||||B001\r{MSH1}\rPID|1||1\r{MSH2}\rPID|1||2\rBTS|2")
    }

    #[test]
    fn bare_messages_parse_with_control_ids() {
        let intake = Intake::parse(&format!("{MSH1}\rPID|1||1\r{MSH2}\r")).unwrap();
        assert_eq!(intake.batches.len(), 1);
        assert_eq!(intake.message_count(), 2);
        assert_eq!(intake.ok_count(), 2);
        assert_eq!(intake.errors().len(), 0);
        let oks = intake.ok_messages();
        assert_eq!(oks[0].get_control_id(), Some("MSG001"));
        assert_eq!(oks[1].get_control_id(), Some("MSG002"));
        assert!(intake.verify_counts().is_ok());
    }

    #[test]
    fn one_bad_message_does_not_kill_the_file() {
        // A bare "MSH" line has no delimiters and cannot parse.
        let text = format!("{MSH1}\rPID|1||1\rMSH\r{MSH2}\rPID|1||2\r");
        let intake = Intake::parse(&text).unwrap();
        assert_eq!(intake.message_count(), 3);
        assert_eq!(intake.ok_count(), 2);
        assert_eq!(intake.error_count(), 1);
        let errors = intake.errors();
        assert_eq!(errors[0].0, 1);
        // Good messages on both sides still parsed.
        let oks = intake.ok_messages();
        assert_eq!(oks[0].get_control_id(), Some("MSG001"));
        assert_eq!(oks[1].get_control_id(), Some("MSG002"));
    }

    #[test]
    fn stray_lines_become_error_slots() {
        let intake = Intake::parse(&format!("ZZZ|junk\r{MSH1}\r")).unwrap();
        assert_eq!(intake.message_count(), 2);
        assert_eq!(intake.ok_count(), 1);
        let errors = intake.errors();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, 0);
        assert!(errors[0].2.contains("expected MSH"));
    }

    #[test]
    fn batch_envelope_reports_ids_and_counts() {
        let intake = Intake::parse(&batch_text()).unwrap();
        assert_eq!(intake.batches.len(), 1);
        let batch = &intake.batches[0];
        assert_eq!(batch.control_id.as_deref(), Some("B001"));
        assert_eq!(batch.declared_message_count, Some(2));
        assert_eq!(intake.ok_count(), 2);
        assert!(intake.verify_counts().is_ok());
    }

    #[test]
    fn batch_count_mismatch_is_reported() {
        let text = batch_text().replace("BTS|2", "BTS|5");
        let intake = Intake::parse(&text).unwrap();
        assert_eq!(intake.ok_count(), 2);
        match intake.verify_counts() {
            Err(BatchError::CountMismatch {
                location: "BTS",
                declared: 5,
                actual: 2,
            }) => {}
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn file_envelope_with_two_batches() {
        // Reuse batch_text twice with distinct ids.
        let intake = Intake::parse(&format!(
            "FHS|^~\\&|S|F|R|D|20240101||||F001\r{}\r{}\rFTS|2",
            batch_text(),
            batch_text().replace("B001", "B002"),
        ))
        .unwrap();
        assert_eq!(intake.file_control_id.as_deref(), Some("F001"));
        assert_eq!(intake.declared_batch_count, Some(2));
        assert_eq!(intake.batches.len(), 2);
        assert_eq!(intake.message_count(), 4);
        assert_eq!(intake.ok_count(), 4);
        assert!(intake.verify_counts().is_ok());
    }

    #[test]
    fn missing_trailers_are_accepted() {
        let intake = Intake::parse("FHS|^~\\&|S|F|R|D|20240101||||F001\rBHS|^~\\&|S|F|R|D|20240101||||B7\rMSH|^~\\&|S|F|R|D|20240101||ADT^A01|M9|P|2.5").unwrap();
        assert_eq!(intake.ok_count(), 1);
        assert!(intake.verify_counts().is_ok());
        assert_eq!(intake.batches[0].control_id.as_deref(), Some("B7"));
    }

    #[test]
    fn empty_input_is_an_error() {
        assert!(matches!(
            Intake::parse("  \r\n "),
            Err(BatchError::Invalid(_))
        ));
        assert!(matches!(
            Intake::parse_bytes(b"\xff\xfe invalid"),
            Err(BatchError::Invalid(_))
        ));
    }

    #[test]
    fn builder_output_round_trips_through_intake() {
        let m1 = vyna_hl7_parser::parse_message(MSH1).unwrap();
        let m2 = vyna_hl7_parser::parse_message(MSH2).unwrap();
        let file = crate::BatchFileBuilder::new()
            .sending_application("S")
            .message(m1)
            .message(m2)
            .build_file();
        let intake = Intake::parse(&file.encode()).unwrap();
        assert_eq!(intake.ok_count(), 2);
        assert!(intake.verify_counts().is_ok());
    }
}
