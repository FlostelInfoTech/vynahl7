# Social Post: The Full Pipeline

**LinkedIn / Facebook / WeChat**

---

**The Full Pipeline: From Wire to FHIR**

We published 10 articles benchmarking every layer of VynaHL7 — from 10 ns data type validation to 137 µs MLLP network round-trips.

Here's the complete performance picture for an HL7 integration library written in Rust:

All in-memory numbers are single-threaded. Network numbers specify client count.

Parse ADT^A01: 41 µs (24,200/sec, single-threaded)
Validate against HL7 schema: +6 µs
Extract 10 clinical fields: +7 µs
Encode back to HL7 string: 5 µs (47 MiB/s)
Convert to FHIR Patient: 4.3 µs
Serialize to JSON: 650 ns
MLLP network round-trip: 137 µs p50 (single client)
MLLP sustained: 7,200 msg/s single client = 624M messages/day

Interface engine simulation (parse + route + ACK): 19.7 µs per message single-threaded, in-memory. That's 50,000 msg/sec on a single core — network I/O not included.

Every number is backed by a 4-section benchmark report (purpose, methodology, configuration, results). Every number is reproducible with a single command.

The consistent finding: Rust's ownership model, zero-cost abstractions, and compile-time LTO across crate boundaries make performance a non-issue for healthcare integration work. The bottleneck will be your network, your database, or your business logic — not your HL7 library.

VynaHL7 is open source. Run the benchmarks yourself:
```
git clone https://github.com/flostelinfotech/vyna-hl7.git
cd vyna-hl7 && bash bench-results/run_all.sh
```

Full 10-part series:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-11-of-11

#HL7 #FHIR #Healthcare #Rust #OpenSource #Performance #HealthIT #Interoperability
