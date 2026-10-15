//! RSP (Response) message builders
//!
//! This module provides builders for RSP messages which are responses to QBP queries.
//! RSP messages include MSA (acknowledgment), QAK (query acknowledgment), QPD (echoed query),
//! and application-specific data segments.

use super::{MessageBuilder, generate_control_id};
use crate::{
    Version,
    builders::fields::{QakBuilder, QpdBuilder},
    error::Result,
    field::Field,
    message::Message,
    segment::Segment,
};

/// Builder for RSP^K11 - Response to QBP^Q11 (e.g., Immunization History Response)
///
/// RSP^K11 typically includes patient demographics (PID), immunization records (RXA/ORC),
/// and optionally forecasts.
///
/// # Example
/// ```
/// use vyna_hl7_core::{Version, builders::rsp::RspK11Builder};
///
/// let message = RspK11Builder::new(Version::V2_5_1)
///     .sending_application("CAIR2")
///     .sending_facility("CAIR2")
///     .receiving_application("MyEHR")
///     .receiving_facility("FAC001")
///     .in_response_to("MSG-20231115-001")
///     .query_tag("Q123456789")
///     .query_name("Z44^Request Evaluated History and Forecast^CDCPHINVS")
///     .query_response_status("OK")
///     .hit_count(3)
///     .build()
///     .unwrap();
/// ```
pub struct RspK11Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    in_response_to_id: String,
    acknowledgment_code: String,
    query_tag: String,
    query_response_status: String,
    query_name: String,
    hit_count_total: Option<u32>,
    this_payload: Option<u32>,
    hits_remaining: Option<u32>,
    qpd_parameters: Vec<String>,
    data_segments: Vec<Segment>,
}

impl RspK11Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RSP", "K11"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            in_response_to_id: String::new(),
            acknowledgment_code: "AA".to_string(),
            query_tag: String::new(),
            query_response_status: "OK".to_string(),
            query_name: "Z44^Request Evaluated History and Forecast^CDCPHINVS".to_string(),
            hit_count_total: None,
            this_payload: None,
            hits_remaining: None,
            qpd_parameters: Vec::new(),
            data_segments: Vec::new(),
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

    /// Set the message control ID being responded to (MSA-2)
    pub fn in_response_to(mut self, id: &str) -> Self {
        self.in_response_to_id = id.to_string();
        self
    }

    /// Set acknowledgment code (MSA-1): AA, AE, AR
    pub fn acknowledgment_code(mut self, code: &str) -> Self {
        self.acknowledgment_code = code.to_string();
        self
    }

    /// Set query tag (QAK-1) - should match QPD-2 from query
    pub fn query_tag(mut self, tag: &str) -> Self {
        self.query_tag = tag.to_string();
        self
    }

    /// Set query response status (QAK-2): OK, NF, AE, AR, TM, PD
    pub fn query_response_status(mut self, status: &str) -> Self {
        self.query_response_status = status.to_string();
        self
    }

    /// Set query name (QAK-3, QPD-1) - should match QPD-1 from query
    pub fn query_name(mut self, name: &str) -> Self {
        self.query_name = name.to_string();
        self
    }

    /// Set total number of matching records (QAK-4)
    pub fn hit_count(mut self, count: u32) -> Self {
        self.hit_count_total = Some(count);
        self.this_payload = Some(count);
        self.hits_remaining = Some(0);
        self
    }

    /// Set hit counts with pagination (QAK-4, QAK-5, QAK-6)
    pub fn hit_counts(mut self, total: u32, this_payload: u32, remaining: u32) -> Self {
        self.hit_count_total = Some(total);
        self.this_payload = Some(this_payload);
        self.hits_remaining = Some(remaining);
        self
    }

    /// Add a QPD parameter (for echoing back the query)
    pub fn qpd_parameter(mut self, param: &str) -> Self {
        self.qpd_parameters.push(param.to_string());
        self
    }

    /// Add a data segment (PID, ORC, RXA, OBX, etc.)
    pub fn add_segment(mut self, segment: Segment) -> Self {
        self.data_segments.push(segment);
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self.control_id.unwrap_or_else(generate_control_id);

        // MSH segment
        let msh = self.base.create_msh(
            &self.sending_app,
            &self.sending_facility,
            &self.receiving_app,
            &self.receiving_facility,
            &control_id,
            "P",
        )?;
        self.base.message.add_segment(msh);

        // MSA segment
        let mut msa = Segment::new("MSA");
        msa.add_field(Field::from_value(&self.acknowledgment_code));
        msa.add_field(Field::from_value(&self.in_response_to_id));
        self.base.message.add_segment(msa);

        // QAK segment
        let mut qak_builder = QakBuilder::new()
            .query_tag(&self.query_tag)
            .query_response_status(&self.query_response_status)
            .message_query_name(&self.query_name);

        if let Some(count) = self.hit_count_total {
            qak_builder = qak_builder.hit_count_total(count);
        }
        if let Some(count) = self.this_payload {
            qak_builder = qak_builder.this_payload(count);
        }
        if let Some(count) = self.hits_remaining {
            qak_builder = qak_builder.hits_remaining(count);
        }

        self.base.message.add_segment(qak_builder.build());

        // QPD segment (echo from query)
        let mut qpd_builder = QpdBuilder::new()
            .message_query_name(&self.query_name)
            .query_tag(&self.query_tag);

        for param in &self.qpd_parameters {
            qpd_builder = qpd_builder.parameter(param);
        }

        self.base.message.add_segment(qpd_builder.build());

        // Add data segments (PID, ORC, RXA, etc.)
        for segment in self.data_segments {
            self.base.message.add_segment(segment);
        }

        Ok(self.base.build())
    }
}

