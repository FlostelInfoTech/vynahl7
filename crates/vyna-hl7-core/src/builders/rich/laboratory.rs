//! The clinical-order message family: `OML^O21` (laboratory), `OMG^O19`
//! (general) and `OMI^O23` (imaging).
//!
//! All three replace `ORM^O01`, deprecated since HL7 v2.4. They share MSH, PID,
//! PV1, ORC, TQ1 and OBR and differ only in what follows: OML adds the specimen
//! group, OMI the imaging procedure control, OMG neither.
//!
//! The shared segments are emitted by free functions rather than copied into
//! three builders. Three transcriptions of the same ORC would drift, and this
//! codebase has the scar: seven medication-status predicates across five crates
//! disagreed about the same question.

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

/// Specimen information emitted on the optional SPM segment.
#[derive(Debug, Clone, Default)]
pub struct SpecimenInfo {
    /// SPM-4 specimen type code (e.g., "BLD", "UR").
    pub specimen_type: String,
    /// SPM-7 specimen collection method.
    pub collection_method: Option<String>,
    /// SPM-8 specimen source site.
    pub source_site: Option<String>,
}

/// SPM — one specimen, positionally through SPM-8.
///
/// Shared by every builder that sends a SPECIMEN group, so OML^O21 and ORU^R01
/// cannot come to disagree about where the specimen type goes.
pub(crate) fn spm_segment(set_id: usize, spec: &SpecimenInfo) -> Segment {
    let mut spm = Segment::new("SPM");
    spm.add_field(Field::from_value(set_id.to_string())); // SPM-1
    spm.add_field(Field::from_value("")); // SPM-2
    spm.add_field(Field::from_value("")); // SPM-3
    spm.add_field(Field::from_value(&spec.specimen_type)); // SPM-4
    spm.add_field(Field::from_value("")); // SPM-5
    spm.add_field(Field::from_value("")); // SPM-6
    spm.add_field(Field::from_value(
        spec.collection_method.as_deref().unwrap_or(""),
    )); // SPM-7
    spm.add_field(Field::from_value(spec.source_site.as_deref().unwrap_or(""))); // SPM-8
    spm
}

/// Lab order payload for OML^O21.
#[derive(Debug, Clone, Default)]
pub struct LabOrder {
    /// ORC-2 / OBR-2 placer order number.
    pub order_number: String,
    /// ORC-5 order status.
    pub order_status: String,
    /// ORC-9 date/time of transaction.
    pub transaction_datetime: String,
    /// OBR-4 universal service identifier, emitted as the CWE `code^name^system`.
    pub service_code: String,
    pub service_name: String,
    /// OBR-4 third component — `LN` for LOINC, `C4` for CPT, `L` for a local
    /// compendium. Without it a receiver cannot tell a LOINC code from a local
    /// code that happens to look like one.
    pub service_coding_system: Option<String>,
    /// OBR-5 priority.
    pub priority: String,
    /// OBR-7 observation date/time (scheduled).
    pub scheduled_datetime: Option<String>,
    /// ORC-12 ordering provider as an XCN: `(id, family, given)`.
    ///
    /// Structured, NOT a pre-joined `id^family^given` string. `Field::from_value`
    /// escapes `^` to `\S\`, so a joined string arrives as
    /// `1234567890\S\Tan\S\Wei Ming` — one unparseable component where three
    /// were meant.
    pub ordering_provider: Option<(String, String, String)>,
    /// TQ1-3 repeat pattern (frequency code).
    pub frequency_code: Option<String>,
    /// TQ1-7 start date/time.
    pub start_datetime: Option<String>,
    /// ORC-16 order control code reason — why an order was cancelled or
    /// changed. Without it a `CA` says an order was withdrawn and nothing
    /// about why, which is the whole of what a receiving lab needs to know.
    pub order_control_reason: Option<String>,
}

/// ORC for a clinical order, positional through ORC-12.
///
/// `order_control` is HL7 Table 0119: `NW` new, `CA` cancel, `XO` change. It
/// used to be hardcoded `NW`, which meant a cancellation and a new order left
/// this builder looking identical.
fn emit_orc_clinical(o: &LabOrder, order_control: &str) -> Segment {
    let mut orc = Segment::new("ORC");
    orc.add_field(Field::from_value(order_control)); // ORC-1
    orc.add_field(Field::from_value(&o.order_number)); // ORC-2
    orc.add_field(Field::from_value("")); // ORC-3
    orc.add_field(Field::from_value("")); // ORC-4
    orc.add_field(Field::from_value(&o.order_status)); // ORC-5
    for _ in 6..=8 {
        orc.add_field(Field::from_value(""));
    }
    orc.add_field(Field::from_value(&o.transaction_datetime)); // ORC-9
    orc.add_field(Field::from_value("")); // ORC-10
    orc.add_field(Field::from_value("")); // ORC-11
    // ORC-12 as an XCN, built from components. Joining them into one string
    // would have `^` escaped by the field writer.
    let mut orc12 = Field::new();
    if let Some((id, family, given)) = o.ordering_provider.as_ref() {
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(id));
        rep.add_component(Component::from_value(family));
        rep.add_component(Component::from_value(given));
        orc12.add_repetition(rep);
    }
    orc.add_field(orc12); // ORC-12
    for _ in 13..=15 {
        orc.add_field(Field::from_value(""));
    }
    orc.add_field(Field::from_value(
        o.order_control_reason.as_deref().unwrap_or(""),
    )); // ORC-16
    orc
}

