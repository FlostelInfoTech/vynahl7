//! Message-level FHIR conversion: whole messages to Bundles.
//!
//! While the per-resource converters translate individual segments,
//! [`convert_message_to_bundle`] dispatches on MSH-9 and assembles every
//! resource a message carries into one [`Bundle`]:
//!
//! | Message | Bundle contents |
//! |---------|-----------------|
//! | ADT | Patient, plus Encounter when PV1 is present |
//! | ORU | Observations, plus DiagnosticReports when OBR is present |
//! | VXU | Immunizations |
//! | PPR | Conditions |
//! | OMP | Patient plus ServiceRequests |
//!
//! ```rust
//! use vyna_hl7_fhir::bundle::convert_message_to_bundle;
//! use vyna_hl7_parser::parse_message;
//!
//! let message = parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7").unwrap();
//! let bundle = convert_message_to_bundle(&message).unwrap();
//! assert_eq!(bundle.len(), 1);
//! ```

use serde::Serialize;
use serde_json::Value;
use vyna_hl7_core::Message;

use crate::converters::condition::ConditionConverter;
use crate::converters::diagnostic_report::DiagnosticReportConverter;
use crate::converters::encounter::EncounterConverter;
use crate::converters::immunization::ImmunizationConverter;
use crate::converters::observation::ObservationConverter;
use crate::converters::patient::PatientConverter;
use crate::converters::service_request::ServiceRequestConverter;
use crate::error::{ConversionError, ConversionResult};
use crate::resources::Bundle;

/// Fluent builder for [`Bundle`]s.
pub struct BundleBuilder {
    bundle: Bundle,
}

impl BundleBuilder {
    /// Start a bundle of the given type (e.g. `"collection"`).
    pub fn new(bundle_type: &str) -> Self {
        Self {
            bundle: Bundle::new(bundle_type),
        }
    }

    /// Set the bundle id.
    pub fn id(mut self, id: &str) -> Self {
        self.bundle.id = Some(id.to_string());
        self
    }

    /// Stamp the bundle with the current UTC time.
    pub fn timestamp_now(mut self) -> Self {
        self.bundle.timestamp = Some(chrono::Utc::now().to_rfc3339());
        self
    }

    /// Append a typed resource (any serializable FHIR struct).
    ///
    /// A serialization failure is returned to the caller; nothing is
    /// appended.
    pub fn add_resource<T: Serialize>(mut self, resource: &T) -> ConversionResult<Self> {
        self.bundle.add_resource(resource)?;
        Ok(self)
    }

    /// Append an already-serialized resource (e.g. mapping-engine output).
    pub fn add_json(mut self, resource: Value) -> Self {
        self.bundle.add_json(resource);
        self
    }

    /// Finish the bundle.
    pub fn build(self) -> Bundle {
        self.bundle
    }
}

