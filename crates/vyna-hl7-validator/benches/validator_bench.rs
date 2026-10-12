//! Validator benchmarks: schema validation, data type validation,
//! vocabulary validation, and full parse+validate pipeline.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use vyna_hl7_core::types::DataType;
use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;
use vyna_hl7_validator::{validate_data_type, TableRegistry, Validator};
use std::hint::black_box;

// Representative messages for validation benchmarks
const ADT_A01: &str = "\
MSH|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
EVN|A01|20240315143000\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD^^MD|||||||||VIP|||||||||||||||||||||||||||||||HOSPITAL|||||20240315140000";

const ORU_R01: &str = "\
MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M\r\
PV1|1|O|ER^201^B^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD123456|LAB789012||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD123456|LAB789012|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||7.5|10*9/L|4.5-11.0|N|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500";

const ORM_O01: &str = "\
MSH|^~\\&|EMR|HOSPITAL|LAB|HOSPITAL|20240315200000||ORM^O01^ORM_O01|MSG00007|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|NW|ORD100001||ORD100001|IP||||20240315200000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD100001||CBC^Complete Blood Count^LN|||20240315200000|||||||||1234567^SMITH^JANE^M^MD";

// ============================================================================
// Schema Validation
// ============================================================================

fn bench_schema_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Schema_Validation");

    // Benchmark schema loading
    let schemas: Vec<(&str, &str, &str)> = vec![
        ("ADT_A01", "ADT", "A01"),
        ("ORU_R01", "ORU", "R01"),
        ("ORM_O01", "ORM", "O01"),
    ];

    for (name, msg_type, trigger) in &schemas {
        group.bench_with_input(
            BenchmarkId::new("load_schema", *name),
            &(msg_type, trigger),
            |b, (mt, te)| {
                b.iter(|| {
                    black_box(Validator::for_message_type(Version::V2_5, mt, te).unwrap())
                });
            },
        );
    }

    // Benchmark validation with pre-loaded validator
    let messages: Vec<(&str, &str, &str, &str)> = vec![
        ("ADT_A01", "ADT", "A01", ADT_A01),
        ("ORU_R01", "ORU", "R01", ORU_R01),
        ("ORM_O01", "ORM", "O01", ORM_O01),
    ];

    for (name, msg_type, trigger, raw_msg) in &messages {
        let validator = Validator::for_message_type(Version::V2_5, msg_type, trigger).unwrap();
        let parsed = parse_message(raw_msg).unwrap();

        group.bench_with_input(
            BenchmarkId::new("validate", *name),
            &(&validator, &parsed),
            |b, (v, m)| {
                b.iter(|| black_box(v.validate(m)));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Data Type Validation
// ============================================================================

fn bench_datatype_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("DataType_Validation");

    let test_cases: Vec<(&str, &str, DataType)> = vec![
        ("DT_valid", "20240315", DataType::DT),
        ("TM_valid", "143000", DataType::TM),
        ("DTM_valid", "20240315143000", DataType::DTM),
        ("NM_valid", "7.5", DataType::NM),
        ("SI_valid", "1", DataType::SI),
        ("ST_valid", "Normal string value", DataType::ST),
        ("CWE_valid", "WBC^White Blood Cell^LN", DataType::CWE),
        ("XPN_valid", "DOE^JOHN^ALLEN^JR^DR^PHD", DataType::XPN),
    ];

    for (name, value, data_type) in &test_cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), value, |b, val| {
            b.iter(|| black_box(validate_data_type(black_box(val), *data_type)));
        });
    }

    group.finish();
}

// ============================================================================
// Vocabulary Validation
// ============================================================================

fn bench_vocabulary_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Vocabulary_Validation");
    let registry = TableRegistry::new();

    let test_cases: Vec<(&str, &str, &str)> = vec![
        ("Table_0001_sex_M", "0001", "M"),
        ("Table_0001_sex_F", "0001", "F"),
        ("Table_0004_class_I", "0004", "I"),
        ("Table_0004_class_O", "0004", "O"),
        ("Table_0085_status_F", "0085", "F"),
        ("Table_0085_status_P", "0085", "P"),
        ("Table_0103_proc_T", "0103", "T"),
        ("Table_0001_invalid", "0001", "Z"),
    ];

    for (name, table_id, code) in &test_cases {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(table_id, code),
            |b, (tid, c)| {
                b.iter(|| black_box(registry.validate(black_box(tid), black_box(c))));
            },
        );
    }

    group.finish();
}

// ============================================================================
// Full Validation Pipeline (Parse + Validate)
// ============================================================================

fn bench_full_validation_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("Full_Validation_Pipeline");

    let messages: Vec<(&str, &str, &str, &str)> = vec![
        ("ADT_A01", "ADT", "A01", ADT_A01),
        ("ORU_R01", "ORU", "R01", ORU_R01),
        ("ORM_O01", "ORM", "O01", ORM_O01),
    ];

    for (name, msg_type, trigger, raw_msg) in &messages {
        let validator = Validator::for_message_type(Version::V2_5, msg_type, trigger).unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            raw_msg,
            |b, msg| {
                b.iter(|| {
                    let parsed = parse_message(black_box(msg)).unwrap();
                    black_box(validator.validate(&parsed))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_schema_validation,
    bench_datatype_validation,
    bench_vocabulary_validation,
    bench_full_validation_pipeline
);
criterion_main!(benches);
