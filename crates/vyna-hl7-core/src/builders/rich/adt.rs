//! ADT (Admit/Discharge/Transfer) message builders

use super::{MessageBuilder, generate_control_id, rich_base};
use crate::{
    Version,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
};

/// Builder for ADT messages
pub struct RichAdtBuilder {
    #[allow(dead_code)]
    base: MessageBuilder,
}

impl RichAdtBuilder {
    /// Create a new ADT message builder
    pub fn new(version: Version, trigger_event: &str) -> Self {
        Self {
            base: rich_base(version, "ADT", trigger_event),
        }
    }

    /// Create ADT^A01 - Admit/Visit Notification
    pub fn a01(version: Version) -> RichAdtA01Builder {
        RichAdtA01Builder::new(version)
    }

    /// Create ADT^A02 - Transfer a Patient
    pub fn a02(version: Version) -> RichAdtA02Builder {
        RichAdtA02Builder::new(version)
    }

    /// Create ADT^A03 - Discharge/End Visit
    pub fn a03(version: Version) -> RichAdtA03Builder {
        RichAdtA03Builder::new(version)
    }

    /// Create ADT^A04 - Register a Patient
    pub fn a04(version: Version) -> RichAdtA04Builder {
        RichAdtA04Builder::new(version)
    }

    /// Create ADT^A05 - Pre-admit a Patient
    pub fn a05(version: Version) -> RichAdtA05Builder {
        RichAdtA05Builder::new(version)
    }

    /// Create ADT^A06 - Change Outpatient to Inpatient
    pub fn a06(version: Version) -> RichAdtA06Builder {
        RichAdtA06Builder::new(version)
    }

    /// Create ADT^A07 - Change Inpatient to Outpatient
    pub fn a07(version: Version) -> RichAdtA07Builder {
        RichAdtA07Builder::new(version)
    }

    /// Create ADT^A08 - Update Patient Information
    pub fn a08(version: Version) -> RichAdtA08Builder {
        RichAdtA08Builder::new(version)
    }

    /// Create ADT^A11 - Cancel Admit/Visit Notification
    pub fn a11(version: Version) -> RichAdtA11Builder {
        RichAdtA11Builder::new(version)
    }

    /// Create ADT^A12 - Cancel Transfer
    pub fn a12(version: Version) -> RichAdtA12Builder {
        RichAdtA12Builder::new(version)
    }

    /// Create ADT^A13 - Cancel Discharge/End Visit
    pub fn a13(version: Version) -> RichAdtA13Builder {
        RichAdtA13Builder::new(version)
    }

    /// Create ADT^A09 - Patient Departing - Tracking
    pub fn a09(version: Version) -> RichAdtA09Builder {
        RichAdtA09Builder::new(version)
    }

    /// Create ADT^A10 - Patient Arriving - Tracking
    pub fn a10(version: Version) -> RichAdtA10Builder {
        RichAdtA10Builder::new(version)
    }

    /// Create ADT^A17 - Swap Patients
    pub fn a17(version: Version) -> RichAdtA17Builder {
        RichAdtA17Builder::new(version)
    }

    /// Create ADT^A28 - Add Person Information
    pub fn a28(version: Version) -> RichAdtA28Builder {
        RichAdtA28Builder::new(version)
    }

    /// Create ADT^A31 - Update Person Information
    pub fn a31(version: Version) -> RichAdtA31Builder {
        RichAdtA31Builder::new(version)
    }

    /// Create ADT^A40 - Merge Patient - Patient Identifier List
    pub fn a40(version: Version) -> RichAdtA40Builder {
        RichAdtA40Builder::new(version)
    }
}

