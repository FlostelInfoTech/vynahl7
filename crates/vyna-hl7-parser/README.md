# vyna-hl7-parser

High-performance HL7 v2.x message parser for Rust with delimiter-based parsing.

## Overview

`vyna-hl7-parser` provides fast, efficient parsing of HL7 v2.x messages into the vyna-hl7-core data structures. The parser uses delimiter-based splitting to achieve minimal allocations and maximum performance.

## Features

- **Efficient Parsing**: Minimizes memory allocations for optimal performance
- **Complete HL7 Support**: Parse all HL7 v2.x message types and versions
- **Escape Sequence Handling**: Proper handling of HL7 encoding characters
- **Batch/File Parsing**: Parse FHS/BHS file and batch messages
- **Error Handling**: Detailed error messages for malformed messages
- **High Performance**: 2-5 µs for small messages, 8-12 µs for medium messages
- **Optimized Parser**: Pre-allocation strategies for component-heavy messages

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-parser = "0.19"
```

## Basic Usage

### Parsing a Message

```rust
use vyna_hl7_parser::parse_message;

let hl7 = r"MSH|^~\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|20231010120000||ADT^A01|MSG001|P|2.5
PID|1||12345^^^HOSPITAL^MR||Doe^John^M||19800115|M|||123 Main St^^Springfield^IL^62701^USA
PV1|1|I|ICU^Room 5^Bed 1|||||||||||||||V123456";

match parse_message(hl7) {
    Ok(message) => {
        println!("Parsed {} segments", message.segments.len());
        for segment in &message.segments {
            println!("Segment: {}", segment.id);
        }
    }
    Err(e) => eprintln!("Parse error: {}", e),
}
```

### Accessing Parsed Data

```rust
use vyna_hl7_parser::parse_message;

let hl7 = "MSH|^~\\&|APP|FAC|EMR|HOSP|20231010120000||ADT^A01|MSG001|P|2.5\r\
           PID|1||12345^^^MR||Doe^John";

let msg = parse_message(hl7)?;

// Access MSH segment
let msh = &msg.segments[0];
assert_eq!(msh.id, "MSH");

// Access fields (note: MSH field indexing is special)
// MSH-3 = Sending Application = fields[2] (0-indexed after field separator)
if let Some(sending_app) = msh.fields.get(2) {
    println!("Sending Application: {}", sending_app.value().unwrap_or(""));
}

// Access PID segment
let pid = &msg.segments[1];
assert_eq!(pid.id, "PID");

// Access patient name (PID-5)
if let Some(name_field) = pid.fields.get(4) {  // PID-5 is at index 4
    // Patient name has components: Family^Given^Middle
    let components = &name_field.repetitions[0].components;
    if components.len() >= 2 {
        println!("Patient: {} {}",
            components[1].value,  // Given name
            components[0].value   // Family name
        );
    }
}
```

## Parsing Batch and File Messages

### Parse Batch Messages

```rust
use vyna_hl7_parser::parse_batch;

let batch_hl7 = r"BHS|^~\&|SENDING_APP|SENDING_FAC|RECEIVING_APP|RECEIVING_FAC|20231010120000|SECURITY|BATCH_NAME|BATCH_123|COMMENT
MSH|^~\&|APP1|FAC1|EMR|HOSP|20231010120001||ADT^A01|MSG001|P|2.5
PID|1||12345^^^MR||Doe^John
MSH|^~\&|APP1|FAC1|EMR|HOSP|20231010120002||ADT^A01|MSG002|P|2.5
PID|1||67890^^^MR||Smith^Jane
BTS|2|COMMENT";

let batch = parse_batch(batch_hl7)?;
println!("Batch contains {} messages", batch.messages.len());
println!("Sender: {}", batch.header.sender_application.as_deref().unwrap_or(""));
```

### Parse File Messages

```rust
use vyna_hl7_parser::parse_file;

let file_hl7 = r"FHS|^~\&|SYSTEM_A|FAC_A|SYSTEM_B|FAC_B|20231010120000|||FILE_123
BHS|^~\&|APP1|FAC1|APP2|FAC2|20231010120001|||BATCH_001
MSH|^~\&|APP1|FAC1|APP2|FAC2|20231010120002||ADT^A01|MSG001|P|2.5
PID|1||12345^^^MR||Doe^John
BTS|1
FTS|1";

let file = parse_file(file_hl7)?;
println!("File contains {} batches", file.batches.len());
println!("Total messages: {}",
    file.batches.iter().map(|b| b.messages.len()).sum::<usize>());
```

## Performance Optimization

The parser includes optimizations for high-throughput scenarios:

```rust
use vyna_hl7_parser::optimized::{parse_field_optimized, parse_repetition_optimized};

