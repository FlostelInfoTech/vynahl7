# vyna-hl7-core

Core data structures and utilities for HL7 v2.x message processing in Rust.

## Overview

`vyna-hl7-core` is the foundational crate of the VynaHL7 library, providing the essential data structures, encoding/decoding functionality, and message builders for working with HL7 v2.x healthcare messages.

## Features

- **Core Data Structures**: Message, Segment, Field, Repetition, Component, Subcomponent
- **HL7 Encoding/Decoding**: Handle escape sequences and special characters
- **Message Builders**: Fluent API for creating HL7 messages programmatically
- **Batch/File Support**: BHS/BTS (batch) and FHS/FTS (file) message structures
- **Field Delimiters**: Configurable separators for segments, fields, and components
- **Efficient Design**: Optimized memory usage with minimal allocations

## Message Hierarchy

```
Message
  └─ Segment (MSH, PID, OBX, etc.)
       └─ Field (separated by |)
            └─ Repetition (separated by ~)
                 └─ Component (separated by ^)
                      └─ Subcomponent (separated by &)
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-core = "0.9"
```

## Basic Usage

### Creating a Message

```rust
use vyna_hl7_core::{Message, Segment, Field};

// Create a new message
let mut msg = Message::default();

// Create MSH segment
let mut msh = Segment::new("MSH");
msh.fields.push(Field::from_value("|"));         // Field separator
msh.fields.push(Field::from_value("^~\\&"));     // Encoding characters
msh.fields.push(Field::from_value("SENDING_APP"));
msh.fields.push(Field::from_value("SENDING_FAC"));
msg.segments.push(msh);

// Encode to HL7 string
let encoded = msg.encode();
println!("{}", encoded);
```

### Using Message Builders

```rust
use vyna_hl7_core::builders::adt::AdtA01Builder;

// Build an ADT^A01 message using fluent API
let msg = AdtA01Builder::new()
    .msh_sending_application("MY_APP")
    .msh_sending_facility("MY_FACILITY")
    .msh_message_control_id("MSG001")
    .pid_patient_id("12345", "MR", "HOSPITAL")
    .pid_patient_name("Doe", "John", "M")
    .pid_date_of_birth("19800115")
    .pid_sex("M")
    .pv1_patient_class("I")  // Inpatient
    .pv1_assigned_location("ICU^Room 5^Bed 1")
    .build();

println!("{}", msg.encode());
```

## Available Message Builders

### ADT (Admission/Discharge/Transfer)
- A01 - Admit/Visit Notification
- A02 - Transfer a Patient
- A03 - Discharge/End Visit
- A04 - Register a Patient
- A05 - Pre-admit a Patient
- A06 - Change Outpatient to Inpatient
- A07 - Change Inpatient to Outpatient
- A08 - Update Patient Information
- A09-A13 - Patient Tracking
- A17 - Swap Patients
- A28 - Add Person Information
- A31 - Update Person Information
- A40 - Merge Patient

### ORM/ORU (Orders and Results)
- ORM^O01 - General Order Message
- ORU^R01 - Unsolicited Observation Result

### SIU (Scheduling)
- SIU^S12 - Notification of New Appointment Booking

### MDM (Medical Document Management)
- MDM^T01/T02/T04 - Document notifications

### DFT (Financial Transactions)
- DFT^P03/P11 - Detailed Financial Transaction

### QBP/RSP (Query/Response)
- QBP^Q11/Q15/Q21/Q22 - Query by Parameter
- RSP^K11/K15/K21/K22 - Query Responses

### Pharmacy Messages
- RDE^O11 - Pharmacy/Treatment Encoded Order
- RAS^O17 - Pharmacy/Treatment Administration
- RDS^O13 - Pharmacy/Treatment Dispense
- RGV^O15 - Pharmacy/Treatment Give

### Laboratory Messages
- OUL^R21 - Unsolicited Laboratory Observation
- OML^O21 - Laboratory Order

