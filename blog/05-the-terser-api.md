# The Terser API: Elegant Field Access for HL7 Messages

*Part 5 of a 12-part series on VynaHL7, a comprehensive Rust library for HL7 v2 healthcare message processing.*

---

In the [previous post](./04-understanding-hl7-v2-messages.md), we explored the hierarchical structure of HL7 v2 messages. While understanding this structure is important, manually navigating it can be tedious. That's where Terser comes in.

The Terser API, inspired by the HAPI library's Terser class, provides path-based field access using a simple, intuitive notation. Instead of traversing segments, fields, repetitions, and components manually, you simply specify a path like `PID-5-1` to get the patient's family name.

## Path Notation Basics

Terser uses a path notation that maps directly to HL7's structure:

| Pattern | Meaning | Example |
|---------|---------|---------|
| `SEG-F` | Segment, Field | `PID-5` - Patient Name field |
| `SEG-F-C` | Segment, Field, Component | `PID-5-1` - Family name |
| `SEG-F-C-S` | With Subcomponent | `PID-5-1-2` - Subcomponent of family name |
| `SEG(N)-F` | Nth occurrence of segment | `OBX(2)-5` - Second OBX, field 5 |
| `SEG-F(N)` | Nth repetition of field | `PID-13(2)` - Second phone number |

**Important:** Segment indexing in VynaHL7 is **1-based** (matching HAPI and HL7 conventions):
- `OBX(1)-5` or just `OBX-5` = First OBX segment, field 5
- `OBX(2)-5` = Second OBX segment, field 5

## Basic Terser Usage

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7 = r"MSH|^~\&|LAB|HOSP|EMR|CLINIC|20240315||ORU^R01|MSG001|P|2.5
PID|1||123456||DOE^JOHN^MICHAEL^JR^DR||19800515|M|||123 Main St^Apt 4^Boston^MA^02101||555-1234~555-5678
OBX|1|NM|WBC^White Blood Count||7.5|10*3/uL|4.0-11.0|N|||F
OBX|2|NM|RBC^Red Blood Count||4.8|10*6/uL|4.5-5.9|L|||F";

    let message = parse_message(hl7)?;
    let terser = Terser::new(&message);

    // Access patient demographics
    println!("Patient: {} {}",
        terser.get("PID-5-2")?.unwrap_or(""),  // Given name
        terser.get("PID-5-1")?.unwrap_or("")   // Family name
    );
    println!("DOB: {}", terser.get("PID-7")?.unwrap_or("N/A"));
    println!("Gender: {}", terser.get("PID-8")?.unwrap_or("N/A"));

    // Access address components
    println!("\nAddress:");
    println!("  Street: {}", terser.get("PID-11-1")?.unwrap_or("N/A"));
    println!("  City: {}", terser.get("PID-11-3")?.unwrap_or("N/A"));
    println!("  State: {}", terser.get("PID-11-4")?.unwrap_or("N/A"));
    println!("  ZIP: {}", terser.get("PID-11-5")?.unwrap_or("N/A"));

    // Access multiple OBX segments
    println!("\nLab Results:");
    println!("  WBC: {} {}",
        terser.get("OBX-5")?.unwrap_or("N/A"),     // First OBX
        terser.get("OBX-6")?.unwrap_or("")
    );
    println!("  RBC: {} {}",
        terser.get("OBX(2)-5")?.unwrap_or("N/A"),  // Second OBX
        terser.get("OBX(2)-6")?.unwrap_or("")
    );

    Ok(())
}
```

Output:
```
Patient: JOHN DOE
DOB: 19800515
Gender: M

Address:
  Street: 123 Main St
  City: Boston
  State: MA
  ZIP: 02101

Lab Results:
  WBC: 7.5 10*3/uL
  RBC: 4.8 10*6/uL
```

## Modifying Messages with TerserMut

To modify messages, use `TerserMut`:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::TerserMut;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7 = r"MSH|^~\&|APP|FAC|RECV|DEST|20240315||ADT^A01|MSG001|P|2.5
PID|1||123456||DOE^JOHN||19800515|M";

    let mut message = parse_message(hl7)?;
    let mut terser = TerserMut::new(&mut message);

    // Update patient information
    terser.set("PID-5-1", "SMITH")?;     // Change family name
    terser.set("PID-5-2", "JANE")?;       // Change given name
    terser.set("PID-8", "F")?;            // Change gender

    println!("Modified message:");
    println!("{}", message.encode().replace('\r', "\n"));

    Ok(())
}
```

