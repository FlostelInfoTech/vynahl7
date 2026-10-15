//! Pharmacy / immunization message builders (RDE, RDS, VXU).

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

// ---------------------------------------------------------------------------
// Shared prescription payload
// ---------------------------------------------------------------------------

/// Prescription / pharmacy order as emitted on RXE / RXR.
#[derive(Debug, Clone, Default)]
pub struct PharmacyPrescription {
    /// Placer order number (ORC-2).
    pub order_id: String,
    /// RXE-2 give code (CWE: code^name^coding_system).
    pub medication_code: Option<String>,
    pub medication_name: String,
    /// Coding system for RXE-2 third component (RxNorm etc.).
    pub coding_system: String,
    /// RXE-3 give amount (minimum).
    pub dose: String,
    /// RXE-5 give units.
    pub dose_unit: String,
    /// RXR-1 route.
    pub route: String,
    /// RXE-22 give per (time unit) — used here as human-readable frequency.
    pub frequency: String,
    /// RXE-10 dispense amount.
    pub dispense_amount: Option<String>,
    /// RXE-12 number of refills.
    pub refills: Option<i32>,
    /// RXE-7 provider's administration instructions.
    pub pharmacy_instructions: Option<String>,
    /// ORC-12 prescriber as an XCN: `(id, family, given)`.
    ///
    /// Structured, NOT a pre-joined string: `Field::from_value` escapes `^` to
    /// `\S\`, so `1234567890^Tan^Wei Ming` went out as
    /// `1234567890\S\Tan\S\Wei Ming` — one unparseable component where three
    /// were meant. Inherited from the RDE builder and carried into OMP before
    /// a laboratory test caught the same shape.
    pub prescriber: Option<(String, String, String)>,
    /// ORC-5 order status (HL7 Table 0038), when the message exists BECAUSE the
    /// status changed — `DC` discontinued, `CA` cancelled. Empty for a new
    /// order, whose status is implied by ORC-1 = `NW`.
    pub order_status: Option<String>,
    /// ORC-16 order control code reason — why the order was discontinued or
    /// cancelled. Empty for a new order: there is no reason for a thing that is
    /// simply being requested.
    pub order_control_reason: Option<String>,
}

/// A CE field carrying TEXT and no code: identifier empty, text in
/// component 2.
///
/// The alternative — putting the text in component 1 — says the display name
/// IS the coded identifier, so a receiver looking up the code finds a name
/// that is in no table, and a receiver reading the text finds nothing. `None`
/// and the empty string both produce an empty field.
fn ce_text_only(text: Option<&str>) -> Field {
    let text = text.unwrap_or("");
    if text.is_empty() {
        return Field::from_value("");
    }
    let mut field = Field::new();
    let mut rep = Repetition::new();
    rep.add_component(Component::from_value(""));
    rep.add_component(Component::from_value(text));
    field.add_repetition(rep);
    field
}

fn emit_rxe(prescription: &PharmacyPrescription) -> Segment {
    let mut rxe = Segment::new("RXE");
    rxe.add_field(Field::from_value("")); // RXE-1 quantity/timing (deprecated)
    // RXE-2 give code (CWE).
    let mut rxe2 = Field::new();
    let mut rep = Repetition::new();
    rep.add_component(Component::from_value(
        prescription.medication_code.as_deref().unwrap_or(""),
    ));
    rep.add_component(Component::from_value(&prescription.medication_name));
    rep.add_component(Component::from_value(&prescription.coding_system));
    rxe2.add_repetition(rep);
    rxe.add_field(rxe2);
    rxe.add_field(Field::from_value(&prescription.dose)); // RXE-3
    rxe.add_field(Field::from_value("")); // RXE-4
    rxe.add_field(Field::from_value(&prescription.dose_unit)); // RXE-5
    rxe.add_field(Field::from_value("")); // RXE-6
    rxe.add_field(Field::from_value(
        prescription.pharmacy_instructions.as_deref().unwrap_or(""),
    )); // RXE-7
    rxe.add_field(Field::from_value("")); // RXE-8
    rxe.add_field(Field::from_value("")); // RXE-9
    rxe.add_field(Field::from_value(
        prescription.dispense_amount.as_deref().unwrap_or(""),
    )); // RXE-10
    rxe.add_field(Field::from_value("")); // RXE-11
    rxe.add_field(Field::from_value(
        prescription
            .refills
            .map(|r| r.to_string())
            .unwrap_or_default(),
    )); // RXE-12
    for _ in 13..=21 {
        rxe.add_field(Field::from_value(""));
    }
    rxe.add_field(Field::from_value(&prescription.frequency)); // RXE-22
    rxe
}