### Batch/File Support
```rust
use vyna_hl7_core::builders::batch::{BatchBuilder, FileBuilder};

// Create a batch of messages
let batch = BatchBuilder::new()
    .sender_application("LAB_SYSTEM")
    .receiver_application("EMR")
    .add_message(msg1)
    .add_message(msg2)
    .build();

// Create a file containing multiple batches
let file = FileBuilder::new()
    .sender_application("HOSPITAL_A")
    .receiver_application("HOSPITAL_B")
    .add_batch(batch1)
    .add_batch(batch2)
    .build();
```

## Complex Field Builders

Build composite HL7 data types:

```rust
use vyna_hl7_core::builders::fields::{XpnBuilder, XadBuilder, XtnBuilder};

// Build patient name (XPN)
let name = XpnBuilder::new()
    .family_name("Doe")
    .given_name("John")
    .middle_name("M")
    .suffix("Jr")
    .build();

// Build address (XAD)
let address = XadBuilder::new()
    .street("123 Main St")
    .city("Springfield")
    .state("IL")
    .postal_code("62701")
    .country("USA")
    .build();

// Build phone number (XTN)
let phone = XtnBuilder::new()
    .phone_number("555-1234")
    .use_code("PRN")  // Primary
    .equipment_type("PH")  // Telephone
    .build();
```

## Data Structures

### Message
```rust
pub struct Message {
    pub segments: Vec<Segment>,
}
```

### Segment
```rust
pub struct Segment {
    pub id: String,
    pub fields: Vec<Field>,
}
```

### Field
```rust
pub struct Field {
    pub repetitions: Vec<Repetition>,
}
```

### Repetition
```rust
pub struct Repetition {
    pub components: Vec<Component>,
}
```

## Error Handling

```rust
use vyna_hl7_core::Error;

fn process_message(msg: &Message) -> Result<(), Error> {
    // Message processing logic
    Ok(())
}
```

## HL7 Encoding

```rust
use vyna_hl7_core::encoding::Encoding;
use vyna_hl7_core::delimiters::Delimiters;

let delimiters = Delimiters::default();

// Encode special characters
let encoded = Encoding::encode("Value with | separator", &delimiters);
// Result: "Value with \\F\\ separator"

// Decode escape sequences
let decoded = Encoding::decode("Value with \\F\\ separator", &delimiters).unwrap();
// Result: "Value with | separator"
```

## Batch and File Messages

### Batch Messages
```rust
use vyna_hl7_core::batch::{Batch, BatchHeader, BatchTrailer};

let batch = Batch {
    header: BatchHeader::new()
        .with_sender_application("APP")
        .with_receiver_application("EMR")
        .build(),
    messages: vec![msg1, msg2],
    trailer: BatchTrailer::new(2),  // 2 messages
};

let encoded = batch.encode("\r");  // Use \r for transmission
```

### File Messages
```rust
use vyna_hl7_core::batch::{File, FileHeader, FileTrailer};

let file = File {
    header: FileHeader::new()
        .with_sender_application("SYSTEM_A")
        .with_receiver_application("SYSTEM_B")
        .build(),
    batches: vec![batch1, batch2],
    trailer: FileTrailer::new(2),  // 2 batches
};

let encoded = file.encode("\r");
```

## Supported HL7 Versions

- HL7 v2.3
- HL7 v2.3.1
- HL7 v2.4
- HL7 v2.5
- HL7 v2.5.1
- HL7 v2.6
- HL7 v2.7
- HL7 v2.7.1
- HL7 v2.8
- HL7 v2.8.1
- HL7 v2.8.2

## Related Crates

- **vyna-hl7-parser**: Parse HL7 messages from strings
- **vyna-hl7-validator**: Validate messages against HL7 schemas
- **vyna-hl7-terser**: Access message fields using path notation (e.g., "PID-5-1")
- **vyna-hl7-mllp**: MLLP network protocol for message transmission
- **vyna-hl7-http**: HTTP transport for HL7 messages

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