/// Builder for RSP^K21 - Response to QBP^Q21 (Get Person Demographics)
pub struct RspK21Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    in_response_to_id: String,
    acknowledgment_code: String,
    query_tag: String,
    query_response_status: String,
    query_name: String,
    hit_count_total: Option<u32>,
    this_payload: Option<u32>,
    hits_remaining: Option<u32>,
    qpd_parameters: Vec<String>,
    data_segments: Vec<Segment>,
}

impl RspK21Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RSP", "K21"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            in_response_to_id: String::new(),
            acknowledgment_code: "AA".to_string(),
            query_tag: String::new(),
            query_response_status: "OK".to_string(),
            query_name: "Q21^Get Person Demographics^HL7".to_string(),
            hit_count_total: None,
            this_payload: None,
            hits_remaining: None,
            qpd_parameters: Vec::new(),
            data_segments: Vec::new(),
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

    pub fn in_response_to(mut self, id: &str) -> Self {
        self.in_response_to_id = id.to_string();
        self
    }

    pub fn acknowledgment_code(mut self, code: &str) -> Self {
        self.acknowledgment_code = code.to_string();
        self
    }

    pub fn query_tag(mut self, tag: &str) -> Self {
        self.query_tag = tag.to_string();
        self
    }

    pub fn query_response_status(mut self, status: &str) -> Self {
        self.query_response_status = status.to_string();
        self
    }

    pub fn query_name(mut self, name: &str) -> Self {
        self.query_name = name.to_string();
        self
    }

    pub fn hit_count(mut self, count: u32) -> Self {
        self.hit_count_total = Some(count);
        self.this_payload = Some(count);
        self.hits_remaining = Some(0);
        self
    }

    pub fn qpd_parameter(mut self, param: &str) -> Self {
        self.qpd_parameters.push(param.to_string());
        self
    }

    pub fn add_segment(mut self, segment: Segment) -> Self {
        self.data_segments.push(segment);
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

        let mut msa = Segment::new("MSA");
        msa.add_field(Field::from_value(&self.acknowledgment_code));
        msa.add_field(Field::from_value(&self.in_response_to_id));
        self.base.message.add_segment(msa);

        let mut qak_builder = QakBuilder::new()
            .query_tag(&self.query_tag)
            .query_response_status(&self.query_response_status)
            .message_query_name(&self.query_name);

        if let Some(count) = self.hit_count_total {
            qak_builder = qak_builder.hit_count_total(count);
        }
        if let Some(count) = self.this_payload {
            qak_builder = qak_builder.this_payload(count);
        }
        if let Some(count) = self.hits_remaining {
            qak_builder = qak_builder.hits_remaining(count);
        }

        self.base.message.add_segment(qak_builder.build());

        let mut qpd_builder = QpdBuilder::new()
            .message_query_name(&self.query_name)
            .query_tag(&self.query_tag);

        for param in &self.qpd_parameters {
            qpd_builder = qpd_builder.parameter(param);
        }

        self.base.message.add_segment(qpd_builder.build());

        for segment in self.data_segments {
            self.base.message.add_segment(segment);
        }

        Ok(self.base.build())
    }
}

/// Builder for RSP^K22 - Response to QBP^Q22 (Find Candidates)
///
/// RSP^K22 returns multiple PID segments for matching candidates.
pub struct RspK22Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    in_response_to_id: String,
    acknowledgment_code: String,
    query_tag: String,
    query_response_status: String,
    query_name: String,
    hit_count_total: Option<u32>,
    this_payload: Option<u32>,
    hits_remaining: Option<u32>,
    qpd_parameters: Vec<String>,
    data_segments: Vec<Segment>,
}

