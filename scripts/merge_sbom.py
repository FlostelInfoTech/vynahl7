#!/usr/bin/env python3
"""Merge per-crate CycloneDX SBOMs into one workspace SBOM.

cargo-cyclonedx emits one file per workspace member; a release needs a
single supply-chain record. This unions components and dependency edges by
bom-ref (cargo-cyclonedx bom-refs are unique `pkg:cargo/name@version`
coordinates) and keeps the root file's metadata.

Usage (from the workspace root, after generating per-crate files):
    python3 scripts/merge_sbom.py vynahl7-sbom.json docs/sbom/vynahl7-X.Y.Z.cdx.json
"""

import json
import sys

root_path, out_path = sys.argv[1], sys.argv[2]
root = json.load(open(root_path))

components = {}
dependencies = {}
for path in [root_path] + sorted(__import__("glob").glob("crates/*/vynahl7-sbom.json")):
    bom = json.load(open(path))
    # Each member describes itself in metadata, not components: hoist it so
    # every first-party crate appears in the merged record.
    meta = (bom.get("metadata") or {}).get("component")
    if meta and "bom-ref" in meta:
        components.setdefault(meta["bom-ref"], meta)
    for comp in bom.get("components", []):
        components.setdefault(comp["bom-ref"], comp)
    for dep in bom.get("dependencies", []):
        dependencies.setdefault(dep["ref"], dep)

root["components"] = [components[k] for k in sorted(components)]
root["dependencies"] = [dependencies[k] for k in sorted(dependencies)]
json.dump(root, open(out_path, "w"), indent=2)
print(f"merged {len(components)} components from workspace SBOMs -> {out_path}")
