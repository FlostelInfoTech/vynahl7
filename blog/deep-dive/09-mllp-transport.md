# MLLP Transport: TCP Performance for Healthcare

*Part 9 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

In-memory parsing benchmarks tell you how fast the library is. But healthcare systems communicate over networks — typically MLLP (Minimal Lower Layer Protocol) over TCP for intra-organization messaging. How fast can VynaHL7 send, receive, parse, and acknowledge HL7 messages over a real TCP connection?

## How We Measured

This benchmark uses VynaHL7's actual `MllpServer` and `MllpClient` implementations — the same code you'd deploy in production. Both client and server run on localhost with auto-allocated ephemeral ports.

**The full round-trip measured:**
1. Client: encode message → MLLP frame (VT + payload + FS + CR) → TCP send
2. Server: TCP receive → deframe → parse HL7 → generate ACK → frame → TCP send
3. Client: TCP receive → deframe → parse ACK → return

- **Messages:** ADT^A01 (315B), ORU^R01 (746B), RDE^O11 (518B)
- **Concurrency:** 1, 4, 8 simultaneous persistent connections
- **Warmup:** 1,000 messages discarded
- **Iterations:** 10,000 (single), 20,000 (4 clients), 40,000 (8 clients)
- **Sustained:** 15-second continuous send, single client
- **Timing:** `std::time::Instant` per round-trip, sorted for percentile analysis

## Results

### Single Client Performance

| Message Type | Size | Throughput | p50 | p90 | p99 | max |
|-------------|------|-----------|------|------|------|------|
| ADT^A01 | 315B | 6,000 msg/s | 0.137 ms | 0.264 ms | 0.386 ms | 0.546 ms |
| ORU^R01 | 746B | 4,758 msg/s | 0.172 ms | 0.345 ms | 0.482 ms | 0.771 ms |
| RDE^O11 | 518B | 6,595 msg/s | 0.126 ms | 0.246 ms | 0.388 ms | 2.180 ms |

Single-client throughput ranges from 4,758 to 6,595 msg/s depending on message size and complexity. The p50 latency (median) is 126-172 µs — the time from calling `send_message()` to receiving the parsed ACK.

### Concurrent Client Scaling

| Clients | Throughput | p50 | p99 | Scaling Factor |
|---------|-----------|------|------|---------------|
| 1 | 6,000 msg/s | 0.137 ms | 0.386 ms | 1.0x |
| 4 | 34,688 msg/s | 0.106 ms | 0.252 ms | 5.8x |
| 8 | 49,922 msg/s | 0.141 ms | 0.405 ms | 8.3x |

**Super-linear scaling from 1 to 4 clients** (5.8x throughput with 4 clients). This happens because the single-client test is limited by round-trip latency — the client waits for the ACK before sending the next message. With 4 concurrent clients, the server's Tokio event loop keeps the CPU busy while individual clients wait for responses.

**Near-linear scaling from 4 to 8** (8.3x total). The p50 latency stays flat at ~0.1-0.14 ms, while p99 increases modestly from 0.252 to 0.405 ms. The server handles contention gracefully.

> **Why Rust?** VynaHL7's MLLP server is built on Tokio — Rust's async runtime. Each TCP connection is a lightweight Tokio task (not an OS thread), and the event loop uses `epoll` on Linux for zero-cost connection multiplexing. The server processes 49,922 msg/s with 8 clients on a 6-core laptop CPU. The key optimization: TCP_NODELAY disables Nagle's algorithm for immediate packet transmission, and chunk-based 4KB buffer reads replace byte-by-byte I/O. These are possible because Rust gives you direct control over socket options and buffer sizes — no framework abstraction layer hiding the syscalls.

### Sustained Throughput (15-Second Continuous)

| Metric | Value |
|--------|-------|
| Messages processed | 108,347 |
| Throughput | 7,223 msg/s |
| Projected daily capacity | **624 million messages/day** |

The sustained throughput (7,223 msg/s) is higher than the single-client benchmark (6,000 msg/s) because the 15-second test amortizes connection setup and warmup costs.

## Under the Hood

**MLLP framing is trivial overhead.** The MLLP frame adds exactly 3 bytes per message (VT start byte, FS end byte, CR terminator). At 315 bytes per ADT message, that's less than 1% overhead. The real cost is TCP round-trip latency.

**Parse + ACK dominates the server side.** The server-side processing for each message is: deframe (~0) + parse (~14 µs for ADT) + generate ACK (~2 µs) + encode ACK (~1 µs) + frame + send. The ~17 µs server-side processing, combined with ~120 µs TCP round-trip on localhost, accounts for the ~137 µs p50 latency.

**Tail latency is well-controlled.** The p99/p50 ratio for single-client ADT is 2.8x (0.386/0.137). For 8 clients, it's 2.9x (0.405/0.141). Tail latency doesn't blow up under concurrency — a sign that there are no lock contention hotspots or GC pauses in the event loop.

## Practical Implications

- **For interface engine sizing:** A single VynaHL7 MLLP endpoint on commodity hardware handles 6,000-7,000 msg/s with a single client. With 8 concurrent clients, it reaches ~50,000 msg/s. The server itself is async (Tokio-based), not single-threaded. Most hospital interfaces process 100-1,000 msg/s — VynaHL7 has 6-50x headroom.
- **For latency-sensitive workflows:** The 137 µs p50 latency includes the full network round-trip. For real-time ADT notifications or critical lab results, this is well within the "immediate" perception threshold.
- **For capacity planning:** 624 million messages/day sustained throughput means a single VynaHL7 instance can handle the daily message volume of a large hospital system with significant margin.

## Reproduce It Yourself

```bash
cargo bench --bench transport_bench --features testing
```

## Next in Series

[Part 10: HTTP Transport and Protocol Comparison](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-10-of-11) — How does VynaHL7's HTTP transport compare to MLLP, and when should you use which?