/// Builder for ADT^A01 - Admit/Visit Notification
pub struct RichAdtA01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    /// All PID material lives in the shared struct, exactly as the other
    /// nine patient-carrying builders do. ADT previously kept its own
    /// copies plus a byte-identical duplicate of the PID rendering code;
    /// extending the segment then fixed nine message types and silently
    /// missed this one.
    demographics: PatientDemographics,
    /// AL1 / NK1 segments, emitted after PID in the order HL7 defines.
    /// Both were absent from every message before this: an ADT admitting a
    /// patient carried no allergies and no emergency contact.
    allergies: Vec<PatientAllergy>,
    next_of_kin: Vec<NextOfKin>,
    /// All PV1 material lives in the shared struct, as PID does.
    visit: VisitInfo,
    // MRG segment material — populated for merge / link variants
    // (A34/A35/A36/A39/A40/A41/A42). When set, build() emits an
    // MRG segment after PV1 carrying the retired (source) patient.
    event_occurred: Option<String>,
    prior_patient_identifier: Option<PatientIdentifier>,
    prior_patient_name: Option<(String, String)>,
}

// Re-export the shared patient identifier / address structs so
// existing call sites (rs7_core::builders::adt::PatientIdentifier,
// ::PatientAddress) keep working after the move into the parent
// module.
pub use super::{CodedValue, NextOfKin, PatientAddress, PatientAllergy, PatientIdentifier};
use super::{PatientDemographics, VisitInfo};

impl RichAdtA01Builder {
    /// Create a new ADT^A01 builder
    pub fn new(version: Version) -> Self {
        Self {
            base: rich_base(version, "ADT", "A01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            demographics: PatientDemographics::default(),
            allergies: Vec::new(),
            next_of_kin: Vec::new(),
            visit: VisitInfo::default(),
            event_occurred: None,
            prior_patient_identifier: None,
            prior_patient_name: None,
        }
    }

    /// Set sending application
    pub fn sending_application(mut self, app: &str) -> Self {
        self.sending_app = app.to_string();
        self
    }

    /// Set sending facility
    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.sending_facility = facility.to_string();
        self
    }

    /// Set receiving application
    pub fn receiving_application(mut self, app: &str) -> Self {
        self.receiving_app = app.to_string();
        self
    }

