//! Standalone benchmark environment report.
//!
//! Prints system configuration details to contextualize benchmark results.
//! Run this before criterion benchmarks to document the test environment.
//!
//! Usage: cargo bench -p vyna-hl7-parser --bench env_report_bench

#[allow(dead_code)]
mod bench_env;

fn main() {
    let env = bench_env::EnvInfo::collect();
    env.print_report();
}
