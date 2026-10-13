//! DFT (Detailed Financial Transaction) message builders

use super::{generate_control_id, MessageBuilder};
use crate::{
    error::Result, field::Field, message::Message, segment::Segment, types::format_timestamp,
    Version,
};
use chrono::Local;

/// Builder for DFT^P03 - Post Detail Financial Transaction
pub struct DftP03Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    transaction_code: Option<String>,
    transaction_amount: Option<String>,
    transaction_date: Option<String>,
    transaction_type: Option<String>,
}

impl DftP03Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "DFT", "P03"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            transaction_code: None,
            transaction_amount: None,
            transaction_date: None,
            transaction_type: None,
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

    pub fn transaction_code(mut self, code: &str) -> Self {
        self.transaction_code = Some(code.to_string());
        self
    }

    pub fn transaction_amount(mut self, amount: &str) -> Self {
        self.transaction_amount = Some(amount.to_string());
        self
    }

    /// FT1-4 Transaction Date (defaults to now).
    pub fn transaction_date(mut self, datetime: &str) -> Self {
        self.transaction_date = Some(datetime.to_string());
        self
    }

    /// FT1-6 Transaction Type, HL7 table 0017 (defaults to `CG` charge).
    pub fn transaction_type(mut self, code: &str) -> Self {
        self.transaction_type = Some(code.to_string());
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

        let evn = self.base.create_evn(self.base.trigger_event(), None)?;
        self.base.message.add_segment(evn);

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

        // FT1 segment
        let mut ft1 = Segment::new("FT1");
        ft1.add_field(Field::from_value("1")); // Set ID
        ft1.add_field(Field::from_value("")); // Transaction ID
        ft1.add_field(Field::from_value("")); // Transaction Batch ID
        let txn_date = self.transaction_date.unwrap_or_else(|| {
            format_timestamp(&Local::now().naive_local())
        });
        ft1.add_field(Field::from_value(txn_date)); // FT1-4: Transaction Date
        ft1.add_field(Field::from_value("")); // Transaction Posting Date
        ft1.add_field(Field::from_value(
            self.transaction_type.as_deref().unwrap_or("CG"),
        )); // FT1-6: Transaction Type
        ft1.add_field(Field::from_value(
            self.transaction_code.as_deref().unwrap_or(""),
        )); // FT1-7: Transaction Code
        if self.transaction_amount.is_some() {
            ft1.add_field(Field::from_value("")); // Transaction Description
            ft1.add_field(Field::from_value("")); // Transaction Description - Alt
            ft1.add_field(Field::from_value("")); // Transaction Quantity
            ft1.add_field(Field::from_value(
                self.transaction_amount.as_deref().unwrap_or(""),
            )); // FT1-11: Transaction Amount - Extended
        }

        self.base.message.add_segment(ft1);

        Ok(self.base.build())
    }
}

/// Builder for DFT^P11 - Post Detail Financial Transactions - Expanded
pub struct DftP11Builder {
    base: DftP03Builder,
}

impl DftP11Builder {
    pub fn new(version: Version) -> Self {
        let mut base = DftP03Builder::new(version);
        base.base = MessageBuilder::new(version, "DFT", "P11");
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

    pub fn transaction_code(mut self, code: &str) -> Self {
        self.base = self.base.transaction_code(code);
        self
    }

    pub fn transaction_amount(mut self, amount: &str) -> Self {
        self.base = self.base.transaction_amount(amount);
        self
    }

    pub fn transaction_date(mut self, datetime: &str) -> Self {
        self.base = self.base.transaction_date(datetime);
        self
    }

    pub fn transaction_type(mut self, code: &str) -> Self {
        self.base = self.base.transaction_type(code);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}
