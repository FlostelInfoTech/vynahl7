//! Rich clinical message builders (the completeness layer).
//!
//! Where the per-trigger builders in the parent module cover the common
//! fields with scalar setters, the builders here carry the full clinical
//! payload: rich PID demographics (race, religion, address, nationality,
//! death indicators), PV1 visits, AL1 allergies, NK1 contacts, and the
//! referral (REF/RRI), billing (BAR/DFT), laboratory (OMG/OMI/OML),
//! pharmacy (OMP/RDE/RDS/RAS/VXU), and document (MDM) families.
//!
//! Ported from the retired in-repo `rs7` workspace so existing senders
//! keep byte-identical wire output; the per-trigger builders remain the
//! recommended entry point for new integrations with simple needs.

pub mod adt;
pub mod billing;
pub mod dft;
pub mod laboratory;
pub mod mdm;
pub mod oru;
pub mod pharmacy;
pub mod referral;
pub mod siu;

use crate::{
    Version,
    delimiters::Delimiters,
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
    types::format_timestamp,
};
use chrono::Utc;

/// Structured patient identifier for PID-3 / MRG-1 (CX data type).
#[derive(Debug, Clone, Default)]
pub struct PatientIdentifier {
    pub id: String,
    pub check_digit: Option<String>,
    pub check_digit_scheme: Option<String>,
    pub assigning_authority: Option<String>,
    /// CX-5, identifier type code (HL7 table 0203): `MR` for a medical
    /// record number, `NI` for a national unique identifier, `PPN` for a
    /// passport. Emitted only when set, so an identifier written before
    /// this field existed still renders as four components.
    pub identifier_type_code: Option<String>,
}

/// A coded element (CE / CWE) for the demographic PID fields.
///
/// Follows the shape already used for OBR-4 in `orm.rs`: when a code is
/// known it renders `code^text^system`; when it is not, the human-readable
/// text goes in component 1 on its own. Emitting free text in component 1
/// is the house convention — putting it in component 2 would be defensible
/// HL7 but would diverge from every other coded field this library emits,
/// and partners that read component 1 would see nothing.
#[derive(Debug, Clone, Default)]
pub struct CodedValue {
    /// CE-1. `None` when we hold only free text.
    pub code: Option<String>,
    /// CE-2 when coded, CE-1 when not.
    pub text: String,
    /// CE-3, the coding system (e.g. `ISO639-1`, `HL70002`).
    pub system: Option<String>,
}

impl CodedValue {
    /// Free text with no code — the common case for demographics typed by
    /// a human ("Buddhism", "Singaporean").
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            code: None,
            text: value.into(),
            system: None,
        }
    }

    /// A value that carries a real code from a named system.
    pub fn coded(
        code: impl Into<String>,
        text: impl Into<String>,
        system: impl Into<String>,
    ) -> Self {
        Self {
            code: Some(code.into()),
            text: text.into(),
            system: Some(system.into()),
        }
    }

    /// The value as one repetition, for fields that may repeat.
    ///
    /// `to_field` wraps exactly this in a single-repetition Field, so a repeated
    /// field and a single one build their components from the same code.
    fn to_repetition(&self) -> Repetition {
        let mut rep = Repetition::new();
        match &self.code {
            Some(code) => {
                rep.add_component(Component::from_value(code));
                rep.add_component(Component::from_value(&self.text));
                if let Some(system) = &self.system {
                    rep.add_component(Component::from_value(system));
                }
            }
            None => rep.add_component(Component::from_value(&self.text)),
        }
        rep
    }

    fn to_field(&self) -> Field {
        let mut field = Field::new();
        let mut rep = Repetition::new();
        match &self.code {
            Some(code) => {
                rep.add_component(Component::from_value(code));
                rep.add_component(Component::from_value(&self.text));
                if let Some(system) = &self.system {
                    rep.add_component(Component::from_value(system));
                }
            }
            None => rep.add_component(Component::from_value(&self.text)),
        }
        field.add_repetition(rep);
        field
    }
}

/// Structured patient address for PID-11 (XAD data type).
#[derive(Debug, Clone, Default)]
pub struct PatientAddress {
    pub line1: String,
    pub line2: String,
    pub city: String,
    pub state: String,
    pub postal_code: String,
    pub country: String,
}

/// The visit (PV1 segment), shared by every message type that carries one.
///
/// PV1 had **seven** independent implementations emitting between 2 and 17 of
/// its 52 fields, so the same visit described itself differently depending on
/// which message type happened to carry it — a lab result carried a 2-field
/// PV1 and an admission a 17-field one, from the same database row.
///
/// Provider fields are **resolved XCN strings** (`NPI^family^given`), never
/// our internal `provider_id`. That key is meaningless to a partner, and our
/// own inbound resolves these against `Provider.npi`; sending the key is the
/// defect fixed across ORC-12, PV1-7, BAR^P01 and SIU AIP. Resolution needs a
/// database round trip, so the caller does it and passes the result.
#[derive(Debug, Clone, Default)]
pub struct VisitInfo {
    /// PV1-2, patient class. PV1 is emitted only when this is set.
    pub patient_class: Option<String>,
    /// PV1-3, assigned location, as a caller-assembled PL string.
    pub assigned_location: Option<String>,
    pub admission_type: Option<String>,
    pub prior_patient_location: Option<String>,
    /// PV1-7 / 8 / 9 / 17 — attending, referring, consulting, admitting.
    pub attending_doctor: Option<String>,
    pub referring_doctor: Option<String>,
    pub consulting_doctor: Option<String>,
    pub admitting_doctor: Option<String>,
    /// PV1-10, hospital service.
    pub hospital_service: Option<String>,
    /// PV1-14, admit source.
    pub admit_source: Option<String>,
    /// PV1-18, patient type.
    pub patient_type: Option<String>,
    pub visit_number: Option<String>,
    /// PV1-20, financial class.
    pub financial_class: Option<String>,
    pub discharge_disposition: Option<String>,
    /// PV1-37, discharged-to location.
    pub discharged_to_location: Option<String>,
    /// PV1-39, servicing facility.
    pub servicing_facility: Option<String>,
    /// PV1-41, account status.
    pub account_status: Option<String>,
    pub admit_datetime: Option<String>,
    pub discharge_datetime: Option<String>,
}

