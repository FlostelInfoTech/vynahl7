#!/usr/bin/env bash
# Publish all VynaHL7 crates to crates.io in dependency order.
#
# Prerequisites:
#   - Run from the workspace root on the oss-release branch (MIT license).
#   - `cargo login <CRATES_IO_TOKEN>` first (token needs publish scope).
#   - Network access to crates.io.
#
# The script is resumable: already-published crates are skipped.
set -u

# Topological order over required dependencies (dev-deps excluded:
# core/tests and benches intentionally avoid the parser dev-dep).
CRATES=(
  vyna-hl7-core
  vyna-hl7-macros
  vyna-hl7-parser
  vyna-hl7-queue
  vyna-hl7-redact
  vyna-hl7-templates
  vyna-hl7-terser
  vyna-hl7-transform
  vyna-hl7-validator
  vyna-hl7-wasm
  vyna-hl7-xml
  vyna-hl7-batch
  vyna-hl7-codegen
  vyna-hl7-conformance
  vyna-hl7-convert
  vyna-hl7-custom
  vyna-hl7-fhir
  vyna-hl7-http
  vyna-hl7-mllp
  vyna-hl7-orchestration
  vyna-hl7-testpanel
  vyna-hl7
  vyna-hl7-cli
)

VERSION="$(cargo metadata --format-version 1 --no-deps --offline 2>/dev/null \
  | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "vyna-hl7-core"))')"
if [ -z "$VERSION" ]; then
  echo "error: could not determine workspace version (run from the workspace root with python3 installed)" >&2
  exit 1
fi
echo "Publishing ${#CRATES[@]} crates at v$VERSION"

already_published() {
  # crates.io rejects API requests without a User-Agent (403);
  # without -A this check always fails and resumability breaks.
  curl -sf -A "vynahl7-publish-script" -o /dev/null --max-time 20 "https://crates.io/api/v1/crates/$1/$2"
}

FAILED=()
for crate in "${CRATES[@]}"; do
  if already_published "$crate" "$VERSION"; then
    echo "== $crate $VERSION already published, skipping"
    continue
  fi
  echo "== publishing $crate $VERSION ($(date -u +%H:%M:%S))"
  ok=0
  for attempt in 1 2 3 4 5 6 7 8 9 10 11 12; do
    if cargo publish -p "$crate"; then ok=1; break; fi
    echo "   attempt $attempt failed, waiting 60s for index propagation..."
    sleep 60
  done
  if [ "$ok" -ne 1 ]; then FAILED+=("$crate"); break; fi
  # Pace uploads: crates.io rate-limits new crates (rolling ~5 per
  # 10 min window); 300s holds us near 2 per 10 min even mid-window.
  sleep 300
done

if [ "${#FAILED[@]}" -ne 0 ]; then
  echo "FAILED: ${FAILED[*]} (and everything after it). Fix and re-run; published crates are skipped."
  exit 1
fi
echo "All ${#CRATES[@]} crates published."
