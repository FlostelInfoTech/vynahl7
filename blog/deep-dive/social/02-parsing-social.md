# Social Post: Parsing HL7 at 24,000+ msg/sec (Single-Threaded)

**LinkedIn / Facebook / WeChat**

---

**Parsing HL7 at 24,000+ Messages Per Second (Single-Threaded)**

How fast can you parse the full spectrum of HL7 v2 messages — ADT, ORU, pharmacy, scheduling, documents, billing — in a single library?

We benchmarked VynaHL7 across 7 production message types on a single thread (in-memory parsing, no network I/O). The results:

| Message Type | Parse Time | Messages/sec |
|-------------|-----------|-------------|
| SIU^S12 (scheduling) | 21 µs | 46,700 |
| ORM^O01 (orders) | 22 µs | 45,700 |
| MDM^T02 (documents) | 26 µs | 37,900 |
| RDE^O11 (pharmacy) | 29 µs | 35,100 |
| DFT^P03 (financial) | 34 µs | 29,600 |
| ADT^A01 (admission) | 41 µs | 24,200 |
| ORU^R01 (lab results) | 58 µs | 17,300 |

The surprising finding: message size doesn't predict parse time. A 2,627-byte MDM document parses faster than a 959-byte ADT admission. Why? Structural complexity — delimiter density and nesting depth — dominates, not byte count.

Built in Rust. No garbage collector. No GC pauses. Deterministic p99 latency.

Every number is reproducible: `cargo bench -p vyna-hl7-parser --bench message_types_bench`

Full technical deep dive with methodology, architecture analysis, and practical implications:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-2-of-11

#HL7 #Healthcare #Rust #Interoperability #HealthIT #Performance
