# vyna-hl7-conformance

Conformance profile validation for HL7 v2.x messages.

## Features

- XML conformance profile parsing
- Usage validation (R, RE, O, X, C)
- Cardinality validation
- Length constraints
- Conditional predicates
- Detailed validation reports

## Installation

```toml
[dependencies]
vyna-hl7-conformance = "0.19"
```

## Quick Example

```rust
use vyna_hl7_conformance::{ProfileParser, ConformanceValidator};

let profile = ProfileParser::from_file("adt_a01_profile.xml")?;
let validator = ConformanceValidator::new(profile);
let result = validator.validate(&message)?;

if !result.is_valid() {
    for error in result.errors {
        eprintln!("{}: {}", error.location, error.message);
    }
}
```

See [main README](../../README.md) for full documentation.

## License

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.
