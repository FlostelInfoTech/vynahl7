# Social Post: Introducing VynaHL7

**LinkedIn / Facebook / WeChat**

---

**VynaHL7: A Rust Library for HL7 v2 Healthcare Messaging**

HL7 v2 is a 30-year-old protocol. It's also the backbone of healthcare integration — billions of messages per year between hospitals, labs, and pharmacies.

VynaHL7 is a Rust library that handles the full HL7 v2 lifecycle:

- Parse 17,000-47,000 messages/sec (single-threaded, in-memory)
- Validate against HL7 schemas (v2.3 through v2.8)
- Build messages with fluent builders
- 12 FHIR R4 converters for modern interoperability
- MLLP and HTTP transport with TLS/mTLS
- WebAssembly support for browser-based tools

No garbage collector. No reflection. Deterministic performance with zero-cost abstractions.

We're publishing an 11-part technical deep dive measuring performance across every layer — parsing, memory, validation, field access, FHIR conversion, and network transport. Every number is reproducible with a single `cargo bench` command.

Part 1 starts here:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-1-of-11

#HL7 #FHIR #Healthcare #Rust #Interoperability #HealthIT
