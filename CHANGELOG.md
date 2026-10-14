# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0] - 2026-10-13

### Fixed

- Packaging blockers for crates.io: testpanel/macros path-only deps now use workspace deps, root facade made publishable, per-crate LICENSE copies, READMEs for batch/codegen/convert/queue/redact/testpanel/xml, `rust-version` 1.91 everywhere
- Panic elimination: parser lenient short-input OOB, streaming byte-slicing, delimiters/datetime slicing, batch intake non-ASCII slicing, FHIR date/datetime guards (all converters + mapping engine), conformance predicate non-ASCII/quote panics, terser bulk wildcard panics — all return `Err` now
- Silent data corruption now fails loud: XML decoder rewritten (repetitions, subcomponents, 4-part names round-trip), FHIR condition indexes, `obs-{OBX-1}` report refs, unparseable NM/amount values error, `Bundle::add_resource` propagates failures, reverse encounter PV1-4/PV1-10, template component positions and whitespace placeholders
- `TerserMut::set` gap-fills missing segment occurrences with a documented roundtrip contract; `CachedTerser`/`Terser` missing-vs-malformed behavior unified and documented
- Redactor gains `unmatched_rules()` (typo'd rules detectable) and `InvalidRule` for field/component 0; Safe Harbor free-text/Z-segment gap documented with locking tests
- Credential leaks closed: redacting `Debug` impls for FTP/SFTP/HTTP auth types; constant-time API key compare
- Unbounded buffering capped: HTTP response size, FTP/SFTP/poller file limits, queue depth caps, WebSocket binary size checks; blocking fs moved to `spawn_blocking`; FTP/SFTP command timeouts
- CLI `validate` exits 1 on invalid messages; `extract` rejects malformed paths on stderr; proc macros emit spanned errors instead of panicking; dead code removed (`PooledConnectionGuard`, reqwest-retry/middleware deps)
- XML root element uses the full MSH-9 value (`ADT_A01`, not `ADT`); repetitions always encode (empty ones as empty elements) so positions survive round-trips
- Remaining byte-slicing panics eliminated (core date/timestamp helpers, custom datetime helpers, transform date formatters)
- Poller redrops reprocess (composite settled keys + listing reconciliation); per-file failures collected into `PollReport::errors` instead of aborting the poll
- `WebhookConfig` Debug redacts header values; testpanel verifies TLS certs and SSH host keys by default with explicit opt-outs
- Predicate parsing ignores quoted keyword/operator lookalikes; `validate` rejects unrecognized `--hl7-version` values
- FHIR invalid-datetime rule documented (strict primary timestamps, lenient ancillary ones); stale README version pins and private-repo wording fixed
- Minor review round: dead core error variants removed; streaming/transform panics return errors; transform path validation accepts mid-path repetitions; convert guards field-0 moves; terser value resolution deduplicated; group `from_pattern` API; MDM^T02 identity fixed; CLI rejects unknown formats; MLLP pool documents manual cleanup and gains `with_cleanup_interval`; READMEs/docs corrected (1-based terser paths, builder APIs, schema coverage, MSRV)
- Third review round (23 majors): streaming parser gains line-split/escape/delimiter parity with the regular parser; `continue_on_error` surfaces skipped segments as `SKIPPED_SEGMENT` warnings; version-convert moves merge into occupied targets as repetitions; whole-field `TerserMut::set` clears stale components; missing QAK-2 no longer means success; template trigger-event check fixed with a real parsed fixture; custom `Vec`/`String` parses error instead of dropping/truncating; HTTP builders preserve TLS+HTTP2 across calls; network/timeout errors are retryable; MLLP TLS handshakes are timeout-bounded; staged FTP/SFTP uploads are hidden from pollers; conformance `R` usage requires content; self-closing profile segments keep siblings; XML element numbers are capped; `DateShift` is multibyte-safe; Safe Harbor covers DG1/STF/PRD/TXA identifiers; WASM validates against the MSH-9 schema and generates MSH-10/11; testpanel caps HTTP bodies and locks the unimplemented SFTP server
- Third review round (36 minors): saturating substring and char-correct capped pad; validator skips component key 0; codegen sanitizes doc comments and maps components of optional fields to `Option`; dead parser flags wired up (`preserve_invalid_escapes`, `validate_segment_ids` with a known-ID table); truncation warnings emitted (new `SegmentsTruncated` code); derive checks MSH-9, fails missing required fields, rejects duplicate mappings, and round-trips escapes; datetime accepts reduced precision and rejects trailing garbage; partial option-tuples preserved; z_segment enforces IDs and segment match; dead `__z_segment_*` helpers removed; terser paths reject garbage, queries search whole fields, iterators document skipping, QAK counts are unknown-not-zero, `set` gap-fill capped; templates gain literal-variable mode, position caps, and sticky inheritance flags; API-key compare leaks nothing; logging/metrics gain `try_init`; host-key pins surface `HostKey`; queue IDs reject traversal; empty HMAC keys rejected; poller tracks mtime stability and reports skips; filesystem temp handling fixed; MemTransport reports mtimes; XML element names sanitized; mapping indexes and predicate depth capped; WASM validates MSH inputs; testpanel caps MLLP frames and download sizes
- Fourth review round (15 majors): codegen sanitizes attribute strings and struct names; transform rules keep private params; control IDs gain counter+pid uniqueness; `hl7_type` fails missing required components; pool evictions close gracefully (`test_on_acquire` removed as unimplementable without I/O probes; permit-leak claim disproven); HTTP builders preserve timeouts; TLS handshake/connection timeouts; WebSocket broadcast implemented; conformance X-usage requires content; ServiceRequest maps coding systems; Redactor Debug redacts the HMAC key; WASM `version()` returns dotted form; testpanel truncation is char-safe and tree repetition paths are 1-based
- Fourth review round (44 minors): strict streaming segment IDs; NM grammar check; honest composite-validation docs; out-of-schema segments warn in all modes (schemas are minimal); `ValidationContext::default` is strict; timestamps reject offsets and fold fractions; atomic in-place transforms; BLC replaced with BLG plus a schema-crosschecked ID table; `Vec` tuples error on partials; `SerializeSegmentField` propagates errors; inheritance depth capped; FHIR deceased-choice, ORC-9 fallback, calendar dates, and redacted errors; capped webhook backoff; media-type and scheme auth fixes; WS timeouts; TLS error context; explicit body-limit knob; XML digit IDs; truncated-profile and bad-Length errors; symlink/absolute-path hardening; normalized FTP paths; narrowed MLSD fallback; mtime-aware stability; all-fail quarantine; per-poll byte budget; MSH-18 intake decoding; collision-proof queue IDs; checked date shifts; sanitized test-server ACKs; loopback FTP defaults; XSS-free example; bounded test reads; raw MSH JSON; delimiter-aware CLI display; WASM point releases; CI feature matrix; workspace quick-xml pin; explicit readme keys and docs.rs links (router ACK-drop and same-second redrop claims disproven/documented)
- Fifth review round (6 majors): Safe Harbor extended to 75 more rules across GT1/IN1/NK1/PV1/PD1/ORC/EVN/OBR/TXA/IN2/IN3/SCH/ROL/PR1/CTD/RXD/RXE/FT1/BLG/ACC/PDA with documented 34-segment coverage; refused medication maps to `not-done`; all FHIR converters validate calendar datetimes via shared helpers; `is_acknowledgment` recognizes bare MSH-9 ACKs; datetime errors redacted across converters and mappings
- Fifth review round (32 minors): capped mapping target depth; uncapped FHIR names/ids/addresses with empty-gap skipping; repeat-emitting reverse patient writers; empty-is-absent guards (RXA-9, SPM-20, ORC-2/3/4, AL1-2/3/4, absent PV1 doctors); round-trip-safe reverse observation (D status, DTM type, text-only ranges); per-OBR DiagnosticReport linking; ORC-paired TQ1 priority; preserved zones, minutes, and YYYY-MM precision across converters and mappings; Practitioner id into PV1-7; XTN email telecoms; strict composite rejection in custom scalar/tuple/`Vec` parses; fraction-preserving datetime serialization; single-pass template substitution; absent MSH-9 fails template checks; partial MSH-9 agreement and whole/component overlap rejection in derives; MSH-1/2 template rejection; `get_if` surfaces malformed paths; hygienic `z_segment!` paste; char-counted length checks; exact client media-type gate; JWT behavior tests; doc-attachment repairs; corrected WASM perf claim; strict `MemTransport` paths; queue temp-file reaping; `route_all` fallback tests
- Sixth review round (5 blockers, 30 majors, 24 minors): streaming parser preserves empty fields; MLLP delivers pipelined frames via per-connection leftover buffer; Safe Harbor covers OBR-32..35/OBX-25/PID-12 plus audited IN1/IN2/SCH name fields; unique FTP/SFTP staging names; repeating/optional derive failures propagate; required-`Vec` derive support with codegen emission; uncapped-FHIR-style datetime/zone/precision completion (observation, encounter, medication, procedure); vaccine/coding-system mapping; PRB display+system; PV1-3 location detail; empty-is-absent across condition/medication/procedure/immunization/service-request/specimen extractors; reverse telecom/name/datetime fixes; TM validation; dose-error redaction; mapping non-string and default-interpolation errors; XPN-6 degree; text-only allergy codes; conformance MSH-1/2 and RE fixes; ACK trigger normalization; unparsable MSH-12 rejection; char-counted validator lengths; lenient encoding padding; all-repetition field checks; ASCII ID validation; rule-severity honesty; batch separator/totals fixes; MSH placeholder docs; bare-MSH-9 agreement; partial-tuple acceptance; whole-composite template lengths; registry doc fixes; required Segment `id`; plain-HTTP request timeout; mTLS negative test; archive versioning; atomic dequeue claims; saturating queue math; garbled-trailer errors; orphan-body reaping; strict XML duplicate rejection; WASM full-value extracts, segment search fallback, version guard, `@throws` docs, corrected perf claims; bracket-aware CLI SNI; full CI feature matrix; CLI parse/convert/info e2e tests
- Seventh review round (2 blockers, 21 majors, 35 minors): file-queue at-least-once (body-before-meta, lock-held mutating paths, runtime skip of partial entries); WASM JS clock for MSH-7/control-ID (node-verified); shared FHIR coding-system table; RXA-21 Action-Code unmapped; DG1/PRB/RXA-5 system mapping; immunization/specimen empty sweeps; reverse XPN-6/XCN-7/PL round-trips; validator cardinality enforcement with DG1 schema-flag corrections; second-MSH alignment; capped PHI in errors; streaming trailing-delimiter and bare-MSH parity; conformance message delimiters; unsplit-MSH-9 tolerance; documented first-component Terser contract; `hl7_type` attribute/generic preservation; engine MSH placeholders; subcomponent-aware derive round-trips; cause-specific TryFrom errors; empty-segment-id and delimiter-field set rejection; subcomponent-counted template lengths; mock HTTP parity layers and knobs; auth-before-content-type; pool wait honesty; outbound MLLP size checks; XML media-type gate removal; truncation-specific EOF errors; mock AE NACK on handler errors; suffixed XML message roots; transform minute-precision datetimes; uniform queue skip format; self-settling poller rejection; JSON-compatible WASM serialization with string keys and nulls; CI and `--charset` living-doc updates (valueBoolean non-round-trip documented: ST+Y is indistinguishable from string "Y")
- Eighth review round (5 blockers, 10 majors, 45 minors): Safe Harbor identifier/telecom/date sweeps (schema-verified IN1/IN2/GT1/NK1/SCH/ORC/OBR/OBX/DG1/ROL/PR1/FT1/RXD/RXE/CTD/EVN/ACC/PD1/PV1 rules), PD1-19 un-removed, structure-safe DR handling; XAD other-designation lines; FHIR `HH:MM:SS` times; RXR-2 body-site pairing; text-only CE values; shared-system allergy codes; reverse birth-order, stable system names, XCN-10 maiden/temp; literal unsplit-value semantics with `from_delimited` docs; SI nonnegative; PT/Table-0103 agreement; empty-aware conformance cardinality; programmatic-only Usage C; leading-blank and explicit-empty-MSH-3 parse parity; codepoint-only charset errors; truncated NA echoes; tolerant partial `Vec`-tuples; escape-aware derive component/`hl7_type` round-trips; empty-String absence; all-repetition terser filter; literal-`&` set docs; codegen MSH-1/2 skip and key fallback; required-`Vec` runtime pins; RFC 9110 `WWW-Authenticate` challenges; empty-CA rejection; slow-handshake-tolerant mock accept; NLST directory probing; tolerant SFTP mkdir; locked queue stats; trailing-empty and dotted-root XML round-trips; strict transform pad/date tails; shippable WASM types with async-init docs; source-build install docs; segments-first search; full testpanel version list; mllp `full` CI row (C4 unified under the shared coding table as `urn:oid:C4`)
- Ninth review round (4 blockers, 17 majors, 28 minors): text-only OBX-5/CE round-trips; stable reverse system names; text-only Condition/Procedure/ServiceRequest/Medication concepts; JWT `nbf` enforcement; all-repetition BTS totals; ORC-before-OBR codegen order; `--tls` refused on plain HTTP; SFTP empty-root listing; parent/order identifier redaction (ORC-8, OBR-26/29, PRB-4/5), STF DLN/date coverage, TQ datetime components, wrong-date-rule removals (PRB-24/25, IN1-14/15); allergy YAML system parity; given-only practitioner names; discharge-finished encounters; RXA-3/vaccine-code absence errors; YYYYMM fallback completion; mapping fraction parity; strict BTS/FTS numerics; decoded repeating lengths with documented first-component rule for coded fields; unterminated-escape errors; decoded conformance lengths; empty-input streaming errors; eager client URL validation with diagnostic bodies; TLS Nagle parity; pool min/max honesty; bulk case/empty-ID handling; whitespace-ID rejection; template autofill/required/placeholder rules; `hl7_type`/segment-model duplicate and `required` rejections; struct-name collision suffixes; invalid-present runtime pins; documented `Vec<String>`/schema trust models; lenient XML mismatch skips; settle-failure pins; `--server-name` guard (whole-value non-repeating length rejected: incompatible with real coded-field text; SPM-17 already range-aware)
- Eleventh review round (second blocker-free: 18 majors, 25 minors): shared procedure coding fallback; encounter status honors undischarged classes (empty class unknown); text-only OBR-4/RXA-5/SPM-4 preserved as text; reverse datetimes keep negative-offset signs; reverse CVX/NDC/MVX arms; reverse usual/unknown map to XCN-10 U; reverse name/address overflow space-joined; anchored HTTP retry verdicts; quarantined queue entries surface in dead_letters; JWT array audiences accepted; wire-faithful Safe Harbor hashing plus IN1-7 removal; per-pair hex-escape decoding; pre-mutation zero-index rejection in TerserMut; position-keeping derive reads; untrimmed field lengths; streaming post-header separator parity; XCN display builders (OBX-16, ORC-12, RXA-10); display-only participants; empty type-code filters; PV1/ORC repetition sweeps; convert presence via whole-field emptiness; codegen dedupe and SIU RGS/AIS-group order; case-insensitive HTTP routing and schemes; empty-rename rejection; standalone BroadcastChannel docs; strict foreign-prefix XML rejection; required-scalar completeness docs; position-based testpanel backfill; transform format docs; unsigned parse_date; truncated from_iana echoes; strict Z-segment ids
- Tenth review round (first blocker-free: 10 majors, 22 minors): MSH-9-3/CX-5 optional in all 129 schemas with a documented installed-base deviation; shared observation coding table; ServiceRequest order-group pairing and OBR-31-3 reason systems; lenient invalid-PID-29 drops with field-attributed datetime errors; Encounter participant displays include given names; XCN-10 name-type mapping; RXR-1 dosage routes; per-bound SPM-17 leniency; DG1-backed Condition mapping with a narrowed single-coded-PRB doc claim; JWT `exp` pinned required; MLLP stale-buffered-frame sends refused; HTTP/1-only TLS docs; FHS/BHS delimiter guards across derive/templates with built-header placeholders; `TerserMut` creation-path segment-ID validation; HAPI-verified PRD Safe Harbor audit; strict substring lengths; empty XML field elements; FHS/BHS-13 slot preservation; MSH/FHS/BHS post-header separator errors; bad header datetimes error; NM-grammar BTS totals; ASCII-only delimiters on both parse paths; long-value truncation in segment/version/charset/trailer echoes; Table 0104 gains 2.8.2; CI runs the wasm-pack browser suite
- Twelfth review round (1 blocker, 19 majors, 24 minors): SFTP empty-path rejection before I/O; unique restarted-OBX observation ids; OBR-3 filler identifiers; RXA-20 `NA` maps to `not-done`; shared 0396 short-form coding arms; unset practitioner `name.use`; explicit component-3 systems honored over bound-table defaults; text-only OBX-3 preserved; reverse U-name/omitted-address round-trips; interpretation display round-trips; lossless same-version conversion of local fields; interleaved WebSocket control frames; testpanel trigger restamp and dropped-field stamping; first-repetition CLI display; unsigned custom date/time parses; composite RCP fields; deterministic validator ordering; validated header datetimes; UCUM quantity systems; compilable codegen idents; deterministic codegen keys; uppercase conformance ids/paths; unescaped profile entities; documented reverse first-only writes; absolute SFTP roots; lone-file poller guarantee; honest quarantine labels; clamped broadcast capacity; documented mock panics; case-insensitive CLI schemes; documented transform expansions/defaults/errors; strict template placeholders; accurate segment-id errors; sorted registry ids; required QAK-1

### Changed

- **Breaking**: `Bundle::add_resource`/`BundleBuilder::add_resource` return `ConversionResult`; `ConvertError` gains a `Schema` variant (loses `PartialEq`/`Eq`); unparseable FHIR numeric/datetime values now error instead of being silently dropped
- MSRV is now Rust 1.91 (`rust-version` in every manifest)

## [0.8.0] - 2026-10-13

### Added

- Living SDLC documentation: requirements specification (`docs/requirements.md`), design specification (`docs/design.md`), integration guide (`docs/integration.md` with executed snippets), CycloneDX SBOM (`docs/sbom/`), and a documentation index with update rules (`docs/README.md`)
- `scripts/merge_sbom.py` merges per-crate CycloneDX files into one workspace SBOM
- Declarative FHIR mappings gain a `fallback_source` rule key (second Terser path tried when `source` resolves nothing); the shipped allergy table uses it for `code.text`

### Changed

- **Breaking**: server-side `with_auth` (`HttpServer`, `MockHttpServer`) now requires the `auth` feature; without it the method does not exist (previously every authenticated request failed with 500 at runtime)
- **Breaking**: `FieldProfile` gains an `item_no` field preserving the HL7 item number (update struct literals)
- Measured performance figures replace estimates: cached Terser ~60ns (~1.4x), small ADT ~60k msg/s, medium ORU ~19k msg/s

### Fixed

- Documentation staleness: 23-crate workspace map (was 10), 12+4 FHIR converters (was 9), ADT A34, full v2.1–v2.8.2 version list, BAR/MFN builder coverage, and a broken `with_strict_mode` blog snippet now using `ValidationContext`
- FHIR converters used 0-based Terser paths: displays now read component 2, coding systems component 3, repetitions/segments 1-based (patient identifiers/authority/type, medication, procedure, diagnostic report, condition, specimen, service request, immunization, encounter, allergy with description/system reads)
- `BulkTerser` repetition wildcards (`PID-5(*)-1`) always errored; `(*)` is now parsed and expanded 1-based
- WASM `extractPatientDemographics`/`extractObservations` indexing: family/given names, per-segment reads, and test names
- `MockHttpServer::with_tls` was silently ignored; the mock now serves real HTTPS with `https://` URLs (new `serve_tls_on`)
- Conformance profile `ItemNo` misused as field position; position is now the 1-based element ordinal
- `RetryConfig::new(0, _)` panicked; attempts clamp to a minimum of 1
- Test panel XML tab missing from the tab bar (`ActiveTab::all()`)
- SRS/SDS/guide corrections (converter count, derive names, `HttpClient::with_tls`, echo-by-design transports), README rules snippet/throughput/crate map, blog WASM/TLS/perf claims, crate READMEs, RFC 0001 deviations, and `TERSER_INDEXING.md` rewritten to 1-based truth

## [0.7.0] - 2026-10-13

### Added

- BAR builders: `BarP01Builder` (add patient accounts: demographics, visit, account, diagnoses, procedures, guarantor, insurance) and delegating `BarP02Builder` (purge)
- MFN builder: `MfnM01Builder` with schema-valid master-file defaults plus `add_segment` payload passthrough for record data
- Conformance coverage for BAR_P01, BAR_P02, MFN_M01 (56 tests total)

### Fixed

- Table 0292 (CVX) gains verified code 141 (CDC IIS: Influenza, split virus, trivalent, preservative, Active); snapshot ceiling (01-122, 141, 998-999) now documented at the source
- ORU OBX-5 takes the shape of OBX-2: coded/composite value types split delimiters into structure, `ST`/`TX`/`FT` stay literal so text escapes correctly
- Last pre-existing clippy warnings removed workspace-wide (`table_iso4217` rename, mllp test-server `while let`, redundant borrows)

## [0.6.0] - 2026-10-13

### Changed

- Strict validation is now the default spec-compliance posture: all `Validator` constructors use `ValidationContext::strict()`, so MSH-12 version drift fails instead of warning. `standard()` (drift warns) and `lenient()` remain explicit opt-outs
- `vynahl7 validate` defaults to strict; `--strict` restates it, new `--standard` flag covers the middle ground, `--lenient` unchanged. JSON output gains a `mode` field and text output prints the validation mode
- Testpanel Validator tab validates strict with an explicit compliance label

### Added

- `TableRegistry::get_table_mut` for extending standard code tables with site-local codes (e.g. NDC alongside CVX in 0292) without replacing them; documented with a worked example plus `tests/site_tables.rs` proving strict rejects unregistered codes and accepts extended ones

## [0.5.1] - 2026-10-13

### Fixed

- Builder wire corruption: composite values passed through `Field::from_value` were stored as a single component and escape-encoded on output (`PID-5` built from `Smith`/`John` encoded as `Smith\S\John`). Builders now split composites into real structure, so built messages encode with literal delimiters and Terser paths resolve on built messages
- `vynahl7 validate` and the testpanel Validator tab now auto-select the message schema from MSH-9, falling back to structural validation with an explicit note when no schema exists (QBP/RSP/batch)
- Testpanel enriched samples corrected to schema truth (17/17 pass strict validation): RDE recast as a CVX-coded influenza vaccine order (v2.5 tables RXE-2 to 0292 in every version), PR1-3 procedure-method code `C4`, DRG/DG1 coding-system cleanup, TXA-17/18 remap, case-sensitive vocabulary fixes
- Conformance test data corrected where proper structure exposed real values: FT1-7 coding system `CPT` is not a table 0396 code, now `C4`

### Added

- `Field::from_delimited` constructor (splits `~`/`^`/`&` into structure) plus `set_delimited_field` builder helpers; `from_value` documented as literal-only
- CLI `validate --format json` gains a `schema_applied` flag and fallback note
- Conformance suite: wire-escape regression check inside `assert_conformant` plus 7 composite-structure tests (53 total)

## [0.5.0] - 2026-10-13

### Fixed

- Validator no longer reports required components for empty repetitions: an empty field means absent in HL7, so parsed messages (which materialize empty wire positions) validate like sparse-built ones
- Field-level table checks are skipped when the schema assigns the same table to a component other than 1 (e.g. XPN name-type table 0200 on component 7); the component-level checks own those
- Field-level tables on CQ quantity/units fields are checked against the units (component 2) instead of the numeric quantity, so textbook values like QRD-7 `10^RD` validate
- `DFT` builders wire `transaction_code`/`transaction_amount` into FT1 (previously dropped by `build()`); `MDM` builders wire `unique_document_number` into TXA-12
- ADT/DFT/MDM delegates emit their own trigger in EVN-1 (shared shapes hardcoded the first trigger, e.g. A02 messages carried EVN-1 `A01`)

### Added

- Builder conformance suite (`vyna-hl7-validator/tests/builder_conformance.rs`, 46 tests): every message builder with a schema validates clean at v2.5 as built and after a strict parse round-trip
- New builder setters for schema-required content: ADT `patient_class` forwarding (A04/A08/A11/A12/A13/A17/A28/A31), DFT `transaction_date`/`transaction_type`, ORU `universal_service_id`, OUL `result_status`, QRY `quantity_limited_request`/`who_subject_filter`/`what_department_data_code`, MDM `patient_class`/`document_completion_status`/`observation_identifier`/`observation_value`/`result_status`, SIU `event_reason`/`filler_contact`/`entered_by`/`resource_action_code`, ACK `trigger_event` (auto-filled by `for_message`, so MSH-9 reads `ACK^A01`), RDE `give_amount_minimum`/`give_units`/`route`, RAS `administered_code`/`amount`/`datetime_start`/`datetime_end`/`route`, RDS `dispense_code`/`datetime_dispensed`/`dispense_amount`/`prescription_number`/`route`, RGV `give_amount_minimum`/`give_units`/`route`, RRA/RRD `ack_code`/`acknowledged_control_id`
- Builders now emit previously missing required segments: RGS (SIU), PV1 (MDM), OBX (MDM T02/T04), RXR/TQ1 (RDE/RGV), MSA (RRA/RRD)

### Changed

- **Breaking:** pharmacy builders emit their own message shapes instead of reusing the RDE shape: RAS emits RXA+RXR, RDS emits RXD+RXR, RGV emits RXG+RXR+TQ1, RRA/RRD emit MSA (+PID when fully identified). `RasO17Builder::give_code` is now `administered_code`, `RdsO13Builder::give_code` is now `dispense_code`, and `RraO18Builder`/`RrdO14Builder` lose `order_control`/`placer_order_number`/`give_code` (replaced by `ack_code`/`acknowledged_control_id`)

## [0.4.0] - 2026-10-13

### Removed

- DICOM support removed from the test panel (SCU/SCP tab, `dicom-*` dependencies) to keep the project HL7-only
- 22 validation schemas for message structures that do not exist in their version (10 pre-dating v2.3 pharmacy/laboratory triggers and DFT^P11; QRY/ORM/MFN^M01/OUL^R21 withdrawn from v2.7/v2.8 in favor of QBP/OMG/MFN^M04/OUL^R22); `list_available_schemas` is now version-accurate

### Fixed

- Validation schemas re-skinned against the HAPI structure database (segments) and hl7apy (fields/components), arbitrated where the oracles disagree: ~7.9k datatype corrections (v2.3 CM_* composites, IS/CWE migrations, TS/DTM), segment sets/flags aligned with HAPI, ~1.6k table backfills, bogus table codes dropped, `CON_` renamed to the real `CON` segment ID
- Vocabulary registry regenerated: 265 enforced tables (was 196), 28 site-defined tables documented, ISO 4217 currency codes added, hl7apy `9999` unknown-table sentinel purged from schemas

## [0.3.0] - 2026-10-12

### Fixed

- MSH-9.3 (Message Structure) is now optional in all schemas: real wires carry 2 MSH-9 components, and the library's own builders emit 2
- `AdtA40Builder` emits EVN-1 `A40` (was `A01`), the required PV1 segment, and the MRG segment with repeatable prior IDs/names

### Added

- WASM bindings accept HL7 versions 2.1 and 2.2

### Changed

- Dependencies updated to latest (`hmac` 0.13, `sha2` 0.11, `russh` 0.64)

## [0.2.0] - 2026-10-12

### Added

- Typed codegen model: schema-driven HAPI-style message structs
- Message coverage: VXU_V04 immunization, PPR_PC1 problems, OMP_O09 pharmacy orders, ADT_A34 merges (schemas, builders, validation)
- FHIR: declarative YAML mapping plus Bundle output
- New crates: Safe Harbor PHI redaction, durable store-and-forward queue, batch file exchange with FTP/SFTP transports, schema-driven cross-version conversion
- Serde `Serialize`/`Deserialize` derives for the core model
- Vocabulary: 183-table registry with code patterns and component-level table checks
- CLI: `send` (MLLP/HTTP) and `diff` commands plus a core message diff engine
- Typed HL7 timestamps with precision and UTC offsets
- Validation contexts: strict/standard/lenient severity profiles (`--strict`/`--lenient` CLI flags)
- Streaming MLLP codec (`tokio_util` `Decoder`/`Encoder`, `Framed` support)
- HL7 v2.1 (23 messages) and v2.2 (27 messages) validation schemas; legacy `PN/AD/TN/CN/CK/RI/CM` datatypes

### Fixed

- ID datatype rule rejected dotted values (false MSH-12 errors on 2.2/2.5+)

## [0.1.0] - 2026-10-12

### Changed

- Initial VynaHL7 release (Vyna Healthcare product family library), forked from rs7 1.6.1 with versioning restarted at 0.1.0
- Renamed project from rs7 to VynaHL7: all crates renamed from `rs7-*` to `vyna-hl7-*`, Rust paths from `rs7_*` to `vyna_hl7_*`, CLI binary to `vynahl7`, and documentation updated accordingly

[Unreleased]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.9.0...HEAD
[0.9.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/FlostelInfoTech/vynahl7/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/FlostelInfoTech/vynahl7/releases/tag/v0.1.0
