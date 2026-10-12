# The Terser and CachedTerser: Field Access at Scale

*Part 5 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

After parsing an HL7 message into a `Message` struct, you need to extract specific fields — patient ID, lab results, order status. VynaHL7 provides two APIs for this: `Terser` (direct tree traversal) and `CachedTerser` (HashMap-cached lookups). How do they compare, and what does field access cost in real clinical workflows?

## How We Measured

- **Terser:** `Terser::new(&message).get("PID-5-1")` — parses the path string and walks the segment/field/component tree on each call
- **CachedTerser:** `CachedTerser::new(&message).get("PID-5-1")` — same on first access, then returns cached result on subsequent calls
- **Test message:** An ORU^R01 with 8 OBX segments (realistic lab results message)
- **Comparison:** Same 10 paths accessed 5 times each (50 lookups total)
- **Clinical workflows:** Demographics (6 paths), lab results (32 paths across 8 OBX), order info (5 paths)
- **Scaling:** 10, 50, 100 unique paths with a second pass to test cache hits

## Results

### CachedTerser vs Terser

| API | 10 paths x 5 accesses | Speedup |
|-----|----------------------|---------|
| Terser | 7.89 µs | baseline |
| CachedTerser | 4.72 µs | **1.67x faster** |
| CachedTerser (pre-warmed) | 4.93 µs | 1.60x faster |

The CachedTerser is 1.67x faster for repeated access patterns. The pre-warmed variant (calling `warm_cache()` upfront) is slightly slower overall because the warmup cost amortizes only when paths are accessed many more times.

### Clinical Workflow Extraction

| Workflow | Paths | Time | Per-Path |
|----------|-------|------|----------|
| Patient demographics | 6 | 1.58 µs | 263 ns |
| Lab results (8 OBX segments) | 32 | 13.4 µs | 419 ns |
| Order information | 5 | 1.31 µs | 263 ns |

Demographics and order extraction are fast — under 2 µs for a complete extraction. Lab results take longer because each `OBX(n)-field` path requires the parser to count through segments to find the nth OBX, which is O(n) in segment count.

> **Why Rust?** The `Terser<'a>` holds a `&'a Message` — a borrowed reference with a lifetime tied to the message. This means field access returns `&str` slices pointing directly into the parsed message data, with zero copying. The `CachedTerser` stores these borrowed references in a `HashMap<String, CacheEntry>`, and Rust's borrow checker guarantees at compile time that the cache cannot outlive the message it references. In a GC language, the equivalent pattern would keep the entire message alive as long as any cached reference exists; in Rust, the lifetime relationship is explicit and enforced.

### Scaling Behavior

| Unique Paths | First + Second Pass | Per-Path (amortized) |
|--------------|--------------------|--------------------|
| 10 | 3.10 µs | 155 ns |
| 50 | 19.8 µs | 198 ns |
| 100 | 42.6 µs | 213 ns |

Scaling is approximately linear with slight per-path overhead growth as the HashMap grows. At 100 unique paths, amortized cost is still only 213 ns per path — well under a microsecond.

## Practical Implications

- **Use CachedTerser when you access the same fields multiple times** — routing logic that checks MSH-9 and PID-3 on every message, result aggregation that pulls the same OBX paths repeatedly.
- **Use plain Terser for one-shot extraction** — the cache setup cost isn't justified if each path is accessed only once.
- **For lab result processing,** the 32-path extraction at 13.4 µs on a single thread means you can extract all results from 74,000 already-parsed lab messages per second on this hardware (field extraction only, excluding parse time).
- **Budget 250-420 ns per field extraction** depending on path complexity (simple fields are faster, indexed segments like `OBX(8)-5` are slower).

## Reproduce It Yourself

```bash
cargo bench -p vyna-hl7-terser --bench cached_terser_bench
```

## Next in Series

[Part 6: Three Layers of HL7 Validation](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-6-of-11) — Parsing succeeds doesn't mean the message is correct. We benchmark VynaHL7's schema, datatype, and vocabulary validation layers.
