# vyna-hl7-xml

XML encoding and decoding for HL7 v2.x messages.

## Overview

`vyna-hl7-xml` encodes `vyna_hl7_core::Message` to the HL7 XML Encoding
Rules format and decodes it back, including bidirectional conversion
between the ER7 pipe-delimited format and XML.

## Features

- **XmlEncoder**: render a message as HL7 v2 XML (`urn:hl7-org:v2xml`)
- **XmlDecoder**: parse HL7 v2 XML back into a `Message`
- **er7_to_xml / xml_to_er7**: one-call ER7 <-> XML conversion
  (requires the `convert` feature)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-xml = "0.9"
```

## Basic Usage

```rust
use vyna_hl7_core::message::Message;
use vyna_hl7_xml::{XmlDecoder, XmlEncoder};

let message = Message::new();
let encoder = XmlEncoder::new();
let xml = encoder.encode(&message).unwrap();

let decoder = XmlDecoder::new();
let decoded = decoder.decode(&xml).unwrap();
assert_eq!(decoded.segments.len(), message.segments.len());
```

With the `convert` feature, convert ER7 directly:

```rust,ignore
let xml = vyna_hl7_xml::er7_to_xml("MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5").unwrap();
let er7 = vyna_hl7_xml::xml_to_er7(&xml).unwrap();
```

## Related Crates

- **vyna-hl7-core**: the `Message` model being encoded
- **vyna-hl7-parser**: ER7 parsing used by the `convert` feature
- **vyna-hl7-fhir**: conversion to FHIR R4 resources

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