/// Convert a whole message into a [`Bundle`], dispatching on MSH-9.
///
/// Secondary resources are only attempted when their trigger segment is
/// present (PV1 for Encounter, OBR for DiagnosticReport); a missing primary
/// resource or an unsupported message type is an error.
pub fn convert_message_to_bundle(message: &Message) -> ConversionResult<Bundle> {
    let (msg_type, trigger) = message
        .get_message_type()
        .ok_or_else(|| ConversionError::MissingSegment("MSH-9 message type".to_string()))?;

    let mut builder = BundleBuilder::new("collection").timestamp_now();
    match msg_type.as_str() {
        "ADT" => {
            builder = builder.add_resource(&PatientConverter::convert(message)?)?;
            if message.get_segment_by_id("PV1").is_some() {
                builder = builder.add_resource(&EncounterConverter::convert(message)?)?;
            }
        }
        "ORU" => {
            for observation in ObservationConverter::convert_all(message)? {
                builder = builder.add_resource(&observation)?;
            }
            if message.get_segment_by_id("OBR").is_some() {
                for report in DiagnosticReportConverter::convert_all(message)? {
                    builder = builder.add_resource(&report)?;
                }
            }
        }
        "VXU" => {
            for immunization in ImmunizationConverter::convert_all(message)? {
                builder = builder.add_resource(&immunization)?;
            }
        }
        "PPR" => {
            for condition in ConditionConverter::convert_all(message)? {
                builder = builder.add_resource(&condition)?;
            }
        }
        "OMP" => {
            builder = builder.add_resource(&PatientConverter::convert(message)?)?;
            for request in ServiceRequestConverter::convert_all(message)? {
                builder = builder.add_resource(&request)?;
            }
        }
        _ => {
            return Err(ConversionError::UnsupportedMessageType(
                msg_type.clone(),
                trigger.clone(),
            ));
        }
    }
    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_parser::parse_message;

    const ADT: &str = "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\rPV1|1|O\r";
    const ORU: &str = "MSH|^~\\&|A|B|C|D|20240101||ORU^R01|1|P|2.5\rPID|1||7\rOBR|1|||CBC^Blood Count\rOBX|1|NM|WBC||8.5||||||F\r";
    const VXU: &str = "MSH|^~\\&|A|B|C|D|20240101||VXU^V04|1|P|2.5\rPID|1||7\rORC|RE\rRXA|0|1|20240101|20240101|08^HepB^CVX\r";
    const PPR: &str = "MSH|^~\\&|A|B|C|D|20240101||PPR^PC1|1|P|2.5\rPID|1||7\rPRB|AD|20240101|38341003^HTN^SCT|P1\r";
    const OMP: &str = "MSH|^~\\&|A|B|C|D|20240101||OMP^O09|1|P|2.5\rPID|1||7\rORC|NW|RX-1\rRXO|00093-0058^Loraz^NDC\rRXR|PO\r";

    fn kinds(bundle: &Bundle) -> Vec<&str> {
        bundle
            .entry
            .as_ref()
            .unwrap()
            .iter()
            .map(|e| e.resource.get("resourceType").unwrap().as_str().unwrap())
            .collect()
    }

    #[test]
    fn adt_bundle_holds_patient_and_encounter() {
        let message = parse_message(ADT).unwrap();
        let bundle = convert_message_to_bundle(&message).unwrap();
        assert_eq!(kinds(&bundle), vec!["Patient", "Encounter"]);
        assert!(bundle.timestamp.is_some());
        assert_eq!(
            bundle.entry.as_ref().unwrap()[0].resource["identifier"][0]["value"],
            "7"
        );
    }

    #[test]
    fn adt_without_pv1_holds_patient_only() {
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\r").unwrap();
        let bundle = convert_message_to_bundle(&message).unwrap();
        assert_eq!(kinds(&bundle), vec!["Patient"]);
    }

    #[test]
    fn oru_bundle_holds_observation_and_report() {
        let message = parse_message(ORU).unwrap();
        let bundle = convert_message_to_bundle(&message).unwrap();
        assert_eq!(kinds(&bundle), vec!["Observation", "DiagnosticReport"]);
    }

    #[test]
    fn vxu_bundle_holds_immunizations() {
        let message = parse_message(VXU).unwrap();
        let bundle = convert_message_to_bundle(&message).unwrap();
        assert_eq!(kinds(&bundle), vec!["Immunization"]);
    }

    #[test]
    fn ppr_bundle_holds_conditions() {
        let message = parse_message(PPR).unwrap();
        let bundle = convert_message_to_bundle(&message).unwrap();
        assert_eq!(kinds(&bundle), vec!["Condition"]);
    }

    #[test]
    fn omp_bundle_holds_patient_and_service_request() {
        let message = parse_message(OMP).unwrap();
        let bundle = convert_message_to_bundle(&message).unwrap();
        assert_eq!(kinds(&bundle), vec!["Patient", "ServiceRequest"]);
    }

    #[test]
    fn unsupported_message_type_is_an_error() {
        let message =
            parse_message("MSH|^~\\&|A|B|C|D|20240101||SIU^S12|1|P|2.5\rPID|1||7\r").unwrap();
        let result = convert_message_to_bundle(&message);
        assert!(matches!(
            result,
            Err(ConversionError::UnsupportedMessageType(_, _))
        ));
    }

    #[test]
    fn builder_propagates_resource_serialization_failure() {
        use serde::ser::{Error as SerError, Serializer};

        struct Failing;

        impl Serialize for Failing {
            fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(S::Error::custom("boom"))
            }
        }

        let result = BundleBuilder::new("collection").add_resource(&Failing);
        assert!(result.is_err());
    }

    #[test]
    fn builder_mixes_typed_and_json_resources() {
        let bundle = BundleBuilder::new("collection")
            .id("b1")
            .timestamp_now()
            .add_json(serde_json::json!({"resourceType": "Patient", "id": "7"}))
            .build();
        assert_eq!(bundle.id.as_deref(), Some("b1"));
        assert!(bundle.timestamp.is_some());
        assert_eq!(bundle.len(), 1);
    }
}
