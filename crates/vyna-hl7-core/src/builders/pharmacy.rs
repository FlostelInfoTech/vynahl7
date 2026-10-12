//! Pharmacy message builders (RDE, RAS, RDS, RGV, RRA, RRD)

use super::{generate_control_id, MessageBuilder};
use crate::{error::Result, field::Field, message::Message, segment::Segment, Version};

/// Builder for RDE^O11 - Pharmacy/Treatment Encoded Order
pub struct RdeO11Builder {
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
    give_code: Option<String>,
}

impl RdeO11Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RDE", "O11"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            order_control: Some("NW".to_string()),
            placer_order_number: None,
            give_code: None,
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

    pub fn give_code(mut self, code: &str) -> Self {
        self.give_code = Some(code.to_string());
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

        // RXE segment
        let mut rxe = Segment::new("RXE");
        rxe.add_field(Field::from_value("")); // Quantity/Timing
        rxe.add_field(Field::from_value(self.give_code.as_deref().unwrap_or(""))); // Give Code

        self.base.message.add_segment(rxe);

        Ok(self.base.build())
    }
}

/// Builder for RAS^O17 - Pharmacy/Treatment Administration
pub struct RasO17Builder {
    base: RdeO11Builder,
}

impl RasO17Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RdeO11Builder::new(version);
        base.base = MessageBuilder::new(version, "RAS", "O17");
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

    pub fn order_control(mut self, control: &str) -> Self {
        self.base = self.base.order_control(control);
        self
    }

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.base = self.base.placer_order_number(number);
        self
    }

    pub fn give_code(mut self, code: &str) -> Self {
        self.base = self.base.give_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for RDS^O13 - Pharmacy/Treatment Dispense
pub struct RdsO13Builder {
    base: RdeO11Builder,
}

impl RdsO13Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RdeO11Builder::new(version);
        base.base = MessageBuilder::new(version, "RDS", "O13");
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

    pub fn order_control(mut self, control: &str) -> Self {
        self.base = self.base.order_control(control);
        self
    }

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.base = self.base.placer_order_number(number);
        self
    }

    pub fn give_code(mut self, code: &str) -> Self {
        self.base = self.base.give_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for RGV^O15 - Pharmacy/Treatment Give
pub struct RgvO15Builder {
    base: RdeO11Builder,
}

impl RgvO15Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RdeO11Builder::new(version);
        base.base = MessageBuilder::new(version, "RGV", "O15");
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

    pub fn order_control(mut self, control: &str) -> Self {
        self.base = self.base.order_control(control);
        self
    }

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.base = self.base.placer_order_number(number);
        self
    }

    pub fn give_code(mut self, code: &str) -> Self {
        self.base = self.base.give_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for RRA^O18 - Pharmacy/Treatment Administration Acknowledgment
pub struct RraO18Builder {
    base: RdeO11Builder,
}

impl RraO18Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RdeO11Builder::new(version);
        base.base = MessageBuilder::new(version, "RRA", "O18");
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

    pub fn order_control(mut self, control: &str) -> Self {
        self.base = self.base.order_control(control);
        self
    }

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.base = self.base.placer_order_number(number);
        self
    }

    pub fn give_code(mut self, code: &str) -> Self {
        self.base = self.base.give_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for RRD^O14 - Pharmacy/Treatment Dispense Information
pub struct RrdO14Builder {
    base: RdeO11Builder,
}

impl RrdO14Builder {
    pub fn new(version: Version) -> Self {
        let mut base = RdeO11Builder::new(version);
        base.base = MessageBuilder::new(version, "RRD", "O14");
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

    pub fn order_control(mut self, control: &str) -> Self {
        self.base = self.base.order_control(control);
        self
    }

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.base = self.base.placer_order_number(number);
        self
    }

    pub fn give_code(mut self, code: &str) -> Self {
        self.base = self.base.give_code(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}