/// RXO — Pharmacy/Treatment ORDER.
///
/// The placer's segment: what the PRESCRIBER asked for. Deliberately distinct
/// from [`emit_rxe`], which is the FILLER's — what pharmacy encoded after
/// reviewing it. HL7 v2.7.1 ch.4 assigns them to opposite ends of the exchange,
/// and emitting RXE from an OMP would assert that the placer had already done
/// the pharmacy's encoding.
///
/// Field numbering differs from RXE and is not a rename: RXO-2/3 are the
/// requested give amount MIN/MAX where RXE-3/4 are give amount min/max, and the
/// dispense fields sit at RXO-10/11 against RXE-10/11. They are transcribed
/// explicitly rather than shared with `emit_rxe`, because a shared writer would
/// have to know which segment it was in — and that is exactly the confusion the
/// two segments exist to prevent.
fn emit_rxo(prescription: &PharmacyPrescription) -> Segment {
    let mut rxo = Segment::new("RXO");

    // RXO-1 requested give code (CWE: code^name^coding_system).
    let mut rxo1 = Field::new();
    let mut rep = Repetition::new();
    rep.add_component(Component::from_value(
        prescription.medication_code.as_deref().unwrap_or(""),
    ));
    rep.add_component(Component::from_value(&prescription.medication_name));
    rep.add_component(Component::from_value(&prescription.coding_system));
    rxo1.add_repetition(rep);
    rxo.add_field(rxo1);

    rxo.add_field(Field::from_value(&prescription.dose)); // RXO-2 give amount min
    rxo.add_field(Field::from_value("")); // RXO-3 give amount max
    rxo.add_field(Field::from_value(&prescription.dose_unit)); // RXO-4 give units
    rxo.add_field(Field::from_value("")); // RXO-5 requested dosage form
    rxo.add_field(Field::from_value("")); // RXO-6 pharmacy/treatment instructions
    rxo.add_field(Field::from_value(
        prescription.pharmacy_instructions.as_deref().unwrap_or(""),
    )); // RXO-7 provider's administration instructions
    rxo.add_field(Field::from_value("")); // RXO-8 deliver-to location
    rxo.add_field(Field::from_value("")); // RXO-9 allow substitutions
    rxo.add_field(Field::from_value("")); // RXO-10 requested dispense code
    rxo.add_field(Field::from_value(
        prescription.dispense_amount.as_deref().unwrap_or(""),
    )); // RXO-11 requested dispense amount
    rxo.add_field(Field::from_value("")); // RXO-12 requested dispense units
    rxo.add_field(Field::from_value(
        prescription
            .refills
            .map(|r| r.to_string())
            .unwrap_or_default(),
    )); // RXO-13 number of refills

    rxo
}

/// ORC for a pharmacy order, positional through ORC-16.
///
/// Emitted through 16 unconditionally, exactly as `emit_orc` does for the
/// laboratory builders. A segment whose LENGTH depends on whether a reason is
/// present gives a positional indexer two shapes for one message type, which is
/// a worse failure than the trailing empty fields HL7 explicitly permits.
///
/// ORC-5 and ORC-16 exist for the DISCONTINUE case (`ORC-1 = DC`): a filler
/// that is told to stop dispensing needs the new status and the reason. Before
/// they were added the segment stopped at ORC-12, so a discontinuation could
/// carry neither.
fn emit_orc_pharmacy(prescription: &PharmacyPrescription, order_control: &str) -> Segment {
    let mut orc = Segment::new("ORC");
    orc.add_field(Field::from_value(order_control)); // ORC-1
    orc.add_field(Field::from_value(&prescription.order_id)); // ORC-2
    orc.add_field(Field::from_value("")); // ORC-3
    orc.add_field(Field::from_value("")); // ORC-4
    orc.add_field(Field::from_value(
        prescription.order_status.as_deref().unwrap_or(""),
    )); // ORC-5
    for _ in 6..=11 {
        orc.add_field(Field::from_value(""));
    }
    // ORC-12 as an XCN, built from components — see `PharmacyPrescription`.
    let mut orc12 = Field::new();
    if let Some((id, family, given)) = prescription.prescriber.as_ref() {
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
        prescription.order_control_reason.as_deref().unwrap_or(""),
    )); // ORC-16
    orc
}

