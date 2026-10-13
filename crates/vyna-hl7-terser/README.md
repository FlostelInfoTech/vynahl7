# vyna-hl7-terser

Terser API for easy HL7 field access using path notation. The Terser provides a convenient, HAPI-inspired interface for navigating HL7 messages without manual segment and field traversal.

## Features

- **Path-based field access** - Use simple paths like `PID-5-1` or `OBX(2)-5`
- **Cached access** - CachedTerser provides faster repeated access by caching parsed paths
- **Bulk extraction** - Extract multiple fields at once with BulkTerser
- **Pattern matching** - Use wildcards to extract data from repeating segments
- **Iterator API** - Iterate over fields, components, and repetitions
- **Conditional queries** - Find and filter segments based on field values
- **Type-safe** - All operations use Rust's type system for safety

## Quick Start

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;

let hl7 = r"MSH|^~\&|APP|FAC|||20250120||ADT^A01|123|P|2.5
PID|1||PAT001||DOE^JOHN^A^^^DR||19800515|M";

let message = parse_message(hl7)?;
let terser = Terser::new(&message);

// Get patient name components
let family_name = terser.get("PID-5-1")?; // "DOE"
let given_name = terser.get("PID-5-2")?;  // "JOHN"
let prefix = terser.get("PID-5-6")?;      // "DR"
```

## Path Notation

### Basic Syntax

Terser paths follow HL7 v2 conventions with 1-based indexing:

- `SEG-F` - Segment, field
- `SEG-F-C` - Segment, field, component
- `SEG-F-C-S` - Segment, field, component, subcomponent
- `SEG(I)-F` - Specific segment instance, 1-based (`OBX(1)` is the first OBX)
- `SEG-F(R)-C` - Field repetition, 1-based (`PID-5(1)` is the first repetition)

### Examples

```rust
terser.get("PID-5")?;        // Patient name (first component)
terser.get("PID-5-1")?;      // Family name
terser.get("PID-5-2")?;      // Given name
terser.get("OBX(2)-5")?;     // Second OBX segment, observation value
terser.get("PID-11(1)-1")?;  // First address repetition, street
```

## Core Terser API

### Reading Values

```rust
use vyna_hl7_terser::Terser;

let terser = Terser::new(&message);

// Get single field
if let Some(value) = terser.get("PID-7")? {
    println!("Date of birth: {}", value);
}

// Navigate complex fields
let city = terser.get("PID-11-3")?;     // Address - city
let state = terser.get("PID-11-4")?;    // Address - state
let zip = terser.get("PID-11-5")?;      // Address - zip
```

### Writing Values

```rust
use vyna_hl7_terser::TerserMut;

let mut message = parse_message(hl7)?;
let mut terser = TerserMut::new(&mut message);

// Set patient name
terser.set("PID-5-1", "SMITH")?;   // Family name
terser.set("PID-5-2", "JANE")?;    // Given name
terser.set("PID-5-3", "M")?;       // Middle name

// Set complex fields
terser.set("PID-11-1", "123 Main St")?;
terser.set("PID-11-3", "Boston")?;
terser.set("PID-11-4", "MA")?;
```

## Cached Terser (Performance)

CachedTerser speeds up repeated access to the same fields by caching parsed paths.

```rust
use vyna_hl7_terser::CachedTerser;

let mut terser = CachedTerser::new(&message);

// These lookups are cached and very fast on repeated access
for _ in 0..1000 {
    terser.get("PID-5-1")?;
    terser.get("PID-7")?;
    terser.get("OBX(1)-5")?;
}
```

**When to use CachedTerser:**
- Processing batches of messages with the same field access patterns
- Repeatedly accessing the same fields within a single message
- Performance-critical code paths

**Benchmark results** (from `cargo bench -p vyna-hl7-terser --bench cached_terser_bench`):
- Regular Terser: ~80 ns per lookup
- Cached Terser: ~60 ns per cached lookup (~1.4x faster)

## Bulk Field Extraction

BulkTerser enables efficient extraction of multiple fields at once.

### Extract Multiple Fields

```rust
use vyna_hl7_terser::BulkTerser;
use std::collections::HashMap;

let bulk = BulkTerser::new(&message);

// Extract patient demographics in one call
let paths = vec!["PID-5-1", "PID-5-2", "PID-7", "PID-8"];
let results: HashMap<String, Option<&str>> = bulk.get_multiple(&paths)?;

let last_name = results.get("PID-5-1").and_then(|v| *v);
let first_name = results.get("PID-5-2").and_then(|v| *v);
```

### Pattern Matching with Wildcards

Use `(*)` wildcards to extract data from all instances of repeating segments:

```rust
// Get all observation values
let values = bulk.get_pattern("OBX(*)-5")?;
// Returns: vec![("OBX(1)-5", "7.5"), ("OBX(2)-5", "98"), ...]

// Get all observation identifiers (field 3, component 1)
let test_codes = bulk.get_pattern("OBX(*)-3-1")?;

// Convenience method for common pattern
let values = bulk.get_all_from_segments("OBX", 5)?;
// Returns: vec!["7.5", "98", "14.2", ...]
```

## Iterator API

Iterate over field values from repeating segments using standard Rust iterators.

### Field Iterator

```rust
let terser = Terser::new(&message);

