//! ADT (Admit/Discharge/Transfer) message builders

use super::{generate_control_id, MessageBuilder};
use crate::{
    error::Result,
    field::{Component, Field, Repetition},
    message::Message,
    segment::Segment,
    Version,
};

/// Builder for ADT messages
pub struct AdtBuilder {
    #[allow(dead_code)]
    base: MessageBuilder,
}

impl AdtBuilder {
    /// Create a new ADT message builder
    pub fn new(version: Version, trigger_event: &str) -> Self {
        Self {
            base: MessageBuilder::new(version, "ADT", trigger_event),
        }
    }

    /// Create ADT^A01 - Admit/Visit Notification
    pub fn a01(version: Version) -> AdtA01Builder {
        AdtA01Builder::new(version)
    }

    /// Create ADT^A02 - Transfer a Patient
    pub fn a02(version: Version) -> AdtA02Builder {
        AdtA02Builder::new(version)
    }

    /// Create ADT^A03 - Discharge/End Visit
    pub fn a03(version: Version) -> AdtA03Builder {
        AdtA03Builder::new(version)
    }

    /// Create ADT^A04 - Register a Patient
    pub fn a04(version: Version) -> AdtA04Builder {
        AdtA04Builder::new(version)
    }

    /// Create ADT^A05 - Pre-admit a Patient
    pub fn a05(version: Version) -> AdtA05Builder {
        AdtA05Builder::new(version)
    }

    /// Create ADT^A06 - Change Outpatient to Inpatient
    pub fn a06(version: Version) -> AdtA06Builder {
        AdtA06Builder::new(version)
    }

    /// Create ADT^A07 - Change Inpatient to Outpatient
    pub fn a07(version: Version) -> AdtA07Builder {
        AdtA07Builder::new(version)
    }

    /// Create ADT^A08 - Update Patient Information
    pub fn a08(version: Version) -> AdtA08Builder {
        AdtA08Builder::new(version)
    }

    /// Create ADT^A11 - Cancel Admit/Visit Notification
    pub fn a11(version: Version) -> AdtA11Builder {
        AdtA11Builder::new(version)
    }

    /// Create ADT^A12 - Cancel Transfer
    pub fn a12(version: Version) -> AdtA12Builder {
        AdtA12Builder::new(version)
    }

    /// Create ADT^A13 - Cancel Discharge/End Visit
    pub fn a13(version: Version) -> AdtA13Builder {
        AdtA13Builder::new(version)
    }

    /// Create ADT^A09 - Patient Departing - Tracking
    pub fn a09(version: Version) -> AdtA09Builder {
        AdtA09Builder::new(version)
    }

    /// Create ADT^A10 - Patient Arriving - Tracking
    pub fn a10(version: Version) -> AdtA10Builder {
        AdtA10Builder::new(version)
    }

    /// Create ADT^A17 - Swap Patients
    pub fn a17(version: Version) -> AdtA17Builder {
        AdtA17Builder::new(version)
    }

    /// Create ADT^A28 - Add Person Information
    pub fn a28(version: Version) -> AdtA28Builder {
        AdtA28Builder::new(version)
    }

    /// Create ADT^A31 - Update Person Information
    pub fn a31(version: Version) -> AdtA31Builder {
        AdtA31Builder::new(version)
    }

    /// Create ADT^A40 - Merge Patient - Patient Identifier List
    pub fn a40(version: Version) -> AdtA40Builder {
        AdtA40Builder::new(version)
    }

    /// Create ADT^A34 - Merge Patient - Patient ID Only
    pub fn a34(version: Version) -> AdtA34Builder {
        AdtA34Builder::new(version)
    }
}

/// Builder for ADT^A01 - Admit/Visit Notification
pub struct AdtA01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>, // (family, given)
    dob: Option<String>,
    sex: Option<String>,
    patient_class: Option<String>,
    assigned_location: Option<String>,
    attending_doctor: Option<String>,
    admit_datetime: Option<String>,
}