// ---------------------------------------------------------------------------
// OMP^O09 — Pharmacy/Treatment Order
//
// THE PLACER'S message: the EMR asking pharmacy for a medication. Its
// acknowledgment is ORP^O10.
//
// Not to be confused with RDE^O11 below, which is the FILLER's reply — pharmacy
// telling the EMR what it encoded. An EMR that sends RDE is claiming to have
// done the pharmacy's own work; VynaHealth has a pharmacy module built in, but
// that filler role is internal and never crosses a wire.
// ---------------------------------------------------------------------------

pub struct OmpO09Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    order_control: String,
    prescription: PharmacyPrescription,
}

impl OmpO09Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "OMP", "O09");
        base.message_structure("OMP_O09");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            order_control: "NW".to_string(),
            prescription: PharmacyPrescription::default(),
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
    /// ORC-1, HL7 Table 0119. `NW` new, `CA` cancel, `XO` change.
    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }
    pub fn prescription(mut self, p: PharmacyPrescription) -> Self {
        self.prescription = p;
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
            .add_segment(emit_orc_pharmacy(&self.prescription, &self.order_control));
        // RXO, not RXE — see `emit_rxo`.
        self.base.message.add_segment(emit_rxo(&self.prescription));

        // RXR route.
        let mut rxr = Segment::new("RXR");
        rxr.add_field(Field::from_value(&self.prescription.route));
        self.base.message.add_segment(rxr);

        Ok(self.base.build())
    }
}

// ---------------------------------------------------------------------------
// RDE^O11 — Pharmacy/Treatment Encoded Order
// ---------------------------------------------------------------------------

pub struct RdeO11Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    order_control: String,
    prescription: PharmacyPrescription,
}

impl RdeO11Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "RDE", "O11");
        base.message_structure("RDE_O11");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            order_control: "NW".to_string(),
            prescription: PharmacyPrescription::default(),
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

    /// Supply the whole visit at once — see `VisitInfo::to_pv1_segment`.
    pub fn visit(mut self, visit: VisitInfo) -> Self {
        self.visit = visit;
        self
    }

    // ── ORC / RXE ──────────────────────────────────────────────────

    /// ORC-1 order control. Defaults to "NW" (new order).
    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }
    pub fn prescription(mut self, prescription: PharmacyPrescription) -> Self {
        self.prescription = prescription;
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
            .add_segment(emit_orc_pharmacy(&self.prescription, &self.order_control));
        self.base.message.add_segment(emit_rxe(&self.prescription));

        // RXR route.
        let mut rxr = Segment::new("RXR");
        rxr.add_field(Field::from_value(&self.prescription.route));
        self.base.message.add_segment(rxr);

        Ok(self.base.build())
    }
}

// ---------------------------------------------------------------------------
// RDS^O13 — Pharmacy/Treatment Dispense
// ---------------------------------------------------------------------------

/// Dispense event emitted on RXD by [`RdsO13Builder`].
#[derive(Debug, Clone, Default)]
pub struct DispenseEvent {
    /// Order the dispense is against (ORC-2).
    pub order_id: String,
    /// RXD-2 dispense/give code (CWE: code^name^coding_system).
    pub dispense_code: String,
    pub dispense_code_name: String,
    pub coding_system: String,
    /// RXD-3 date/time dispensed.
    pub dispense_date: String,
    /// RXD-4 actual dispense amount.
    pub actual_dose: String,
    /// RXD-5 actual dispense units.
    pub actual_dose_unit: String,
    /// RXD-6 actual dosage form — free-text / empty.
    pub actual_strength: Option<String>,
    /// RXD-7 prescription number / filler order number — not used here.
    /// RXR-1 actual route.
    pub actual_route: String,
    /// RXD-8 prescribed quantity dispensed.
    pub quantity_dispensed: Option<String>,
    /// RXD-9 pharmacy-specific free-text notes.
    pub pharmacy_notes: Option<String>,
}

