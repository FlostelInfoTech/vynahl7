//! PPR^PC1 builder - Patient Problem (Add problem)

use super::{MessageBuilder, generate_control_id};
use crate::{Version, error::Result, field::Field, message::Message, segment::Segment};

/// One problem entry: a PRB segment with optional following NTE notes.
///
/// `Problem::new` takes the problem ID as `code^text^system` (e.g.
/// `38341003^Hypertension^SCT`), the problem instance ID, and the action
/// date/time. PRB-1 (action code) defaults to `AD` (add).
pub struct Problem {
    action_code: String,
    action_datetime: String,
    problem_id: String,
    instance_id: String,
    priority: Option<String>,
    classification: Option<String>,
    life_cycle_status: Option<String>,
    onset_datetime: Option<String>,
    notes: Vec<String>,
}

impl Problem {
    pub fn new(problem_id: &str, instance_id: &str, action_datetime: &str) -> Self {
        Self {
            action_code: "AD".to_string(),
            action_datetime: action_datetime.to_string(),
            problem_id: problem_id.to_string(),
            instance_id: instance_id.to_string(),
            priority: None,
            classification: None,
            life_cycle_status: None,
            onset_datetime: None,
            notes: Vec::new(),
        }
    }

    pub fn action_code(mut self, code: &str) -> Self {
        self.action_code = code.to_string();
        self
    }

    pub fn priority(mut self, priority: &str) -> Self {
        self.priority = Some(priority.to_string());
        self
    }

    pub fn classification(mut self, classification: &str) -> Self {
        self.classification = Some(classification.to_string());
        self
    }

    pub fn life_cycle_status(mut self, status: &str) -> Self {
        self.life_cycle_status = Some(status.to_string());
        self
    }

    pub fn onset_datetime(mut self, datetime: &str) -> Self {
        self.onset_datetime = Some(datetime.to_string());
        self
    }

    /// Add a free-text note, emitted as an NTE segment after the PRB.
    pub fn note(mut self, text: &str) -> Self {
        self.notes.push(text.to_string());
        self
    }
}

/// Set a segment field, ignoring the impossible index-0 error.
fn set_field(segment: &mut Segment, index: usize, value: &str) {
    let _ = segment.set_field(index, Field::from_value(value));
}

/// Builder for PPR^PC1 - Patient Problem (Add problem)
pub struct PprPc1Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    date_of_birth: Option<String>,
    sex: Option<String>,
    patient_class: Option<String>,
    problems: Vec<Problem>,
    extra_segments: Vec<Segment>,
}

