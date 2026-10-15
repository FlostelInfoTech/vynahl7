//! DFT (Detailed Financial Transaction) message builders.
//!
//! One canonical builder — `RichDftP03Builder` — carries the MSH / EVN /
//! PID / PV1 / FT1 fields our outbound billing flows populate. Variants
//! (P11) share the same structure per HL7 v2.5 and can be expressed
//! via [`RichDftP03Builder::trigger_event`].

use super::VisitInfo;
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

pub struct RichDftP03Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    // EVN
    event_datetime: Option<String>,
    // PID
    demographics: PatientDemographics,
    // PV1
    visit: VisitInfo,
    // FT1
    transaction_id: Option<String>,
    transaction_batch_id: Option<String>,
    transaction_date: Option<String>,
    transaction_posting_date: Option<String>,
    transaction_type: Option<String>,
    charge_code: Option<String>,
    charge_description: Option<String>,
    transaction_quantity: Option<String>,
    transaction_amount: Option<String>,
    performed_by_id: Option<String>,
    performed_by_name: Option<String>,
}

impl RichDftP03Builder {
    pub fn new(version: Version) -> Self {
        let mut base = rich_base(version, "DFT", "P03");
        base.message_structure("DFT_P03");
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
            transaction_id: None,
            transaction_batch_id: None,
            transaction_date: None,
            transaction_posting_date: None,
            transaction_type: None,
            charge_code: None,
            charge_description: None,
            transaction_quantity: None,
            transaction_amount: None,
            performed_by_id: None,
            performed_by_name: None,
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

    /// Override trigger event (P03 / P11). MSH-9 second component
    /// only; the third component stays DFT_P03 since both variants
    /// share that message structure.
    pub fn trigger_event(mut self, trigger: &str) -> Self {
        self.base.set_trigger_event(trigger);
        self
    }

    // ── EVN ────────────────────────────────────────────────────────

    /// EVN-2 recorded date/time. Defaults to the current local time
    /// when unset.
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

    // ── PV1 ────────────────────────────────────────────────────────

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

    // ── FT1 ────────────────────────────────────────────────────────

    pub fn transaction_id(mut self, id: &str) -> Self {
        self.transaction_id = Some(id.to_string());
        self
    }
    pub fn transaction_batch_id(mut self, id: &str) -> Self {
        self.transaction_batch_id = Some(id.to_string());
        self
    }
    pub fn transaction_date(mut self, date: &str) -> Self {
        self.transaction_date = Some(date.to_string());
        self
    }
    pub fn transaction_posting_date(mut self, date: &str) -> Self {
        self.transaction_posting_date = Some(date.to_string());
        self
    }
    /// FT1-6 transaction type (CG charge / PY payment / AJ adjustment).
    pub fn transaction_type(mut self, txn_type: &str) -> Self {
        self.transaction_type = Some(txn_type.to_string());
        self
    }
    /// FT1-7 transaction code (CE data type: charge_code^description).
    pub fn transaction_code(mut self, code: &str, description: &str) -> Self {
        self.charge_code = Some(code.to_string());
        self.charge_description = Some(description.to_string());
        self
    }
    pub fn transaction_quantity(mut self, quantity: &str) -> Self {
        self.transaction_quantity = Some(quantity.to_string());
        self
    }
    pub fn transaction_amount(mut self, amount: &str) -> Self {
        self.transaction_amount = Some(amount.to_string());
        self
    }
    /// FT1-20 performed by (XCN: id^name).
    pub fn performed_by(mut self, id: &str, name: &str) -> Self {
        self.performed_by_id = Some(id.to_string());
        self.performed_by_name = Some(name.to_string());
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

        let evn = self.base.create_evn(
            self.base.trigger_event().to_string().as_str(),
            self.event_datetime.as_deref(),
        )?;
        self.base.message.add_segment(evn);

        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());

        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }

        // FT1 positional through FT1-20.
        let mut ft1 = Segment::new("FT1");
        ft1.add_field(Field::from_value("1")); // FT1-1 Set ID
        ft1.add_field(Field::from_value(
            self.transaction_id.as_deref().unwrap_or(""),
        )); // FT1-2
        ft1.add_field(Field::from_value(
            self.transaction_batch_id.as_deref().unwrap_or(""),
        )); // FT1-3
        ft1.add_field(Field::from_value(
            self.transaction_date.as_deref().unwrap_or(""),
        )); // FT1-4
        ft1.add_field(Field::from_value(
            self.transaction_posting_date.as_deref().unwrap_or(""),
        )); // FT1-5
        ft1.add_field(Field::from_value(
            self.transaction_type.as_deref().unwrap_or(""),
        )); // FT1-6
        // FT1-7 transaction code (CE: code^description).
        if self.charge_code.is_some() || self.charge_description.is_some() {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(
                self.charge_code.as_deref().unwrap_or(""),
            ));
            rep.add_component(Component::from_value(
                self.charge_description.as_deref().unwrap_or(""),
            ));
            field.add_repetition(rep);
            ft1.add_field(field);
        } else {
            ft1.add_field(Field::from_value(""));
        }
        ft1.add_field(Field::from_value("")); // FT1-8
        ft1.add_field(Field::from_value("")); // FT1-9
        ft1.add_field(Field::from_value(
            self.transaction_quantity.as_deref().unwrap_or(""),
        )); // FT1-10
        ft1.add_field(Field::from_value(
            self.transaction_amount.as_deref().unwrap_or(""),
        )); // FT1-11
        for _ in 12..=19 {
            ft1.add_field(Field::from_value(""));
        }
        // FT1-20 performed by (XCN: id^name).
        if self.performed_by_id.is_some() || self.performed_by_name.is_some() {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(
                self.performed_by_id.as_deref().unwrap_or(""),
            ));
            rep.add_component(Component::from_value(
                self.performed_by_name.as_deref().unwrap_or(""),
            ));
            field.add_repetition(rep);
            ft1.add_field(field);
        } else {
            ft1.add_field(Field::from_value(""));
        }
        self.base.message.add_segment(ft1);

        Ok(self.base.build())
    }
}
