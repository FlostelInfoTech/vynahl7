//! ORU (Observation Result) message builders

use super::{generate_control_id, MessageBuilder};
use crate::{
    error::Result,
    field::Field,
    message::Message,
    segment::Segment,
    Version,
};

/// Builder for ORU^R01 - Unsolicited Observation Message
pub struct OruR01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    processing_id: String,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    #[allow(dead_code)]
    order_control: String,
    placer_order_number: Option<String>,
    filler_order_number: Option<String>,
    universal_service_id: Option<String>,
    result_status: Option<String>,
    visit_number: Option<String>,
    observations: Vec<Observation>,
}

pub struct Observation {
    pub set_id: u32,
    pub value_type: String,
    pub identifier: String,
    /// Observation value. Takes the shape of `value_type`: free-text types
    /// (`ST`/`TX`/`FT`) stay literal, every other type splits `~`/`^`/`&`
    /// into structure (atomic values are unaffected by splitting).
    pub value: String,
    pub units: Option<String>,
    pub status: String,
}

/// OBX-5 takes the shape of OBX-2: free-text types stay literal (delimiter
/// characters are data and must escape on the wire), every other type splits
/// `~`/`^`/`&` into structure.
fn observation_value_field(value_type: &str, value: &str) -> Field {
    match value_type {
        "ST" | "TX" | "FT" => Field::from_value(value),
        _ => Field::from_delimited(value),
    }
}

impl OruR01Builder {
    /// Create a new ORU^R01 builder
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "ORU", "R01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            processing_id: "P".to_string(),
            patient_id: None,
            patient_name: None,
            order_control: "RE".to_string(),
            placer_order_number: None,
            filler_order_number: None,
            universal_service_id: None,
            result_status: None,
            visit_number: None,
            observations: Vec::new(),
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

