# The Full Pipeline: From Wire to FHIR

*Part 11 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

We've benchmarked individual components — parsing, validation, field extraction, encoding, FHIR conversion, MLLP, HTTP. But in production, these compose into pipelines. What does the full cost look like when you chain them together? And how did we build a benchmark framework that makes every number in this series reproducible?

## How We Measured

The pipeline benchmark chains VynaHL7's cross-crate APIs into realistic workflows:

| Pipeline                    | Steps                                                           |
| --------------------------- | --------------------------------------------------------------- |
| Parse only                  | `parse_message()` — baseline                                    |
| Parse + validate            | `parse_message()` + `validator.validate()`                      |
| Parse + extract             | `parse_message()` + `Terser.get()` on 10 clinical fields        |
| Parse + transform + encode  | `parse_message()` + `TerserMut.set()` + `message.encode()`      |
| Interface engine sim        | `parse_message()` + extract routing fields + generate ACK string |

Two message types: ADT^A01 (admission, 315B) and ORU^R01 (lab results, 746B). Framework: Criterion 0.7.

## Results

### Pipeline Cost Breakdown

| Pipeline                                   | ADT^A01           | ORU^R01          |
| ------------------------------------------ | ----------------- | ---------------- |
| Parse only                                 | 13.6 µs           | 36.7 µs          |
| Parse + validate                           | 29.2 µs (+115%)   | 47.7 µs (+30%)   |
| Parse + extract 10 fields (Terser)         | 20.5 µs (+51%)    | —                |
| Parse + extract 10 fields (CachedTerser)   | 22.1 µs (+63%)    | —                |
| Parse + transform + encode                 | 28.6 µs (+110%)   | 45.5 µs (+24%)   |
| Interface engine sim                       | 19.7 µs (+45%)    | 29.2 µs (-20%*)  |

*The ORU interface engine sim is faster than parse-only ORU because the sim uses a smaller message variant optimized for the pipeline benchmark.

### What Each Step Adds

For ADT^A01 (315B pipeline variant):

```
Parse:                 13.6 µs  ████████████████
+ Validation:          +15.6 µs ██████████████████████████████
+ 10 field extraction: +6.9 µs  ████████
+ Field modification:  +1.5 µs  ██
+ Encode:              +13.5 µs ████████████████
+ ACK generation:      +6.1 µs  ████████
```

*Note: This chart uses the 315B ADT variant from the pipeline benchmark to isolate per-step costs. The summary table below uses the full 959B ADT from Part 2 with isolated per-operation measurements from each article. The numbers differ because of the different message sizes and measurement contexts.*

Parse is the largest single cost, but validation nearly matches it. Encoding (reconstructing the HL7 string) costs as much as the initial parse. Field extraction and modification are comparatively cheap.

### Implied Throughput Per Pipeline (Single-Threaded, In-Memory)

| Pipeline                      | ADT msgs/sec | ORU msgs/sec |
| ----------------------------- | ------------ | ------------ |
| Parse only                    | 73,500       | 27,200       |
| Parse + validate              | 34,200       | 21,000       |
| Parse + extract               | 48,800       | —            |
| Parse + transform + encode    | 34,900       | 22,000       |
| Interface engine sim          | 50,900       | 34,200       |

*All rates are single-threaded, in-memory (no network I/O). See Parts 9-10 for network throughput with single and multiple clients.*

> **Why Rust?** These pipeline benchmarks span four crates (vyna-hl7-parser, vyna-hl7-validator, vyna-hl7-terser, vyna-hl7-core) compiled as a single binary. Rust's LTO (Link-Time Optimization) with `codegen-units=1` enables cross-crate inlining — the compiler can inline `parse_message()` from vyna-hl7-parser into the pipeline benchmark in the root crate. In a language with separate compilation units (Java JARs, C# assemblies), cross-module inlining depends on JIT warmup and heuristics. In Rust, it's deterministic and happens at compile time. The `opt-level=3` release profile aggressively optimizes the composed pipeline, not just individual functions.

## Building a Reproducible Benchmark Framework

Every number in this 11-part series follows a consistent structure. Here's how we designed the framework to ensure reproducibility:

### The 4-Section Report

Each benchmark produces a self-documenting report:

1. **Purpose** — What specific question does this benchmark answer?
2. **Methodology** — What function is called, what inputs are used, how timing works
3. **Configuration** — Hardware (CPU, RAM, OS), toolchain (Rust version, target, profile), benchmark parameters (iterations, warmup)
4. **Results** — Clean data tables with no framework noise

### Test Corpus

A shared `corpus.rs` module provides 9 realistic HL7 messages representing the 7 most common message types in production healthcare:

| Type     | Clinical Use         | Characteristics                        |
| -------- | -------------------- | -------------------------------------- |
| ADT^A01  | Patient admission    | Deep nesting, many short fields        |
| ORU^R01  | Lab results          | Multiple OBR/OBX groups, numeric data  |
| RDE^O11  | Pharmacy orders      | RXE/RXR/RXC/TQ1 medication segments    |
| SIU^S12  | Scheduling           | SCH/AIG/AIL/AIP resource segments      |
| MDM^T02  | Clinical documents   | Long narrative OBX text blocks         |
| DFT^P03  | Billing              | FT1/DG1/PR1 financial segments         |
| ORM^O01  | General orders       | ORC/OBR order pairs with NTE notes     |

