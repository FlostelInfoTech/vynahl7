//! ORM (Order Message) message builders

use super::{generate_control_id, MessageBuilder};
use crate::{
    error::Result,
    field::Field,
    message::Message,
    segment::Segment,
    Version,
};

/// Builder for ORM^O01 - General Order Message
pub struct OrmO01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    placer_order_number: Option<String>,
    order_control: String,
    universal_service_id: Option<String>,
}

impl OrmO01Builder {
    /// Create a new ORM^O01 builder
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "ORM", "O01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            patient_id: None,
            patient_name: None,
            placer_order_number: None,
            order_control: "NW".to_string(), // New order
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

    pub fn control_id(mut self, id: &str) -> Self {
        self.control_id = Some(id.to_string());
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

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.placer_order_number = Some(number.to_string());
        self
    }

    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }

    pub fn universal_service_id(mut self, id: &str) -> Self {
        self.universal_service_id = Some(id.to_string());
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

        // Create PID segment
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        pid.add_field(Field::from_value(""));
        pid.add_field(Field::from_value(self.patient_id.as_deref().unwrap_or("")));
        pid.add_field(Field::from_value(""));

        if let Some((family, given)) = &self.patient_name {
            let name = format!("{}^{}", family, given);
            pid.add_field(Field::from_value(&name));
        } else {
            pid.add_field(Field::from_value(""));
        }

        self.base.message.add_segment(pid);

        // Create ORC segment
        let mut orc = Segment::new("ORC");
        orc.add_field(Field::from_value(&self.order_control)); // ORC-1: Order Control
        orc.add_field(Field::from_value(self.placer_order_number.as_deref().unwrap_or(""))); // ORC-2: Placer Order Number

        self.base.message.add_segment(orc);

        // Create OBR segment
        let mut obr = Segment::new("OBR");
        obr.add_field(Field::from_value("1")); // OBR-1: Set ID
        obr.add_field(Field::from_value(self.placer_order_number.as_deref().unwrap_or(""))); // OBR-2: Placer Order Number
        obr.add_field(Field::from_value("")); // OBR-3: Filler Order Number
        obr.add_field(Field::from_value(self.universal_service_id.as_deref().unwrap_or(""))); // OBR-4: Universal Service ID

        self.base.message.add_segment(obr);

        Ok(self.base.build())
    }
}
