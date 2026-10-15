use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use vyna_hl7_parser::parse_message;

// Small ADT message
const ADT_SMALL: &str = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|12345|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M|||123 Main St^^Boston^MA^02101||555-1234
PV1|1|I|ER^101^1||||12345^SMITH^JANE^^^MD";

// Medium ORU message with multiple observations
const ORU_MEDIUM: &str = r"MSH|^~\&|LAB|Hospital|RecApp|RecFac|20240315143000||ORU^R01|MSG001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M
OBR|1|ORD123|LAB456|CBC^Complete Blood Count^LN|||20240315120000
OBX|1|NM|WBC^White Blood Count^LN||7.5|10*3/uL|4.5-11.0|N|||F|||20240315120000
OBX|2|NM|RBC^Red Blood Count^LN||4.8|10*6/uL|4.2-5.9|N|||F|||20240315120000
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315120000
OBX|4|NM|HCT^Hematocrit^LN||42|%|36-46|N|||F|||20240315120000
OBX|5|NM|PLT^Platelet Count^LN||250|10*3/uL|150-400|N|||F|||20240315120000";

// Large message with many segments
fn generate_large_oru() -> String {
    let mut msg = String::from(
        r"MSH|^~\&|LAB|Hospital|RecApp|RecFac|20240315143000||ORU^R01|MSG001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M
",
    );

    for i in 1..=100 {
        msg.push_str(&format!(
            "OBR|{}|ORD{}|LAB{}|TEST^Test Panel^LN|||20240315120000\n",
            i, i, i
        ));
        for j in 1..=10 {
            msg.push_str(&format!(
                "OBX|{}|NM|T{}^Test {}^LN||{}.{}|unit|0-100|N|||F|||20240315120000\n",
                j,
                i * 10 + j,
                i * 10 + j,
                i,
                j
            ));
        }
    }

    msg
}

fn bench_parse_small(c: &mut Criterion) {
    c.bench_function("parse_small_adt", |b| {
        b.iter(|| parse_message(black_box(ADT_SMALL)).unwrap())
    });
}

fn bench_parse_medium(c: &mut Criterion) {
    c.bench_function("parse_medium_oru", |b| {
        b.iter(|| parse_message(black_box(ORU_MEDIUM)).unwrap())
    });
}

fn bench_parse_large(c: &mut Criterion) {
    let large_msg = generate_large_oru();

    c.bench_function("parse_large_oru_1000_segments", |b| {
        b.iter(|| parse_message(black_box(&large_msg)).unwrap())
    });
}

fn bench_parse_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_by_segment_count");

    for segment_count in [10, 50, 100, 250, 500].iter() {
        let mut msg = String::from(
            r"MSH|^~\&|LAB|Hospital|RecApp|RecFac|20240315143000||ORU^R01|MSG001|P|2.5
PID|1|12345|67890^^^MRN||DOE^JOHN^A||19800101|M
",
        );

        for i in 1..=*segment_count {
            msg.push_str(&format!(
                "OBX|{}|NM|T{}^Test {}^LN||{}.5|unit|0-100|N|||F|||20240315120000\n",
                i, i, i, i
            ));
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(segment_count),
            segment_count,
            |b, _| b.iter(|| parse_message(black_box(&msg)).unwrap()),
        );
    }

    group.finish();
}

fn bench_parse_complex_fields(c: &mut Criterion) {
    // Message with deeply nested components and subcomponents
    let complex = r"MSH|^~\&|SendApp|SendFac|RecApp|RecFac|20240315143000||ADT^A01|12345|P|2.5
PID|1|PAT001&ID&ISO|MRN123&MRN&Hospital~SSN456&SS&USA||DOE^JOHN^ALLEN^JR^DR^PHD||19800101|M|||123&Main&St^Apt&4B^Boston^MA^02101^USA^H||555-1234^PRN^PH~555-5678^WPN^PH~john.doe@email.com^NET^Internet
NK1|1|DOE^JANE^MARIE||SPO^Spouse^HL70063|123&Main&St^Apt&4B^Boston^MA^02101||555-9999^PRN^PH";

    c.bench_function("parse_complex_fields", |b| {
        b.iter(|| parse_message(black_box(complex)).unwrap())
    });
}

criterion_group!(
    benches,
    bench_parse_small,
    bench_parse_medium,
    bench_parse_large,
    bench_parse_by_size,
    bench_parse_complex_fields
);
criterion_main!(benches);