impl AdtA01Builder {
    /// Create a new ADT^A01 builder
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "ADT", "A01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            patient_id: None,
            patient_name: None,
            dob: None,
            sex: None,
            patient_class: None,
            assigned_location: None,
            attending_doctor: None,
            admit_datetime: None,
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
        self.patient_id = Some(id.to_string());
        self
    }

    /// Set patient name
    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.patient_name = Some((family.to_string(), given.to_string()));
        self
    }

    /// Set date of birth (format: YYYYMMDD)
    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.dob = Some(dob.to_string());
        self
    }

    /// Set administrative sex
    pub fn sex(mut self, sex: &str) -> Self {
        self.sex = Some(sex.to_string());
        self
    }

    /// Set patient class (I=Inpatient, O=Outpatient, E=Emergency, etc.)
    pub fn patient_class(mut self, class: &str) -> Self {
        self.patient_class = Some(class.to_string());
        self
    }

    /// Set assigned patient location
    pub fn assigned_location(mut self, location: &str) -> Self {
        self.assigned_location = Some(location.to_string());
        self
    }

    /// Set attending doctor
    pub fn attending_doctor(mut self, doctor: &str) -> Self {
        self.attending_doctor = Some(doctor.to_string());
        self
    }

    /// Set admit date/time
    pub fn admit_datetime(mut self, datetime: &str) -> Self {
        self.admit_datetime = Some(datetime.to_string());
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

        // Create EVN segment
        let evn = self.base.create_evn("A01", None)?;
        self.base.message.add_segment(evn);

        // Create PID segment
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1")); // PID-1: Set ID

        // PID-2: Patient ID (External) - often empty
        pid.add_field(Field::from_value(""));

        // PID-3: Patient Identifier List
        if let Some(id) = &self.patient_id {
            pid.add_field(Field::from_value(id));
        } else {
            pid.add_field(Field::from_value(""));
        }

        // PID-4: Alternate Patient ID - empty
        pid.add_field(Field::from_value(""));

        // PID-5: Patient Name (XPN data type - proper component structure)
        if let Some((family, given)) = &self.patient_name {
            let mut field = Field::new();
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(family));
            rep.add_component(Component::from_value(given));
            field.add_repetition(rep);
            pid.add_field(field);
        } else {
            pid.add_field(Field::from_value(""));
        }

        // PID-6: Mother's Maiden Name - empty
        pid.add_field(Field::from_value(""));

        // PID-7: Date of Birth
        pid.add_field(Field::from_value(self.dob.as_deref().unwrap_or("")));

        // PID-8: Sex
        pid.add_field(Field::from_value(self.sex.as_deref().unwrap_or("")));

        self.base.message.add_segment(pid);

        // Create PV1 segment if patient class is provided
        if let Some(class) = &self.patient_class {
            let mut pv1 = Segment::new("PV1");
            pv1.add_field(Field::from_value("1")); // PV1-1: Set ID
            pv1.add_field(Field::from_value(class)); // PV1-2: Patient Class

            // PV1-3: Assigned Patient Location (PL data type - proper component structure)
            if let Some(location) = &self.assigned_location {
                // Parse location if it contains ^ separator, otherwise treat as single value
                let parts: Vec<&str> = location.split('^').collect();
                let mut field = Field::new();
                let mut rep = Repetition::new();
                for part in parts {
                    rep.add_component(Component::from_value(part));
                }
                field.add_repetition(rep);
                pv1.add_field(field);
            } else {
                pv1.add_field(Field::from_value(""));
            }

            // PV1-4: Admission Type - empty for now
            pv1.add_field(Field::from_value(""));

            // PV1-5: Preadmit Number - empty
            pv1.add_field(Field::from_value(""));

            // PV1-6: Prior Patient Location - empty
            pv1.add_field(Field::from_value(""));

            // PV1-7: Attending Doctor (XCN data type - proper component structure)
            if let Some(doctor) = &self.attending_doctor {
                // Parse doctor if it contains ^ separator, otherwise treat as single value
                let parts: Vec<&str> = doctor.split('^').collect();
                let mut field = Field::new();
                let mut rep = Repetition::new();
                for part in parts {
                    rep.add_component(Component::from_value(part));
                }
                field.add_repetition(rep);
                pv1.add_field(field);
            } else {
                pv1.add_field(Field::from_value(""));
            }

            self.base.message.add_segment(pv1);
        }

        Ok(self.base.build())
    }
}

/// Builder for ADT^A02 - Transfer a Patient
pub struct AdtA02Builder {
    base: AdtA01Builder,
}

