#!/usr/bin/env bash
#
# VynaHL7 Full Benchmark Suite Runner
#
# Runs all benchmarks and saves structured reports to bench-results/.
# Each report follows the 4-section format: Purpose, Methodology,
# Configuration, Results.
#
# Usage:
#   cd /path/to/vyna-hl7
#   bash bench-results/run_all.sh

set -uo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$DIR/.."

# ── Collect environment info ─────────────────────────────────────────────

collect_env() {
    local os kernel cpu cores ram rust cargo target
    os=$(grep PRETTY_NAME /etc/os-release 2>/dev/null | cut -d'"' -f2 || uname -o)
    kernel=$(uname -r)
    cpu=$(grep "model name" /proc/cpuinfo 2>/dev/null | head -1 | cut -d: -f2 | xargs || sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "unknown")
    cores=$(nproc 2>/dev/null || sysctl -n hw.logicalcpu 2>/dev/null || echo "unknown")
    ram=$(awk '/MemTotal/{printf "%.1f GB", $2/1048576}' /proc/meminfo 2>/dev/null || echo "unknown")
    rust=$(rustc --version)
    cargo=$(cargo --version)
    target=$(rustc -vV | grep host | awk '{print $2}')

    cat <<ENVEOF
  Hardware:
    OS:            $os
    Kernel:        $kernel
    CPU:           $cpu
    Cores:         $cores
    RAM:           $ram

  Toolchain:
    Rust:          $rust
    Cargo:         $cargo
    Target:        $target
    Profile:       release (opt-level=3, lto=true, codegen-units=1)
ENVEOF
}

ENV_INFO=$(collect_env)
TIMESTAMP=$(date -u +"%Y-%m-%d %H:%M:%S UTC")

# ── Helper: wrap criterion output with 4-section report ──────────────────

