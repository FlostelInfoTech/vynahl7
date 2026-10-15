#![no_main]

use libfuzzer_sys::fuzz_target;

// Parse-then-encode round trip: encoding a successfully parsed
// message must never panic either.
fuzz_target!(|data: &[u8]| {
    if let Ok(msg) = vyna_hl7_parser::parse_message_bytes(data) {
        let _ = msg.encode();
    }
});
