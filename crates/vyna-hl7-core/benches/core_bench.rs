//! Core crate benchmarks: encoding, escape sequence handling, and builder construction.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use vyna_hl7_core::builders::adt::AdtA01Builder;
use vyna_hl7_core::builders::oru::OruR01Builder;
use vyna_hl7_core::{Delimiters, Encoding, Version};
use vyna_hl7_parser::parse_message;
use std::hint::black_box;

// Corpus messages for encode benchmarks (inline to avoid cross-crate bench dep)
const ADT_SMALL: &str = "\
MSH|^~\\&|HIS|HOSPITAL|RIS|RADIOLOGY|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD^^MD";

const ORU_MEDIUM: &str = "\
MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M\r\
PV1|1|O|ER^201^B^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD123456|LAB789012||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD123456|LAB789012|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||7.5|10*9/L|4.5-11.0|N|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500\r\
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315115500\r\
OBX|4|NM|PLT^Platelet Count^LN||250|10*9/L|150-400|N|||F|||20240315115500";

// ============================================================================
// Encode Isolated (encoding cost separated from parsing)
// ============================================================================

fn bench_encode_isolated(c: &mut Criterion) {
    let mut group = c.benchmark_group("Encode_Isolated");

    let adt = parse_message(ADT_SMALL).unwrap();
    let adt_size = ADT_SMALL.len();
    group.throughput(Throughput::Bytes(adt_size as u64));
    group.bench_function("ADT_A01_encode", |b| {
        b.iter(|| black_box(adt.encode()));
    });

    let oru = parse_message(ORU_MEDIUM).unwrap();
    let oru_size = ORU_MEDIUM.len();
    group.throughput(Throughput::Bytes(oru_size as u64));
    group.bench_function("ORU_R01_encode", |b| {
        b.iter(|| black_box(oru.encode()));
    });

    group.finish();
}

// ============================================================================
// Escape Sequence Encode/Decode
// ============================================================================

fn bench_escape_encode_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("EscapeSequence_EncodeDecode");
    let delimiters = Delimiters::default();

    // No escapes
    let plain = "Normal text without any special characters at all here";
    group.bench_function("encode_plain", |b| {
        b.iter(|| black_box(Encoding::encode(black_box(plain), &delimiters)));
    });
    group.bench_function("decode_plain", |b| {
        b.iter(|| black_box(Encoding::decode(black_box(plain), &delimiters)));
    });

    // Light escapes (~10%)
    let light = "Patient name: O'Brien|Smith & Jones ^ Department";
    let light_encoded = Encoding::encode(light, &delimiters);
    group.bench_function("encode_light_escapes", |b| {
        b.iter(|| black_box(Encoding::encode(black_box(light), &delimiters)));
    });
    group.bench_function("decode_light_escapes", |b| {
        b.iter(|| black_box(Encoding::decode(black_box(&light_encoded), &delimiters)));
    });

    // Heavy escapes (~40%)
    let heavy = "BP: 120\\S\\80 | Temp: 98.6\\S\\F | Meds: PCN\\T\\AMOX\\T\\SULFA | Notes: See \\F\\ chart";
    group.bench_function("encode_heavy_escapes", |b| {
        b.iter(|| black_box(Encoding::encode(black_box(heavy), &delimiters)));
    });
    group.bench_function("decode_heavy_escapes", |b| {
        b.iter(|| black_box(Encoding::decode(black_box(heavy), &delimiters)));
    });

    group.finish();
}

// ============================================================================
// Builder Construction
// ============================================================================

fn bench_builder_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Builder_Construction");

    group.bench_function("AdtA01Builder", |b| {
        b.iter(|| {
            let msg = AdtA01Builder::new(Version::V2_5)
                .sending_application("HIS")
                .sending_facility("HOSPITAL")
                .receiving_application("EMR")
                .receiving_facility("HOSPITAL")
                .control_id("MSG00001")
                .patient_id("MRN12345")
                .patient_name("DOE", "JOHN")
                .date_of_birth("19800315")
                .sex("M")
                .patient_class("I")
                .assigned_location("ICU^101^A")
                .attending_doctor("1234567^SMITH^JANE^M^MD")
                .build();
            black_box(msg)
        });
    });

    group.bench_function("OruR01Builder", |b| {
        b.iter(|| {
            let msg = OruR01Builder::new(Version::V2_5)
                .sending_application("LAB")
                .sending_facility("HOSPITAL")
                .receiving_application("EMR")
                .receiving_facility("HOSPITAL")
                .control_id("MSG00002")
                .patient_id("MRN12345")
                .patient_name("DOE", "JOHN")
                .filler_order_number("LAB789012")
                .add_observation(vyna_hl7_core::builders::oru::Observation {
                    set_id: 1,
                    value_type: "NM".to_string(),
                    identifier: "WBC^White Blood Cell Count^LN".to_string(),
                    value: "7.5".to_string(),
                    units: Some("10*9/L".to_string()),
                    status: "F".to_string(),
                })
                .add_observation(vyna_hl7_core::builders::oru::Observation {
                    set_id: 2,
                    value_type: "NM".to_string(),
                    identifier: "RBC^Red Blood Cell Count^LN".to_string(),
                    value: "4.8".to_string(),
                    units: Some("10*12/L".to_string()),
                    status: "F".to_string(),
                })
                .build();
            black_box(msg)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_isolated,
    bench_escape_encode_decode,
    bench_builder_construction
);
criterion_main!(benches);
