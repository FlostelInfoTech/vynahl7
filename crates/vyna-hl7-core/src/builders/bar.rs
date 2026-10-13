//! BAR (Billing Account Record) message builders

use super::{MessageBuilder, generate_control_id};
use crate::{Version, error::Result, field::Field, message::Message, segment::Segment};

/// A billing diagnosis (DG1): coded diagnosis plus HL7 table 0052 type
/// (`A` admitting, `W` working, `F` final).
pub struct Diagnosis {
    code: String,
    diagnosis_type: String,
}

impl Diagnosis {
    pub fn new(code: &str, diagnosis_type: &str) -> Self {
        Self {
            code: code.to_string(),
            diagnosis_type: diagnosis_type.to_string(),
        }
    }
}

/// A billing procedure (PR1): coded procedure plus procedure datetime.
pub struct Procedure {
    code: String,
    datetime: String,
}

impl Procedure {
    pub fn new(code: &str, datetime: &str) -> Self {
        Self {
            code: code.to_string(),
            datetime: datetime.to_string(),
        }
    }
}

/// Insurance coverage (IN1): plan ID plus insurance company ID.
pub struct Insurance {
    plan_id: String,
    company_id: String,
}

impl Insurance {
    pub fn new(plan_id: &str, company_id: &str) -> Self {
        Self {
            plan_id: plan_id.to_string(),
            company_id: company_id.to_string(),
        }
    }
}

/// Builder for BAR^P01 - Add Patient Accounts
pub struct BarP01Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    patient_account: Option<String>,
    patient_class: Option<String>,
    assigned_location: Option<String>,
    visit_number: Option<String>,
    event_datetime: Option<String>,
    diagnoses: Vec<Diagnosis>,
    procedures: Vec<Procedure>,
    guarantor_name: Option<(String, String)>,
    insurance: Option<Insurance>,
}

