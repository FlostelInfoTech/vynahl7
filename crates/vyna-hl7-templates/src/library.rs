//! Standard template library with common HL7 message templates.

use crate::{FieldTemplate, MessageTemplate, SegmentTemplate};
use std::collections::HashMap;

/// Standard template library
pub struct TemplateLibrary {
    templates: HashMap<String, MessageTemplate>,
}

impl TemplateLibrary {
    /// Create a new template library with standard templates
    pub fn new() -> Self {
        let mut lib = Self {
            templates: HashMap::new(),
        };

        // Add standard templates
        lib.add_template(Self::adt_a01());
        lib.add_template(Self::adt_a04());
        lib.add_template(Self::adt_a08());
        lib.add_template(Self::oru_r01());
        lib.add_template(Self::orm_o01());
        lib.add_template(Self::siu_s12());
        lib.add_template(Self::mdm_t02());

        lib
    }

    /// Add a template to the library
    pub fn add_template(&mut self, template: MessageTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<&MessageTemplate> {
        self.templates.get(name)
    }

    /// List all template names
    pub fn list_templates(&self) -> Vec<&String> {
        self.templates.keys().collect()
    }

    // ===== ADT Templates =====

    /// ADT^A01 - Admit/Visit Notification
    pub fn adt_a01() -> MessageTemplate {
        let mut template = MessageTemplate::new("ADT_A01", "2.5", "ADT", "A01")
            .with_description("Admit/Visit Notification");

        // MSH segment
        let mut msh = SegmentTemplate::new("MSH").required();
        msh.add_field(3, FieldTemplate::new().required().with_placeholder("{{sending_app}}"));
        msh.add_field(4, FieldTemplate::new().required().with_placeholder("{{sending_facility}}"));
        msh.add_field(5, FieldTemplate::new().with_placeholder("{{receiving_app}}"));
        msh.add_field(6, FieldTemplate::new().with_placeholder("{{receiving_facility}}"));
        template.add_segment(msh);

        // EVN segment
        let mut evn = SegmentTemplate::new("EVN").required();
        evn.add_field(1, FieldTemplate::new().required().with_default("A01"));
        evn.add_field(2, FieldTemplate::new().required().with_placeholder("{{event_datetime}}"));
        template.add_segment(evn);

        // PID segment
        let mut pid = SegmentTemplate::new("PID").required();
        pid.add_field(3, FieldTemplate::new().required().with_placeholder("{{patient_id}}"));
        pid.add_field(5, FieldTemplate::new().required().with_placeholder("{{patient_name}}"));
        pid.add_field(7, FieldTemplate::new().with_placeholder("{{date_of_birth}}"));
        pid.add_field(8, FieldTemplate::new().with_placeholder("{{sex}}"));
        pid.add_field(11, FieldTemplate::new().with_placeholder("{{address}}"));
        template.add_segment(pid);

        // PV1 segment
        let mut pv1 = SegmentTemplate::new("PV1").required();
        pv1.add_field(2, FieldTemplate::new().required().with_placeholder("{{patient_class}}"));
        pv1.add_field(3, FieldTemplate::new().with_placeholder("{{assigned_location}}"));
        pv1.add_field(7, FieldTemplate::new().with_placeholder("{{attending_doctor}}"));
        pv1.add_field(19, FieldTemplate::new().with_placeholder("{{visit_number}}"));
        template.add_segment(pv1);

        template
    }

    /// ADT^A04 - Register a Patient
    pub fn adt_a04() -> MessageTemplate {
        let mut template = MessageTemplate::new("ADT_A04", "2.5", "ADT", "A04")
            .with_description("Register a Patient");

        // Similar structure to A01 but for registration
        let mut msh = SegmentTemplate::new("MSH").required();
        msh.add_field(3, FieldTemplate::new().required().with_placeholder("{{sending_app}}"));
        msh.add_field(4, FieldTemplate::new().required().with_placeholder("{{sending_facility}}"));
        template.add_segment(msh);

        let mut evn = SegmentTemplate::new("EVN").required();
        evn.add_field(1, FieldTemplate::new().required().with_default("A04"));
        evn.add_field(2, FieldTemplate::new().required().with_placeholder("{{event_datetime}}"));
        template.add_segment(evn);

        let mut pid = SegmentTemplate::new("PID").required();
        pid.add_field(3, FieldTemplate::new().required().with_placeholder("{{patient_id}}"));
        pid.add_field(5, FieldTemplate::new().required().with_placeholder("{{patient_name}}"));
        pid.add_field(7, FieldTemplate::new().with_placeholder("{{date_of_birth}}"));
        pid.add_field(8, FieldTemplate::new().with_placeholder("{{sex}}"));
        template.add_segment(pid);

        template
    }

    /// ADT^A08 - Update Patient Information
    pub fn adt_a08() -> MessageTemplate {
        let mut template = MessageTemplate::new("ADT_A08", "2.5", "ADT", "A08")
            .with_description("Update Patient Information");

        let mut msh = SegmentTemplate::new("MSH").required();
        msh.add_field(3, FieldTemplate::new().required().with_placeholder("{{sending_app}}"));
        msh.add_field(4, FieldTemplate::new().required().with_placeholder("{{sending_facility}}"));
        template.add_segment(msh);

        let mut evn = SegmentTemplate::new("EVN").required();
        evn.add_field(1, FieldTemplate::new().required().with_default("A08"));
        evn.add_field(2, FieldTemplate::new().required().with_placeholder("{{event_datetime}}"));
        template.add_segment(evn);

        let mut pid = SegmentTemplate::new("PID").required();
        pid.add_field(3, FieldTemplate::new().required().with_placeholder("{{patient_id}}"));
        pid.add_field(5, FieldTemplate::new().with_placeholder("{{patient_name}}"));
        pid.add_field(7, FieldTemplate::new().with_placeholder("{{date_of_birth}}"));
        pid.add_field(8, FieldTemplate::new().with_placeholder("{{sex}}"));
        pid.add_field(11, FieldTemplate::new().with_placeholder("{{address}}"));
        template.add_segment(pid);

        template
    }

    // ===== ORU Templates =====

    /// ORU^R01 - Observation Result
    pub fn oru_r01() -> MessageTemplate {
        let mut template = MessageTemplate::new("ORU_R01", "2.5", "ORU", "R01")
            .with_description("Unsolicited Observation Result");

        let mut msh = SegmentTemplate::new("MSH").required();
        msh.add_field(3, FieldTemplate::new().required().with_placeholder("{{sending_app}}"));
        msh.add_field(4, FieldTemplate::new().required().with_placeholder("{{sending_facility}}"));
        template.add_segment(msh);

        let mut pid = SegmentTemplate::new("PID").required();
        pid.add_field(3, FieldTemplate::new().required().with_placeholder("{{patient_id}}"));
        pid.add_field(5, FieldTemplate::new().required().with_placeholder("{{patient_name}}"));
        template.add_segment(pid);

        let mut obr = SegmentTemplate::new("OBR").required();
        obr.add_field(1, FieldTemplate::new().required().with_placeholder("{{set_id}}"));
        obr.add_field(4, FieldTemplate::new().required().with_placeholder("{{universal_service_id}}"));
        obr.add_field(7, FieldTemplate::new().with_placeholder("{{observation_datetime}}"));
        template.add_segment(obr);

        let mut obx = SegmentTemplate::new("OBX").repeating();
        obx.add_field(1, FieldTemplate::new().required().with_placeholder("{{set_id}}"));
        obx.add_field(2, FieldTemplate::new().required().with_placeholder("{{value_type}}"));
        obx.add_field(3, FieldTemplate::new().required().with_placeholder("{{observation_id}}"));
        obx.add_field(5, FieldTemplate::new().required().with_placeholder("{{observation_value}}"));
        obx.add_field(6, FieldTemplate::new().with_placeholder("{{units}}"));
        obx.add_field(11, FieldTemplate::new().required().with_default("F")); // Final
        template.add_segment(obx);

        template
    }

    // ===== ORM Templates =====

    /// ORM^O01 - General Order Message
    pub fn orm_o01() -> MessageTemplate {
        let mut template = MessageTemplate::new("ORM_O01", "2.5", "ORM", "O01")
            .with_description("General Order Message");

        let mut msh = SegmentTemplate::new("MSH").required();
        msh.add_field(3, FieldTemplate::new().required().with_placeholder("{{sending_app}}"));
        msh.add_field(4, FieldTemplate::new().required().with_placeholder("{{sending_facility}}"));
        template.add_segment(msh);

        let mut pid = SegmentTemplate::new("PID").required();
        pid.add_field(3, FieldTemplate::new().required().with_placeholder("{{patient_id}}"));
        pid.add_field(5, FieldTemplate::new().required().with_placeholder("{{patient_name}}"));
        template.add_segment(pid);

        let mut orc = SegmentTemplate::new("ORC").required();
        orc.add_field(1, FieldTemplate::new().required().with_placeholder("{{order_control}}"));
        orc.add_field(2, FieldTemplate::new().with_placeholder("{{placer_order_number}}"));
        orc.add_field(5, FieldTemplate::new().with_placeholder("{{order_status}}"));
        template.add_segment(orc);

        let mut obr = SegmentTemplate::new("OBR").required();
        obr.add_field(1, FieldTemplate::new().required().with_placeholder("{{set_id}}"));
        obr.add_field(4, FieldTemplate::new().required().with_placeholder("{{universal_service_id}}"));
        template.add_segment(obr);

        template
    }

    // ===== SIU Templates =====

    /// SIU^S12 - Notification of New Appointment
    pub fn siu_s12() -> MessageTemplate {
        let mut template = MessageTemplate::new("SIU_S12", "2.5", "SIU", "S12")
            .with_description("Notification of New Appointment Booking");

        let mut msh = SegmentTemplate::new("MSH").required();
        msh.add_field(3, FieldTemplate::new().required().with_placeholder("{{sending_app}}"));
        msh.add_field(4, FieldTemplate::new().required().with_placeholder("{{sending_facility}}"));
        template.add_segment(msh);

        let mut sch = SegmentTemplate::new("SCH").required();
        sch.add_field(1, FieldTemplate::new().with_placeholder("{{placer_appointment_id}}"));
        sch.add_field(2, FieldTemplate::new().with_placeholder("{{filler_appointment_id}}"));
        sch.add_field(11, FieldTemplate::new().required().with_placeholder("{{appointment_timing}}"));
        template.add_segment(sch);

        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(3, FieldTemplate::new().with_placeholder("{{patient_id}}"));
        pid.add_field(5, FieldTemplate::new().with_placeholder("{{patient_name}}"));
        template.add_segment(pid);

        template
    }

    // ===== MDM Templates =====

    /// MDM^T02 - Document Status Change
    pub fn mdm_t02() -> MessageTemplate {
        let mut template = MessageTemplate::new("MDM_T02", "2.5", "MDM", "T02")
            .with_description("Original Document Notification");

        let mut msh = SegmentTemplate::new("MSH").required();
        msh.add_field(3, FieldTemplate::new().required().with_placeholder("{{sending_app}}"));
        msh.add_field(4, FieldTemplate::new().required().with_placeholder("{{sending_facility}}"));
        template.add_segment(msh);

        let mut evn = SegmentTemplate::new("EVN").required();
        evn.add_field(1, FieldTemplate::new().required().with_default("T02"));
        evn.add_field(2, FieldTemplate::new().required().with_placeholder("{{event_datetime}}"));
        template.add_segment(evn);

        let mut pid = SegmentTemplate::new("PID").required();
        pid.add_field(3, FieldTemplate::new().required().with_placeholder("{{patient_id}}"));
        pid.add_field(5, FieldTemplate::new().required().with_placeholder("{{patient_name}}"));
        template.add_segment(pid);

        let mut txa = SegmentTemplate::new("TXA").required();
        txa.add_field(1, FieldTemplate::new().required().with_placeholder("{{set_id}}"));
        txa.add_field(2, FieldTemplate::new().required().with_placeholder("{{document_type}}"));
        txa.add_field(12, FieldTemplate::new().required().with_placeholder("{{unique_document_number}}"));
        template.add_segment(txa);

        template
    }
}

impl Default for TemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_creation() {
        let library = TemplateLibrary::new();
        assert!(library.get("ADT_A01").is_some());
        assert!(library.get("ORU_R01").is_some());
        assert!(library.get("NonExistent").is_none());
    }