impl VisitInfo {
    /// Render as PV1, padded to 52 fields so consumers indexing by position
    /// find every field where the standard says it is.
    ///
    /// Returns `None` when no patient class is set: PV1-2 is required, and a
    /// PV1 without it is not a visit description a receiver can use.
    pub fn to_pv1_segment(&self) -> Option<Segment> {
        let class = self.patient_class.as_deref()?;
        let mut pv1 = Segment::new("PV1");

        // A caller-assembled `^`-delimited value rendered as components.
        let composite = |value: &str| {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            for part in value.split('^') {
                rep.add_component(Component::from_value(part));
            }
            field.add_repetition(rep);
            field
        };
        let opt_composite = |value: &Option<String>| match value {
            Some(v) => composite(v),
            None => Field::from_value(""),
        };
        let plain = |value: &Option<String>| Field::from_value(value.as_deref().unwrap_or(""));

        pv1.add_field(Field::from_value("1")); // PV1-1: Set ID
        pv1.add_field(Field::from_value(class)); // PV1-2: Patient Class
        pv1.add_field(opt_composite(&self.assigned_location)); // PV1-3
        pv1.add_field(plain(&self.admission_type)); // PV1-4
        pv1.add_field(Field::from_value("")); // PV1-5: Preadmit Number
        pv1.add_field(plain(&self.prior_patient_location)); // PV1-6
        pv1.add_field(opt_composite(&self.attending_doctor)); // PV1-7
        pv1.add_field(opt_composite(&self.referring_doctor)); // PV1-8
        pv1.add_field(opt_composite(&self.consulting_doctor)); // PV1-9
        pv1.add_field(plain(&self.hospital_service)); // PV1-10
        pv1.add_field(Field::from_value("")); // PV1-11: Temporary Location
        pv1.add_field(Field::from_value("")); // PV1-12: Preadmit Test Indicator
        pv1.add_field(Field::from_value("")); // PV1-13: Re-admission Indicator
        pv1.add_field(plain(&self.admit_source)); // PV1-14
        pv1.add_field(Field::from_value("")); // PV1-15: Ambulatory Status
        pv1.add_field(Field::from_value("")); // PV1-16: VIP Indicator
        pv1.add_field(opt_composite(&self.admitting_doctor)); // PV1-17
        pv1.add_field(plain(&self.patient_type)); // PV1-18
        pv1.add_field(plain(&self.visit_number)); // PV1-19
        pv1.add_field(plain(&self.financial_class)); // PV1-20
        // PV1-21 .. PV1-35 — no source.
        for _ in 21..=35 {
            pv1.add_field(Field::from_value(""));
        }
        pv1.add_field(plain(&self.discharge_disposition)); // PV1-36
        pv1.add_field(plain(&self.discharged_to_location)); // PV1-37
        pv1.add_field(Field::from_value("")); // PV1-38: Diet Type
        pv1.add_field(plain(&self.servicing_facility)); // PV1-39
        pv1.add_field(Field::from_value("")); // PV1-40: Bed Status
        pv1.add_field(plain(&self.account_status)); // PV1-41
        pv1.add_field(Field::from_value("")); // PV1-42: Pending Location
        pv1.add_field(Field::from_value("")); // PV1-43: Prior Temporary Location
        pv1.add_field(plain(&self.admit_datetime)); // PV1-44
        pv1.add_field(plain(&self.discharge_datetime)); // PV1-45
        // PV1-46 .. PV1-52 — no source; padded for strict consumers.
        for _ in 46..=52 {
            pv1.add_field(Field::from_value(""));
        }
        Some(pv1)
    }
}

