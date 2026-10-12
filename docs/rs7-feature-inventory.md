# rs7 Feature Inventory (Pre-Fork History)

VynaHL7 was forked from rs7 at commit `8129978` ("Rename rs7 to VynaHL7,
reset version to 0.1.0"). The 89 rs7-era commits (2025-10-07 to
2026-10-04) were later squashed out of this repository's history; this
file preserves what rs7 had built, so the provenance of the inherited
code is not lost.

Post-fork removals: the DICOM test-panel tab (SCU/SCP, C-ECHO/C-STORE/
C-FIND) was removed to keep VynaHL7 HL7-only; the `dicom-*` dependencies
are gone and no DICOM code remains in the project.

## Workspace at Fork (17 crates)

| Crate | Purpose |
|---|---|
| rs7-core | Message model, builders, ACK builder, batch/file types |
| rs7-parser | Delimiter parser, strict/lenient modes, streaming parser |
| rs7-validator | Schema, datatype, vocabulary, rules-engine validation |
| rs7-terser | Path field access, bulk/iterator/query APIs, groups |
| rs7-mllp | MLLP client/server, TLS, pooling, routing |
| rs7-http | HL7-over-HTTP client/server, auth, middleware |
| rs7-fhir | HL7 v2 ↔ FHIR R4 converters (12 forward, 4 reverse) |
| rs7-conformance | Conformance profiles, predicates, value sets |
| rs7-custom | Z-segment framework (`z_segment!` macro) |
| rs7-templates | Message templates, YAML/JSON, inheritance |
| rs7-transform | Field-mapping transformation framework |
| rs7-orchestration | Content routing, workflows, filters, retry |
| rs7-macros | Derive macros (`Hl7Segment`, `Hl7Message`) |
| rs7-xml | ER7 ↔ XML encoding |
| rs7-cli | Command-line message analysis tool |
| rs7-wasm | WebAssembly bindings + JS schema definitions |
| rs7-testpanel | egui GUI test application (10+ tabs) |

## Feature History by Release

### V0.1.x — Foundation (2025-10-07)
- **V0.1.0**: initial commit; 6 crates (core, macros, mllp, parser,
  terser, validator); Message→Segment→Field→Repetition→Component→
  Subcomponent model; delimiter-based parser; MLLP framing.
- **V0.1.1**: fluent message builders (ADT/ORM/ORU/laboratory/
  pharmacy/scheduling) plus complex field builders (XPN, XAD, XTN,
  CX, XCN).
- **V0.1.2**: datatype validation (dates, times, numerics, coded
  values) and vocabulary/code-set validation against HL7 tables.
- **V0.1.3**: message schemas for BAR, RAS, RDE, RDS, MFN, RGV, RRD,
  RRA, OUL, OML.

### V0.2.0–V0.6.0 — Coverage and Surfaces (2025-10-08/09)
- **V0.2.0**: all common message types and trigger events
  (ADT A01–A13/A17/A28/A31/A40, SIU, MDM, DFT, QRY, ORM, ORU, ACK).
- **V0.3.0**: FHIR conversion utilities — 9 forward converters
  (Patient, Observation, Encounter, DiagnosticReport, Practitioner,
  AllergyIntolerance, MedicationAdministration, Condition,
  Procedure).
- **V0.4.0**: performance optimizations pass.
- **V0.5.0**: WebAssembly bindings for JavaScript/TypeScript.
- **V0.6.0**: CLI tool (`parse`, `validate`, `extract`, `convert`,
  `info`).
- **HTTP support**: rs7-http crate — HL7-over-HTTP client/server,
  Basic auth, configurable timeouts, auto-ACK.
- **Security/reliability**: constant-time auth comparison,
  `MllpConfig` (connect/read/write timeouts, 10 MB message cap,
  DoS protection), pre-allocated I/O buffers.

### V0.8.0 — Custom Z-Segment Framework (2025-11-19)
- rs7-custom crate: type-safe `z_segment!` macro, fluent builders,
  message extension trait, validation hooks, global registry.
- Field types added progressively: primitives
  (`String/u32/i32/i64/f64/bool` with HL7 boolean conventions),
  chrono datetimes, `Option<T>`, `Vec<T>` repetitions, tuple
  components (2–5), `Option<Tuple>`, `Vec<Tuple>` repeating
  components.
- Terser migrated to 1-based segment indexing (`OBX(1)`).

### V0.9.0–V0.13.0 — Validation and Messaging Depth (2025-11-19/20)
- **V0.9.0**: conformance profile framework — XML profile parser
  (`HL7v2xConformanceProfile`), R/RE/O/X usage, cardinality,
  length checks, severity-graded reporting.
- **V0.10.0**: terser BulkTerser (`OBX(*)-5` wildcards), field/
  component/repetition iterators, `TerserQuery` conditional API.
