//! CachedTerser benchmarks: comparison with Terser, clinical workflow extraction,
//! and scaling behavior with increasing path counts.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::{CachedTerser, Terser};
use std::hint::black_box;

// ORU with multiple OBX for realistic extraction
const ORU_MESSAGE: &str = "\
MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|O|ER^201^B^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
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

const PATHS_10: &[&str] = &[
    "PID-3",
    "PID-5-1",
    "PID-5-2",
    "PID-7",
    "PID-8",
    "OBX-3",
    "OBX-5",
    "OBX-7",
    "OBX(2)-5",
    "OBR-4",
];

// ============================================================================
// CachedTerser vs Terser Comparison
// ============================================================================

fn bench_cached_vs_terser(c: &mut Criterion) {
    let mut group = c.benchmark_group("CachedTerser_vs_Terser");
    let message = parse_message(ORU_MESSAGE).unwrap();

    // Standard Terser: 10 paths, accessed 5 times each (simulating repeated access)
    group.bench_function("Terser_10paths_x5", |b| {
        let terser = Terser::new(&message);
        b.iter(|| {
            for _ in 0..5 {
                for path in PATHS_10 {
                    let _ = terser.get(black_box(path));
                }
            }
        });
    });

    // CachedTerser: same 10 paths, accessed 5 times each
    group.bench_function("CachedTerser_10paths_x5", |b| {
        b.iter(|| {
            let mut cached = CachedTerser::new(&message);
            for _ in 0..5 {
                for path in PATHS_10 {
                    let _ = cached.get(black_box(path));
                }
            }
        });
    });

    // CachedTerser with warm_cache
    group.bench_function("CachedTerser_warmed_10paths_x5", |b| {
        b.iter(|| {
            let mut cached = CachedTerser::new(&message);
            cached.warm_cache(PATHS_10).ok();
            for _ in 0..5 {
                for path in PATHS_10 {
                    let _ = cached.get(black_box(path));
                }
            }
        });
    });

    group.finish();
}

// ============================================================================
// Clinical Workflow Extraction Patterns
// ============================================================================

fn bench_clinical_workflows(c: &mut Criterion) {
    let mut group = c.benchmark_group("Clinical_Workflow_Extract");
    let message = parse_message(ORU_MESSAGE).unwrap();

    // Patient demographics extraction
    let demographics_paths: &[&str] = &[
        "PID-3", "PID-5-1", "PID-5-2", "PID-7", "PID-8", "PID-11",
    ];
    group.bench_function("demographics", |b| {
        b.iter(|| {
            let mut cached = CachedTerser::new(&message);
            for path in demographics_paths {
                let _ = cached.get(black_box(path));
            }
        });
    });

    // Lab results summary (extracting key fields from all OBX segments)
    let lab_paths: Vec<String> = (1..=8)
        .flat_map(|i| {
            vec![
                format!("OBX({})-3", i),
                format!("OBX({})-5", i),
                format!("OBX({})-7", i),
                format!("OBX({})-8", i),
            ]
        })
        .collect();
    let lab_path_refs: Vec<&str> = lab_paths.iter().map(|s| s.as_str()).collect();

    group.bench_function("lab_results_8_observations", |b| {
        b.iter(|| {
            let mut cached = CachedTerser::new(&message);
            for path in &lab_path_refs {
                let _ = cached.get(black_box(path));
            }
        });
    });

    // Order information extraction
    let order_paths: &[&str] = &["ORC-2", "ORC-3", "OBR-4", "OBR-7", "OBR-25"];
    group.bench_function("order_info", |b| {
        b.iter(|| {
            let mut cached = CachedTerser::new(&message);
            for path in order_paths {
                let _ = cached.get(black_box(path));
            }
        });
    });

    group.finish();
}

// ============================================================================
// CachedTerser Scaling with Path Count
// ============================================================================

fn bench_cached_terser_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("CachedTerser_Scaling");
    let message = parse_message(ORU_MESSAGE).unwrap();

    // Generate increasing numbers of unique paths
    let all_paths: Vec<String> = {
        let mut paths = Vec::new();
        // PID fields
        for f in 1..=30 {
            paths.push(format!("PID-{}", f));
        }
        // OBX indexed fields
        for seg in 1..=8 {
            for f in 1..=15 {
                paths.push(format!("OBX({})-{}", seg, f));
            }
        }
        // PV1, ORC, OBR fields
        for f in 1..=20 {
            paths.push(format!("PV1-{}", f));
            paths.push(format!("ORC-{}", f));
            paths.push(format!("OBR-{}", f));
        }
        paths
    };

    for count in [10, 50, 100] {
        let paths: Vec<&str> = all_paths.iter().take(count).map(|s| s.as_str()).collect();

        group.bench_with_input(
            BenchmarkId::new("unique_paths", count),
            &paths,
            |b, paths| {
                b.iter(|| {
                    let mut cached = CachedTerser::new(&message);
                    for path in paths.iter() {
                        let _ = cached.get(black_box(path));
                    }
                    // Access again to test cache hit performance
                    for path in paths.iter() {
                        let _ = cached.get(black_box(path));
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cached_vs_terser,
    bench_clinical_workflows,
    bench_cached_terser_scaling
);
criterion_main!(benches);
