<!-- Internal: Part 8 - HL7 v2 to FHIR R4 Conversion -->
<!-- Copy everything below this line into LinkedIn -->

From HL7 v2 to FHIR R4: Converting at Microsecond Speed

Most hospital systems speak HL7 v2. Modern APIs want FHIR R4. Bridging them is the daily work of healthcare integration.

VynaHL7 includes 12 FHIR R4 converters. How fast are they?

Condition (DG1 to FHIR): 627 ns
AllergyIntolerance (AL1 to FHIR): 869 ns
Practitioner (PV1 to FHIR): 1.94 us
Patient (PID to FHIR): 4.29 us
4 Observations (OBX to FHIR): 13.1 us

Full pipeline -- raw HL7 string to JSON-serialized FHIR resources (single-threaded):
ORU to all resources + JSON: 63 us (16,000 conversions/sec)
ADT to all resources + JSON: 29 us (34,000 conversions/sec)

The JSON serialization itself? 650 ns for a Patient resource. Rust's serde generates specialized serialization code at compile time -- no reflection, no runtime type inspection. Each field write is a direct method call.

Converting a day's worth of lab results (100,000 ORU messages) takes 6.3 seconds of single-threaded CPU time.

Full deep dive with pipeline breakdown:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-8-of-11

#FHIR #HL7 #Healthcare #Interoperability #Rust #HealthIT
