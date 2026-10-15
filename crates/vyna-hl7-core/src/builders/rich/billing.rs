//! BAR (Add/Change Billing Account) message builders.

use super::VisitInfo;
use super::referral::{Diagnosis, emit_dg1};
use super::{
    MessageBuilder, PatientAddress, PatientDemographics, PatientIdentifier, generate_control_id,
    rich_base,
};
use crate::{
    Version,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
};

/// Guarantor / insurance payload surfaced on GT1 + IN1.
#[derive(Debug, Clone, Default)]
pub struct InsuranceInfo {
    /// GT1-3 guarantor name.
    pub guarantor_name: Option<String>,
    /// GT1-5 guarantor address, STRUCTURED.
    ///
    /// This was a single `CONCAT_WS(', ', ...)` string, so the whole address
    /// — including the country — landed in XAD component 1, the street line.
    /// A receiver reading GT1-5-3 for the city got nothing. PID-11 splits the
    /// same data correctly, so one message carried one address in two
    /// different shapes.
    pub guarantor_address: Option<PatientAddress>,
    /// GT1-11 guarantor relationship to the patient (HL7 table 0063).
    pub guarantor_relationship: Option<String>,
    /// GT1-6 guarantor home phone.
    pub guarantor_phone: Option<String>,
    /// IN1-3 insurance company ID (CX) — required by the schema.
    pub company_id: Option<String>,
    /// IN1-4 insurance company name (payer).
    pub payer_name: Option<String>,
    /// IN1-2 insurance plan ID, text component. The plan's NAME
    /// ("Gold PPO 2025") — a marketing label, not a code.
    ///
    /// This used to feed IN1-15, which is Plan TYPE (HL7 table 0086, a coded
    /// field). A product name in a coded slot is the same defect as OBR-4 and
    /// ORC-12: a value in the code position that is not a code.
    pub plan_name: Option<String>,
    /// IN1-15 plan type — the CODED value (table 0086).
    pub plan_type: Option<String>,
    /// IN1-36 policy number.
    pub member_id: Option<String>,
    /// IN1-49 insured's ID number.
    ///
    /// Distinct from the policy number, and kept separate: these were
    /// COALESCE'd into one value that went out as IN1-36, so an insured's ID
    /// was transmitted as a policy number. The gateway's own comment already
    /// named IN1-49 as this field's home.
    pub subscriber_id: Option<String>,
    /// IN1-8 group number.
    pub group_number: Option<String>,
    /// IN1-12 plan effective date.
    pub coverage_start_date: Option<String>,
    /// IN1-13 plan expiration date.
    pub coverage_end_date: Option<String>,
}

pub struct RichBarP01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    event_datetime: Option<String>,
    demographics: PatientDemographics,
    // PV1
    visit: VisitInfo,
    diagnosis: Option<Diagnosis>,
    insurance: InsuranceInfo,
}

