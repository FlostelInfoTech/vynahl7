//! VXU^V04 builder - Unsolicited Vaccination Record Update

use super::{MessageBuilder, generate_control_id};
use crate::{Version, error::Result, field::Field, message::Message, segment::Segment};

/// One immunization administration: an ORC/RXA group with an optional RXR.
///
/// `ImmunizationDose::new` takes the administered code as `code^text^system`
/// (e.g. `08^HepB adult^CVX`) and the administration start time. RXA-4
/// (end time) defaults to the start time and RXA-20 (completion status)
/// defaults to `CP` unless overridden.
pub struct ImmunizationDose {
    placer_order_number: Option<String>,
    order_control: Option<String>,
    give_sub_id: String,
    administration_sub_id: String,
    datetime_start: String,
    datetime_end: Option<String>,
    administered_code: String,
    amount: Option<String>,
    units: Option<String>,
    lot_number: Option<String>,
    expiration_date: Option<String>,
    manufacturer: Option<String>,
    completion_status: String,
    route: Option<String>,
    site: Option<String>,
}

impl ImmunizationDose {
    pub fn new(administered_code: &str, datetime_start: &str) -> Self {
        Self {
            placer_order_number: None,
            order_control: None,
            give_sub_id: "0".to_string(),
            administration_sub_id: "1".to_string(),
            datetime_start: datetime_start.to_string(),
            datetime_end: None,
            administered_code: administered_code.to_string(),
            amount: None,
            units: None,
            lot_number: None,
            expiration_date: None,
            manufacturer: None,
            completion_status: "CP".to_string(),
            route: None,
            site: None,
        }
    }

    pub fn placer_order_number(mut self, number: &str) -> Self {
        self.placer_order_number = Some(number.to_string());
        self
    }

    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = Some(control.to_string());
        self
    }

    pub fn datetime_end(mut self, datetime: &str) -> Self {
        self.datetime_end = Some(datetime.to_string());
        self
    }

    pub fn amount(mut self, amount: &str) -> Self {
        self.amount = Some(amount.to_string());
        self
    }

    pub fn units(mut self, units: &str) -> Self {
        self.units = Some(units.to_string());
        self
    }

    pub fn lot_number(mut self, lot: &str) -> Self {
        self.lot_number = Some(lot.to_string());
        self
    }

    pub fn expiration_date(mut self, date: &str) -> Self {
        self.expiration_date = Some(date.to_string());
        self
    }

    pub fn manufacturer(mut self, manufacturer: &str) -> Self {
        self.manufacturer = Some(manufacturer.to_string());
        self
    }

    pub fn completion_status(mut self, status: &str) -> Self {
        self.completion_status = status.to_string();
        self
    }

    pub fn route(mut self, route: &str) -> Self {
        self.route = Some(route.to_string());
        self
    }

    /// Administration site (RXR-2). Only emitted when [`route`](Self::route)
    /// is also set, because RXR-1 is required whenever RXR is present.
    pub fn site(mut self, site: &str) -> Self {
        self.site = Some(site.to_string());
        self
    }
}

/// Set a segment field, ignoring the impossible index-0 error.
fn set_field(segment: &mut Segment, index: usize, value: &str) {
    let _ = segment.set_field(index, Field::from_value(value));
}
/// Set a segment field from HL7-delimited content (`~`, `^`, `&` are split).
fn set_delimited_field(segment: &mut Segment, index: usize, value: &str) {
    let _ = segment.set_field(index, Field::from_delimited(value));
}

/// Builder for VXU^V04 - Unsolicited Vaccination Record Update
pub struct VxuV04Builder {
    base: MessageBuilder,
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,
    control_id: Option<String>,
    patient_id: Option<String>,
    patient_name: Option<(String, String)>,
    date_of_birth: Option<String>,
    sex: Option<String>,
    order_control: String,
    doses: Vec<ImmunizationDose>,
    extra_segments: Vec<Segment>,
}

