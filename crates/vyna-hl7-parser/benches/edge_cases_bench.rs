//! Parser benchmarks for edge cases: escape sequences, batch/file parsing,
//! lenient vs strict mode, and production-like messy messages.

#[allow(dead_code)]
mod corpus;

use corpus::*;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use vyna_hl7_parser::{parse_batch, parse_file, parse_message, parse_message_with_config, ParserConfig};
use std::hint::black_box;

// ============================================================================
// Escape Sequence Density
// ============================================================================

fn generate_escape_message(escape_pct: usize) -> String {
    // Build a PID segment where `escape_pct` % of field values contain escapes
    let mut fields: Vec<String> = Vec::new();
    for i in 0..20 {
        if (i * 100 / 20) < escape_pct {
            fields.push(format!("Value\\T\\{}\\S\\data\\F\\end", i));
        } else {
            fields.push(format!("NormalValue{}", i));
        }
    }
    format!(
        "MSH|^~\\&|APP|FAC|RECV|RFAC|20240315143000||ADT^A01|MSG001|P|2.5\r\
PID|1||MRN001|||{}",
        fields.join("|")
    )
}

fn bench_escape_density(c: &mut Criterion) {
    let mut group = c.benchmark_group("EscapeSequence_Density");

    for pct in [0, 5, 20, 50] {
        let msg = generate_escape_message(pct);
        group.throughput(Throughput::Bytes(msg.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse", format!("{}%_escapes", pct)),
            &msg,
            |b, msg| {
                b.iter(|| parse_message(black_box(msg)).unwrap());
            },
        );
    }

    group.finish();
}

// ============================================================================
// Batch and File Parsing
// ============================================================================

fn bench_batch_file_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Batch_File_Parsing");

    for count in [1, 5, 10, 50] {
        let batch = generate_batch_message(count);
        group.throughput(Throughput::Bytes(batch.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_batch", format!("{}_messages", count)),
            &batch,
            |b, msg| {
                b.iter(|| parse_batch(black_box(msg)).unwrap());
            },
        );
    }

    // File with 2 batches of 10 messages each
    let file_msg = generate_file_message(2, 10);
    group.throughput(Throughput::Bytes(file_msg.len() as u64));
    group.bench_function("parse_file_2x10", |b| {
        b.iter(|| parse_file(black_box(&file_msg)).unwrap());
    });

    group.finish();
}

// ============================================================================
// Lenient vs Strict Parsing
// ============================================================================

fn bench_lenient_vs_strict(c: &mut Criterion) {
    let mut group = c.benchmark_group("Lenient_vs_Strict");

    let strict_config = ParserConfig::strict();
    let lenient_config = ParserConfig::lenient();

    // Use the messy production message
    let msg = PRODUCTION_MESSY;
    group.throughput(Throughput::Bytes(msg.len() as u64));

    group.bench_function("strict", |b| {
        b.iter(|| {
            // Strict may error on messy input - that's fine, we measure the attempt
            let _ = parse_message_with_config(black_box(msg), &strict_config);
        });
    });

    group.bench_function("lenient", |b| {
        b.iter(|| {
            let _ = parse_message_with_config(black_box(msg), &lenient_config);
        });
    });

    group.finish();
}

// ============================================================================
// Production Patterns
// ============================================================================

fn bench_production_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("Production_Patterns");

    // Trailing delimiters / empty components
    let msg = PRODUCTION_MESSY;
    group.throughput(Throughput::Bytes(msg.len() as u64));
    group.bench_function("trailing_delimiters", |b| {
        let config = ParserConfig::lenient();
        b.iter(|| {
            let _ = parse_message_with_config(black_box(msg), &config);
        });
    });

    // Escape-heavy message
    let msg = ESCAPE_HEAVY;
    group.throughput(Throughput::Bytes(msg.len() as u64));
    group.bench_function("escape_heavy", |b| {
        b.iter(|| parse_message(black_box(msg)).unwrap());
    });

    // Large narrative OBX (MDM document)
    let msg = MDM_T02_DOCUMENT;
    group.throughput(Throughput::Bytes(msg.len() as u64));
    group.bench_function("large_narrative_obx", |b| {
        b.iter(|| parse_message(black_box(msg)).unwrap());
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_escape_density,
    bench_batch_file_parsing,
    bench_lenient_vs_strict,
    bench_production_patterns
);
criterion_main!(benches);
