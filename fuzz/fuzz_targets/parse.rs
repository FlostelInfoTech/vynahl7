#![no_main]

use libfuzzer_sys::fuzz_target;

// The byte entry point must never panic or hang on any input:
// malformed bytes are an expected `Err`, never a crash.
fuzz_target!(|data: &[u8]| {
    let _ = vyna_hl7_parser::parse_message_bytes(data);
});
