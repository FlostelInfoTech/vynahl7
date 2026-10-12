//! HL7 Performance Benchmarks
//!
//! Test categories follow standard interface engine benchmarking methodology
//! (T1/T2/T3 classifications from HL7 throughput testing practices):
//!
//! 1. **Store-and-Forward (T1)**: Raw parsing without transformation
//! 2. **Message Translation (T2)**: Parse + extract fields + create ACK
//! 3. **Round-Trip (T3)**: Parse + modify + encode
//! 4. **Sustained Throughput**: Continuous processing simulation
//! 5. **Message Size Scaling**: Performance across different message sizes

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use vyna_hl7_parser::parse_message;
use std::hint::black_box;
use std::time::{Duration, Instant};

// ============================================================================
// Test Messages - Industry Representative Samples
// ============================================================================

/// Small ADT^A01 - Typical admission notification (~250 bytes)
/// Representative of high-frequency real-time notifications
const ADT_SMALL: &str = "MSH|^~\\&|HIS|Hospital|RIS|Radiology|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^Hospital^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU^101^A^Hospital||||1234567^SMITH^JANE^M^MD^^MD|||||||||VIP|||||||||||||||||||||||||20240315140000";

/// Medium ORU^R01 - Lab results with multiple OBX (~1.5KB)
/// Representative of laboratory result messages
const ORU_MEDIUM: &str = "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^Hospital^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|O|ER^201^B^Hospital||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD123456|LAB789012||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD123456|LAB789012|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||7.5|10*9/L|4.5-11.0|N|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500\r\
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315115500\r\
OBX|4|NM|HCT^Hematocrit^LN||42.0|%|36.0-46.0|N|||F|||20240315115500\r\
OBX|5|NM|PLT^Platelet Count^LN||250|10*9/L|150-400|N|||F|||20240315115500\r\
OBX|6|NM|MCV^Mean Corpuscular Volume^LN||88.0|fL|80.0-100.0|N|||F|||20240315115500\r\
OBX|7|NM|MCH^Mean Corpuscular Hemoglobin^LN||30.2|pg|27.0-33.0|N|||F|||20240315115500\r\
OBX|8|NM|MCHC^Mean Corpuscular Hemoglobin Concentration^LN||34.5|g/dL|32.0-36.0|N|||F|||20240315115500";

/// Large batch message with multiple patient results (~10KB)
fn generate_large_batch() -> String {
    let mut msg = String::with_capacity(12000);
    msg.push_str("MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315143000||ORU^R01^ORU_R01|MSG00003|P|2.5|||AL|NE\r");

    for patient in 1..=5 {
        msg.push_str(&format!(
            "PID|{}||MRN{:05}^^^Hospital^MR||PATIENT^TEST^{}||19800{}15|M\r",
            patient, patient, patient, patient
        ));
        msg.push_str(&format!(
            "PV1|1|O|LAB^{}01^A^Hospital||||1234567^DOCTOR^TEST^M^MD\r",
            patient
        ));
        msg.push_str(&format!(
            "ORC|RE|ORD{:06}|LAB{:06}||CM||||20240315120000|||1234567^DOCTOR^TEST^M^MD\r",
            patient * 1000, patient * 1000
        ));
        msg.push_str(&format!(
            "OBR|1|ORD{:06}|LAB{:06}|PANEL^Test Panel^LN|||20240315110000|||||||||1234567^DOCTOR^TEST^M^MD||||||20240315120000|||F\r",
            patient * 1000, patient * 1000
        ));

        for obs in 1..=10 {
            msg.push_str(&format!(
                "OBX|{}|NM|TEST{}^Test Observation {}^LN||{}.{}|unit|0-100|N|||F|||20240315115500\r",
                obs, obs, obs, patient * 10 + obs, obs
            ));
        }
    }

    msg
}

/// Very large message (~50KB) - stress test
fn generate_stress_test_message() -> String {
    let mut msg = String::with_capacity(55000);
    msg.push_str("MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315143000||ORU^R01^ORU_R01|MSG00004|P|2.5|||AL|NE\r");
    msg.push_str("PID|1||MRN99999^^^Hospital^MR||STRESS^TEST^PATIENT||19800101|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r");
    msg.push_str("PV1|1|I|ICU^101^A^Hospital||||1234567^DOCTOR^STRESS^M^MD\r");

    for order in 1..=20 {
        msg.push_str(&format!(
            "ORC|RE|ORD{:06}|LAB{:06}||CM||||20240315120000\r",
            order * 100, order * 100
        ));
        msg.push_str(&format!(
            "OBR|{}|ORD{:06}|LAB{:06}|PANEL{}^Panel {}^LN|||20240315110000|||||||||1234567^DOCTOR^STRESS^M^MD||||||20240315120000|||F\r",
            order, order * 100, order * 100, order, order
        ));

        for obs in 1..=25 {
            msg.push_str(&format!(
                "OBX|{}|NM|T{:03}^Test Observation {:03}^LN||{}.{}|unit|0.0-100.0|N|||F|||20240315115500||TECH001^TECHNICIAN^LAB\r",
                obs, order * 100 + obs, order * 100 + obs, order, obs
            ));
        }
    }

    msg
}

