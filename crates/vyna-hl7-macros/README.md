# vyna-hl7-macros

Procedural macros for HL7 message processing.

## Features

- `#[derive(Segment)]` - Type-safe segment definitions
- `#[derive(Message)]` - Complete message structures
- `#[hl7_type]` - Composite data type attribute macro

## Generated API

- `#[derive(Segment)]` generates `from_segment` / `to_segment` conversions
  plus `SEGMENT_ID`, guarded by 1-based `#[hl7(field = N)]` positions.
- `#[derive(Message)]` generates `from_message` / `to_message` conversions
  plus `MESSAGE_TYPE` / `TRIGGER_EVENT`, matching segments by ID.
- `#[hl7_type]` generates `from_repetition` / `to_repetition` conversions
  for composite data types such as XPN.

Use the `vyna-hl7-custom` crate's `z_segment!` declarative macro for custom
segment definitions.

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