- **V0.11.0**: 3 more FHIR converters (Immunization, ServiceRequest,
  Specimen) with CVX/HL7-table code mappings; VXU/ORM/OML support.
- **V0.12.0**: QBP/RSP query/response — Q11/Q15/Q21/Q22 builders,
  K11/K21/K22 responses, QPD/RCP/QAK segment builders,
  `QueryResultParser` with pagination and CDC profiles.
- **V0.13.0**: batch/file processing (BHS/BTS/FHS/FTS,
  `BatchBuilder`, nested files→batches→messages) in rs7-core, plus
  the rs7-transform crate (field mappings, builtin/custom
  transforms, YAML/JSON config).

### V0.15.0–V0.18.0 — Enterprise Validation (2025-11-20)
- **V0.15.0**: rs7-templates — reusable message patterns with
  `{{variable}}` substitution, engine, validator, multi-level
  inheritance, YAML/JSON round-trip, 7 standard templates.
- **V0.16.0**: conformance phase 2 — predicate engine
  (`IS VALUED`, comparisons, `AND/OR/NOT`), conditional usage,
  component profiles, value-set bindings with strengths,
  co-constraints.
- **V0.17.0**: validation rules engine — 24 builtin rules, 7
  cross-field patterns, declarative YAML/JSON rules, severity
  levels, validator integration.
- **V0.18.0**: rs7-orchestration — content-based routing, async
  multi-step workflows, ALL/ANY filters, retry with backoff,
  structured errors.

### V0.19.0 — Transport Security and Testing (2025-11-20)
- TLS/mTLS for MLLP and HTTP (`TlsClientConfig`/`TlsServerConfig`,
  `connect_tls`, `bind_tls`/`serve_tls`, feature flags).
- `MockMllpServer`/`MockHttpServer` testing utilities (`testing`
  feature).
- TLS integration tests with auto-generated X.509 v3 certificates.

### V0.20.x — Enhancement Suite, Macros, XML, Test Panel (2025-12-04)
- **V0.20.0** (9 features): ACK/NAK builder (original + enhanced
  mode, ERR segments); versions 2.1/2.2/2.8–2.8.2 with ordering
  helpers; MLLP application router; strict/lenient parser modes
  (`ParserConfig`); located/severity-graded errors; terser segment
  groups; MLLP connection pooling; event-based streaming parser.
- **V0.20.1**: derive macros rewritten for syn 2
  (`Hl7Segment`/`Hl7Message`); rs7-xml crate (ER7 ↔ XML);
  4 reverse FHIR converters (Patient/Observation/Encounter/
  Practitioner → v2).
- **V0.20.2**: rs7-testpanel GUI (Parser, Builder, Terser,
  Validator, MLLP, FHIR, XML tabs; egui; sample messages).
- 22 follow-up commits: real MLLP networking, clickable terser
  tree, file open/save, MLLP 8.9× throughput (chunked I/O,
  TCP_NODELAY; 49k msg/s at 8 clients), HTTP/FTP/DICOM tabs,
  DICOM C-ECHO/C-STORE/C-FIND, Windows icon/branding.

### v1.x — Stable, Schemas, Charset (2026-03 → 2026-10)
- **v1.0.0**: benchmark framework (9 Criterion suites, message
  corpus, 4-section reports); unified 1.0.0 versioning.
- **v1.1.0**: complete schemas for v2.3–v2.8 — 48 segment types,
  25 composite datatypes, version-specific PID/OBX/OBR/MSH
  differences, 16 new `DataType` variants, `ComponentDefinition`,
  `scripts/generate_schemas.py`.
- Component-level validation (required/length/type/vocabulary per
  component).
- **v1.2.x/v1.3.x**: API-key and Bearer [REDACTED] auth (HTTP client,
  axum middleware, test panel); HTTPS test-panel tabs
  (reqwest+rustls); FHIR-server send; newline-separated display.
- **v1.5.0**: MSH-18 charset support (ASCII, UTF-8, 8859/1–11/13–16)
  across all crates; enriched samples; 1-based terser repetitions
  (breaking); proprietary licensing.
- **v1.6.x**: builder templates with variable fields, version/
  charset options, message save/load; dependency major bumps;
  ORU builder OBR-2/OBR-25/PV1-19; rustls PEM loader migration.
- Content: 12 introductory + 11 deep-dive blog posts with
  benchmarks (EN/CN/LinkedIn variants).

## Notes
- rs7 release tags at fork: v0.1.0, v0.8.0, v0.9.0, v0.10.0,
  v0.11.0, v0.19.0 (retired with the history).
- Everything after the fork (typed codegen, VXU/PPR/OMP/A34, FHIR
  mapping/Bundle, redaction, serde, conversion, queue, batch
  transports, vocabulary depth, CLI send/diff, timestamps,
  validation contexts, streaming codec, v2.1/v2.2 schemas) is
  VynaHL7-native work and remains in `git log`.