// ============================================================================
// T1: Store-and-Forward Benchmarks (Passthrough Parsing)
// ============================================================================

fn bench_t1_store_forward(c: &mut Criterion) {
    let mut group = c.benchmark_group("T1_Store_Forward");
    group.measurement_time(Duration::from_secs(10));

    // Small ADT - high frequency messages
    let small_size = ADT_SMALL.len();
    group.throughput(Throughput::Bytes(small_size as u64));
    group.bench_with_input(
        BenchmarkId::new("ADT_A01_Small", format!("{}B", small_size)),
        ADT_SMALL,
        |b, msg| {
            b.iter(|| parse_message(black_box(msg)).unwrap());
        },
    );

    // Medium ORU - typical lab results
    let medium_size = ORU_MEDIUM.len();
    group.throughput(Throughput::Bytes(medium_size as u64));
    group.bench_with_input(
        BenchmarkId::new("ORU_R01_Medium", format!("{}B", medium_size)),
        ORU_MEDIUM,
        |b, msg| {
            b.iter(|| parse_message(black_box(msg)).unwrap());
        },
    );

    // Large batch
    let large_msg = generate_large_batch();
    let large_size = large_msg.len();
    group.throughput(Throughput::Bytes(large_size as u64));
    group.bench_with_input(
        BenchmarkId::new("Batch_Large", format!("{}KB", large_size / 1024)),
        &large_msg,
        |b, msg| {
            b.iter(|| parse_message(black_box(msg)).unwrap());
        },
    );

    // Stress test
    let stress_msg = generate_stress_test_message();
    let stress_size = stress_msg.len();
    group.throughput(Throughput::Bytes(stress_size as u64));
    group.bench_with_input(
        BenchmarkId::new("Stress_VeryLarge", format!("{}KB", stress_size / 1024)),
        &stress_msg,
        |b, msg| {
            b.iter(|| parse_message(black_box(msg)).unwrap());
        },
    );

    group.finish();
}

// ============================================================================
// T2: Message Translation Benchmarks (Parse + Extract + Generate ACK)
// ============================================================================

fn generate_ack(original: &vyna_hl7_core::Message) -> String {
    // Extract key fields for ACK generation
    let msh = &original.segments[0];
    let msg_control_id = msh.fields.get(9).and_then(|f| f.value()).unwrap_or("0");
    let sending_app = msh.fields.get(2).and_then(|f| f.value()).unwrap_or("");
    let sending_fac = msh.fields.get(3).and_then(|f| f.value()).unwrap_or("");
    let receiving_app = msh.fields.get(4).and_then(|f| f.value()).unwrap_or("");
    let receiving_fac = msh.fields.get(5).and_then(|f| f.value()).unwrap_or("");
    let version = msh.fields.get(11).and_then(|f| f.value()).unwrap_or("2.5");

    format!(
        "MSH|^~\\&|{}|{}|{}|{}|20240315143001||ACK^A01^ACK|ACK{}|P|{}\r\
MSA|AA|{}\r",
        receiving_app,
        receiving_fac,
        sending_app,
        sending_fac,
        msg_control_id,
        version,
        msg_control_id
    )
}

fn bench_t2_translation(c: &mut Criterion) {
    let mut group = c.benchmark_group("T2_Translation");
    group.measurement_time(Duration::from_secs(10));

    // Parse + ACK generation (typical interface engine operation)
    group.bench_function("ADT_Parse_Generate_ACK", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ADT_SMALL)).unwrap();
            let ack = generate_ack(&msg);
            black_box(ack)
        });
    });

    group.bench_function("ORU_Parse_Generate_ACK", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ORU_MEDIUM)).unwrap();
            let ack = generate_ack(&msg);
            black_box(ack)
        });
    });

    group.finish();
}

// ============================================================================
// T3: Round-Trip Benchmarks (Parse + Modify + Encode)
// ============================================================================

fn bench_t3_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("T3_RoundTrip");
    group.measurement_time(Duration::from_secs(10));

    // Full round-trip: Parse -> Encode (no modification)
    group.bench_function("ADT_Parse_Encode", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ADT_SMALL)).unwrap();
            let encoded = msg.encode();
            black_box(encoded)
        });
    });

    group.bench_function("ORU_Parse_Encode", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ORU_MEDIUM)).unwrap();
            let encoded = msg.encode();
            black_box(encoded)
        });
    });

    let large_msg = generate_large_batch();
    group.bench_function("Batch_Parse_Encode", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(&large_msg)).unwrap();
            let encoded = msg.encode();
            black_box(encoded)
        });
    });

    group.finish();
}

// ============================================================================
// Sustained Throughput Test
// ============================================================================

