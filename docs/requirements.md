# Software Requirements Specification — VynaHL7

Version: 0.9.0 (this document tracks the workspace; update per
[docs rules](README.md)). VynaHL7 is a Rust workspace (23 crates) for
parsing, validating, building, converting, and transporting HL7 v2.x
healthcare messages.

Requirements are numbered `REQ-<AREA>-NNN`. Each carries a verification
pointer to the suite that proves it. Roadmap items are marked
**Roadmap** instead of being omitted.

## 1. Parsing (vyna-hl7-parser)

- REQ-PARSE-010: The parser shall parse ER7-encoded HL7 v2.x messages
  (MSH-12 v2.1–v2.8.2) into the core message model. Verify: parser unit
  tests, workspace suite.
- REQ-PARSE-020: The parser shall support strict (default, spec-compliant)
  and lenient (tolerant of real-world deviations) modes via `ParserConfig`.
  Verify: parser mode tests.
- REQ-PARSE-030: The parser shall provide streaming parsing for large
  inputs (`streaming` module). Verify: parser streaming tests.
- REQ-PARSE-040: Parsing shall never validate: the parser depends only on
  the core model, and validation failures never block parsing or encoding.
  Verify: dependency direction (`parser/Cargo.toml`), CLI behavior.

## 2. Core model and builders (vyna-hl7-core)

- REQ-CORE-010: The model shall represent Message → Segment → Field →
  Repetition → Component → Subcomponent, with MSH field numbering offset
  (MSH-1 is the separator itself, MSH-2 the encoding characters).
  Verify: core model tests.