## Handling Repetitions

HL7 fields can have multiple values (repetitions). Access them with `(N)` notation:

```rust
// PID-13 contains multiple phone numbers
let primary_phone = terser.get("PID-13")?;       // First phone
let work_phone = terser.get("PID-13(2)")?;       // Second phone (1-based repetition)

// Iterate over all repetitions
for phone in terser.iter_repetitions("PID", 13, 0) {
    println!("Phone: {}", phone);
}
```

## CachedTerser for Performance

When accessing the same fields repeatedly, `CachedTerser` provides 5-10x better performance by caching parsed paths and values:

```rust
use vyna_hl7_terser::CachedTerser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = parse_message(hl7)?;

    // Regular Terser - parses path on every access
    let terser = Terser::new(&message);

    // CachedTerser - caches parsed paths and values
    let mut cached = CachedTerser::new(&message);

    // Or with pre-allocated capacity
    let mut cached = CachedTerser::with_capacity(&message, 20);

    // Usage is identical
    let name = cached.get("PID-5-1")?;

    // Pre-warm the cache for known fields
    let fields = vec!["PID-5-1", "PID-5-2", "PID-7", "PID-8"];
    cached.warm_cache(&fields)?;

    // Check cache statistics
    println!("Cache entries: {}", cached.cache_size());

    Ok(())
}
```

### Performance Comparison

```rust
use std::time::Instant;

// Benchmark regular vs cached
let iterations = 1000;
let fields = vec!["PID-5-1", "PID-5-2", "PID-7", "PID-8", "PV1-2", "PV1-3"];

// Regular Terser
let terser = Terser::new(&message);
let start = Instant::now();
for _ in 0..iterations {
    for field in &fields {
        let _ = terser.get(field)?;
    }
}
let regular_time = start.elapsed();

// Cached Terser
let mut cached = CachedTerser::new(&message);
let start = Instant::now();
for _ in 0..iterations {
    for field in &fields {
        let _ = cached.get(field)?;
    }
}
let cached_time = start.elapsed();

println!("Regular: {:?}", regular_time);
println!("Cached: {:?} ({:.1}x faster)",
    cached_time,
    regular_time.as_nanos() as f64 / cached_time.as_nanos() as f64
);
```

Typical output:
```
Regular: 15.2ms
Cached: 2.1ms (7.2x faster)
```

## BulkTerser for Multiple Fields

When you need to extract many fields at once, `BulkTerser` is more efficient:

```rust
use vyna_hl7_terser::BulkTerser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = parse_message(hl7)?;
    let bulk = BulkTerser::new(&message);

    // Extract multiple fields in one call
    let paths = vec!["PID-5-1", "PID-5-2", "PID-7", "PID-8", "PID-11"];
    let results = bulk.get_multiple(&paths)?;

    // Results is a HashMap<&str, Option<&str>>
    println!("Family: {}", results.get("PID-5-1").and_then(|v| *v).unwrap_or("N/A"));
    println!("Given: {}", results.get("PID-5-2").and_then(|v| *v).unwrap_or("N/A"));

    Ok(())
}
```

### Pattern Matching with Wildcards

BulkTerser supports wildcard patterns for extracting data from multiple segments:

```rust
let bulk = BulkTerser::new(&message);

// Get field 5 from ALL OBX segments
let all_values = bulk.get_pattern("OBX(*)-5")?;
for (path, value) in all_values {
    println!("{}: {}", path, value);
}
// Output:
// OBX(1)-5: 7.5
// OBX(2)-5: 4.8
// OBX(3)-5: 14.2

// Get specific field from all segments of a type
let test_ids = bulk.get_all_from_segments("OBX", 3)?;
for id in test_ids {
    println!("Test: {}", id);
}
```

## Field Iteration

Terser provides iterators for processing repeating segments efficiently:

