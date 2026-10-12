# Parsing HL7 at 24,000+ Messages Per Second in Rust (Single-Threaded)

*Part 2 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

How fast can you parse the full spectrum of HL7 v2 message types — admissions, lab results, pharmacy orders, scheduling, clinical documents, billing, and general orders — and what drives the performance differences between them?

Most HL7 libraries publish a single benchmark number for a single message type. That tells you very little about production behavior, where your interface engine processes a mix of ADT, ORU, ORM, RDE, and other messages with wildly different structural characteristics.

We benchmarked VynaHL7's parser across 7 representative message types with Criterion 0.7, measuring both parse-only and full round-trip (parse + encode) performance. Every number in this article is reproducible on your own hardware.

## How We Measured

- **Function under test:** `vyna_hl7_parser::parse_message()` — takes an `&str`, returns a `Message` struct
- **Round-trip:** `parse_message()` followed by `message.encode()` to reconstruct the HL7 string
- **Framework:** Criterion 0.7 with 100 samples per benchmark
- **Messages:** 7 realistic HL7 messages from our shared corpus, ranging from 702 to 2,627 bytes
- **Hardware:** AMD Ryzen 5 4600U (6 cores / 12 threads), 15 GB RAM, Linux Mint 22.3
- **Toolchain:** Rust 1.94.0, release profile with `opt-level=3`, LTO enabled, `codegen-units=1`
- **Reproducibility:** Run-to-run variance is typically <2% across repeated executions. No CPU pinning or frequency governor locking was applied.

## Results

### Parse Performance

| Message Type | Size | Parse Time | Throughput | Messages/sec |
|---|---|---|---|---|
| SIU^S12 (scheduling) | 729B | 21.4 µs | 32.4 MiB/s | ~46,700 |
| ORM^O01 (orders) | 702B | 21.9 µs | 30.6 MiB/s | ~45,700 |
| MDM^T02 (documents) | 2,627B | 26.4 µs | 94.9 MiB/s | ~37,900 |
| RDE^O11 (pharmacy) | 813B | 28.5 µs | 27.2 MiB/s | ~35,100 |
| DFT^P03 (financial) | 1,064B | 33.8 µs | 30.0 MiB/s | ~29,600 |
| ADT^A01 (admission) | 959B | 41.4 µs | 22.1 MiB/s | ~24,200 |
| ORU^R01 (lab results) | 1,808B | 57.7 µs | 29.9 MiB/s | ~17,300 |

### Round-Trip Performance (Parse + Encode)

| Message Type | Size | Round-Trip Time | Throughput |
|---|---|---|---|
| SIU^S12 | 729B | 36.3 µs | 19.2 MiB/s |
| ORM^O01 | 702B | 36.2 µs | 18.5 MiB/s |
| RDE^O11 | 813B | 45.5 µs | 17.0 MiB/s |
| MDM^T02 | 2,627B | 48.1 µs | 52.1 MiB/s |
| ADT^A01 | 959B | 73.7 µs | 12.4 MiB/s |
| DFT^P03 | 1,064B | 54.5 µs | 18.6 MiB/s |
| ORU^R01 | 1,808B | 126.6 µs | 13.6 MiB/s |

## Under the Hood

Several observations stand out from this data:

**Message size doesn't linearly predict parse time.** The MDM^T02 document message is the largest (2,627 bytes) but parses faster than the ADT^A01 (959 bytes). Why? MDM messages contain large OBX text blocks — long contiguous strings with few delimiters. The parser scans through them quickly. ADT messages pack many short fields with deep component nesting (addresses, phone numbers, insurance records), requiring more delimiter splits and more `Field`/`Component`/`Subcomponent` struct allocations.

**Structural complexity is the dominant cost, not byte count.** The throughput in MiB/s tells this story clearly: MDM achieves 94.9 MiB/s because most of its bytes are bulk text. ADT manages only 22.1 MiB/s because nearly every byte requires structural parsing.

**The encoding pass roughly doubles the cost.** Round-trip times are 1.5-2x the parse time, which means `message.encode()` costs about the same as the initial parse. This is expected — encoding walks the entire `Message` tree to reconstruct the HL7 string with proper delimiters.

> **Why Rust?** VynaHL7's parser achieves these numbers without a garbage collector. Each `parse_message()` call allocates the exact `Message` struct tree needed, and Rust's ownership system deallocates it deterministically when the caller drops it. There are no GC pauses, no young-generation collections, no stop-the-world events — critical for maintaining consistent p99 latency under sustained load. The `&str` slicing used during delimiter detection operates on borrowed references with zero copying until the final `String` allocation for each field value.

## Practical Implications

- **For interface engine architects:** Even on a modest laptop CPU, VynaHL7 sustains 17,000-47,000 single-threaded parses/sec (in-memory, no network I/O) depending on message type. A dedicated server with higher clock speed and more cache will push well beyond this.
- **For capacity planning:** Budget parse time by message type, not a single average. An ADT-heavy feed will parse at roughly half the throughput of an ORM-heavy feed at the same message volume.
- **For round-trip workflows:** If your pipeline is parse-transform-encode, the encode pass is not free. Budget roughly 2x the parse time for the full round-trip.

> **Network context:** These are in-memory, single-threaded parsing rates. Over MLLP (TCP loopback), end-to-end throughput — including network I/O, ACK generation, and round-trip latency — is ~6,000 msg/s with a single client and ~50,000 msg/s with 8 concurrent clients (see [Part 9](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-9-of-11)). Over HTTP, single-client throughput is ~3,300 msg/s (see [Part 10](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-10-of-11)). Parse time is a fraction of the total network round-trip cost.

## Reproduce It Yourself

```bash
cargo bench -p vyna-hl7-parser --bench message_types_bench
```

Results are saved to `target/criterion/` with HTML reports. Raw data is in `bench-results/message_types_bench.txt`.

## Next in Series

[Part 3: Memory Under the Microscope](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-3-of-11) — We wrote a custom allocator to count every heap allocation during parsing. The results reveal which message types are allocation-heavy and why.
