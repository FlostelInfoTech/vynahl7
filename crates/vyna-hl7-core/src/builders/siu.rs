//! SIU (Scheduling Information Unsolicited) message builders

use super::{generate_control_id, MessageBuilder};
use crate::{error::Result, field::Field, message::Message, segment::Segment, Version};

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
        sch.add_field(Field::from_value(self.placer_appointment_id.as_deref().unwrap_or("")));
        sch.add_field(Field::from_value(self.filler_appointment_id.as_deref().unwrap_or("")));
        self.base.message.add_segment(sch);

        // PID segment if patient info provided
        if self.patient_id.is_some() || self.patient_name.is_some() {
            let mut pid = Segment::new("PID");
            pid.add_field(Field::from_value("1"));
            pid.add_field(Field::from_value(""));
            pid.add_field(Field::from_value(self.patient_id.as_deref().unwrap_or("")));
            pid.add_field(Field::from_value(""));

            if let Some((family, given)) = &self.patient_name {
                pid.add_field(Field::from_value(format!("{}^{}", family, given)));
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

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}
