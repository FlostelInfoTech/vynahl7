//! Full cross-crate pipeline benchmarks measuring cumulative cost
//! of parse → validate → extract → transform → encode workflows.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::{CachedTerser, Terser, TerserMut};
use vyna_hl7_validator::Validator;

const ADT_A01: &str = "\
MSH|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
EVN|A01|20240315143000\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD^^MD|||||||||VIP|||||||||||||||||||||||||||||||HOSPITAL|||||20240315140000";

const ORU_R01: &str = "\
MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA\r\
PV1|1|O|ER^201^B^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD123456|LAB789012||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD123456|LAB789012|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||7.5|10*9/L|4.5-11.0|N|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500\r\
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315115500\r\
OBX|4|NM|PLT^Platelet Count^LN||250|10*9/L|150-400|N|||F|||20240315115500";

// ============================================================================
// Pipeline: Parse Only (baseline)
// ============================================================================

fn bench_pipeline_parse_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pipeline_Parse_Only");

    group.throughput(Throughput::Bytes(ADT_A01.len() as u64));
    group.bench_function("ADT_A01", |b| {
        b.iter(|| parse_message(black_box(ADT_A01)).unwrap());
    });

    group.throughput(Throughput::Bytes(ORU_R01.len() as u64));
    group.bench_function("ORU_R01", |b| {
        b.iter(|| parse_message(black_box(ORU_R01)).unwrap());
    });

    group.finish();
}

// ============================================================================
// Pipeline: Parse + Validate
// ============================================================================

fn bench_pipeline_parse_validate(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pipeline_Parse_Validate");

    let adt_validator = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
    group.bench_function("ADT_A01", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ADT_A01)).unwrap();
            black_box(adt_validator.validate(&msg))
        });
    });

    let oru_validator = Validator::for_message_type(Version::V2_5, "ORU", "R01").unwrap();
    group.bench_function("ORU_R01", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ORU_R01)).unwrap();
            black_box(oru_validator.validate(&msg))
        });
    });

    group.finish();
}

// ============================================================================
// Pipeline: Parse + Extract 10 Clinical Fields
// ============================================================================

fn bench_pipeline_parse_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pipeline_Parse_Extract");

    let clinical_paths = [
        "PID-3", "PID-5-1", "PID-5-2", "PID-7", "PID-8", "PV1-2", "PV1-3", "PV1-7-1", "PV1-44",
        "MSH-9",
    ];

    group.bench_function("ADT_10_fields_terser", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ADT_A01)).unwrap();
            let terser = Terser::new(&msg);
            for path in &clinical_paths {
                let _ = terser.get(black_box(path));
            }
        });
    });

    group.bench_function("ADT_10_fields_cached_terser", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ADT_A01)).unwrap();
            let mut cached = CachedTerser::new(&msg);
            for path in &clinical_paths {
                let _ = cached.get(black_box(path));
            }
        });
    });

    group.finish();
}

// ============================================================================
// Pipeline: Parse + Transform + Encode
// ============================================================================

fn bench_pipeline_parse_transform_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pipeline_Parse_Transform_Encode");

    group.bench_function("ADT_modify_and_encode", |b| {
        b.iter(|| {
            let mut msg = parse_message(black_box(ADT_A01)).unwrap();
            let mut terser = TerserMut::new(&mut msg);
            let _ = terser.set("PID-5-1", "SMITH");
            let _ = terser.set("PV1-3", "MED^201^B");
            black_box(msg.encode())
        });
    });

    group.bench_function("ORU_modify_and_encode", |b| {
        b.iter(|| {
            let mut msg = parse_message(black_box(ORU_R01)).unwrap();
            let mut terser = TerserMut::new(&mut msg);
            let _ = terser.set("OBX-8", "HH");
            black_box(msg.encode())
        });
    });

    group.finish();
}

// ============================================================================
// Pipeline: Interface Engine Simulation (Parse + Route + ACK)
// ============================================================================

fn generate_ack(original: &vyna_hl7_core::Message) -> String {
    let msh = &original.segments[0];
    let msg_control_id = msh.fields.get(9).and_then(|f| f.value()).unwrap_or("0");
    let sending_app = msh.fields.get(2).and_then(|f| f.value()).unwrap_or("");
    let sending_fac = msh.fields.get(3).and_then(|f| f.value()).unwrap_or("");
    let receiving_app = msh.fields.get(4).and_then(|f| f.value()).unwrap_or("");
    let receiving_fac = msh.fields.get(5).and_then(|f| f.value()).unwrap_or("");
    let version = msh.fields.get(11).and_then(|f| f.value()).unwrap_or("2.5");

    format!(
        "MSH|^~\\&|{}|{}|{}|{}|20240315143001||ACK|ACK{}|P|{}\rMSA|AA|{}\r",
        receiving_app,
        receiving_fac,
        sending_app,
        sending_fac,
        msg_control_id,
        version,
        msg_control_id
    )
}

fn bench_pipeline_interface_engine_sim(c: &mut Criterion) {
    let mut group = c.benchmark_group("Pipeline_Interface_Engine_Sim");

    group.bench_function("ADT_route_and_ack", |b| {
        b.iter(|| {
            // Parse
            let msg = parse_message(black_box(ADT_A01)).unwrap();
            // Extract routing fields
            let terser = Terser::new(&msg);
            let _ = terser.get("MSH-9").unwrap(); // message type
            let _ = terser.get("MSH-5").unwrap(); // receiving app
            let _ = terser.get("PID-3").unwrap(); // patient ID
            let _ = terser.get("PV1-2").unwrap(); // patient class
            // Generate ACK
            let ack = generate_ack(&msg);
            black_box(ack)
        });
    });

    group.bench_function("ORU_route_and_ack", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ORU_R01)).unwrap();
            let terser = Terser::new(&msg);
            let _ = terser.get("MSH-9").unwrap();
            let _ = terser.get("MSH-5").unwrap();
            let _ = terser.get("PID-3").unwrap();
            let _ = terser.get("OBR-4").unwrap(); // order type
            let ack = generate_ack(&msg);
            black_box(ack)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_pipeline_parse_only,
    bench_pipeline_parse_validate,
    bench_pipeline_parse_extract,
    bench_pipeline_parse_transform_encode,
    bench_pipeline_interface_engine_sim
);
criterion_main!(benches);
