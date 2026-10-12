# Memory Under the Microscope: Allocation Profiling an HL7 Parser

*Part 3 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

When VynaHL7 parses an HL7 message, how many heap allocations does it make, how many bytes does it request, and how does that relate to the size of the input? The ratio of bytes allocated to bytes of input — the *amplification factor* — is a key indicator of memory efficiency that directly impacts throughput under sustained load.

## How We Measured

Rust's `GlobalAlloc` trait lets you replace the system allocator with a custom one — a capability that is effectively impossible in garbage-collected languages like Java or C#, where the runtime owns the heap. We wrote a `CountingAllocator` that wraps the standard `System` allocator and counts every `alloc()` call:

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct CountingAllocator {
    alloc_count: AtomicU64,
    alloc_bytes: AtomicU64,
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_count.fetch_add(1, Ordering::Relaxed);
        self.alloc_bytes.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}
```

For each message type, we warm up with 10 parses (to stabilize allocator internal state), then measure 1,000 iterations and report per-parse averages.

> **Why Rust?** The `GlobalAlloc` trait is a first-class language feature. You declare `#[global_allocator] static ALLOC: CountingAllocator = ...;` and *every* heap allocation in the entire process goes through your code. No bytecode instrumentation, no sampling profiler, no VM flags — just a trait implementation. This level of allocator control is unique to systems languages.

## Results

| Message Type | Size | Allocs/Parse | Bytes/Parse | Amplification |
|---|---|---|---|---|
| ADT^A01 Full | 959B | 832 | 57,603 | 60.1x |
| ORU^R01 Comprehensive | 1,808B | 1,410 | 88,946 | 49.2x |
| RDE^O11 Pharmacy | 813B | 727 | 46,402 | 57.1x |
| SIU^S12 Scheduling | 729B | 547 | 33,673 | 46.2x |
| MDM^T02 Document | 2,627B | 588 | 38,297 | **14.6x** |
| DFT^P03 Financial | 1,064B | 860 | 56,724 | 53.3x |
| ORM^O01 Order | 702B | 570 | 35,468 | 50.5x |
| Escape Heavy | 493B | 319 | 18,193 | 36.9x |
| Production Messy | 342B | **571** | **41,716** | **122.0x** |
| Batch (5 msgs) | 1,146B | 1,045 | 63,775 | 55.7x |
| Batch (50 msgs) | 11,002B | 9,826 | 609,435 | 55.4x |

**Amplification = bytes allocated / bytes of input.** Lower is better. 1.0x would mean zero overhead (theoretical minimum).

## Under the Hood

**The MDM outlier (14.6x).** The MDM^T02 document message is 2,627 bytes — the largest input — but has the *lowest* amplification. Why? Most of its bytes are long OBX text blocks that become single `String` allocations. Fewer structural splits means fewer `Field`, `Component`, and `Subcomponent` objects. The allocator does less work per byte.

**The Production Messy outlier (122.0x).** This 342-byte message with trailing delimiters, empty components, and extra whitespace has the *highest* amplification. The parser creates `Field` and `Component` structs for every delimiter position, even when the value is empty. A 342-byte input triggers 571 allocations totaling 41KB. This is the cost of preserving the full HL7 structure faithfully — every empty component is a real structural element.

**Allocation count scales with structural complexity, not byte count.** The ADT^A01 (959 bytes) triggers 832 allocations, while the MDM^T02 (2,627 bytes — nearly 3x larger) triggers only 588. This directly correlates with the parse time observations from Part 1: structural complexity, not message size, drives both allocation count and parse time.

**Batch processing scales linearly.** The 50-message batch (9,826 allocs) is almost exactly 50x the single-message overhead, confirming there's no per-batch amortization — each message in a batch is parsed independently.

## Practical Implications

- **For throughput optimization:** If your workload is MDM-heavy (document notifications), the parser is extremely efficient — 14.6x amplification with only 588 allocations per parse. For ADT-heavy workloads, expect 60x amplification with 832 allocations.
- **For custom allocators:** Knowing the allocation count (547-1,410 per parse) makes arena allocators viable. Pre-allocating a 90KB arena would cover even the largest message type without falling back to the system allocator.
- **For memory budgeting:** At sustained single-threaded throughput of 20,000 msg/sec (in-memory parsing) on a typical ADT mix, expect ~1.1 GB/sec of allocation churn. Rust's allocator handles this without GC pauses, but it's useful to know for capacity planning.

## Reproduce It Yourself

```bash
cargo bench -p vyna-hl7-parser --bench alloc_profile_bench
```

This runs a standalone binary (not Criterion) that prints the allocation table directly.

## Next in Series

[Part 4: Escape Sequences, Batch Parsing, and Edge Cases](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-4-of-11) — How does parser performance degrade when messages contain heavy escape sequences, batch envelopes, or production-messy formatting?