/// One clinical document's header, rendered as a TXA segment.
///
/// TXA carries 23 fields and the MDM builder populated SIX of them — 1, 2,
/// 4, 5, 12 and 17 — while the inbound MDM handler on the other side of
/// this same product had just been taught to READ TXA-9, TXA-13 and
/// TXA-22. An asymmetry between the two halves of one interface is an
/// oversight, not a decision: a partner speaking our own dialect back to
/// us would have been understood, and a document we sent could not be.
///
/// Every field is optional and an absent one renders EMPTY. A document
/// header is a set of assertions about a document; an assertion we cannot
/// make is one the receiver must not be given.
#[derive(Debug, Clone, Default)]
pub struct DocumentHeader {
    /// TXA-2, document type (table 0270 — `HP`, `CN`, `DS`, …).
    pub document_type: Option<String>,
    /// TXA-3, document content presentation (table 0191). `TX` when the
    /// body rides in an OBX; empty when no body is sent, because the field
    /// describes content that would not be there.
    pub content_presentation: Option<String>,
    /// TXA-4, activity date/time.
    pub activity_datetime: Option<String>,
    /// TXA-5, primary activity provider, as an XCN `id^family^given`.
    pub primary_activity_provider: Option<String>,
    /// TXA-6, origination date/time — when the document came into being.
    pub origination_datetime: Option<String>,
    /// TXA-7, transcription date/time.
    pub transcription_datetime: Option<String>,
    /// TXA-8, edit date/time — the last revision.
    pub edit_datetime: Option<String>,
    /// TXA-9, originator: the document's author, as an XCN.
    pub originator: Option<String>,
    /// TXA-10, assigned document authenticator — who is required to
    /// authenticate it, which is not always who did.
    pub assigned_authenticator: Option<String>,
    /// TXA-11, transcriptionist, as an XCN.
    pub transcriptionist: Option<String>,
    /// TXA-12, unique document number.
    pub unique_document_number: Option<String>,
    /// TXA-13, parent document number — the document an addendum or
    /// amendment extends. TXA-12 is this document's OWN number; a receiver
    /// that confuses the two files an addendum as a standalone document.
    pub parent_document_number: Option<String>,
    /// TXA-16, unique document file name.
    pub document_file_name: Option<String>,
    /// TXA-17, document completion status (table 0271 — `AU`
    /// authenticated, `LA` legally authenticated, `IP` in progress,
    /// `IN` incomplete, `DI` dictated, `DO` documented, `PA`
    /// pre-authenticated).
    pub completion_status: Option<String>,
    /// TXA-18, document confidentiality status (table 0272).
    pub confidentiality_status: Option<String>,
    /// TXA-19, document availability status (table 0273 — `AV`
    /// available, `DE` deleted, `UN` unavailable).
    pub availability_status: Option<String>,
    /// TXA-21, document change reason.
    pub change_reason: Option<String>,
    /// TXA-22 person half: who authenticated the document, as an XCN
    /// `id^family^given`. Renders with [`Self::authenticated_datetime`]
    /// into ONE PPN — see [`Self::to_txa_segment`].
    pub authenticator: Option<String>,
    /// TXA-22 time half: when they authenticated it.
    pub authenticated_datetime: Option<String>,
}

impl DocumentHeader {
    /// Render as TXA, padded to 23 fields so a consumer indexing by
    /// position finds every field where the standard says it is.
    ///
    /// TXA-22 is a **PPN**, not an XCN: the person occupies components 1-3
    /// and the timestamp component **15** (`PPN.15 Date/Time Action
    /// Performed`). Putting it anywhere else names a family name where a
    /// receiver expects a time.
    pub fn to_txa_segment(&self) -> Segment {
        let mut txa = Segment::new("TXA");

        // A caller-assembled `^`-delimited value rendered as components, so
        // an XCN arrives as an XCN rather than as one opaque string.
        let composite = |value: &str| {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            for part in value.split('^') {
                rep.add_component(Component::from_value(part));
            }
            field.add_repetition(rep);
            field
        };
        let opt_composite = |value: &Option<String>| match value {
            Some(v) => composite(v),
            None => Field::from_value(""),
        };
        let plain = |value: &Option<String>| Field::from_value(value.as_deref().unwrap_or(""));

        txa.add_field(Field::from_value("1")); // TXA-1: Set ID
        txa.add_field(plain(&self.document_type)); // TXA-2
        txa.add_field(plain(&self.content_presentation)); // TXA-3
        txa.add_field(plain(&self.activity_datetime)); // TXA-4
        txa.add_field(opt_composite(&self.primary_activity_provider)); // TXA-5
        txa.add_field(plain(&self.origination_datetime)); // TXA-6
        txa.add_field(plain(&self.transcription_datetime)); // TXA-7
        txa.add_field(plain(&self.edit_datetime)); // TXA-8
        txa.add_field(opt_composite(&self.originator)); // TXA-9
        txa.add_field(opt_composite(&self.assigned_authenticator)); // TXA-10
        txa.add_field(opt_composite(&self.transcriptionist)); // TXA-11
        txa.add_field(plain(&self.unique_document_number)); // TXA-12
        txa.add_field(plain(&self.parent_document_number)); // TXA-13
        txa.add_field(Field::from_value("")); // TXA-14: Placer Order Number
        txa.add_field(Field::from_value("")); // TXA-15: Filler Order Number
        txa.add_field(plain(&self.document_file_name)); // TXA-16
        txa.add_field(plain(&self.completion_status)); // TXA-17
        txa.add_field(plain(&self.confidentiality_status)); // TXA-18
        txa.add_field(plain(&self.availability_status)); // TXA-19
        txa.add_field(Field::from_value("")); // TXA-20: Document Storage Status
        txa.add_field(plain(&self.change_reason)); // TXA-21
        txa.add_field(self.to_authentication_ppn()); // TXA-22
        txa.add_field(Field::from_value("")); // TXA-23: Distributed Copies
        txa
    }