impl VxuV04Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "VXU", "V04"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            date_of_birth: None,
            sex: None,
            order_control: "RE".to_string(),
            doses: Vec::new(),
            extra_segments: Vec::new(),
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

    pub fn date_of_birth(mut self, dob: &str) -> Self {
        self.date_of_birth = Some(dob.to_string());
        self
    }

    pub fn sex(mut self, sex: &str) -> Self {
        self.sex = Some(sex.to_string());
        self
    }

    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }

    /// Add one immunization administration (ORC/RXA, plus RXR when set).
    pub fn add_immunization(mut self, dose: ImmunizationDose) -> Self {
        self.doses.push(dose);
        self
    }

    /// Append a caller-built segment (e.g. PV1, OBX, NTE) after the orders.
    pub fn add_segment(mut self, segment: Segment) -> Self {
        self.extra_segments.push(segment);
        self
    }

    pub fn build(mut self) -> Result<Message> {
        let control_id = self.control_id.take().unwrap_or_else(generate_control_id);

        let msh = self.base.create_msh(
            &self.sending_app,
            &self.sending_facility,
            &self.receiving_app,
            &self.receiving_facility,
            &control_id,
            "P",
        )?;
        self.base.message.add_segment(msh);

        // PID segment. Sparse on purpose: unset fields stay absent instead
        // of "" placeholders, so optional composite fields (PID-2, PID-4)
        // do not trip required-component checks during validation.
        let mut pid = Segment::new("PID");
        set_field(&mut pid, 1, "1");
        if let Some(id) = self.patient_id.as_deref() {
            set_field(&mut pid, 3, id);
        }
        if let Some((family, given)) = self.patient_name.as_ref() {
            set_delimited_field(&mut pid, 5, &format!("{family}^{given}"));
        }
        if let Some(dob) = self.date_of_birth.as_deref() {
            set_field(&mut pid, 7, dob);
        }
        if let Some(sex) = self.sex.as_deref() {
            set_field(&mut pid, 8, sex);
        }
        self.base.message.add_segment(pid);

        for dose in &self.doses {
            // ORC segment
            let mut orc = Segment::new("ORC");
            let control = dose.order_control.as_deref().unwrap_or(&self.order_control);
            orc.add_field(Field::from_value(control)); // ORC-1
            orc.add_field(Field::from_delimited(
                dose.placer_order_number.as_deref().unwrap_or(""),
            )); // ORC-2
            self.base.message.add_segment(orc);

            // RXA segment (fields 1-22, positional)
            let mut rxa = Segment::new("RXA");
            let values: [&str; 22] = [
                &dose.give_sub_id,                                            // 1
                &dose.administration_sub_id,                                  // 2
                &dose.datetime_start,                                         // 3
                dose.datetime_end.as_deref().unwrap_or(&dose.datetime_start), // 4
                &dose.administered_code,                                      // 5
                dose.amount.as_deref().unwrap_or(""),                         // 6
                dose.units.as_deref().unwrap_or(""),                          // 7
                "",                                                           // 8
                "",                                                           // 9
                "",                                                           // 10
                "",                                                           // 11
                "",                                                           // 12
                "",                                                           // 13
                "",                                                           // 14
                dose.lot_number.as_deref().unwrap_or(""),                     // 15
                dose.expiration_date.as_deref().unwrap_or(""),                // 16
                dose.manufacturer.as_deref().unwrap_or(""),                   // 17
                "",                                                           // 18
                "",                                                           // 19
                &dose.completion_status,                                      // 20
                "",                                                           // 21
                "",                                                           // 22
            ];
            for value in values {
                rxa.add_field(Field::from_delimited(value));
            }
            self.base.message.add_segment(rxa);

            // RXR segment (only when a route is set; RXR-1 is required)
            if let Some(route) = &dose.route {
                let mut rxr = Segment::new("RXR");
                rxr.add_field(Field::from_delimited(route)); // RXR-1
                rxr.add_field(Field::from_delimited(dose.site.as_deref().unwrap_or(""))); // RXR-2
                self.base.message.add_segment(rxr);
            }
        }

        for segment in self.extra_segments.drain(..) {
            self.base.message.add_segment(segment);
        }

        Ok(self.base.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_builder() -> VxuV04Builder {
        VxuV04Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .control_id("MSG001")
            .patient_id("12345")
            .patient_name("Smith", "John")
    }

    #[test]
    fn test_vxu_builds_msh_pid_and_order() {
        let message = minimal_builder()
            .add_immunization(
                ImmunizationDose::new("08^HepB adult^CVX", "20240101100000")
                    .amount("1.0")
                    .units("mL"),
            )
            .build()
            .unwrap();

        let msh = message.get_segment_by_id("MSH").unwrap();
        // MSH-9 carries proper component structure (VXU^V04), so read it whole.
        let msg_type = msh
            .get_field(9)
            .unwrap()
            .get_repetition(0)
            .unwrap()
            .full_value();
        assert_eq!(msg_type.as_deref(), Some("VXU^V04"));
        assert_eq!(msh.get_field_value(10), Some("MSG001"));
        assert_eq!(msh.get_field_value(12), Some("2.5"));

        let pid = message.get_segment_by_id("PID").unwrap();
        assert_eq!(pid.get_field_value(3), Some("12345"));
        assert_eq!(pid.get_field_value(5), Some("Smith"));
        assert_eq!(
            pid.get_field(5)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("John")
        );

        let orc = message.get_segment_by_id("ORC").unwrap();
        assert_eq!(orc.get_field_value(1), Some("RE"));

        let rxa = message.get_segment_by_id("RXA").unwrap();
        assert_eq!(rxa.get_field_value(1), Some("0"));
        assert_eq!(rxa.get_field_value(3), Some("20240101100000"));
        // RXA-4 defaults to the start time; RXA-20 defaults to CP.
        assert_eq!(rxa.get_field_value(4), Some("20240101100000"));
        assert_eq!(rxa.get_field_value(5), Some("08"));
        assert_eq!(
            rxa.get_field(5)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("HepB adult")
        );
        assert_eq!(rxa.get_field_value(6), Some("1.0"));
        assert_eq!(rxa.get_field_value(20), Some("CP"));
    }

    #[test]
    fn test_vxu_multiple_doses_stay_in_order() {
        let message = minimal_builder()
            .add_immunization(
                ImmunizationDose::new("08^HepB adult^CVX", "20240101100000")
                    .placer_order_number("IZ-1"),
            )
            .add_immunization(
                ImmunizationDose::new("20^DTaP^CVX", "20240101101500").placer_order_number("IZ-2"),
            )
            .build()
            .unwrap();

        let rxas = message.get_segments_by_id("RXA");
        assert_eq!(rxas.len(), 2);
        assert_eq!(rxas[0].get_field_value(5), Some("08"));
        assert_eq!(
            rxas[0]
                .get_field(5)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("HepB adult")
        );
        assert_eq!(rxas[1].get_field_value(5), Some("20"));
        assert_eq!(
            rxas[1]
                .get_field(5)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("DTaP")
        );

        let orcs = message.get_segments_by_id("ORC");
        assert_eq!(orcs.len(), 2);
        assert_eq!(orcs[0].get_field_value(2), Some("IZ-1"));
        assert_eq!(orcs[1].get_field_value(2), Some("IZ-2"));
    }

    #[test]
    fn test_vxu_rxr_only_with_route() {
        let with_route = minimal_builder()
            .add_immunization(
                ImmunizationDose::new("08^HepB adult^CVX", "20240101100000")
                    .route("IM^Intramuscular^HL70162")
                    .site("LD^Left Deltoid^HL70163"),
            )
            .build()
            .unwrap();
        let rxr = with_route.get_segment_by_id("RXR").unwrap();
        assert_eq!(rxr.get_field_value(1), Some("IM"));
        assert_eq!(
            rxr.get_field(1)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("Intramuscular")
        );
        assert_eq!(rxr.get_field_value(2), Some("LD"));
        assert_eq!(
            rxr.get_field(2)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("Left Deltoid")
        );

        // Site alone must not emit RXR: RXR-1 is required when present.
        let site_only = minimal_builder()
            .add_immunization(
                ImmunizationDose::new("08^HepB adult^CVX", "20240101100000").site("LD"),
            )
            .build()
            .unwrap();
        assert!(site_only.get_segment_by_id("RXR").is_none());

        let no_route = minimal_builder()
            .add_immunization(ImmunizationDose::new("08^HepB adult^CVX", "20240101100000"))
            .build()
            .unwrap();
        assert!(no_route.get_segment_by_id("RXR").is_none());
    }

    #[test]
    fn test_vxu_lot_manufacturer_and_refusal() {
        let message = minimal_builder()
            .add_immunization(
                ImmunizationDose::new("08^HepB adult^CVX", "20240101100000")
                    .lot_number("LOT123")
                    .expiration_date("20251231")
                    .manufacturer("MFR123^Merck^MVX")
                    .completion_status("RE"),
            )
            .build()
            .unwrap();

        let rxa = message.get_segment_by_id("RXA").unwrap();
        assert_eq!(rxa.get_field_value(15), Some("LOT123"));
        assert_eq!(rxa.get_field_value(16), Some("20251231"));
        assert_eq!(rxa.get_field_value(17), Some("MFR123"));
        assert_eq!(
            rxa.get_field(17)
                .unwrap()
                .get_component(0, 1)
                .unwrap()
                .value(),
            Some("Merck")
        );
        assert_eq!(rxa.get_field_value(20), Some("RE"));
    }
}
