<!-- Internal: Part 3 - Memory Allocation Profiling -->
<!-- Copy everything below this line into LinkedIn -->

Memory Under the Microscope: Allocation Profiling an HL7 Parser

We wrote a custom allocator to count every heap allocation when parsing HL7 messages -- not as part of VynaHL7 itself, but as a measurement technique in our benchmark harness.

Rust's GlobalAlloc trait lets you temporarily replace the system allocator with your own -- a single trait implementation, and every heap allocation in the process goes through your code. We used this to build a CountingAllocator that wraps the standard allocator and records every alloc() call. In Java or C#, the garbage collector owns the heap, so this level of allocation-level visibility isn't available.

The key metric: "amplification" -- bytes allocated divided by bytes of input.

MDM^T02 (documents): 2,627B input, 38KB allocated, 14.6x amplification
SIU^S12 (scheduling): 729B input, 34KB allocated, 46.2x amplification
ADT^A01 (admission): 959B input, 58KB allocated, 60.1x amplification
Production messy: 342B input, 42KB allocated, 122.0x amplification

The MDM document -- the largest message -- has the LOWEST amplification. Long text blocks = fewer structural allocations. The 342-byte "messy" message with empty fields and trailing delimiters? Highest amplification, because every delimiter position creates a struct.

Allocation count drives parse time. This data told us exactly where to optimize.

Full analysis with allocation profiling technique and implications:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-3-of-11

#Rust #Performance #HealthIT #HL7 #SystemsProgramming
