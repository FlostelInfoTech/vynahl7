# vyna-hl7-orchestration

Message routing and workflow orchestration for HL7 v2.x messages.

## Features

- Content-based routing with predicates
- Multi-step async workflows
- Message filtering (ALL/ANY modes)
- Retry logic with linear backoff
- Error handling and recovery

## Installation

```toml
[dependencies]
vyna-hl7-orchestration = "0.9"
```

## Quick Example

```rust
use vyna_hl7_core::Message;
use vyna_hl7_orchestration::{
    filtering::MessageFilter, orchestration::MessageOrchestrator, routing::ContentRouter,
};
use vyna_hl7_terser::Terser;

# async fn example(message: Message) -> Result<(), Box<dyn std::error::Error>> {
// Route messages based on content
let mut router = ContentRouter::new();
router.add_route(
    "adt_messages",
    |msg| {
        let terser = Terser::new(msg);
        terser.get("MSH-9-1").ok().flatten().as_deref() == Some("ADT")
    },
    |msg| async move {
        println!("Processing ADT message");
        Ok(msg)
    },
);

// Execute multi-step workflow
let mut orchestrator = MessageOrchestrator::new();
orchestrator
    .add_step("validate", |msg| async move { Ok(msg) })
    .add_step("enrich", |msg| async move { Ok(msg) })
    .add_step("persist", |msg| async move { Ok(msg) });

let processed = orchestrator.execute(message).await?;
let _ = router.route(processed).await?;

// Filter messages with predicate rules
let mut filter = MessageFilter::new();
filter.add_rule("production_only", |msg| {
    let terser = Terser::new(msg);
    terser.get("MSH-11").ok().flatten().as_deref() == Some("P")
});
# Ok(())
# }
```

See [main README](../../README.md) for full documentation.

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
