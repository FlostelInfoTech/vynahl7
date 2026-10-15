//! SIU (Scheduling Information Unsolicited) message builders

use super::{MessageBuilder, generate_control_id};
use crate::{Version, error::Result, field::Field, message::Message, segment::Segment};

/// Builder for SIU^S12 - Notification of New Appointment Booking
pub struct SiuS12Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    placer_appointment_id: Option<String>,
    filler_appointment_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    event_reason: Option<String>,
    filler_contact: Option<String>,
    entered_by: Option<String>,
    resource_action_code: Option<String>,
}

impl SiuS12Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "SIU", "S12"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            placer_appointment_id: None,
            filler_appointment_id: None,
            patient_id: None,
            patient_name: None,
            event_reason: None,
            filler_contact: None,
            entered_by: None,
            resource_action_code: None,
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

    pub fn placer_appointment_id(mut self, id: &str) -> Self {
        self.placer_appointment_id = Some(id.to_string());
        self
    }

    pub fn filler_appointment_id(mut self, id: &str) -> Self {
        self.filler_appointment_id = Some(id.to_string());
        self
    }

    pub fn patient_id(mut self, id: &str) -> Self {
        self.patient_id = Some(id.to_string());
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.patient_name = Some((family.to_string(), given.to_string()));
        self
    }

    /// SCH-6 Event Reason (required by the schema).
    pub fn event_reason(mut self, reason: &str) -> Self {
        self.event_reason = Some(reason.to_string());
        self
    }

    /// SCH-16 Filler Contact Person (required by the schema).
    pub fn filler_contact(mut self, contact: &str) -> Self {
        self.filler_contact = Some(contact.to_string());
        self
    }

    /// SCH-20 Entered By Person (required by the schema).
    pub fn entered_by(mut self, person: &str) -> Self {
        self.entered_by = Some(person.to_string());
        self
    }

    /// RGS-2 Segment Action Code, HL7 table 0206 (e.g. `A` add).
    pub fn resource_action_code(mut self, code: &str) -> Self {
        self.resource_action_code = Some(code.to_string());
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
            "P",
        )?;
        self.base.message.add_segment(msh);

        // SCH segment
        let mut sch = Segment::new("SCH");
        sch.add_field(Field::from_delimited(
            self.placer_appointment_id.as_deref().unwrap_or(""),
        ));
        sch.add_field(Field::from_delimited(
            self.filler_appointment_id.as_deref().unwrap_or(""),
        ));
        for _ in 3..6 {
            sch.add_field(Field::from_value(""));
        }
        sch.add_field(Field::from_delimited(
            self.event_reason.as_deref().unwrap_or(""),
        )); // SCH-6: Event Reason
        for _ in 7..16 {
            sch.add_field(Field::from_value(""));
        }
        sch.add_field(Field::from_delimited(
            self.filler_contact.as_deref().unwrap_or(""),
        )); // SCH-16: Filler Contact Person
        for _ in 17..20 {
            sch.add_field(Field::from_value(""));
        }
        sch.add_field(Field::from_delimited(
            self.entered_by.as_deref().unwrap_or(""),
        )); // SCH-20: Entered By Person
        self.base.message.add_segment(sch);

        // RGS segment (required by the schema)
        let mut rgs = Segment::new("RGS");
        rgs.add_field(Field::from_value("1")); // Set ID
        if self.resource_action_code.is_some() {
            rgs.add_field(Field::from_value(
                self.resource_action_code.as_deref().unwrap_or(""),
            )); // RGS-2: Segment Action Code
        }
        self.base.message.add_segment(rgs);

        // PID segment if patient info provided
        if self.patient_id.is_some() || self.patient_name.is_some() {
            let mut pid = Segment::new("PID");
            pid.add_field(Field::from_value("1"));
            pid.add_field(Field::from_value(""));
            pid.add_field(Field::from_value(self.patient_id.as_deref().unwrap_or("")));
            pid.add_field(Field::from_value(""));

            if let Some((family, given)) = &self.patient_name {
                pid.add_field(Field::from_delimited(format!("{}^{}", family, given)));
            }

            self.base.message.add_segment(pid);
        }

        Ok(self.base.build())
    }
}

/// Builder for SIU^S13 - Notification of Appointment Rescheduling
pub struct SiuS13Builder {
    base: SiuS12Builder,
}

impl SiuS13Builder {
    pub fn new(version: Version) -> Self {
        let mut base = SiuS12Builder::new(version);
        base.base = MessageBuilder::new(version, "SIU", "S13");
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

    pub fn placer_appointment_id(mut self, id: &str) -> Self {
        self.base = self.base.placer_appointment_id(id);
        self
    }

    pub fn filler_appointment_id(mut self, id: &str) -> Self {
        self.base = self.base.filler_appointment_id(id);
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

    pub fn event_reason(mut self, reason: &str) -> Self {
        self.base = self.base.event_reason(reason);
        self
    }

    pub fn filler_contact(mut self, contact: &str) -> Self {
        self.base = self.base.filler_contact(contact);
        self
    }

    pub fn entered_by(mut self, person: &str) -> Self {
        self.base = self.base.entered_by(person);
        self
    }

    pub fn resource_action_code(mut self, code: &str) -> Self {
        self.base = self.base.resource_action_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for SIU^S14 - Notification of Appointment Modification
pub struct SiuS14Builder {
    base: SiuS12Builder,
}

impl SiuS14Builder {
    pub fn new(version: Version) -> Self {
        let mut base = SiuS12Builder::new(version);
        base.base = MessageBuilder::new(version, "SIU", "S14");
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

    pub fn placer_appointment_id(mut self, id: &str) -> Self {
        self.base = self.base.placer_appointment_id(id);
        self
    }

    pub fn filler_appointment_id(mut self, id: &str) -> Self {
        self.base = self.base.filler_appointment_id(id);
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

    pub fn event_reason(mut self, reason: &str) -> Self {
        self.base = self.base.event_reason(reason);
        self
    }

    pub fn filler_contact(mut self, contact: &str) -> Self {
        self.base = self.base.filler_contact(contact);
        self
    }

    pub fn entered_by(mut self, person: &str) -> Self {
        self.base = self.base.entered_by(person);
        self
    }

    pub fn resource_action_code(mut self, code: &str) -> Self {
        self.base = self.base.resource_action_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for SIU^S15 - Notification of Appointment Cancellation
pub struct SiuS15Builder {
    base: SiuS12Builder,
}

impl SiuS15Builder {
    pub fn new(version: Version) -> Self {
        let mut base = SiuS12Builder::new(version);
        base.base = MessageBuilder::new(version, "SIU", "S15");
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

    pub fn placer_appointment_id(mut self, id: &str) -> Self {
        self.base = self.base.placer_appointment_id(id);
        self
    }

    pub fn filler_appointment_id(mut self, id: &str) -> Self {
        self.base = self.base.filler_appointment_id(id);
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

    pub fn event_reason(mut self, reason: &str) -> Self {
        self.base = self.base.event_reason(reason);
        self
    }

    pub fn filler_contact(mut self, contact: &str) -> Self {
        self.base = self.base.filler_contact(contact);
        self
    }

    pub fn entered_by(mut self, person: &str) -> Self {
        self.base = self.base.entered_by(person);
        self
    }

    pub fn resource_action_code(mut self, code: &str) -> Self {
        self.base = self.base.resource_action_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}
