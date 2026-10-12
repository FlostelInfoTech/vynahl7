//! MDM (Medical Document Management) message builders

use super::{generate_control_id, MessageBuilder};
use crate::{error::Result, field::Field, message::Message, segment::Segment, Version};

/// Builder for MDM^T01 - Original Document Notification
pub struct MdmT01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    document_type: Option<String>,
    unique_document_number: Option<String>,
}

impl MdmT01Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "MDM", "T01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            document_type: None,
            unique_document_number: None,
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

    pub fn document_type(mut self, doc_type: &str) -> Self {
        self.document_type = Some(doc_type.to_string());
        self
    }

    pub fn unique_document_number(mut self, number: &str) -> Self {
        self.unique_document_number = Some(number.to_string());
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

        let evn = self.base.create_evn("T01", None)?;
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

        // TXA segment
        let mut txa = Segment::new("TXA");
        txa.add_field(Field::from_value("1")); // Set ID
        txa.add_field(Field::from_value(self.document_type.as_deref().unwrap_or(""))); // Document Type

        self.base.message.add_segment(txa);

        Ok(self.base.build())
    }
}

/// Builder for MDM^T02 - Original Document Notification and Content
pub struct MdmT02Builder {
    base: MdmT01Builder,
}

impl MdmT02Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MdmT01Builder::new(version);
        base.base = MessageBuilder::new(version, "MDM", "T02");
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

    pub fn document_type(mut self, doc_type: &str) -> Self {
        self.base = self.base.document_type(doc_type);
        self
    }

    pub fn unique_document_number(mut self, number: &str) -> Self {
        self.base = self.base.unique_document_number(number);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

/// Builder for MDM^T04 - Document Status Change Notification
pub struct MdmT04Builder {
    base: MdmT01Builder,
}

impl MdmT04Builder {
    pub fn new(version: Version) -> Self {
        let mut base = MdmT01Builder::new(version);
        base.base = MessageBuilder::new(version, "MDM", "T04");
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

    pub fn document_type(mut self, doc_type: &str) -> Self {
        self.base = self.base.document_type(doc_type);
        self
    }

    pub fn unique_document_number(mut self, number: &str) -> Self {
        self.base = self.base.unique_document_number(number);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}
