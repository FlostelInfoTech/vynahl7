# vyna-hl7-orchestration

Message routing and workflow orchestration for HL7 v2.x messages.

## Features

- Content-based routing with predicates
- Multi-step async workflows
- Message filtering (ALL/ANY modes)
- Retry logic with exponential backoff
- Error handling and recovery

## Installation

```toml
[dependencies]
vyna-hl7-orchestration = "0.19"
```

## Quick Example

```rust
use vyna_hl7_orchestration::{ContentRouter, MessageOrchestrator};

// Route messages based on content
let mut router = ContentRouter::new();
router.add_route("adt_messages", |msg| {
    msg.message_type() == Some("ADT^A01")
}, handle_adt);

// Execute multi-step workflow
let mut orchestrator = MessageOrchestrator::new();
orchestrator
    .add_step("validate", validate_message)
    .add_step("enrich", enrich_data)
    .add_step("persist", save_to_db);

orchestrator.execute(&message).await?;
```

See [main README](../../README.md) for full documentation.

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
