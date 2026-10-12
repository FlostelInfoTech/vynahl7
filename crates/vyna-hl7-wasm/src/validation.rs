//! Validation utilities for WebAssembly

use wasm_bindgen::prelude::*;
use vyna_hl7_validator::ValidationResult;
use serde::Serialize;

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
            errors: result.errors.iter().map(|e| WasmValidationError {
                location: e.location.clone(),
                message: e.message.clone(),
                severity: "error".to_string(),
            }).collect(),
            warnings: result.warnings.iter().map(|w| WasmValidationError {
                location: w.location.clone(),
                message: w.message.clone(),
                severity: "warning".to_string(),
            }).collect(),
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
            errors: self.errors.iter().map(|e| JsonError {
                location: e.location.clone(),
                message: e.message.clone(),
                severity: e.severity.clone(),
            }).collect(),
            warnings: self.warnings.iter().map(|w| JsonError {
                location: w.location.clone(),
                message: w.message.clone(),
                severity: w.severity.clone(),
            }).collect(),
        };

        serde_wasm_bindgen::to_value(&json)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

#[derive(Clone, Serialize)]
struct WasmValidationError {
    location: String,
    message: String,
    severity: String,
}