    /// Set receiving facility
    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.receiving_facility = facility.to_string();
        self
    }

    /// Set message control ID (auto-generated if not set)
    pub fn control_id(mut self, id: &str) -> Self {
        self.control_id = Some(id.to_string());
        self
    }

    /// Set processing ID (defaults to "P" for Production)
    pub fn processing_id(mut self, id: &str) -> Self {
        self.processing_id = id.to_string();
        self
    }

    /// Set patient ID
    pub fn patient_id(mut self, id: &str) -> Self {
        self.demographics.patient_id_flat = Some(id.to_string());
        self
    }

    /// Set patient name
    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.demographics.name = Some((family.to_string(), given.to_string()));
        self
    }

    /// Set date of birth (format: YYYYMMDD)
    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.demographics.date_of_birth = Some(dob.to_string());
        self
    }

    /// Set administrative sex
    pub fn sex(mut self, sex: &str) -> Self {
        self.demographics.sex = Some(sex.to_string());
        self
    }

    /// Set patient class (I=Inpatient, O=Outpatient, E=Emergency, etc.)
    pub fn patient_class(mut self, class: &str) -> Self {
        self.visit.patient_class = Some(class.to_string());
        self
    }

    /// Set assigned patient location
    pub fn assigned_location(mut self, location: &str) -> Self {
        self.visit.assigned_location = Some(location.to_string());
        self
    }

    /// Set prior patient location (PV1-6). Required by A02 (transfer)
    /// so downstream systems know where the patient came from.
    pub fn prior_patient_location(mut self, location: &str) -> Self {
        self.visit.prior_patient_location = Some(location.to_string());
        self
    }

    /// Set attending doctor
    pub fn attending_doctor(mut self, doctor: &str) -> Self {
        self.visit.attending_doctor = Some(doctor.to_string());
        self
    }

    /// Set admit date/time (populates PV1-44).
    pub fn admit_datetime(mut self, datetime: &str) -> Self {
        self.visit.admit_datetime = Some(datetime.to_string());
        self
    }

    /// Set structured patient identifier (PID-3 as CX:
    /// id^check_digit^check_scheme^assigning_authority). Wins over the
    /// flat [`Self::patient_id`] setter when both are provided.
    pub fn patient_identifier(mut self, identifier: PatientIdentifier) -> Self {
        self.demographics.identifier = Some(identifier);
        self
    }

    /// Set structured patient address (PID-11 XAD).
    pub fn patient_address(mut self, address: PatientAddress) -> Self {
        self.demographics.address = Some(address);
        self
    }

    /// Set home phone (PID-13).
    pub fn phone_home(mut self, phone: &str) -> Self {
        self.demographics.phone_home = Some(phone.to_string());
        self
    }

    /// Attach the patient's allergies, rendered as AL1 segments after PID.
    ///
    /// Sending an admission without these means the receiving system's own
    /// allergy checking starts from nothing, however much the sender knew.
    pub fn allergies(mut self, allergies: Vec<PatientAllergy>) -> Self {
        self.allergies = allergies;
        self
    }

    /// Attach next-of-kin / emergency contacts, rendered as NK1 segments.
    pub fn next_of_kin(mut self, contacts: Vec<NextOfKin>) -> Self {
        self.next_of_kin = contacts;
        self
    }

    /// Set the whole PV1 payload at once, so a caller assembles the visit
    /// once and every message type describes it identically.
    pub fn visit(mut self, visit: VisitInfo) -> Self {
        self.visit = visit;
        self
    }

    /// Set the whole PID payload at once.
    ///
    /// Lets a caller assemble demographics in one place and hand the same
    /// struct to every message type, rather than re-deriving the patient
    /// wiring per builder — which is how the ten call sites in the gateway
    /// came to disagree about which PID fields get sent.
    pub fn demographics(mut self, demographics: PatientDemographics) -> Self {
        self.demographics = demographics;
        self
    }

    /// Add a typed identifier as a further PID-3 repetition (CX).
    ///
    /// This is where a national ID belongs — NRIC, HKID, MyKad, Aadhaar —
    /// with `identifier_type_code: Some("NI")`. NOT PID-19: that field is
    /// the SSN, and a national ID placed there is read as one.
    pub fn additional_identifier(mut self, ident: PatientIdentifier) -> Self {
        self.demographics.additional_identifiers.push(ident);
        self
    }

    /// Set XPN-3 middle name and XPN-4 suffix inside PID-5.
    pub fn name_parts(mut self, middle: Option<&str>, suffix: Option<&str>) -> Self {
        self.demographics.middle_name = middle.map(str::to_string);
        self.demographics.name_suffix = suffix.map(str::to_string);
        self
    }

    /// Set patient alias (PID-9) — in APAC usually the local-script name.
    pub fn alias(mut self, alias: &str) -> Self {
        self.demographics.alias = Some(alias.to_string());
        self
    }

    /// Add one race (PID-10).
    ///
    /// PID-10 repeats, so calling this more than once ADDS a repetition rather
    /// than replacing the previous value.
    pub fn race(mut self, value: CodedValue) -> Self {
        self.demographics.race.push(value);
        self
    }

    /// Set every race at once (PID-10), replacing any already added.
    pub fn races(mut self, values: Vec<CodedValue>) -> Self {
        self.demographics.race = values;
        self
    }

    /// Set business phone (PID-14).
    pub fn phone_business(mut self, phone: &str) -> Self {
        self.demographics.phone_business = Some(phone.to_string());
        self
    }

    /// Set primary language (PID-15).
    pub fn primary_language(mut self, value: CodedValue) -> Self {
        self.demographics.primary_language = Some(value);
        self
    }

    /// Set marital status (PID-16).
    pub fn marital_status(mut self, value: CodedValue) -> Self {
        self.demographics.marital_status = Some(value);
        self
    }

    /// Set religion (PID-17).
    pub fn religion(mut self, value: CodedValue) -> Self {
        self.demographics.religion = Some(value);
        self
    }

    /// Set driver's licence number and issuing state (PID-20).
    pub fn driver_license(mut self, number: &str, state: Option<&str>) -> Self {
        self.demographics.driver_license = Some((number.to_string(), state.map(str::to_string)));
        self
    }

    /// Add one ethnic group (PID-22).
    ///
    /// Repeats like PID-10: calling this more than once adds a repetition.
    pub fn ethnic_group(mut self, value: CodedValue) -> Self {
        self.demographics.ethnic_group.push(value);
        self
    }

    /// Set every ethnic group at once (PID-22), replacing any already added.
    pub fn ethnic_groups(mut self, values: Vec<CodedValue>) -> Self {
        self.demographics.ethnic_group = values;
        self
    }

    /// Set nationality (PID-28).
    pub fn nationality(mut self, value: CodedValue) -> Self {
        self.demographics.nationality = Some(value);
        self
    }

    /// Set death date/time (PID-29) and the death indicator (PID-30).
    ///
    /// Taken together so a message cannot say a patient died without
    /// saying so in PID-30, which is the field receivers actually branch
    /// on. `deceased = false` renders `N`; leaving this unset renders an
    /// empty PID-30, because "not asserted" is not "alive".
    pub fn death(mut self, deceased: bool, datetime: Option<&str>) -> Self {
        self.demographics.death_indicator = Some(deceased);
        self.demographics.death_datetime = datetime.map(str::to_string);
        self
    }

    /// Set the last-update date/time for the demographics (PID-33).
    pub fn last_update_datetime(mut self, ts: &str) -> Self {
        self.demographics.last_update_datetime = Some(ts.to_string());
        self
    }

    /// Set admission type (PV1-4, e.g. "E" Emergency, "R" Routine).
    pub fn admission_type(mut self, admission_type: &str) -> Self {
        self.visit.admission_type = Some(admission_type.to_string());
        self
    }

    /// Set visit number (PV1-19). Typically the encounter number.
    pub fn visit_number(mut self, number: &str) -> Self {
        self.visit.visit_number = Some(number.to_string());
        self
    }

    /// Set discharge date/time (PV1-45). Used by A03 builders derived
    /// from A01; storing it here keeps the field exposed on the base.
    pub fn discharge_datetime(mut self, datetime: &str) -> Self {
        self.visit.discharge_datetime = Some(datetime.to_string());
        self
    }

    /// Set discharge disposition (PV1-36).
    pub fn discharge_disposition(mut self, disposition: &str) -> Self {
        self.visit.discharge_disposition = Some(disposition.to_string());
        self
    }

    /// Set the MSH-9 third component (message structure), e.g.
    /// `"ADT_A01"`. Required by HL7 v2.5 partner systems that route
    /// on the full MSG triple.
    pub fn message_structure(mut self, structure: &str) -> Self {
        self.base.message_structure(structure);
        self
    }

    /// Override the trigger event for this builder — lets one builder
    /// emit any ADT variant (A02/A03/…) without reinstantiating. MSH-9
    /// second component and EVN-1 both follow this value.
    pub fn trigger_event(mut self, trigger: &str) -> Self {
        self.base.set_trigger_event(trigger);
        self
    }

    /// EVN-6 — when the event this message describes actually occurred.
    ///
    /// Distinct from EVN-2 (when it was recorded), and the field a conformant
    /// receiver reads for the admission / discharge / transfer instant.
    pub fn event_occurred(mut self, datetime: &str) -> Self {
        self.event_occurred = Some(datetime.to_string());
        self
    }

    /// Set the retired (source) patient identifier for MRG-1. Used
    /// by merge / link events (A40/A41/A42, etc.). When set, build()
    /// emits an MRG segment after PV1.
    pub fn prior_patient_identifier(mut self, identifier: PatientIdentifier) -> Self {
        self.prior_patient_identifier = Some(identifier);
        self
    }

    /// Set the retired (source) patient name for MRG-7.
    pub fn prior_patient_name(mut self, family: &str, given: &str) -> Self {
        self.prior_patient_name = Some((family.to_string(), given.to_string()));
        self
    }

    /// Build the message
    pub fn build(mut self) -> Result<Message> {
        let control_id = self.control_id.unwrap_or_else(generate_control_id);

        // Create MSH segment
        let msh = self.base.create_msh(
            &self.sending_app,
            &self.sending_facility,
            &self.receiving_app,
            &self.receiving_facility,
            &control_id,
            &self.processing_id,
        )?;
        self.base.message.add_segment(msh);

        // Create EVN segment — mirrors the builder's current trigger
        // so a single RichAdtA01Builder instance can emit any variant
        // (A02/A03/…) via [`Self::trigger_event`].
        let trigger = self.base.trigger_event().to_string();
        // EVN-6 carries WHEN THE EVENT HAPPENED. `create_evn` alone emits only
        // EVN-2 — the RECORDED time, which defaults to now — so a receiver
        // reading EVN-2 as the event instant got the moment this message was
        // built. Ours did.
        let evn =
            self.base
                .create_evn_with_occurrence(&trigger, None, self.event_occurred.as_deref())?;
        self.base.message.add_segment(evn);

        // PID segment — built by the SHARED `PatientDemographics`, which
        // every other message type already uses (ORM / ORU / SIU / MDM /
        // DFT / BAR / laboratory / referral / pharmacy). ADT used to carry
        // its own byte-identical copy of this code; two implementations of
        // one segment is how the ADT PID and everyone else's drift apart,
        // and the copy meant extending the segment fixed eight message
        // types and missed the ninth.
        self.base
            .message
            .add_segment(self.demographics.to_pid_segment());

        // NK1 then AL1, both after PID. Set IDs are 1-based and increment
        // per segment, which is what lets a receiver tell repeats apart.
        for (i, nk) in self.next_of_kin.iter().enumerate() {
            self.base.message.add_segment(nk.to_nk1_segment(i + 1));
        }
        for (i, allergy) in self.allergies.iter().enumerate() {
            self.base.message.add_segment(allergy.to_al1_segment(i + 1));
        }

        // PV1 — built by the SHARED `VisitInfo`, as PID is. This segment had
        // SEVEN independent implementations across the builders, emitting
        // between 2 and 17 of its 52 fields, so the same visit described
        // itself differently depending on which message type carried it.
        if let Some(pv1) = self.visit.to_pv1_segment() {
            self.base.message.add_segment(pv1);
        }

        // MRG segment — emitted for merge / link events when the
        // retired-patient identifier has been set. Structure follows
        // HL7 v2.5: MRG-1 prior patient identifier list (CX), MRG-7
        // prior patient name (XPN). Positions 2..=6 padded empty.
        if let Some(ident) = &self.prior_patient_identifier {
            let mut mrg = Segment::new("MRG");
            let mut mrg1 = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(&ident.id));
            rep.add_component(Component::from_value(
                ident.check_digit.as_deref().unwrap_or(""),
            ));
            rep.add_component(Component::from_value(
                ident.check_digit_scheme.as_deref().unwrap_or(""),
            ));
            rep.add_component(Component::from_value(
                ident.assigning_authority.as_deref().unwrap_or(""),
            ));
            if let Some(type_code) = &ident.identifier_type_code {
                rep.add_component(Component::from_value(type_code));
            }
            mrg1.add_repetition(rep);
            mrg.add_field(mrg1);
            for _ in 2..=6 {
                mrg.add_field(Field::from_value(""));
            }
            if let Some((family, given)) = &self.prior_patient_name {
                let mut mrg7 = Field::new();
                let mut name_rep = Repetition::new();
                name_rep.add_component(Component::from_value(family));
                name_rep.add_component(Component::from_value(given));
                mrg7.add_repetition(name_rep);
                mrg.add_field(mrg7);
            }
            self.base.message.add_segment(mrg);
        }

        Ok(self.base.build())
    }
}

