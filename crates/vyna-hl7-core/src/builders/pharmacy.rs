//! Pharmacy message builders (RDE, RAS, RDS, RGV, RRA, RRD)

use super::{generate_control_id, MessageBuilder};
use crate::{error::Result, field::Field, message::Message, segment::Segment, types::format_timestamp, Version};
use chrono::Local;

fn now_timestamp() -> String {
    format_timestamp(&Local::now().naive_local())
}

fn pid_segment(patient_id: Option<&str>, patient_name: &Option<(String, String)>) -> Segment {
    let mut pid = Segment::new("PID");
    pid.add_field(Field::from_value("1"));
    pid.add_field(Field::from_value(""));
    pid.add_field(Field::from_value(patient_id.unwrap_or("")));
    pid.add_field(Field::from_value(""));

    if let Some((family, given)) = patient_name {
        pid.add_field(Field::from_value(format!("{family}^{given}")));
    }

    pid
}

fn orc_segment(order_control: &str, placer_order_number: Option<&str>) -> Segment {
    let mut orc = Segment::new("ORC");
    orc.add_field(Field::from_value(order_control)); // ORC-1: Order Control
    orc.add_field(Field::from_value(placer_order_number.unwrap_or(""))); // ORC-2: Placer Order Number
    orc
}

fn rxr_segment(route: Option<&str>) -> Segment {
    let mut rxr = Segment::new("RXR");
    rxr.add_field(Field::from_value(route.unwrap_or(""))); // RXR-1: Route
    rxr
}

fn tq1_segment() -> Segment {
    let mut tq1 = Segment::new("TQ1");
    tq1.add_field(Field::from_value("1")); // Set ID
    tq1
}

fn msa_segment(ack_code: &str, control_id: &str) -> Segment {
    let mut msa = Segment::new("MSA");
    msa.add_field(Field::from_value(ack_code)); // MSA-1: Acknowledgment Code
    msa.add_field(Field::from_value(control_id)); // MSA-2: Message Control ID
    msa
}

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
    give_amount_minimum: Option<String>,
    give_units: Option<String>,
    route: Option<String>,
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
            give_amount_minimum: None,
            give_units: None,
            route: None,
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

    /// RXE-2 Give Code (required by the schema).
    pub fn give_code(mut self, code: &str) -> Self {
        self.give_code = Some(code.to_string());
        self
    }

    /// RXE-3 Give Amount - Minimum (required by the schema).
    pub fn give_amount_minimum(mut self, amount: &str) -> Self {
        self.give_amount_minimum = Some(amount.to_string());
        self
    }

    /// RXE-5 Give Units (required by the schema).
    pub fn give_units(mut self, units: &str) -> Self {
        self.give_units = Some(units.to_string());
        self
    }

    /// RXR-1 Route, HL7 table 0162 (required by the schema).
    pub fn route(mut self, route: &str) -> Self {
        self.route = Some(route.to_string());
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

        let pid = pid_segment(self.patient_id.as_deref(), &self.patient_name);
        self.base.message.add_segment(pid);

        let orc = orc_segment(
            self.order_control.as_deref().unwrap_or("NW"),
            self.placer_order_number.as_deref(),
        );
        self.base.message.add_segment(orc);

        // RXE segment
        let mut rxe = Segment::new("RXE");
        rxe.add_field(Field::from_value("")); // Quantity/Timing
        rxe.add_field(Field::from_value(self.give_code.as_deref().unwrap_or(""))); // RXE-2: Give Code
        rxe.add_field(Field::from_value(
            self.give_amount_minimum.as_deref().unwrap_or(""),
        )); // RXE-3: Give Amount - Minimum
        rxe.add_field(Field::from_value("")); // Give Amount - Maximum
        rxe.add_field(Field::from_value(self.give_units.as_deref().unwrap_or(""))); // RXE-5: Give Units

        self.base.message.add_segment(rxe);

        self.base
            .message
            .add_segment(rxr_segment(self.route.as_deref()));
        self.base.message.add_segment(tq1_segment());

        Ok(self.base.build())
    }
}

/// Builder for RAS^O17 - Pharmacy/Treatment Administration
pub struct RasO17Builder {
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
    administered_code: Option<String>,
    amount: Option<String>,
    datetime_start: Option<String>,
    datetime_end: Option<String>,
    route: Option<String>,
}

