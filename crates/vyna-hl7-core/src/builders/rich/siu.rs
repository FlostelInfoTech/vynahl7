//! SIU (Scheduling Information Unsolicited) message builders.
//!
//! One canonical builder — `SiuS12Builder` — carries all the SCH /
//! PV1 / RGS / AIP / AIL fields our outbound flows populate. Variants
//! (S13 reschedule / S14 modify / S15 cancel / S26 no-show) share the
//! same structure per HL7 v2.5 (all use the ADT_S12 / SIU_S12 message
//! structure); the variant is expressed via [`SiuS12Builder::trigger_event`]
//! and the SCH-6 event reason.

use super::VisitInfo;
use super::{
    MessageBuilder, PatientAddress, PatientDemographics, PatientIdentifier, generate_control_id,
};
use crate::{
    Version,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
};

/// Resource slot emitted as RGS + AIP (+ optional AIL) triples. A
/// typical appointment carries one resource group with one provider
/// resource, but the builder accepts multiples for cases where the
/// appointment blocks a room and an attending at the same time.
#[derive(Debug, Clone, Default)]
pub struct AppointmentResource {
    pub resource_group_id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub start_datetime: String,
    pub duration_minutes: i32,
    pub location: Option<String>,
}

pub struct SiuS12Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    // SCH
    placer_appointment_id: Option<String>,
    filler_appointment_id: Option<String>,
    event_reason: Option<String>,
    appointment_reason: Option<String>,
    appointment_type: Option<String>,
    appointment_datetime: Option<String>,
    appointment_end_datetime: Option<String>,
    duration_minutes: Option<i32>,
    filler_status_code: Option<String>,
    // PID
    demographics: PatientDemographics,
    // PV1 — minimal by default; visit_number/admit fields opt-in when
    // the SIU carries an encounter link.
    visit: VisitInfo,
    // Resources (RGS/AIP/AIL per slot)
    resources: Vec<AppointmentResource>,
}

