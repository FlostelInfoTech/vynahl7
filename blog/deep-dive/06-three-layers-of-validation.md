# Three Layers of HL7 Validation: Schema, DataType, Vocabulary

*Part 6 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

A message that parses successfully isn't necessarily correct. Is MSH-9 a valid message type? Is the date in PID-7 formatted properly? Is the sex code in PID-8 from the right HL7 table? VynaHL7 provides three validation layers — schema, data type, and vocabulary — each catching different classes of errors. What does each layer cost?

## How We Measured

- **Schema loading:** `Validator::for_message_type(Version::V2_5, "ADT", "A01")` — loads and parses the JSON schema for a specific message type
- **Schema validation:** `validator.validate(&message)` on a pre-parsed message with a pre-loaded schema
- **Data type validation:** `validate_data_type(value, DataType::DT)` for individual field values
- **Vocabulary validation:** `TableRegistry::validate("0001", "M")` for code lookups against HL7 tables
- **Full pipeline:** `parse_message()` + `validator.validate()` combined

Framework: Criterion 0.7, 100 samples. Hardware: AMD Ryzen 5 4600U, Rust 1.94.0.

## Results

### Schema Loading (One-Time Cost)

| Message Type | Load Time |
|--------------|-----------|
| ADT_A01 (17 required/optional segments) | 53.5 µs |
| ORU_R01 (complex group structure) | 46.8 µs |
| ORM_O01 (order groups) | 46.1 µs |

Schema loading is a one-time cost — you load the validator once and reuse it for all messages of that type. At ~50 µs, it's equivalent to parsing two messages.

### Schema Validation (Per-Message Cost)

| Message Type | Validate Time |
|--------------|---------------|
| ADT_A01 | 6.40 µs |
| ORU_R01 | 5.89 µs |
| ORM_O01 | 4.56 µs |

Validation itself is fast — 4.5 to 6.4 µs per message. This is cheaper than the parse step, meaning adding validation to your pipeline adds roughly 30-70% to the total cost but doesn't dominate it.

### Data Type Validation

| Data Type | Value | Time |
|-----------|-------|------|
| SI (Sequence ID) | "1" | 9.6 ns |
| ST (String) | "Normal string value" | 9.9 ns |
| CWE (Coded with Exceptions) | "WBC^White Blood Cell^LN" | 9.9 ns |
| XPN (Person Name) | "DOE^JOHN^ALLEN^JR^DR^PHD" | 9.8 ns |
| NM (Numeric) | "7.5" | 14.3 ns |
| DT (Date) | "20240315" | 17.0 ns |
| TM (Time) | "143000" | 39.4 ns |
| DTM (DateTime) | "20240315143000" | 44.3 ns |

Data type validation operates in the **nanosecond** range — 10 to 44 ns per field. The simple types (ST, SI, CWE, XPN) essentially just check for structural validity and return in under 10 ns. Date/time types (DT, TM, DTM) are slower because they validate digit positions, ranges, and format compliance.

> **Why Rust?** The `DataType` enum with exhaustive `match` ensures that every data type has a validation path — the compiler refuses to compile if you add a new variant without handling it. The 10 ns validation times are possible because there's no boxing, no virtual dispatch, and no runtime type checking. Each `validate_data_type()` call compiles to a direct function call with the value as a string slice — no heap allocation, no dynamic dispatch.

### Vocabulary Validation

| Lookup | Time |
|--------|------|
| 0001 (Sex) - valid "M" | 42.0 ns |
| 0001 (Sex) - valid "F" | 41.2 ns |
| 0004 (Patient Class) - valid "I" | 40.7 ns |
| 0085 (Observation Status) - valid "F" | 41.0 ns |
| 0103 (Processing Type) - valid "T" | 40.4 ns |
| 0001 (Sex) - **invalid "Z"** | **402.7 ns** |

Valid code lookups are ~40 ns — a HashMap lookup. Invalid codes take ~400 ns (10x longer) because the error path constructs an error message string describing which codes *are* valid. In production, most codes are valid, so the fast path dominates.

### Full Validation Pipeline

| Pipeline | ADT_A01 (315B) | ORU_R01 (746B) | ORM_O01 |
|----------|---------------|---------------|---------|
| Parse only | 13.6 µs | 36.7 µs | — |
| Parse + validate | 22.6 µs | 27.5 µs | 18.6 µs |
| Validation overhead | +66% | — | — |

*Note: These benchmarks use smaller message variants (315B ADT, 746B ORU) than the 959B and 1,808B messages in Part 2. The smaller messages isolate validation cost more clearly.*

Adding schema validation to the parse pipeline costs roughly 5-10 µs additional per message — a 30-70% overhead depending on message complexity.

## Practical Implications

- **Load validators at startup, not per-message.** The 50 µs schema loading cost is negligible if done once, catastrophic if done per message.
- **Data type validation is essentially free.** At 10-44 ns per field, validating every field in a 20-field message adds ~500 ns — invisible in a 40 µs parse pipeline.
- **Vocabulary validation is fast for valid codes.** The 40 ns per lookup means checking all coded fields in a message adds ~1 µs total.
- **The 400 ns invalid-code path is still fast** — it only matters if you're processing millions of messages with consistently invalid codes, which is itself a data quality problem.

## Reproduce It Yourself

```bash
cargo bench -p vyna-hl7-validator --bench validator_bench
```

## Next in Series

[Part 7: Encoding, Decoding, and Building Messages](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-7-of-11) — What does it cost to encode escape sequences and construct messages programmatically using the builder pattern?