    /// TXA-22 as a PPN: the authenticating person in components 1-3 and
    /// the time they acted in component 15.
    ///
    /// The two halves are independent. A person with no timestamp renders
    /// as the person alone rather than as fourteen empty components; a
    /// timestamp with no person still occupies component 15, because
    /// moving it forward would place it where a receiver reads an ID.
    fn to_authentication_ppn(&self) -> Field {
        const DATETIME_COMPONENT: usize = 15;

        let person = self.authenticator.as_deref().unwrap_or("");
        let time = self.authenticated_datetime.as_deref().unwrap_or("");
        if person.is_empty() && time.is_empty() {
            return Field::from_value("");
        }

        let mut rep = Repetition::new();
        if !person.is_empty() {
            for part in person.split('^') {
                rep.add_component(Component::from_value(part));
            }
        }
        if !time.is_empty() {
            while rep.components.len() < DATETIME_COMPONENT - 1 {
                rep.add_component(Component::from_value(""));
            }
            rep.add_component(Component::from_value(time));
        }
        let mut field = Field::new();
        field.add_repetition(rep);
        field
    }
}

/// One allergy, rendered as an AL1 segment.
///
/// AL1 was emitted by NOTHING before this existed. An ADT admitting a
/// patient to another system carried no allergy information at all, so the
/// receiving system's own allergy and interaction checking had nothing to
/// work with — while this product ran its own allergy gate at prescribing
/// time internally. The data was held and simply never sent.
#[derive(Debug, Clone, Default)]
pub struct PatientAllergy {
    /// AL1-2, allergen type (drug / food / environmental).
    pub allergen_type: Option<String>,
    /// AL1-3, the allergen itself. `code` is optional — a free-text
    /// allergen genuinely has none, and inventing one would assert a
    /// precision the record does not have.
    pub allergen_code: Option<String>,
    pub allergen_name: String,
    /// AL1-4, severity (HL7 table 0128: SV / MO / MI).
    pub severity: Option<String>,
    /// AL1-5, the reaction actually observed.
    pub reaction: Option<String>,
    /// AL1-6, identification date (YYYYMMDD).
    pub identification_date: Option<String>,
}

impl PatientAllergy {
    /// Render as AL1. `set_id` is 1-based and must increment per segment.
    pub fn to_al1_segment(&self, set_id: usize) -> Segment {
        let mut al1 = Segment::new("AL1");
        // AL1-1: Set ID.
        al1.add_field(Field::from_value(set_id.to_string()));
        // AL1-2: Allergen Type Code.
        al1.add_field(Field::from_value(
            self.allergen_type.as_deref().unwrap_or(""),
        ));
        // AL1-3: Allergen Code / Mnemonic / Description (CE).
        let mut field = Field::new();
        let mut rep = Repetition::new();
        match &self.allergen_code {
            Some(code) => {
                rep.add_component(Component::from_value(code));
                rep.add_component(Component::from_value(&self.allergen_name));
            }
            // No code: the name goes in component 1 on its own, matching
            // how every other coded field in this library degrades.
            None => rep.add_component(Component::from_value(&self.allergen_name)),
        }
        field.add_repetition(rep);
        al1.add_field(field);
        // AL1-4: Allergy Severity Code.
        al1.add_field(Field::from_value(self.severity.as_deref().unwrap_or("")));
        // AL1-5: Allergy Reaction Code.
        al1.add_field(Field::from_value(self.reaction.as_deref().unwrap_or("")));
        // AL1-6: Identification Date.
        al1.add_field(Field::from_value(
            self.identification_date.as_deref().unwrap_or(""),
        ));
        al1
    }
}

/// One next-of-kin / emergency contact, rendered as an NK1 segment.
///
/// Like AL1, emitted by nothing before this existed: no emergency contact
/// reached any downstream system.
#[derive(Debug, Clone, Default)]
pub struct NextOfKin {
    /// NK1-2, the contact's name as held (a single field, not split —
    /// APAC names often have no reliable family/given boundary).
    pub name: String,
    /// NK1-3, relationship to the patient.
    pub relationship: Option<String>,
    /// NK1-4, address.
    pub address: Option<PatientAddress>,
    /// NK1-5, primary phone. NK1-6, business phone.
    pub phone: Option<String>,
    pub phone_business: Option<String>,
    /// NK1-7, contact role (HL7 table 0131): `EC` emergency contact,
    /// `NOK` next of kin.
    pub contact_role: Option<String>,
    /// NK1-20, the contact's preferred language.
    pub primary_language: Option<CodedValue>,
}

impl NextOfKin {
    /// Render as NK1. `set_id` is 1-based and must increment per segment.
    pub fn to_nk1_segment(&self, set_id: usize) -> Segment {
        let mut nk1 = Segment::new("NK1");
        // NK1-1: Set ID.
        nk1.add_field(Field::from_value(set_id.to_string()));
        // NK1-2: Name (XPN). Held as one string, so it goes in the family
        // component; splitting a name we were given whole would invent a
        // boundary that may not exist.
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.name));
        field.add_repetition(rep);
        nk1.add_field(field);
        // NK1-3: Relationship.
        nk1.add_field(Field::from_value(
            self.relationship.as_deref().unwrap_or(""),
        ));
        // NK1-4: Address (XAD).
        match &self.address {
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
                nk1.add_field(field);
            }
            None => nk1.add_field(Field::from_value("")),
        }
        // NK1-5: Phone Number. NK1-6: Business Phone.
        nk1.add_field(Field::from_value(self.phone.as_deref().unwrap_or("")));
        nk1.add_field(Field::from_value(
            self.phone_business.as_deref().unwrap_or(""),
        ));
        // NK1-7: Contact Role.
        nk1.add_field(Field::from_value(
            self.contact_role.as_deref().unwrap_or(""),
        ));
        // NK1-8 .. NK1-19 — no source.
        for _ in 0..12 {
            nk1.add_field(Field::from_value(""));
        }
        // NK1-20: Primary Language.
        match &self.primary_language {
            Some(v) => nk1.add_field(v.to_field()),
            None => nk1.add_field(Field::from_value("")),
        }
        nk1
    }
}