impl BarP01Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "BAR", "P01"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            patient_account: None,
            patient_class: None,
            assigned_location: None,
            visit_number: None,
            event_datetime: None,
            diagnoses: Vec::new(),
            procedures: Vec::new(),
            guarantor_name: None,
            insurance: None,
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

    /// PID-18 patient account number (CX).
    pub fn patient_account(mut self, account: &str) -> Self {
        self.patient_account = Some(account.to_string());
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.patient_class = Some(class.to_string());
        self
    }

    /// PV1-3 assigned patient location (PL, `^`-delimited).
    pub fn assigned_location(mut self, location: &str) -> Self {
        self.assigned_location = Some(location.to_string());
        self
    }

    /// PV1-19 visit number (CX).
    pub fn visit_number(mut self, visit: &str) -> Self {
        self.visit_number = Some(visit.to_string());
        self
    }

    /// EVN-2 recorded datetime (defaults to now).
    pub fn event_datetime(mut self, datetime: &str) -> Self {
        self.event_datetime = Some(datetime.to_string());
        self
    }

    pub fn add_diagnosis(mut self, diagnosis: Diagnosis) -> Self {
        self.diagnoses.push(diagnosis);
        self
    }

    pub fn add_procedure(mut self, procedure: Procedure) -> Self {
        self.procedures.push(procedure);
        self
    }

    pub fn guarantor_name(mut self, family: &str, given: &str) -> Self {
        self.guarantor_name = Some((family.to_string(), given.to_string()));
        self
    }

    pub fn insurance(mut self, insurance: Insurance) -> Self {
        self.insurance = Some(insurance);
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self.control_id.clone().unwrap_or_else(generate_control_id);
        let msh = self.base.create_msh(
            &self.sending_app,
            &self.sending_facility,
            &self.receiving_app,
            &self.receiving_facility,
            &control_id,
            "P",
        )?;
        self.base.message.add_segment(msh);

        let evn = self
            .base
            .create_evn(self.base.trigger_event(), self.event_datetime.as_deref())?;
        self.base.message.add_segment(evn);

        // PID segment
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        pid.add_field(Field::from_value(""));
        pid.add_field(Field::from_value(self.patient_id.as_deref().unwrap_or("")));
        pid.add_field(Field::from_value(""));

        if self.patient_account.is_some() {
            // PID-5 must exist (even empty) so the PID-18 padding lands.
            if let Some((family, given)) = &self.patient_name {
                pid.add_field(Field::from_delimited(format!("{family}^{given}")));
            } else {
                pid.add_field(Field::from_value(""));
            }
            for _ in 6..18 {
                pid.add_field(Field::from_value(""));
            }
            pid.add_field(Field::from_delimited(
                self.patient_account.as_deref().unwrap_or(""),
            )); // PID-18: Patient Account
        } else if let Some((family, given)) = &self.patient_name {
            pid.add_field(Field::from_delimited(format!("{family}^{given}")));
        }

        self.base.message.add_segment(pid);

        // PV1 segment (visit): only when a patient class is set.
        if let Some(class) = &self.patient_class {
            let mut pv1 = Segment::new("PV1");
            pv1.add_field(Field::from_value("1")); // PV1-1: Set ID
            pv1.add_field(Field::from_value(class)); // PV1-2: Patient Class
            pv1.add_field(Field::from_delimited(
                self.assigned_location.as_deref().unwrap_or(""),
            )); // PV1-3: Assigned Patient Location
            if self.visit_number.is_some() {
                for _ in 4..19 {
                    pv1.add_field(Field::from_value(""));
                }
                pv1.add_field(Field::from_delimited(
                    self.visit_number.as_deref().unwrap_or(""),
                )); // PV1-19: Visit Number
            }
            self.base.message.add_segment(pv1);
        }

        // DG1 segments (diagnoses)
        for (i, diagnosis) in self.diagnoses.iter().enumerate() {
            let mut dg1 = Segment::new("DG1");
            dg1.add_field(Field::from_value((i + 1).to_string())); // DG1-1: Set ID
            dg1.add_field(Field::from_value("")); // DG1-2: Diagnosis Coding Method
            dg1.add_field(Field::from_delimited(&diagnosis.code)); // DG1-3: Diagnosis Code
            dg1.add_field(Field::from_value("")); // DG1-4: Diagnosis Description
            dg1.add_field(Field::from_value("")); // DG1-5: Diagnosis Date/Time
            dg1.add_field(Field::from_value(&diagnosis.diagnosis_type)); // DG1-6: Diagnosis Type
            self.base.message.add_segment(dg1);
        }

        // PR1 segments (procedures)
        for (i, procedure) in self.procedures.iter().enumerate() {
            let mut pr1 = Segment::new("PR1");
            pr1.add_field(Field::from_value((i + 1).to_string())); // PR1-1: Set ID
            pr1.add_field(Field::from_value("")); // PR1-2: Procedure Coding Method
            pr1.add_field(Field::from_delimited(&procedure.code)); // PR1-3: Procedure Code
            pr1.add_field(Field::from_value("")); // PR1-4: Procedure Description
            pr1.add_field(Field::from_value(&procedure.datetime)); // PR1-5: Procedure Date/Time
            self.base.message.add_segment(pr1);
        }

        // GT1 segment (guarantor)
        if let Some((family, given)) = &self.guarantor_name {
            let mut gt1 = Segment::new("GT1");
            gt1.add_field(Field::from_value("1")); // GT1-1: Set ID
            gt1.add_field(Field::from_value("")); // GT1-2: Guarantor Number
            gt1.add_field(Field::from_delimited(format!("{family}^{given}"))); // GT1-3: Guarantor Name
            self.base.message.add_segment(gt1);
        }

        // IN1 segment (insurance)
        if let Some(insurance) = &self.insurance {
            let mut in1 = Segment::new("IN1");
            in1.add_field(Field::from_value("1")); // IN1-1: Set ID
            in1.add_field(Field::from_delimited(&insurance.plan_id)); // IN1-2: Insurance Plan ID
            in1.add_field(Field::from_delimited(&insurance.company_id)); // IN1-3: Insurance Company ID
            self.base.message.add_segment(in1);
        }

        Ok(self.base.build())
    }
}