impl RichBarP01Builder {
    pub fn new(version: Version) -> Self {
        let mut base = rich_base(version, "BAR", "P01");
        base.message_structure("BAR_P01");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            event_datetime: None,
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            diagnosis: None,
            insurance: InsuranceInfo::default(),
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
    pub fn event_datetime(mut self, datetime: &str) -> Self {
        self.event_datetime = Some(datetime.to_string());
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

    // ── PV1 (full admit PV1 — BAR always carries a live encounter) ──

    pub fn patient_class(mut self, class: &str) -> Self {
        self.visit.patient_class = Some(class.to_string());
        self
    }
    pub fn admission_type(mut self, admission_type: &str) -> Self {
        self.visit.admission_type = Some(admission_type.to_string());
        self
    }
    pub fn attending_doctor(mut self, doctor_id: &str) -> Self {
        self.visit.attending_doctor = Some(doctor_id.to_string());
        self
    }
    pub fn visit_number(mut self, visit_number: &str) -> Self {
        self.visit.visit_number = Some(visit_number.to_string());
        self
    }
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

    pub fn diagnosis(mut self, diagnosis: Diagnosis) -> Self {
        self.diagnosis = Some(diagnosis);
        self
    }
    pub fn insurance(mut self, insurance: InsuranceInfo) -> Self {
        self.insurance = insurance;
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

        let evn = self
            .base
            .create_evn("P01", self.event_datetime.as_deref())?;
        self.base.message.add_segment(evn);

        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());

        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }

        // DG1 (shared with REF via referral::emit_dg1).
        let dg1 = self.diagnosis.take().unwrap_or_default();
        self.base.message.add_segment(emit_dg1(&dg1));

        // GT1 positional through GT1-6.
        let ins = &self.insurance;
        let mut gt1 = Segment::new("GT1");
        gt1.add_field(Field::from_value("1")); // GT1-1 set ID
        gt1.add_field(Field::from_value("")); // GT1-2 guarantor number
        gt1.add_field(Field::from_value(
            ins.guarantor_name.as_deref().unwrap_or(""),
        )); // GT1-3
        gt1.add_field(Field::from_value("")); // GT1-4 spouse name
        // GT1-5 Guarantor Address (XAD), component-structured as PID-11 is.
        match &ins.guarantor_address {
            Some(addr) => {
                let mut field = Field::new();
                let mut rep = Repetition::new();
                rep.add_component(Component::from_value(&addr.line1));
                rep.add_component(Component::from_value(&addr.line2));
                rep.add_component(Component::from_value(&addr.city));
                rep.add_component(Component::from_value(&addr.state));
                rep.add_component(Component::from_value(&addr.postal_code));
                rep.add_component(Component::from_value(&addr.country));
                field.add_repetition(rep);
                gt1.add_field(field);
            }
            None => gt1.add_field(Field::from_value("")),
        }
        gt1.add_field(Field::from_value(
            ins.guarantor_phone.as_deref().unwrap_or(""),
        )); // GT1-6
        // GT1-7 .. GT1-10 — no source.
        for _ in 7..=10 {
            gt1.add_field(Field::from_value(""));
        }
        // GT1-11 Guarantor Relationship. Empty when unrecorded rather than
        // assuming "self": whether the patient is their own guarantor is
        // exactly the question this field answers.
        gt1.add_field(Field::from_value(
            ins.guarantor_relationship.as_deref().unwrap_or(""),
        ));
        self.base.message.add_segment(gt1);

        // IN1 positional through IN1-36 (sparse — most fields empty).
        let mut in1 = Segment::new("IN1");
        in1.add_field(Field::from_value("1")); // IN1-1
        // IN1-2 Insurance Plan ID (CE). No plan code is held, so the name
        // travels as the text component rather than being forced into a
        // coded field.
        match ins.plan_name.as_deref() {
            Some(name) if !name.is_empty() => {
                let mut field = Field::new();
                let mut rep = Repetition::new();
                rep.add_component(Component::from_value(""));
                rep.add_component(Component::from_value(name));
                field.add_repetition(rep);
                in1.add_field(field);
            }
            _ => in1.add_field(Field::from_value("")),
        }
        // IN1-3 Insurance Company ID (CX) — required by the schema.
        in1.add_field(Field::from_delimited(
            ins.company_id.as_deref().unwrap_or(""),
        ));
        in1.add_field(Field::from_value(ins.payer_name.as_deref().unwrap_or(""))); // IN1-4
        for _ in 5..=7 {
            in1.add_field(Field::from_value(""));
        }
        in1.add_field(Field::from_value(ins.group_number.as_deref().unwrap_or(""))); // IN1-8
        for _ in 9..=11 {
            in1.add_field(Field::from_value(""));
        }
        in1.add_field(Field::from_value(
            ins.coverage_start_date.as_deref().unwrap_or(""),
        )); // IN1-12
        in1.add_field(Field::from_value(
            ins.coverage_end_date.as_deref().unwrap_or(""),
        )); // IN1-13
        in1.add_field(Field::from_value("")); // IN1-14
        // IN1-15 Plan Type (HL7 table 0086). Coded, so it takes `plan_type`
        // and is EMPTY when none is recorded — a plan whose type we do not
        // know is not a plan of type "Gold PPO 2025".
        in1.add_field(Field::from_value(ins.plan_type.as_deref().unwrap_or(""))); // IN1-15
        for _ in 16..=35 {
            in1.add_field(Field::from_value(""));
        }
        in1.add_field(Field::from_value(ins.member_id.as_deref().unwrap_or(""))); // IN1-36
        // IN1-37 .. IN1-48 — no source.
        for _ in 37..=48 {
            in1.add_field(Field::from_value(""));
        }
        // IN1-49 Insured's ID Number — the insured's own identifier, which is
        // not the policy number.
        in1.add_field(Field::from_value(
            ins.subscriber_id.as_deref().unwrap_or(""),
        ));
        self.base.message.add_segment(in1);

        Ok(self.base.build())
    }
}