/// Full patient demographics (PID segment) — reused by every message
/// type that carries a patient (ADT / ORM / ORU / SIU / MDM / VXU /
/// DFT / REF / BAR).
#[derive(Debug, Clone, Default)]
pub struct PatientDemographics {
    pub patient_id_flat: Option<String>,
    pub identifier: Option<PatientIdentifier>,
    /// Additional typed identifiers, emitted as further PID-3 repetitions
    /// after `identifier`. A national ID (NRIC / HKID / MyKad / Aadhaar)
    /// belongs here with `identifier_type_code: "NI"`, NOT in PID-19 —
    /// PID-19 is the SSN field, and labelling a national ID as an SSN
    /// misrepresents what the number is to every downstream reader.
    pub additional_identifiers: Vec<PatientIdentifier>,
    pub name: Option<(String, String)>, // (family, given)
    /// XPN-3 middle name and XPN-4 suffix, emitted inside PID-5.
    pub middle_name: Option<String>,
    pub name_suffix: Option<String>,
    pub date_of_birth: Option<String>,
    pub sex: Option<String>,
    /// PID-9. In APAC this is usually the local-script name (e.g. 陳大文)
    /// rather than a nickname, which is why it is worth sending.
    pub alias: Option<String>,
    /// PID-10 repeats: a patient may hold more than one race.
    pub race: Vec<CodedValue>,
    pub address: Option<PatientAddress>,
    pub phone_home: Option<String>,
    pub phone_business: Option<String>,
    pub primary_language: Option<CodedValue>,
    pub marital_status: Option<CodedValue>,
    pub religion: Option<CodedValue>,
    /// PID-20, DLN: number ^ (issuing state/province in component 2).
    pub driver_license: Option<(String, Option<String>)>,
    /// PID-22 repeats, like PID-10.
    pub ethnic_group: Vec<CodedValue>,
    pub nationality: Option<CodedValue>,
    /// PID-29. Sent only alongside `death_indicator`.
    pub death_datetime: Option<String>,
    /// PID-30. `Some(true)` renders `Y`, `Some(false)` renders `N`, and
    /// `None` leaves the field empty — "we are not asserting either way"
    /// is a different statement from "this patient is alive".
    pub death_indicator: Option<bool>,
    /// PID-33, last update date/time for this demographic record.
    pub last_update_datetime: Option<String>,
}

