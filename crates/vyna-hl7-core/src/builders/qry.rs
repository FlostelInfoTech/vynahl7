//! QRY (Query) message builders

use super::{generate_control_id, MessageBuilder};
use crate::{error::Result, field::Field, message::Message, segment::Segment, types::format_timestamp, Version};
use chrono::Local;

/// Builder for QRY^A19 - Patient Query
pub struct QryA19Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    query_id: String,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
}

impl QryA19Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "QRY", "A19"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            query_id: generate_control_id(),
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

    pub fn control_id(mut self, id: &str) -> Self {
        self.control_id = Some(id.to_string());
        self
    }

    pub fn query_id(mut self, id: &str) -> Self {
        self.query_id = id.to_string();
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

        // QRD segment - Query Definition
        let mut qrd = Segment::new("QRD");

        // QRD-1: Query Date/Time
        let timestamp = format_timestamp(&Local::now().naive_local());
        qrd.add_field(Field::from_value(&timestamp));

        // QRD-2: Query Format Code (R = Record-oriented)
        qrd.add_field(Field::from_value("R"));

        // QRD-3: Query Priority (I = Immediate)
        qrd.add_field(Field::from_value("I"));

        // QRD-4: Query ID
        qrd.add_field(Field::from_value(&self.query_id));

        // QRD-5: Deferred Response Type (empty)
        qrd.add_field(Field::from_value(""));

        // QRD-6: Deferred Response Date/Time (empty)
        qrd.add_field(Field::from_value(""));

        // QRD-7: Quantity Limited Request (empty)
        qrd.add_field(Field::from_value(""));

        // QRD-8: Who Subject Filter
        if let Some((family, given)) = &self.patient_name {
            let name = format!("{}^{}", family, given);
            qrd.add_field(Field::from_value(&name));
        } else if let Some(id) = &self.patient_id {
            qrd.add_field(Field::from_value(id));
        } else {
            qrd.add_field(Field::from_value(""));
        }

        // QRD-9: What Subject Filter - DEM for demographics
        qrd.add_field(Field::from_value("DEM"));

        // QRD-10: What Department Data Code (empty)
        qrd.add_field(Field::from_value(""));

        self.base.message.add_segment(qrd);

        Ok(self.base.build())
    }
}

/// Builder for QRY^Q01 - Query Sent for Immediate Response
pub struct QryQ01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    query_id: String,
    what_subject_filter: String,
}

impl QryQ01Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "QRY", "Q01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            query_id: generate_control_id(),
            what_subject_filter: String::new(),
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

    pub fn query_id(mut self, id: &str) -> Self {
        self.query_id = id.to_string();
        self
    }

    pub fn what_subject_filter(mut self, filter: &str) -> Self {
        self.what_subject_filter = filter.to_string();
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

        // QRD segment
        let mut qrd = Segment::new("QRD");

        let timestamp = format_timestamp(&Local::now().naive_local());
        qrd.add_field(Field::from_value(&timestamp));
        qrd.add_field(Field::from_value("R")); // Format
        qrd.add_field(Field::from_value("I")); // Priority
        qrd.add_field(Field::from_value(&self.query_id));
        qrd.add_field(Field::from_value("")); // Deferred Response Type
        qrd.add_field(Field::from_value("")); // Deferred Response Date/Time
        qrd.add_field(Field::from_value("")); // Quantity Limited Request
        qrd.add_field(Field::from_value("")); // Who Subject Filter
        qrd.add_field(Field::from_value(&self.what_subject_filter)); // What Subject Filter

        self.base.message.add_segment(qrd);

        Ok(self.base.build())
    }
}

/// Builder for QRY^Q02 - Query Sent for Deferred Response
pub struct QryQ02Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    query_id: String,
    what_subject_filter: String,
    deferred_response_type: String,
}

impl QryQ02Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "QRY", "Q02"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            query_id: generate_control_id(),
            what_subject_filter: String::new(),
            deferred_response_type: "B".to_string(), // Before the date/time specified
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

    pub fn query_id(mut self, id: &str) -> Self {
        self.query_id = id.to_string();
        self
    }

    pub fn what_subject_filter(mut self, filter: &str) -> Self {
        self.what_subject_filter = filter.to_string();
        self
    }

    pub fn deferred_response_type(mut self, response_type: &str) -> Self {
        self.deferred_response_type = response_type.to_string();
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

        // QRD segment
        let mut qrd = Segment::new("QRD");

        let timestamp = format_timestamp(&Local::now().naive_local());
        qrd.add_field(Field::from_value(&timestamp));
        qrd.add_field(Field::from_value("R")); // Format
        qrd.add_field(Field::from_value("D")); // Priority - Deferred
        qrd.add_field(Field::from_value(&self.query_id));
        qrd.add_field(Field::from_value(&self.deferred_response_type)); // Deferred Response Type
        qrd.add_field(Field::from_value("")); // Deferred Response Date/Time
        qrd.add_field(Field::from_value("")); // Quantity Limited Request
        qrd.add_field(Field::from_value("")); // Who Subject Filter
        qrd.add_field(Field::from_value(&self.what_subject_filter)); // What Subject Filter

        self.base.message.add_segment(qrd);

        Ok(self.base.build())
    }
}
