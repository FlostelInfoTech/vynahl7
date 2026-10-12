<!-- Internal: Part 10 - HTTP Transport and Protocol Comparison -->
<!-- Copy everything below this line into LinkedIn -->

HTTP vs MLLP: Which Transport for Healthcare Integration?

MLLP for intra-hospital. HTTP for inter-organization. VynaHL7 supports both. How do they compare?

Head-to-head on the same message, same hardware:

Throughput (1 client): MLLP 6,000 msg/s vs HTTP 3,320 msg/s -- MLLP 1.8x faster
p50 latency: MLLP 0.137 ms vs HTTP 0.275 ms -- MLLP 2.0x lower
8-client throughput: MLLP 49,900 msg/s vs HTTP 24,100 msg/s -- MLLP 2.1x faster
Daily sustained: MLLP 624M msgs vs HTTP 300M msgs -- MLLP 2.1x more

MLLP is ~2x faster across the board. The gap is consistent -- it doesn't widen or narrow with concurrency.

Where does the HTTP overhead come from? Not the Rust framework. Axum wraps Hyper wraps Tokio -- two abstraction layers that compile away thanks to zero-cost abstractions. The 2x difference is protocol overhead: HTTP headers, status lines, content negotiation. MLLP adds 3 bytes of framing. HTTP adds ~200 bytes.

But here's the thing: even HTTP at 3,320 msg/s with a single client over loopback TCP exceeds what most healthcare organizations need. Choose based on network requirements, not performance.

Use MLLP internally. HTTP at organizational boundaries. Both support TLS/mTLS.

Full comparison with scaling data:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-10-of-11

#HL7 #MLLP #HTTP #Healthcare #Rust #Interoperability