pub struct RdsO13Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    dispense: DispenseEvent,
}

impl RdsO13Builder {
    /// Set the whole PID payload at once.
    ///
    /// Lets a caller assemble demographics once and hand the same struct to
    /// every message type, rather than re-deriving the patient wiring per
    /// builder — which is how the gateway's call sites came to disagree
    /// about which PID fields get sent.
    pub fn demographics(mut self, demographics: PatientDemographics) -> Self {
        self.demographics = demographics;
        self
    }
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "RDS", "O13");
        base.message_structure("RDS_O13");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            dispense: DispenseEvent::default(),
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

    /// Supply the whole visit at once — see `VisitInfo::to_pv1_segment`.
    pub fn visit(mut self, visit: VisitInfo) -> Self {
        self.visit = visit;
        self
    }

    pub fn dispense(mut self, event: DispenseEvent) -> Self {
        self.dispense = event;
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

        let d = &self.dispense;

        // ORC — "RE" (observations/performed).
        let mut orc = Segment::new("ORC");
        orc.add_field(Field::from_value("RE"));
        orc.add_field(Field::from_value(&d.order_id));
        self.base.message.add_segment(orc);

        // RXD positional through RXD-9.
        let mut rxd = Segment::new("RXD");
        rxd.add_field(Field::from_value("1")); // RXD-1 dispense sub-ID
        // RXD-2 dispense/give code (CWE).
        let mut rxd2 = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&d.dispense_code));
        rep.add_component(Component::from_value(&d.dispense_code_name));
        rep.add_component(Component::from_value(&d.coding_system));
        rxd2.add_repetition(rep);
        rxd.add_field(rxd2);
        rxd.add_field(Field::from_value(&d.dispense_date)); // RXD-3
        rxd.add_field(Field::from_value(&d.actual_dose)); // RXD-4
        rxd.add_field(Field::from_value(&d.actual_dose_unit)); // RXD-5
        rxd.add_field(Field::from_value(
            d.actual_strength.as_deref().unwrap_or(""),
        )); // RXD-6
        rxd.add_field(Field::from_value("")); // RXD-7 prescription number
        rxd.add_field(Field::from_value(
            d.quantity_dispensed.as_deref().unwrap_or(""),
        )); // RXD-8
        rxd.add_field(Field::from_value(d.pharmacy_notes.as_deref().unwrap_or(""))); // RXD-9
        self.base.message.add_segment(rxd);

        // RXR — actual route.
        let mut rxr = Segment::new("RXR");
        rxr.add_field(Field::from_value(&d.actual_route));
        self.base.message.add_segment(rxr);

        Ok(self.base.build())
    }
}

// ---------------------------------------------------------------------------
// VXU^V04 — Immunization Registry Update
// ---------------------------------------------------------------------------

/// Immunization record used by [`VxuV04Builder`].
#[derive(Debug, Clone, Default)]
pub struct ImmunizationEvent {
    /// RXA-5 CVX code.
    pub vaccine_code: Option<String>,
    /// RXA-5 human-readable vaccine name.
    pub vaccine_name: String,
    /// RXA-3 / RXA-4 date/time of administration.
    pub administered_date: String,
    /// RXA-15 substance lot number.
    pub lot_number: Option<String>,
    /// RXA-16 substance expiration date (YYYYMMDD).
    pub expiration_date: Option<String>,
    /// RXA-17 manufacturer name. Written to the CE's TEXT component, not its
    /// identifier: RXA-17 is keyed by table MVX, and a caller holding only a
    /// name has no MVX code to put in the identifier. Naming the text as the
    /// code asserts a coded value that does not exist.
    pub manufacturer: Option<String>,
    /// RXR-1 route (IM, SC, PO, …).
    pub route: Option<String>,
    /// RXR-2 administration site (LA = left arm, etc.).
    pub site: Option<String>,
    /// OBX-5 dose-in-series number (LOINC 30973-2). NOT RXA-6, which is the
    /// administered AMOUNT in mL — the CDC guide is explicit that the dose
    /// number does not belong in the RXA segment.
    pub dose_number: Option<i32>,
    /// RXA-10 administering provider (free-text or XCN).
    pub administered_by: Option<String>,
    /// RXA-18 refusal reason, in the CE's text component. Only meaningful
    /// alongside a `completion_status` of `RE`.
    pub refusal_reason: Option<String>,
    /// RXA-20 completion status, HL7 table 0322: `CP` complete, `RE` refused,
    /// `NA` not administered, `PA` partially administered.
    ///
    /// `None` emits an empty field rather than defaulting to `CP`. A default
    /// of "complete" would have every caller that never considered the
    /// question announce a dose as given — including the refusals.
    pub completion_status: Option<String>,
}

