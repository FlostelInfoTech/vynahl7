<!-- Internal: Part 7 - Encoding and Builders -->
<!-- Copy everything below this line into LinkedIn -->

Encoding and Builders: Creating HL7 Messages in Rust

Parsing gets the attention. But production HL7 systems also CREATE messages -- ACK responses, transformed outputs, new orders.

VynaHL7 encoding and builder benchmarks:

Encoding a parsed Message back to HL7 string (single-threaded): 5.3 us (ADT), 15.2 us (ORU). That's 47 MiB/s -- 3-4x faster than parsing, because the structure is already known.

Escape sequence handling: encoding plain text costs 177 ns. Heavy escapes (40% special chars) cost 339 ns. Decoding plain text? 27 ns -- it just scans for backslashes and finds none.

Building messages from scratch using the builder pattern: 9.7 us for an ADT^A01 with 13 fields. Comparable to parsing a similarly sized message (13.6 us for a 315B ADT), and faster than parsing a full-size ADT (41 us for 959B with many more fields).

The Rust builder pattern uses ownership transfer (self, not &mut self). The compiler optimizes the method chain into in-place mutation -- no intermediate copies.

Practical takeaway: building > parsing when creating new messages. Use builders for ACKs and new orders. Use parse+modify+encode for transformations.

Full technical analysis:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-7-of-11

#HL7 #Rust #Healthcare #Performance #BuilderPattern
