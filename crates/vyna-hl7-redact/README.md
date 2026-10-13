# vyna-hl7-redact

Safe Harbor de-identification for HL7 v2.x messages.

## Overview

`vyna-hl7-redact` rewrites protected health information (PHI) in place
using explicit, auditable rules. `Redactor::safe_harbor` ships a
profile covering the HIPAA Safe Harbor identifiers (names, addresses,
telecom, dates, IDs, account and visit numbers) across 34 segments:
ACC, AL1, BLG, CTD, DG1, EVN, FT1, GT1, IN1, IN2, IN3, MRG, MSH, NK1,
OBR, OBX, ORC, PD1, PDA, PID, PR1, PRB, PRD, PV1, ROL, RXA, RXD, RXE,
RXO, SCH, SPM, STF, TQ1, TXA.

Not covered: free-text notes (NTE, OBX-5), site-defined Z-segments,
and segments outside the list above. Add explicit `Rule`s when your
policy requires them.

## Features

- **safe_harbor**: ready-made HIPAA Safe Harbor redaction profile
- **Strategy::Hash**: 12 hex characters of HMAC-SHA256, preserving
  linkage across messages with the same key
- **Strategy::DateShift**: shifts dates while preserving time-of-day;
  unparseable values are removed rather than left in place
- Rules apply to every matching repetition and never create structure

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-redact = "0.9"
```

## Basic Usage

```rust
use vyna_hl7_parser::parse_message;
use vyna_hl7_redact::Redactor;

let mut message = parse_message(
    "MSH|^~\\&|A|B|C|D|20240101120000||ADT^A01|1|P|2.5\rPID|1||7||Smith^John||19800101|M",
)
.unwrap();
let redactor = Redactor::safe_harbor(b"site-secret", 30);
let report = redactor.redact(&mut message).unwrap();
assert!(!report.is_empty());
let pid = message.get_segment_by_id("PID").unwrap();
assert!(pid.get_field_value(5).is_none()); // name gone
assert_eq!(pid.get_field_value(7), Some("19800131")); // DOB shifted +30d
```

## Related Crates

- **vyna-hl7-parser**: parsing messages before redaction
- **vyna-hl7-validator**: validating messages after redaction
- **vyna-hl7-transform**: rule-based message transformation

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
