# Social Post: HL7 v2 to FHIR R4 Conversion

**LinkedIn / Facebook / WeChat**

---

**From HL7 v2 to FHIR R4: Converting at Microsecond Speed**

Most hospital systems speak HL7 v2. Modern APIs want FHIR R4. Bridging them is the daily work of healthcare integration.

VynaHL7 includes 12 FHIR R4 converters. How fast are they?

| Converter | Time |
|-----------|------|
| Condition (DG1 → FHIR) | 627 ns |
| AllergyIntolerance (AL1 → FHIR) | 869 ns |
| Practitioner (PV1 → FHIR) | 1.94 µs |
| Patient (PID → FHIR) | 4.29 µs |
| 4 Observations (OBX → FHIR) | 13.1 µs |

Full pipeline — raw HL7 string to JSON-serialized FHIR resources (single-threaded):
- ORU → all resources + JSON: 63 µs (16,000 conversions/sec)
- ADT → all resources + JSON: 29 µs (34,000 conversions/sec)

The JSON serialization itself? 650 ns for a Patient resource. Rust's serde generates specialized serialization code at compile time — no reflection, no runtime type inspection. Each field write is a direct method call.

Converting a day's worth of lab results (100,000 ORU messages) takes 6.3 seconds of single-threaded CPU time.

Full deep dive with pipeline breakdown:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-8-of-11

#FHIR #HL7 #Healthcare #Interoperability #Rust #HealthIT