Plus two edge-case variants: escape-heavy (20% of values contain escape sequences) and production-messy (trailing delimiters, empty components).

### Environment Reporting

A `bench_env.rs` module automatically collects system information — OS, kernel, CPU model, core count, frequency, RAM, Rust version, target triple, and compiler profile — and includes it in every report. No manual note-taking.

### One Command to Run Everything

```bash
bash bench-results/run_all.sh
```

This script runs all 9 benchmark suites, wraps each with the 4-section report header, and saves results to `bench-results/`. Total runtime: ~10 minutes.

## The Complete VynaHL7 Performance Picture

Bringing together all 11 articles into one view. These numbers use full-size production messages and isolated per-operation measurements from each article:

| Operation                          | Time           | Implied Rate                      | Context                              |
| ---------------------------------- | -------------- | --------------------------------- | ------------------------------------ |
| Parse ADT^A01 (959B, full)        | 41 µs          | 24,200/sec                        | single-threaded, in-memory           |
| Parse ORU^R01 (1,808B, full)      | 58 µs          | 17,300/sec                        | single-threaded, in-memory           |
| Validate against schema (isolated) | 5-6 µs         | adds ~30-70% to parse             | single-threaded                      |
| Validate single data type          | 10-44 ns       | essentially free                  | single-threaded                      |
| Validate vocabulary code           | 40 ns          | essentially free                  | single-threaded                      |
| Extract field (Terser)             | 260 ns         | 3.8M lookups/sec                  | single-threaded, pre-parsed          |
| Encode ADT message (959B, full)    | 5.3 µs         | 47 MiB/s                          | single-threaded                      |
| Encode/decode escape (heavy)       | 340/284 ns     | per field, negligible             | single-threaded                      |
| Build ADT from scratch             | 9.7 µs         | 103,000 msgs/sec (single-threaded) | single-threaded                    |
| Convert PID → FHIR Patient        | 4.3 µs         | 233,000/sec                       | single-threaded, pre-parsed          |
| Convert OBX → FHIR Observation    | 2.3 µs         | 435,000/sec                       | single-threaded, pre-parsed          |
| Serialize Patient to JSON          | 650 ns         | 1.5M/sec                          | single-threaded                      |
| Full ORU → FHIR → JSON pipeline   | 63 µs          | 16,000/sec                        | single-threaded, in-memory           |
| MLLP round-trip                    | 137 µs (p50)   | 6,000/sec                         | single client, loopback TCP          |
| MLLP round-trip                    | 141 µs (p50)   | 50,000/sec                        | 8 concurrent clients, loopback TCP   |
| HTTP round-trip                    | 275 µs (p50)   | 3,300/sec                         | single client, loopback TCP          |
| MLLP sustained (15 sec)           | —              | 7,200/sec (624M/day)              | single client, loopback TCP          |

All numbers measured on an AMD Ryzen 5 4600U laptop (6 cores, 15 GB RAM) with Rust 1.94.0. Run-to-run variance is typically <2% across repeated benchmark executions. No CPU pinning or frequency governor locking was applied. Production servers with higher clock speeds, more cache, and dedicated resources will perform better.

## Reproduce the Entire Series

```bash
# Clone VynaHL7
git clone https://github.com/flostelinfotech/vyna-hl7.git
cd vyna-hl7

# Run all benchmarks (~10 minutes)
bash bench-results/run_all.sh

# Or run individual suites
cargo bench -p vyna-hl7-parser --bench message_types_bench
cargo bench -p vyna-hl7-parser --bench edge_cases_bench
cargo bench -p vyna-hl7-parser --bench alloc_profile_bench
cargo bench -p vyna-hl7-terser --bench cached_terser_bench
cargo bench -p vyna-hl7-validator --bench validator_bench
cargo bench -p vyna-hl7-core --bench core_bench
cargo bench -p vyna-hl7-fhir --bench fhir_bench
cargo bench --bench pipeline_bench
cargo bench --bench transport_bench --features testing
```

## Series Conclusion

Across 11 articles, we've measured VynaHL7's performance from individual function calls (10 ns data type validation) to full network round-trips (137 µs MLLP). The consistent finding: **Rust's ownership model, zero-cost abstractions, and compile-time optimization make VynaHL7 fast enough that performance is unlikely to be your bottleneck in healthcare integration work.**

The benchmark framework itself — with its 4-section reports, shared corpus, environment reporting, and single-command execution — is designed to evolve with the library. As VynaHL7 adds features, these benchmarks catch regressions and document the cost of new capabilities.

VynaHL7 is open source, and every number in this series is reproducible. We encourage you to run the benchmarks on your own hardware, compare results, and contribute improvements.

---

*This concludes the VynaHL7 Technical Deep Dive series. For getting started with VynaHL7, see the [introductory series](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-1-of-11). For the source code and documentation, visit [github.com/flostelinfotech/vyna-hl7](https://github.com/flostelinfotech/vyna-hl7).*
