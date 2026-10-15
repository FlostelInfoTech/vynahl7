//! Enhanced Terser API Examples
//!
//! This example demonstrates the enhanced Terser capabilities in VynaHL7:
//! - BulkTerser: Bulk field extraction and pattern matching
//! - Iterators: Field iteration over repeating segments
//! - TerserQuery: Conditional queries and filtering
//!
//! Run with: cargo run --example enhanced_terser

use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::{BulkTerser, Terser, TerserQuery};

fn main() {
    println!("=== Enhanced Terser API Examples ===\n");

    // Sample HL7 ORU message with multiple OBX segments
    let hl7 = r"MSH|^~\&|LAB|HOSPITAL|EMR|CLINIC|20250120100000||ORU^R01|MSG001|P|2.5
PID|1||PAT12345||SMITH^JOHN^A^^^DR||19800515|M|||123 MAIN ST^^BOSTON^MA^02101||555-1234~555-5678|||S||ACC001
PV1|1|I|ICU^101^A||||DOC123^JONES^ROBERT^^^MD||||||||||VIS001
OBR|1|ORD001|RES001|CBC^Complete Blood Count|||20250120090000
OBX|1|NM|WBC^White Blood Count||7.5|10*3/uL|4.0-11.0|N|||F
OBX|2|NM|RBC^Red Blood Count||4.8|10*6/uL|4.5-5.9|N|||F
OBX|3|NM|HGB^Hemoglobin||14.2|g/dL|13.5-17.5|N|||F
OBX|4|NM|HCT^Hematocrit||42.1|%|38.8-50.0|N|||F
OBX|5|NM|PLT^Platelet Count||250|10*3/uL|150-400|N|||F
OBX|6|NM|GLUCOSE^Glucose||98|mg/dL|70-100|N|||F
OBX|7|NM|CREAT^Creatinine||1.1|mg/dL|0.7-1.3|N|||F
OBX|8|NM|BUN^Blood Urea Nitrogen||18|mg/dL|7-20|N|||F";

    let message = parse_message(hl7).expect("Failed to parse message");

    // Example 1: Bulk Field Extraction
    println!("=== Example 1: Bulk Field Extraction ===");
    bulk_extraction_example(&message);

    // Example 2: Pattern Matching
    println!("\n=== Example 2: Pattern Matching ===");
    pattern_matching_example(&message);

    // Example 3: Field Iteration
    println!("\n=== Example 3: Field Iteration ===");
    field_iteration_example(&message);

    // Example 4: Component Iteration
    println!("\n=== Example 4: Component Iteration ===");
    component_iteration_example(&message);

    // Example 5: Conditional Queries
    println!("\n=== Example 5: Conditional Queries ===");
    conditional_queries_example(&message);

    // Example 6: Filtering Segments
    println!("\n=== Example 6: Filtering Segments ===");
    filtering_segments_example(&message);

    // Example 7: Complex Query Patterns
    println!("\n=== Example 7: Complex Query Patterns ===");
    complex_queries_example(&message);

    println!("\n=== All Examples Complete ===");
}

/// Example 1: Extract multiple fields at once using BulkTerser
fn bulk_extraction_example(message: &vyna_hl7_core::message::Message) {
    let bulk = BulkTerser::new(message);

    // Extract patient demographics in one call
    let paths = vec![
        "PID-5-1", // Family name
        "PID-5-2", // Given name
        "PID-7",   // Date of birth
        "PID-8",   // Gender
        "PID-11",  // Address
        "PID-13",  // Phone numbers
    ];

    match bulk.get_multiple(&paths) {
        Ok(results) => {
            println!("Patient Demographics:");
            println!(
                "  Last Name: {}",
                results.get("PID-5-1").and_then(|v| *v).unwrap_or("N/A")
            );
            println!(
                "  First Name: {}",
                results.get("PID-5-2").and_then(|v| *v).unwrap_or("N/A")
            );
            println!(
                "  DOB: {}",
                results.get("PID-7").and_then(|v| *v).unwrap_or("N/A")
            );
            println!(
                "  Gender: {}",
                results.get("PID-8").and_then(|v| *v).unwrap_or("N/A")
            );
        }
        Err(e) => println!("Error: {}", e),
    }
}