impl RasO17Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RAS", "O17"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            order_control: Some("NW".to_string()),
            placer_order_number: None,
            administered_code: None,
            amount: None,
            datetime_start: None,
            datetime_end: None,
            route: None,
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

    /// RXA-5 Administered Code (required by the schema).
    pub fn administered_code(mut self, code: &str) -> Self {
        self.administered_code = Some(code.to_string());
        self
    }

    /// RXA-6 Administered Amount (required by the schema).
    pub fn amount(mut self, amount: &str) -> Self {
        self.amount = Some(amount.to_string());
        self
    }

    /// RXA-3 Date/Time Start of Administration (defaults to now).
    pub fn datetime_start(mut self, datetime: &str) -> Self {
        self.datetime_start = Some(datetime.to_string());
        self
    }

    /// RXA-4 Date/Time End of Administration (defaults to the start time).
    pub fn datetime_end(mut self, datetime: &str) -> Self {
        self.datetime_end = Some(datetime.to_string());
        self
    }

    /// RXR-1 Route, HL7 table 0162 (required by the schema).
    pub fn route(mut self, route: &str) -> Self {
        self.route = Some(route.to_string());
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

        let pid = pid_segment(self.patient_id.as_deref(), &self.patient_name);
        self.base.message.add_segment(pid);

        let orc = orc_segment(
            self.order_control.as_deref().unwrap_or("NW"),
            self.placer_order_number.as_deref(),
        );
        self.base.message.add_segment(orc);

        // RXA segment
        let start = self.datetime_start.unwrap_or_else(now_timestamp);
        let end = self.datetime_end.unwrap_or_else(|| start.clone());
        let mut rxa = Segment::new("RXA");
        rxa.add_field(Field::from_value("0")); // Give Sub-ID Counter
        rxa.add_field(Field::from_value("1")); // Administration Sub-ID Counter
        rxa.add_field(Field::from_value(start)); // RXA-3: Start of Administration
        rxa.add_field(Field::from_value(end)); // RXA-4: End of Administration
        rxa.add_field(Field::from_value(
            self.administered_code.as_deref().unwrap_or(""),
        )); // RXA-5: Administered Code
        rxa.add_field(Field::from_value(self.amount.as_deref().unwrap_or(""))); // RXA-6: Administered Amount

        self.base.message.add_segment(rxa);
        self.base
            .message
            .add_segment(rxr_segment(self.route.as_deref()));

        Ok(self.base.build())
    }
}

/// Builder for RDS^O13 - Pharmacy/Treatment Dispense
pub struct RdsO13Builder {
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
    dispense_code: Option<String>,
    datetime_dispensed: Option<String>,
    dispense_amount: Option<String>,
    prescription_number: Option<String>,
    route: Option<String>,
}

impl RdsO13Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RDS", "O13"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            order_control: Some("NW".to_string()),
            placer_order_number: None,
            dispense_code: None,
            datetime_dispensed: None,
            dispense_amount: None,
            prescription_number: None,
            route: None,
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

    /// RXD-2 Dispense/Give Code (required by the schema).
    pub fn dispense_code(mut self, code: &str) -> Self {
        self.dispense_code = Some(code.to_string());
        self
    }

    /// RXD-3 Date/Time Dispensed (defaults to now).
    pub fn datetime_dispensed(mut self, datetime: &str) -> Self {
        self.datetime_dispensed = Some(datetime.to_string());
        self
    }

    /// RXD-4 Actual Dispense Amount (required by the schema).
    pub fn dispense_amount(mut self, amount: &str) -> Self {
        self.dispense_amount = Some(amount.to_string());
        self
    }

    /// RXD-7 Prescription Number (required by the schema).
    pub fn prescription_number(mut self, number: &str) -> Self {
        self.prescription_number = Some(number.to_string());
        self
    }

    /// RXR-1 Route, HL7 table 0162 (required by the schema).
    pub fn route(mut self, route: &str) -> Self {
        self.route = Some(route.to_string());
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

        let pid = pid_segment(self.patient_id.as_deref(), &self.patient_name);
        self.base.message.add_segment(pid);

        let orc = orc_segment(
            self.order_control.as_deref().unwrap_or("NW"),
            self.placer_order_number.as_deref(),
        );
        self.base.message.add_segment(orc);

        // RXD segment
        let mut rxd = Segment::new("RXD");
        rxd.add_field(Field::from_value("0")); // Dispense Sub-ID Counter
        rxd.add_field(Field::from_value(
            self.dispense_code.as_deref().unwrap_or(""),
        )); // RXD-2: Dispense/Give Code
        rxd.add_field(Field::from_value(
            self.datetime_dispensed.unwrap_or_else(now_timestamp),
        )); // RXD-3: Date/Time Dispensed
        rxd.add_field(Field::from_value(
            self.dispense_amount.as_deref().unwrap_or(""),
        )); // RXD-4: Actual Dispense Amount
        rxd.add_field(Field::from_value("")); // Pharmacy/Treatment Supplier's Verification
        rxd.add_field(Field::from_value("")); // Actual Dispense Units
        rxd.add_field(Field::from_value(
            self.prescription_number.as_deref().unwrap_or(""),
        )); // RXD-7: Prescription Number

        self.base.message.add_segment(rxd);
        self.base
            .message
            .add_segment(rxr_segment(self.route.as_deref()));

        Ok(self.base.build())
    }
}

