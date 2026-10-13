# VynaHL7 Documentation

Living SDLC documentation for the VynaHL7 HL7 v2.x workspace (23 crates).
The project README is the front door; this directory holds the durable
specifications. API reference lives in rustdoc (`cargo doc`).

## Map

| Document | Audience | Contents |
|---|---|---|
| [Requirements](requirements.md) (SRS) | Product, QA, integrators | Numbered functional/non-functional requirements with verification pointers |
| [Design](design.md) (SDS) | Contributors, auditors | Architecture, crate catalog, data model, component designs, key decisions |
| [Integration guide](integration.md) | Integrators | Install, quickstarts, transports/TLS, CLI, site-table overrides, troubleshooting |
| [SBOM](sbom/) | Security, compliance | CycloneDX software bill of materials + regeneration instructions |
| [RFCs](rfcs/) | Contributors | Design proposals for significant changes (start here before big work) |
| [Schema provenance](../crates/vyna-hl7-validator/schemas/README.md) | Contributors | How the 310 validation schemas were produced and what the loader does |
| [Performance](performance.md) | Integrators | Benchmark results and methodology |
| `rs7-*.md` (this dir) | Historians | Archived pre-fork history, preserved verbatim; not living docs |

## Living-document rules

These documents are part of the definition of done. When you change code,
update the matching document in the same commit:

- New builder or message support → SRS requirement ID + SDS crate/builder
  section + conformance test (see `builder_conformance.rs`).
- New validation behavior or severity → SRS validation section + SDS
  validator section + rustdoc on the API.
- New crate, feature flag, or public API → SDS crate catalog + integration
  guide if integrator-facing.
- New dependency → regenerate the SBOM (`docs/sbom/`).
- Behavior fix → CHANGELOG entry; spec fix if a requirement was wrong.

Requirements are numbered (`REQ-AREA-NNN`) so tests, commits, and reviews
can cite them. Mark roadmap items explicitly instead of deleting them.