impl PatientDemographics {
    /// Render the patient into a PID segment (PID-1 .. PID-13).
    /// Positional empties between populated fields keep downstream
    /// indexers aligned.
    pub fn to_pid_segment(&self) -> Segment {
        let mut pid = Segment::new("PID");
        // PID-1: Set ID.
        pid.add_field(Field::from_value("1"));
        // PID-2: Patient ID (External) — empty.
        pid.add_field(Field::from_value(""));
        // PID-3: Patient Identifier List (CX when structured, else flat).
        if let Some(ident) = &self.identifier {
            let mut field = Field::new();
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
            field.add_repetition(rep);
            // Further typed identifiers ride as additional repetitions. The
            // primary identifier stays FIRST so readers that take PID-3-1
            // (ours included) keep seeing the MRN.
            for extra in &self.additional_identifiers {
                let mut rep = Repetition::new();
                rep.add_component(Component::from_value(&extra.id));
                rep.add_component(Component::from_value(
                    extra.check_digit.as_deref().unwrap_or(""),
                ));
                rep.add_component(Component::from_value(
                    extra.check_digit_scheme.as_deref().unwrap_or(""),
                ));
                rep.add_component(Component::from_value(
                    extra.assigning_authority.as_deref().unwrap_or(""),
                ));
                rep.add_component(Component::from_value(
                    extra.identifier_type_code.as_deref().unwrap_or(""),
                ));
                field.add_repetition(rep);
            }
            pid.add_field(field);
        } else if let Some(id) = &self.patient_id_flat {
            pid.add_field(Field::from_value(id));
        } else {
            pid.add_field(Field::from_value(""));
        }
        // PID-4: Alternate Patient ID — empty.
        pid.add_field(Field::from_value(""));
        // PID-5: Patient Name (XPN).
        if let Some((family, given)) = &self.name {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(family));
            rep.add_component(Component::from_value(given));
            // XPN-3 middle, XPN-4 suffix. Emitted only when present so a
            // name with neither still renders as two components.
            if self.middle_name.is_some() || self.name_suffix.is_some() {
                rep.add_component(Component::from_value(
                    self.middle_name.as_deref().unwrap_or(""),
                ));
                rep.add_component(Component::from_value(
                    self.name_suffix.as_deref().unwrap_or(""),
                ));
            }
            field.add_repetition(rep);
            pid.add_field(field);
        } else {
            pid.add_field(Field::from_value(""));
        }
        // PID-6: Mother's Maiden Name — empty.
        pid.add_field(Field::from_value(""));
        // PID-7: Date of Birth.
        pid.add_field(Field::from_value(
            self.date_of_birth.as_deref().unwrap_or(""),
        ));
        // PID-8: Sex.
        pid.add_field(Field::from_value(self.sex.as_deref().unwrap_or("")));
        // PID-9: Patient Alias. In APAC this is the local-script name.
        match &self.alias {
            Some(alias) => {
                let mut field = Field::new();
                let mut rep = Repetition::new();
                rep.add_component(Component::from_value(alias));
                field.add_repetition(rep);
                pid.add_field(field);
            }
            None => pid.add_field(Field::from_value("")),
        }
        // PID-10: Race. Repeating -- every value is emitted, `~`-separated.
        if self.race.is_empty() {
            pid.add_field(Field::from_value(""));
        } else {
            let mut field = Field::new();
            for v in &self.race {
                field.add_repetition(v.to_repetition());
            }
            pid.add_field(field);
        }
        // PID-11: Address (XAD).
        if let Some(addr) = &self.address {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(&addr.line1));
            rep.add_component(Component::from_value(&addr.line2));
            rep.add_component(Component::from_value(&addr.city));
            rep.add_component(Component::from_value(&addr.state));
            rep.add_component(Component::from_value(&addr.postal_code));
            rep.add_component(Component::from_value(&addr.country));
            field.add_repetition(rep);
            pid.add_field(field);
        } else {
            pid.add_field(Field::from_value(""));
        }
        // PID-12: County Code — empty.
        pid.add_field(Field::from_value(""));
        // PID-13: Phone — Home.
        pid.add_field(Field::from_value(self.phone_home.as_deref().unwrap_or("")));
        // PID-14: Phone — Business.
        pid.add_field(Field::from_value(
            self.phone_business.as_deref().unwrap_or(""),
        ));
        // PID-15: Primary Language.
        match &self.primary_language {
            Some(v) => pid.add_field(v.to_field()),
            None => pid.add_field(Field::from_value("")),
        }
        // PID-16: Marital Status.
        match &self.marital_status {
            Some(v) => pid.add_field(v.to_field()),
            None => pid.add_field(Field::from_value("")),
        }
        // PID-17: Religion.
        match &self.religion {
            Some(v) => pid.add_field(v.to_field()),
            None => pid.add_field(Field::from_value("")),
        }
        // PID-18: Patient Account Number — empty.
        pid.add_field(Field::from_value(""));
        // PID-19: SSN — DELIBERATELY EMPTY. A national ID (NRIC / HKID /
        // MyKad / Aadhaar) is not a Social Security Number; it travels as a
        // typed PID-3 repetition instead. See `additional_identifiers`.
        pid.add_field(Field::from_value(""));
        // PID-20: Driver's License Number (DLN: number ^ issuing state).
        match &self.driver_license {
            Some((number, state)) => {
                let mut field = Field::new();
                let mut rep = Repetition::new();
                rep.add_component(Component::from_value(number));
                rep.add_component(Component::from_value(state.as_deref().unwrap_or("")));
                field.add_repetition(rep);
                pid.add_field(field);
            }
            None => pid.add_field(Field::from_value("")),
        }
        // PID-21: Mother's Identifier — empty.
        pid.add_field(Field::from_value(""));
        // PID-22: Ethnic Group. Repeating, like PID-10.
        if self.ethnic_group.is_empty() {
            pid.add_field(Field::from_value(""));
        } else {
            let mut field = Field::new();
            for v in &self.ethnic_group {
                field.add_repetition(v.to_repetition());
            }
            pid.add_field(field);
        }
        // PID-23 Birth Place, PID-24 Multiple Birth Indicator,
        // PID-25 Birth Order, PID-26 Citizenship, PID-27 Veterans Military
        // Status — no source for any of these.
        //
        // PID-26 is left empty deliberately: citizenship is not the same
        // assertion as nationality, and we hold only the latter.
        for _ in 0..5 {
            pid.add_field(Field::from_value(""));
        }
        // PID-28: Nationality.
        match &self.nationality {
            Some(v) => pid.add_field(v.to_field()),
            None => pid.add_field(Field::from_value("")),
        }
        // PID-29: Patient Death Date and Time.
        pid.add_field(Field::from_value(
            self.death_datetime.as_deref().unwrap_or(""),
        ));
        // PID-30: Patient Death Indicator.
        pid.add_field(Field::from_value(match self.death_indicator {
            Some(true) => "Y",
            Some(false) => "N",
            None => "",
        }));
        // PID-31 Identity Unknown Indicator, PID-32 Identity Reliability
        // Code — no source.
        pid.add_field(Field::from_value(""));
        pid.add_field(Field::from_value(""));
        // PID-33: Last Update Date/Time.
        pid.add_field(Field::from_value(
            self.last_update_datetime.as_deref().unwrap_or(""),
        ));
        pid
    }
}

/// Base message builder with common functionality
pub struct MessageBuilder {
    message: Message,
    version: Version,
    message_type: String,
    trigger_event: String,
    message_structure: Option<String>,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new(version: Version, message_type: &str, trigger_event: &str) -> Self {
        Self {
            message: Message::new(),
            version,
            message_type: message_type.to_string(),
            trigger_event: trigger_event.to_string(),
            message_structure: None,
        }
    }

