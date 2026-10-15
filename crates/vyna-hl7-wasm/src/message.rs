//! WebAssembly message wrapper

use serde::Serialize;
use vyna_hl7_core::Message;
use vyna_hl7_terser::TerserMut;
use wasm_bindgen::prelude::*;

/// WebAssembly-friendly wrapper for HL7 Message
#[wasm_bindgen]
pub struct WasmMessage {
    inner: Message,
}

impl WasmMessage {
    /// Get a reference to the inner message
    pub(crate) fn inner(&self) -> &Message {
        &self.inner
    }

    /// Get a mutable reference to the inner message
    #[allow(dead_code)]
    pub(crate) fn inner_mut(&mut self) -> &mut Message {
        &mut self.inner
    }
}

impl From<Message> for WasmMessage {
    fn from(message: Message) -> Self {
        Self { inner: message }
    }
}

/// Serializable JSON form of a message (built by [`WasmMessage::json_data`]).
#[derive(Serialize)]
struct JsonMessage {
    version: Option<String>,
    message_type: Option<String>,
    sending_application: Option<String>,
    sending_facility: Option<String>,
    receiving_application: Option<String>,
    receiving_facility: Option<String>,
    control_id: Option<String>,
    segments: Vec<JsonSegment>,
}

/// Serializable JSON form of one segment.
#[derive(Serialize)]
struct JsonSegment {
    id: String,
    fields: Vec<String>,
}

#[wasm_bindgen]
impl WasmMessage {
    /// Get the HL7 version as a string
    pub fn version(&self) -> Option<String> {
        // get_version returns Option<Version>
        self.inner.get_version().map(|v| v.as_str().to_string())
    }

    /// Get the message type (e.g., "ADT^A01")
    #[wasm_bindgen(js_name = messageType)]
    pub fn message_type(&self) -> Option<String> {
        // get_message_type returns Option<(String, String)> - combine them
        self.inner
            .get_message_type()
            .map(|(msg, event)| format!("{}^{}", msg, event))
    }

    /// Get the sending application
    #[wasm_bindgen(js_name = sendingApplication)]
    pub fn sending_application(&self) -> Option<String> {
        self.inner.get_sending_application().map(|s| s.to_string())
    }

    /// Get the sending facility
    #[wasm_bindgen(js_name = sendingFacility)]
    pub fn sending_facility(&self) -> Option<String> {
        self.inner.get_sending_facility().map(|s| s.to_string())
    }

    /// Get the receiving application
    #[wasm_bindgen(js_name = receivingApplication)]
    pub fn receiving_application(&self) -> Option<String> {
        self.inner
            .get_receiving_application()
            .map(|s| s.to_string())
    }

    /// Get the receiving facility
    #[wasm_bindgen(js_name = receivingFacility)]
    pub fn receiving_facility(&self) -> Option<String> {
        self.inner.get_receiving_facility().map(|s| s.to_string())
    }

    /// Get the message control ID
    #[wasm_bindgen(js_name = controlId)]
    pub fn control_id(&self) -> Option<String> {
        // get_control_id returns Option<&str>
        self.inner.get_control_id().map(|s| s.to_string())
    }

    /// Encode the message back to HL7 string
    pub fn encode(&self) -> String {
        self.inner.encode()
    }

    /// Get the number of segments
    #[wasm_bindgen(js_name = segmentCount)]
    pub fn segment_count(&self) -> usize {
        self.inner.segments.len()
    }

    /// Get segment IDs as an array
    #[wasm_bindgen(js_name = segmentIds)]
    pub fn segment_ids(&self) -> Vec<String> {
        self.inner.segments.iter().map(|s| s.id.clone()).collect()
    }

    /// Set a field value using Terser path
    pub(crate) fn set_value(&mut self, path: &str, value: &str) -> Result<(), String> {
        let mut terser = TerserMut::new(&mut self.inner);
        terser.set(path, value).map_err(|e| e.to_string())
    }

    /// Build the serializable JSON structure (pure helper for testing).
    fn json_data(&self) -> JsonMessage {
        JsonMessage {
            version: self.version().map(|v| v.to_string()),
            message_type: self.message_type(),
            sending_application: self.sending_application(),
            sending_facility: self.sending_facility(),
            receiving_application: self.receiving_application(),
            receiving_facility: self.receiving_facility(),
            control_id: self.control_id(),
            segments: self
                .inner
                .segments
                .iter()
                .map(|seg| JsonSegment {
                    id: seg.id.clone(),
                    fields: seg
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            // MSH-1/MSH-2 hold raw delimiter chars; encode()
                            // would escape them ("|" becomes "\\F\\").
                            if seg.id == "MSH" && i < 2 {
                                f.value().unwrap_or("").to_string()
                            } else {
                                f.encode(&self.inner.delimiters)
                            }
                        })
                        .collect(),
                })
                .collect(),
        }
    }

    /// Convert to JSON
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json_msg = self.json_data();
        serde_wasm_bindgen::to_value(&json_msg).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use crate::parse_message as parse_wasm;

    const ADT: &str = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
         PID|1||MRN123||DOE^JOHN||19800101|M";

    #[test]
    fn set_value_rejects_malformed_path() {
        // Error path stays in pure Rust (Result<(), String>), so it is
        // testable on native targets where JsValue cannot be built.
        let mut msg = parse_wasm(ADT).unwrap();
        let err = msg.set_value("!!!bad-path", "x").unwrap_err();
        assert!(!err.is_empty(), "error must say what failed");
    }

    #[test]
    fn set_value_round_trip() {
        let mut msg = parse_wasm(ADT).unwrap();
        msg.set_value("PID-5-1", "SMITH").unwrap();
        let terser = vyna_hl7_terser::Terser::new(msg.inner());
        assert_eq!(terser.get("PID-5-1").unwrap(), Some("SMITH"));
    }

    #[test]
    fn version_returns_dotted_string() {
        // d.ts, README and getVersion rustdoc promise "2.5",
        // not the Debug form "V2_5".
        let msg = parse_wasm(ADT).unwrap();
        assert_eq!(msg.version(), Some("2.5".to_string()));
    }

    #[test]
    fn json_keeps_msh_delimiters_raw() {
        // toJson must match parse display: MSH-1/MSH-2 stay raw
        // ("|", "^~\\&"), not HL7-escaped ("\\F\\", ...).
        let msg = parse_wasm(ADT).unwrap();
        let data = msg.json_data();
        assert_eq!(data.segments[0].id, "MSH");
        assert_eq!(data.segments[0].fields[0], "|");
        assert_eq!(data.segments[0].fields[1], "^~\\&");
    }

    #[test]
    fn json_carries_receiving_app_and_facility() {
        let msg = parse_wasm(ADT).unwrap();
        let data = msg.json_data();
        assert_eq!(data.sending_application.as_deref(), Some("SendApp"));
        assert_eq!(data.sending_facility.as_deref(), Some("SendFac"));
        assert_eq!(data.receiving_application.as_deref(), Some("RecApp"));
        assert_eq!(data.receiving_facility.as_deref(), Some("RecFac"));
    }
}
