# Encoding, Decoding, and Building Messages in Rust

*Part 7 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

Parsing is only half the story. Production HL7 systems also *create* messages — ACK responses, transformed outputs, new orders. How fast can VynaHL7 encode a `Message` struct back to an HL7 string, handle escape sequences, and construct messages using the builder pattern?

## How We Measured

Three benchmark groups:

- **Encode isolated:** `message.encode()` on pre-parsed messages (separates encoding cost from parsing)
- **Escape encode/decode:** `Encoding::encode()` and `Encoding::decode()` with varying escape density
- **Builder construction:** `AdtA01Builder` and `OruR01Builder` with full field population and `.build()`

Framework: Criterion 0.7. Hardware: AMD Ryzen 5 4600U, Rust 1.94.0.

## Results

### Isolated Encoding

| Message Type | Size | Encode Time | Throughput |
|--------------|------|-------------|------------|
| ADT^A01 | 959B | 5.33 µs | 47.4 MiB/s |
| ORU^R01 | 1,808B | 15.2 µs | 44.5 MiB/s |

Encoding is roughly 3-4x faster than parsing. This makes sense: encoding walks a known tree structure and concatenates strings with delimiters, while parsing must detect delimiters in an unknown input. The throughput is consistent at ~45 MiB/s regardless of message type.

### Escape Sequence Encode/Decode

| Operation | Input Type | Time |
|-----------|------------|------|
| Encode plain text | No special chars | 177 ns |
| Decode plain text | No escape sequences | 27 ns |
| Encode light (~10% special chars) | `O'Brien\|Smith & Jones` | 201 ns |
| Decode light escapes | Encoded version | 206 ns |
| Encode heavy (~40% special chars) | Multiple `\S\`, `\T\`, `\F\` | 339 ns |
| Decode heavy escapes | Multiple escape sequences | 284 ns |

Two key observations:

**Decoding plain text is 6.5x faster than encoding it** (27 ns vs 177 ns). Decoding scans for `\` characters and short-circuits when there are none. Encoding must check every character against the delimiter set, even when no escaping is needed.

**Heavy escapes double the encode cost** (339 ns vs 177 ns) but only increase decode cost by 10x from the plain baseline (284 ns vs 27 ns). The decode baseline is so fast that even a 10x increase is under 300 ns.

> **Why Rust?** The escape encoding operates on `&str` slices with character-by-character iteration. Rust's `String::with_capacity()` pre-allocates the output buffer based on input length, avoiding reallocation in most cases. The `match` on each character compiles to a jump table — there's no regex engine, no string scanning library, just direct character comparison at native speed. The 27 ns plain-text decode is essentially the cost of scanning 54 characters for a backslash and finding none.

### Builder Construction

| Builder | Time |
|---------|------|
| AdtA01Builder (13 fields, PID/PV1 segments) | 9.68 µs |
| OruR01Builder (8 fields, 2 OBX observations) | 7.92 µs |

Builders construct a `Message` from scratch — allocating segments, fields, components, and populating values. The cost (9.7 µs) is comparable to parsing a similarly sized message (~13.6 µs for a 315B ADT), which makes sense: both create the same `Message` struct tree, just from different inputs (API calls vs raw HL7 text). Builders are faster than parsing a full-size ADT (41 µs for 959B) because they populate fewer fields.

## Under the Hood

**Encoding throughput is consistent because the work is proportional to the output.** Every field value is written once, every delimiter is written once. The ADT encodes at 47.4 MiB/s and the ORU at 44.5 MiB/s — a 6% difference despite being structurally very different messages.

**Builder cost = allocation cost.** The builder pattern uses Rust's ownership transfer (`self` not `&mut self`) for method chaining:

```rust
let message = AdtA01Builder::new(Version::V2_5)
    .sending_application("HIS")
    .sending_facility("HOSPITAL")
    .patient_id("MRN12345")
    .patient_name("DOE", "JOHN")
    .build()?;
```

Each method consumes `self` and returns a new `Self`, which the compiler optimizes into in-place mutation. The final `.build()` assembles the `Message` struct. The 9.68 µs cost is almost entirely heap allocation for `Segment`, `Field`, and `String` objects.

## Practical Implications

- **ACK generation is cheap.** A minimal ACK (MSH + MSA) is ~2 segments, so encoding costs roughly 1-2 µs. Combined with the ~20 µs parse of the incoming message, the full receive-parse-ACK cycle is under 25 µs.
- **Escape handling is not a bottleneck.** Even at 40% escape density, encode/decode adds under 700 ns total per field — negligible in a message with 20-50 fields.
- **Builders vs parse-modify-encode:** Building a message from scratch (9.7 µs) is faster than parsing an existing one (41 µs) and modifying it. Use builders when creating new messages; use parse + TerserMut + encode when transforming existing ones.

## Reproduce It Yourself

```bash
cargo bench -p vyna-hl7-core --bench core_bench
```

## Next in Series

[Part 8: From HL7 v2 to FHIR R4](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-8-of-11) — VynaHL7 includes 12 FHIR R4 converters. We benchmark each one and measure the full parse-to-FHIR-to-JSON pipeline.
