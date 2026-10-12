//! OMP^O09 builder - Pharmacy/Treatment Order

use super::{MessageBuilder, generate_control_id};
use crate::{Version, error::Result, field::Field, message::Message, segment::Segment};

/// One medication order: an ORC/RXO/RXR group.
///
/// `MedicationOrder::new` takes the requested give code as `code^text^system`
/// (e.g. `00093-0058^Lorazepam^NDC`) and the route (e.g. `PO^Oral^HL70162`).
/// The route is required because RXR is a required segment in OMP orders.
pub struct MedicationOrder {
    placer_order_number: Option<String>,
    order_control: Option<String>,
    give_code: String,
    amount_minimum: Option<String>,
    amount_maximum: Option<String>,
    units: Option<String>,
    dosage_form: Option<String>,
    refills: Option<String>,
    route: String,
    site: Option<String>,
}

impl MedicationOrder {
    pub fn new(give_code: &str, route: &str) -> Self {
        Self {
            placer_order_number: None,
            order_control: None,
            give_code: give_code.to_string(),
            amount_minimum: None,
            amount_maximum: None,
            units: None,
            dosage_form: None,
            refills: None,
            route: route.to_string(),
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

    pub fn amount_minimum(mut self, amount: &str) -> Self {
        self.amount_minimum = Some(amount.to_string());
        self
    }

    pub fn amount_maximum(mut self, amount: &str) -> Self {
        self.amount_maximum = Some(amount.to_string());
        self
    }

    pub fn units(mut self, units: &str) -> Self {
        self.units = Some(units.to_string());
        self
    }

    pub fn dosage_form(mut self, form: &str) -> Self {
        self.dosage_form = Some(form.to_string());
        self
    }

    pub fn refills(mut self, refills: &str) -> Self {
        self.refills = Some(refills.to_string());
        self
    }

    pub fn site(mut self, site: &str) -> Self {
        self.site = Some(site.to_string());
        self
    }
}

/// Set a segment field, ignoring the impossible index-0 error.
fn set_field(segment: &mut Segment, index: usize, value: &str) {
    let _ = segment.set_field(index, Field::from_value(value));
}

/// Builder for OMP^O09 - Pharmacy/Treatment Order
pub struct OmpO09Builder {
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
    patient_class: Option<String>,
    order_control: String,
    orders: Vec<MedicationOrder>,
    extra_segments: Vec<Segment>,
}

impl OmpO09Builder {
    pub fn new(version: Version) -> Self {
        Self {
            base: MessageBuilder::new(version, "OMP", "O09"),
            sending_app: String::new(),
            sending_facility: String::new(),
            receiving_app: String::new(),
            receiving_facility: String::new(),
            control_id: None,
            patient_id: None,
            patient_name: None,
            date_of_birth: None,
            sex: None,
            patient_class: None,
            order_control: "NW".to_string(),
            orders: Vec::new(),
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

    /// PV1-2 patient class (required by the schema when PV1 is validated).
    pub fn patient_class(mut self, class: &str) -> Self {
        self.patient_class = Some(class.to_string());
        self
    }

    pub fn order_control(mut self, control: &str) -> Self {
        self.order_control = control.to_string();
        self
    }

    /// Add one medication order (ORC/RXO/RXR).
    pub fn add_order(mut self, order: MedicationOrder) -> Self {
        self.orders.push(order);
        self
    }

    /// Append a caller-built segment (e.g. OBX, NTE, FT1) after the orders.
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

        // PID segment, sparse: unset fields stay absent instead of ""
        // placeholders, so optional composite fields do not trip
        // required-component checks during validation.
        let mut pid = Segment::new("PID");
        set_field(&mut pid, 1, "1");
        if let Some(id) = self.patient_id.as_deref() {
            set_field(&mut pid, 3, id);
        }
        if let Some((family, given)) = self.patient_name.as_ref() {
            set_field(&mut pid, 5, &format!("{family}^{given}"));
        }
        if let Some(dob) = self.date_of_birth.as_deref() {
            set_field(&mut pid, 7, dob);
        }
        if let Some(sex) = self.sex.as_deref() {
            set_field(&mut pid, 8, sex);
        }
        self.base.message.add_segment(pid);

        // PV1 segment (PV1-2 patient class is schema-required)
        let mut pv1 = Segment::new("PV1");
        set_field(&mut pv1, 1, "1");
        if let Some(class) = self.patient_class.as_deref() {
            set_field(&mut pv1, 2, class);
        }
        self.base.message.add_segment(pv1);

        for order in &self.orders {
            let mut orc = Segment::new("ORC");
            let control = order
                .order_control
                .as_deref()
                .unwrap_or(&self.order_control);
            set_field(&mut orc, 1, control);
            if let Some(placer) = order.placer_order_number.as_deref() {
                set_field(&mut orc, 2, placer);
            }
            self.base.message.add_segment(orc);

            let mut rxo = Segment::new("RXO");
            set_field(&mut rxo, 1, &order.give_code);
            if let Some(amount) = order.amount_minimum.as_deref() {
                set_field(&mut rxo, 2, amount);
            }
            if let Some(amount) = order.amount_maximum.as_deref() {
                set_field(&mut rxo, 3, amount);
            }
            if let Some(units) = order.units.as_deref() {
                set_field(&mut rxo, 4, units);
            }
            if let Some(form) = order.dosage_form.as_deref() {
                set_field(&mut rxo, 5, form);
            }
            if let Some(refills) = order.refills.as_deref() {
                set_field(&mut rxo, 13, refills);
            }
            self.base.message.add_segment(rxo);

            let mut rxr = Segment::new("RXR");
            set_field(&mut rxr, 1, &order.route);
            if let Some(site) = order.site.as_deref() {
                set_field(&mut rxr, 2, site);
            }
            self.base.message.add_segment(rxr);
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

    fn minimal_builder() -> OmpO09Builder {
        OmpO09Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .control_id("MSG001")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .patient_class("O")
    }

    #[test]
    fn test_omp_builds_msh_pid_pv1_and_order() {
        let message = minimal_builder()
            .add_order(MedicationOrder::new(
                "00093-0058^Lorazepam^NDC",
                "PO^Oral^HL70162",
            ))
            .build()
            .unwrap();

        let msh = message.get_segment_by_id("MSH").unwrap();
        let msg_type = msh
            .get_field(9)
            .unwrap()
            .get_repetition(0)
            .unwrap()
            .full_value();
        assert_eq!(msg_type.as_deref(), Some("OMP^O09"));

        let pid = message.get_segment_by_id("PID").unwrap();
        assert_eq!(pid.get_field_value(3), Some("12345"));

        let orc = message.get_segment_by_id("ORC").unwrap();
        assert_eq!(orc.get_field_value(1), Some("NW"));

        let rxo = message.get_segment_by_id("RXO").unwrap();
        assert_eq!(rxo.get_field_value(1), Some("00093-0058^Lorazepam^NDC"));

        let rxr = message.get_segment_by_id("RXR").unwrap();
        assert_eq!(rxr.get_field_value(1), Some("PO^Oral^HL70162"));
    }

    #[test]
    fn test_omp_multiple_orders_stay_in_order() {
        let message = minimal_builder()
            .add_order(
                MedicationOrder::new("00093-0058^Lorazepam^NDC", "PO").placer_order_number("RX-1"),
            )
            .add_order(
                MedicationOrder::new("00071-0156^Aspirin^NDC", "PO").placer_order_number("RX-2"),
            )
            .build()
            .unwrap();

        let rxos = message.get_segments_by_id("RXO");
        assert_eq!(rxos.len(), 2);
        assert_eq!(rxos[0].get_field_value(1), Some("00093-0058^Lorazepam^NDC"));
        assert_eq!(rxos[1].get_field_value(1), Some("00071-0156^Aspirin^NDC"));

        let orcs = message.get_segments_by_id("ORC");
        assert_eq!(orcs[0].get_field_value(2), Some("RX-1"));
        assert_eq!(orcs[1].get_field_value(2), Some("RX-2"));
    }

    #[test]
    fn test_omp_optional_order_fields() {
        let message = minimal_builder()
            .add_order(
                MedicationOrder::new("00093-0058^Lorazepam^NDC", "PO^Oral^HL70162")
                    .amount_minimum("1")
                    .amount_maximum("2")
                    .units("TAB^Tablet^HL70164")
                    .dosage_form("TAB^Tablet^HL70164")
                    .refills("3"),
            )
            .build()
            .unwrap();

        let rxo = message.get_segment_by_id("RXO").unwrap();
        assert_eq!(rxo.get_field_value(2), Some("1"));
        assert_eq!(rxo.get_field_value(3), Some("2"));
        assert_eq!(rxo.get_field_value(4), Some("TAB^Tablet^HL70164"));
        assert_eq!(rxo.get_field_value(5), Some("TAB^Tablet^HL70164"));
        assert_eq!(rxo.get_field_value(13), Some("3"));
    }
}
