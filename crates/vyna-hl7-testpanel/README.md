# vyna-hl7-testpanel

GUI Test Panel for the VynaHL7 HL7 library - parse, build, validate,
and exchange messages interactively.

## Overview

`vyna-hl7-testpanel` is a desktop GUI application (built with
[egui](https://www.egui.rs/)) demonstrating the capabilities of the
VynaHL7 workspace. Each tab exercises one or more library crates
against sample messages you can edit in place.

## Features

- **Parser**: parse and analyze HL7 v2.x messages with a tree view
- **Builder**: visually build HL7 messages using the fluent API
- **Terser**: access and modify fields using path notation
- **Validator**: validate messages against HL7 schemas
- **MLLP / HTTP / FTP**: send and receive messages over transports
- **FHIR / XML**: convert between HL7 v2.x, FHIR R4, and XML

## Running

```bash
cargo run -p vyna-hl7-testpanel
```

This crate is a binary application, not a library; it is published so
it can be installed with `cargo install vyna-hl7-testpanel`.

## Related Crates

- **vyna-hl7-cli**: the same workflows as command-line commands
- **vyna-hl7-parser**: message parsing shown in the Parser tab
- **vyna-hl7-validator**: schema validation shown in the Validator tab

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
