//! FHIR Bundle resource
//!
//! Based on FHIR R4 Bundle: <https://www.hl7.org/fhir/R4/bundle.html>
//!
//! A Bundle groups the resources converted from one HL7 v2 message so they
//! can be exchanged or stored as a single document. Entries hold generic
//! JSON so typed converter output and declarative-mapping output mix freely.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::ConversionResult;

/// FHIR Bundle resource - a container for a collection of resources
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    /// Resource type (always "Bundle")
    pub resource_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Bundle type: document | message | transaction | batch | history |
    /// searchset | collection
    #[serde(rename = "type")]
    pub bundle_type: String,

    /// When the bundle was assembled (FHIR instant)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,

    /// Total number of entries, if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,

    /// The resources in the bundle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Vec<BundleEntry>>,
}

/// One entry in a Bundle: a resource plus its identity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BundleEntry {
    /// Absolute URI of the resource (e.g. `Patient/12345`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_url: Option<String>,

    /// The resource itself
    pub resource: Value,
}

impl Bundle {
    /// Create a new Bundle of the given type (e.g. `"collection"`)
    pub fn new(bundle_type: &str) -> Self {
        Self {
            resource_type: "Bundle".to_string(),
            id: None,
            bundle_type: bundle_type.to_string(),
            timestamp: None,
            total: None,
            entry: None,
        }
    }

    /// Append a resource, deriving `fullUrl` from its `resourceType`/`id`
    /// when an id is present.
    ///
    /// A serialization failure is returned to the caller; nothing is
    /// appended and `total` is left unchanged.
    pub fn add_resource<T: Serialize>(&mut self, resource: &T) -> ConversionResult<()> {
        let value = serde_json::to_value(resource)?;
        self.add_json(value);
        Ok(())
    }

    /// Append an already-serialized resource value.
    pub fn add_json(&mut self, resource: Value) {
        let full_url = match (&resource.get("resourceType"), &resource.get("id")) {
            (Some(Value::String(kind)), Some(Value::String(id))) => Some(format!("{kind}/{id}")),
            _ => None,
        };
        let entries = self.entry.get_or_insert_with(Vec::new);
        entries.push(BundleEntry { full_url, resource });
        self.total = Some(entries.len() as u32);
    }

    /// Number of entries in the bundle
    pub fn len(&self) -> usize {
        self.entry.as_ref().map(Vec::len).unwrap_or(0)
    }

    /// Whether the bundle holds no entries
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for Bundle {
    fn default() -> Self {
        Self::new("collection")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::patient::Patient;

    #[test]
    fn bundle_entry_derives_full_url_from_id() {
        let mut bundle = Bundle::new("collection");
        let mut patient = Patient::new();
        patient.id = Some("12345".to_string());
        bundle.add_resource(&patient).unwrap();

        assert_eq!(bundle.len(), 1);
        assert_eq!(bundle.total, Some(1));
        let entry = &bundle.entry.as_ref().unwrap()[0];
        assert_eq!(entry.full_url.as_deref(), Some("Patient/12345"));
        assert_eq!(entry.resource.get("resourceType").unwrap(), "Patient");
    }

    #[test]
    fn bundle_entry_without_id_has_no_full_url() {
        let mut bundle = Bundle::new("searchset");
        bundle.add_resource(&Patient::new()).unwrap();

        let entry = &bundle.entry.as_ref().unwrap()[0];
        assert!(entry.full_url.is_none());
    }

    #[test]
    fn add_resource_propagates_serialization_failure() {
        use serde::ser::{Error as SerError, Serializer};

        struct Failing;

        impl Serialize for Failing {
            fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(S::Error::custom("boom"))
            }
        }

        let mut bundle = Bundle::new("collection");
        let result = bundle.add_resource(&Failing);
        assert!(result.is_err());
        assert!(bundle.is_empty());
        assert_eq!(bundle.total, None);
    }

    #[test]
    fn bundle_serializes_fhir_shape() {
        let mut bundle = Bundle::new("collection");
        bundle.add_json(serde_json::json!({"resourceType": "Patient", "id": "1"}));
        let json = serde_json::to_value(&bundle).unwrap();
        assert_eq!(json["resourceType"], "Bundle");
        assert_eq!(json["type"], "collection");
        assert_eq!(json["total"], 1);
        assert_eq!(json["entry"][0]["fullUrl"], "Patient/1");
    }
}