fn bench_sustained_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("Sustained_Throughput");
    group.measurement_time(Duration::from_secs(30)); // Longer measurement for sustained test
    group.sample_size(50); // Fewer samples but longer duration

    // Simulate continuous message processing
    let messages: Vec<&str> = vec![ADT_SMALL; 1000];

    group.throughput(Throughput::Elements(1000));
    group.bench_function("1000_ADT_Messages", |b| {
        b.iter(|| {
            for msg in &messages {
                let _ = parse_message(black_box(*msg)).unwrap();
            }
        });
    });

    // Mixed message types (more realistic)
    let large_msg = generate_large_batch();
    let mixed_messages: Vec<&str> = (0..100)
        .map(|i| {
            if i % 10 == 0 {
                large_msg.as_str()
            } else if i % 3 == 0 {
                ORU_MEDIUM
            } else {
                ADT_SMALL
            }
        })
        .collect();

    group.throughput(Throughput::Elements(100));
    group.bench_function("100_Mixed_Messages", |b| {
        b.iter(|| {
            for msg in &mixed_messages {
                let _ = parse_message(black_box(*msg)).unwrap();
            }
        });
    });

    group.finish();
}

// ============================================================================
// Message Size Scaling Analysis
// ============================================================================

fn bench_size_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("Size_Scaling");
    group.measurement_time(Duration::from_secs(10));

    // Test parsing performance as message size increases
    let sizes = [
        (5, "5_segments"),
        (10, "10_segments"),
        (25, "25_segments"),
        (50, "50_segments"),
        (100, "100_segments"),
        (200, "200_segments"),
    ];

    for (segment_count, label) in sizes {
        let mut msg = String::from(
            "MSH|^~\\&|LAB|Hospital|EMR|Hospital|20240315143000||ORU^R01|MSG001|P|2.5\r\
PID|1||MRN12345^^^Hospital^MR||DOE^JOHN||19800101|M\r",
        );

        for i in 1..=segment_count {
            msg.push_str(&format!(
                "OBX|{}|NM|T{}^Test {}^LN||{}.5|unit|0-100|N|||F\r",
                i, i, i, i
            ));
        }

        let size = msg.len();
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("OBX_Segments", label), &msg, |b, msg| {
            b.iter(|| parse_message(black_box(msg)).unwrap());
        });
    }

    group.finish();
}

// ============================================================================
// Latency Percentile Analysis (Custom)
// ============================================================================

// Used only by the ignored `latency_percentiles` test.
#[allow(dead_code)]
fn measure_latency_percentiles() {
    println!("\n=== Latency Percentile Analysis ===\n");

    let iterations = 10000;
    let mut latencies: Vec<Duration> = Vec::with_capacity(iterations);

    // Warm up
    for _ in 0..1000 {
        let _ = parse_message(ADT_SMALL).unwrap();
    }

    // Measure
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = parse_message(ADT_SMALL).unwrap();
        latencies.push(start.elapsed());
    }

    latencies.sort();

    let p50 = latencies[iterations * 50 / 100];
    let p90 = latencies[iterations * 90 / 100];
    let p95 = latencies[iterations * 95 / 100];
    let p99 = latencies[iterations * 99 / 100];
    let p999 = latencies[iterations * 999 / 1000];

    println!("ADT_A01 Small Message ({} iterations):", iterations);
    println!("  p50:   {:>10.3} µs", p50.as_nanos() as f64 / 1000.0);
    println!("  p90:   {:>10.3} µs", p90.as_nanos() as f64 / 1000.0);
    println!("  p95:   {:>10.3} µs", p95.as_nanos() as f64 / 1000.0);
    println!("  p99:   {:>10.3} µs", p99.as_nanos() as f64 / 1000.0);
    println!("  p99.9: {:>10.3} µs", p999.as_nanos() as f64 / 1000.0);
    println!(
        "  Throughput at p50: {:>10.0} msg/s",
        1_000_000_000.0 / p50.as_nanos() as f64
    );
}

// ============================================================================
// Summary Report Generator
// ============================================================================

// Used only by the ignored `latency_percentiles` test.
#[allow(dead_code)]
fn print_summary_header() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║              VynaHL7 HL7 Performance Benchmarks                          ║");
    println!("╠══════════════════════════════════════════════════════════════════════╣");
    println!("║  Test methodology: T1/T2/T3 classifications from standard            ║");
    println!("║  HL7 interface engine throughput testing practices                    ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
}

criterion_group! {
    name = industry_benches;
    config = Criterion::default()
        .significance_level(0.01)
        .noise_threshold(0.03);
    targets =
        bench_t1_store_forward,
        bench_t2_translation,
        bench_t3_roundtrip,
        bench_sustained_throughput,
        bench_size_scaling
}

criterion_main!(industry_benches);

#[cfg(test)]
mod tests {
    
    

    #[test]
    fn test_messages_parse() {
        parse_message(ADT_SMALL).unwrap();
        parse_message(ORU_MEDIUM).unwrap();
        parse_message(&generate_large_batch()).unwrap();
        parse_message(&generate_stress_test_message()).unwrap();
    }

    #[test]
    fn test_ack_generation() {
        let msg = parse_message(ADT_SMALL).unwrap();
        let ack = generate_ack(&msg);
        assert!(ack.contains("MSA|AA|"));
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored --nocapture
    fn latency_percentiles() {
        print_summary_header();
        measure_latency_percentiles();
    }
}
