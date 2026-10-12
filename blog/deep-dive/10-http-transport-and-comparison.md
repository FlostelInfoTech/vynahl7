# HTTP Transport and Protocol Comparison

*Part 10 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

MLLP is the traditional HL7 transport for intra-organization messaging. But inter-organization communication — HIE networks, cloud integrations, partner connections — increasingly uses HTTP. VynaHL7 supports both. How does HTTP compare to MLLP in throughput and latency, and when should you choose one over the other?

## How We Measured

The HTTP benchmark uses VynaHL7's `HttpServer` (built on Axum/Hyper) and `HttpClient` (built on Reqwest) — the same implementations you'd deploy in production. Same test structure as the MLLP benchmarks in Part 9: same messages, same iteration counts, same hardware.

- **Protocol:** HTTP/1.1 with keep-alive (persistent connections)
- **Content-Type:** `x-application/hl7-v2+er7` (HL7-over-HTTP standard)
- **Server:** VynaHL7 HttpServer with ACK handler
- **Client:** VynaHL7 HttpClient (Reqwest-based)

## Results

### Single Client Performance

| Message Type | Size | Throughput | p50 | p90 | p99 |
|-------------|------|-----------|------|------|------|
| ADT^A01 | 315B | 3,320 msg/s | 0.275 ms | 0.442 ms | 0.551 ms |
| ORU^R01 | 746B | 2,781 msg/s | 0.320 ms | 0.552 ms | 0.684 ms |
| RDE^O11 | 518B | 3,099 msg/s | 0.296 ms | 0.478 ms | 0.569 ms |

### Concurrent Client Scaling

| Clients | Throughput | p50 | p99 | Scaling Factor |
|---------|-----------|------|------|---------------|
| 1 | 3,320 msg/s | 0.275 ms | 0.551 ms | 1.0x |
| 4 | 17,985 msg/s | 0.178 ms | 0.370 ms | 5.4x |
| 8 | 24,139 msg/s | 0.249 ms | 0.740 ms | 7.3x |

### Sustained Throughput

| Metric | Value |
|--------|-------|
| Messages processed | 52,142 |
| Throughput | 3,476 msg/s |
| Projected daily capacity | **300 million messages/day** |

## Protocol Comparison: MLLP vs HTTP

### Head-to-Head (Single Client, ADT^A01)

| Metric | MLLP | HTTP | Ratio |
|--------|------|------|-------|
| Throughput | 6,000 msg/s | 3,320 msg/s | MLLP 1.8x faster |
| p50 latency | 0.137 ms | 0.275 ms | MLLP 2.0x lower |
| p99 latency | 0.386 ms | 0.551 ms | MLLP 1.4x lower |
| Sustained | 7,223 msg/s | 3,476 msg/s | MLLP 2.1x faster |
| Daily capacity | 624M msg/day | 300M msg/day | MLLP 2.1x more |

### Concurrent Scaling Comparison

| Clients | MLLP | HTTP | MLLP Advantage |
|---------|------|------|---------------|
| 1 | 6,000 msg/s | 3,320 msg/s | 1.8x |
| 4 | 34,688 msg/s | 17,985 msg/s | 1.9x |
| 8 | 49,922 msg/s | 24,139 msg/s | 2.1x |

MLLP maintains a consistent ~2x throughput advantage across all concurrency levels. The gap doesn't widen or narrow — it's a fixed overhead difference.

## Under the Hood

**Where does the HTTP overhead come from?** The ~2x difference is explained by HTTP's protocol overhead:

1. **Request framing:** HTTP adds headers (Content-Type, Content-Length, Host, Connection) to every request. An ADT^A01 is 315 bytes of HL7 payload, but the HTTP request is ~500 bytes total. MLLP adds only 3 bytes.
2. **Response framing:** The ACK response carries HTTP status, headers, and body. MLLP returns only the framed ACK.
3. **HTTP parsing:** Hyper must parse HTTP headers on every request. MLLP's frame detection is a simple byte scan for VT/FS markers.
4. **Connection management:** HTTP keep-alive requires header negotiation. MLLP connections are just raw TCP streams.

> **Why Rust?** Both transports benefit from Rust's async ecosystem. The MLLP server uses Tokio directly for TCP socket management. The HTTP server uses Axum (which wraps Hyper, which wraps Tokio) — adding two abstraction layers. Yet the HTTP stack is still fast because Rust's zero-cost abstractions mean those layers compile away. In a language with runtime overhead, each abstraction layer would add measurable latency. In Rust, the 2x difference is almost entirely protocol overhead, not framework overhead.

**Both protocols scale super-linearly from 1 to 4 clients** because the single-client test is latency-bound (waiting for ACK), while concurrent clients keep the server's event loop saturated. HTTP scales slightly less (5.4x vs 5.8x) because HTTP request parsing consumes more CPU per message, leaving less headroom for concurrent processing.

## When to Use Which

| Scenario | Recommendation | Reason |
|----------|---------------|--------|
| Intra-hospital (EHR ↔ Lab, ADT feeds) | **MLLP** | Lower latency, higher throughput, standard for internal HL7 |
| Inter-organization (HIE, cloud) | **HTTP** | Firewall-friendly, standard ports, TLS built-in |
| High-volume feeds (>5,000 msg/s) | **MLLP** | 2x throughput advantage matters at scale |
| Mixed protocol environment | **Both** | VynaHL7 supports both; use MLLP internally, HTTP at boundaries |
| Security-first (mTLS required) | **Either** | Both VynaHL7 transports support TLS/mTLS with the `tls` feature flag |

The key insight: even VynaHL7's "slower" HTTP transport at 3,320 msg/s single-client exceeds what most healthcare organizations need. Choose based on your network requirements, not performance.

## Practical Implications

- **HTTP's 300M messages/day sustained capacity** exceeds the daily volume of most large hospital systems.
- **The 2x MLLP advantage is consistent** — don't expect HTTP to "catch up" with more concurrent clients.
- **For FHIR facade patterns** (HTTP frontend accepting FHIR, converting to HL7 v2 internally, routing via MLLP), the HTTP endpoint won't be the bottleneck.

## Reproduce It Yourself

```bash
cargo bench --bench transport_bench --features testing
```

## Next in Series

[Part 11: The Full Pipeline — From Wire to FHIR](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-11-of-11) — We tie everything together: cross-crate pipeline benchmarks, interface engine simulation, and how to reproduce every number in this series.