/// Builder for BAR^P02 - Purge Patient Accounts
pub struct BarP02Builder {
    base: BarP01Builder,
}

impl BarP02Builder {
    pub fn new(version: Version) -> Self {
        let mut base = BarP01Builder::new(version);
        base.base = MessageBuilder::new(version, "BAR", "P02");
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

    pub fn control_id(mut self, id: &str) -> Self {
        self.base = self.base.control_id(id);
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

    pub fn patient_account(mut self, account: &str) -> Self {
        self.base = self.base.patient_account(account);
        self
    }

    pub fn patient_class(mut self, class: &str) -> Self {
        self.base = self.base.patient_class(class);
        self
    }

    pub fn assigned_location(mut self, location: &str) -> Self {
        self.base = self.base.assigned_location(location);
        self
    }

    pub fn visit_number(mut self, visit: &str) -> Self {
        self.base = self.base.visit_number(visit);
        self
    }

    pub fn event_datetime(mut self, datetime: &str) -> Self {
        self.base = self.base.event_datetime(datetime);
        self
    }

    pub fn add_diagnosis(mut self, diagnosis: Diagnosis) -> Self {
        self.base = self.base.add_diagnosis(diagnosis);
        self
    }

    pub fn add_procedure(mut self, procedure: Procedure) -> Self {
        self.base = self.base.add_procedure(procedure);
        self
    }

    pub fn guarantor_name(mut self, family: &str, given: &str) -> Self {
        self.base = self.base.guarantor_name(family, given);
        self
    }

    pub fn insurance(mut self, insurance: Insurance) -> Self {
        self.base = self.base.insurance(insurance);
        self
    }

    pub fn build(self) -> Result<Message> {
        self.base.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(msg: &Message, id: &str) -> String {
        msg.encode()
            .split(['\r', '\n'])
            .find(|l| l.starts_with(id))
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn bar_p01_builds_msh_evn_pid() {
        let msg = BarP01Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .control_id("BAR1")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .build()
            .unwrap();
        let wire = msg.encode();
        assert!(wire.contains("BAR^P01"), "{wire}");
        assert!(segment(&msg, "PID|").contains("Smith^John"));
        let evn = segment(&msg, "EVN|");
        assert!(evn.starts_with("EVN|P01|"), "{evn}");
    }

    #[test]
    fn bar_p01_billing_segments_land() {
        let msg = BarP01Builder::new(Version::V2_5)
            .patient_id("12345")
            .patient_name("Smith", "John")
            .patient_class("I")
            .add_diagnosis(Diagnosis::new("I10^Essential hypertension^I10", "F"))
            .add_procedure(Procedure::new(
                "C4^Coronary angiography^C4",
                "20240115130000",
            ))
            .guarantor_name("Smith", "John")
            .insurance(Insurance::new("PPO^Preferred^L", "BCBS001"))
            .build()
            .unwrap();
        let wire = msg.encode();
        for id in ["PV1|", "DG1|", "PR1|", "GT1|", "IN1|"] {
            assert!(wire.contains(id), "{id} missing: {wire}");
        }
        assert!(segment(&msg, "DG1|").contains("|F"), "{wire}");
    }

    #[test]
    fn bar_p02_repurposes_p01_shape() {
        let msg = BarP02Builder::new(Version::V2_5)
            .patient_id("12345")
            .patient_name("Smith", "John")
            .build()
            .unwrap();
        let wire = msg.encode();
        assert!(wire.contains("BAR^P02"), "{wire}");
        assert!(segment(&msg, "EVN|").starts_with("EVN|P02|"));
    }
}