impl AdtA02Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A02");
        Self { base }
    }

    // Delegate all methods to AdtA01Builder
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
pub struct AdtA03Builder {
    base: AdtA01Builder,
}

impl AdtA03Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A03");
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
pub struct AdtA04Builder {
    base: AdtA01Builder,
}

impl AdtA04Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A04");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A08 - Update Patient Information
pub struct AdtA08Builder {
    base: AdtA01Builder,
}

impl AdtA08Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A08");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A05 - Pre-admit a Patient
pub struct AdtA05Builder {
    base: AdtA01Builder,
}

impl AdtA05Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A05");
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
pub struct AdtA06Builder {
    base: AdtA01Builder,
}

impl AdtA06Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A06");
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
pub struct AdtA07Builder {
    base: AdtA01Builder,
}

impl AdtA07Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A07");
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
pub struct AdtA11Builder {
    base: AdtA01Builder,
}

impl AdtA11Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A11");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A12 - Cancel Transfer
pub struct AdtA12Builder {
    base: AdtA01Builder,
}

impl AdtA12Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A12");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A13 - Cancel Discharge/End Visit
pub struct AdtA13Builder {
    base: AdtA01Builder,
}

impl AdtA13Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A13");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A09 - Patient Departing - Tracking
pub struct AdtA09Builder {
    base: AdtA01Builder,
}

impl AdtA09Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A09");
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
pub struct AdtA10Builder {
    base: AdtA01Builder,
}

impl AdtA10Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A10");
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
pub struct AdtA17Builder {
    base: AdtA01Builder,
}

impl AdtA17Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A17");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A28 - Add Person Information
pub struct AdtA28Builder {
    base: AdtA01Builder,
}

impl AdtA28Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A28");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A31 - Update Person Information
pub struct AdtA31Builder {
    base: AdtA01Builder,
}

impl AdtA31Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A31");
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for ADT^A40 - Merge Patient - Patient Identifier List
pub struct AdtA40Builder {
    base: AdtA01Builder,
    prior_patient_ids: Vec<String>,
    prior_patient_names: Vec<(String, String)>,
}

impl AdtA40Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A40");
        Self {
            base,
            prior_patient_ids: Vec::new(),
            prior_patient_names: Vec::new(),
        }
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

    pub fn control_id(mut self, id: &str) -> Self {
        self.base = self.base.control_id(id);
        self
    }

    pub fn processing_id(mut self, id: &str) -> Self {
        self.base = self.base.processing_id(id);
        self
    }

    /// Set the surviving (correct) patient ID (PID-3).
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

    /// Set the patient class (PV1-2, required by the A40 schema).
    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    /// Add a deprecated patient ID being merged away (MRG-1 repetition).
    /// Repeat for identifier-list merges; at least one is schema-required.
    pub fn add_prior_patient_id(mut self, id: &str) -> Self {
        self.prior_patient_ids.push(id.to_string());
        self
    }

    /// Add a deprecated patient name (MRG-7 repetition, family/given).
    pub fn add_prior_patient_name(mut self, family: &str, given: &str) -> Self {
        self.prior_patient_names
            .push((family.to_string(), given.to_string()));
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self
            .base
            .control_id
            .take()
            .unwrap_or_else(generate_control_id);

        let msh = self.base.base.create_msh(
            &self.base.sending_app,
            &self.base.sending_facility,
            &self.base.receiving_app,
            &self.base.receiving_facility,
            &control_id,
            &self.base.processing_id,
        )?;
        self.base.base.message.add_segment(msh);

        let evn = self.base.base.create_evn("A40", None)?;
        self.base.base.message.add_segment(evn);

        // PID segment, sparse: unset fields stay absent instead of ""
        // placeholders, so optional composite fields do not trip
        // required-component checks during validation.
        let mut pid = Segment::new("PID");
        set_field(&mut pid, 1, "1");
        if let Some(id) = self.base.patient_id.as_deref() {
            set_field(&mut pid, 3, id);
        }
        if let Some((family, given)) = self.base.patient_name.as_ref() {
            set_field(&mut pid, 5, &format!("{family}^{given}"));
        }
        if let Some(dob) = self.base.dob.as_deref() {
            set_field(&mut pid, 7, dob);
        }
        if let Some(sex) = self.base.sex.as_deref() {
            set_field(&mut pid, 8, sex);
        }
        self.base.base.message.add_segment(pid);

        // PV1 segment (PV1-2 patient class is schema-required)
        let mut pv1 = Segment::new("PV1");
        set_field(&mut pv1, 1, "1");
        set_field(
            &mut pv1,
            2,
            self.base.patient_class.as_deref().unwrap_or(""),
        );
        self.base.base.message.add_segment(pv1);

        // MRG segment (MRG-1 prior patient ID list is schema-required)
        let mut mrg = Segment::new("MRG");
        if self.prior_patient_ids.is_empty() {
            set_field(&mut mrg, 1, "");
        } else {
            let mut field = Field::new();
            for id in &self.prior_patient_ids {
                let mut rep = Repetition::new();
                rep.add_component(Component::from_value(id));
                field.add_repetition(rep);
            }
            let _ = mrg.set_field(1, field);
        }
        if !self.prior_patient_names.is_empty() {
            let mut field = Field::new();
            for (family, given) in &self.prior_patient_names {
                let mut rep = Repetition::new();
                rep.add_component(Component::from_value(family));
                rep.add_component(Component::from_value(given));
                field.add_repetition(rep);
            }
            let _ = mrg.set_field(7, field);
        }
        self.base.base.message.add_segment(mrg);

        Ok(self.base.base.build())
    }
}

