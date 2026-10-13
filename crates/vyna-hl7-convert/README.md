# vyna-hl7-convert

Schema-driven cross-version conversion for HL7 v2.x messages.

## Overview

`vyna-hl7-convert` rewrites a message from one HL7 version to another
using the validator's per-version schemas, reporting everything that
could not be carried over. Fields convert by structural rules, not
hand-written maps: same field numbers are copied, renamed fields are
matched by name, unknown segments pass through verbatim, and MSH-12 is
stamped with the target version.

## Features

- **convert**: convert a message between any two supported HL7 versions
- **ConversionReport**: audit-friendly record of dropped fields and
  segments for follow-up
- Z-segments and local extensions pass through untouched

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-convert = "0.9"
```

## Basic Usage

```rust
use vyna_hl7_convert::convert;
use vyna_hl7_core::Version;
use vyna_hl7_parser::parse_message;

let message = parse_message(
    "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7",
)
.unwrap();
let (converted, report) = convert(&message, Version::V2_5, Version::V2_3).unwrap();
assert!(report.dropped_fields.is_empty());
assert_eq!(
    converted.get_segment_by_id("MSH").unwrap().get_field_value(12),
    Some("2.3")
);
```

## Related Crates

- **vyna-hl7-validator**: per-version schemas driving the conversion
- **vyna-hl7-parser**: parsing messages before conversion
- **vyna-hl7-transform**: rule-based message transformation

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
