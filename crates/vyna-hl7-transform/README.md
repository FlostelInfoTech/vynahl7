# vyna-hl7-transform

Message transformation framework for HL7 v2.x messages.

## Overview

Transform HL7 messages using field mappings, transformation functions, and declarative configuration.

## Features

- Field-to-field mappings
- 15 built-in transformation functions (uppercase, lowercase, trim, date formatting, etc.)
- Custom transformation functions
- YAML/JSON configuration support
- Context data for parameterized transforms

## Installation

```toml
[dependencies]
vyna-hl7-transform = "0.19"
```

## Quick Example

```rust
use vyna_hl7_transform::MessageTransformer;

let transformer = MessageTransformer::new()
    .add_mapping("PID-5-1", "PID-5-1")  // Copy family name
    .add_transform("PID-5-2", "PID-5-2", uppercase)  // Uppercase given name
    .add_transform("PID-7", "PID-7", format_date);  // Reformat date

let transformed = transformer.transform(&message)?;
```

See [main README](../../README.md) for full documentation.

## License

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.