- REQ-CORE-020: Encoding shall round-trip: parsing encoded output yields
  an equal model, with delimiter characters in literal values
  escape-encoded (`\F\`, `\S\`, `\T\`, `\R\`, `\E\`). Verify: core
  encode tests, builder round-trip conformance.
- REQ-CORE-030: `Field::from_value` shall store input literally as one
  component (delimiters escaped on output); `Field::from_delimited`
  shall split `~`/`^`/`&` into structure. Verify: `field.rs` unit tests.
- REQ-CORE-040: Fluent builders shall exist for ADT (A01–A13, A17, A28,
  A31, A34, A40), BAR (P01, P02), DFT (P03, P11), MDM (T01, T02, T04),
  ORM (O01), ORU (R01), OML (O21), OUL (R21), OMP (O09), PPR (PC1), QRY
  (A19, Q01, Q02), QBP, RSP, RAS/RDE/RDS/RGV/RRD/RRA (pharmacy), SIU
  (S12–S15), VXU (V04), ACK, and batch/file envelopes. Verify: builder
  unit tests.
- REQ-CORE-050: Every message builder with a schema shall produce output
  that validates clean at v2.5 as built and after a strict parse
  round-trip, with composite inputs encoded as literal delimiters
  (never `\S\`/`\T\`/`\R\`). Verify: `builder_conformance.rs` (56 tests).
- REQ-CORE-060: Character-set handling shall support MSH-18 declared
  charsets for encode/decode paths. Verify: charset tests.
- REQ-CORE-070: Errors shall use typed `thiserror` errors with a
  `Result<T>` alias per crate. Verify: API review, `cargo doc`.

## 3. Validation (vyna-hl7-validator)

- REQ-VAL-010: Validation shall be strict by default: every finding class
  (missing required, datatype, length, cardinality, value, structural,
  version drift) is an error. Verify: strict-default tests, CLI drift test.
- REQ-VAL-020: The validator shall offer `standard()` (drift warns) and
  `lenient()` (only structural fails) contexts plus per-class `with_*`
  overrides. Verify: context tests.
- REQ-VAL-030: Schema validation shall cover the shipped schemas (310
  JSON files, v2.1–v2.8; sub-version mapping per `schemas/README.md`)
  selected by MSH-9, with an explicit fallback to structure-only
  validation when no schema exists (QBP/RSP/batch). Verify: schema
  loader tests, CLI schema tests.
- REQ-VAL-040: Datatype validation shall check formats (DT/TM/DTM/TS,
  NM, coded values) against the schema's declared types. Verify:
  datatype tests.
- REQ-VAL-050: Vocabulary validation shall enforce HL7-defined tables
  from the registry and skip user-defined tables (`USER_TABLES`
  allowlist); every schema-referenced table shall be registered or
  allowlisted. Verify: registry coverage test.
- REQ-VAL-060: Sites shall be able to extend standard tables with local
  codes without replacing them (`table_registry_mut` +
  `get_table_mut`/`add_value`). Verify: `site_tables.rs`, rustdoc example.
- REQ-VAL-070: A declarative rules engine shall support cross-field and
  business-rule validation with rule-declared severities unaffected by
  the severity context. Verify: rules tests.

## 4. Terser (vyna-hl7-terser)

- REQ-TER-010: Path access (`PID-5-1`, `OBX(2)-5`, repetitions) shall use
  1-based segment instances and field/component positions per HAPI
  conventions. Verify: terser tests.
- REQ-TER-020: `CachedTerser` shall accelerate repeated access to the same
  fields. Verify: terser cache tests and benches.
- REQ-TER-030: Bulk, iterator, and group query APIs shall exist for
  set-oriented access. Verify: terser module tests.

## 5. Transports (vyna-hl7-mllp, vyna-hl7-http, vyna-hl7-batch)

- REQ-TR-010: MLLP shall frame messages (`0x0B` … `0x1C 0x0D`) over async
  Tokio I/O with a `Codec` for streaming pipelines. Verify: mllp tests.
- REQ-TR-020: MLLP and HTTP shall support TLS and mutual TLS behind the
  `tls` feature. Verify: TLS integration tests.
- REQ-TR-030: HTTP transport shall support Basic auth (behind the
  `auth` feature), timeouts, and a pluggable response handler (default
  echo; callers supply ACK generation). Verify: http tests.
- REQ-TR-040: Mock servers (`MockMllpServer`, `MockHttpServer`) shall
  support integration tests behind the `testing` feature. Verify: mock
  server tests.
- REQ-TR-050: Batch/file building, lenient intake parsing, and file-drop
  transports (filesystem, FTP, SFTP) shall be supported. Verify: batch
  tests (incl. `ftp`/`sftp` tabs coverage where applicable).

## 6. Conversion (vyna-hl7-fhir, vyna-hl7-xml, vyna-hl7-convert)

- REQ-CONV-010: HL7 v2 → FHIR R4 conversion shall cover 12 resources
  (Patient, Observation, Encounter, DiagnosticReport, Practitioner,
  AllergyIntolerance, MedicationAdministration, Condition, Procedure,
  Immunization, ServiceRequest, Specimen) plus 4 reverse
  converters (Patient, Observation, Encounter, Practitioner). Verify:
  fhir converter tests and examples.
- REQ-CONV-020: ER7 ↔ XML transcoding (`er7_to_xml`, `xml_to_er7`) shall
  round-trip messages. Verify: xml tests.
- REQ-CONV-030: Schema-driven cross-version conversion with a
  `ConversionReport` shall be supported. Verify: convert tests.

## 7. Data lifecycle (queue, redact, templates, transform, orchestration, custom, macros, codegen, conformance)

- REQ-DATA-010: A durable store-and-forward queue shall support leasing,
  retry policies, dead letters, stats, and pluggable clocks. Verify:
  queue tests.
- REQ-DATA-020: Safe Harbor de-identification with rule-based strategies
  shall be supported. Verify: redact tests.
- REQ-DATA-030: Message/segment templates (YAML/JSON) and a
  field-mapping transformation framework shall be supported. Verify:
  templates and transform tests.
- REQ-DATA-040: Content routing, filtering, workflows, and retry
  orchestration shall be supported. Verify: orchestration tests and
  examples.
- REQ-DATA-050: Custom Z-segments shall be definable and registrable
  (`z_segment!` macro framework). Verify: custom tests, RFC 0001.
- REQ-DATA-060: Derive macros (`Segment`, `Message`), the `hl7`
  helper attribute, and the `hl7_type` attribute macro shall exist
  for typed models. Verify: macros
  tests.
- REQ-DATA-070: Schema-driven code generation shall emit HAPI-style
  typed models from message schemas. Verify: codegen tests.
- REQ-DATA-080: Conformance-profile validation (profiles, predicates,
  value sets) shall be supported. Verify: conformance tests.

## 8. User interfaces (vyna-hl7-cli, vyna-hl7-testpanel, vyna-hl7-wasm)

- REQ-UX-010: The CLI shall provide parse, validate, extract, convert,
  info, send, and diff commands. Verify: CLI integration tests.
- REQ-UX-020: `vynahl7 validate` shall default to strict HL7
  specification compliance, report the active mode, auto-select the
  schema from MSH-9, and honestly report structure-only fallback.
  `--standard` and `--lenient` relax severity; `--strict` restates the
  default. Verify: CLI validate tests.
- REQ-UX-030: The test panel GUI shall provide parser, builder, terser,
  validator (strict, labeled), FHIR, MLLP, HTTP, FTP, and XML workbench
  tabs with schema-valid sample messages. Verify: testpanel tests
  (incl. `enriched_samples_pass_schema_validation`).
- REQ-UX-040: WASM bindings shall expose parse, encode, terser get/set,
  and validate to JavaScript/TypeScript. Verify: wasm-pack builds.

## 9. Non-functional requirements

- REQ-NFR-010 (performance): Throughput targets and methodology are
  tracked in [performance guide](performance.md); performance-sensitive
  changes require before/after benches (`cargo bench`). Verify: benches.
- REQ-NFR-020 (compatibility): The workspace uses Rust edition 2024 and
  sets no `rust-version` pin; behavior changes follow semantic versioning
  with CHANGELOG entries. Verify: release review.
- REQ-NFR-030 (supply chain): Dependencies are declared in
  `Cargo.toml`; a CycloneDX SBOM is generated per release (see
  [SBOM](sbom/)). Verify: SBOM regeneration.
- REQ-NFR-040 (quality gates): `#![warn]`-clean `cargo clippy
  --workspace --all-targets` and per-area test suites are required
  before merge. Verify: gates (no CI wired yet — see roadmap).

## 10. Verification mapping

| Area | Primary suites |
|---|---|
| Parsing | `cargo test -p vyna-hl7-parser` |
| Core/builders | `cargo test -p vyna-hl7-core`, `builder_conformance.rs` (56) |
| Validation | `cargo test -p vyna-hl7-validator`, `site_tables.rs` |
| Transports | mllp/http tests incl. `--features tls,testing` |
| UX | CLI tests, testpanel tests, wasm-pack builds |
| Full gate | `cargo test --workspace` (1384 passing at 0.9.0), clippy clean |

## 11. Roadmap (explicitly out of scope for 0.9.0)

- QBP/RSP/batch message schemas (validator falls back honestly today).
- Full CVX (0292) registry refresh from the CDC source (covers 01–122,
  141, 998–999 today).
- CI wiring (build/test/clippy/fmt gates) and a declared MSRV.
- Repo-wide `rustfmt` normalization (large pre-existing baseline drift).

