# Escape Sequences, Batch Parsing, and Edge Cases

*Part 4 of 11 | VynaHL7 Technical Deep Dive*

---

## The Question

Production HL7 messages are rarely clean. They contain escape sequences (`\T\`, `\S\`, `\.br\`), batch/file envelopes wrapping dozens of messages, trailing delimiters, empty components, and whitespace where it shouldn't be. How does VynaHL7's parser handle the messy reality of healthcare data, and what's the performance cost?

## How We Measured

We designed four benchmark groups targeting specific edge cases:

- **Escape density:** Synthetic messages where 0%, 5%, 20%, and 50% of field values contain escape sequences
- **Batch/file parsing:** `parse_batch()` on BHS/BTS envelopes with 1 to 50 messages; `parse_file()` on FHS/FTS envelopes
- **Lenient vs strict:** `ParserConfig::strict()` vs `ParserConfig::lenient()` on a production-messy message
- **Production patterns:** Real-world edge cases — trailing delimiters, escape-heavy messages, large narrative OBX blocks

All measurements use Criterion 0.7 with 100 samples. Hardware: AMD Ryzen 5 4600U, Rust 1.94.0, release profile.

## Results

### Escape Sequence Density

| Escape Density | Parse Time | Throughput | Cost vs 0% |
|---|---|---|---|
| 0% (no escapes) | 7.45 µs | 44.7 MiB/s | baseline |
| 5% | 8.25 µs | 41.5 MiB/s | +10.7% |
| 20% | 8.61 µs | 43.1 MiB/s | +15.5% |
| 50% | 9.25 µs | 46.3 MiB/s | +24.2% |

Escape handling adds measurable cost, but it scales sub-linearly. Going from 0% to 50% escaped values only increases parse time by 24%. The throughput in MiB/s actually *increases* at 50% because the escaped values are longer strings (more bytes per field).

### Batch and File Parsing

| Input | Parse Time | Per-Message Cost | Throughput |
|---|---|---|---|
| 1 message (batch) | 10.1 µs | 10.1 µs | 26.6 MiB/s |
| 5 messages (batch) | 54.8 µs | 11.0 µs | 19.9 MiB/s |
| 10 messages (batch) | 79.0 µs | 7.9 µs | 27.0 MiB/s |
| 50 messages (batch) | 549.5 µs | 11.0 µs | 19.1 MiB/s |
| 2x10 messages (file) | 208.9 µs | 10.4 µs | 17.9 MiB/s |

The BHS/BTS and FHS/FTS envelope overhead is minimal. Per-message cost within a batch is roughly constant at ~10-11 µs, confirming that batch parsing scales linearly.

### Lenient vs Strict Mode

| Mode | Parse Time | Throughput |
|---|---|---|
| Strict | 25.9 µs | 12.6 MiB/s |
| Lenient | 10.9 µs | 30.0 MiB/s |

Strict mode is **2.4x slower** on the messy production message. This is because strict parsing performs additional validation checks at each delimiter — verifying no trailing delimiters, no non-standard characters, no empty segment IDs. On clean messages, the difference is minimal; on messy messages, it's substantial.

> **Why Rust?** The strict vs lenient mode is implemented using Rust's `ParserConfig` struct with boolean flags that control code paths at each delimiter. Because Rust compiles to native code without a JIT warmup period, the branching cost is predictable — you see the 2.4x difference on the first message, not after a warmup phase. The `Result<T, E>` type forces every parse error to be explicitly handled, making it impossible to silently ignore a validation failure in strict mode.

### Production Patterns

| Pattern | Parse Time | Throughput |
|---|---|---|
| Trailing delimiters (lenient) | 10.8 µs | 30.2 MiB/s |
| Escape-heavy message | 13.1 µs | 36.0 MiB/s |
| Large narrative OBX (2.6KB) | 26.9 µs | 93.2 MiB/s |

The large narrative OBX result (93.2 MiB/s) confirms the pattern from Parts 2 and 3: bulk text content with few delimiters parses at very high throughput.

## Practical Implications

- **For production deployments:** Use `ParserConfig::lenient()` unless you have a specific reason to reject non-conformant messages. The 2.4x throughput improvement is significant, and most production HL7 feeds contain minor formatting quirks.
- **For escape-heavy feeds:** Clinical narratives (MDM, OBX with TX data) often contain escape sequences. The 24% overhead at 50% density is manageable — and in practice, most messages are well below 50%.
- **For batch interfaces:** If your trading partner sends batch files, `parse_batch()` and `parse_file()` add negligible overhead per message. The linear scaling means you can predict processing time from message count.

## Reproduce It Yourself

```bash
cargo bench -p vyna-hl7-parser --bench edge_cases_bench
```

## Next in Series

[Part 5: The Terser and CachedTerser: Field Access at Scale](https://www.flostel.com/posts/vyna-hl7-technical-deep-dive-part-5-of-11) — After parsing, you need to extract fields. We compare VynaHL7's two field access strategies and benchmark real clinical extraction patterns.
