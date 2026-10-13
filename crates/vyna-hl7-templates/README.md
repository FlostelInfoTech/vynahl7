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

let mut engine = TemplateEngine::new();
engine.set_variable("sending_app", "MyApp");
engine.set_variable("sending_facility", "MyHospital");
engine.set_variable("receiving_app", "CentralSystem");
engine.set_variable("receiving_facility", "DataCenter");
engine.set_variable("event_datetime", "20250120120000");
engine.set_variable("patient_id", "12345");
engine.set_variable("patient_name", "Doe^John");
engine.set_variable("date_of_birth", "19900101");
engine.set_variable("sex", "M");
engine.set_variable("address", "123 Main St^^Springfield^IL^62701");
engine.set_variable("patient_class", "I");
engine.set_variable("assigned_location", "4N^401");
engine.set_variable("attending_doctor", "12345^Smith^Jane");
engine.set_variable("visit_number", "V123456");

let message = engine.create_message(template)?;
```

See [main README](../../README.md) for full documentation.

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
