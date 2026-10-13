# SBOM — Software Bill of Materials

`vynahl7-0.9.0.cdx.json`: CycloneDX 1.5 SBOM for the 0.9.0 workspace —
all 23 member crates plus their dependency closures, all features, all
targets (842 components, including the optional TLS stack). It is the
supply-chain record for the release; do not hand-edit it.

## Regenerate

Requires `cargo-cyclonedx` (`cargo install cargo-cyclonedx`).
`cargo-cyclonedx` emits one file per workspace member, so generation is
two steps — emit, then merge (`scripts/merge_sbom.py` unions components
and dependency edges by bom-ref and hoists each member's own record):

```bash
# From the workspace root, at release time.
cargo cyclonedx -f json --target all --spec-version 1.5 \
  --all-features --override-filename vynahl7-sbom
python3 scripts/merge_sbom.py vynahl7-sbom.json \
  docs/sbom/vynahl7-<VERSION>.cdx.json
# Remove the per-crate intermediates (root + crates/*/).
rm vynahl7-sbom.json crates/*/vynahl7-sbom.json
```

Keep one SBOM per release (replace, don't accumulate) and regenerate
whenever dependencies change. For public releases, anonymize the
machine-specific `path+file://` prefix first (replace the absolute
workspace path with `path+file:///vynahl7`) so contributor machine
layouts don't leak into `bom-ref`s.
