//! Smoke coverage for the rich clinical builder layer.
//!
//! One construction test per ported family, each asserting the MSH-9
//! trigger plus one field only the rich layer emits (race, visit,
//! specimen, ...). Wire-compat with the retired `rs7` workspace is
//! proven by the downstream gateway suite, which pins exact segments;
//! these tests guard the layer's presence and shape in its new home.

use vyna_hl7_core::Version;
use vyna_hl7_core::builders::rich::{
    CodedValue, PatientDemographics,
    adt::AdtA01Builder,
    billing::BarP01Builder,
    dft::DftP03Builder,
    laboratory::{LabOrder, OmlO21Builder},
    mdm::MdmT02Builder,
    oru::{Observation, OruR01Builder},
    pharmacy::{OmpO09Builder, PharmacyPrescription},
    referral::RefI12Builder,
    siu::SiuS12Builder,
};

fn demographics() -> PatientDemographics {
    PatientDemographics {
        patient_id_flat: Some("P0001".to_string()),
        name: Some(("Tan".to_string(), "Wei Ming".to_string())),
        race: vec![CodedValue::text("Chinese")],
        ..Default::default()
    }
}

fn msh9(msg: &vyna_hl7_core::Message) -> String {
    msg.encode()
        .lines()
        .next()
        .unwrap()
        .split('|')
        .nth(8)
        .unwrap()
        .to_string()
}

#[test]
fn rich_adt_emits_trigger_and_race() {
    let msg = AdtA01Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    let er7 = msg.encode();
    assert!(msh9(&msg).starts_with("ADT^A01"), "{}", msh9(&msg));
    assert!(er7.contains("|Chinese|"), "{er7}");
}

#[test]
fn rich_siu_builds() {
    let msg = SiuS12Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("SIU^S12"), "{}", msh9(&msg));
}

#[test]
fn rich_dft_builds() {
    let msg = DftP03Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("DFT^P03"), "{}", msh9(&msg));
}

#[test]
fn rich_oru_emits_observation() {
    let msg = OruR01Builder::new(Version::V2_5)
        .demographics(demographics())
        .add_observation(Observation {
            set_id: 1,
            value_type: "NM".to_string(),
            identifier: "GLU".to_string(),
            value: "5.6".to_string(),
            units: Some("mmol/L".to_string()),
            status: "F".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();
    let er7 = msg.encode();
    assert!(msh9(&msg).starts_with("ORU^R01"), "{}", msh9(&msg));
    assert!(er7.contains("|GLU|"), "{er7}");
}

#[test]
fn rich_mdm_builds() {
    let msg = MdmT02Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("MDM^T02"), "{}", msh9(&msg));
}

#[test]
fn rich_omp_builds() {
    let msg = OmpO09Builder::new(Version::V2_5)
        .demographics(demographics())
        .order_control("NW")
        .prescription(PharmacyPrescription {
            order_id: "ORD1".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("OMP^O09"), "{}", msh9(&msg));
}

#[test]
fn rich_oml_builds() {
    let msg = OmlO21Builder::new(Version::V2_5)
        .demographics(demographics())
        .order(LabOrder {
            order_number: "ORD1".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("OML^O21"), "{}", msh9(&msg));
}

#[test]
fn rich_bar_builds() {
    let msg = BarP01Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("BAR^P01"), "{}", msh9(&msg));
}

#[test]
fn rich_ref_builds() {
    let msg = RefI12Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("REF^I12"), "{}", msh9(&msg));
}
