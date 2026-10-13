# vyna-hl7-templates

Template system for creating and validating HL7 messages from reusable patterns.

## Features

- Reusable message templates with variable placeholders
- Template inheritance and resolution
- YAML/JSON configuration
- Standard template library for common messages
- Template validation

## Installation

```toml
[dependencies]
vyna-hl7-templates = "0.9"
```

## Quick Example

```rust
use vyna_hl7_templates::{TemplateEngine, TemplateLibrary};

let library = TemplateLibrary::new();
let template = library.get("ADT_A01").unwrap();

let engine = TemplateEngine::new()
    .set_variable("patient_id", "12345")
    .set_variable("patient_name", "Doe^John");

let message = engine.create_message(template)?;
```

See [main README](../../README.md) for full documentation.

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
