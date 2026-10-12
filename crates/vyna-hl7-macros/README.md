# vyna-hl7-macros

Procedural macros for HL7 message processing (placeholder for future implementation).

## Status

This crate is reserved for future procedural macro implementations. Currently contains placeholder derive macros.

## Planned Features (Phase 5)

- `#[derive(Segment)]` - Type-safe segment definitions
- `#[derive(Message)]` - Complete message structures
- `#[hl7_type]` - Composite data type macros

## Current Status

All macros return empty token streams. Use the `vyna-hl7-custom` crate's `z_segment!` declarative macro for custom segment definitions in the meantime.

See [ROADMAP.md](../../ROADMAP.md) for planned enhancements.

## License

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.
