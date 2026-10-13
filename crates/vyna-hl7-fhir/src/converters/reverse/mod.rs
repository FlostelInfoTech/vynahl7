//! Reverse converters for transforming FHIR resources to HL7 v2.x segments
//!
//! These converters complement the forward converters by enabling bidirectional
//! conversion between FHIR R4 resources and HL7 v2.x messages.

pub mod encounter;
pub mod observation;
pub mod patient;
pub mod practitioner;

pub use encounter::EncounterReverseConverter;
pub use observation::ObservationReverseConverter;
pub use patient::PatientReverseConverter;
pub use practitioner::PractitionerReverseConverter;

/// Split a trailing numeric zone (`[+-]HH:MM` or `[+-]HHMM`) off a
/// datetime, returning the stem and the zone as `±HHMM`.
///
/// The sign search stays past the `T` time separator so date dashes
/// never match; anything that is not a four-digit zone (or no zone at
/// all) yields an empty zone and the whole input as the stem.
pub(crate) fn split_trailing_zone(datetime: &str) -> (&str, String) {
    let Some(t) = datetime.find('T') else {
        return (datetime, String::new());
    };
    let Some(offset) = datetime[t..].rfind(['+', '-']) else {
        return (datetime, String::new());
    };
    let (stem, zone) = datetime.split_at(t + offset);
    let digits: String = zone.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() == 4 {
        (stem, format!("{}{}", &zone[..1], digits))
    } else {
        (datetime, String::new())
    }
}