/// Set a segment field, ignoring the impossible index-0 error.
fn set_field(segment: &mut Segment, index: usize, value: &str) {
    let _ = segment.set_field(index, Field::from_value(value));
}

/// Builder for ADT^A34 - Merge Patient - Patient ID Only
///
/// Unlike the other ADT wrappers, this builder has its own `build`: A34
/// carries no visit segments, and it must emit the required MRG segment
/// holding the prior (deprecated) patient identifiers.
pub struct AdtA34Builder {
    base: AdtA01Builder,
    prior_patient_id: Option<String>,
    prior_patient_name: Option<(String, String)>,
}

impl AdtA34Builder {
    pub fn new(version: Version) -> Self {
        let mut base = AdtA01Builder::new(version);
        base.base = MessageBuilder::new(version, "ADT", "A34");
        Self {
            base,
            prior_patient_id: None,
            prior_patient_name: None,
        }
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

    pub fn control_id(mut self, id: &str) -> Self {
        self.base = self.base.control_id(id);
        self
    }

    pub fn processing_id(mut self, id: &str) -> Self {
        self.base = self.base.processing_id(id);
        self
    }

    /// Set the surviving (correct) patient ID (PID-3).
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

    /// Set the deprecated patient ID being merged away (MRG-1, required).
    pub fn prior_patient_id(mut self, id: &str) -> Self {
        self.prior_patient_id = Some(id.to_string());
        self
    }

    pub fn prior_patient_name(mut self, family: &str, given: &str) -> Self {
        self.prior_patient_name = Some((family.to_string(), given.to_string()));
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self
            .base
            .control_id
            .take()
            .unwrap_or_else(generate_control_id);

        let msh = self.base.base.create_msh(
            &self.base.sending_app,
            &self.base.sending_facility,
            &self.base.receiving_app,
            &self.base.receiving_facility,
            &control_id,
            &self.base.processing_id,
        )?;
        self.base.base.message.add_segment(msh);

        let evn = self.base.base.create_evn("A34", None)?;
        self.base.base.message.add_segment(evn);

        // PID segment, sparse: unset fields stay absent instead of ""
        // placeholders, so optional composite fields do not trip
        // required-component checks during validation.
        let mut pid = Segment::new("PID");
        set_field(&mut pid, 1, "1");
        if let Some(id) = self.base.patient_id.as_deref() {
            set_field(&mut pid, 3, id);
        }
        if let Some((family, given)) = self.base.patient_name.as_ref() {
            set_field(&mut pid, 5, &format!("{family}^{given}"));
        }
        if let Some(dob) = self.base.dob.as_deref() {
            set_field(&mut pid, 7, dob);
        }
        if let Some(sex) = self.base.sex.as_deref() {
            set_field(&mut pid, 8, sex);
        }
        self.base.base.message.add_segment(pid);

        // MRG segment (MRG-1 prior patient ID is schema-required)
        let mut mrg = Segment::new("MRG");
        set_field(&mut mrg, 1, self.prior_patient_id.as_deref().unwrap_or(""));
        if let Some((family, given)) = self.prior_patient_name.as_ref() {
            set_field(&mut mrg, 7, &format!("{family}^{given}"));
        }
        self.base.base.message.add_segment(mrg);

        Ok(self.base.base.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_builder() -> AdtA34Builder {
        AdtBuilder::a34(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .control_id("MSG001")
            .patient_id("NEW123")
            .patient_name("Smith", "John")
            .prior_patient_id("OLD456")
    }

    #[test]
    fn test_a34_builds_merge_without_visit() {
        let message = minimal_builder().build().unwrap();

        let msh = message.get_segment_by_id("MSH").unwrap();
        let msg_type = msh
            .get_field(9)
            .unwrap()
            .get_repetition(0)
            .unwrap()
            .full_value();
        assert_eq!(msg_type.as_deref(), Some("ADT^A34"));

        let evn = message.get_segment_by_id("EVN").unwrap();
        assert_eq!(evn.get_field_value(1), Some("A34"));

        let pid = message.get_segment_by_id("PID").unwrap();
        assert_eq!(pid.get_field_value(3), Some("NEW123"));

        let mrg = message.get_segment_by_id("MRG").unwrap();
        assert_eq!(mrg.get_field_value(1), Some("OLD456"));

        // Patient ID only: no visit segments.
        assert!(message.get_segment_by_id("PV1").is_none());
    }

    #[test]
    fn test_a34_prior_name_is_optional() {
        let without = minimal_builder().build().unwrap();
        assert!(
            without
                .get_segment_by_id("MRG")
                .unwrap()
                .get_field(7)
                .is_none()
        );

        let with = minimal_builder()
            .prior_patient_name("Smyth", "Jon")
            .build()
            .unwrap();
        assert_eq!(
            with.get_segment_by_id("MRG").unwrap().get_field_value(7),
            Some("Smyth^Jon")
        );
    }

    fn minimal_a40_builder() -> AdtA40Builder {
        AdtBuilder::a40(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .control_id("MSG001")
            .patient_id("NEW123")
            .patient_name("Smith", "John")
            .patient_class("I")
            .add_prior_patient_id("OLD456")
            .add_prior_patient_id("OLD789")
    }

    #[test]
    fn test_a40_builds_merge_with_visit_and_mrg_list() {
        let message = minimal_a40_builder().build().unwrap();

        let msh = message.get_segment_by_id("MSH").unwrap();
        let msg_type = msh
            .get_field(9)
            .unwrap()
            .get_repetition(0)
            .unwrap()
            .full_value();
        assert_eq!(msg_type.as_deref(), Some("ADT^A40"));

        let evn = message.get_segment_by_id("EVN").unwrap();
        assert_eq!(evn.get_field_value(1), Some("A40"));

        // Identifier-list merge keeps its visit segment.
        let pv1 = message.get_segment_by_id("PV1").unwrap();
        assert_eq!(pv1.get_field_value(2), Some("I"));

        // Prior IDs accumulate as MRG-1 repetitions.
        let mrg = message.get_segment_by_id("MRG").unwrap();
        let mrg1 = mrg.get_field(1).unwrap();
        assert_eq!(mrg1.repetitions.len(), 2);
        assert_eq!(mrg.get_field_value(1), Some("OLD456"));
    }

    #[test]
    fn test_a40_prior_names_are_optional_repetitions() {
        let without = minimal_a40_builder().build().unwrap();
        assert!(
            without
                .get_segment_by_id("MRG")
                .unwrap()
                .get_field(7)
                .is_none()
        );

        let with = minimal_a40_builder()
            .add_prior_patient_name("Smyth", "Jon")
            .add_prior_patient_name("Doe", "Jane")
            .build()
            .unwrap();
        let mrg7 = with.get_segment_by_id("MRG").unwrap().get_field(7).unwrap();
        assert_eq!(mrg7.repetitions.len(), 2);
        let first = mrg7.get_repetition(0).unwrap().full_value();
        assert_eq!(first.as_deref(), Some("Smyth^Jon"));
    }
}
