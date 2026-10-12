//! Parser benchmarks across all major HL7 message types
//!
//! Measures parse and round-trip (parse+encode) performance for 7 distinct
//! message types that represent real production healthcare workloads.

#[allow(dead_code)]
mod corpus;

use corpus::*;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use vyna_hl7_parser::parse_message;
use std::hint::black_box;

fn bench_message_type_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("MessageType_Parse");

    let messages: Vec<(&str, &str)> = vec![
        ("ADT_A01", ADT_A01_FULL),
        ("ORU_R01", ORU_R01_COMPREHENSIVE),
        ("RDE_O11", RDE_O11_PHARMACY),
        ("SIU_S12", SIU_S12_SCHEDULING),
        ("MDM_T02", MDM_T02_DOCUMENT),
        ("DFT_P03", DFT_P03_FINANCIAL),
        ("ORM_O01", ORM_O01_ORDER),
    ];

    for (name, msg) in &messages {
        group.throughput(Throughput::Bytes(msg.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse", format!("{} ({}B)", name, msg.len())),
            *msg,
            |b, msg| {
                b.iter(|| parse_message(black_box(msg)).unwrap());
            },
        );
    }

    group.finish();
}

fn bench_message_type_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("MessageType_RoundTrip");

    let messages: Vec<(&str, &str)> = vec![
        ("ADT_A01", ADT_A01_FULL),
        ("ORU_R01", ORU_R01_COMPREHENSIVE),
        ("RDE_O11", RDE_O11_PHARMACY),
        ("SIU_S12", SIU_S12_SCHEDULING),
        ("MDM_T02", MDM_T02_DOCUMENT),
        ("DFT_P03", DFT_P03_FINANCIAL),
        ("ORM_O01", ORM_O01_ORDER),
    ];

    for (name, msg) in &messages {
        group.throughput(Throughput::Bytes(msg.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("roundtrip", format!("{} ({}B)", name, msg.len())),
            *msg,
            |b, msg| {
                b.iter(|| {
                    let parsed = parse_message(black_box(msg)).unwrap();
                    black_box(parsed.encode())
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_message_type_parse, bench_message_type_roundtrip);
criterion_main!(benches);
