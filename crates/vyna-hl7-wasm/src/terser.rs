//! Terser utilities for WebAssembly

use wasm_bindgen::prelude::*;
use vyna_hl7_core::Message;
use vyna_hl7_terser::Terser;
use crate::WasmMessage;

/// Read one field's full encoded value (all components).
///
/// A whole-field `Terser::get` returns only the first component, which
/// truncates composite values (PID-11 addresses, PID-13 phones, CWE
/// OBX-5 results); encoding the field preserves every component.
/// Returns `None` when the segment, field, or first repetition is
/// absent, matching `Terser::get`.
fn full_field_value(
    message: &Message,
    segment_id: &str,
    segment_index: usize,
    field_index: usize,
) -> Option<String> {
    let segment = message
        .get_segments_by_id(segment_id)
        .get(segment_index)
        .copied()?;
    let field = segment.get_field(field_index)?;
    field.get_repetition(0)?;
    Some(field.encode(&message.delimiters))
}

/// Get multiple values at once using Terser paths
///
/// # Arguments
///
/// * `message` - The message to query
/// * `paths` - Array of Terser paths
///
/// # Returns
///
/// An object mapping paths to values
#[wasm_bindgen(js_name = getTerserValues)]
pub fn get_terser_values(message: &WasmMessage, paths: Vec<String>) -> Result<JsValue, JsValue> {
    let terser = Terser::new(message.inner());
    let mut results = serde_json::Map::new();

    for path in paths {
        let value = terser.get(&path)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        if let Some(v) = value {
            results.insert(path, serde_json::Value::String(v.to_string()));
        } else {
            results.insert(path, serde_json::Value::Null);
        }
    }

    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract patient demographics from a message
///
/// # Arguments
///
/// * `message` - The message (typically ADT)
///
/// # Returns
///
/// An object with patient demographic fields
#[wasm_bindgen(js_name = extractPatientDemographics)]
pub fn extract_patient_demographics(message: &WasmMessage) -> Result<JsValue, JsValue> {
    let terser = Terser::new(message.inner());

    #[derive(serde::Serialize)]
    struct Demographics {
        patient_id: Option<String>,
        mrn: Option<String>,
        family_name: Option<String>,
        given_name: Option<String>,
        date_of_birth: Option<String>,
        gender: Option<String>,
        address: Option<String>,
        phone: Option<String>,
    }

    let demographics = Demographics {
        patient_id: terser.get("PID-2").ok().flatten().map(|s| s.to_string()),
        mrn: terser.get("PID-3").ok().flatten().map(|s| s.to_string()),
        family_name: terser.get("PID-5-1").ok().flatten().map(|s| s.to_string()),
        given_name: terser.get("PID-5-2").ok().flatten().map(|s| s.to_string()),
        date_of_birth: terser.get("PID-7").ok().flatten().map(|s| s.to_string()),
        gender: terser.get("PID-8").ok().flatten().map(|s| s.to_string()),
        address: full_field_value(message.inner(), "PID", 0, 11),
        phone: full_field_value(message.inner(), "PID", 0, 13),
    };

    serde_wasm_bindgen::to_value(&demographics)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract observations from an ORU message
///
/// # Arguments
///
/// * `message` - The ORU message
///
/// # Returns
///
/// An array of observation objects
#[wasm_bindgen(js_name = extractObservations)]
pub fn extract_observations(message: &WasmMessage) -> Result<JsValue, JsValue> {
    let terser = Terser::new(message.inner());

    #[derive(serde::Serialize)]
    struct Observation {
        set_id: Option<String>,
        value_type: Option<String>,
        identifier: Option<String>,
        test_name: Option<String>,
        value: Option<String>,
        units: Option<String>,
        reference_range: Option<String>,
        abnormal_flags: Option<String>,
        status: Option<String>,
        observation_datetime: Option<String>,
    }

    // Count OBX segments
    let obx_count = message.inner().segments.iter()
        .filter(|s| s.id == "OBX")
        .count();

    let mut observations = Vec::new();

    for i in 0..obx_count {
        // Terser segment indexes are 1-based: OBX(1) is the first OBX.
        let prefix = if i == 0 {
            "OBX".to_string()
        } else {
            format!("OBX({})", i + 1)
        };

        let obs = Observation {
            set_id: terser.get(&format!("{}-1", prefix)).ok().flatten().map(|s| s.to_string()),
            value_type: terser.get(&format!("{}-2", prefix)).ok().flatten().map(|s| s.to_string()),
            identifier: terser.get(&format!("{}-3", prefix)).ok().flatten().map(|s| s.to_string()),
            test_name: terser.get(&format!("{}-3-2", prefix)).ok().flatten().map(|s| s.to_string()),
            value: full_field_value(message.inner(), "OBX", i, 5),
            units: terser.get(&format!("{}-6", prefix)).ok().flatten().map(|s| s.to_string()),
            reference_range: terser.get(&format!("{}-7", prefix)).ok().flatten().map(|s| s.to_string()),
            abnormal_flags: terser.get(&format!("{}-8", prefix)).ok().flatten().map(|s| s.to_string()),
            status: terser.get(&format!("{}-11", prefix)).ok().flatten().map(|s| s.to_string()),
            observation_datetime: terser.get(&format!("{}-14", prefix)).ok().flatten().map(|s| s.to_string()),
        };

        observations.push(obs);
    }

    serde_wasm_bindgen::to_value(&observations)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_message as parse_wasm;
    use wasm_bindgen_test::*;

    fn from_js(value: JsValue) -> serde_json::Value {
        serde_wasm_bindgen::from_value(value).unwrap()
    }

    #[wasm_bindgen_test]
    fn test_demographics_reads_1_based_name_components() {
        let msg = parse_wasm(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123||DOE^JOHN||19800101|M",
        )
        .unwrap();

        let demographics = from_js(extract_patient_demographics(&msg).unwrap());

        assert_eq!(demographics["family_name"], "DOE");
        assert_eq!(demographics["given_name"], "JOHN");
        assert_eq!(demographics["mrn"], "MRN123");
    }

    #[test]
    fn full_field_value_keeps_all_components() {
        let msg = parse_wasm(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\r\
             PID|1||7||DOE^JOHN||19800101|M|||123 Main St^^Boston^MA^02101||\
             ^PRN^PH^^1^617^5551234",
        )
        .unwrap();
        // A whole-field Terser::get returns only the first component
        // ("123 Main St", ""); the demographics extract must keep all.
        assert_eq!(
            full_field_value(msg.inner(), "PID", 0, 11).as_deref(),
            Some("123 Main St^^Boston^MA^02101")
        );
        assert_eq!(
            full_field_value(msg.inner(), "PID", 0, 13).as_deref(),
            Some("^PRN^PH^^1^617^5551234")
        );
        assert_eq!(
            full_field_value(msg.inner(), "PID", 0, 12).as_deref(),
            Some(""),
            "empty field must match Terser::get"
        );
        assert_eq!(
            full_field_value(msg.inner(), "PID", 0, 30),
            None,
            "absent field must stay None like Terser::get"
        );
        assert_eq!(
            full_field_value(msg.inner(), "NK1", 0, 2),
            None,
            "absent segment must stay None like Terser::get"
        );
    }

    #[test]
    fn full_field_value_keeps_obx5_display_text() {
        let msg = parse_wasm(
            "MSH|^~\\&|S|F|R|D|20240101||ORU^R01|M1|P|2.5\r\
             PID|1||7\rOBX|1|CWE|T^Test^L||POS^Positive^LN||||||F",
        )
        .unwrap();
        assert_eq!(
            full_field_value(msg.inner(), "OBX", 0, 5).as_deref(),
            Some("POS^Positive^LN")
        );
    }

    #[wasm_bindgen_test]
    fn test_demographics_keep_full_address_and_phone() {
        let msg = parse_wasm(
            "MSH|^~\\&|SendApp|SendFac|RecApp|RecFac|20240315||ADT^A01|12345|P|2.5\r\
             PID|1||MRN123||DOE^JOHN||19800101|M|||123 Main St^^Boston^MA^02101||\
             ^PRN^PH^^1^617^5551234",
        )
        .unwrap();

        let demographics = from_js(extract_patient_demographics(&msg).unwrap());

        assert_eq!(demographics["address"], "123 Main St^^Boston^MA^02101");
        assert_eq!(demographics["phone"], "^PRN^PH^^1^617^5551234");
    }

    #[wasm_bindgen_test]
    fn test_observations_keep_full_cwe_value() {
        let msg = parse_wasm(
            "MSH|^~\\&|App|Fac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
             PID|1||MRN123||DOE^JOHN||19800101|M\r\
             OBX|1|CWE|T^Test^L||POS^Positive^LN||||||F",
        )
        .unwrap();

        let observations = from_js(extract_observations(&msg).unwrap());

        assert_eq!(observations[0]["value"], "POS^Positive^LN");
    }

    #[wasm_bindgen_test]
    fn test_observations_read_each_obx_segment_with_test_name() {
        let msg = parse_wasm(
            "MSH|^~\\&|App|Fac|RecApp|RecFac|20240315||ORU^R01|12345|P|2.5\r\
             PID|1||MRN123||DOE^JOHN||19800101|M\r\
             OBX|1|NM|GLU^Glucose^L||98|mg/dL|||||F\r\
             OBX|2|NM|NA^Sodium^L||140|mmol/L|||||F",
        )
        .unwrap();

        let observations = from_js(extract_observations(&msg).unwrap());

        assert_eq!(observations[0]["value"], "98");
        assert_eq!(observations[0]["test_name"], "Glucose");
        assert_eq!(observations[1]["value"], "140");
        assert_eq!(observations[1]["test_name"], "Sodium");
    }
}