/// Builder for VXU^V04 — Unsolicited Vaccination Update.
///
/// Structure: MSH · PID · ORC (RE) · RXA · RXR · OBX (dose-in-series).
pub struct VxuV04Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    immunization: ImmunizationEvent,
}

impl VxuV04Builder {
    /// Set the whole PID payload at once.
    ///
    /// Lets a caller assemble demographics once and hand the same struct to
    /// every message type, rather than re-deriving the patient wiring per
    /// builder — which is how the gateway's call sites came to disagree
    /// about which PID fields get sent.
    pub fn demographics(mut self, demographics: PatientDemographics) -> Self {
        self.demographics = demographics;
        self
    }
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "VXU", "V04");
        base.message_structure("VXU_V04");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            immunization: ImmunizationEvent::default(),
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

    /// Carries the immunization to emit on RXA / RXR / OBX.
    pub fn immunization(mut self, event: ImmunizationEvent) -> Self {
        self.immunization = event;
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

        // ORC — "RE" (observations/results to follow) is the canonical
        // order-control for registry updates.
        let mut orc = Segment::new("ORC");
        orc.add_field(Field::from_value("RE"));
        self.base.message.add_segment(orc);

        let imm = &self.immunization;

        // RXA positional through RXA-17.
        let mut rxa = Segment::new("RXA");
        rxa.add_field(Field::from_value("0")); // RXA-1
        rxa.add_field(Field::from_value("1")); // RXA-2
        rxa.add_field(Field::from_value(&imm.administered_date)); // RXA-3
        rxa.add_field(Field::from_value(&imm.administered_date)); // RXA-4
        // RXA-5 administered code (CVX: code^name^CVX).
        let mut rxa5 = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(
            imm.vaccine_code.as_deref().unwrap_or(""),
        ));
        rep.add_component(Component::from_value(&imm.vaccine_name));
        rep.add_component(Component::from_value("CVX"));
        rxa5.add_repetition(rep);
        rxa.add_field(rxa5);
        rxa.add_field(Field::from_value("999")); // RXA-6 amount (unknown)
        for _ in 7..=9 {
            rxa.add_field(Field::from_value(""));
        }
        rxa.add_field(Field::from_value(
            imm.administered_by.as_deref().unwrap_or(""),
        )); // RXA-10
        for _ in 11..=14 {
            rxa.add_field(Field::from_value(""));
        }
        rxa.add_field(Field::from_value(imm.lot_number.as_deref().unwrap_or(""))); // RXA-15
        rxa.add_field(Field::from_value(
            imm.expiration_date.as_deref().unwrap_or(""),
        )); // RXA-16
        // RXA-17 / RXA-18 are CE fields keyed by tables MVX and NIP002. We
        // hold text and no code, so the text goes in component 2 and the
        // identifier stays empty — the text-only shape of a CE.
        rxa.add_field(ce_text_only(imm.manufacturer.as_deref())); // RXA-17
        rxa.add_field(ce_text_only(imm.refusal_reason.as_deref())); // RXA-18
        rxa.add_field(Field::from_value("")); // RXA-19
        rxa.add_field(Field::from_value(
            imm.completion_status.as_deref().unwrap_or(""),
        )); // RXA-20
        self.base.message.add_segment(rxa);

        // RXR — route / site.
        let mut rxr = Segment::new("RXR");
        rxr.add_field(Field::from_value(imm.route.as_deref().unwrap_or(""))); // RXR-1
        rxr.add_field(Field::from_value(imm.site.as_deref().unwrap_or(""))); // RXR-2
        self.base.message.add_segment(rxr);

        // OBX — dose-in-series (LOINC 30973-2).
        let mut obx = Segment::new("OBX");
        obx.add_field(Field::from_value("1")); // OBX-1
        obx.add_field(Field::from_value("NM")); // OBX-2
        let mut obx3 = Field::new();
        let mut obx3_rep = Repetition::new();
        obx3_rep.add_component(Component::from_value("30973-2"));
        obx3_rep.add_component(Component::from_value("Dose number in series"));
        obx3_rep.add_component(Component::from_value("LN"));
        obx3.add_repetition(obx3_rep);
        obx.add_field(obx3);
        obx.add_field(Field::from_value("")); // OBX-4
        obx.add_field(Field::from_value(
            imm.dose_number.map(|d| d.to_string()).unwrap_or_default(),
        )); // OBX-5
        for _ in 6..=10 {
            obx.add_field(Field::from_value(""));
        }
        obx.add_field(Field::from_value("F")); // OBX-11
        self.base.message.add_segment(obx);

        Ok(self.base.build())
    }
}