    /// OBR-2: the placer-issued order number (e.g. the HIS requisition
    /// the filler echoes back so the placer can close the loop).
    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.placer_order_number = Some(number.to_string());
        self
    }

    pub fn filler_order_number(mut self, number: &str) -> Self {
        self.filler_order_number = Some(number.to_string());
        self
    }

    /// OBR-4: Universal Service ID (required by the schema).
    pub fn universal_service_id(mut self, id: &str) -> Self {
        self.universal_service_id = Some(id.to_string());
        self
    }

    /// OBR-25: result status (`F` final, `C` corrected, `X` canceled...).
    /// Unset by default, in which case the segment stays short and
    /// previously emitted bytes are unchanged.
    pub fn result_status(mut self, status: &str) -> Self {
        self.result_status = Some(status.to_string());
        self
    }

    /// PV1-19: the visit/encounter number the result files under
    /// (unsolicited filing keys on it). Unset by default, in which
    /// case no PV1 segment is emitted at all.
    pub fn visit_number(mut self, number: &str) -> Self {
        self.visit_number = Some(number.to_string());
        self
    }

    /// Add an observation
    pub fn add_observation(mut self, obs: Observation) -> Self {
        self.observations.push(obs);
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
            pid.add_field(Field::from_delimited(&name));
        } else {
            pid.add_field(Field::from_value(""));
        }

        self.base.message.add_segment(pid);

        // Create PV1 segment (visit): only when a visit number is set,
        // so messages without one keep their exact prior bytes.
        if let Some(visit) = self.visit_number.as_deref() {
            let mut pv1 = Segment::new("PV1");
            pv1.add_field(Field::from_value("1")); // PV1-1: Set ID
            for _ in 2..19 {
                pv1.add_field(Field::from_value(""));
            }
            pv1.add_field(Field::from_delimited(visit)); // PV1-19: Visit Number
            self.base.message.add_segment(pv1);
        }

        // Create OBR segment
        let mut obr = Segment::new("OBR");
        obr.add_field(Field::from_value("1")); // OBR-1: Set ID
        obr.add_field(Field::from_delimited(self.placer_order_number.as_deref().unwrap_or(""))); // OBR-2: Placer Order Number
        obr.add_field(Field::from_delimited(self.filler_order_number.as_deref().unwrap_or(""))); // OBR-3: Filler Order Number
        obr.add_field(Field::from_delimited(self.universal_service_id.as_deref().unwrap_or(""))); // OBR-4: Universal Service ID
        if let Some(status) = self.result_status.as_deref() {
            for _ in 5..25 {
                obr.add_field(Field::from_value(""));
            }
            obr.add_field(Field::from_value(status)); // OBR-25: Result Status
        }

        self.base.message.add_segment(obr);

        // Create OBX segments for observations
        for obs in &self.observations {
            let mut obx = Segment::new("OBX");
            obx.add_field(Field::from_value(obs.set_id.to_string())); // OBX-1: Set ID
            obx.add_field(Field::from_value(&obs.value_type)); // OBX-2: Value Type
            obx.add_field(Field::from_delimited(&obs.identifier)); // OBX-3: Observation Identifier
            obx.add_field(Field::from_value("")); // OBX-4: Observation Sub-ID
            obx.add_field(observation_value_field(&obs.value_type, &obs.value)); // OBX-5: Observation Value

            if let Some(units) = &obs.units {
                obx.add_field(Field::from_delimited(units)); // OBX-6: Units
            } else {
                obx.add_field(Field::from_value(""));
            }

            obx.add_field(Field::from_value("")); // OBX-7: References Range
            obx.add_field(Field::from_value("")); // OBX-8: Abnormal Flags
            obx.add_field(Field::from_value("")); // OBX-9: Probability
            obx.add_field(Field::from_value("")); // OBX-10: Nature of Abnormal Test
            obx.add_field(Field::from_value(&obs.status)); // OBX-11: Observation Result Status

            self.base.message.add_segment(obx);
        }

        Ok(self.base.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obr_segment(msg: &Message) -> String {
        msg.encode()
            .split(['\r', '\n'])
            .find(|l| l.starts_with("OBR|"))
            .unwrap_or("")
            .to_string()
    }

    fn pv1_segment(msg: &Message) -> Option<String> {
        msg.encode()
            .split(['\r', '\n'])
            .find(|l| l.starts_with("PV1|"))
            .map(str::to_string)
    }

    #[test]
    fn placer_and_filler_numbers_land_in_obr_2_and_3() {
        let msg = OruR01Builder::new(Version::V2_5)
            .sending_application("VynaECG")
            .sending_facility("VynaECG")
            .receiving_application("HIS")
            .receiving_facility("HIS")
            .control_id("VYN-1-1")
            .patient_id("P1")
            .patient_name("F", "G")
            .placer_order_number("1811652775")
            .filler_order_number("VYN-ACC-1")
            .add_observation(Observation {
                set_id: 1,
                value_type: "ST".to_string(),
                identifier: "ECG^Interpretation".to_string(),
                value: "Normal".to_string(),
                units: None,
                status: "F".to_string(),
            })
            .build()
            .unwrap();
        let obr = obr_segment(&msg);
        let fields: Vec<&str> = obr.split('|').collect();
        // fields[0] == "OBR", so OBR-2 is index 2, OBR-3 index 3.
        assert_eq!(fields[2], "1811652775");
        assert_eq!(fields[3], "VYN-ACC-1");
    }

    #[test]
    fn result_status_lands_in_obr_25() {
        let msg = OruR01Builder::new(Version::V2_5)
            .control_id("VYN-1-CANCEL")
            .placer_order_number("1811652775")
            .filler_order_number("VYN-ACC-1")
            .result_status("X")
            .build()
            .unwrap();
        let obr = obr_segment(&msg);
        let fields: Vec<&str> = obr.split('|').collect();
        assert_eq!(fields[2], "1811652775");
        assert_eq!(fields[3], "VYN-ACC-1");
        assert_eq!(fields[25], "X");
    }

    #[test]
    fn obr_stays_short_without_result_status() {
        let msg = OruR01Builder::new(Version::V2_5)
            .control_id("VYN-1-1")
            .filler_order_number("VYN-ACC-1")
            .build()
            .unwrap();
        let obr = obr_segment(&msg);
        assert_eq!(obr, "OBR|1||VYN-ACC-1|");
    }

    #[test]
    fn visit_number_lands_in_pv1_19() {
        let msg = OruR01Builder::new(Version::V2_5)
            .control_id("VYN-1-1")
            .patient_id("P1")
            .filler_order_number("VYN-ACC-1")
            .visit_number("100287442245")
            .build()
            .unwrap();
        let pv1 = pv1_segment(&msg).expect("PV1 emitted");
        let fields: Vec<&str> = pv1.split('|').collect();
        assert_eq!(fields[1], "1");
        assert_eq!(fields[19], "100287442245");
        // PV1 sits between PID and OBR.
        let wire = msg.encode();
        let pid = wire.find("\rPID|").unwrap();
        let pv1_pos = wire.find("\rPV1|").unwrap();
        let obr = wire.find("\rOBR|").unwrap();
        assert!(pid < pv1_pos && pv1_pos < obr);
    }

    #[test]
    fn no_pv1_without_visit_number() {
        let msg = OruR01Builder::new(Version::V2_5)
            .control_id("VYN-1-1")
            .filler_order_number("VYN-ACC-1")
            .build()
            .unwrap();
        assert_eq!(pv1_segment(&msg), None);
    }

    #[test]
    fn obr_2_stays_empty_without_placer() {
        let msg = OruR01Builder::new(Version::V2_5)
            .control_id("VYN-1-1")
            .filler_order_number("VYN-ACC-1")
            .build()
            .unwrap();
        let obr = obr_segment(&msg);
        let fields: Vec<&str> = obr.split('|').collect();
        assert_eq!(fields[2], "");
        assert_eq!(fields[3], "VYN-ACC-1");
    }

    fn obx_lines(msg: &Message) -> Vec<String> {
        msg.encode()
            .split(['\r', '\n'])
            .filter(|l| l.starts_with("OBX|"))
            .map(str::to_string)
            .collect()
    }

    fn observation(value_type: &str, value: &str) -> Observation {
        Observation {
            set_id: 1,
            value_type: value_type.to_string(),
            identifier: "8867-4^Heart rate^LN".to_string(),
            value: value.to_string(),
            units: None,
            status: "F".to_string(),
        }
    }

    #[test]
    fn coded_obx5_splits_into_structure() {
        let msg = OruR01Builder::new(Version::V2_5)
            .add_observation(observation("CWE", "POS^Positive^L"))
            .build()
            .unwrap();
        let lines = obx_lines(&msg);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("|POS^Positive^L|"), "{}", lines[0]);
        let obx = msg.get_segment_by_id("OBX").unwrap();
        assert_eq!(
            obx.get_field(5)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("Positive")
        );
    }

    #[test]
    fn text_obx5_keeps_delimiters_literal() {
        let msg = OruR01Builder::new(Version::V2_5)
            .add_observation(observation("TX", "R&D follow-up"))
            .build()
            .unwrap();
        let lines = obx_lines(&msg);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("R\\T\\D follow-up"), "{}", lines[0]);
    }
}
