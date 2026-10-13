//! WebAssembly bindings for VynaHL7 HL7 v2.x library
//!
//! This crate provides JavaScript-friendly bindings for the VynaHL7 library,
//! allowing HL7 message parsing and manipulation in browser and Node.js environments.
//!
//! # Usage
//!
//! ```javascript
//! import init, { parseMessage, getTerserValue } from './vyna_hl7_wasm';
//!
//! await init();
//!
//! const hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\n" +
//!             "PID|1||MRN123||DOE^JOHN||19800101|M";
//!
//! const message = parseMessage(hl7);
//! const patientName = getTerserValue(message, "PID-5-1");
//! console.log(patientName); // "DOE"
//! ```

use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message as vyna_hl7_parse;
use vyna_hl7_terser::Terser;
use vyna_hl7_validator::Validator;
use wasm_bindgen::prelude::*;

mod definitions;
mod message;
mod terser;
mod utils;
mod validation;

pub use definitions::*;
pub use message::*;
pub use terser::*;
pub use validation::*;

/// Initialize panic hook for better error messages in wasm
#[wasm_bindgen(start)]
pub fn init() {
    utils::set_panic_hook();
}

/// Parse an HL7 message from a string
///
/// # Arguments
///
/// * `input` - The HL7 message string
///
/// # Returns
///
/// A JsValue containing the parsed message, or an error
///
/// # Example
///
/// ```javascript
/// const message = parseMessage("MSH|^~\\&|...");
/// ```
#[wasm_bindgen(js_name = parseMessage)]
pub fn parse_message(input: &str) -> Result<WasmMessage, JsValue> {
    vyna_hl7_parse(input)
        .map(WasmMessage::from)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the HL7 version from a message
///
/// # Arguments
///
/// * `message` - The parsed message
///
/// # Returns
///
/// A string representing the HL7 version (e.g., "2.5")
#[wasm_bindgen(js_name = getVersion)]
pub fn get_version(message: &WasmMessage) -> Option<String> {
    message.version()
}

/// Get the message type from a message
///
/// # Arguments
///
/// * `message` - The parsed message
///
/// # Returns
///
/// A string representing the message type (e.g., "ADT^A01")
#[wasm_bindgen(js_name = getMessageType)]
pub fn get_message_type(message: &WasmMessage) -> Option<String> {
    message.message_type()
}

/// Encode a message back to HL7 string format
///
/// # Arguments
///
/// * `message` - The message to encode
///
/// # Returns
///
/// The encoded HL7 string
#[wasm_bindgen(js_name = encodeMessage)]
pub fn encode_message(message: &WasmMessage) -> String {
    message.encode()
}

/// Get a field value using Terser path notation
///
/// # Arguments
///
/// * `message` - The parsed message
/// * `path` - The Terser path (e.g., "PID-5-1")
///
/// # Returns
///
/// The field value if found, or null
///
/// # Example
///
/// ```javascript
/// const name = getTerserValue(message, "PID-5");
/// const family = getTerserValue(message, "PID-5-1");
/// ```
#[wasm_bindgen(js_name = getTerserValue)]
pub fn get_terser_value(message: &WasmMessage, path: &str) -> Result<Option<String>, JsValue> {
    let terser = Terser::new(message.inner());
    terser
        .get(path)
        .map(|opt| opt.map(|s| s.to_string()))
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Set a field value using Terser path notation
///
/// # Arguments
///
/// * `message` - The parsed message (will be modified)
/// * `path` - The Terser path (e.g., "PID-5-1")
/// * `value` - The value to set
///
/// # Example
///
/// ```javascript
/// setTerserValue(message, "PID-5-1", "SMITH");
/// ```
#[wasm_bindgen(js_name = setTerserValue)]
pub fn set_terser_value(message: &mut WasmMessage, path: &str, value: &str) -> Result<(), JsValue> {
    message
        .set_value(path, value)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Validate a message against HL7 standards
///
/// # Arguments
///
/// * `message` - The message to validate
///
/// # Returns
///
/// A validation result object
#[wasm_bindgen(js_name = validateMessage)]
pub fn validate_message(message: &WasmMessage) -> Result<WasmValidationResult, JsValue> {
    // get_version returns Option<Version>
    let version = message.inner().get_version().unwrap_or(Version::V2_5);
    let validator = Validator::new(version);
    let result = validator.validate(message.inner());

    Ok(WasmValidationResult::from(result))
}

/// Create a new HL7 message with MSH segment
///
/// # Arguments
///
/// * `version` - HL7 version string (e.g., "2.5")
/// * `message_type` - Message type (e.g., "ADT^A01")
/// * `sending_app` - Sending application
/// * `sending_fac` - Sending facility
///
/// # Returns
///
/// A new message with MSH segment populated
#[wasm_bindgen(js_name = createMessage)]
pub fn create_message(
    version: &str,
    message_type: &str,
    sending_app: &str,
    sending_fac: &str,
) -> Result<WasmMessage, JsValue> {
    // Build MSH segment
    let msh = format!(
        "MSH|^~\\&|{}|{}|||{}||{}|||{}",
        sending_app,
        sending_fac,
        chrono::Utc::now().format("%Y%m%d%H%M%S"),
        message_type,
        version
    );

    let parsed = vyna_hl7_parse(&msh).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(WasmMessage::from(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_parse_message() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123||DOE^JOHN||19800101|M";

        let result = parse_message(hl7);
        assert!(result.is_ok());
    }

    #[wasm_bindgen_test]
    fn test_terser_get() {
        let hl7 = "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
                   PID|1||MRN123||DOE^JOHN||19800101|M";

        let msg = parse_message(hl7).unwrap();
        // A whole-field read returns the first component.
        let value = get_terser_value(&msg, "PID-5").unwrap();
        assert_eq!(value, Some("DOE".to_string()));
        let given = get_terser_value(&msg, "PID-5-2").unwrap();
        assert_eq!(given, Some("JOHN".to_string()));
    }
}
