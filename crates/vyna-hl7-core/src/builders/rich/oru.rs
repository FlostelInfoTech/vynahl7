//! ORU (Observation Result) message builders

use super::VisitInfo;
use super::laboratory::{SpecimenInfo, spm_segment};
use super::{
    MessageBuilder, PatientAddress, PatientDemographics, PatientIdentifier, generate_control_id,
};
use crate::{
    Version,
    error::Result,
    field::{Component, Field, Repetition, SubComponent},
    message::Message,
    segment::Segment,
};

/// Structured OBX observation — one entry per result component.
///
/// The flat `identifier` string is still accepted for backwards
/// compatibility; when the coded form is needed (CWE: code^name^system)
/// populate [`Self::identifier_code`] + [`Self::identifier_system`]
/// alongside `identifier`.
#[derive(Debug, Clone, Default)]
pub struct Observation {
    pub set_id: u32,
    /// OBX-2 value type (NM, ST, CWE, TX, etc.).
    pub value_type: String,
    /// OBX-3 observation identifier — the human name half of the CWE.
    pub identifier: String,
    /// OBX-3 code (coding-system-specific id).
    pub identifier_code: Option<String>,
    /// OBX-3 coding system (LOINC, SNOMED, local).
    pub identifier_system: Option<String>,
    /// OBX-3.1's SECOND SUBCOMPONENT — the HL7 v2.5.1 §7.2.3 section suffix,
    /// as in `71020&IMP`.
    ///
    /// A separate field rather than part of `identifier_code`, because `&` is
    /// the subcomponent separator: embedding it in the code would be ESCAPED
    /// to `\T\` and reach the receiver as literal text rather than as a
    /// subcomponent boundary.
    pub identifier_suffix: Option<String>,
    /// OBX-5 observation value.
    pub value: String,
    /// OBX-6 units.
    pub units: Option<String>,
    /// OBX-7 reference range.
    pub reference_range: Option<String>,
    /// OBX-8 abnormal flags — HL7 table 0078, and **repeating**.
    ///
    /// A list rather than one string, because the field genuinely repeats and
    /// the table mixes orthogonal axes into it: a laboratory reporting a value
    /// that is both high and sharply changed sends `H~U`, and both statements
    /// are true.
    ///
    /// Handing this a single `"H~U"` cannot work — `~` is the repetition
    /// separator, so the encoder ESCAPES it to `\R\` and the receiver reads
    /// one nonexistent code instead of two real ones. Same trap as
    /// `identifier_suffix` and `&` above; modelled the same way.
    pub abnormal_flag: Vec<String>,
    /// OBX-11 observation result status (F / P / C / etc.). Defaults
    /// to "F" (final) when empty at build time.
    pub status: String,
}

/// Builder for ORU^R01 — Unsolicited Observation Message.
pub struct OruR01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    demographics: PatientDemographics,
    visit: VisitInfo,
    // OBR
    placer_order_number: Option<String>,
    filler_order_number: Option<String>,
    universal_service_code: Option<String>,
    universal_service_name: Option<String>,
    universal_service_system: Option<String>,
    observation_datetime: Option<String>,
    results_reported_datetime: Option<String>,
    diagnostic_service_section: Option<String>,
    result_status: Option<String>,
    // OBX
    observations: Vec<Observation>,
    // SPM
    specimens: Vec<SpecimenInfo>,
}

