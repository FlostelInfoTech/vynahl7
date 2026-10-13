# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

- Initial VynaHL7 release (Vyna Healthcare product family internal library), forked from rs7 1.6.1 with versioning restarted at 0.1.0
- Renamed project from rs7 to VynaHL7: all crates renamed from `rs7-*` to `vyna-hl7-*`, Rust paths from `rs7_*` to `vyna_hl7_*`, CLI binary to `vynahl7`, and documentation updated accordingly

[Unreleased]: https://gitlab.flostel.com/alexshao/VynaHL7/compare/v0.4.0...HEAD
[0.4.0]: https://gitlab.flostel.com/alexshao/VynaHL7/compare/v0.3.0...v0.4.0
[0.3.0]: https://gitlab.flostel.com/alexshao/VynaHL7/compare/v0.2.0...v0.3.0
[0.2.0]: https://gitlab.flostel.com/alexshao/VynaHL7/compare/v0.1.0...v0.2.0
[0.1.0]: https://gitlab.flostel.com/alexshao/VynaHL7/releases/tag/v0.1.0
