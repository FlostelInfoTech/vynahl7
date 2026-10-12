<!-- Internal: Part 6 - Three Layers of Validation -->
<!-- Copy everything below this line into LinkedIn -->

Three Layers of HL7 Validation: Schema, DataType, Vocabulary

A message that parses successfully isn't necessarily correct.

VynaHL7 validates HL7 messages at three layers. Here's what each costs (single-threaded):

Schema validation (structure against HL7 standard): 4.5-6.4 us per message
Data type validation (date format, numeric range): 10-44 nanoseconds per field
Vocabulary validation (code against HL7 tables): 40 nanoseconds per lookup

Simple types like SI and ST validate in under 10 ns. Even DateTime (the most expensive) is only 44 ns.

The full parse + validate pipeline for an ADT^A01 (315B): 22.6 us. Validation adds ~66% to the 13.6 us parse cost -- but you go from "structurally parseable" to "clinically correct."

Rust's DataType enum with exhaustive match means the compiler refuses to compile if you add a new data type without writing its validation logic. No forgotten cases. No runtime type-checking overhead. Each validation compiles to a direct function call -- no boxing, no reflection.

Load your validators at startup (50 us one-time cost), reuse per message. Don't reload schemas per message -- that's 50 us wasted every time.

Full benchmark methodology and results:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-6-of-11

#HL7 #Healthcare #DataQuality #Rust #Validation
