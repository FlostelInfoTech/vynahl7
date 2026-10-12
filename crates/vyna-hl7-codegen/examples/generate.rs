//! Regenerate the checked-in typed-model fixtures.
//!
//! Run from anywhere in the workspace:
//! `cargo run -p vyna-hl7-codegen --example generate`
//!
//! The consistency test in `tests/codegen_tests.rs` compares generator output
//! against these files, so re-run this example after changing the generator.

use std::path::PathBuf;

use vyna_hl7_codegen::generate_for;
use vyna_hl7_core::Version;

fn main() {
    let fixtures = [(Version::V2_5, "ADT", "A01", "v2_5_adt_a01.rs")];
    let generated_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("generated");
    std::fs::create_dir_all(&generated_dir).expect("create generated dir");

    for (version, message_type, trigger_event, file) in fixtures {
        let source = generate_for(version, message_type, trigger_event)
            .expect("schema must exist for fixture");
        let path = generated_dir.join(file);
        std::fs::write(&path, &source).expect("write fixture");
        println!("wrote {} ({} bytes)", path.display(), source.len());
    }
}
