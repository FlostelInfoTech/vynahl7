# vyna-hl7-macros

Procedural macros for HL7 message processing.

## Features

- `#[derive(Segment)]` - Type-safe segment definitions
- `#[derive(Message)]` - Complete message structures
- `#[hl7_type]` - Composite data type attribute macro

## Current Status

`#[derive(Segment)]` and `#[derive(Message)]` generate `from_segment`/`to_segment` conversions. Use the `vyna-hl7-custom` crate's `z_segment!` declarative macro for custom segment definitions.

## License

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.