// ---------------------------------------------------------------------------
// RAS^O17 — Pharmacy/Treatment Administration
//
// THE PLACER REPORTING WHAT WAS GIVEN. The EMR tells an external Pharmacy
// Information System that a dose was administered — or was NOT, which is why
// RXA-20 is load-bearing rather than a formality.
//
// Its acknowledgment is RRA^O18. Not to be confused with RDS^O13, the filler
// telling us what it DISPENSED: dispensing and administration are different
// events, and a PIS that hears only the first has no medication administration
// record at all.
//
// Sent only where the customer runs a PIS outside VynaHealth. A deployment
// using the built-in pharmacy subscribes to nothing here and the message is
// never built.
// ---------------------------------------------------------------------------

/// One administration event, for [`RasO17Builder`].
#[derive(Debug, Clone, Default)]
pub struct AdministrationEvent {
    /// ORC-2 / ORC-3 placer order number — the order this dose answers.
    pub order_id: String,
    /// RXA-5 administered code (CWE: `code^name^system`).
    pub medication_code: Option<String>,
    pub medication_name: String,
    /// RXA-5 third component. RxNorm where we hold one.
    pub coding_system: String,
    /// RXA-3 start of administration.
    pub start_datetime: String,
    /// RXA-4 end of administration. Same as the start for a bolus dose.
    pub end_datetime: String,
    /// RXA-6 administered amount.
    ///
    /// HL7 requires RXA-6, and `"0"` is the conventional value for a dose that
    /// was not given — see [`AdministrationEvent::completion_status`]. It is
    /// NOT a stand-in for an amount nobody recorded: a given dose with no
    /// amount is a gap, and the caller must decide what to send.
    pub administered_amount: String,
    /// RXA-7 administered units.
    pub administered_units: Option<String>,
    /// RXA-9 administration notes.
    pub administration_notes: Option<String>,
    /// RXA-10 administering provider as an XCN: `(id, family, given)`.
    ///
    /// Structured, never a pre-joined string — `Field::from_value` escapes `^`
    /// to `\S\`, the defect that reached the wire twice already.
    pub administered_by: Option<(String, String, String)>,
    /// RXA-11 administered-at location, as a caller-assembled `^`-delimited
    /// value: `point_of_care^room^bed^facility^status^type^building^floor`.
    ///
    /// RXA-11 is **LA2**, not the **PL** of PV1-3 — but LA2's first eight
    /// components ARE PL's, and the rest are address parts we do not send. The
    /// same string therefore serves both, which is why the caller can hand this
    /// straight from `resolve_location_pl`.
    pub administered_at_location: Option<String>,
    /// RXA-18 substance/treatment refusal reason — why the patient refused, or
    /// why the dose was held.
    pub refusal_reason: Option<String>,
    /// RXA-20 completion status, HL7 Table 0322: `CP` complete, `RE` refused,
    /// `NA` not administered, `PA` partially administered.
    ///
    /// The whole reason a not-given dose is worth sending. A PIS told only of
    /// completed doses holds an administration record that disagrees with the
    /// chart about every hold and refusal.
    pub completion_status: String,
    /// RXR-1 route.
    pub route: Option<String>,
    /// RXR-2 administration site.
    pub site: Option<String>,
}

/// Builder for `RAS^O17` — Pharmacy/Treatment Administration.
///
/// Structure: MSH · PID · \[PV1\] · ORC · RXA · RXR.
pub struct RasO17Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    administration: AdministrationEvent,
}

