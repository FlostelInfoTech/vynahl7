//! QBP (Query by Parameter) message builders
//!
//! This module provides builders for QBP messages which use QPD (Query Parameter Definition)
//! segments instead of the older QRD-based queries. QBP is the standard query mechanism in
//! HL7 v2.5+.

use super::{generate_control_id, MessageBuilder};
use crate::{
    builders::fields::{QpdBuilder, RcpBuilder},
    error::Result,
    message::Message,
    Version,
};

/// Builder for QBP^Q11 - Query by Parameter (e.g., Immunization History Query)
///
/// QBP^Q11 is commonly used for immunization queries using the Z34/Z44 CDC profiles.
///
/// # Example
/// ```
/// use vyna_hl7_core::{Version, builders::qbp::QbpQ11Builder};
///
/// let message = QbpQ11Builder::new(Version::V2_5_1)
///     .sending_application("MyEHR")
///     .sending_facility("FAC001")
///     .receiving_application("CAIR2")
///     .receiving_facility("CAIR2")
///     .query_name("Z44^Request Evaluated History and Forecast^CDCPHINVS")
///     .query_tag("Q123456789")
///     .patient_id("234567^^^MYEHR^MR")
///     .patient_name("DOE^JANE^MARIE^^^^L")
///     .build()
///     .unwrap();
/// ```
pub struct QbpQ11Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    query_name: String,
    query_tag: String,
    qpd_parameters: Vec<String>,
    query_priority: String,
    quantity_limit: Option<String>,
}

impl QbpQ11Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "QBP", "Q11"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            query_name: "Z44^Request Evaluated History and Forecast^CDCPHINVS".to_string(),
            query_tag: generate_control_id(),
            qpd_parameters: Vec::new(),
            query_priority: "I".to_string(),
            quantity_limit: Some("100^RD".to_string()),
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

    /// Set the query name (QPD-1): identifies the query profile
    pub fn query_name(mut self, name: &str) -> Self {
        self.query_name = name.to_string();
        self
    }

    /// Set the query tag (QPD-2): unique identifier for this query
    pub fn query_tag(mut self, tag: &str) -> Self {
        self.query_tag = tag.to_string();
        self
    }

    /// Add a patient ID parameter (QPD-3)
    pub fn patient_id(mut self, id: &str) -> Self {
        self.qpd_parameters.push(id.to_string());
        self
    }

    /// Add patient name parameter (QPD-4)
    pub fn patient_name(mut self, name: &str) -> Self {
        self.qpd_parameters.push(name.to_string());
        self
    }

    /// Add mother's maiden name parameter (QPD-5)
    pub fn mothers_name(mut self, name: &str) -> Self {
        self.qpd_parameters.push(name.to_string());
        self
    }

    /// Add date of birth parameter (QPD-6)
    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.qpd_parameters.push(dob.to_string());
        self
    }

    /// Add sex parameter (QPD-7)
    pub fn sex(mut self, sex: &str) -> Self {
        self.qpd_parameters.push(sex.to_string());
        self
    }

    /// Add address parameter (QPD-8)
    pub fn address(mut self, address: &str) -> Self {
        self.qpd_parameters.push(address.to_string());
        self
    }

    /// Add a custom QPD parameter
    pub fn parameter(mut self, param: &str) -> Self {
        self.qpd_parameters.push(param.to_string());
        self
    }

    /// Set query priority (RCP-1): I=Immediate, D=Deferred
    pub fn query_priority(mut self, priority: &str) -> Self {
        self.query_priority = priority.to_string();
        self
    }

    /// Set quantity limit (RCP-2): e.g., "100^RD" for 100 records
    pub fn quantity_limit(mut self, limit: &str) -> Self {
        self.quantity_limit = Some(limit.to_string());
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

        // QPD segment
        let mut qpd_builder = QpdBuilder::new()
            .message_query_name(&self.query_name)
            .query_tag(&self.query_tag);

        for param in &self.qpd_parameters {
            qpd_builder = qpd_builder.parameter(param);
        }

        self.base.message.add_segment(qpd_builder.build());

        // RCP segment
        let mut rcp_builder = RcpBuilder::new()
            .query_priority(&self.query_priority);

        if let Some(limit) = &self.quantity_limit {
            rcp_builder = rcp_builder.quantity_limit(limit);
        }

        self.base.message.add_segment(rcp_builder.build());

        Ok(self.base.build())
    }
}