    /// Set the MSH-9 message structure (third component of the MSG
    /// data type — e.g. "ADT_A01" for an ADT^A01 message). Required
    /// by HL7 v2.5+ and by partner systems that route on the full
    /// triple. When unset, MSH-9 remains the two-component
    /// `message_type^trigger_event` form.
    pub fn message_structure(&mut self, structure: &str) -> &mut Self {
        self.message_structure = Some(structure.to_string());
        self
    }

    /// Override the trigger event after construction. Lets a single
    /// specialised builder (e.g. [`super::adt::AdtA01Builder`]) emit
    /// variant messages (A02/A03/…) without rebuilding the base.
    pub fn set_trigger_event(&mut self, trigger: &str) -> &mut Self {
        self.trigger_event = trigger.to_string();
        self
    }

    /// Override the message type after construction. Symmetric to
    /// [`Self::set_trigger_event`]; lets a single specialised builder emit
    /// related response messages with a different MSH-9-1 (e.g. a
    /// REF^I12 builder emitting RRI^I12 by flipping "REF" → "RRI"
    /// while reusing all the segment-building logic).
    pub fn set_message_type(&mut self, message_type: &str) -> &mut Self {
        self.message_type = message_type.to_string();
        self
    }

    /// Current trigger event (e.g. "A01"). Used by specialised
    /// builders that create variant-specific EVN segments.
    pub fn trigger_event_str(&self) -> &str {
        &self.trigger_event
    }

    /// Create MSH segment with basic fields
    pub fn create_msh(
        &self,
        sending_app: &str,
        sending_facility: &str,
        receiving_app: &str,
        receiving_facility: &str,
        control_id: &str,
        processing_id: &str,
    ) -> Result<Segment> {
        let mut msh = Segment::new("MSH");
        let delims = Delimiters::default();

        // MSH-1: Field separator (special handling)
        msh.add_field(Field::from_value(delims.field_separator.to_string()));

        // MSH-2: Encoding characters
        msh.add_field(Field::from_value(format!(
            "{}{}{}{}",
            delims.component_separator,
            delims.repetition_separator,
            delims.escape_character,
            delims.subcomponent_separator
        )));

        // MSH-3: Sending Application
        msh.add_field(Field::from_value(sending_app));

        // MSH-4: Sending Facility
        msh.add_field(Field::from_value(sending_facility));

        // MSH-5: Receiving Application
        msh.add_field(Field::from_value(receiving_app));

        // MSH-6: Receiving Facility
        msh.add_field(Field::from_value(receiving_facility));

        // MSH-7: Date/Time of Message.
        //
        // UTC, not `Local`, for the same reason EVN-2 is: every other
        // timestamp in the message is rendered from a `DateTime<Utc>`, and an
        // HL7 timestamp with no offset carries no zone — so a receiver
        // computing message age against a local-time MSH-7 was out by the
        // sending host's offset with nothing on the wire to say so.
        let timestamp = format_timestamp(&Utc::now().naive_utc());
        msh.add_field(Field::from_value(&timestamp));

        // MSH-8: Security (empty)
        msh.add_field(Field::from_value(""));

        // MSH-9: Message Type (MSG data type - proper component structure).
        // Emits the third component (message structure) when the
        // builder has it set — required by partner systems that route
        // on the full ADT_A01 / ORM_O01 triple.
        let mut msg_type_field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.message_type));
        rep.add_component(Component::from_value(&self.trigger_event));
        if let Some(structure) = &self.message_structure {
            rep.add_component(Component::from_value(structure));
        }
        msg_type_field.add_repetition(rep);
        msh.add_field(msg_type_field);

        // MSH-10: Message Control ID
        msh.add_field(Field::from_value(control_id));

        // MSH-11: Processing ID
        msh.add_field(Field::from_value(processing_id));

        // MSH-12: Version ID
        msh.add_field(Field::from_value(self.version.as_str()));

        // MSH-13 .. MSH-17 — no source. Emitted EMPTY rather than omitted,
        // because MSH-18 below has to land at position 18 for a receiver
        // indexing by position to find it.
        //
        // MSH-15 / MSH-16 (accept- and application-acknowledgment type) are
        // deliberately left empty here and NOT derived from the integration's
        // outbound ACK policy. That is a separate change with a design
        // question attached: `OutboundAckPolicy::HonorMsh16` would then be
        // reading back a value this builder had just stamped, which is a
        // category error — on an OUTBOUND message MSH-15/16 state what WE
        // request, not what a partner told us. Left as it was so this change
        // alters no ACK behaviour.
        for _ in 13..=17 {
            msh.add_field(Field::from_value(""));
        }

        // MSH-18: Character Set (HL7 table 0211).
        //
        // This segment used to stop at MSH-12, so every message this product
        // has ever sent carried UTF-8 bytes while declaring no character set
        // at all. HL7 v2.5 §2.16.9 is explicit that an absent MSH-18 means the
        // receiver applies ITS OWN default — normally the 7-bit set — so a
        // conformant partner was entitled to mis-decode every non-ASCII name
        // we sent, and we had no standing to call it wrong.
        //
        // That is not a hypothetical for an APAC-first product: its own
        // patients are named 李小明, 陳大文 and 田中太郎.
        //
        // Hard-coded rather than configurable BECAUSE it is not a preference.
        // Rust strings are UTF-8 and the MLLP layer writes those bytes
        // directly, so `UNICODE UTF-8` is a statement of what the encoder
        // actually does. A configurable value could only ever be a way to
        // declare something untrue.
        msh.add_field(Field::from_value("UNICODE UTF-8"));

        Ok(msh)
    }

    /// Create EVN segment
    pub fn create_evn(
        &self,
        event_type_code: &str,
        recorded_datetime: Option<&str>,
    ) -> Result<Segment> {
        let mut evn = Segment::new("EVN");

        // EVN-1: Event Type Code
        evn.add_field(Field::from_value(event_type_code));

        // EVN-2: Recorded Date/Time — when the transaction was ENTERED. The
        // standard says "most systems will default to the system date/time
        // when the transaction was entered", which is what this does.
        //
        // UTC, not `Local`: every other timestamp in the message is rendered
        // from a `DateTime<Utc>`, so a local-time EVN-2 made a message
        // disagree with itself by the host's offset — eight hours in the
        // deployments this product targets — and the receiver has no way to
        // tell which fields are in which zone.
        let timestamp = if let Some(dt) = recorded_datetime {
            dt.to_string()
        } else {
            format_timestamp(&Utc::now().naive_utc())
        };
        evn.add_field(Field::from_value(&timestamp));

        Ok(evn)
    }

    /// EVN with `EVN-6` — when the event actually OCCURRED.
    ///
    /// `EVN-2` answers a different question (when the transaction was
    /// recorded) and a receiver reading it as the event time gets the moment
    /// the message was built. Emitting nothing in `EVN-6` left every receiver
    /// with only `EVN-2` and `PV1-44`/`PV1-45` to choose between, and our own
    /// reader chose `EVN-2`.
    pub fn create_evn_with_occurrence(
        &self,
        event_type_code: &str,
        recorded_datetime: Option<&str>,
        event_occurred: Option<&str>,
    ) -> Result<Segment> {
        let mut evn = self.create_evn(event_type_code, recorded_datetime)?;
        // EVN-3 .. EVN-5 have no source here; EVN-6 is the point of this.
        for _ in 3..=5 {
            evn.add_field(Field::from_value(""));
        }
        evn.add_field(Field::from_value(event_occurred.unwrap_or("")));
        Ok(evn)
    }

    /// Build the final message
    pub fn build(self) -> Message {
        self.message
    }
}