```rust
let terser = Terser::new(&message);

// Iterate over field values from all OBX segments
println!("All observation values:");
for value in terser.iter_field("OBX", 5) {
    println!("  - {}", value);
}

// Use standard iterator methods
let numeric_count = terser.iter_field("OBX", 5)
    .filter(|v| v.parse::<f64>().is_ok())
    .count();

let high_values: Vec<f64> = terser.iter_field("OBX", 5)
    .filter_map(|v| v.parse::<f64>().ok())
    .filter(|&v| v > 100.0)
    .collect();

// Iterate over specific components
for code in terser.iter_component("OBX", 3, 1) {
    println!("Test code: {}", code);
}

// Iterate over repetitions within a single field
for phone in terser.iter_repetitions("PID", 13, 0) {
    println!("Phone: {}", phone);
}
```

## TerserQuery for Complex Queries

`TerserQuery` provides advanced querying capabilities:

```rust
use vyna_hl7_terser::TerserQuery;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let message = parse_message(hl7)?;
    let query = TerserQuery::new(&message);

    // Find first segment matching a field value
    if let Some(glucose_seg) = query.find_first("OBX", 3, "GLUCOSE") {
        let value = glucose_seg.get_field(5).and_then(|f| f.value());
        let units = glucose_seg.get_field(6).and_then(|f| f.value());
        println!("Glucose: {:?} {:?}", value, units);
    }

    // Check if any results are abnormal
    let has_abnormal = query.any_match("OBX", |seg| {
        seg.get_field(8)
            .and_then(|f| f.value())
            .map(|v| v != "N")
            .unwrap_or(false)
    });
    println!("Has abnormal results: {}", has_abnormal);

    // Check if all results are final
    let all_final = query.all_match("OBX", |seg| {
        seg.get_field(11)
            .and_then(|f| f.value())
            .map(|v| v == "F")
            .unwrap_or(false)
    });
    println!("All results final: {}", all_final);

    // Filter segments by field value
    let numeric_obs = query.filter_repeating("OBX", 2, "NM");
    println!("Numeric observations: {}", numeric_obs.len());

    // Filter by component value
    let wbc_tests = query.filter_by_component("OBX", 3, 1, "WBC");
    for seg in wbc_tests {
        println!("Found WBC segment");
    }

    // Count segments matching a predicate
    let abnormal_count = query.count_where("OBX", |seg| {
        seg.get_field(8)
            .and_then(|f| f.value())
            .map(|v| v == "A" || v == "H" || v == "L")
            .unwrap_or(false)
    });
    println!("Abnormal results: {}", abnormal_count);

    Ok(())
}
```

### Conditional Field Access

```rust
// Only get a field if a condition is met
let name = query.get_if("PID-5", |terser| {
    // Only return name if patient has an active visit
    terser.get("PV1-19").ok().flatten().is_some()
});

if let Some(name) = name {
    println!("Patient with active visit: {}", name);
}
```

## Real-World Example: Lab Result Processor

Here's a complete example showing Terser in action for processing lab results:

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::{Terser, CachedTerser, TerserQuery};

struct LabResult {
    test_code: String,
    test_name: String,
    value: String,
    units: String,
    reference_range: String,
    abnormal_flag: Option<String>,
    status: String,
}

