use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::Terser;
use std::hint::black_box;

const TEST_MESSAGE: &str = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|12345|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 Main St^^Boston^MA^02101||555-1234
PV1|1|I|ER^101^1||||12345^SMITH^JANE^^^MD
OBX|1|NM|WBC^White Blood Count^LN||7.5|10*3/uL|4.5-11.0|N|||F|||20240315120000
OBX|2|NM|RBC^Red Blood Count^LN||4.8|10*6/uL|4.2-5.9|N|||F|||20240315120000";

fn bench_terser_get_simple(c: &mut Criterion) {
    let message = parse_message(TEST_MESSAGE).unwrap();
    let terser = Terser::new(&message);

    c.bench_function("terser_get_simple_field", |b| {
        b.iter(|| {
            terser.get(black_box("PID-5")).unwrap()
        })
    });
}

fn bench_terser_get_component(c: &mut Criterion) {
    let message = parse_message(TEST_MESSAGE).unwrap();
    let terser = Terser::new(&message);

    c.bench_function("terser_get_component", |b| {
        b.iter(|| {
            terser.get(black_box("PID-5-1")).unwrap()
        })
    });
}

fn bench_terser_get_indexed_segment(c: &mut Criterion) {
    let message = parse_message(TEST_MESSAGE).unwrap();
    let terser = Terser::new(&message);

    c.bench_function("terser_get_indexed_segment", |b| {
        b.iter(|| {
            // OBX(2) = second OBX segment (1-based indexing)
            terser.get(black_box("OBX(2)-5")).unwrap()
        })
    });
}

fn bench_terser_multiple_gets(c: &mut Criterion) {
    let message = parse_message(TEST_MESSAGE).unwrap();
    let terser = Terser::new(&message);

    c.bench_function("terser_10_sequential_gets", |b| {
        b.iter(|| {
            let _ = terser.get(black_box("PID-5")).unwrap();
            let _ = terser.get(black_box("PID-5-1")).unwrap();
            let _ = terser.get(black_box("PID-7")).unwrap();
            let _ = terser.get(black_box("PID-8")).unwrap();
            let _ = terser.get(black_box("PV1-2")).unwrap();
            let _ = terser.get(black_box("OBX-3")).unwrap();
            let _ = terser.get(black_box("OBX-3-1")).unwrap();
            let _ = terser.get(black_box("OBX-5")).unwrap();
            // OBX(2) = second OBX segment (1-based indexing)
            let _ = terser.get(black_box("OBX(2)-3")).unwrap();
            let _ = terser.get(black_box("OBX(2)-5")).unwrap();
        })
    });
}

fn bench_terser_path_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("terser_path_parsing");

    let paths = vec![
        "PID-5",
        "PID-5-1",
        "PID-5-1-2",
        "OBX(2)-5",
        "PID-11(2)-1",
    ];

    for path in paths {
        group.bench_with_input(
            BenchmarkId::from_parameter(path),
            path,
            |b, path| {
                let message = parse_message(TEST_MESSAGE).unwrap();
                let terser = Terser::new(&message);
                b.iter(|| {
                    terser.get(black_box(path)).ok()
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_terser_get_simple,
    bench_terser_get_component,
    bench_terser_get_indexed_segment,
    bench_terser_multiple_gets,
    bench_terser_path_parsing
);
criterion_main!(benches);