// Use optimized parsers for component-heavy messages
// These pre-allocate based on delimiter counts
// 10-30% faster for messages with many components
```

## Delimiter Extraction

```rust
use vyna_hl7_parser::{extract_delimiters, extract_delimiters_from_bhs, extract_delimiters_from_fhs};

// Extract delimiters from MSH segment
let delimiters = extract_delimiters("MSH|^~\\&|APP|FAC...")?;
println!("Field separator: {}", delimiters.field);
println!("Component separator: {}", delimiters.component);
println!("Repetition separator: {}", delimiters.repetition);
println!("Escape character: {}", delimiters.escape);
println!("Subcomponent separator: {}", delimiters.subcomponent);

// Extract from batch header (BHS)
let batch_delimiters = extract_delimiters_from_bhs("BHS|^~\\&|...")?;

// Extract from file header (FHS)
let file_delimiters = extract_delimiters_from_fhs("FHS|^~\\&|...")?;
```

## Escape Sequence Handling

The parser automatically handles HL7 escape sequences:

```rust
use vyna_hl7_parser::parse_message;

let hl7 = r"MSH|^~\&|APP|FAC|EMR|HOSP|20231010120000||ADT^A01|MSG001|P|2.5
PID|1||12345||Doe\\F\\John";  // \F\ represents | in the name

let msg = parse_message(hl7)?;
let pid = &msg.segments[1];

// The parser decodes escape sequences
// Field value will be "Doe|John" (decoded)
```

Supported escape sequences:
- `\F\` - Field separator (|)
- `\S\` - Component separator (^)
- `\T\` - Subcomponent separator (&)
- `\R\` - Repetition separator (~)
- `\E\` - Escape character (\)
- `\Xhhhh\` - Hexadecimal character codes

## Error Handling

```rust
use vyna_hl7_parser::{parse_message, Error};

fn process_hl7(hl7_str: &str) -> Result<(), Error> {
    let message = parse_message(hl7_str)?;

    // Process message
    println!("Message type: {}", message.message_type().unwrap_or("Unknown"));

    Ok(())
}

// Handle errors
match process_hl7(invalid_hl7) {
    Ok(_) => println!("Success"),
    Err(Error::Parse(msg)) => eprintln!("Parse error: {}", msg),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

## Message Line Endings

The parser accepts both `\r` (HL7 standard) and `\n` (common in text files):

```rust
// HL7 standard format (with \r)
let hl7_cr = "MSH|^~\\&|...\rPID|...\rPV1|...";
let msg1 = parse_message(hl7_cr)?;

// Unix format (with \n)
let hl7_lf = "MSH|^~\\&|...\nPID|...\nPV1|...";
let msg2 = parse_message(hl7_lf)?;

// Mixed format (also supported)
let hl7_mixed = "MSH|^~\\&|...\r\nPID|...\nPV1|...";
let msg3 = parse_message(hl7_mixed)?;
```

## Performance Characteristics

Based on benchmarks with `cargo bench`:

| Message Size | Parse Time | Throughput |
|--------------|-----------|------------|
| Small (3 segments) | 2-5 µs | ~200,000 msg/sec |
| Medium (8 segments) | 8-12 µs | ~80,000 msg/sec |
| Large (25+ segments) | 30-50 µs | ~30,000 msg/sec |

Optimizations applied:
- Fast-path escape detection (skip decode for values without escape sequences)
- Pre-allocation for repeated structures
- Fast path for fields without escape sequences
- Minimal allocations during parsing

## Parser Architecture

The parser uses delimiter-based splitting for robust parsing:

1. **Delimiter Extraction**: Parse MSH/BHS/FHS to get field separators
2. **Segment Parsing**: Split message into segments
3. **Field Parsing**: Parse each field into repetitions and components
4. **Escape Decoding**: Decode HL7 escape sequences
5. **Structure Building**: Construct Message with parsed data

## Testing

Run parser tests:

```bash
cargo test -p vyna-hl7-parser
```

Run performance benchmarks:

```bash
cargo bench -p vyna-hl7-parser
```

## Related Crates

- **vyna-hl7-core**: Core data structures for parsed messages
- **vyna-hl7-validator**: Validate parsed messages against HL7 schemas
- **vyna-hl7-terser**: Query parsed messages using path notation
- **vyna-hl7-transform**: Transform parsed messages

## License

Copyright (c) 2024-2026 Alex SHAO Fan <alex.shaofan@flostel.com>. All rights reserved.

This software is proprietary and confidential. Unauthorized copying, distribution, or use is strictly prohibited.

at your option.
