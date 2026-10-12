//! Laboratory message builders (OUL, OML)

use super::{generate_control_id, MessageBuilder};
use crate::{error::Result, field::Field, message::Message, segment::Segment, Version};

/// Builder for OUL^R21 - Unsolicited Laboratory Observation
pub struct OulR21Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    observation_id: Option<String>,
    observation_value: Option<String>,
}

impl OulR21Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "OUL", "R21"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            observation_id: None,
            observation_value: None,
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

    pub fn patient_id(mut self, id: &str) -> Self {
        self.patient_id = Some(id.to_string());
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.patient_name = Some((family.to_string(), given.to_string()));
        self
    }

    pub fn observation_id(mut self, id: &str) -> Self {
        self.observation_id = Some(id.to_string());
        self
    }

    pub fn observation_value(mut self, value: &str) -> Self {
        self.observation_value = Some(value.to_string());
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

        // PID segment (optional)
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

        // OBR segment
        let mut obr = Segment::new("OBR");
        obr.add_field(Field::from_value("1")); // Set ID
        obr.add_field(Field::from_value("")); // Placer Order Number
        obr.add_field(Field::from_value("")); // Filler Order Number
        obr.add_field(Field::from_value(self.observation_id.as_deref().unwrap_or(""))); // Universal Service ID

        self.base.message.add_segment(obr);

        // OBX segment (optional)
        if self.observation_value.is_some() {
            let mut obx = Segment::new("OBX");
            obx.add_field(Field::from_value("1")); // Set ID
            obx.add_field(Field::from_value("ST")); // Value Type
            obx.add_field(Field::from_value(self.observation_id.as_deref().unwrap_or(""))); // Observation ID
            obx.add_field(Field::from_value("")); // Observation Sub-ID
            obx.add_field(Field::from_value(self.observation_value.as_deref().unwrap_or(""))); // Observation Value

            self.base.message.add_segment(obx);
        }

        Ok(self.base.build())
    }
}

/// Builder for OML^O21 - Laboratory Order
pub struct OmlO21Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    order_control: Option<String>,
    placer_order_number: Option<String>,
    universal_service_id: Option<String>,
}

impl OmlO21Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "OML", "O21"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            order_control: Some("NW".to_string()),
            placer_order_number: None,
            universal_service_id: None,
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

    pub fn patient_id(mut self, id: &str) -> Self {
        self.patient_id = Some(id.to_string());
        self
    }

    pub fn patient_name(mut self, family: &str, given: &str) -> Self {
        self.patient_name = Some((family.to_string(), given.to_string()));
        self
    }

    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = Some(control.to_string());
        self
    }

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.placer_order_number = Some(number.to_string());
        self
    }

    pub fn universal_service_id(mut self, id: &str) -> Self {
        self.universal_service_id = Some(id.to_string());
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

        // PID segment
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        pid.add_field(Field::from_value(""));
        pid.add_field(Field::from_value(self.patient_id.as_deref().unwrap_or("")));
        pid.add_field(Field::from_value(""));

        if let Some((family, given)) = &self.patient_name {
            pid.add_field(Field::from_value(format!("{}^{}", family, given)));
        }

        self.base.message.add_segment(pid);

        // ORC segment
        let mut orc = Segment::new("ORC");
        orc.add_field(Field::from_value(self.order_control.as_deref().unwrap_or("NW")));
        orc.add_field(Field::from_value(self.placer_order_number.as_deref().unwrap_or("")));

        self.base.message.add_segment(orc);

        // OBR segment
        let mut obr = Segment::new("OBR");
        obr.add_field(Field::from_value("1")); // Set ID
        obr.add_field(Field::from_value(self.placer_order_number.as_deref().unwrap_or(""))); // Placer Order Number
        obr.add_field(Field::from_value("")); // Filler Order Number
        obr.add_field(Field::from_value(self.universal_service_id.as_deref().unwrap_or(""))); // Universal Service ID

        self.base.message.add_segment(obr);

        Ok(self.base.build())
    }
}
