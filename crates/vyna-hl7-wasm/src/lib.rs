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
use vyna_hl7_core::builders::generate_control_id;
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
    // Select the message schema from MSH-9 so validation checks
    // structure, not just the envelope (CLI/testpanel parity). ACK
    // carries no trigger event; types without a schema fall back to
    // structure-only validation.
    let schema_key = message.inner().get_message_type().map(|(t, e)| {
        if t == "ACK" {
            (t, String::new())
        } else {
            (t, e)
        }
    });
    let mut validator = Validator::new(version);
    if let Some((msg_type, trigger)) = &schema_key
        && let Ok(schema_validator) = Validator::for_message_type(version, msg_type, trigger)
    {
        validator = schema_validator;
    }
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
/// Reject empty values and raw HL7 structural characters in a
/// value interpolated into the MSH segment, where they would shift
/// field positions or inject segments. `message_type` legitimately
/// uses `^` (`ADT^A01`), so callers allow it for that value only.
/// Returns a plain string because `JsValue` cannot be built on
/// non-wasm test targets; callers map it at the boundary.
fn validate_msh_value(name: &str, value: &str, allow_caret: bool) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("invalid {name}: value must not be empty"));
    }
    let bad = value.chars().find(|c| {
        matches!(c, '|' | '^' | '~' | '&' | '\\' | '\r' | '\n') && (*c != '^' || !allow_caret)
    });
    if let Some(bad) = bad {
        return Err(format!(
            "invalid {name}: character {bad:?} would corrupt MSH field positions"
        ));
    }
    Ok(())
}

#[wasm_bindgen(js_name = createMessage)]
pub fn create_message(
    version: &str,
    message_type: &str,
    sending_app: &str,
    sending_fac: &str,
) -> Result<WasmMessage, JsValue> {
    validate_msh_value("version", version, false).map_err(|e| JsValue::from_str(&e))?;
    crate::definitions::validate_version_string(version).map_err(|e| JsValue::from_str(&e))?;
    validate_msh_value("message_type", message_type, true).map_err(|e| JsValue::from_str(&e))?;
    validate_msh_value("sending_app", sending_app, false).map_err(|e| JsValue::from_str(&e))?;
    validate_msh_value("sending_fac", sending_fac, false).map_err(|e| JsValue::from_str(&e))?;

    // Build MSH segment. MSH-10 (control ID) and MSH-11 (processing
    // ID) are required by the schemas: generate the control ID and
    // default the processing ID to "P" (Production) like the core
    // builders; callers can change it via setTerserValue.
    let control_id = generate_control_id();
    let msh = format!(
        "MSH|^~\\&|{}|{}|||{}||{}|{}|P|{}",
        sending_app,
        sending_fac,
        chrono::Utc::now().format("%Y%m%d%H%M%S"),
        message_type,
        control_id,
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

    #[test]
    fn validate_message_flags_missing_required_segments() {
        // MSH-only ADT^A01 lacks required EVN/PID/PV1 segments. The
        // schema selected from MSH-9 must surface that (CLI parity).
        let msg = parse_message("MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5").unwrap();
        let result = validate_message(&msg).unwrap();
        assert!(!result.is_valid());
        assert!(result.error_count() > 0);
    }

    #[test]
    fn validate_message_accepts_complete_adt_a01() {
        let msg = parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\r\
             EVN||20240101\rPID|1||7||DOE^JOHN\rPV1||I\r",
        )
        .unwrap();
        let result = validate_message(&msg).unwrap();
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn validate_message_falls_back_without_schema() {
        // Unknown message types have no schema; validation falls back
        // to structure-only instead of erroring.
        let msg =
            parse_message("MSH|^~\\&|S|F|R|D|20240101||ZZZ^Z99|M1|P|2.5\rPID|1||7\r").unwrap();
        let result = validate_message(&msg).unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn validate_msh_value_rejects_field_separator() {
        // A raw `|` interpolated into MSH-3 shifts every later MSH
        // field (pre-fix: MSH-3="BAD", MSH-4="APP"). The
        // `create_message` error path itself cannot run natively
        // (`JsValue` needs wasm), so the guard is tested directly.
        let err = validate_msh_value("sending_app", "BAD|APP", false).unwrap_err();
        assert!(err.contains("sending_app"), "unclear error: {err}");
        assert!(err.contains("'|'"), "unclear error: {err}");
    }

    #[test]
    fn validate_msh_value_rejects_empty_and_delimiter_values() {
        assert!(
            validate_msh_value("sending_app", "", false)
                .unwrap_err()
                .contains("empty")
        );
        assert!(validate_msh_value("sending_fac", "", false).is_err());
        assert!(validate_msh_value("message_type", "", true).is_err());
        assert!(validate_msh_value("message_type", "ADT|A01", true).is_err());
        assert!(validate_msh_value("sending_app", "A\rMSH|x", false).is_err());
        assert!(validate_msh_value("sending_fac", "F&B", false).is_err());
        assert!(validate_msh_value("sending_app", "A~B", false).is_err());
        // Happy paths still pass, including `^` in message types.
        assert!(validate_msh_value("message_type", "ADT^A01", true).is_ok());
        assert!(validate_msh_value("sending_app", "SendApp", false).is_ok());
        assert!(validate_msh_value("version", "2.5", false).is_ok());
    }

    #[test]
    fn create_message_rejects_unknown_version() {
        // The `create_message` error path builds a `JsValue`, which
        // cannot run natively, so the shared guard is tested directly;
        // `create_message` maps its error like `parse_version` does.
        let err = crate::definitions::validate_version_string("9.9").unwrap_err();
        assert!(err.contains("Unsupported HL7 version: '9.9'"), "{err}");
        assert!(crate::definitions::validate_version_string("2.5").is_ok());
    }

    #[wasm_bindgen_test]
    fn test_create_message_rejects_unknown_version() {
        assert!(create_message("9.9", "ADT^A01", "App", "Fac").is_err());
        assert!(create_message("2.5", "ADT^A01", "App", "Fac").is_ok());
    }

    #[test]
    fn create_message_populates_control_and_processing_ids() {
        let msg = create_message("2.5", "ADT^A01", "SendApp", "SendFac").unwrap();
        let control_id = msg.control_id().unwrap_or_default();
        assert!(!control_id.is_empty(), "MSH-10 must be generated");
        assert!(control_id.len() <= 20, "MSH-10 exceeds max length 20");
        let terser = Terser::new(msg.inner());
        assert_eq!(terser.get("MSH-11").unwrap(), Some("P"));
        // Required MSH-10/MSH-11 schema violations must be gone.
        let version = msg.inner().get_version().unwrap();
        let (msg_type, trigger) = msg.inner().get_message_type().unwrap();
        let validator = Validator::for_message_type(version, &msg_type, &trigger).unwrap();
        let result = validator.validate(msg.inner());
        assert!(
            result
                .errors
                .iter()
                .all(|e| !e.location.starts_with("MSH[0]-10")
                    && !e.location.starts_with("MSH[0]-11")),
            "MSH-10/MSH-11 errors remain: {:?}",
            result.errors
        );
    }
}
