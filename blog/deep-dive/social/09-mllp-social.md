# Social Post: MLLP Transport Performance

**LinkedIn / Facebook / WeChat**

---

**MLLP Transport: TCP Performance for Healthcare Messaging**

In-memory benchmarks are nice. But healthcare systems communicate over NETWORKS.

We benchmarked VynaHL7's MLLP server and client over real TCP — the same code you'd deploy in production. Full round-trip: client sends → server parses → server generates ACK → client receives.

Single client: 6,000 msg/s at 137 µs median latency.
4 concurrent clients: 34,700 msg/s (5.8x scaling).
8 concurrent clients: 49,900 msg/s (8.3x scaling — super-linear).

Sustained single-client throughput over 15 seconds: 7,223 msg/s. That's 624 million messages per day on a laptop.

Why super-linear scaling? The single-client test is latency-bound — waiting for each ACK. With 4 clients, Tokio's event loop stays saturated. Each connection is a lightweight async task, not an OS thread. epoll-based multiplexing with zero-cost connection management.

TCP_NODELAY kills Nagle's algorithm. 4KB chunk reads replace byte-by-byte I/O. These optimizations are possible because Rust gives you direct socket-level control — no framework hiding the syscalls.

Most hospital interfaces: 100-1,000 msg/s. VynaHL7 has 6-50x headroom.

Full latency percentile analysis and architecture details:
https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-9-of-11

#HL7 #MLLP #Healthcare #Rust #Tokio #Performance