impl RspK22Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "RSP", "K22"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            in_response_to_id: String::new(),
            acknowledgment_code: "AA".to_string(),
            query_tag: String::new(),
            query_response_status: "OK".to_string(),
            query_name: "Q22^Find Candidates^HL7".to_string(),
            hit_count_total: None,
            this_payload: None,
            hits_remaining: None,
            qpd_parameters: Vec::new(),
            data_segments: Vec::new(),
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

    pub fn in_response_to(mut self, id: &str) -> Self {
        self.in_response_to_id = id.to_string();
        self
    }

    pub fn acknowledgment_code(mut self, code: &str) -> Self {
        self.acknowledgment_code = code.to_string();
        self
    }

    pub fn query_tag(mut self, tag: &str) -> Self {
        self.query_tag = tag.to_string();
        self
    }

    pub fn query_response_status(mut self, status: &str) -> Self {
        self.query_response_status = status.to_string();
        self
    }

    pub fn query_name(mut self, name: &str) -> Self {
        self.query_name = name.to_string();
        self
    }

    pub fn hit_count(mut self, count: u32) -> Self {
        self.hit_count_total = Some(count);
        self.this_payload = Some(count);
        self.hits_remaining = Some(0);
        self
    }

    pub fn hit_counts(mut self, total: u32, this_payload: u32, remaining: u32) -> Self {
        self.hit_count_total = Some(total);
        self.this_payload = Some(this_payload);
        self.hits_remaining = Some(remaining);
        self
    }

    pub fn qpd_parameter(mut self, param: &str) -> Self {
        self.qpd_parameters.push(param.to_string());
        self
    }

    pub fn add_segment(mut self, segment: Segment) -> Self {
        self.data_segments.push(segment);
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

        let mut msa = Segment::new("MSA");
        msa.add_field(Field::from_value(&self.acknowledgment_code));
        msa.add_field(Field::from_value(&self.in_response_to_id));
        self.base.message.add_segment(msa);

        let mut qak_builder = QakBuilder::new()
            .query_tag(&self.query_tag)
            .query_response_status(&self.query_response_status)
            .message_query_name(&self.query_name);

        if let Some(count) = self.hit_count_total {
            qak_builder = qak_builder.hit_count_total(count);
        }
        if let Some(count) = self.this_payload {
            qak_builder = qak_builder.this_payload(count);
        }
        if let Some(count) = self.hits_remaining {
            qak_builder = qak_builder.hits_remaining(count);
        }

        self.base.message.add_segment(qak_builder.build());

        let mut qpd_builder = QpdBuilder::new()
            .message_query_name(&self.query_name)
            .query_tag(&self.query_tag);

        for param in &self.qpd_parameters {
            qpd_builder = qpd_builder.parameter(param);
        }

        self.base.message.add_segment(qpd_builder.build());

        for segment in self.data_segments {
            self.base.message.add_segment(segment);
        }

        Ok(self.base.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsp_k11_basic() {
        let message = RspK11Builder::new(Version::V2_5_1)
            .sending_application("CAIR2")
            .sending_facility("CAIR2")
            .receiving_application("MyEHR")
            .receiving_facility("FAC001")
            .in_response_to("MSG-001")
            .query_tag("Q123456789")
            .query_response_status("OK")
            .hit_count(3)
            .build()
            .unwrap();

        assert_eq!(message.segments.len(), 4); // MSH, MSA, QAK, QPD
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "MSA");
        assert_eq!(message.segments[2].id, "QAK");
        assert_eq!(message.segments[3].id, "QPD");
    }

    #[test]
    fn test_rsp_k22_no_data() {
        let message = RspK22Builder::new(Version::V2_5_1)
            .sending_application("HOSPMPI")
            .receiving_application("CLINREG")
            .in_response_to("Q-20231115-045")
            .query_tag("987654321")
            .query_response_status("NF")
            .hit_count(0)
            .build()
            .unwrap();

        assert_eq!(message.segments.len(), 4); // MSH, MSA, QAK, QPD
        assert_eq!(message.segments[2].id, "QAK");
    }

    #[test]
    fn test_rsp_k21_with_data() {
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1")); // Set ID

        let message = RspK21Builder::new(Version::V2_5)
            .sending_application("MPI")
            .receiving_application("APP")
            .in_response_to("MSG-002")
            .query_tag("TAG-001")
            .hit_count(1)
            .add_segment(pid)
            .build()
            .unwrap();

        assert_eq!(message.segments.len(), 5); // MSH, MSA, QAK, QPD, PID
        assert_eq!(message.segments[4].id, "PID");
    }
}