/// TQ1 timing/quantity, positional through TQ1-9.
fn emit_tq1(o: &LabOrder) -> Segment {
    let mut tq1 = Segment::new("TQ1");
    tq1.add_field(Field::from_value("1")); // TQ1-1
    tq1.add_field(Field::from_value("")); // TQ1-2
    tq1.add_field(Field::from_value(o.frequency_code.as_deref().unwrap_or(""))); // TQ1-3
    tq1.add_field(Field::from_value("")); // TQ1-4
    tq1.add_field(Field::from_value("")); // TQ1-5
    tq1.add_field(Field::from_value("")); // TQ1-6
    tq1.add_field(Field::from_value(o.start_datetime.as_deref().unwrap_or(""))); // TQ1-7
    tq1.add_field(Field::from_value("")); // TQ1-8
    tq1.add_field(Field::from_value(&o.priority)); // TQ1-9
    tq1
}

/// OBR, positional through OBR-7.
///
/// # OBR-4 is `code^name^system`, and used to be backwards
///
/// It emitted `name^code`, with a comment claiming that matched "the ORM
/// builder's non-standard-but-consistent ordering". It did not: `OrmO01Builder`
/// emits `code^name^system`, the correct CWE order. The comment asserted the
/// opposite of what both builders did, and the claim of consistency was the
/// only thing holding the wrong order in place.
///
/// Corrected here because it costs nothing: `OmlO21Builder` had no production
/// caller when this was fixed.
fn emit_obr(o: &LabOrder) -> Segment {
    let mut obr = Segment::new("OBR");
    obr.add_field(Field::from_value("1")); // OBR-1
    obr.add_field(Field::from_value(&o.order_number)); // OBR-2
    obr.add_field(Field::from_value("")); // OBR-3

    let mut obr4 = Field::new();
    let mut rep = Repetition::new();
    if o.service_code.is_empty() {
        // No catalogue code: a FLAT OBR-4 carrying the name alone. Emitting
        // `^Complete Blood Count` would claim an empty identifier rather than
        // no identifier, and a receiver parsing component 1 would read "".
        rep.add_component(Component::from_value(&o.service_name));
    } else {
        rep.add_component(Component::from_value(&o.service_code));
        rep.add_component(Component::from_value(&o.service_name));
        if let Some(system) = o.service_coding_system.as_deref() {
            rep.add_component(Component::from_value(system));
        }
    }
    obr4.add_repetition(rep);
    obr.add_field(obr4);

    obr.add_field(Field::from_value(&o.priority)); // OBR-5
    obr.add_field(Field::from_value("")); // OBR-6
    obr.add_field(Field::from_value(
        o.scheduled_datetime.as_deref().unwrap_or(""),
    )); // OBR-7
    obr
}

pub struct OmlO21Builder {
    base: MessageBuilder,
    order_control: String,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    order: LabOrder,
    specimen: Option<SpecimenInfo>,
}

impl OmlO21Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "OML", "O21");
        base.message_structure("OML_O21");
        Self {
            base,
            order_control: "NW".to_string(),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            order: LabOrder::default(),
            specimen: None,
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

    // ── PV1 ────────────────────────────────────────────────────────

    /// PV1-2 patient class. Defaults to "N" (not-applicable — typical
    /// for pre-admission lab orders) when unset.
    pub fn patient_class(mut self, class: &str) -> Self {
        self.visit.patient_class = Some(class.to_string());
        self
    }

    /// Supply the whole visit at once — see `VisitInfo::to_pv1_segment`.
    pub fn visit(mut self, visit: VisitInfo) -> Self {
        self.visit = visit;
        self
    }

    // ── Order ──────────────────────────────────────────────────────

    /// ORC-1, HL7 Table 0119. `NW` new, `CA` cancel, `XO` change.
    ///
    /// A cancellation is the same message with a different control code, not a
    /// different message — which is why retiring `ORM^O01` did not need
    /// separate cancel and status-change builders.
    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }

    pub fn order(mut self, order: LabOrder) -> Self {
        self.order = order;
        self
    }
    pub fn specimen(mut self, specimen: SpecimenInfo) -> Self {
        self.specimen = Some(specimen);
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

        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());

        // PV1 through the shared builder. This emitted TWO fields — set id
        // and a patient class defaulted to `N` — so a lab order or result
        // described no visit at all, and asserted "not applicable" about a
        // visit it had simply not been given.
        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }

        let o = &self.order;
        self.base
            .message
            .add_segment(emit_orc_clinical(o, &self.order_control));
        self.base.message.add_segment(emit_tq1(o));
        self.base.message.add_segment(emit_obr(o));

        // SPM — the specimen group is what makes OML the LABORATORY order and
        // not a general one. It is why a lab order cannot simply be sent as
        // OMG: one order can name several physical tubes, which ORM^O01 and
        // OMG^O19 have no structure to express.
        if let Some(spec) = &self.specimen {
            self.base.message.add_segment(spm_segment(1, spec));
        }

        Ok(self.base.build())
    }
}

