# Software Design Specification — VynaHL7

Companion to the [requirements](requirements.md). Covers the 0.9.0
workspace: 23 crates, Rust edition 2024, `rust-version` 1.91.

## 1. Crate catalog and dependency direction

Dependencies flow strictly downward — lower layers never depend on upper
ones (in particular: the parser never validates, and core never depends
on the validator, which is why builder conformance tests live in
`vyna-hl7-validator`).

```
UX:        cli  testpanel  wasm
                 \    |    /
Transports:       mllp  http  batch  queue
Conversions:      fhir  xml  convert
Intelligence:     conformance  orchestration  transform  templates
                  redact  custom  codegen  macros
Access:           terser  validator
Foundation:       parser
                  core
```

| Crate | Role | Key API |
|---|---|---|
| core | Message model, builders, batch/file types, delimiters, charset, errors | `Message`, `Field::from_delimited`, `builders::*` |
| parser | ER7 parsing, strict/lenient `ParserConfig`, streaming | `parse_message`, `streaming` |
| validator | Schema/datatype/vocabulary/rules validation, strict contexts | `Validator`, `ValidationContext`, `TableRegistry` |
| terser | Path access, cached/bulk/iterator/group queries | `Terser`, `CachedTerser` |
| transform | Field-mapping transformation framework | `transformer`, `rule`, `config` |
| templates | YAML/JSON message templates | template engine |
| mllp | MLLP client/server, codec, pool, router, TLS, mocks | `MllpClient`, `MllpCodec`, `testing` |
| http | HL7-over-HTTP client/server, auth, TLS, metrics, mocks | `HttpClient`, `HttpServer` |
| macros | `Segment`/`Message` derives, `hl7` helper attribute, `hl7_type` attribute macro | proc macros |
| codegen | Typed-model generation from message schemas | `generate_for`, `generate_module` |
| redact | Safe Harbor de-identification | `Redactor`, `Strategy`, `Rule` |
| convert | Cross-version conversion + `ConversionReport` | `convert`, `convert_with_schemas` |
| queue | Durable store-and-forward (lease/retry/dead-letter) | `MessageQueue`, `RetryPolicy`, clocks |
| batch | Batch files, lenient intake, file-drop (fs/FTP/SFTP) | `ftp`, `sftp`, `BatchBuilder` (core) |
| fhir | v2→FHIR R4 (12) + reverse (4) converters, bundles | `converters`, `bundle` |
| custom | Z-segment framework (`z_segment!`), registry | `registry`, `message_ext` |
| conformance | Conformance-profile validation | `profile`, `predicate`, `validator` |
| orchestration | Routing, filtering, workflows, retry | `routing`, `filtering`, `orchestration` |
| wasm | JS/TS bindings (parse/encode/terser/validate) | `#[wasm_bindgen]` fns |
| cli | `parse/validate/extract/convert/info/send/diff` | clap app |
| xml | ER7 ↔ XML transcoding | `er7_to_xml`, `xml_to_er7` |
| testpanel | egui workbench (9 tabs) + schema-valid samples | GUI binary |

## 2. Data model

```
Message
 └─ Segment (MSH, PID, OBX, …; MSH-1 is the separator itself)
      └─ Field (split on |)
           └─ Repetition (split on ~, 0-based internally)
                └─ Component (split on ^)
                     └─ Subcomponent (split on &)
```