/// Example 2: Use pattern matching to extract all observations
fn pattern_matching_example(message: &vyna_hl7_core::message::Message) {
    let bulk = BulkTerser::new(message);

    // Extract all observation values using wildcard
    match bulk.get_pattern("OBX(*)-5") {
        Ok(results) => {
            println!("All Lab Values:");
            for (path, value) in results {
                println!("  {}: {}", path, value);
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Extract all observation identifiers
    match bulk.get_all_from_segments("OBX", 3) {
        Ok(test_ids) => {
            println!("\nTest Identifiers:");
            for id in test_ids {
                println!("  - {}", id);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
}

/// Example 3: Iterate over field values from repeating segments
fn field_iteration_example(message: &vyna_hl7_core::message::Message) {
    let terser = Terser::new(message);

    // Iterate over all observation values
    println!("Observation Values (using iterator):");
    for value in terser.iter_field("OBX", 5) {
        println!("  - {}", value);
    }

    // Count numeric values
    let count = terser
        .iter_field("OBX", 5)
        .filter(|v| v.parse::<f64>().is_ok())
        .count();
    println!("\nNumeric values count: {}", count);

    // Get all values above a threshold
    let high_values: Vec<_> = terser
        .iter_field("OBX", 5)
        .filter_map(|v| v.parse::<f64>().ok())
        .filter(|&v| v > 100.0)
        .collect();
    println!("Values > 100: {:?}", high_values);
}

/// Example 4: Iterate over components
fn component_iteration_example(message: &vyna_hl7_core::message::Message) {
    let terser = Terser::new(message);

    // Get all observation codes (first component of field 3)
    println!("Observation Codes:");
    for code in terser.iter_component("OBX", 3, 1) {
        println!("  - {}", code);
    }

    // Get all observation names (second component of field 3)
    println!("\nObservation Names:");
    for name in terser.iter_component("OBX", 3, 2) {
        println!("  - {}", name);
    }

    // Iterate over phone numbers (field 13 has repetitions)
    println!("\nPatient Phone Numbers:");
    for phone in terser.iter_repetitions("PID", 13, 0) {
        println!("  - {}", phone);
    }
}

/// Example 5: Use conditional queries to find specific segments
fn conditional_queries_example(message: &vyna_hl7_core::message::Message) {
    let query = TerserQuery::new(message);

    // Find the first OBX segment with a specific test code
    match query.find_first("OBX", 3, "GLUCOSE") {
        Some(segment) => {
            println!("Found Glucose test:");
            if let Some(value) = segment.get_field(5).and_then(|f| f.value()) {
                println!("  Value: {}", value);
            }
            if let Some(units) = segment.get_field(6).and_then(|f| f.value()) {
                println!("  Units: {}", units);
            }
            if let Some(range) = segment.get_field(7).and_then(|f| f.value()) {
                println!("  Reference Range: {}", range);
            }
        }
        None => println!("Glucose test not found"),
    }

    // Check if any abnormal results exist
    let has_abnormal = query.any_match("OBX", |seg| {
        seg.get_field(8)
            .and_then(|f| f.value())
            .map(|v| v != "N")
            .unwrap_or(false)
    });
    println!("\nHas abnormal results: {}", has_abnormal);

    // Check if all results are final
    let all_final = query.all_match("OBX", |seg| {
        seg.get_field(11)
            .and_then(|f| f.value())
            .map(|v| v == "F")
            .unwrap_or(false)
    });
    println!("All results final: {}", all_final);
}

/// Example 6: Filter segments by field values
fn filtering_segments_example(message: &vyna_hl7_core::message::Message) {
    let query = TerserQuery::new(message);

    // Get all numeric observation types
    let numeric_obs = query.filter_repeating("OBX", 2, "NM");
    println!("Numeric Observations: {} found", numeric_obs.len());

    // Get observations for specific test codes
    let cbc_tests = query.filter_by_component("OBX", 3, 1, "WBC");
    println!("WBC tests found: {}", cbc_tests.len());

    // Count observations with abnormal flags
    let abnormal_count = query.count_where("OBX", |seg| {
        seg.get_field(8)
            .and_then(|f| f.value())
            .map(|v| v == "A" || v == "H" || v == "L")
            .unwrap_or(false)
    });
    println!("Abnormal results: {}", abnormal_count);
}

/// Example 7: Complex query patterns
fn complex_queries_example(message: &vyna_hl7_core::message::Message) {
    let query = TerserQuery::new(message);

    // Get test names for all numeric observations
    let test_names = query.get_values_where("OBX", 2, "NM", 3);
    println!("Numeric Test Names:");
    for name in test_names {
        println!("  - {}", name);
    }

    // Get values for specific test types
    println!("\nBlood Count Results:");
    for test in &["WBC", "RBC", "HGB", "HCT", "PLT"] {
        let segments = query.filter_by_component("OBX", 3, 1, test);
        for seg in segments {
            if let Some(value) = seg.get_field(5).and_then(|f| f.value())
                && let Some(units) = seg.get_field(6).and_then(|f| f.value())
            {
                println!("  {}: {} {}", test, value, units);
            }
        }
    }

    // Conditional extraction based on message state
    println!("\nConditional Field Access:");
    let value = query.get_if("PID-5", |t| {
        // Only get patient name if visit number exists
        t.get("PV1-19").ok().flatten().is_some()
    });

    if let Some(name) = value {
        println!("  Patient name (visit active): {}", name);
    } else {
        println!("  Patient name: Not available (no active visit)");
    }
}