// ---------------------------------------------------------------------------
// OMG^O19 — General Clinical Order
// ---------------------------------------------------------------------------

/// The general clinical order: everything that is neither laboratory nor
/// imaging — a procedure, a consult, a diet, a surgical booking.
///
/// It is deliberately the DEFAULT in the outbound routing table. Enumerating
/// which order types get OMG would mean a newly-added type falls off the table
/// into silence; defaulting means it gets a valid general order instead.
///
/// Its acknowledgment is `ORG^O20`.
pub struct OmgO19Builder {
    base: MessageBuilder,
    order_control: String,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    order: LabOrder,
}

impl OmgO19Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "OMG", "O19");
        base.message_structure("OMG_O19");
        Self {
            base,
            order_control: "NW".to_string(),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            order: LabOrder::default(),
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
    pub fn demographics(mut self, d: PatientDemographics) -> Self {
        self.demographics = d;
        self
    }
    pub fn visit(mut self, v: VisitInfo) -> Self {
        self.visit = v;
        self
    }
    /// ORC-1, HL7 Table 0119.
    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }
    pub fn order(mut self, order: LabOrder) -> Self {
        self.order = order;
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
        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());
        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }
        self.base
            .message
            .add_segment(emit_orc_clinical(&self.order, &self.order_control));
        self.base.message.add_segment(emit_tq1(&self.order));
        self.base.message.add_segment(emit_obr(&self.order));
        // No specimen group and no IPC: a general order has neither a tube nor
        // a study. Emitting an empty one would assert we had something to say.
        Ok(self.base.build())
    }
}

// ---------------------------------------------------------------------------
// OMI^O23 — Imaging Order
// ---------------------------------------------------------------------------

/// The imaging order. Its acknowledgment is `ORI^O24`.
///
/// Carries `IPC`, the imaging procedure control, which is what distinguishes it
/// from a general order the way the specimen group distinguishes `OML`.
pub struct OmiO23Builder {
    base: MessageBuilder,
    order_control: String,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    order: LabOrder,
    modality: Option<String>,
}

impl OmiO23Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "OMI", "O23");
        base.message_structure("OMI_O23");
        Self {
            base,
            order_control: "NW".to_string(),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            order: LabOrder::default(),
            modality: None,
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
    pub fn demographics(mut self, d: PatientDemographics) -> Self {
        self.demographics = d;
        self
    }
    pub fn visit(mut self, v: VisitInfo) -> Self {
        self.visit = v;
        self
    }
    /// ORC-1, HL7 Table 0119.
    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }
    pub fn order(mut self, order: LabOrder) -> Self {
        self.order = order;
        self
    }
    /// IPC-5 modality (`CT`, `MR`, `US`, `DX`), when the order names one.
    pub fn modality(mut self, modality: &str) -> Self {
        self.modality = Some(modality.to_string());
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
        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());
        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }
        self.base
            .message
            .add_segment(emit_orc_clinical(&self.order, &self.order_control));
        self.base.message.add_segment(emit_tq1(&self.order));
        self.base.message.add_segment(emit_obr(&self.order));

        // IPC — imaging procedure control.
        //
        // IPC-1 accession and IPC-3 study instance UID are assigned by the
        // RIS/PACS, not by the placer, so they go out EMPTY rather than filled
        // with our order number dressed up as an accession. A fabricated
        // accession is worse than an absent one: it looks authoritative and
        // matches nothing on the receiving side.
        let mut ipc = Segment::new("IPC");
        ipc.add_field(Field::from_value("")); // IPC-1 accession identifier
        ipc.add_field(Field::from_value("")); // IPC-2 requested procedure ID
        ipc.add_field(Field::from_value("")); // IPC-3 study instance UID
        ipc.add_field(Field::from_value(&self.order.order_number)); // IPC-4 scheduled procedure step ID
        ipc.add_field(Field::from_value(self.modality.as_deref().unwrap_or(""))); // IPC-5 modality
        self.base.message.add_segment(ipc);

        Ok(self.base.build())
    }
}
