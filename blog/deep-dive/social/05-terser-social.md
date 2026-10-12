# Social Post: Terser and CachedTerser

**LinkedIn / Facebook / WeChat**

---

**Terser and CachedTerser: HL7 Field Access at Scale**

After parsing an HL7 message, you need to extract fields. Patient ID. Lab values. Order status.

VynaHL7 offers two approaches: Terser (direct tree walk) and CachedTerser (HashMap-cached lookups).

CachedTerser is 1.67x faster for repeated access — 4.72 µs vs 7.89 µs for 10 paths accessed 5 times each.

Real clinical workflow timings:
- Patient demographics (6 fields): 1.58 µs
- Lab results across 8 OBX segments (32 fields): 13.4 µs
- Order information (5 fields): 1.31 µs

The Rust advantage: Terser<'a> holds a borrowed reference to the Message with a compile-time lifetime guarantee. Field access returns &str slices pointing directly into the parsed data — zero copying. The borrow checker proves at compile time that your extracted references can't outlive the message. No defensive cloning needed.

At 263 ns per field extraction on a single thread, you can pull every field from 74,000 lab messages per second on a laptop — field extraction only, excluding parse time.

Full benchmark data with scaling analysis:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-5-of-11

#HL7 #Rust #Healthcare #Performance #Interoperability