impl OruR01Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "ORU", "R01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            visit: VisitInfo::default(),
            placer_order_number: None,
            filler_order_number: None,
            universal_service_code: None,
            universal_service_name: None,
            universal_service_system: None,
            observation_datetime: None,
            results_reported_datetime: None,
            diagnostic_service_section: None,
            result_status: None,
            observations: Vec::new(),
            specimens: Vec::new(),
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
    pub fn message_structure(mut self, structure: &str) -> Self {
        self.base.message_structure(structure);
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

    // ── OBR ────────────────────────────────────────────────────────

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.placer_order_number = Some(number.to_string());
        self
    }
    pub fn filler_order_number(mut self, number: &str) -> Self {
        self.filler_order_number = Some(number.to_string());
        self
    }
    pub fn universal_service(mut self, code: &str, name: &str) -> Self {
        self.universal_service_code = Some(code.to_string());
        self.universal_service_name = Some(name.to_string());
        self
    }

    /// OBR-4 with its coding system — `("24323-8", "Comprehensive metabolic
    /// panel", "LN")`.
    ///
    /// The ORM builder has carried this since its own OBR-4 fix; this one
    /// never gained it, so every result message named a battery by a code and
    /// left the receiver to guess whose vocabulary it came from. A bare
    /// `24323-8` is a LOINC code, a local compendium number, or neither,
    /// depending on who is reading.
    pub fn universal_service_coded(mut self, code: &str, name: &str, system: &str) -> Self {
        self.universal_service_code = Some(code.to_string());
        self.universal_service_name = Some(name.to_string());
        self.universal_service_system = Some(system.to_string());
        self
    }
    pub fn observation_datetime(mut self, datetime: &str) -> Self {
        self.observation_datetime = Some(datetime.to_string());
        self
    }

    /// OBR-22 — when the laboratory REPORTED this result.
    ///
    /// A different event from OBR-7, which is when the specimen was
    /// collected: a blood culture is collected days before it turns positive.
    /// A receiver with only OBR-7 cannot tell how old a report is, and one
    /// that files OBR-7 as the collection time (as any conformant reader
    /// does) learns nothing at all about when the result was produced.
    pub fn results_reported_datetime(mut self, datetime: &str) -> Self {
        self.results_reported_datetime = Some(datetime.to_string());
        self
    }
    /// OBR-24 — the diagnostic service section (HL7 table 0074: `RAD`, `LAB`,
    /// `MB`, `SP`, `EC`, …).
    ///
    /// The only field that says what KIND of report this is. A receiver files a
    /// report without it under whatever its default category is, which for most
    /// EMRs is laboratory.
    pub fn diagnostic_service_section(mut self, section: &str) -> Self {
        self.diagnostic_service_section = Some(section.to_string());
        self
    }

    /// OBR-25 result status (F / P / C / I / R / S / X / Y).
    pub fn result_status(mut self, status: &str) -> Self {
        self.result_status = Some(status.to_string());
        self
    }

    // ── OBX ────────────────────────────────────────────────────────

    pub fn add_observation(mut self, obs: Observation) -> Self {
        self.observations.push(obs);
        self
    }

    // ── SPM ────────────────────────────────────────────────────────

    /// A specimen this report's observations were made on, sent as an SPM in
    /// the SPECIMEN group that follows the OBX (HL7 v2.5 `ORU_R01`).
    ///
    /// SPM-4 is where v2.5 puts the specimen type. This builder used to write
    /// it into OBX-15, which §7.4.2.15 defines as Producer's ID — the
    /// laboratory that ran the test — so a conformant receiver read the
    /// specimen as a laboratory's name.
    pub fn add_specimen(mut self, specimen: SpecimenInfo) -> Self {
        self.specimens.push(specimen);
        self
    }

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

    /// The visit this order/result belongs to, rendered as PV1.
    ///
    /// PV1 is OPTIONAL in both `ORM_O01` and `ORU_R01` (it sits in the
    /// optional PATIENT/VISIT group), and this builder emitted none at all —
    /// so every outbound lab order and every outbound result described no
    /// visit whatsoever. A receiver filing a result against the right
    /// encounter had nothing to key on, while `ClinicalOrder.encounter_id`
    /// and `ClinicalResult.encounter_id` both sat in the row being read.
    ///
    /// Rendered by [`VisitInfo::to_pv1_segment`], the same builder every ADT
    /// uses, so a partner reconciling this against the admit reads the same
    /// twenty fields in both. Unset leaves the segment out, which is what the
    /// structure permits.
    pub fn visit(mut self, visit: VisitInfo) -> Self {
        self.visit = visit;
        self
    }

    /// Build the message.
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

        // PV1 after PID: the optional VISIT group sits between the
        // patient and the order in both structures.
        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }

        // OBR positionally through OBR-25 so result_status lands at
        // its expected offset for partner parsers.
        let mut obr = Segment::new("OBR");
        obr.add_field(Field::from_value("1")); // OBR-1
        obr.add_field(Field::from_value(
            self.placer_order_number.as_deref().unwrap_or(""),
        )); // OBR-2
        obr.add_field(Field::from_value(
            self.filler_order_number.as_deref().unwrap_or(""),
        )); // OBR-3
        // OBR-4 Universal Service Identifier (CWE): identifier^text.
        //
        // Same reversal the ORM builder carried, and fixed with it: the
        // identifier comes first. A result report that names the battery in the
        // code position cannot be matched to the order that requested it.
        if self.universal_service_code.is_some() || self.universal_service_name.is_some() {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            match self.universal_service_code.as_deref() {
                Some(code) => {
                    rep.add_component(Component::from_value(code));
                    rep.add_component(Component::from_value(
                        self.universal_service_name.as_deref().unwrap_or(""),
                    ));
                    // `^system` only when populated — a trailing empty
                    // component makes strict parsers index wrong positions.
                    if let Some(system) = self.universal_service_system.as_deref() {
                        rep.add_component(Component::from_value(system));
                    }
                }
                None => rep.add_component(Component::from_value(
                    self.universal_service_name.as_deref().unwrap_or(""),
                )),
            }
            field.add_repetition(rep);
            obr.add_field(field);
        } else {
            obr.add_field(Field::from_value(""));
        }
        obr.add_field(Field::from_value("")); // OBR-5 Priority
        obr.add_field(Field::from_value("")); // OBR-6 Requested Date/Time
        obr.add_field(Field::from_value(
            self.observation_datetime.as_deref().unwrap_or(""),
        )); // OBR-7 Observation Date/Time
        for _ in 8..=21 {
            obr.add_field(Field::from_value(""));
        }
        obr.add_field(Field::from_value(
            self.results_reported_datetime.as_deref().unwrap_or(""),
        )); // OBR-22 Results Rpt/Status Chng - Date/Time
        obr.add_field(Field::from_value("")); // OBR-23 Charge to Practice
        obr.add_field(Field::from_value(
            self.diagnostic_service_section.as_deref().unwrap_or(""),
        )); // OBR-24 Diagnostic Serv Sect ID
        obr.add_field(Field::from_value(
            self.result_status.as_deref().unwrap_or(""),
        )); // OBR-25 Result Status
        self.base.message.add_segment(obr);

        // OBX per observation.
        for obs in &self.observations {
            let mut obx = Segment::new("OBX");
            obx.add_field(Field::from_value(obs.set_id.to_string())); // OBX-1
            obx.add_field(Field::from_value(&obs.value_type)); // OBX-2
            // OBX-3 Observation Identifier (CWE). Components emitted:
            // code^text when identifier_code is set; adds ^system only
            // if the caller populated identifier_system (avoids a
            // trailing empty component that makes strict parsers
            // index wrong positions).
            let mut field = Field::new();
            let mut rep = Repetition::new();
            if let Some(code) = &obs.identifier_code {
                match &obs.identifier_suffix {
                    // `code&SUFFIX` — two SUBcomponents of OBX-3.1, which is
                    // what §7.2.3 specifies for a narrative report's sections.
                    Some(suffix) => {
                        let mut ident = Component::new();
                        ident.add_subcomponent(SubComponent::new(code.clone()));
                        ident.add_subcomponent(SubComponent::new(suffix.clone()));
                        rep.add_component(ident);
                    }
                    None => rep.add_component(Component::from_value(code)),
                }
                rep.add_component(Component::from_value(&obs.identifier));
                if let Some(system) = &obs.identifier_system {
                    rep.add_component(Component::from_value(system));
                }
            } else {
                rep.add_component(Component::from_value(&obs.identifier));
            }
            field.add_repetition(rep);
            obx.add_field(field);
            obx.add_field(Field::from_value("")); // OBX-4 Sub-ID
            obx.add_field(Field::from_value(&obs.value)); // OBX-5 Value
            obx.add_field(Field::from_value(obs.units.as_deref().unwrap_or(""))); // OBX-6
            obx.add_field(Field::from_value(
                obs.reference_range.as_deref().unwrap_or(""),
            )); // OBX-7
            // OBX-8: one repetition per flag, never one value with a tilde in it.
            let mut flags = Field::new();
            for flag in &obs.abnormal_flag {
                flags.add_repetition(Repetition::from_value(flag));
            }
            if obs.abnormal_flag.is_empty() {
                flags.add_repetition(Repetition::from_value(""));
            }
            obx.add_field(flags); // OBX-8
            obx.add_field(Field::from_value("")); // OBX-9 Probability
            obx.add_field(Field::from_value("")); // OBX-10 Nature of Abnormal Test
            let status = if obs.status.is_empty() {
                "F"
            } else {
                &obs.status
            };
            obx.add_field(Field::from_value(status)); // OBX-11 Result Status
            self.base.message.add_segment(obx);
        }

        // SPM per specimen: the SPECIMEN group, after the OBX it describes.
        for (i, specimen) in self.specimens.iter().enumerate() {
            self.base.message.add_segment(spm_segment(i + 1, specimen));
        }

        Ok(self.base.build())
    }
}