    #[test]
    fn test_list_templates() {
        let library = TemplateLibrary::new();
        let templates = library.list_templates();
        assert!(templates.len() >= 7);
        assert!(templates.contains(&&"ADT_A01".to_string()));
    }

    #[test]
    fn test_adt_a01_template() {
        let template = TemplateLibrary::adt_a01();
        assert_eq!(template.name, "ADT_A01");
        assert_eq!(template.message_type, "ADT");
        assert_eq!(template.trigger_event, "A01");
        assert_eq!(template.version, "2.5");

        // Should have MSH, EVN, PID, PV1
        assert_eq!(template.segments.len(), 4);
        assert_eq!(template.segments[0].id, "MSH");
        assert_eq!(template.segments[1].id, "EVN");
        assert_eq!(template.segments[2].id, "PID");
        assert_eq!(template.segments[3].id, "PV1");
    }

    #[test]
    fn test_oru_r01_template() {
        let template = TemplateLibrary::oru_r01();
        assert_eq!(template.name, "ORU_R01");
        assert_eq!(template.message_type, "ORU");
        assert_eq!(template.trigger_event, "R01");

        // Should have MSH, PID, OBR, OBX
        assert!(template.segments.iter().any(|s| s.id == "MSH"));
        assert!(template.segments.iter().any(|s| s.id == "PID"));
        assert!(template.segments.iter().any(|s| s.id == "OBR"));
        assert!(template.segments.iter().any(|s| s.id == "OBX"));

        // OBX should be repeating
        let obx = template.segments.iter().find(|s| s.id == "OBX").unwrap();
        assert!(obx.repeating);
    }

    #[test]
    fn test_template_has_placeholders() {
        let template = TemplateLibrary::adt_a01();
        let pid = template.segments.iter().find(|s| s.id == "PID").unwrap();

        assert!(pid.fields.is_some());
        let fields = pid.fields.as_ref().unwrap();

        // PID-3 should have patient_id placeholder
        assert!(fields.contains_key(&3));
        let field3 = &fields[&3];
        assert_eq!(field3.placeholder, Some("{{patient_id}}".to_string()));
    }

    #[test]
    fn test_add_custom_template() {
        let mut library = TemplateLibrary::new();
        let custom = MessageTemplate::new("CUSTOM", "2.5", "ADT", "A99");

        library.add_template(custom);
        assert!(library.get("CUSTOM").is_some());
    }
}