/// Builder for RGV^O15 - Pharmacy/Treatment Give
pub struct RgvO15Builder {
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
    give_amount_minimum: Option<String>,
    give_units: Option<String>,
    route: Option<String>,
}

impl RgvO15Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RGV", "O15"),
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
            give_amount_minimum: None,
            give_units: None,
            route: None,
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

    /// RXG-4 Give Code (required by the schema).
    pub fn give_code(mut self, code: &str) -> Self {
        self.give_code = Some(code.to_string());
        self
    }

    /// RXG-5 Give Amount - Minimum (required by the schema).
    pub fn give_amount_minimum(mut self, amount: &str) -> Self {
        self.give_amount_minimum = Some(amount.to_string());
        self
    }

    /// RXG-7 Give Units (required by the schema).
    pub fn give_units(mut self, units: &str) -> Self {
        self.give_units = Some(units.to_string());
        self
    }

    /// RXR-1 Route, HL7 table 0162 (required by the schema).
    pub fn route(mut self, route: &str) -> Self {
        self.route = Some(route.to_string());
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

        let pid = pid_segment(self.patient_id.as_deref(), &self.patient_name);
        self.base.message.add_segment(pid);

        let orc = orc_segment(
            self.order_control.as_deref().unwrap_or("NW"),
            self.placer_order_number.as_deref(),
        );
        self.base.message.add_segment(orc);

        // RXG segment
        let mut rxg = Segment::new("RXG");
        rxg.add_field(Field::from_value("0")); // Give Sub-ID Counter
        rxg.add_field(Field::from_value("")); // Dispense Sub-ID Counter
        rxg.add_field(Field::from_value("")); // Quantity/Timing
        rxg.add_field(Field::from_value(self.give_code.as_deref().unwrap_or(""))); // RXG-4: Give Code
        rxg.add_field(Field::from_value(
            self.give_amount_minimum.as_deref().unwrap_or(""),
        )); // RXG-5: Give Amount - Minimum
        rxg.add_field(Field::from_value("")); // Give Amount - Maximum
        rxg.add_field(Field::from_value(self.give_units.as_deref().unwrap_or(""))); // RXG-7: Give Units

        self.base.message.add_segment(rxg);
        self.base
            .message
            .add_segment(rxr_segment(self.route.as_deref()));
        self.base.message.add_segment(tq1_segment());

        Ok(self.base.build())
    }
}

/// Builder for RRA^O18 - Pharmacy/Treatment Administration Acknowledgment
pub struct RraO18Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    ack_code: String,
    acknowledged_control_id: Option<String>,
}

impl RraO18Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RRA", "O18"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            ack_code: "AA".to_string(),
            acknowledged_control_id: None,
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

    /// MSA-1 Acknowledgment Code, HL7 table 0008 (defaults to `AA`).
    pub fn ack_code(mut self, code: &str) -> Self {
        self.ack_code = code.to_string();
        self
    }

    /// MSA-2 Message Control ID of the acknowledged message (required).
    pub fn acknowledged_control_id(mut self, id: &str) -> Self {
        self.acknowledged_control_id = Some(id.to_string());
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

        // PID is optional context; emit it only when fully identified so a
        // half-populated PID cannot fail its own required fields.
        if self.patient_id.is_some() && self.patient_name.is_some() {
            let pid = pid_segment(self.patient_id.as_deref(), &self.patient_name);
            self.base.message.add_segment(pid);
        }

        let msa = msa_segment(
            &self.ack_code,
            self.acknowledged_control_id.as_deref().unwrap_or(""),
        );
        self.base.message.add_segment(msa);

        Ok(self.base.build())
    }
}

/// Builder for RRD^O14 - Pharmacy/Treatment Dispense Information
pub struct RrdO14Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    ack_code: String,
    acknowledged_control_id: Option<String>,
}

impl RrdO14Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RRD", "O14"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            ack_code: "AA".to_string(),
            acknowledged_control_id: None,
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

    /// MSA-1 Acknowledgment Code, HL7 table 0008 (defaults to `AA`).
    pub fn ack_code(mut self, code: &str) -> Self {
        self.ack_code = code.to_string();
        self
    }

    /// MSA-2 Message Control ID of the acknowledged message (required).
    pub fn acknowledged_control_id(mut self, id: &str) -> Self {
        self.acknowledged_control_id = Some(id.to_string());
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

        // PID is optional context; emit it only when fully identified so a
        // half-populated PID cannot fail its own required fields.
        if self.patient_id.is_some() && self.patient_name.is_some() {
            let pid = pid_segment(self.patient_id.as_deref(), &self.patient_name);
            self.base.message.add_segment(pid);
        }

        let msa = msa_segment(
            &self.ack_code,
            self.acknowledged_control_id.as_deref().unwrap_or(""),
        );
        self.base.message.add_segment(msa);

        Ok(self.base.build())
    }
}
