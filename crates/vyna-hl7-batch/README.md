# vyna-hl7-batch

HL7 v2.x batch file building, lenient intake parsing, and file-drop
transports (filesystem, FTP, SFTP).

## Overview

`vyna-hl7-batch` adds the operational layer around the batch/file data
model in `vyna-hl7-core` and the strict batch parsing in
`vyna-hl7-parser`: build outbound batch files, parse inbound drops
leniently, and move files over drop folders, FTP, or SFTP.

## Features

- **BatchFileBuilder**: wrap messages in `FHS`/`BHS`/`BTS`/`FTS`
  envelopes with generated timestamps, control IDs, and counts
- **Intake**: lenient inbound parsing; auto-detects file, batch, or bare
  message sequences and reports per-message failures
- **FileTransport**: async file movement with a built-in
  `FilesystemTransport`, `FtpTransport` (`ftp` feature), and
  `SftpTransport` (`sftp` feature)
- **Poller**: poll an inbox, parse each file with `Intake`, and settle
  (archive or delete) what was consumed

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
vyna-hl7-batch = "0.9"
```

Enable network transports with the `ftp` and `sftp` features.

## Basic Usage

```rust,no_run
use vyna_hl7_batch::{BatchFileBuilder, FileTransport, FilesystemTransport, Poller};

# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
// Outbound: wrap messages and drop the file.
let file = BatchFileBuilder::new()
    .sending_application("SENDER")
    .message(vyna_hl7_parser::parse_message(
        "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|1|P|2.5",
    )?)
    .build_file();
let mut outbox = FilesystemTransport::open("/data/hl7/outbox")?;
outbox.put("adt.hl7", file.encode().as_bytes()).await?;

// Inbound: poll, parse leniently, archive what was consumed.
let inbox = FilesystemTransport::open("/data/hl7/inbox")?;
let mut poller = Poller::new(inbox).with_archive_dir("archive");
let report = poller.poll_once().await?;
for polled in &report.files {
    println!("{}: {} ok, {} failed", polled.name, polled.ok_count(), polled.error_count());
}
# Ok(())
# }
```

## Related Crates

- **vyna-hl7-core**: batch/file data model (`FHS`/`BHS` envelopes)
- **vyna-hl7-parser**: strict batch parsing (`parse_batch`, `parse_file`)
- **vyna-hl7-queue**: durable store-and-forward for single messages

## License

Copyright (c) 2024-2026 Alex SHAO Fan.

Licensed under the MIT License — see the [LICENSE](../../LICENSE) file.