/// Builder for QBP^Q15 - Display Response Query
///
/// Used for queries that expect a display-formatted response.
pub struct QbpQ15Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    query_name: String,
    query_tag: String,
    qpd_parameters: Vec<String>,
    query_priority: String,
    quantity_limit: Option<String>,
}

impl QbpQ15Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "QBP", "Q15"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            query_name: "Q15^Display Response^HL7".to_string(),
            query_tag: generate_control_id(),
            qpd_parameters: Vec::new(),
            query_priority: "I".to_string(),
            quantity_limit: Some("100^RD".to_string()),
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

    pub fn query_name(mut self, name: &str) -> Self {
        self.query_name = name.to_string();
        self
    }

    pub fn query_tag(mut self, tag: &str) -> Self {
        self.query_tag = tag.to_string();
        self
    }

    pub fn parameter(mut self, param: &str) -> Self {
        self.qpd_parameters.push(param.to_string());
        self
    }

    pub fn query_priority(mut self, priority: &str) -> Self {
        self.query_priority = priority.to_string();
        self
    }

    pub fn quantity_limit(mut self, limit: &str) -> Self {
        self.quantity_limit = Some(limit.to_string());
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

        let mut qpd_builder = QpdBuilder::new()
            .message_query_name(&self.query_name)
            .query_tag(&self.query_tag);

        for param in &self.qpd_parameters {
            qpd_builder = qpd_builder.parameter(param);
        }

        self.base.message.add_segment(qpd_builder.build());

        let mut rcp_builder = RcpBuilder::new()
            .query_priority(&self.query_priority);

        if let Some(limit) = &self.quantity_limit {
            rcp_builder = rcp_builder.quantity_limit(limit);
        }

        self.base.message.add_segment(rcp_builder.build());

        Ok(self.base.build())
    }
}

/// Builder for QBP^Q21 - Get Person Demographics Query
///
/// Used to retrieve full demographic information for a known person.
pub struct QbpQ21Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    query_name: String,
    query_tag: String,
    qpd_parameters: Vec<String>,
    query_priority: String,
    quantity_limit: Option<String>,
}

impl QbpQ21Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "QBP", "Q21"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            query_name: "Q21^Get Person Demographics^HL7".to_string(),
            query_tag: generate_control_id(),
            qpd_parameters: Vec::new(),
            query_priority: "I".to_string(),
            quantity_limit: Some("1^RD".to_string()),
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

    pub fn query_name(mut self, name: &str) -> Self {
        self.query_name = name.to_string();
        self
    }

    pub fn query_tag(mut self, tag: &str) -> Self {
        self.query_tag = tag.to_string();
        self
    }

    /// Add patient ID parameter
    pub fn patient_id(mut self, id: &str) -> Self {
        self.qpd_parameters.push(id.to_string());
        self
    }

    pub fn parameter(mut self, param: &str) -> Self {
        self.qpd_parameters.push(param.to_string());
        self
    }

    pub fn query_priority(mut self, priority: &str) -> Self {
        self.query_priority = priority.to_string();
        self
    }

    pub fn quantity_limit(mut self, limit: &str) -> Self {
        self.quantity_limit = Some(limit.to_string());
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

        let mut qpd_builder = QpdBuilder::new()
            .message_query_name(&self.query_name)
            .query_tag(&self.query_tag);

        for param in &self.qpd_parameters {
            qpd_builder = qpd_builder.parameter(param);
        }

        self.base.message.add_segment(qpd_builder.build());

        let mut rcp_builder = RcpBuilder::new()
            .query_priority(&self.query_priority);

        if let Some(limit) = &self.quantity_limit {
            rcp_builder = rcp_builder.quantity_limit(limit);
        }

        self.base.message.add_segment(rcp_builder.build());

        Ok(self.base.build())
    }
}