Values encode with HL7 escapes (`\F\ \S\ \T\ \R\ \E\`); the MSH
segment regenerates MSH-1/MSH-2 from the active delimiters on encode.
Terser paths are 1-based (`OBX(2)-5`); repetition indexes in paths
follow the same 1-based addressing documented in the terser crate.

## 3. Parser

Delimiter-based splitting with pre-allocation for common shapes
(`optimized` strategies). Strict mode is the default; lenient mode
tolerates real-world deviations via `ParserConfig`. A `streaming`
module handles large inputs incrementally. The parser produces the
core model only — no schema or vocabulary checks (REQ-PARSE-040).

## 4. Validator

Four layers, all strict by default (every finding class is an error,
including MSH-12 drift):

1. **Schema** — 310 JSON schemas (`v2_1`–`v2_8`, see
   `schemas/README.md` for provenance and sub-version mapping),
   selected by MSH-9; honest fallback to structure-only validation
   when no schema exists.
2. **Datatype** — format checks per the schema's declared types.
3. **Vocabulary** — HL7-defined tables from the registry; user-defined
   tables stay unregistered under the `USER_TABLES` allowlist, and a
   coverage test fails if any schema-referenced table is neither
   registered nor allowlisted. Empty means absent at every level.
4. **Rules engine** — declarative/cross-field business rules with
   rule-declared severities, unaffected by the severity context.

Severity contexts (`strict`/`standard`/`lenient` + per-class `with_*`)
downgrade finding classes explicitly; `table_registry_mut` extends
standard tables with site-local codes without replacing them.

## 5. Builders

Fluent per-trigger builders (`AdtBuilder::a01`, `BarP01Builder`,
`MfnM01Builder`, …) construct segments explicitly. Construction rules:

- Composite inputs go through `Field::from_delimited` (structure);
  literal values through `Field::from_value` (delimiters escaped).
- OBX-5 takes the shape of OBX-2: coded/composite types split,
  `ST`/`TX`/`FT` stay literal.
- Free-text fields (NTE, descriptions) always stay literal.

Conformance over codegen: instead of generating builders from schemas,
`builder_conformance.rs` (56 tests) proves every message builder with a
schema validates clean at v2.5 as built and after a strict parse
round-trip, with a wire-escape regression check. The suite lives in the
validator crate to avoid a core→validator dependency cycle.

## 6. Transports

- **MLLP**: `0x0B` … `0x1C 0x0D` framing over Tokio; `MllpCodec`
  implements tokio-util `Decoder`/`Encoder` (noise skipping, first
  `FS CR` terminates, size limit) for streaming pipelines.
  `tls` feature: server/client TLS and mutual TLS. `testing` feature:
  `MockMllpServer` with automatic ports and handler injection.
- **HTTP**: HL7-over-HTTP client/server with Basic auth (server-side
  `with_auth` requires the `auth` feature), timeouts, and a pluggable
  response handler (default echo); `tls` for HTTPS/mTLS; `testing` for
  `MockHttpServer` (serves HTTPS when configured `with_tls`).
- **Batch/file**: `BatchBuilder`/`FileBuilder` envelopes plus lenient
  intake parsing and file-drop transports (filesystem, FTP, SFTP).

## 7. Conversions

- **FHIR**: hand-written v2→R4 converters per resource (12 forward:
  Patient, Observation, Encounter, DiagnosticReport, Practitioner,
  AllergyIntolerance, Condition, Procedure, Immunization, Medication,
  ServiceRequest, Specimen; 4 reverse: Patient, Observation, Encounter,
  Practitioner) with Bundle assembly.
- **XML**: direct ER7 ↔ XML transcoding functions.
- **convert**: schema-driven cross-version conversion reporting a
  `ConversionReport`.

## 8. Cross-cutting

- **Errors**: typed `thiserror` errors per crate with `Result<T>`
  aliases; validation findings carry locations (`PID[3]-5-6`).
- **Features**: `tls` (transports), `testing` (mock servers) are the
  main gates; combine as `--features tls,testing`.
- **Schemas**: generated/curated JSON under the validator crate; the
  loader resolves version sub-mapping and v2.3 fallback for types
  introduced after v2.2. Never hand-edit generated content without
  updating the pipeline (`scripts/generate_schemas.py`).

## 9. Key decisions

- 2026-10: Strict validation is the default posture (spec compliance);
  `standard`/`lenient` are explicit opt-outs. Rationale: a clean result
  must mean compliance; feeds that differ say so at the call site.
- 2026-10: Builder conformance suite instead of schema codegen for
  builders. Rationale: builders carry domain logic; 56 cheap tests pin
  behavior without a core→validator cycle.
- 2026-10: `from_delimited`/`from_value` split. Rationale: one
  constructor cannot serve structure and literal text; wire corruption
  from conflating them was found and fixed.
- 2026-10: User-defined tables stay unregistered (`USER_TABLES`) with a
  coverage test, rather than enforcing example values. Registered
  HL7-defined tables (e.g. 0088, 0292) are enforced as specified.
- Pre-fork: see [RFCs](rfcs/) (e.g. 0001 Z-segment framework) for
  earlier design records.
