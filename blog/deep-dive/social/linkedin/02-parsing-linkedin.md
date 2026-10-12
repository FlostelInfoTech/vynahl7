<!-- Internal: Part 2 - Parsing HL7 at 24,000+ msg/sec -->
<!-- Copy everything below this line into LinkedIn -->

Parsing HL7 at 24,000+ Messages Per Second (Single-Threaded)

How fast can you parse the full spectrum of HL7 v2 messages -- ADT, ORU, pharmacy, scheduling, documents, billing -- in a single library?

We benchmarked VynaHL7 across 7 production message types on a single thread (in-memory parsing, no network I/O). The results:

SIU^S12 (scheduling): 21 us -- 46,700 msg/sec
ORM^O01 (orders): 22 us -- 45,700 msg/sec
MDM^T02 (documents): 26 us -- 37,900 msg/sec
RDE^O11 (pharmacy): 29 us -- 35,100 msg/sec
DFT^P03 (financial): 34 us -- 29,600 msg/sec
ADT^A01 (admission): 41 us -- 24,200 msg/sec
ORU^R01 (lab results): 58 us -- 17,300 msg/sec

The surprising finding: message size doesn't predict parse time. A 2,627-byte MDM document parses faster than a 959-byte ADT admission. Why? Structural complexity -- delimiter density and nesting depth -- dominates, not byte count.

Built in Rust. No garbage collector. No GC pauses. Deterministic p99 latency.

Every number is reproducible: cargo bench -p vyna-hl7-parser --bench message_types_bench

Full technical deep dive with methodology, architecture analysis, and practical implications:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-2-of-11

#HL7 #Healthcare #Rust #Interoperability #HealthIT #Performance