/// Builder for QBP^Q22 - Find Candidates Query
///
/// Used for patient search/matching using query-by-example parameters.
///
/// # Example
/// ```
/// use vyna_hl7_core::{Version, builders::qbp::QbpQ22Builder};
///
/// let message = QbpQ22Builder::new(Version::V2_5_1)
///     .sending_application("CLINREG")
///     .sending_facility("WESTCLIN")
///     .receiving_application("HOSPMPI")
///     .receiving_facility("HOSP")
///     .query_tag("987654321")
///     .parameter("@PID.5.1^SMITH")      // Family name
///     .parameter("@PID.5.2^JOHN")       // Given name
///     .parameter("@PID.7^19850610")     // DOB
///     .parameter("@PID.8^M")            // Sex
///     .quantity_limit("50^RD")
///     .build()
///     .unwrap();
/// ```
pub struct QbpQ22Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    query_name: String,
    query_tag: String,
    qpd_parameters: Vec<String>,
    query_priority: String,
    quantity_limit: Option<String>,
}

impl QbpQ22Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "QBP", "Q22"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            query_name: "Q22^Find Candidates^HL7".to_string(),
            query_tag: generate_control_id(),
            qpd_parameters: Vec::new(),
            query_priority: "I".to_string(),
            quantity_limit: Some("50^RD".to_string()),
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

    pub fn query_name(mut self, name: &str) -> Self {
        self.query_name = name.to_string();
        self
    }

    pub fn query_tag(mut self, tag: &str) -> Self {
        self.query_tag = tag.to_string();
        self
    }

    /// Add a query-by-example parameter (e.g., "@PID.5.1^SMITH")
    pub fn parameter(mut self, param: &str) -> Self {
        self.qpd_parameters.push(param.to_string());
        self
    }

    pub fn query_priority(mut self, priority: &str) -> Self {
        self.query_priority = priority.to_string();
        self
    }

    pub fn quantity_limit(mut self, limit: &str) -> Self {
        self.quantity_limit = Some(limit.to_string());
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

        let mut qpd_builder = QpdBuilder::new()
            .message_query_name(&self.query_name)
            .query_tag(&self.query_tag);

        for param in &self.qpd_parameters {
            qpd_builder = qpd_builder.parameter(param);
        }

        self.base.message.add_segment(qpd_builder.build());

        let mut rcp_builder = RcpBuilder::new()
            .query_priority(&self.query_priority);

        if let Some(limit) = &self.quantity_limit {
            rcp_builder = rcp_builder.quantity_limit(limit);
        }

        self.base.message.add_segment(rcp_builder.build());

        Ok(self.base.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qbp_q11_basic() {
        let message = QbpQ11Builder::new(Version::V2_5_1)
            .sending_application("MyEHR")
            .sending_facility("FAC001")
            .receiving_application("CAIR2")
            .receiving_facility("CAIR2")
            .query_tag("Q123456789")
            .patient_id("234567^^^MYEHR^MR")
            .build()
            .unwrap();

        assert_eq!(message.segments.len(), 3); // MSH, QPD, RCP
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "QPD");
        assert_eq!(message.segments[2].id, "RCP");
    }

    #[test]
    fn test_qbp_q22_find_candidates() {
        let message = QbpQ22Builder::new(Version::V2_5_1)
            .sending_application("CLINREG")
            .receiving_application("HOSPMPI")
            .query_tag("987654321")
            .parameter("@PID.5.1^SMITH")
            .parameter("@PID.5.2^JOHN")
            .build()
            .unwrap();

        assert_eq!(message.segments.len(), 3);
        assert_eq!(message.segments[1].id, "QPD");

        // QPD should have: QPD-1 (query name), QPD-2 (tag), QPD-3+ (parameters)
        assert!(message.segments[1].fields.len() >= 4);
    }

    #[test]
    fn test_qbp_q21_demographics() {
        let message = QbpQ21Builder::new(Version::V2_5)
            .sending_application("APP")
            .sending_facility("FAC")
            .receiving_application("MPI")
            .receiving_facility("HOSP")
            .patient_id("12345^^^MPI^MR")
            .build()
            .unwrap();

        assert_eq!(message.segments.len(), 3);
        assert_eq!(message.segments[0].id, "MSH");
        assert_eq!(message.segments[1].id, "QPD");
        assert_eq!(message.segments[2].id, "RCP");
    }
}
