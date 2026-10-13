//! Utility functions for WebAssembly

use wasm_bindgen::prelude::*;

/// Set up better panic messages in wasm
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Bindings to the browser console API
///
/// These wrap `console.log`, `console.warn`, and `console.error` for
/// diagnostics from WebAssembly. Prefer the [`console_log!`](crate::console_log)
/// macro for formatted informational messages.
#[wasm_bindgen]
extern "C" {
    /// Write an informational message to `console.log`.
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    /// Write a warning message to `console.warn`.
    #[wasm_bindgen(js_namespace = console)]
    pub fn warn(s: &str);

    /// Write an error message to `console.error`.
    #[wasm_bindgen(js_namespace = console)]
    pub fn error(s: &str);
}

/// Macro for logging to console
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => {
        $crate::utils::log(&format!($($t)*))
    };
}
