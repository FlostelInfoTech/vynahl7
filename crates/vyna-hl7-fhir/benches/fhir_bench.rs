//! FHIR conversion benchmarks: individual converters, JSON serialization,
//! and full parse-to-FHIR pipeline.

use criterion::{criterion_group, criterion_main, Criterion};
use vyna_hl7_fhir::converters::{
    AllergyIntoleranceConverter, ConditionConverter, DiagnosticReportConverter,
    EncounterConverter, ObservationConverter, PatientConverter, PractitionerConverter,
    ProcedureConverter,
};
use vyna_hl7_parser::parse_message;
use std::hint::black_box;

// ORU with OBR + multiple OBX for observation/diagnostic report conversion
const ORU_R01: &str = "\
MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20240315143000||ORU^R01^ORU_R01|MSG00002|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|O|ER^201^B^HOSPITAL||||1234567^SMITH^JANE^M^MD\r\
ORC|RE|ORD123456|LAB789012||CM||||20240315120000|||1234567^SMITH^JANE^M^MD\r\
OBR|1|ORD123456|LAB789012|CBC^Complete Blood Count^LN|||20240315110000|||||||||1234567^SMITH^JANE^M^MD||||||20240315120000|||F\r\
OBX|1|NM|WBC^White Blood Cell Count^LN||7.5|10*9/L|4.5-11.0|N|||F|||20240315115500\r\
OBX|2|NM|RBC^Red Blood Cell Count^LN||4.8|10*12/L|4.2-5.9|N|||F|||20240315115500\r\
OBX|3|NM|HGB^Hemoglobin^LN||14.5|g/dL|12.0-16.0|N|||F|||20240315115500\r\
OBX|4|NM|PLT^Platelet Count^LN||250|10*9/L|150-400|N|||F|||20240315115500";

// ADT with allergy, diagnosis, and procedure segments
const ADT_FULL: &str = "\
MSH|^~\\&|HIS|HOSPITAL|EMR|HOSPITAL|20240315143000||ADT^A01^ADT_A01|MSG00001|P|2.5|||AL|NE\r\
PID|1||MRN12345^^^HOSPITAL^MR||DOE^JOHN^ALLEN||19800315|M|||123 MAIN ST^^BOSTON^MA^02101^USA||^PRN^PH^^1^617^5551234\r\
PV1|1|I|ICU^101^A^HOSPITAL||||1234567^SMITH^JANE^M^MD^^MD\r\
AL1|1|DA|PCN^Penicillin^L|SV^Severe|Anaphylaxis|20200101\r\
DG1|1|ICD10|I21.0^Acute transmural myocardial infarction of anterior wall^ICD10||20240315|AD\r\
PR1|1|CPT|93458^Cardiac Catheterization^CPT|Cardiac Cath|20240315140000|||||1234567^SMITH^JANE^M^MD";

// ============================================================================
// Individual FHIR Converters
// ============================================================================

fn bench_fhir_individual(c: &mut Criterion) {
    let mut group = c.benchmark_group("FHIR_Individual");

    let oru = parse_message(ORU_R01).unwrap();
    let adt = parse_message(ADT_FULL).unwrap();

    group.bench_function("PatientConverter", |b| {
        b.iter(|| black_box(PatientConverter::convert(black_box(&oru))));
    });

    group.bench_function("ObservationConverter_all", |b| {
        b.iter(|| black_box(ObservationConverter::convert_all(black_box(&oru))));
    });

    group.bench_function("ObservationConverter_single", |b| {
        b.iter(|| black_box(ObservationConverter::convert_single(black_box(&oru), 0)));
    });

    group.bench_function("EncounterConverter", |b| {
        b.iter(|| black_box(EncounterConverter::convert(black_box(&oru))));
    });

    group.bench_function("DiagnosticReportConverter", |b| {
        b.iter(|| black_box(DiagnosticReportConverter::convert_all(black_box(&oru))));
    });

    group.bench_function("PractitionerConverter", |b| {
        b.iter(|| black_box(PractitionerConverter::convert_attending_doctor(black_box(&oru))));
    });

    group.bench_function("AllergyIntoleranceConverter", |b| {
        b.iter(|| black_box(AllergyIntoleranceConverter::convert_all(black_box(&adt))));
    });

    group.bench_function("ConditionConverter", |b| {
        b.iter(|| black_box(ConditionConverter::convert_all(black_box(&adt))));
    });

    group.bench_function("ProcedureConverter", |b| {
        b.iter(|| black_box(ProcedureConverter::convert_all(black_box(&adt))));
    });

    group.finish();
}

// ============================================================================
// FHIR + JSON Serialization
// ============================================================================

fn bench_fhir_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("FHIR_Serialization");

    let oru = parse_message(ORU_R01).unwrap();

    // Patient → JSON
    if let Ok(patient) = PatientConverter::convert(&oru) {
        group.bench_function("Patient_to_json", |b| {
            b.iter(|| black_box(serde_json::to_string(black_box(&patient))));
        });
    }

    // Observations → JSON
    if let Ok(observations) = ObservationConverter::convert_all(&oru) {
        group.bench_function("Observations_to_json", |b| {
            b.iter(|| black_box(serde_json::to_string(black_box(&observations))));
        });
    }

    // Encounter → JSON
    if let Ok(encounter) = EncounterConverter::convert(&oru) {
        group.bench_function("Encounter_to_json", |b| {
            b.iter(|| black_box(serde_json::to_string(black_box(&encounter))));
        });
    }

    group.finish();
}

// ============================================================================
// Full Pipeline: Parse HL7 → Convert → Serialize JSON
// ============================================================================

fn bench_fhir_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("FHIR_Full_Pipeline");

    group.bench_function("ORU_parse_convert_all_serialize", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ORU_R01)).unwrap();

            let patient = PatientConverter::convert(&msg).ok();
            let observations = ObservationConverter::convert_all(&msg).ok();
            let encounter = EncounterConverter::convert(&msg).ok();
            let reports = DiagnosticReportConverter::convert_all(&msg).ok();
            let practitioner = PractitionerConverter::convert_attending_doctor(&msg).ok();

            // Serialize everything
            let _ = black_box(serde_json::to_string(&patient));
            let _ = black_box(serde_json::to_string(&observations));
            let _ = black_box(serde_json::to_string(&encounter));
            let _ = black_box(serde_json::to_string(&reports));
            let _ = black_box(serde_json::to_string(&practitioner));
        });
    });

    group.bench_function("ADT_parse_convert_all_serialize", |b| {
        b.iter(|| {
            let msg = parse_message(black_box(ADT_FULL)).unwrap();

            let patient = PatientConverter::convert(&msg).ok();
            let encounter = EncounterConverter::convert(&msg).ok();
            let allergy = AllergyIntoleranceConverter::convert_all(&msg).ok();
            let condition = ConditionConverter::convert_all(&msg).ok();
            let procedure = ProcedureConverter::convert_all(&msg).ok();

            let _ = black_box(serde_json::to_string(&patient));
            let _ = black_box(serde_json::to_string(&encounter));
            let _ = black_box(serde_json::to_string(&allergy));
            let _ = black_box(serde_json::to_string(&condition));
            let _ = black_box(serde_json::to_string(&procedure));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fhir_individual,
    bench_fhir_serialization,
    bench_fhir_full_pipeline
);
criterion_main!(benches);