/// Generate a message control ID (simple implementation)
pub fn generate_control_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("MSG{}", timestamp)
}

#[cfg(test)]
mod timestamp_zone_tests {
    use super::*;
    use crate::Version;

    /// `MSH-7` and `EVN-2` are the only two timestamps this library mints
    /// itself, and they were minted from `chrono::Local` while every other
    /// timestamp in every message is rendered from a `DateTime<Utc>`.
    ///
    /// An HL7 timestamp with no offset suffix carries no zone, so a receiver
    /// has nothing on the wire that says which of the two rules applies to
    /// which field. On the deployments this product targets — UTC+8 — a
    /// message therefore disagreed with itself by eight hours: `MSH-7` said
    /// the message was built eight hours after the admission `PV1-44`
    /// reported, and a receiver computing message age or ordering a feed by
    /// `MSH-7` was wrong by the host's offset.
    ///
    /// ⚠️ **This guard only discriminates on a host whose local time is not
    /// UTC.** On a UTC machine — which CI is — `Local` and `Utc` agree and
    /// the assertion is vacuous. It is kept because every developer machine
    /// and every deployment this product targets IS offset, so the guard
    /// fires exactly where the defect is reachable. A guard that is vacuous
    /// in CI and live on a dev box is worth more than none; do not read a
    /// green CI run as evidence that this passed.
    #[test]
    fn the_timestamps_this_library_mints_itself_are_utc() {
        let builder = MessageBuilder::new(Version::V2_5, "ADT", "A01");
        let now = Utc::now().naive_utc();

        let msh = builder
            .create_msh("APP", "FAC", "RAPP", "RFAC", "CTRL1", "P")
            .expect("MSH");
        let msh7 = msh
            .get_field(7)
            .and_then(|f| f.get_component(0, 0))
            .and_then(|c| c.value())
            .expect("MSH-7 is populated")
            .to_string();

        let evn = builder.create_evn("A01", None).expect("EVN");
        let evn2 = evn
            .get_field(2)
            .and_then(|f| f.get_component(0, 0))
            .and_then(|c| c.value())
            .expect("EVN-2 is populated")
            .to_string();

        // Both minted from the same clock a moment ago, so a whole minute of
        // slack is generous and an offset of hours cannot hide inside it.
        for (name, rendered) in [("MSH-7", &msh7), ("EVN-2", &evn2)] {
            let parsed = chrono::NaiveDateTime::parse_from_str(rendered, "%Y%m%d%H%M%S")
                .unwrap_or_else(|e| panic!("{name} must be YYYYMMDDHHMMSS, got {rendered}: {e}"));
            let drift = (parsed - now).num_seconds().abs();
            assert!(
                drift < 60,
                "{name} must be UTC, like every other timestamp in the message. \
                 Rendered {rendered}, UTC now is {now}, {drift}s apart — an offset \
                 of hours means it was minted from the host's LOCAL clock."
            );
        }
    }
}