fn process_lab_results(hl7: &str) -> Result<Vec<LabResult>, Box<dyn std::error::Error>> {
    let message = parse_message(hl7)?;
    let mut terser = CachedTerser::new(&message);

    // Get patient info
    let patient_name = format!("{} {}",
        terser.get("PID-5-2")?.unwrap_or(""),
        terser.get("PID-5-1")?.unwrap_or("")
    );
    let mrn = terser.get("PID-3-1")?.unwrap_or("Unknown");
    println!("Processing results for: {} (MRN: {})", patient_name, mrn);

    // Count OBX segments
    let obx_count = message.get_segments_by_id("OBX").len();
    let mut results = Vec::with_capacity(obx_count);

    for i in 1..=obx_count {
        let prefix = if i == 1 {
            "OBX".to_string()
        } else {
            format!("OBX({})", i)
        };

        let result = LabResult {
            test_code: terser.get(&format!("{}-3-1", prefix))?.unwrap_or("").to_string(),
            test_name: terser.get(&format!("{}-3-2", prefix))?.unwrap_or("").to_string(),
            value: terser.get(&format!("{}-5", prefix))?.unwrap_or("").to_string(),
            units: terser.get(&format!("{}-6", prefix))?.unwrap_or("").to_string(),
            reference_range: terser.get(&format!("{}-7", prefix))?.unwrap_or("").to_string(),
            abnormal_flag: terser.get(&format!("{}-8", prefix))?.map(|s| s.to_string()),
            status: terser.get(&format!("{}-11", prefix))?.unwrap_or("").to_string(),
        };

        results.push(result);
    }

    Ok(results)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hl7 = r"MSH|^~\&|LAB|HOSPITAL|EMR|CLINIC|20240315||ORU^R01|MSG001|P|2.5
PID|1||PAT12345||SMITH^JOHN^A||19800515|M
OBX|1|NM|GLU^Glucose||98|mg/dL|70-100|N|||F
OBX|2|NM|HBA1C^Hemoglobin A1c||5.4|%|<5.7|N|||F
OBX|3|NM|CHOL^Cholesterol||245|mg/dL|<200|H|||F";

    let results = process_lab_results(hl7)?;

    println!("\nLab Results:");
    println!("{:<10} {:<20} {:>10} {:<10} {:<12} {}",
        "Code", "Test", "Value", "Units", "Range", "Flag");
    println!("{}", "-".repeat(70));

    for r in results {
        let flag = r.abnormal_flag.unwrap_or_else(|| "".to_string());
        let flag_indicator = match flag.as_str() {
            "H" => " [HIGH]",
            "L" => " [LOW]",
            "A" => " [ABNORMAL]",
            _ => "",
        };

        println!("{:<10} {:<20} {:>10} {:<10} {:<12} {}",
            r.test_code, r.test_name, r.value, r.units, r.reference_range, flag_indicator);
    }

    Ok(())
}
```

Output:
```
Processing results for: JOHN SMITH (MRN: PAT12345)

Lab Results:
Code       Test                      Value Units      Range        Flag
----------------------------------------------------------------------
GLU        Glucose                      98 mg/dL     70-100
HBA1C      Hemoglobin A1c              5.4 %         <5.7
CHOL       Cholesterol                 245 mg/dL     <200          [HIGH]
```

## Best Practices

1. **Use CachedTerser for repeated access** to the same fields across multiple messages or within tight loops.

2. **Pre-warm the cache** when you know which fields you'll need:
   ```rust
   let fields = vec!["PID-5-1", "PID-5-2", "PID-7", "PV1-2", "PV1-19"];
   cached_terser.warm_cache(&fields)?;
   ```

3. **Use BulkTerser for extracting many fields** in a single operation.

4. **Use TerserQuery for complex filtering** on repeating segments.

5. **Handle None values gracefully** - HL7 fields are often optional:
   ```rust
   let value = terser.get("PID-19")?.unwrap_or("N/A");
   ```

6. **Remember indexing conventions:**
   - Segment occurrences: 1-based (`OBX(2)` = second OBX)
   - Field numbers: match HL7 spec (1-based)
   - Component numbers: 1-based (`PID-5-1` = first component)

## Quick Reference

```rust
// Basic access
let terser = Terser::new(&message);
let value = terser.get("PID-5-1")?;

// Modification
let mut terser = TerserMut::new(&mut message);
terser.set("PID-5-1", "SMITH")?;

// Cached access
let mut cached = CachedTerser::new(&message);
cached.warm_cache(&["PID-5", "PID-7", "PV1-2"])?;

// Bulk extraction
let bulk = BulkTerser::new(&message);
let values = bulk.get_multiple(&["PID-5-1", "PID-5-2"])?;
let pattern = bulk.get_pattern("OBX(*)-5")?;

// Iteration
for value in terser.iter_field("OBX", 5) { }
for code in terser.iter_component("OBX", 3, 1) { }

// Queries
let query = TerserQuery::new(&message);
let segment = query.find_first("OBX", 3, "GLUCOSE");
let matches = query.filter_repeating("OBX", 2, "NM");
```

---

*Next in series: [Message Validation: Ensuring HL7 Compliance with VynaHL7](./06-message-validation.md)*

*Previous: [Understanding HL7 v2 Messages](./04-understanding-hl7-v2-messages.md)*