impl PprPc1Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "PPR", "PC1"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            date_of_birth: None,
            sex: None,
            patient_class: None,
            problems: Vec::new(),
            extra_segments: Vec::new(),
        }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.sending_app = app.to_string();
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.sending_facility = facility.to_string();
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.receiving_app = app.to_string();
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.receiving_facility = facility.to_string();
        self
    }

    pub fn control_id(mut self, id: &str) -> Self {
        self.control_id = Some(id.to_string());
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.patient_id = Some(id.to_string());
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.patient_name = Some((family.to_string(), given.to_string()));
        self
    }

    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.date_of_birth = Some(dob.to_string());
        self
    }

    pub fn sex(mut self, sex: &str) -> Self {
        self.sex = Some(sex.to_string());
        self
    }

    /// PV1-2 patient class (required by the schema when PV1 is validated).
    pub fn patient_class(mut self, class: &str) -> Self {
        self.patient_class = Some(class.to_string());
        self
    }

    /// Add one problem (PRB plus its NTE notes).
    pub fn add_problem(mut self, problem: Problem) -> Self {
        self.problems.push(problem);
        self
    }

    /// Append a caller-built segment (e.g. OBX, VAR, ROL) after the problems.
    pub fn add_segment(mut self, segment: Segment) -> Self {
        self.extra_segments.push(segment);
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self.control_id.take().unwrap_or_else(generate_control_id);

        let msh = self.base.create_msh(
            &self.sending_app,
            &self.sending_facility,
            &self.receiving_app,
            &self.receiving_facility,
            &control_id,
            "P",
        )?;
        self.base.message.add_segment(msh);

        // PID segment, sparse: unset fields stay absent instead of ""
        // placeholders, so optional composite fields do not trip
        // required-component checks during validation.
        let mut pid = Segment::new("PID");
        set_field(&mut pid, 1, "1");
        if let Some(id) = self.patient_id.as_deref() {
            set_field(&mut pid, 3, id);
        }
        if let Some((family, given)) = self.patient_name.as_ref() {
            set_field(&mut pid, 5, &format!("{family}^{given}"));
        }
        if let Some(dob) = self.date_of_birth.as_deref() {
            set_field(&mut pid, 7, dob);
        }
        if let Some(sex) = self.sex.as_deref() {
            set_field(&mut pid, 8, sex);
        }
        self.base.message.add_segment(pid);

        // PV1 segment (PV1-2 patient class is schema-required)
        let mut pv1 = Segment::new("PV1");
        set_field(&mut pv1, 1, "1");
        if let Some(class) = self.patient_class.as_deref() {
            set_field(&mut pv1, 2, class);
        }
        self.base.message.add_segment(pv1);

        let mut note_id = 1;
        for problem in &self.problems {
            let mut prb = Segment::new("PRB");
            set_field(&mut prb, 1, &problem.action_code);
            set_field(&mut prb, 2, &problem.action_datetime);
            set_field(&mut prb, 3, &problem.problem_id);
            set_field(&mut prb, 4, &problem.instance_id);
            if let Some(priority) = problem.priority.as_deref() {
                set_field(&mut prb, 6, priority);
            }
            if let Some(classification) = problem.classification.as_deref() {
                set_field(&mut prb, 10, classification);
            }
            if let Some(status) = problem.life_cycle_status.as_deref() {
                set_field(&mut prb, 14, status);
            }
            if let Some(onset) = problem.onset_datetime.as_deref() {
                set_field(&mut prb, 16, onset);
            }
            self.base.message.add_segment(prb);

            for note in &problem.notes {
                let mut nte = Segment::new("NTE");
                set_field(&mut nte, 1, &note_id.to_string());
                set_field(&mut nte, 3, note);
                self.base.message.add_segment(nte);
                note_id += 1;
            }
        }

        for segment in self.extra_segments.drain(..) {
            self.base.message.add_segment(segment);
        }

        Ok(self.base.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_builder() -> PprPc1Builder {
        PprPc1Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .control_id("MSG001")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .patient_class("O")
    }

    #[test]
    fn test_ppr_builds_msh_pid_pv1_and_problem() {
        let message = minimal_builder()
            .add_problem(Problem::new(
                "38341003^Hypertension^SCT",
                "PROB-1",
                "20240101100000",
            ))
            .build()
            .unwrap();

        let msh = message.get_segment_by_id("MSH").unwrap();
        let msg_type = msh
            .get_field(9)
            .unwrap()
            .get_repetition(0)
            .unwrap()
            .full_value();
        assert_eq!(msg_type.as_deref(), Some("PPR^PC1"));

        let pid = message.get_segment_by_id("PID").unwrap();
        assert_eq!(pid.get_field_value(3), Some("12345"));

        let pv1 = message.get_segment_by_id("PV1").unwrap();
        assert_eq!(pv1.get_field_value(2), Some("O"));

        let prb = message.get_segment_by_id("PRB").unwrap();
        assert_eq!(prb.get_field_value(1), Some("AD"));
        assert_eq!(prb.get_field_value(2), Some("20240101100000"));
        assert_eq!(prb.get_field_value(3), Some("38341003^Hypertension^SCT"));
        assert_eq!(prb.get_field_value(4), Some("PROB-1"));
    }

    #[test]
    fn test_ppr_multiple_problems_stay_in_order() {
        let message = minimal_builder()
            .add_problem(Problem::new(
                "38341003^Hypertension^SCT",
                "PROB-1",
                "20240101100000",
            ))
            .add_problem(Problem::new(
                "44054006^Diabetes^SCT",
                "PROB-2",
                "20240102100000",
            ))
            .build()
            .unwrap();

        let prbs = message.get_segments_by_id("PRB");
        assert_eq!(prbs.len(), 2);
        assert_eq!(prbs[0].get_field_value(4), Some("PROB-1"));
        assert_eq!(prbs[1].get_field_value(4), Some("PROB-2"));
    }

    #[test]
    fn test_ppr_notes_become_nte_segments() {
        let message = minimal_builder()
            .add_problem(
                Problem::new("38341003^Hypertension^SCT", "PROB-1", "20240101100000")
                    .note("Stage 1, monitor")
                    .note("Recheck in 3 months"),
            )
            .build()
            .unwrap();

        let ntes = message.get_segments_by_id("NTE");
        assert_eq!(ntes.len(), 2);
        assert_eq!(ntes[0].get_field_value(1), Some("1"));
        assert_eq!(ntes[0].get_field_value(3), Some("Stage 1, monitor"));
        assert_eq!(ntes[1].get_field_value(1), Some("2"));
        assert_eq!(ntes[1].get_field_value(3), Some("Recheck in 3 months"));
    }

    #[test]
    fn test_ppr_optional_problem_fields() {
        let message = minimal_builder()
            .add_problem(
                Problem::new("38341003^Hypertension^SCT", "PROB-1", "20240101100000")
                    .priority("1")
                    .classification("359939006^Diagnosis^SCT")
                    .life_cycle_status("A^Active^HL70528")
                    .onset_datetime("20230101"),
            )
            .build()
            .unwrap();

        let prb = message.get_segment_by_id("PRB").unwrap();
        assert_eq!(prb.get_field_value(6), Some("1"));
        assert_eq!(prb.get_field_value(10), Some("359939006^Diagnosis^SCT"));
        assert_eq!(prb.get_field_value(14), Some("A^Active^HL70528"));
        assert_eq!(prb.get_field_value(16), Some("20230101"));
    }
}
