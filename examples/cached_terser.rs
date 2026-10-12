//! Example demonstrating the performance benefits of CachedTerser
//!
//! This example shows how to use CachedTerser for improved performance
//! when accessing the same fields multiple times.

use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::{Terser, CachedTerser};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample ADT message with patient demographics
    let hl7 = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|12345|P|2.5
PID|1|PAT001|MRN123456^^^Hospital^MR||DOE^JOHN^ALLEN^JR^DR^PhD||19800101|M|||123 Main St^Apt 4B^Boston^MA^02101^USA||555-1234^PRN^PH~555-5678^WPN^PH||EN^English^HL70296|M^Married^HL70002|||123-45-6789|||N
PV1|1|I|ER^101^1^Hospital^^^^^ER|||12345^SMITH^JANE^M^^MD^L^^^NPI||||MED||||1|||12345^SMITH^JANE^M^^MD^L^^^NPI|INT|Visit001^^^Hospital^VN|||||||||||||||||||||||||20240315100000
OBX|1|NM|WBC^White Blood Count^LN||7.5|10*3/uL|4.5-11.0|N|||F|||20240315120000
OBX|2|NM|RBC^Red Blood Count^LN||4.8|10*6/uL|4.2-5.9|N|||F|||20240315120000
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315120000";

    let message = parse_message(hl7)?;

    println!("=== CachedTerser Performance Demo ===\n");

    // Define the fields we'll be accessing repeatedly
    let fields = vec![
        "PID-5",    // Patient name
        "PID-5-1",  // Family name (1-based component indexing)
        "PID-5-2",  // Given name (1-based component indexing)
        "PID-7",    // Date of birth
        "PID-8",    // Gender
        "PID-11",   // Address
        "PID-11-1", // Street (1-based component indexing)
        "PID-11-3", // City (1-based component indexing)
        "PID-13",   // Phone
        "PV1-2",    // Patient class
        "PV1-7",    // Attending doctor
        "OBX-3",    // Observation identifier (first OBX)
        "OBX-5",    // Observation value (first OBX)
        "OBX(2)-3", // Second observation identifier (1-based segment indexing)
        "OBX(2)-5", // Second observation value (1-based segment indexing)
    ];

    // Part 1: Regular Terser (no caching)
    println!("1. Regular Terser (no caching):");
    let terser = Terser::new(&message);

    let start = Instant::now();
    let iterations = 1000;

    for _ in 0..iterations {
        for field in &fields {
            let _ = terser.get(field)?;
        }
    }

    let regular_duration = start.elapsed();
    let regular_per_access = regular_duration.as_nanos() / (iterations * fields.len() as u128);

    println!("   Total time: {:?}", regular_duration);
    println!("   Per-access: {} ns\n", regular_per_access);

    // Part 2: CachedTerser (with caching)
    println!("2. CachedTerser (with caching):");
    let mut cached_terser = CachedTerser::with_capacity(&message, fields.len());

    let start = Instant::now();

    for _ in 0..iterations {
        for field in &fields {
            let _ = cached_terser.get(field)?;
        }
    }

    let cached_duration = start.elapsed();
    let cached_per_access = cached_duration.as_nanos() / (iterations * fields.len() as u128);

    println!("   Total time: {:?}", cached_duration);
    println!("   Per-access: {} ns", cached_per_access);
    println!("   Cache size: {} entries\n", cached_terser.cache_size());

    // Part 3: CachedTerser with pre-warming
    println!("3. CachedTerser with cache warming:");
    let mut warmed_terser = CachedTerser::with_capacity(&message, fields.len());

    // Pre-warm the cache
    let warm_start = Instant::now();
    let field_strs: Vec<&str> = fields.iter().map(|s| s.as_ref()).collect();
    warmed_terser.warm_cache(&field_strs)?;
    let warm_duration = warm_start.elapsed();

    println!("   Warming time: {:?}", warm_duration);

    let start = Instant::now();

    for _ in 0..iterations {
        for field in &fields {
            let _ = warmed_terser.get(field)?;
        }
    }

    let warmed_duration = start.elapsed();
    let warmed_per_access = warmed_duration.as_nanos() / (iterations * fields.len() as u128);

    println!("   Total time: {:?}", warmed_duration);
    println!("   Per-access: {} ns\n", warmed_per_access);

    // Summary
    println!("=== Performance Summary ===");
    println!("Regular Terser:  {} ns/access", regular_per_access);
    println!("Cached Terser:   {} ns/access ({:.1}x faster)",
        cached_per_access, regular_per_access as f64 / cached_per_access as f64);
    println!("Warmed Cache:    {} ns/access ({:.1}x faster)",
        warmed_per_access, regular_per_access as f64 / warmed_per_access as f64);

    println!("\n=== Sample Field Values ===");
    let mut terser = CachedTerser::new(&message);

    println!("Patient Name: {}", terser.get("PID-5")?.unwrap_or("N/A"));
    println!("  Family: {}", terser.get("PID-5-1")?.unwrap_or("N/A"));
    println!("  Given:  {}", terser.get("PID-5-2")?.unwrap_or("N/A"));
    println!("  Middle: {}", terser.get("PID-5-3")?.unwrap_or("N/A"));
    println!("Date of Birth: {}", terser.get("PID-7")?.unwrap_or("N/A"));
    println!("Gender: {}", terser.get("PID-8")?.unwrap_or("N/A"));
    println!("Patient Class: {}", terser.get("PV1-2")?.unwrap_or("N/A"));
    println!("Attending Doctor: {}", terser.get("PV1-7")?.unwrap_or("N/A"));

    println!("\nObservations:");
    for i in 0..3 {
        // Use 1-based segment indexing: OBX or OBX(1) for first, OBX(2) for second, etc.
        let path_prefix = if i == 0 { String::from("OBX") } else { format!("OBX({})", i + 1) };
        let test_name = terser.get(&format!("{}-3-1", path_prefix))?.unwrap_or("N/A").to_string();
        let value = terser.get(&format!("{}-5", path_prefix))?.unwrap_or("N/A").to_string();
        let units = terser.get(&format!("{}-6", path_prefix))?.unwrap_or("").to_string();
        println!("  {}: {} {}", test_name, value, units);
    }

    println!("\nCache statistics:");
    println!("  Entries: {}", terser.cache_size());
    println!("  Memory: ~{} bytes", terser.cache_size() * 100);

    Ok(())
}
