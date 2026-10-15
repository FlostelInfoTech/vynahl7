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
    adt::RichAdtA01Builder,
    billing::RichBarP01Builder,
    dft::RichDftP03Builder,
    laboratory::{LabOrder, RichOmgO19Builder, RichOmiO23Builder, RichOmlO21Builder},
    mdm::RichMdmT02Builder,
    oru::{Observation, RichOruR01Builder},
    pharmacy::{PharmacyPrescription, RichOmpO09Builder},
    referral::{ReferralInfo, RichRefI12Builder},
    siu::RichSiuS12Builder,
};

fn demographics() -> PatientDemographics {
    PatientDemographics {
        patient_id_flat: Some("P0001".to_string()),
        name: Some(("Tan".to_string(), "Wei Ming".to_string())),
        race: vec![CodedValue::text("Chinese")],
        ..Default::default()
    }
}

fn msh_line(msg: &vyna_hl7_core::Message) -> String {
    msg.encode().split('\r').next().unwrap().to_string()
}

fn msh9(msg: &vyna_hl7_core::Message) -> String {
    msh_line(msg).split('|').nth(8).unwrap().to_string()
}

fn msh18(msg: &vyna_hl7_core::Message) -> String {
    msh_line(msg).split('|').nth(17).unwrap().to_string()
}

#[test]
fn rich_adt_emits_trigger_and_race() {
    let msg = RichAdtA01Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    let er7 = msg.encode();
    assert!(msh9(&msg).starts_with("ADT^A01"), "{}", msh9(&msg));
    assert!(er7.contains("|Chinese|"), "{er7}");
}

#[test]
fn rich_siu_builds() {
    let msg = RichSiuS12Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("SIU^S12"), "{}", msh9(&msg));
}

#[test]
fn rich_dft_builds() {
    let msg = RichDftP03Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("DFT^P03"), "{}", msh9(&msg));
}

#[test]
fn rich_oru_emits_observation() {
    let msg = RichOruR01Builder::new(Version::V2_5)
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
    let msg = RichMdmT02Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("MDM^T02"), "{}", msh9(&msg));
}

#[test]
fn rich_omp_builds() {
    let msg = RichOmpO09Builder::new(Version::V2_5)
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
    let msg = RichOmlO21Builder::new(Version::V2_5)
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
    let msg = RichBarP01Builder::new(Version::V2_5)
        .demographics(demographics())
        .build()
        .unwrap();
    assert!(msh9(&msg).starts_with("BAR^P01"), "{}", msh9(&msg));
}

#[test]
fn rich_ref_builds() {
    let msg = RichRefI12Builder::new(Version::V2_5)
        .demographics(demographics())
        .referral(ReferralInfo {
            status: "A".to_string(),
            priority: "R".to_string(),
            referral_type: "Consult".to_string(),
            referral_id: "REF1".to_string(),
            reason: "Cardiology opinion".to_string(),
        })
        .build()
        .unwrap();
    // No REF schema exists, so this layer — not the conformance suite —
    // pins the trigger triple, the charset declaration, and RF1.
    assert_eq!(msh9(&msg), "REF^I12^REF_I12");
    assert_eq!(msh18(&msg), "UNICODE UTF-8");
    let wire = msg.encode();
    let rf1 = wire.split('\r').find(|l| l.starts_with("RF1")).unwrap();
    assert!(rf1.contains("Cardiology opinion"), "{rf1}");
}

#[test]
fn rich_rri_response_builds() {
    let msg = RichRefI12Builder::new(Version::V2_5)
        .demographics(demographics())
        .as_rri_response()
        .build()
        .unwrap();
    assert_eq!(msh9(&msg), "RRI^I12^RRI_I12");
    assert_eq!(msh18(&msg), "UNICODE UTF-8");
}

#[test]
fn rich_omg_builds() {
    let msg = RichOmgO19Builder::new(Version::V2_5)
        .demographics(demographics())
        .order(LabOrder {
            order_number: "ORD1".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();
    assert_eq!(msh9(&msg), "OMG^O19^OMG_O19");
    assert_eq!(msh18(&msg), "UNICODE UTF-8");
}

#[test]
fn rich_omi_builds() {
    let msg = RichOmiO23Builder::new(Version::V2_5)
        .demographics(demographics())
        .order(LabOrder {
            order_number: "ORD1".to_string(),
            ..Default::default()
        })
        .build()
        .unwrap();
    assert_eq!(msh9(&msg), "OMI^O23^OMI_O23");
    assert_eq!(msh18(&msg), "UNICODE UTF-8");
}
