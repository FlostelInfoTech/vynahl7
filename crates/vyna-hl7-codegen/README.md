# vyna-hl7-codegen

Schema-driven code generation for HAPI-style typed HL7 models.

## Overview

`vyna-hl7-codegen` renders the JSON message schemas embedded in
`vyna-hl7-validator` as Rust structs annotated with the
`vyna-hl7-macros` derive macros, giving every message a strongly-typed
API in the spirit of HAPI's generated model classes.

## Features

- One struct per segment (`PID` -> `Pid`) and one per message
  (`ADT` + `A01` -> `AdtA01`) using the `Segment` / `Message` derives
- Required fields become `String`, optional fields `Option<String>`,
  repeating fields `Vec<String>`
- Deterministic segment order with `MSH` first so generated
  `to_message` output stays valid

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-codegen = "0.8"
```

## Basic Usage

```rust
use vyna_hl7_codegen::generate_for;
use vyna_hl7_core::Version;

let source = generate_for(Version::V2_5, "ADT", "A01").unwrap();
assert!(source.contains("pub struct AdtA01"));
assert!(source.contains("pub struct Pid"));
```

## Related Crates

- **vyna-hl7-validator**: message schemas codegen renders
- **vyna-hl7-macros**: `Segment` / `Message` derive macros
- **vyna-hl7-custom**: custom Z-segment definitions

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
