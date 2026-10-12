//! VynaHL7 Memory Allocation Profile — Heap allocation tracking per parse
//!
//! Run with:
//!   cargo bench -p vyna-hl7-parser --bench alloc_profile_bench

#[allow(dead_code)]
mod alloc_counter;
#[allow(dead_code)]
mod bench_env;
#[allow(dead_code)]
mod corpus;

use alloc_counter::CountingAllocator;

#[global_allocator]
static ALLOC: CountingAllocator = CountingAllocator::new();

use corpus::*;
use vyna_hl7_parser::{parse_batch, parse_message};
use std::hint::black_box;

fn measure_parse(name: &str, input: &str, iterations: usize) {
    for _ in 0..10 {
        let _ = parse_message(input);
    }

    ALLOC.reset();
    for _ in 0..iterations {
        let msg = parse_message(black_box(input)).unwrap();
        black_box(&msg);
    }

    let allocs = ALLOC.alloc_count() as f64 / iterations as f64;
    let bytes = ALLOC.alloc_bytes() as f64 / iterations as f64;
    let amp = bytes / input.len() as f64;

    println!(
        "  {:<22} {:>7} {:>12.0} {:>11.0} {:>8.1}x",
        name,
        input.len(),
        allocs,
        bytes,
        amp
    );
}

fn measure_batch(name: &str, input: &str, iterations: usize) {
    for _ in 0..10 {
        let _ = parse_batch(input);
    }

    ALLOC.reset();
    for _ in 0..iterations {
        let batch = parse_batch(black_box(input)).unwrap();
        black_box(&batch);
    }

    let allocs = ALLOC.alloc_count() as f64 / iterations as f64;
    let bytes = ALLOC.alloc_bytes() as f64 / iterations as f64;
    let amp = bytes / input.len() as f64;

    println!(
        "  {:<22} {:>7} {:>12.0} {:>11.0} {:>8.1}x",
        name,
        input.len(),
        allocs,
        bytes,
        amp
    );
}

fn main() {
    let iterations = 1000;

    // ── 1. PURPOSE ──────────────────────────────────────────────────────
    println!();
    println!("================================================================================");
    println!("  VynaHL7 Memory Allocation Profile Report");
    println!("================================================================================");
    println!();
    println!("  1. PURPOSE");
    println!("  ----------");
    println!("  Measure heap allocation behavior of vyna_hl7_parser::parse_message() to identify");
    println!("  memory efficiency characteristics. The key metric is \"amplification\" — the");
    println!("  ratio of bytes allocated on the heap vs bytes of HL7 input. Lower amplification");
    println!("  means less allocator pressure, fewer cache misses, and better throughput under");
    println!("  sustained load. This data guides optimization of the parser's internal data");
    println!("  structures and pre-allocation strategies.");

    // ── 2. METHODOLOGY ─────────────────────────────────────────────────
    println!();
    println!("  2. METHODOLOGY");
    println!("  ---------------");
    println!("  Allocator:     Custom GlobalAlloc wrapper around std::alloc::System");
    println!("                 Counts every alloc() call and sums requested layout sizes");
    println!("                 Does NOT count reallocs or track dealloc (measures gross allocation)");
    println!("  Warmup:        10 parses discarded (stabilizes allocator internal state)");
    println!("  Iterations:    {} per message type (results averaged)", iterations);
    println!("  Measurement:   Total alloc count and alloc bytes across all iterations, divided");
    println!("                 by iteration count to get per-parse averages");
    println!("  Messages:      9 realistic HL7 messages (ADT, ORU, RDE, SIU, MDM, DFT, ORM)");
    println!("                 plus escape-heavy and production-messy variants, plus batch msgs");
    println!("  Amplification: bytes_allocated / bytes_of_input (lower = more efficient)");

    // ── 3. CONFIGURATION ───────────────────────────────────────────────
    let env = bench_env::EnvInfo::collect();

    println!();
    println!("  3. CONFIGURATION");
    println!("  -----------------");
    println!();
    println!("  Hardware:");
    println!("    OS:            {}", env.os);
    println!("    Kernel:        {}", env.kernel);
    println!("    CPU:           {}", env.cpu_model);
    println!("    Cores:         {} physical / {} logical", env.cpu_cores_physical, env.cpu_cores_logical);
    println!("    RAM:           {}", env.memory_total);
    println!();
    println!("  Toolchain:");
    println!("    Rust:          {}", env.rust_version);
    println!("    Target:        {}", env.target_triple);
    println!("    Profile:       {}", env.profile);
    println!();
    println!("  Benchmark Parameters:");
    println!("    Iterations:    {} per message type", iterations);
    println!("    Warmup:        10 iterations (discarded)");
    println!("    Allocator:     CountingAllocator (wraps System)");
    println!();
    println!("  Reproduce:");
    println!("    cargo bench -p vyna-hl7-parser --bench alloc_profile_bench");

    // ── 4. RESULTS ─────────────────────────────────────────────────────
    println!();
    println!("  4. RESULTS");
    println!("  ----------");
    println!();
    println!(
        "  {:<22} {:>7} {:>12} {:>11} {:>12}",
        "Message Type", "Size(B)", "Allocs/Parse", "Bytes/Parse", "Amplification"
    );
    println!("  {}", "─".repeat(70));

    measure_parse("ADT_A01 Full", ADT_A01_FULL, iterations);
    measure_parse("ORU_R01 Comprehensive", ORU_R01_COMPREHENSIVE, iterations);
    measure_parse("RDE_O11 Pharmacy", RDE_O11_PHARMACY, iterations);
    measure_parse("SIU_S12 Scheduling", SIU_S12_SCHEDULING, iterations);
    measure_parse("MDM_T02 Document", MDM_T02_DOCUMENT, iterations);
    measure_parse("DFT_P03 Financial", DFT_P03_FINANCIAL, iterations);
    measure_parse("ORM_O01 Order", ORM_O01_ORDER, iterations);

    println!("  {}", "─".repeat(70));
    measure_parse("Escape Heavy", ESCAPE_HEAVY, iterations);
    measure_parse("Production Messy", PRODUCTION_MESSY, iterations);

    println!("  {}", "─".repeat(70));
    let batch_5 = generate_batch_message(5);
    measure_batch("Batch (5 msgs)", &batch_5, iterations);
    let batch_50 = generate_batch_message(50);
    measure_batch("Batch (50 msgs)", &batch_50, iterations / 10);

    println!("  {}", "─".repeat(70));
    println!();
    println!("  Key:");
    println!("    Allocs/Parse   = average heap allocations per parse_message() call");
    println!("    Bytes/Parse    = average bytes requested from allocator per parse");
    println!("    Amplification  = Bytes/Parse divided by input Size(B)");
    println!("                     Lower is better. 1.0x = zero overhead (theoretical min)");
    println!();
    println!("================================================================================");
    println!();
}
