# vyna-hl7-queue

Durable store-and-forward queue for HL7 v2.x messages.

## Overview

`vyna-hl7-queue` gives outbound delivery somewhere to wait out
outages. `MemoryQueue` keeps messages in process memory while
`FileQueue` journals them to disk, surviving restarts and crashes. Both
share the `MessageQueue` contract: lease-based delivery with
exponential-backoff retries and a dead-letter bin after
`RetryPolicy::max_attempts` deliveries.

## Features

- **MemoryQueue**: in-process queue for tests and ephemeral workloads
- **FileQueue**: crash-safe journaling via atomic renames with fsync
- Lease-based delivery: unacknowledged messages return to pending when
  the lease expires
- Dead-letter bin for messages exhausting their retry budget

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-queue = "0.9"
```

## Basic Usage

```rust
use std::collections::HashMap;
use vyna_hl7_queue::{MemoryQueue, MessageQueue};

let mut queue = MemoryQueue::new();
let id = queue.enqueue(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5", HashMap::new()).unwrap();
let leased = queue.dequeue().unwrap().unwrap();
assert_eq!(leased.id, id);
queue.ack(&id).unwrap();
assert!(queue.dequeue().unwrap().is_none());
```

## Related Crates

- **vyna-hl7-mllp**: network delivery for dequeued messages
- **vyna-hl7-batch**: file-drop exchange for batched messages
- **vyna-hl7-orchestration**: routing and workflow engine

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