// Iterate over all observation values
for value in terser.iter_field("OBX", 5) {
    println!("Lab value: {}", value);
}

// Use standard iterator methods
let count = terser.iter_field("OBX", 5).count();

let high_values: Vec<_> = terser.iter_field("OBX", 5)
    .filter_map(|v| v.parse::<f64>().ok())
    .filter(|&v| v > 100.0)
    .collect();
```

### Component Iterator

```rust
// Get all observation codes (field 3, component 1)
for code in terser.iter_component("OBX", 3, 1) {
    println!("Test code: {}", code);
}

// Get all observation names (field 3, component 2)
let names: Vec<_> = terser.iter_component("OBX", 3, 2).collect();
```

### Repetition Iterator

```rust
// Iterate over field repetitions (e.g., multiple phone numbers).
// The third argument is a 0-based segment position (0 = first PID),
// unlike the 1-based indices in path notation.
for phone in terser.iter_repetitions("PID", 13, 0) {
    println!("Phone: {}", phone);
}
```

**Note:** All iterators automatically skip empty values.

## Conditional Queries

TerserQuery provides powerful filtering and searching capabilities.

```rust
use vyna_hl7_terser::TerserQuery;

let query = TerserQuery::new(&message);
```

### Finding Segments

```rust
// Find first segment matching criteria
if let Some(glucose_obs) = query.find_first("OBX", 3, "GLUCOSE") {
    // Access the segment directly
    if let Some(value) = glucose_obs.get_field(5).and_then(|f| f.value()) {
        println!("Glucose: {}", value);
    }
}

// Filter all segments by field value
let numeric_obs = query.filter_repeating("OBX", 2, "NM");
println!("Found {} numeric observations", numeric_obs.len());

// Filter by component value
let wbc_tests = query.filter_by_component("OBX", 3, 1, "WBC");
```

### Conditional Predicates

```rust
// Check if any segment matches
let has_abnormal = query.any_match("OBX", |seg| {
    seg.get_field(8)
        .and_then(|f| f.value())
        .map(|v| v != "N")
        .unwrap_or(false)
});

// Check if all segments match
let all_final = query.all_match("OBX", |seg| {
    seg.get_field(11)
        .and_then(|f| f.value())
        .map(|v| v == "F")
        .unwrap_or(false)
});

// Count matching segments
let abnormal_count = query.count_where("OBX", |seg| {
    seg.get_field(8)
        .and_then(|f| f.value())
        .map(|v| v == "A" || v == "H" || v == "L")
        .unwrap_or(false)
});
```

### Complex Queries

```rust
// Get field values where another field matches
let test_names = query.get_values_where(
    "OBX",     // Segment ID
    2,         // Filter field (observation type)
    "NM",      // Filter value
    3          // Result field (test name)
);

// Conditional field access
let value = query.get_if("PID-5", |terser| {
    // Only get patient name if visit number exists
    terser.get("PV1-19").ok().flatten().is_some()
});
```

## Examples

Every API above ships runnable doctests; run them all with:

```bash
cargo test -p vyna-hl7-terser --doc
```

The `benches/` directory holds Criterion benchmarks comparing `Terser`
and `CachedTerser` on realistic clinical messages:

```bash
cargo bench -p vyna-hl7-terser --bench cached_terser_bench
```

## Performance Considerations

| API | Use Case | Performance |
|-----|----------|-------------|
| `Terser` | Simple field access, one-off lookups | ~80 ns/lookup |
| `CachedTerser` | Repeated access, batch processing | ~60 ns/lookup (cached, ~1.4x faster) |
| `BulkTerser` | Multiple fields at once | Efficient for bulk operations |
| Iterators | Traversing repeating segments | Zero-allocation iteration |
| `TerserQuery` | Filtering and searching | Optimized for complex queries |

**Best Practices:**
- Use `CachedTerser` when accessing the same paths repeatedly
- Use `BulkTerser` when you need multiple fields from a message
- Use iterators for processing repeating segments (OBX, NK1, etc.)
- Use `TerserQuery` for complex filtering and conditional logic
- Regular `Terser` is fine for simple, infrequent access

## Path Indexing Reference

Every index in path notation is 1-based (HL7 standard):

- Segments: `OBX(1)` = first OBX, `OBX(2)` = second OBX
- Field repetitions: `PID-5(1)` = first repetition, `PID-5(2)` = second
- Components: `PID-5-1` = first component, `PID-5-2` = second component
- Subcomponents: `PID-5-1-1` = first subcomponent

A 0 index in a path is invalid: `OBX(0)` and `PID-5(0)` are errors, and
`PID-5-0` never matches. Internally the crate stores these as 0-based
`Vec` positions and converts automatically.

One exception: `iter_repetitions(segment_id, field, segment_index)` takes
a 0-based `segment_index` argument (a direct position, not path notation),
so `iter_repetitions("PID", 11, 0)` reads the first PID segment.

## Error Handling

All Terser operations return `Result<T, Error>` for safe error handling:

```rust
use vyna_hl7_core::Result;

fn process_message(message: &Message) -> Result<()> {
    let terser = Terser::new(message);

    // Handle potential errors
    match terser.get("PID-5-1")? {
        Some(name) => println!("Patient: {}", name),
        None => println!("Name not present"),
    }

    Ok(())
}
```

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