impl SiuS12Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "SIU", "S12");
        // Every SIU variant shares the SIU_S12 message structure per v2.5.
        base.message_structure("SIU_S12");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            placer_appointment_id: None,
            filler_appointment_id: None,
            event_reason: None,
            appointment_reason: None,
            appointment_type: None,
            appointment_datetime: None,
            appointment_end_datetime: None,
            duration_minutes: None,
            filler_status_code: None,
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            resources: Vec::new(),
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
    pub fn processing_id(mut self, id: &str) -> Self {
        self.processing_id = id.to_string();
        self
    }

    /// Override trigger event (S12/S13/S14/S15/S26). MSH-9 second
    /// component only; the third component stays SIU_S12 since all
    /// SIU variants share that message structure.
    pub fn trigger_event(mut self, trigger: &str) -> Self {
        self.base.set_trigger_event(trigger);
        self
    }

    // ── SCH ────────────────────────────────────────────────────────

    pub fn placer_appointment_id(mut self, id: &str) -> Self {
        self.placer_appointment_id = Some(id.to_string());
        self
    }
    pub fn filler_appointment_id(mut self, id: &str) -> Self {
        self.filler_appointment_id = Some(id.to_string());
        self
    }
    /// SCH-6 event reason (usually the SIU trigger — "S12" etc.).
    pub fn event_reason(mut self, reason: &str) -> Self {
        self.event_reason = Some(reason.to_string());
        self
    }
    /// SCH-7 appointment reason (human free-text).
    pub fn appointment_reason(mut self, reason: &str) -> Self {
        self.appointment_reason = Some(reason.to_string());
        self
    }
    /// SCH-8 appointment type.
    pub fn appointment_type(mut self, appt_type: &str) -> Self {
        self.appointment_type = Some(appt_type.to_string());
        self
    }
    /// SCH-11-4 + AIP-6 — scheduled start date/time.
    pub fn appointment_datetime(mut self, datetime: &str) -> Self {
        self.appointment_datetime = Some(datetime.to_string());
        self
    }
    /// SCH-11-5 — scheduled end date/time.
    ///
    /// Supplied by the caller rather than derived from
    /// `appointment_datetime + duration_minutes`: the scheduler holds a real
    /// end and a derived one would silently disagree with it the first time
    /// the two drift.
    pub fn appointment_end_datetime(mut self, datetime: &str) -> Self {
        self.appointment_end_datetime = Some(datetime.to_string());
        self
    }
    /// SCH-11 quantity / AIP-9 — duration in minutes.
    pub fn duration_minutes(mut self, minutes: i32) -> Self {
        self.duration_minutes = Some(minutes);
        self
    }
    /// SCH-25 filler status code (often the cancellation reason for
    /// S15 messages).
    pub fn filler_status_code(mut self, code: &str) -> Self {
        self.filler_status_code = Some(code.to_string());
        self
    }

    // ── PID ────────────────────────────────────────────────────────

    /// Set the whole PID payload at once.
    ///
    /// Lets a caller assemble demographics once and hand the same struct to
    /// every message type, rather than re-deriving the patient wiring per
    /// builder — which is how the gateway's ten call sites came to disagree
    /// about which PID fields get sent.
    pub fn demographics(mut self, demographics: PatientDemographics) -> Self {
        self.demographics = demographics;
        self
    }
    pub fn patient_id(mut self, id: &str) -> Self {
        self.demographics.patient_id_flat = Some(id.to_string());
        self
    }
    pub fn patient_identifier(mut self, identifier: PatientIdentifier) -> Self {
        self.demographics.identifier = Some(identifier);
        self
    }
    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.demographics.name = Some((family.to_string(), given.to_string()));
        self
    }
    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.demographics.date_of_birth = Some(dob.to_string());
        self
    }
    pub fn sex(mut self, sex: &str) -> Self {
        self.demographics.sex = Some(sex.to_string());
        self
    }
    pub fn patient_address(mut self, address: PatientAddress) -> Self {
        self.demographics.address = Some(address);
        self
    }
    pub fn phone_home(mut self, phone: &str) -> Self {
        self.demographics.phone_home = Some(phone.to_string());
        self
    }

    // ── PV1 ────────────────────────────────────────────────────────

    /// PV1-2 patient class. Defaults to "O" (outpatient) when not set.
    pub fn patient_class(mut self, class: &str) -> Self {
        self.visit.patient_class = Some(class.to_string());
        self
    }
    /// PV1-4 admission type. Only relevant when the SIU carries an
    /// inpatient encounter link.
    pub fn admission_type(mut self, admission_type: &str) -> Self {
        self.visit.admission_type = Some(admission_type.to_string());
        self
    }
    /// PV1-7 attending doctor.
    pub fn attending_doctor(mut self, doctor_id: &str) -> Self {
        self.visit.attending_doctor = Some(doctor_id.to_string());
        self
    }
    /// PV1-19 visit number — the EMR's encounter identifier.
    pub fn visit_number(mut self, visit_number: &str) -> Self {
        self.visit.visit_number = Some(visit_number.to_string());
        self
    }
    /// PV1-44 admit date/time.
    pub fn admit_datetime(mut self, datetime: &str) -> Self {
        self.visit.admit_datetime = Some(datetime.to_string());
        self
    }

    /// Supply the whole visit at once.
    ///
    /// PV1 is rendered by [`VisitInfo::to_pv1_segment`], the single builder
    /// every ADT already uses. This message type used to hand-roll its own
    /// PV1 carrying four fields; the shared one carries twenty and pads to
    /// 52, so a receiver reconciling this message against the admit reads
    /// the same visit description in both.
    pub fn visit(mut self, visit: VisitInfo) -> Self {
        self.visit = visit;
        self
    }

    // ── Resources ──────────────────────────────────────────────────

    pub fn add_resource(mut self, resource: AppointmentResource) -> Self {
        self.resources.push(resource);
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self.control_id.unwrap_or_else(generate_control_id);

        let msh = self.base.create_msh(
            &self.sending_app,
            &self.sending_facility,
            &self.receiving_app,
            &self.receiving_facility,
            &control_id,
            &self.processing_id,
        )?;
        self.base.message.add_segment(msh);

        // SCH positionally through SCH-25.
        let mut sch = Segment::new("SCH");
        sch.add_field(Field::from_value(
            self.placer_appointment_id.as_deref().unwrap_or(""),
        )); // SCH-1
        sch.add_field(Field::from_value(
            self.filler_appointment_id.as_deref().unwrap_or(""),
        )); // SCH-2
        for _ in 3..=5 {
            sch.add_field(Field::from_value(""));
        }
        sch.add_field(Field::from_value(
            self.event_reason.as_deref().unwrap_or(""),
        )); // SCH-6
        sch.add_field(Field::from_value(
            self.appointment_reason.as_deref().unwrap_or(""),
        )); // SCH-7
        sch.add_field(Field::from_value(
            self.appointment_type.as_deref().unwrap_or(""),
        )); // SCH-8
        sch.add_field(Field::from_value("")); // SCH-9 duration (we use SCH-11 quantity)
        sch.add_field(Field::from_value("")); // SCH-10
        // SCH-11 Appointment Timing Quantity, a TQ. The start belongs in
        // TQ-4 and the end in TQ-5 — where HL7 v2.5 puts them, and where our
        // own inbound handler reads them (`SCH-11-4` / `SCH-11-5`).
        //
        // This used to be `format!("{dt}^^^{mins}^min")` handed to
        // `Field::from_value`, which was wrong twice over. The `^` were
        // ESCAPED to `\S\` by the value constructor, so the field held one
        // component and TQ-4 did not exist; and the components were
        // misplaced anyway — the start sat in TQ-1 (Quantity), the duration
        // in TQ-4 and the literal `min` in TQ-5. A receiver reading the
        // standard way got nothing, and the message could not survive this
        // library's own parser. Same defect class as ORC-12, RXA-10 and
        // RXA-11: a composite assembled as a string, then emitted as a
        // literal value.
        //
        // Built as COMPONENTS so nothing is escaped. TQ-1..TQ-3 stay empty:
        // the duration is already carried by AIP-9 and is derivable from the
        // start and end, and inventing a units code for a field nobody reads
        // would assert more than we know.
        let mut timing = Field::new();
        let mut tq = Repetition::new();
        for part in [
            "",
            "",
            "",
            self.appointment_datetime.as_deref().unwrap_or(""),
            self.appointment_end_datetime.as_deref().unwrap_or(""),
        ] {
            tq.add_component(Component::from_value(part));
        }
        timing.add_repetition(tq);
        sch.add_field(timing);
        for _ in 12..=24 {
            sch.add_field(Field::from_value(""));
        }
        sch.add_field(Field::from_value(
            self.filler_status_code.as_deref().unwrap_or(""),
        )); // SCH-25
        self.base.message.add_segment(sch);

        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());

        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }

        // Resource slots — emit RGS / AIP / AIL per resource.
        for (idx, res) in self.resources.iter().enumerate() {
            let mut rgs = Segment::new("RGS");
            rgs.add_field(Field::from_value((idx + 1).to_string())); // RGS-1
            rgs.add_field(Field::from_value("")); // RGS-2 action code
            rgs.add_field(Field::from_value(&res.resource_group_id)); // RGS-3
            self.base.message.add_segment(rgs);

            let mut aip = Segment::new("AIP");
            aip.add_field(Field::from_value("1")); // AIP-1
            aip.add_field(Field::from_value("")); // AIP-2 action code
            // AIP-3 personnel resource ID (XCN: id^name).
            let mut aip3 = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(&res.provider_id));
            rep.add_component(Component::from_value(&res.provider_name));
            aip3.add_repetition(rep);
            aip.add_field(aip3);
            aip.add_field(Field::from_value("")); // AIP-4 resource type
            aip.add_field(Field::from_value("")); // AIP-5 resource group
            aip.add_field(Field::from_value(&res.start_datetime)); // AIP-6
            aip.add_field(Field::from_value("")); // AIP-7
            aip.add_field(Field::from_value("")); // AIP-8
            aip.add_field(Field::from_value(res.duration_minutes.to_string())); // AIP-9
            self.base.message.add_segment(aip);

            if let Some(loc) = &res.location {
                let mut ail = Segment::new("AIL");
                ail.add_field(Field::from_value("1")); // AIL-1
                ail.add_field(Field::from_value("")); // AIL-2
                ail.add_field(Field::from_value(loc)); // AIL-3
                self.base.message.add_segment(ail);
            }
        }

        Ok(self.base.build())
    }
}
