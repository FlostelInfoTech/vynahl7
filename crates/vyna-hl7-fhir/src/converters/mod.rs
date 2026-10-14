//! Converters for transforming HL7 v2.x segments to FHIR resources
//!
//! This module provides bidirectional conversion between HL7 v2.x and FHIR R4:
//!
//! - Forward converters: HL7 v2.x -> FHIR R4
//! - Reverse converters: FHIR R4 -> HL7 v2.x

pub mod allergy_intolerance;
pub mod coding_systems;
pub mod condition;
pub mod diagnostic_report;
pub mod encounter;
pub mod immunization;
pub mod medication;
pub mod observation;
pub mod patient;
pub mod practitioner;
pub mod procedure;
pub mod service_request;
pub mod specimen;

/// Reverse converters for FHIR R4 -> HL7 v2.x conversion
pub mod reverse;

// Forward converters (HL7 v2.x -> FHIR R4)
pub use allergy_intolerance::AllergyIntoleranceConverter;
pub use condition::ConditionConverter;
pub use diagnostic_report::DiagnosticReportConverter;
pub use encounter::EncounterConverter;
pub use immunization::ImmunizationConverter;
pub use medication::MedicationConverter;
pub use observation::ObservationConverter;
pub use patient::PatientConverter;
pub use practitioner::PractitionerConverter;
pub use procedure::ProcedureConverter;
pub use service_request::ServiceRequestConverter;
pub use specimen::SpecimenConverter;

// Reverse converters (FHIR R4 -> HL7 v2.x)
pub use reverse::EncounterReverseConverter;
pub use reverse::ObservationReverseConverter;
pub use reverse::PatientReverseConverter;
pub use reverse::PractitionerReverseConverter;

use vyna_hl7_core::Message;

/// Stable Observation id for the Nth OBX segment (`obx_ordinal` is the
/// 0-based ordinal among OBX segments in message order).
///
/// `obs-{OBX-1}` while every set id is unique in the message; when any
/// set id repeats, every valued id carries its 1-based global ordinal
/// (`obs-{set-id}-{ordinal}`) so a literal set id like `1-2` cannot
/// collide with a disambiguated one. `None` when the OBX has no valued
/// set id. Both the Observation converter and the DiagnosticReport
/// linking use this so references resolve.
pub(crate) fn observation_id(message: &Message, obx_ordinal: usize) -> Option<String> {
    let set_ids: Vec<Option<&str>> = message
        .segments
        .iter()
        .filter(|s| s.id == "OBX")
        .map(|s| s.get_field_value(1).filter(|v| !v.is_empty()))
        .collect();
    let set_id = set_ids.get(obx_ordinal).copied().flatten()?;
    let any_repeat = set_ids.iter().filter_map(|set| *set).any(|candidate| {
        set_ids
            .iter()
            .filter(|set| **set == Some(candidate))
            .count()
            > 1
    });
    if any_repeat {
        Some(format!("obs-{set_id}-{}", obx_ordinal + 1))
    } else {
        Some(format!("obs-{set_id}"))
    }
}