run_criterion_bench() {
    local name="$1"
    local pkg="$2"
    local bench="$3"
    local purpose="$4"
    local methodology="$5"
    local reproduce="$6"
    local outfile="$DIR/${bench}.txt"

    echo "  Running $name..."

    # Run benchmark, extract only benchmark name + timing/throughput lines
    local raw
    raw=$(cargo bench -p "$pkg" --bench "$bench" 2>&1 \
        | awk '
            /^[A-Z][A-Za-z_]+.*\// && !/^Benchmarking/ { name=$0 }
            /time:.*[µnm]s/ && !/change/ && !seen_time[name]++ { print name; print $0 }
            /thrpt:/ && !/change/ && !seen_thrpt[name]++ { print $0 }
          ' \
        | sed 's/^                        /    /')

    cat > "$outfile" <<HEADER
================================================================================
  VynaHL7 Benchmark Report: $name
  Generated: $TIMESTAMP
================================================================================

  1. PURPOSE
  ----------
$purpose

  2. METHODOLOGY
  ---------------
$methodology

  3. CONFIGURATION
  -----------------

$ENV_INFO

  Reproduce:
    $reproduce

  4. RESULTS
  ----------

$raw

================================================================================
HEADER
    echo "    -> $outfile"
}

run_standalone_bench() {
    local name="$1"
    local cmd="$2"
    local outfile="$DIR/$3"

    echo "  Running $name..."
    eval "$cmd" 2>&1 \
        | sed -n '/^=\{5,\}/,$ p' \
        > "$outfile"
    echo "    -> $outfile"
}

# ── Run all benchmarks ───────────────────────────────────────────────────

echo
echo "VynaHL7 Full Benchmark Suite"
echo "========================"
echo "Timestamp: $TIMESTAMP"
echo

# 1. Standalone benchmarks (already have 4-section structure)
run_standalone_bench "Allocation Profile" \
    "cargo bench -p vyna-hl7-parser --bench alloc_profile_bench" \
    "alloc_profile.txt"

run_standalone_bench "Transport (MLLP + HTTP)" \
    "cargo bench --bench transport_bench --features testing" \
    "transport.txt"

# 2. Criterion benchmarks (wrapped with 4-section headers)
run_criterion_bench \
    "Message Types Parse/RoundTrip" \
    "vyna-hl7-parser" \
    "message_types_bench" \
    "  Measure parse and round-trip (parse + encode) performance across 7 major HL7
  message types used in production healthcare systems: ADT^A01 (admission),
  ORU^R01 (lab results), RDE^O11 (pharmacy), SIU^S12 (scheduling), MDM^T02
  (documents), DFT^P03 (financial), ORM^O01 (orders). Throughput is reported
  in bytes/sec to normalize for message size differences." \
  "  Framework:     Criterion 0.8 (statistical microbenchmark)
  Sample size:   100 iterations per benchmark (Criterion default)
  Metric:        Wall-clock time per operation, byte throughput
  Parse:         vyna_hl7_parser::parse_message() on static HL7 string
  RoundTrip:     parse_message() + message.encode()" \
    "cargo bench -p vyna-hl7-parser --bench message_types_bench"

run_criterion_bench \
    "Parser Edge Cases" \
    "vyna-hl7-parser" \
    "edge_cases_bench" \
    "  Measure parser performance under non-ideal conditions: varying escape sequence
  density (0-50%), batch/file envelope parsing (1-50 messages), lenient vs strict
  parsing mode, and production-like messy messages with trailing delimiters and
  empty components." \
    "  Framework:     Criterion 0.8
  Escape test:   Synthetic messages with 0%, 5%, 20%, 50% escaped field values
  Batch test:    BHS/BTS envelope with 1/5/10/50 ADT messages via parse_batch()
  File test:     FHS/FTS envelope with 2 batches of 10 messages via parse_file()
  Config test:   ParserConfig::strict() vs ParserConfig::lenient() on messy input" \
    "cargo bench -p vyna-hl7-parser --bench edge_cases_bench"

run_criterion_bench \
    "Core Encoding & Builders" \
    "vyna-hl7-core" \
    "core_bench" \
    "  Measure encoding performance isolated from parsing, escape sequence encode/decode
  throughput at varying density, and message builder construction cost. This separates
  encoding overhead from parse overhead and quantifies the cost of the builder pattern
  for programmatic message creation." \
    "  Framework:     Criterion 0.8
  Encode:        message.encode() on pre-parsed Message objects (no parse cost)
  Escapes:       Encoding::encode() and Encoding::decode() with plain/light/heavy input
  Builders:      AdtA01Builder and OruR01Builder full construction + build()" \
    "cargo bench -p vyna-hl7-core --bench core_bench"

run_criterion_bench \
    "Validation" \
    "vyna-hl7-validator" \
    "validator_bench" \
    "  Measure validation performance across VynaHL7's three-layer validation system: schema
  validation (message structure against HL7 standards), data type validation (format
  checking for DT, TM, DTM, NM, SI, ST, CWE, XPN), vocabulary validation (code set
  validation against HL7 tables 0001, 0004, 0085, 0103), and full parse+validate
  pipeline cost." \
    "  Framework:     Criterion 0.8
  Schema load:   Validator::for_message_type() (includes JSON schema parsing)
  Validate:      validator.validate() on pre-parsed Message with pre-loaded schema
  DataType:      validate_data_type() for individual field values
  Vocabulary:    TableRegistry::validate() for code lookups
  Pipeline:      parse_message() + validator.validate() combined" \
    "cargo bench -p vyna-hl7-validator --bench validator_bench"

run_criterion_bench \
    "FHIR Conversion" \
    "vyna-hl7-fhir" \
    "fhir_bench" \
    "  Measure HL7 v2 to FHIR R4 conversion performance for 9 resource converters
  (Patient, Observation, Encounter, DiagnosticReport, Practitioner, AllergyIntolerance,
  Condition, Procedure), JSON serialization of converted resources, and full
  parse-to-FHIR-to-JSON pipeline cost." \
    "  Framework:     Criterion 0.8
  Converters:    Each converter benchmarked on pre-parsed Message (no parse cost)
  Serialization: serde_json::to_string() on converted FHIR resources
  Pipeline:      parse_message() + convert all applicable resources + serialize JSON" \
    "cargo bench -p vyna-hl7-fhir --bench fhir_bench"

run_criterion_bench \
    "CachedTerser vs Terser" \
    "vyna-hl7-terser" \
    "cached_terser_bench" \
    "  Compare CachedTerser (HashMap-cached path lookups) against standard Terser for
  repeated field access patterns. Measures clinical workflow extraction scenarios:
  patient demographics (6 paths), lab results summary (32 paths across 8 OBX segments),
  and order info (5 paths). Also tests scaling behavior with 10/50/100 unique paths." \
    "  Framework:     Criterion 0.8
  Comparison:    Same 10 paths accessed 5 times each, Terser vs CachedTerser
  Workflows:     Realistic clinical extraction patterns on pre-parsed ORU message
  Scaling:       Increasing unique path counts with cache hit on second pass" \
    "cargo bench -p vyna-hl7-terser --bench cached_terser_bench"

run_criterion_bench \
    "Full Pipeline" \
    "vyna-hl7" \
    "pipeline_bench" \
    "  Measure cumulative cost of cross-crate processing pipelines that represent real
  interface engine workflows: parse-only (baseline), parse+validate, parse+extract
  (Terser vs CachedTerser), parse+transform+encode (field modification round-trip),
  and interface engine simulation (parse + extract routing fields + generate ACK)." \
    "  Framework:     Criterion 0.8
  Parse only:    vyna_hl7_parser::parse_message() baseline
  Validate:      parse + Validator::for_message_type().validate()
  Extract:       parse + Terser/CachedTerser get 10 clinical fields
  Transform:     parse + TerserMut set fields + message.encode()
  Engine sim:    parse + extract MSH-9/PID-3/PV1-2 + generate ACK string" \
    "cargo bench --bench pipeline_bench"

echo
echo "All benchmarks complete. Results saved to bench-results/"
echo