/// Builder for ADT^A02 - Transfer a Patient
pub struct RichAdtA02Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA02Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A02");
        Self { base }
    }

    // Delegate all methods to RichAdtA01Builder
    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn assigned_location(mut self, location: &str) -> Self {
        self.base = self.base.assigned_location(location);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A03 - Discharge/End Visit
pub struct RichAdtA03Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA03Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A03");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A04 - Register a Patient
pub struct RichAdtA04Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA04Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A04");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.base = self.base.date_of_birth(dob);
        self
    }

    pub fn sex(mut self, sex: &str) -> Self {
        self.base = self.base.sex(sex);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A08 - Update Patient Information
pub struct RichAdtA08Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA08Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A08");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.base = self.base.date_of_birth(dob);
        self
    }

    pub fn sex(mut self, sex: &str) -> Self {
        self.base = self.base.sex(sex);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A05 - Pre-admit a Patient
pub struct RichAdtA05Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA05Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A05");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.base = self.base.date_of_birth(dob);
        self
    }

    pub fn sex(mut self, sex: &str) -> Self {
        self.base = self.base.sex(sex);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A06 - Change Outpatient to Inpatient
pub struct RichAdtA06Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA06Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A06");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn assigned_location(mut self, location: &str) -> Self {
        self.base = self.base.assigned_location(location);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A07 - Change Inpatient to Outpatient
pub struct RichAdtA07Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA07Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A07");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A11 - Cancel Admit/Visit Notification
pub struct RichAdtA11Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA11Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A11");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A12 - Cancel Transfer
pub struct RichAdtA12Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA12Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A12");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A13 - Cancel Discharge/End Visit
pub struct RichAdtA13Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA13Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A13");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A09 - Patient Departing - Tracking
pub struct RichAdtA09Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA09Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A09");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn assigned_location(mut self, location: &str) -> Self {
        self.base = self.base.assigned_location(location);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A10 - Patient Arriving - Tracking
pub struct RichAdtA10Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA10Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A10");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn assigned_location(mut self, location: &str) -> Self {
        self.base = self.base.assigned_location(location);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A17 - Swap Patients
pub struct RichAdtA17Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA17Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A17");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A28 - Add Person Information
pub struct RichAdtA28Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA28Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A28");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.base = self.base.date_of_birth(dob);
        self
    }

    pub fn sex(mut self, sex: &str) -> Self {
        self.base = self.base.sex(sex);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A31 - Update Person Information
pub struct RichAdtA31Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA31Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A31");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.base = self.base.date_of_birth(dob);
        self
    }

    pub fn sex(mut self, sex: &str) -> Self {
        self.base = self.base.sex(sex);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A40 - Merge Patient - Patient Identifier List
pub struct RichAdtA40Builder {
    base: RichAdtA01Builder,
}

impl RichAdtA40Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RichAdtA01Builder::new(version);
        base.base = rich_base(version, "ADT", "A40");
        Self { base }
    }

    pub fn sending_application(mut self, app: &str) -> Self {
        self.base = self.base.sending_application(app);
        self
    }

    pub fn sending_facility(mut self, facility: &str) -> Self {
        self.base = self.base.sending_facility(facility);
        self
    }

    pub fn receiving_application(mut self, app: &str) -> Self {
        self.base = self.base.receiving_application(app);
        self
    }

    pub fn receiving_facility(mut self, facility: &str) -> Self {
        self.base = self.base.receiving_facility(facility);
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.base = self.base.patient_id(id);
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.patient_name(family, given);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    /// Set the retired (source) patient identifier for MRG-1 — required
    /// by the ADT_A40 schema.
    pub fn prior_patient_identifier(mut self, identifier: PatientIdentifier) -> Self {
        self.base = self.base.prior_patient_identifier(identifier);
        self
    }

    /// Set the retired (source) patient name for MRG-7.
    pub fn prior_patient_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.prior_patient_name(family, given);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}
