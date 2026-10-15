//! REF (Patient Referral) message builders.

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

/// Referral payload for the RF1 segment.
#[derive(Debug, Clone, Default)]
pub struct ReferralInfo {
    /// RF1-1 referral status.
    pub status: String,
    /// RF1-2 priority (routine / urgent / stat).
    pub priority: String,
    /// RF1-3 referral type (consultation / transfer / …).
    pub referral_type: String,
    /// RF1-6 originating referral identifier.
    pub referral_id: String,
    /// RF1-10 referral reason (free-text).
    pub reason: String,
}

/// Provider identity for a PRD segment.
#[derive(Debug, Clone, Default)]
pub struct ReferralProvider {
    /// PRD-2 provider name.
    pub name: Option<String>,
    /// PRD-7 provider identifier.
    pub id: Option<String>,
}

/// Diagnosis payload for the optional DG1 segment.
#[derive(Debug, Clone, Default)]
pub struct Diagnosis {
    /// DG1-3 diagnosis code (first component).
    pub code: Option<String>,
    /// DG1-3 diagnosis description (second component) — also surfaces
    /// as a fallback when `code` is None so partners that only read
    /// the description still get something.
    pub description: Option<String>,
    /// DG1-2 / DG1-3.3 coding method (defaults to "I10").
    pub coding_method: Option<String>,
    /// DG1-6 diagnosis type. Defaults to "A" (admitting).
    pub diagnosis_type: Option<String>,
    /// DG1-5 diagnosis date/time.
    pub datetime: Option<String>,
}

pub struct RefI12Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    referral: ReferralInfo,
    referring_provider: Option<ReferralProvider>,
    referred_to_provider: Option<ReferralProvider>,
    diagnosis: Option<Diagnosis>,
}

impl RefI12Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "REF", "I12");
        base.message_structure("REF_I12");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            referral: ReferralInfo::default(),
            referring_provider: None,
            referred_to_provider: None,
            diagnosis: None,
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

    // ── Referral content ───────────────────────────────────────────

    pub fn referral(mut self, info: ReferralInfo) -> Self {
        self.referral = info;
        self
    }
    pub fn referring_provider(mut self, provider: ReferralProvider) -> Self {
        self.referring_provider = Some(provider);
        self
    }
    pub fn referred_to_provider(mut self, provider: ReferralProvider) -> Self {
        self.referred_to_provider = Some(provider);
        self
    }
    pub fn diagnosis(mut self, diagnosis: Diagnosis) -> Self {
        self.diagnosis = Some(diagnosis);
        self
    }

    /// Switch this builder to emit `RRI^I12^RRI_I12` (Return Referral
    /// Information) instead of `REF^I12^REF_I12`. Per HL7 v2.5 Ch 11,
    /// RRI^I12 is the response trigger for `REF^I15` (Request for
    /// Patient Referral) and shares REF^I12's segment grammar
    /// (PID/RF1/PRD/DG1) — only MSH-9 differs. Calling this lets us
    /// reuse all of REF^I12's segment-building logic for the response
    /// message rather than maintaining a parallel builder.
    pub fn as_rri_response(mut self) -> Self {
        self.base.set_message_type("RRI");
        self.base.message_structure("RRI_I12");
        self
    }

    /// Override the trigger event (I12 referral / I13 modify / I14 cancel).
    ///
    /// MSH-9 second component only; the third stays `REF_I12` because all four
    /// referral trigger events share that abstract message structure (HL7 v2.5
    /// Ch 11). The receiver distinguishes them by MSH-9 together with the RF1
    /// status and identifiers, which is why one builder can serve all three.
    ///
    /// Same shape as `SiuS12Builder::trigger_event`, for the same reason.
    pub fn trigger_event(mut self, trigger: &str) -> Self {
        self.base.set_trigger_event(trigger);
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

        // RF1 positional through RF1-10.
        let r = &self.referral;
        let mut rf1 = Segment::new("RF1");
        rf1.add_field(Field::from_value(&r.status)); // RF1-1
        rf1.add_field(Field::from_value(&r.priority)); // RF1-2
        rf1.add_field(Field::from_value(&r.referral_type)); // RF1-3
        rf1.add_field(Field::from_value("")); // RF1-4
        rf1.add_field(Field::from_value("")); // RF1-5
        rf1.add_field(Field::from_value(&r.referral_id)); // RF1-6
        for _ in 7..=9 {
            rf1.add_field(Field::from_value(""));
        }
        rf1.add_field(Field::from_value(&r.reason)); // RF1-10
        self.base.message.add_segment(rf1);

        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());

        // PRD segments — referring (RP) then referred-to (RT).
        if let Some(prov) = &self.referring_provider {
            self.base.message.add_segment(emit_prd(prov, "RP"));
        }
        if let Some(prov) = &self.referred_to_provider {
            self.base.message.add_segment(emit_prd(prov, "RT"));
        }

        // DG1 — emitted only when a diagnosis is attached.
        if let Some(dx) = &self.diagnosis {
            self.base.message.add_segment(emit_dg1(dx));
        }

        Ok(self.base.build())
    }
}

fn emit_prd(provider: &ReferralProvider, role: &str) -> Segment {
    let mut prd = Segment::new("PRD");
    prd.add_field(Field::from_value(role)); // PRD-1 provider role
    prd.add_field(Field::from_value(provider.name.as_deref().unwrap_or(""))); // PRD-2
    for _ in 3..=6 {
        prd.add_field(Field::from_value(""));
    }
    prd.add_field(Field::from_value(provider.id.as_deref().unwrap_or(""))); // PRD-7
    prd
}

/// Render a DG1 segment (exposed so the BAR builder can share the
/// same layout).
pub fn emit_dg1(diagnosis: &Diagnosis) -> Segment {
    let coding_method = diagnosis.coding_method.as_deref().unwrap_or("I10");
    let mut dg1 = Segment::new("DG1");
    dg1.add_field(Field::from_value("1")); // DG1-1 set ID
    dg1.add_field(Field::from_value(coding_method)); // DG1-2
    // DG1-3 diagnosis code (CWE: code^description^coding_method).
    let mut dg1_3 = Field::new();
    let mut rep = Repetition::new();
    rep.add_component(Component::from_value(
        diagnosis.code.as_deref().unwrap_or(""),
    ));
    rep.add_component(Component::from_value(
        diagnosis.description.as_deref().unwrap_or(""),
    ));
    rep.add_component(Component::from_value(coding_method));
    dg1_3.add_repetition(rep);
    dg1.add_field(dg1_3);
    dg1.add_field(Field::from_value("")); // DG1-4 description (duplicated in DG1-3.2)
    dg1.add_field(Field::from_value(
        diagnosis.datetime.as_deref().unwrap_or(""),
    )); // DG1-5
    dg1.add_field(Field::from_value(
        diagnosis.diagnosis_type.as_deref().unwrap_or("A"),
    )); // DG1-6
    dg1
}
