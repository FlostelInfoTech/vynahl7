# From HL7 v2 to FHIR R4: Converting at Microsecond Speed

*Part 8 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

FHIR R4 is the modern interoperability standard, but most hospital systems still speak HL7 v2. Bridging the two means converting parsed HL7 segments into FHIR resources and serializing them to JSON. How fast can VynaHL7's converters do this, and what's the cost breakdown between conversion and serialization?

## How We Measured

VynaHL7 includes 12 FHIR R4 converters, each mapping a specific HL7 v2 segment to a FHIR resource. We benchmark three things:

- **Individual converters:** Each converter running on a pre-parsed message (no parse cost included)
- **JSON serialization:** `serde_json::to_string()` on converted FHIR resources
- **Full pipeline:** Parse HL7 + convert all applicable resources + serialize everything to JSON

Test messages: ORU^R01 (4 OBX observations, OBR, PV1, PID) and ADT^A01 (PID, PV1, AL1, DG1, PR1).

## Results

### Individual Converter Performance

| Converter | HL7 Segment | FHIR Resource | Time |
|-----------|-------------|---------------|------|
| ConditionConverter | DG1 | Condition | 627 ns |
| AllergyIntoleranceConverter | AL1 | AllergyIntolerance | 869 ns |
| ProcedureConverter | PR1 | Procedure | 1.26 µs |
| PractitionerConverter | PV1 (attending) | Practitioner | 1.94 µs |
| ObservationConverter (single) | OBX | Observation | 2.31 µs |
| EncounterConverter | PV1 | Encounter | 2.41 µs |
| DiagnosticReportConverter | OBR | DiagnosticReport | 2.63 µs |
| PatientConverter | PID | Patient | 4.29 µs |
| MedicationConverter | RXA | MedicationAdministration | 1.52 µs |
| ImmunizationConverter | RXA | Immunization | 1.38 µs |
| ServiceRequestConverter | ORC | ServiceRequest | 1.71 µs |
| SpecimenConverter | SPM | Specimen | 1.15 µs |
| ObservationConverter (all 4) | 4x OBX | 4x Observation | 13.1 µs |

Conversion times range from 627 ns (Condition) to 4.29 µs (Patient). The Patient converter is the most expensive because PID is the most complex segment — it extracts identifiers, names, addresses, phone numbers, and coded values, each requiring component-level parsing and FHIR resource construction.

### JSON Serialization

| Resource | Serialization Time |
|----------|-------------------|
| Patient | 650 ns |
| Encounter | 948 ns |
| 4 Observations | 2.65 µs |

Serialization is fast — Rust's `serde` framework generates specialized serialization code at compile time, not through reflection. The Patient resource serializes in 650 ns despite containing nested arrays (identifiers, names, addresses).

> **Why Rust?** Serde's `#[derive(Serialize)]` generates a custom serialization implementation for each FHIR resource struct at compile time. There's no reflection, no runtime type inspection, no boxing of field values. Each field write is a direct method call. The 650 ns Patient serialization includes writing ~15 fields with nested objects — that's ~43 ns per field, approaching the cost of raw string concatenation. In Java, Jackson serialization of an equivalent POJO would typically take 5-20 µs due to reflection overhead and intermediate buffer management.

### Full Pipeline: Parse + Convert + Serialize

| Pipeline | Time | Breakdown |
|----------|------|-----------|
| ORU → Patient + 4 Observations + Encounter + DiagnosticReport + Practitioner + JSON | 62.8 µs | Parse ~37 µs + Convert ~23 µs + Serialize ~3 µs |
| ADT → Patient + Encounter + AllergyIntolerance + Condition + Procedure + JSON | 29.2 µs | Parse ~14 µs + Convert ~10 µs + Serialize ~5 µs |

The full pipeline — from raw HL7 string to JSON-serialized FHIR resources — takes 29-63 µs. That's **16,000-34,000 complete conversions per second** on a laptop.

## Under the Hood

**Conversion cost correlates with the number of fields extracted.** The ConditionConverter pulls 3-4 fields from DG1 (627 ns). The PatientConverter pulls 15+ fields with component-level access from PID (4.29 µs). Each field extraction is a Terser path lookup — the same ~260 ns per field we measured in Part 5.

**Serialization is a small fraction of total cost.** In the ORU pipeline, serialization (3 µs) is only 5% of the total (63 µs). The dominant costs are parsing (59%) and conversion (37%). Optimizing serialization would yield minimal improvement.

**The ObservationConverter scales linearly.** Converting 4 OBX segments (13.1 µs) is almost exactly 4x the single OBX cost (2.31 µs × 4 = 9.24 µs) plus overhead for iterating the segment list. For a comprehensive metabolic panel with 20+ OBX segments, expect ~50 µs.

## Practical Implications

- **For FHIR facade APIs:** At 16,000+ conversions/sec, a single VynaHL7 instance can serve a FHIR API backed by HL7 v2 feeds without becoming a bottleneck.
- **For batch conversion:** Converting a day's worth of lab results (say, 100,000 ORU messages) takes ~6.3 seconds of CPU time.
- **For selective conversion:** If you only need Patient resources (not all 12 types), the 4.3 µs PatientConverter alone handles 230,000 conversions/sec.
- **Parse is the bottleneck, not conversion.** If you're optimizing a FHIR conversion pipeline, focus on the parse step (59% of time), not the converters.

## Reproduce It Yourself

```bash
cargo bench -p vyna-hl7-fhir --bench fhir_bench
```

## Next in Series

[Part 9: MLLP Transport — TCP Performance for Healthcare](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-9-of-11) — We move from in-memory benchmarks to real network I/O, measuring VynaHL7's MLLP server and client over TCP.
