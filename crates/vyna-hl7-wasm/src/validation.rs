//! Validation utilities for WebAssembly

use serde::Serialize;
use vyna_hl7_validator::ValidationResult;
use wasm_bindgen::prelude::*;

/// WebAssembly-friendly validation result
#[wasm_bindgen]
pub struct WasmValidationResult {
    is_valid: bool,
    errors: Vec<WasmValidationError>,
    warnings: Vec<WasmValidationError>,
}

impl From<ValidationResult> for WasmValidationResult {
    fn from(result: ValidationResult) -> Self {
        Self {
            is_valid: result.is_valid(),
            errors: result
                .errors
                .iter()
                .map(|e| WasmValidationError {
                    location: e.location.clone(),
                    message: e.message.clone(),
                    severity: "error".to_string(),
                })
                .collect(),
            warnings: result
                .warnings
                .iter()
                .map(|w| WasmValidationError {
                    location: w.location.clone(),
                    message: w.message.clone(),
                    severity: "warning".to_string(),
                })
                .collect(),
        }
    }
}

#[wasm_bindgen]
impl WasmValidationResult {
    /// Check if the message is valid
    #[wasm_bindgen(js_name = isValid)]
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }

    /// Get the number of errors
    #[wasm_bindgen(js_name = errorCount)]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get the number of warnings
    #[wasm_bindgen(js_name = warningCount)]
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Convert to JSON
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        #[derive(Serialize)]
        struct JsonResult {
            is_valid: bool,
            errors: Vec<JsonError>,
            warnings: Vec<JsonError>,
        }

        #[derive(Serialize)]
        struct JsonError {
            location: String,
            message: String,
            severity: String,
        }

        let json = JsonResult {
            is_valid: self.is_valid,
            errors: self
                .errors
                .iter()
                .map(|e| JsonError {
                    location: e.location.clone(),
                    message: e.message.clone(),
                    severity: e.severity.clone(),
                })
                .collect(),
            warnings: self
                .warnings
                .iter()
                .map(|w| JsonError {
                    location: w.location.clone(),
                    message: w.message.clone(),
                    severity: w.severity.clone(),
                })
                .collect(),
        };

        serde_wasm_bindgen::to_value(&json).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[derive(Clone, Serialize)]
struct WasmValidationError {
    location: String,
    message: String,
    severity: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::Version;
    use vyna_hl7_parser::parse_message as vyna_parse;
    use vyna_hl7_validator::Validator;

    #[test]
    fn invalid_message_surfaces_errors() {
        // Structurally incomplete ADT: missing PID-5, PV1, EVN.
        let message =
            vyna_parse("MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7\r").unwrap();
        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
        let result = WasmValidationResult::from(validator.validate(&message));
        assert!(!result.is_valid());
        assert!(result.error_count() > 0);
    }

    #[test]
    fn valid_message_reports_clean() {
        let message = vyna_parse(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\r\
             EVN||20240101\rPID|1||7||DOE^JOHN\rPV1||I\r",
        )
        .unwrap();
        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
        let result = WasmValidationResult::from(validator.validate(&message));
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
    }
}