impl RasO17Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MessageBuilder::new(version, "RAS", "O17");
        base.message_structure("RAS_O17");
        Self {
            base,
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            administration: AdministrationEvent::default(),
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

    /// Set the whole PID payload at once — the same struct every other builder
    /// takes, so the PID a partner receives does not depend on message type.
    pub fn demographics(mut self, demographics: PatientDemographics) -> Self {
        self.demographics = demographics;
        self
    }

    /// PV1, rendered by [`VisitInfo::to_pv1_segment`]. Omitted entirely when
    /// the visit states no patient class.
    pub fn visit(mut self, visit: VisitInfo) -> Self {
        self.visit = visit;
        self
    }

    pub fn administration(mut self, event: AdministrationEvent) -> Self {
        self.administration = event;
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

        let a = &self.administration;

        // ORC — `RE` (observations/results to follow), the canonical control
        // code for reporting a performed service, as VXU^V04 uses for a
        // vaccination. This message REPORTS an administration; it does not
        // request one, so `NW` would be wrong.
        let mut orc = Segment::new("ORC");
        orc.add_field(Field::from_value("RE")); // ORC-1
        orc.add_field(Field::from_value(&a.order_id)); // ORC-2 placer
        self.base.message.add_segment(orc);

        // RXA, positional through RXA-20.
        let mut rxa = Segment::new("RXA");
        rxa.add_field(Field::from_value("0")); // RXA-1 give sub-ID
        rxa.add_field(Field::from_value("1")); // RXA-2 administration sub-ID
        rxa.add_field(Field::from_value(&a.start_datetime)); // RXA-3
        rxa.add_field(Field::from_value(&a.end_datetime)); // RXA-4
        // RXA-5 administered code (CWE).
        let mut rxa5 = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(
            a.medication_code.as_deref().unwrap_or(""),
        ));
        rep.add_component(Component::from_value(&a.medication_name));
        rep.add_component(Component::from_value(&a.coding_system));
        rxa5.add_repetition(rep);
        rxa.add_field(rxa5);
        rxa.add_field(Field::from_value(&a.administered_amount)); // RXA-6
        rxa.add_field(Field::from_value(
            a.administered_units.as_deref().unwrap_or(""),
        )); // RXA-7
        rxa.add_field(Field::from_value("")); // RXA-8
        rxa.add_field(Field::from_value(
            a.administration_notes.as_deref().unwrap_or(""),
        )); // RXA-9
        // RXA-10 as an XCN, built from components.
        let mut rxa10 = Field::new();
        if let Some((id, family, given)) = a.administered_by.as_ref() {
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(id));
            rep.add_component(Component::from_value(family));
            rep.add_component(Component::from_value(given));
            rxa10.add_repetition(rep);
        }
        rxa.add_field(rxa10); // RXA-10
        // RXA-11 rendered as COMPONENTS, never as the joined string it arrives
        // as: `Field::from_value` escapes `^` to `\S\`, which would turn eight
        // location components into one unparseable blob. That defect has
        // reached this wire twice already (ORC-12, then RXA-10), and PV1-3
        // splits for the same reason.
        let mut rxa11 = Field::new();
        if let Some(pl) = a
            .administered_at_location
            .as_deref()
            .filter(|s| !s.is_empty())
        {
            let mut rep = Repetition::new();
            for part in pl.split('^') {
                rep.add_component(Component::from_value(part));
            }
            rxa11.add_repetition(rep);
        }
        rxa.add_field(rxa11); // RXA-11
        for _ in 12..=17 {
            rxa.add_field(Field::from_value(""));
        }
        rxa.add_field(Field::from_value(a.refusal_reason.as_deref().unwrap_or(""))); // RXA-18
        rxa.add_field(Field::from_value("")); // RXA-19
        rxa.add_field(Field::from_value(&a.completion_status)); // RXA-20
        self.base.message.add_segment(rxa);

        // RXR — route / site. Required by the ADMINISTRATION group, so it is
        // emitted even when the route is unrecorded rather than dropping a
        // mandatory segment.
        let mut rxr = Segment::new("RXR");
        rxr.add_field(Field::from_value(a.route.as_deref().unwrap_or(""))); // RXR-1
        rxr.add_field(Field::from_value(a.site.as_deref().unwrap_or(""))); // RXR-2
        self.base.message.add_segment(rxr);

        Ok(self.base.build())
    }
}
