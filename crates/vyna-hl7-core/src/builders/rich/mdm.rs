//! MDM (Medical Document Management) message builders.
//!
//! One canonical builder — `MdmT02Builder` — carries the MSH / EVN /
//! PID / PV1 / TXA / OBX fields our outbound documentation flows
//! populate. Every MDM variant (T01 original-without-content, T04
//! status change, T06 addendum, T10 replacement, T11 cancel …) shares
//! the MDM_T02 message structure and is expressed via
//! [`MdmT02Builder::trigger_event`].

use super::VisitInfo;
use super::{
    DocumentHeader, MessageBuilder, PatientAddress, PatientDemographics, PatientIdentifier,
    generate_control_id,
};
use crate::{
    Version,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
};

pub struct MdmT02Builder {
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
    // TXA
    document: DocumentHeader,
    // OBX — optional document body
    document_content: Option<String>,
}

impl MdmT02Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "MDM", "T02");
        base.message_structure("MDM_T02");
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
            document: DocumentHeader::default(),
            document_content: None,
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

    /// Override trigger event (T01 through T11 — see
    /// [`crate::message::trigger_events`] for the full table). MSH-9
    /// second component only; the third component stays MDM_T02 which
    /// all MDM variants share per HL7 v2.5.
    pub fn trigger_event(mut self, trigger: &str) -> Self {
        self.base.set_trigger_event(trigger);
        self
    }

    // ── EVN ────────────────────────────────────────────────────────

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

    // ── TXA ────────────────────────────────────────────────────────

    /// Set the whole TXA payload at once.
    ///
    /// The same reasoning as [`Self::demographics`]: a caller assembles the
    /// document header once from what it holds, rather than the builder
    /// offering a setter per field and each call site choosing a different
    /// subset. Six of TXA's 23 fields had setters; the other seventeen
    /// could not be sent at all.
    pub fn document(mut self, document: DocumentHeader) -> Self {
        self.document = document;
        self
    }
    /// TXA-2 document type (HP, CN, DS, etc.).
    pub fn document_type(mut self, doc_type: &str) -> Self {
        self.document.document_type = Some(doc_type.to_string());
        self
    }
    /// TXA-4 activity date/time (when the document was created or
    /// status-changed).
    pub fn activity_datetime(mut self, datetime: &str) -> Self {
        self.document.activity_datetime = Some(datetime.to_string());
        self
    }
    /// TXA-5 primary activity provider (XCN: id^family^given).
    pub fn originator(mut self, id: &str, name: &str) -> Self {
        self.document.primary_activity_provider = Some(format!("{id}^{name}"));
        self
    }
    /// TXA-12 unique document number.
    pub fn unique_document_number(mut self, number: &str) -> Self {
        self.document.unique_document_number = Some(number.to_string());
        self
    }
    /// TXA-17 document completion status (AU authenticated, IP in
    /// progress, etc.).
    pub fn completion_status(mut self, status: &str) -> Self {
        self.document.completion_status = Some(status.to_string());
        self
    }

    // ── OBX (document body) ────────────────────────────────────────

    /// Optional TX-typed OBX carrying the free-text document content.
    pub fn document_content(mut self, content: &str) -> Self {
        self.document_content = Some(content.to_string());
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
            self.base.trigger_event_str().to_string().as_str(),
            self.event_datetime.as_deref(),
        )?;
        self.base.message.add_segment(evn);

        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());

        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }

        self.base
            .message
            .add_segment(self.document.to_txa_segment());

        // Optional OBX carrying the document body as TX.
        if let Some(content) = &self.document_content {
            let mut obx = Segment::new("OBX");
            obx.add_field(Field::from_value("1")); // OBX-1
            obx.add_field(Field::from_value("TX")); // OBX-2
            // OBX-3 observation identifier (CWE: DOCUMENT^Document Content).
            let mut field = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value("DOCUMENT"));
            rep.add_component(Component::from_value("Document Content"));
            field.add_repetition(rep);
            obx.add_field(field);
            obx.add_field(Field::from_value("")); // OBX-4 sub-ID
            obx.add_field(Field::from_value(content)); // OBX-5
            for _ in 6..=10 {
                obx.add_field(Field::from_value(""));
            }
            obx.add_field(Field::from_value("F")); // OBX-11
            self.base.message.add_segment(obx);
        }

        Ok(self.base.build())
    }
}
