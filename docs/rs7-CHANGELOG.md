# rs7 Changelog (Archived)

> This is the pre-fork rs7 changelog, preserved verbatim when the rs7
> git history was squashed out of VynaHL7. Version-compare links below
> reference retired rs7 tags and are kept for historical context only.
> For the inherited feature summary, see rs7-feature-inventory.md.

## [1.6.0] - 2026-09-29

### Added

- Test panel Builder template mode: built-in template picker with auto-generated `{{variable}}` fields covering ADT, ORU, ORM, SIU, and MDM triggers
- Builder message options: HL7 version (MSH-12) and character set (MSH-18) pickers for both form and template modes
- Builder can save messages to file (wire format, charset-aware encoding) and load a message file back into template variables
- Builder can save the current template with values baked in as defaults, and load template files (YAML/JSON)
- Template engine fills MSH-9 and MSH-12 from template metadata when the template leaves them empty

### Changed

- All third-party dependencies updated to latest compatible majors (egui/eframe 0.36, russh 0.63, suppaftp 12, libunftp 0.23, quick-xml 0.42, reqwest 0.13, syn 3, rand 0.10, and others)
- Template engine: substituted values parse as ER7 fragments (repetitions/components/subcomponents with escape decoding), as the documented examples assume
- Test panel migrated to the egui 0.36 `App::ui` and `Panel` APIs

### Fixed

- Template engine segment layout: template position N now lands in HL7 field N (previously the segment ID was emitted as an extra field, duplicating IDs and shifting MSH)
- Template validator field indexing now uses HL7 1-based numbering
- vyna-hl7-http doctests and the `jwt_middleware` layering bound (`Send` boxed errors); reqwest 0.13 also resolved the TLS client-config test failure

## [1.5.0] - 2026-09-29

### Added

- Character-set support for inbound and outbound messages (MSH-18, HL7 table 0211)
- New `vyna_hl7_core::charset::CharacterSet`: ASCII, UNICODE UTF-8, and 8859/1-8859/11/8859/13-8859/16 with strict, lossless conversion (no silent replacements)
- `vyna_hl7_parser::parse_message_bytes` decodes wire bytes per MSH-18; `Message::encode_bytes` encodes outbound messages in the declared set
- MLLP send, receive, and framing honor MSH-18, with a per-peer charset override; CLI file/stdin input decodes per MSH-18 with a `--charset` flag
- HTTP client and server send and honor `Content-Type` charset parameters
- Message builders accept a character set; generated ACKs echo the inbound MSH-18 declaration
- Conformance validator flags content that cannot be encoded in the declared MSH-18 set (new `UnmappableCharacter` error)
- Test panel file loading is charset-aware and shows the declared character set
- Test panel samples for ADT^A02/A04/A11/A12/A13, all enriched to mimic EMR output, plus a UTF-8 i18n ADT^A01 sample with Chinese and Japanese names

### Changed

- Terser field-repetition paths are now 1-based: `PID-5(1)` addresses the first repetition, consistent with segment indices, component indices, and the documented behavior; `PID-5(0)` is now an error. Queries using the old 0-based repetition numbering return different results and must be updated
- VynaHL7 is now private, proprietary software; public open-source development has ended
- License changed from MIT OR Apache-2.0 to proprietary (LICENSE-MIT and LICENSE-APACHE removed)
- All crates set `publish = false` to block accidental publication to crates.io
- Repository, homepage, and documentation URLs point to the private GitLab remote

## [1.0.2] - 2026-03-21

### Fixed

- Standardize all 9 benchmark reports to consistent 4-section format (Purpose, Methodology, Configuration, Results)
- Add `run_all.sh` script to regenerate all benchmark reports in one pass
- Fix transport bench: replace "mock server" terminology with accurate "VynaHL7 MllpServer / HttpServer"
- Clean criterion output: strip progress, warmup, outlier noise — keep only benchmark name + timing + throughput

## [1.0.1] - 2026-03-21

### Fixed

- Fix middleware doctests for axum 0.8.8 Router type inference change
- Sync vyna-hl7-transform regex dependency to workspace version (was pinned to 1.11, workspace uses 1.12)
- Fix vyna-hl7-wasm package.json version (0.6.0 -> 1.0.0) and repository URL (GitLab -> GitHub)
- Fix vyna-hl7-testpanel to use `version.workspace = true` instead of hardcoded version

## [1.0.0] - 2026-03-21

### Added - Healthcare-Grade Benchmark Framework

- **Comprehensive Benchmark Suite** — 9 new benchmark files covering the full VynaHL7 pipeline:
  - `message_types_bench` — Parse and round-trip across 7 HL7 message types (ADT, ORU, RDE, SIU, MDM, DFT, ORM)
  - `edge_cases_bench` — Escape sequence density, batch/file parsing, lenient vs strict mode
  - `core_bench` — Isolated encoding, escape encode/decode, builder construction
  - `validator_bench` — Schema loading/validation, data type validation, vocabulary validation
  - `fhir_bench` — 9 FHIR converters, JSON serialization, full parse-to-FHIR pipeline
  - `cached_terser_bench` — CachedTerser vs Terser comparison, clinical workflow extraction patterns
  - `pipeline_bench` — Cross-crate pipeline (parse, validate, extract, transform, encode)
  - `transport_bench` — End-to-end MLLP and HTTP transport with real TCP network I/O
  - `alloc_profile_bench` — Memory allocation profiling with per-parse heap tracking

- **Shared Message Corpus** (`corpus.rs`) — 9 realistic HL7 messages plus batch/file generators

- **Memory Allocation Profiler** — Custom `GlobalAlloc` wrapper that tracks allocations per parse and reports amplification factor (heap bytes / input bytes)

- **Environment Reporter** — Automatic system info collection (CPU, RAM, OS, Rust toolchain) included in benchmark output

- **Structured Benchmark Reports** — All standalone benchmarks output 4-section reports:
  1. Purpose — what is measured and why
  2. Methodology — how the measurement is taken
  3. Configuration — hardware, toolchain, parameters, reproduction command
  4. Results — tabular data with latency percentiles

### Changed

- **Version 1.0.0** — First stable release. Unified workspace versioning across all 16 crates.
- **vyna-hl7-testpanel** now uses `version.workspace = true` instead of hardcoded version
- Removed third-party engine throughput numbers from benchmark reports (methodology references retained)

## [0.22.0] - 2025-12-07

### Added - Performance Optimization & E2E Benchmarking 🚀

- **MLLP High-Throughput Optimization** - 8.9x performance improvement for network message transmission:
  - **TCP_NODELAY**: Disabled Nagle's algorithm for immediate packet transmission
  - **Chunk-Based I/O**: Process incoming data in 4KB chunks instead of byte-by-byte
  - **Buffered Writes**: Use BufWriter for efficient ACK responses
  - **Results**: Throughput improved from 652 msg/s to 5,831 msg/s
  - **Latency**: Single message round-trip reduced from ~1.5ms to ~170µs

- **E2E Benchmarking Framework** - Comprehensive end-to-end performance testing:
  - Real network I/O through MLLP with MockMllpServer
  - Test messages: ADT^A01 (316 bytes, 3 segments), ORU^R01 (814 bytes, 13 segments)
  - Multiple test scenarios:
    - T1: Single client sequential (10,000 iterations)
    - T2: 4 concurrent clients (20,000 total messages)
    - T3: Connection establishment overhead
  - Latency percentile analysis (p50, p90, p95, p99, p99.9)
  - Feature flag: `testing` for benchmark infrastructure

- **Performance Documentation** - Comprehensive PERFORMANCE.md updates:
  - E2E Testing Setup section with test architecture
  - Complete test results summary with before/after metrics
  - Measurement methodology documentation
  - Connection management details

### Fixed

- **Test Panel Tree View Composite Fields** - Display full composite values correctly:
  - MSH-9 now shows "ADT^A01^ADT_A01" instead of just "ADT"
  - All fields, repetitions, and components display complete values
  - Changed from `value()` (first subcomponent only) to `encode()` (full composite)

- **Test Panel Delimiter Display** - Show delimiters without HL7 escape encoding:
  - MSH-2 now shows `^~\&` instead of `^~\E\&`
  - Added display helper functions for UI rendering without escape sequences
  - Tree view displays raw values for better readability

### Changed

- Updated workspace version to 0.22.0
- Updated vyna-hl7-testpanel version to 0.22.0

## [0.19.0] - 2025-01-20

### Added - Phase 4: Testing Infrastructure & TLS/mTLS Security 🔒

- **TLS/mTLS Support for MLLP** - Secure network transmission with Transport Layer Security:
  - **TlsServerConfig** - Server-side TLS configuration
    - `new(cert_path, key_path)` - Basic TLS with server certificate
    - `with_mtls(cert_path, key_path, ca_cert_path)` - Mutual TLS with client certificate verification
    - X.509 v3 certificate support with proper extensions
    - rustls-based implementation for memory safety
  - **TlsClientConfig** - Client-side TLS configuration
    - `new()` - Use system root certificates
    - `with_ca_cert(ca_cert_path)` - Trust specific CA certificate
    - `with_mtls(ca_cert_path, client_cert_path, client_key_path)` - Client certificate authentication
    - Server Name Indication (SNI) support
  - **MllpServer TLS methods**:
    - `serve_tls(listener, tls_config, handler)` - Serve with TLS
    - Automatic TLS handshake and stream wrapping
    - Support for concurrent TLS connections
  - **MllpClient TLS methods**:
    - `connect_tls(addr, domain, tls_config)` - Connect with TLS
    - SNI hostname verification
    - Certificate validation
  - Feature flag: `tls` (optional dependency on tokio-rustls, rustls, rustls-pemfile, webpki-roots)
  - Examples: `mllp_tls_server.rs`, `mllp_tls_client.rs`

- **TLS/mTLS Support for HTTP** - Secure HTTP transport with HTTPS:
  - **TlsServerConfig** - HTTPS server configuration (shared with MLLP)
    - Basic TLS and mutual TLS support
    - Reusable configuration across protocols
  - **TlsClientConfig** - HTTPS client configuration (shared with MLLP)
    - CA certificate verification
    - Client certificate authentication
  - **HttpServer TLS methods**:
    - `with_tls(tls_config)` - Configure TLS for server
    - `serve_tls()` - Serve HTTPS traffic
  - **HttpClient TLS methods**:
    - `new_tls(url, tls_config)` - Create HTTPS client
    - Certificate verification and hostname validation
  - Feature flag: `tls` (optional dependency on native-tls or rustls)
  - Examples: `http_tls_server.rs`, `http_tls_client.rs`

- **MockMllpServer** - In-process MLLP test server for integration testing:
  - Automatic port allocation (bind to "127.0.0.1:0")
  - Configurable message handlers with `with_handler()`
  - TLS support via `with_tls(tls_config)`
  - Graceful shutdown with `shutdown()`
  - Automatic cleanup via Drop trait
  - URL and address accessors for client connections
  - Default echo handler (returns received message)
  - Zero external dependencies for testing
  - Located in `vyna-hl7-mllp/src/testing.rs`

- **MockHttpServer** - In-process HTTP test server for integration testing:
  - Automatic port allocation for test isolation
  - Custom message handlers via `with_handler()`
  - TLS support via `with_tls(tls_config)` (HTTPS)
  - Compression support via `with_compression()` (optional feature)
  - HTTP Basic Authentication via `with_auth(username, password)`
  - Graceful shutdown and cleanup
  - URL accessor for client connections
  - Built on axum for production-grade HTTP handling
  - Located in `vyna-hl7-http/src/testing.rs`

- **TLS Integration Tests** - Comprehensive test coverage for TLS/mTLS functionality:
  - **8 Integration Tests** in `vyna-hl7-mllp/tests/tls_integration.rs`:
    - `test_tls_basic_connection` - Basic TLS handshake and message exchange
    - `test_tls_custom_handler` - Custom message handlers over TLS
    - `test_mtls_with_client_cert` - Mutual TLS with client certificates
    - `test_tls_multiple_messages` - Multiple messages over same TLS connection
    - `test_tls_connection_refused_without_client_ca` - Security validation
    - `test_tls_concurrent_connections` - 5 concurrent TLS connections
    - Plus 2 certificate generation validation tests
  - All tests passing with proper X.509 v3 certificate extensions

- **Test Certificate Generation Utilities** - Automatic certificate creation for testing:
  - **TestCerts** struct - CA and server certificate bundle
    - Automatic CA certificate generation with v3_ca extensions
    - Server certificate with proper X.509 v3 extensions
    - subjectAltName with DNS:localhost and IP:127.0.0.1
    - extendedKeyUsage for serverAuth
    - Automatic cleanup via Drop trait
  - **TestCertsWithClient** struct - Complete mTLS certificate bundle
    - Includes client certificate for mutual TLS testing
    - Client certificate with clientAuth extendedKeyUsage
    - All certificates signed by test CA
  - **Certificate Generation Functions**:
    - `generate_test_certs()` - Generate CA and server certificates
    - `generate_test_certs_with_client()` - Generate full mTLS certificate set
  - Uses OpenSSL command-line tool for certificate generation
  - Unique temporary directories with UUID for test isolation
  - Located in `vyna-hl7-mllp/tests/test_certs.rs`

- **Dependencies**:
  - Added `tokio-rustls = "0.26"` (optional, tls feature)
  - Added `rustls = "0.23"` (optional, tls feature)
  - Added `rustls-pemfile = "2.2"` (optional, tls feature)
  - Added `webpki-roots = "1.0"` (optional, tls feature)
  - Added `uuid = "1.11"` (dev-dependencies for test certificates)

### Changed

- **vyna-hl7-mllp** crate:
  - Added `testing` module with MockMllpServer
  - Added `tls` module with TLS configuration types
  - New features: `tls`, `testing`, `full` (meta feature)
  - Updated examples to demonstrate TLS usage

- **vyna-hl7-http** crate:
  - Added `testing` module with MockHttpServer
  - Added `tls` module with shared TLS configuration
  - TLS support integrated into HttpServer and HttpClient
  - Feature flags: `tls`, `compression`, `auth`

### Documentation

- **README.md Updates**:
  - Added "Testing Infrastructure" to features list
  - Updated version numbers from 0.18 to 0.19 in examples
  - Added comprehensive TLS/mTLS examples for MLLP
  - Added comprehensive TLS/mTLS examples for HTTP
  - Added testing infrastructure section with MockMllpServer and MockHttpServer usage

- **CLAUDE.md Updates**:
  - Expanded "Testing Patterns" section with integration testing best practices
  - Added TLS Integration Tests documentation
  - Added 8 specific testing best practices guidelines
  - Updated "Network Protocols" section with TLS/mTLS support details
  - Feature flags documentation for testing

### Testing

- **Test Coverage**:
  - 8 comprehensive TLS integration tests for MLLP
  - 2 certificate generation validation tests
  - All tests passing with proper X.509 v3 extensions
  - Zero warnings across workspace

### Technical Details

- **X.509 v3 Certificate Requirements**:
  - Certificates must include proper extensions for rustls compatibility
  - basicConstraints, keyUsage, extendedKeyUsage required
  - subjectAltName required for server certificates (DNS and IP)
  - Extension config files created dynamically during test setup

- **Security**:
  - TLS 1.2 and TLS 1.3 support via rustls
  - Certificate verification and hostname validation
  - Mutual TLS (mTLS) for bidirectional authentication
  - Memory-safe TLS implementation (no OpenSSL runtime dependency)

- **Testing Features**:
  - Feature flag `testing` enables mock servers
  - Feature flag `tls` enables TLS support
  - Use `--features "tls,testing"` for TLS integration tests
  - Mock servers use automatic port allocation for parallel test execution

### Phase 4 Complete

This release completes Phase 4 of VynaHL7's enhanced feature roadmap:
- ✅ Phase 4.1: TLS/mTLS Support (MLLP and HTTP)
- ✅ Phase 4.2: Testing Infrastructure (Mock Servers)
- ✅ Phase 4.3: Integration Tests & Certificate Utilities
- Next: Phase 5 and beyond (TBD)

## [0.18.0] - 2025-01-20

### Added - Phase 3.3 Message Routing & Orchestration 🔄

- **vyna-hl7-orchestration** - New crate for message routing and workflow orchestration:
  - **ContentRouter** - Content-based message routing:
    - `add_route(name, condition, handler)` - Add route with predicate and async handler
    - `set_default_handler(handler)` - Set fallback handler for unmatched messages
    - `route(message)` - Route to first matching handler
    - `route_all(message)` - Route to all matching handlers
    - `route_count()` - Get number of configured routes
    - `clear()` - Remove all routes
  - **MessageOrchestrator** - Multi-step async workflow engine:
    - `add_step(name, handler)` - Add orchestration step
    - `add_step_with_retry(name, handler, config)` - Add step with retry logic
    - `set_error_handler(handler)` - Set error handler for failed steps
    - `execute(message)` - Execute workflow (fail-fast on errors)
    - `execute_continue_on_error(message)` - Execute all steps regardless of errors
    - `step_count()` - Get number of steps
    - `clear()` - Remove all steps
  - **RetryConfig** - Configurable retry behavior:
    - `none()` - No retries (1 attempt)
    - `standard()` - 3 attempts with 100ms delay
    - `aggressive()` - 5 attempts with 50ms delay
    - `new(attempts, delay_ms)` - Custom retry configuration
    - Exponential backoff support
  - **MessageFilter** - Predicate-based message filtering:
    - `new()` - Create filter with ALL mode (AND logic)
    - `new_any()` - Create filter with ANY mode (OR logic)
    - `add_rule(name, predicate)` - Add filter rule
    - `matches(message)` - Check if message passes filters
    - `filter(message)` - Filter with Result return
    - `filter_count()` - Get number of filters
    - `clear()` - Remove all filters
  - **FilterMode** - Filter combination modes:
    - `All` - All filters must pass (AND logic)
    - `Any` - At least one filter must pass (OR logic)
  - **OrchestrationError** - Structured error handling:
    - `NoMatchingRoute` - No route found for message
    - `RouteExecutionFailed` - Route handler failed
    - `StepExecutionFailed` - Orchestration step failed
    - `FilterFailed` - Message filter rejected message
    - `RetryLimitExceeded` - Retry attempts exhausted
    - `Custom` - Custom error messages

- **Examples**:
  - `routing_example` - Content-based routing demonstration
  - `filtering_example` - Message filtering with ALL/ANY modes
  - `workflow_example` - Complete workflow with filtering, orchestration, and routing

- **Testing**:
  - 18 unit tests covering all orchestration features
  - 6 doctests for public API examples
  - Tests for retry logic, error handling, and concurrent routing

## [0.17.0] - 2025-01-20

### Added - Phase 3.2 Advanced Validation Rules 📋

- **Rules Engine** - Flexible business validation framework:
  - **RulesEngine** - Execute custom validation rules against messages
    - `add_rule(rule)` - Add single validation rule
    - `add_rules(rules)` - Add multiple rules at once
    - `validate(message)` - Execute all rules and return violations
    - `rule_count()` - Get number of loaded rules
    - `clear()` - Remove all rules
  - **ValidationRule** - Custom rule definition
    - Rule name and description
    - Severity level (Error, Warning, Info)
    - Closure-based condition evaluation
    - Thread-safe with Arc<dyn Fn>
    - `with_condition(closure)` - Set rule condition
    - `evaluate(message)` - Execute rule against message
  - **RuleSeverity** - Classification of violations
    - Error - Validation failure
    - Warning - Validation concern but not failure
    - Info - Informational message
    - Ordering support for severity levels
    - `from_str()` - Parse from string
    - `as_str()` - Convert to string
  - **RulesValidationResult** - Validation outcome
    - List of rule violations
    - `passed()` - Check if validation passed (no errors)
    - `errors()` - Get all error violations
    - `warnings()` - Get all warning violations
    - `infos()` - Get all info violations
  - **RuleViolation** - Individual violation record
    - Rule name, severity, and message
    - Optional location field
    - `with_location()` - Add location context

- **Cross-Field Validation Patterns** - Pre-built validation helpers:
  - **CrossFieldValidator** - Common validation patterns
    - `if_then(field, value, then_field)` - Conditional field requirements
    - `mutually_exclusive(fields)` - Fields cannot both be valued
    - `at_least_one(fields)` - At least one field must be valued
    - `all_or_none(fields)` - All fields valued or all empty
    - `field_valued(field)` - Field presence check
    - `field_in_set(field, values)` - Value set validation
    - `dependent_fields(primary, dependent)` - Field dependencies
  - Integration with vyna-hl7-terser for field access
  - Fluent builder API for all patterns

- **Declarative Rule Configuration** - YAML/JSON rule loading:
  - **RuleConfig** - Root configuration structure
    - `from_yaml_file(path)` - Load from YAML file
    - `from_yaml_str(content)` - Load from YAML string
    - `from_json_file(path)` - Load from JSON file
    - `from_json_str(content)` - Load from JSON string
    - `into_validation_rules()` - Convert to ValidationRule instances
  - **RuleDefinition** - Single rule configuration
    - Rule name, description, and severity
    - Condition configuration
    - `into_validation_rule()` - Convert to ValidationRule
  - **ConditionConfig** - Condition type definitions
    - FieldValued - Field must have value
    - IfThen - Conditional field requirement
    - MutuallyExclusive - Exclusive field set
    - AtLeastOne - Minimum valued fields
    - AllOrNone - Complete or empty groups
    - FieldInSet - Value set membership
    - DependentFields - Field dependencies
  - Serde-based YAML/JSON parsing
  - Tagged enum for condition types

- **Built-in Rules Library** - Common HL7 validation rules:
  - **BuiltinRules** - Pre-built rule collections
    - `msh_rules()` - MSH segment validation (6 rules)
    - `pid_rules()` - PID segment validation (4 rules)
    - `pv1_rules()` - PV1 segment validation (4 rules)
    - `obr_rules()` - OBR segment validation (4 rules)
    - `obx_rules()` - OBX segment validation (4 rules)
    - `orc_rules()` - ORC segment validation (2 rules)
    - `adt_rules()` - ADT message validation (MSH + PID + PV1)
    - `oru_rules()` - ORU message validation (MSH + PID + OBR + OBX)
    - `orm_rules()` - ORM message validation (MSH + PID + ORC)
    - `all_rules()` - All built-in rules (24 total)
  - Comprehensive segment-level rules
  - Message-type specific rule sets
  - Mixed severity levels (errors and warnings)

- **Validator Integration** - Rules engine integration with existing validator:
  - Extended `Validator` struct with optional rules engine
  - `with_rules_engine(engine)` - Builder method to add rules engine
  - `add_rule(rule)` - Add single rule to validator
  - `add_rules(rules)` - Add multiple rules to validator
  - `rules_engine()` - Get reference to rules engine
  - `rules_engine_mut()` - Get mutable reference to rules engine
  - Automatic rule execution during validation
  - Merging of schema validation and business rules results
  - Conversion of rule violations to validation errors/warnings

### Changed

- **vyna-hl7-validator** crate:
  - Added vyna-hl7-terser dependency for cross-field validation
  - Added serde_yaml dependency for YAML rule loading
  - Enhanced Validator struct with rules engine support
  - Extended validation process to include business rules

### Examples

- **rules_basic.rs** - Basic rules engine usage
  - Simple field validation
  - Cross-field validation patterns
  - Custom rules with closures
  - Mixed severity levels
- **rules_declarative.rs** - Declarative rule loading
  - YAML rule configuration
  - JSON rule configuration
  - All condition types demonstration
- **rules_builtin.rs** - Built-in rules library
  - Segment-specific rules
  - Message-type rules
  - ADT and ORU message validation

## [0.16.0] - 2025-01-20

### Added - Phase 2 Conformance Profiles (Advanced Validation) 🔒

- **Component-Level Validation** - Granular validation at the component level:
  - **ComponentProfile** - Component-level constraints and rules
    - Position-based component identification (1-based)
    - Usage codes: R (Required), RE (Required if Known), O (Optional), X (Not Used), C (Conditional)
    - Data type specification
    - Maximum length constraints
    - HL7 table ID references
    - Optional name and description fields
  - Validation of individual components within composite fields
  - Component presence and absence checking
  - Component-level cardinality support

- **Conditional Usage with Predicates** - Dynamic field requirements based on message content:
  - **ConditionalUsage** enum - Extended usage codes with predicate support
    - All basic usage codes (R, RE, O, X)
    - New `Conditional(Predicate)` variant for dynamic requirements
    - Backward compatible with existing Usage enum
    - `from_usage()` - Convert from basic Usage
    - `as_usage()` - Convert to basic Usage (if possible)
    - `as_str()` - String representation ("R", "RE", "O", "X", "C")
    - `is_conditional()` - Check if usage is conditional
  - **Predicate** structure - Condition-based usage evaluation
    - Condition expression string (e.g., "PID-8 IS VALUED")
    - True usage - usage when condition evaluates to true
    - False usage - usage when condition evaluates to false
    - Optional description for documentation
  - **PredicateParser** - Parse condition expression strings
    - `parse(expression)` - Parse condition string into Condition enum
    - Support for field path notation (e.g., "PID-8", "OBX(1)-5")
    - IS VALUED / IS NOT VALUED checks
    - Equality operators: =, !=, <>
    - Numeric comparisons: >, <, >=, <=
    - Boolean logic: AND, OR, NOT
    - Proper operator precedence (NOT > AND > OR)
    - Quoted string literal support
    - Case-insensitive keywords
  - **Condition** enum - Evaluatable condition types
    - IsValued(path) - Check if field has non-empty value
    - IsNotValued(path) - Check if field is empty or missing
    - Equals(path, value) - String equality check
    - NotEquals(path, value) - String inequality check
    - GreaterThan(path, threshold) - Numeric comparison (>)
    - LessThan(path, threshold) - Numeric comparison (<)
    - GreaterThanOrEqual(path, threshold) - Numeric comparison (>=)
    - LessThanOrEqual(path, threshold) - Numeric comparison (<=)
    - And(left, right) - Logical AND of two conditions
    - Or(left, right) - Logical OR of two conditions
    - Not(condition) - Logical NOT of condition
  - **PredicateEvaluator** - Evaluate predicates against messages
    - `evaluate(predicate, message)` - Determine actual usage based on message content
    - Uses vyna-hl7-terser for field access
    - Returns appropriate Usage based on condition result
    - Integrated with ConformanceValidator for automatic evaluation

- **Value Set Bindings** - Terminology validation and code set constraints:
  - **ValueSetBinding** - Link fields to value sets
    - Value set ID (e.g., "HL70001" for Administrative Sex)
    - Binding strength specification
    - Builder pattern support
  - **BindingStrength** enum - Control terminology enforcement
    - Required - Must use value from set
    - Extensible - Should use value from set, but may extend
    - Preferred - Preferred to use value from set
    - Example - Example values only
    - `from_str()` - Parse from string ("REQUIRED", "R", etc.)
    - Case-insensitive parsing

- **Co-Constraints** - Cross-field validation rules:
  - **CoConstraint** structure - Message-level constraints
    - Unique constraint ID
    - Human-readable description
    - Condition expression (same syntax as predicates)
    - Fluent builder API
  - Message-level co-constraint collection
  - Cross-segment field dependency validation
  - Support for complex business rules

- **Enhanced Field Profiles**:
  - Updated `FieldProfile` to support Phase 2 features
  - Changed `usage` field from `Usage` to `ConditionalUsage` (breaking change)
  - Added `components: Option<Vec<ComponentProfile>>` - Component-level validation
  - Added `value_set: Option<ValueSetBinding>` - Terminology binding
  - `with_components()` - Add component profiles
  - `with_value_set()` - Add value set binding
  - `with_conditional_usage()` - Create with conditional usage
  - Backward compatibility through `from_usage()` conversion

- **Enhanced Message Profiles**:
  - Added `co_constraints: Option<Vec<CoConstraint>>` to MessageProfile
  - `with_co_constraints()` - Add cross-field constraints
  - Support for message-level validation rules

- **Validator Integration**:
  - ConformanceValidator updated to handle ConditionalUsage
  - Automatic predicate evaluation during validation
  - Recursive validation for conditional fields
  - Error messages indicate when conditional logic applies

- **Testing**:
  - 16 new tests for predicate parsing and evaluation
  - Tests for all condition types (IS VALUED, =, >, <, AND, OR, NOT)
  - Tests for complex boolean expressions
  - Tests for invalid expressions and error handling
  - Integration tests with ConformanceValidator
  - All existing tests passing (backward compatible)

- **Examples**:
  - `predicate_validation.rs` - Comprehensive Phase 2 feature demonstration
    - Conditional usage with predicates
    - Component-level validation
    - Value set bindings
    - Predicate parsing and evaluation
    - Multiple test cases showing validation scenarios

- **Error Handling**:
  - New `InvalidPredicate` error variant for predicate parsing errors
  - New `InvalidBindingStrength` error variant for invalid binding strengths
  - Detailed error messages for malformed conditions
  - Path validation for field references

### Technical Details

- **Dependencies**: Added vyna-hl7-terser for field access in predicate evaluation
- **Breaking Changes**:
  - `FieldProfile.usage` changed from `Usage` to `ConditionalUsage`
  - Existing code using `FieldProfile::new()` continues to work (automatic conversion)
  - Pattern matching on `FieldProfile.usage` must use `ConditionalUsage` variants
- **Performance**: Predicate evaluation uses cached Terser for efficient field access
- **LOC**: ~300 LOC for predicate engine, ~500 LOC total for Phase 2 structures
- **Test Coverage**: 16 new tests for predicates, all existing conformance tests passing

### Future Enhancements (deferred to later sprints)

- XML parser support for Phase 2 elements (components, predicates, value sets, co-constraints)
- Predicate expression functions (e.g., AGE(PID-7), LENGTH(field))
- Value set validation against actual code tables
- Co-constraint evaluation engine
- Data type flavors and profiles
- Enhanced error messages with predicate evaluation details

## [0.15.0] - 2025-01-20

### Added - Message Template System 📋

- **Template System** - Comprehensive framework for creating and validating HL7 messages from templates:
  - **MessageTemplate** - Reusable message patterns with segment and field definitions
    - Message type and trigger event specification
    - HL7 version string (e.g., "2.5", "2.5.1")
    - Required and optional segment definitions
    - Variable placeholder support using `{{variable}}` syntax
    - Description and metadata fields
    - Fluent builder API for programmatic construction
  - **SegmentTemplate** - Individual segment specifications
    - Required/optional segment markers
    - Repeating segment support
    - Field templates at specific positions
    - Description and documentation fields
  - **FieldTemplate** - Field-level definitions
    - Required/optional field markers
    - Data type specifications
    - Maximum length constraints
    - Variable placeholders for dynamic values
    - Default value support
    - Component-level templates for composite fields
  - **ComponentTemplate** - Component specifications for composite fields
    - Position within field
    - Required/optional markers
    - Placeholders and defaults
    - Description fields

- **Template Engine** - Create messages from templates with variable substitution:
  - `TemplateEngine::new()` - Create engine instance
  - `set_variable(key, value)` - Set variable for substitution
  - `set_variables(map)` - Set multiple variables at once
  - `create_message(template)` - Generate message from template
  - Automatic variable replacement in placeholders
  - Support for template-level default variables
  - Engine variables override template defaults
  - Proper handling of composite fields and components

- **Template Validation** - Validate messages against template definitions:
  - `TemplateValidator::validate()` - Validate message against template
  - **ValidationResult** - Validation outcome with errors and warnings
    - Valid/invalid status flag
    - List of validation errors with locations
    - List of validation warnings
    - Helper methods: `has_errors()`, `has_warnings()`
  - **ValidationError** - Error with message and field location
  - **ValidationWarning** - Warning with message and field location
  - Validates message type and trigger event
  - Checks required segments present
  - Validates required fields populated
  - Field length validation
  - Clear error messages with field locations (e.g., "PID-5")

- **Template Inheritance** - Extend base templates to create specialized variants:
  - **TemplateResolver** - Manages template hierarchy
    - `register(template)` - Add template to registry
    - `register_all(templates)` - Add multiple templates
    - `resolve(name)` - Resolve template with inheritance
  - Template extension via `extends` property
  - Multi-level inheritance support (grandparent → parent → child)
  - Child segments override base segments with same ID
  - Child variables override base variables
  - Segment field merging
  - Circular dependency detection
  - Clear error messages for missing base templates

- **YAML/JSON Configuration** - Load and save templates:
  - `MessageTemplate::from_yaml()` / `from_yaml_file()` - Load from YAML
  - `MessageTemplate::from_json()` / `from_json_file()` - Load from JSON
  - `MessageTemplate::to_yaml()` / `to_yaml_file()` - Save to YAML
  - `MessageTemplate::to_json()` / `to_json_pretty()` / `to_json_file()` - Save to JSON
  - Full serde support for all template types
  - Human-readable configuration format
  - Easy template sharing and version control

- **Standard Template Library** - Pre-built templates for common messages:
  - **TemplateLibrary::new()** - Create library with standard templates
  - `get(name)` - Retrieve template by name
  - `list_templates()` - List all available templates
  - `add_template(template)` - Add custom template to library
  - **7 Pre-built Templates**:
    - **ADT_A01** - Admit/Visit Notification (MSH, EVN, PID, PV1)
    - **ADT_A04** - Register a Patient (MSH, EVN, PID)
    - **ADT_A08** - Update Patient Information (MSH, EVN, PID)
    - **ORU_R01** - Unsolicited Observation Result (MSH, PID, OBR, OBX)
    - **ORM_O01** - General Order Message (MSH, PID, ORC, OBR)
    - **SIU_S12** - Notification of New Appointment Booking (MSH, SCH, PID)
    - **MDM_T02** - Original Document Notification (MSH, EVN, PID, TXA, OBX)

### Implementation Details

- **Error Handling**:
  - `Error::Parse` - Template parsing errors
  - `Error::Validation` - Template validation errors
  - `Error::Substitution` - Variable substitution errors (missing variables)
  - `Error::NotFound` - Template not found in registry
  - `Error::Inheritance` - Template inheritance errors (circular dependencies)
  - `Error::Yaml` / `Error::Json` - Serialization errors
  - Integration with vyna-hl7-core and vyna-hl7-parser error types

- **Integration with Existing Crates**:
  - Uses `vyna-hl7-core` message structures (Message, Segment, Field, Repetition, Component)
  - Compatible with `vyna-hl7-parser` for parsing messages
  - Works with all HL7 versions supported by vyna-hl7-core
  - Validation complements `vyna-hl7-validator` schema validation

### Testing

- **47 Unit Tests** covering:
  - Template creation and builders (4 tests)
  - Template engine and variable substitution (6 tests)
  - Template inheritance and resolution (8 tests)
  - Standard template library (5 tests)
  - YAML/JSON parsing and serialization (8 tests)
  - Template validation (7 tests)
  - Error handling (9 tests)
- **14 Documentation Tests** embedded in API documentation
- **3 Working Examples**:
  - `template_basic.rs` - Basic template usage (111 LOC)
  - `template_library.rs` - Standard library usage (114 LOC)
  - `template_inheritance.rs` - Template inheritance (139 LOC)

### Dependencies

- Uses `serde`, `serde_json`, `serde_yaml` for configuration
- Uses `regex` for placeholder matching
- Depends on `vyna-hl7-core` and `vyna-hl7-parser`

## [0.14.0] - 2025-11-20

### Added - Message Transformation Framework 🔄

- **Message Transformation Framework** - Comprehensive framework for transforming HL7 v2.x messages:
  - **TransformationRule** - Type-safe transformation rule definition
    - Source and target field paths using terser notation (e.g., "PID-5-1", "OBX(1)-5")
    - Optional transformation functions with flexible signature
    - Default value support for empty source fields
    - Skip-if-empty configuration
    - Automatic path validation ensuring proper terser format
  - **TransformFn** - Function type for custom transformations
    - Accepts value and context parameters
    - Returns transformed value or error
    - Supports both built-in and user-defined functions
  - **TransformContext** - Contextual data for transformations
    - Source message access
    - Key-value data storage for parameterized transforms
    - Fluent builder API for context construction

- **MessageTransformer** - Fluent API for message transformation:
  - `add_mapping()` - Simple field-to-field copy
  - `add_transform()` - Field mapping with transformation function
  - `add_rule()` - Add pre-configured transformation rule
  - `transform()` - Create transformed copy of message
  - `transform_in_place()` - Transform message in-place for efficiency
  - Context data management via `set_context_data()`
  - Rule validation with `validate_rules()`
  - Rule count and clearing methods

- **Built-in Transformation Functions** (15 functions):
  - **String Case**: `uppercase`, `lowercase`
  - **Whitespace**: `trim`, `trim_start`, `trim_end`, `remove_whitespace`
  - **Substring**: `substring` (with start/length params)
  - **Date/Time**: `format_date`, `format_datetime` (YYYYMMDD → various formats)
  - **String Replacement**: `replace`, `regex_replace`
  - **Concatenation**: `prefix`, `suffix`
  - **Padding**: `pad` (left/right with specified character)
  - **Defaults**: `default_if_empty`

- **Declarative Configuration Support** (optional `serde` feature):
  - **TransformConfig** - YAML/JSON configuration structure
    - List of transformation rules
    - Global context data
    - Builder pattern for programmatic construction
  - **RuleConfig** - Individual rule configuration
    - Source and target field paths
    - Transform function name (string-based lookup)
    - Default values and skip-if-empty settings
    - Per-rule parameters as key-value pairs
  - **YAML/JSON Loading**:
    - `from_yaml()` / `from_yaml_file()` - Load from YAML
    - `from_json()` / `from_json_file()` - Load from JSON
    - `to_yaml()` / `to_yaml_file()` - Save to YAML
    - `to_json()` / `to_json_file()` - Save to JSON
  - **Transform Function Registry**:
    - String-based function lookup ("uppercase", "format_date", etc.)
    - Automatic validation of function names
    - Clear error messages for unknown functions

### Implementation Details

- **Error Handling**:
  - `Error::FieldAccess` - Terser path access errors
  - `Error::TransformFn` - Transformation function errors
  - `Error::InvalidRule` - Rule validation errors
  - `Error::Config` - Configuration loading errors (serde feature)
  - `Error::Yaml` / `Error::Json` - Serialization errors (serde feature)
  - Integration with vyna-hl7-core and vyna-hl7-terser error types

- **Integration with Existing Crates**:
  - Uses `vyna-hl7-terser::Terser` for reading field values
  - Uses `vyna-hl7-terser::TerserMut` for setting field values
  - Compatible with all message types supported by vyna-hl7-core
  - Works with both builder-created and parser-generated messages

### Testing

- **49 Unit Tests** covering:
  - Transformation rule creation and validation (8 tests)
  - Context management (3 tests)
  - Built-in transformation functions (14 tests)
  - MessageTransformer operations (11 tests)
  - Configuration loading and serialization (13 tests)
- **19 Documentation Tests** embedded in API documentation
- **3 Working Examples**:
  - `transform_basic.rs` - Basic field mappings and transformations (51 LOC)
  - `transform_config.rs` - YAML-based configuration (79 LOC)
  - `transform_advanced.rs` - Advanced features and custom functions (97 LOC)

### Dependencies

- New workspace dependencies:
  - `regex = "1.11"` - Regular expression support for transforms
  - `serde_yaml = "0.9"` - YAML configuration support (optional)
- Existing dependencies leveraged:
  - `vyna-hl7-core` - Message structures
  - `vyna-hl7-terser` - Field access API
  - `chrono` - Date/time transformation
  - `serde` / `serde_json` - JSON configuration (optional)

### Documentation

- Comprehensive module-level documentation in `vyna-hl7-transform/src/lib.rs`
- Detailed API documentation for all public types and methods
- Example-driven documentation for common use cases
- README section explaining transformation framework
- ROADMAP updated to reflect Phase 2, Sprint 2 completion

### Code Statistics

- **Module**: `vyna-hl7-transform` (new crate)
- **Source Lines**: ~900 LOC across 5 modules
  - `error.rs` - 68 LOC
  - `rule.rs` - 220 LOC (including 18 tests)
  - `transforms.rs` - 410 LOC (including 14 tests)
  - `transformer.rs` - 380 LOC (including 11 tests)
  - `config.rs` - 290 LOC (including 13 tests, serde feature)
- **Test Coverage**: 62 total tests (49 unit + 13 config + 19 doc)
- **Examples**: 227 LOC across 3 examples

## [0.13.0] - 2025-11-20

### Added - Batch/File Message Support 📦

- **Batch Message Support** - Complete implementation for high-volume message processing:
  - **BatchHeader (BHS)** - Batch Header Segment with full HL7 v2.x compliance
    - Sender/receiver application and facility fields (BHS-3 through BHS-6)
    - Creation datetime with automatic timestamp generation (BHS-7)
    - Batch identification fields: name/ID/type, control ID, reference control ID (BHS-9, BHS-11, BHS-12)
    - Security and comment fields (BHS-8, BHS-10)
    - Network address fields for v2.6+ (BHS-13, BHS-14)
  - **BatchTrailer (BTS)** - Batch Trailer Segment with validation
    - Automatic message count tracking (BTS-1)
    - Optional comment field (BTS-2)
    - Repeating batch totals field support (BTS-3)
  - **Batch Structure** - Container for batched messages
    - Automatic message count validation against BTS-1
    - Support for multiple HL7 messages within batch envelope
    - Encoding with configurable separators (\r for transmission, \n for display)

- **File Message Support** - Multi-batch file transmission capability:
  - **FileHeader (FHS)** - File Header Segment
    - File-level sender/receiver information (FHS-3 through FHS-6)
    - File identification: name/ID, control ID, reference control ID (FHS-9, FHS-11, FHS-12)
    - Creation datetime, security, and comment fields (FHS-7, FHS-8, FHS-10)
    - Network address support for v2.6+ (FHS-13, FHS-14)
  - **FileTrailer (FTS)** - File Trailer Segment
    - Automatic batch count tracking (FTS-1)
    - Optional comment field (FTS-2)
  - **File Structure** - Container for multiple batches
    - Hierarchical organization: File → Batches → Messages
    - Automatic batch count validation against FTS-1
    - Total message count calculation across all batches

- **Fluent Builder APIs** - Ergonomic batch/file construction:
  - **BatchBuilder** - Chainable batch message creation
    - Sender/receiver configuration methods
    - Automatic datetime defaults (uses current time if not set)
    - Automatic message count management in trailer
    - Pre-build validation to catch errors early
    - `add_message()` and `add_messages()` for message insertion
    - `trailer_comment()` and `add_total()` for metadata
  - **FileBuilder** - Chainable file message creation
    - File-level configuration methods
    - Automatic datetime and batch count management
    - Pre-build validation for file structure
    - `add_batch()` and `add_batches()` for batch insertion
    - Support for file-level and batch-level comments

- **Parser Extensions** - Full batch/file parsing support:
  - **parse_batch()** - Parse complete batch messages (BHS...messages...BTS)
    - Automatic delimiter extraction from BHS segment
    - Multi-message parsing with MSH detection
    - Integrated validation during parsing
  - **parse_file()** - Parse complete file messages (FHS...batches...FTS)
    - Hierarchical parsing: FHS → BHS → MSH → BTS → FTS
    - Automatic delimiter extraction from FHS segment
    - Batch boundary detection and nested parsing
    - Integrated validation for batch and file counts
  - **Delimiter Extraction** - Specialized extractors for batch/file headers
    - `extract_delimiters_from_bhs()` - BHS delimiter parsing
    - `extract_delimiters_from_fhs()` - FHS delimiter parsing
  - **Datetime Parsing** - HL7 timestamp format support
    - Support for TS format (v2.3-v2.5): YYYYMMDDHHMMSS
    - Support for DTM format (v2.6+): Partial datetime precision
    - Timezone offset handling
  - **Special Segment Parsing** - FHS/BHS/FTS/BTS segment parsers
    - MSH-like structure handling for FHS/BHS (field 1 = separator, field 2 = encoding chars)
    - Proper field indexing for trailer segments

- **Segment Encoding Enhancements**:
  - Updated `Segment::encode()` to handle FHS/BHS special structure
    - Proper encoding for segments with field separator and encoding characters
    - Consistent handling with MSH segment encoding
    - Prevents delimiter escaping in FHS/BHS field 1 and 2

- **Validation Features**:
  - **Message Count Validation** - BTS-1 must match actual message count in batch
  - **Batch Count Validation** - FTS-1 must match actual batch count in file
  - **Automatic Count Management** - Builders set counts automatically to prevent mismatches
  - **Pre-build Validation** - Catch structural errors before encoding
  - **Parse-time Validation** - Validate counts during parsing for early error detection

- **Examples**:
  - `batch_messages.rs` - Comprehensive batch message demonstration (183 LOC)
    - Example 1: Creating batches with BatchBuilder
    - Example 2: Parsing batch messages from text
    - Example 3: Batch validation (valid and invalid cases)
    - Example 4: Encoding batches for transmission and round-trip verification
  - `file_messages.rs` - Complete file message workflows (268 LOC)
    - Example 1: Building multi-batch files with FileBuilder
    - Example 2: Parsing file messages from text
    - Example 3: File validation with count mismatches
    - Example 4: Hierarchical navigation through file → batch → message structure

- **Testing**:
  - **vyna-hl7-core**: 18 tests total (6 for batch module, 6 for batch builders, 6 for file builders) ✅
  - **vyna-hl7-parser**: 24 tests total (13 new tests for batch/file parsing) ✅
  - Comprehensive coverage:
    - Batch/file structure creation and validation
    - Builder auto-count management
    - Segment conversion (headers and trailers to segments)
    - Encoding with different separators
    - Parsing with validation
    - Round-trip encoding and parsing
    - Error cases (count mismatches, missing segments)
    - Datetime field parsing (full timestamps, date-only, timezone handling)
    - Delimiter extraction from FHS/BHS segments

### Technical Details

- **Batch/File Architecture**:
  - Hierarchical structure: `File` contains `Vec<Batch>`, `Batch` contains `Vec<Message>`
  - Datetime fields use `chrono::NaiveDateTime` for timezone-independent storage
  - Encoding uses configurable separators (default: `\r` for HL7 standard)
  - Automatic field management prevents manual count errors

- **Parser Enhancements**:
  - New parsing functions: `parse_batch()`, `parse_file()`
  - Segment-specific parsers: `parse_bhs_segment()`, `parse_fhs_segment()`, `parse_bts_segment()`, `parse_fts_segment()`
  - Helper function `parse_segment_like_msh()` for FHS/BHS parsing
  - Datetime parser `parse_datetime_field()` supports TS and DTM formats

- **Version Compatibility**:
  - FHS/BHS fields 1-12 supported across all HL7 versions (v2.3 - v2.7.1)
  - FHS-13/BHS-13 (Sending Network Address) - v2.6+
  - FHS-14/BHS-14 (Receiving Network Address) - v2.6+
  - Field types evolved from ST→HD (v2.5), TS→DTM (v2.6)

## [0.12.0] - 2025-11-20

### Added - Query/Response Support 🔍

- **QBP (Query by Parameter) Message Builders** - Create structured HL7 v2.5+ queries:
  - **QbpQ11Builder** - Immunization history queries (Z44 CDC profile support)
    - Patient demographics with mother's maiden name
    - Address and contact information
    - Quantity limits and response control
    - Default Z44 (Request Evaluated History and Forecast) query name
  - **QbpQ15Builder** - Display-oriented queries
    - Flexible parameter support for custom query types
    - Response control with priority and modality settings
  - **QbpQ21Builder** - Demographic queries
    - Patient-centric search parameters
    - Support for various query formats
  - **QbpQ22Builder** - Find candidates (patient search)
    - Query-by-example parameter format (@PID.5.1^SMITH)
    - Multiple search criteria support
    - Pagination control via quantity limits

- **RSP (Response) Message Builders** - Generate query responses:
  - **RspK11Builder** - Immunization history responses
    - MSA (Message Acknowledgment) segment
    - QAK (Query Acknowledgment) with hit counts
    - QPD (Query Parameter Definition) echo
    - Support for adding data segments (PID, ORC, RXA, etc.)
  - **RspK21Builder** - Demographic query responses
    - Patient demographics in response payload
    - Multiple patient records support
  - **RspK22Builder** - Find candidates responses
    - Pagination support (total hits, current payload, remaining)
    - Multiple matching patient records
    - Continuation pointer support via DSC segment

- **Segment Builders** - Low-level query segment construction:
  - **QpdBuilder** - Query Parameter Definition segments
    - Message query name (QPD-1)
    - Query tag for request/response matching (QPD-2)
    - Variable parameter support (QPD-3+)
  - **RcpBuilder** - Response Control Parameter segments
    - Query priority (I=Immediate, D=Deferred)
    - Quantity limits with units (e.g., "100^RD")
    - Response modality control
    - Execution and modification timestamps
  - **QakBuilder** - Query Acknowledgment segments
    - Query tag matching (QAK-1)
    - Response status codes from HL7 Table 0208 (QAK-2)
    - Hit count tracking: total, current payload, remaining (QAK-4/5/6)
    - Pagination support

- **QueryResultParser** - Extract and parse RSP message results:
  - **QueryResponseStatus** enum - Type-safe status code handling
    - OK - Data found, no errors
    - NF - No data found, no errors
    - AE - Application error
    - AR - Application reject
    - TM - Too much data found
    - PD - Protected data
    - Unknown - Graceful handling of non-standard codes
  - **QueryAcknowledgment** struct - Parsed QAK segment data
    - Query tag for request/response correlation
    - Response status with success/error helpers
    - Hit count analysis (total, in response, remaining)
    - Pagination helpers (has_more_data, is_complete)
  - **QueryResultParser** API methods:
    - `parse_acknowledgment()` - Extract QAK segment data
    - `get_acknowledgment_code()` - MSA-1 acknowledgment code
    - `get_message_control_id()` - MSA-2 control ID
    - `get_data_segments()` - All data segments (PID, OBX, etc.)
    - `get_continuation_pointer()` - DSC-1 for pagination
    - `is_successful()` - Combined MSA/QAK success check
    - `get_error_text()` - MSA-3 error messages

- **Examples**:
  - `query_response.rs` - Comprehensive QBP/RSP demonstration
    - Example 1: QBP^Q11 immunization query with Z44 profile
    - Example 2: RSP^K11 immunization response with PID, ORC, RXA segments
    - Example 3: QBP^Q22 patient search with query-by-example parameters
    - Example 4: RSP^K22 paginated response (247 total, 2 in payload, 245 remaining)

- **Testing**:
  - 71 tests in vyna-hl7-core all passing ✅
  - 69 tests in vyna-hl7-terser all passing (including QueryResultParser) ✅
  - Comprehensive coverage of query/response workflows
  - Status code parsing and validation
  - Pagination and hit count tracking
  - Data segment extraction

### Technical Details

- **Query/Response Protocol**:
  - Replaces legacy QRD-based queries with modern QPD parameter approach
  - Structured query parameters with @ notation for field references
  - Query tag correlation between requests and responses
  - Response control for pagination, priority, and delivery options

- **HL7 Segments**:
  - QPD (Query Parameter Definition) - Query parameters
  - RCP (Response Control Parameter) - Response delivery control
  - QAK (Query Acknowledgment) - Query-specific acknowledgment
  - MSA (Message Acknowledgment) - Standard message acknowledgment
  - DSC (Continuation Pointer) - Pagination support

- **HL7 Tables**:
  - Table 0208 - Query Response Status (OK, NF, AE, AR, TM, PD)
  - Table 0091 - Query Priority (I, D)
  - Table 0394 - Response Modality (R, T, B)

- **Message Types**:
  - QBP^Q11/RSP^K11 - Immunization history query/response
  - QBP^Q15/RSP^K15 - Display-oriented query/response
  - QBP^Q21/RSP^K21 - Demographic query/response
  - QBP^Q22/RSP^K22 - Find candidates query/response

- **CDC Immunization Profiles**:
  - Z44 - Request Evaluated History and Forecast
  - Z34 - Request Immunization History

### Phase 1, Sprint 3 Complete

This release completes the third sprint of VynaHL7's enhanced feature roadmap:
- ✅ Enhanced Terser Capabilities (Sprint 1 - v0.10.0)
- ✅ FHIR Converters Expansion (Sprint 2 - v0.11.0)
- ✅ Query/Response Support (Sprint 3 - v0.12.0)
- Next: Enhanced Validation (Sprint 4)

## [0.11.0] - 2025-11-20

### Added - FHIR Converters Expansion 🏥

- **Three New FHIR R4 Converters** - vyna-hl7-fhir now includes 12 production-ready converters:
  - **ImmunizationConverter** (RXA → Immunization)
    - Complete vaccine administration records with CVX coding system
    - Lot numbers, expiration dates, manufacturer references
    - Dose quantity, administration site, and route mapping
    - Performer (administering provider) and location tracking
    - Status conversions (CP→completed, RE→not-done)
    - Primary source and report origin tracking
  - **ServiceRequestConverter** (ORC/OBR → ServiceRequest)
    - Laboratory and diagnostic service orders
    - Placer and filler order identifiers with requisition grouping
    - Priority mappings (S→stat, A→asap, R→routine)
    - Status conversions (A→active, CA→revoked, CM→completed)
    - Ordering provider references and clinical notes
    - Specimen and reason code linking
  - **SpecimenConverter** (SPM → Specimen)
    - Complete specimen collection details
    - Collection method, body site, and quantity tracking
    - Received time and availability status
    - Container information and accession identifiers
    - Specimen condition tracking
    - Links to patient and associated service requests

- **FHIR Resource Structures**:
  - `Immunization` - Comprehensive vaccine administration resource
  - `ImmunizationPerformer` - Who performed the immunization
  - `ImmunizationProtocolApplied` - Vaccine protocol details
  - `ServiceRequest` - Diagnostic and therapeutic service requests
  - `Specimen` - Specimen/sample resource
  - `SpecimenCollection` - Collection details and provenance
  - `SpecimenContainer` - Container and handling information
  - `Period`, `Annotation` - Additional FHIR data types

- **Examples**:
  - `convert_vxu.rs` - VXU^V04 immunization message conversion
    - Demonstrates vaccine record updates with multiple immunizations
    - Shows lot tracking, expiration management, and performer details
  - `convert_orm.rs` - ORM^O01 laboratory order conversion
    - Multiple order handling with requisition grouping
    - Priority-based filtering and order status tracking
  - `convert_oml.rs` - OML^O21 specimen collection conversion
    - Multiple specimen types (blood, serum, etc.)
    - Collection details, container information, and condition tracking
    - Specimen availability and accession management

- **Testing**:
  - 17 new tests (5 immunization + 6 service request + 6 specimen) all passing
  - Total vyna-hl7-fhir test suite: 33 tests all passing ✅
  - Comprehensive coverage of field mappings and edge cases
  - Multiple segment handling validated

- **Documentation**:
  - Updated vyna-hl7-fhir README with new converters
  - Detailed field mapping documentation for each converter
  - Working examples with realistic HL7 messages
  - Status and priority conversion tables

### Technical Details

- **Message Type Support**:
  - VXU^V04 - Unsolicited vaccination record update
  - ORM^O01 - General laboratory order
  - OML^O21 - Laboratory order for multiple orders related to a single specimen

- **HL7 Segment Mapping**:
  - RXA (Pharmacy/Treatment Administration) → Immunization
  - ORC (Common Order) → ServiceRequest
  - OBR (Observation Request) → ServiceRequest (supplemental data)
  - SPM (Specimen) → Specimen
  - TQ1 (Timing/Quantity) → ServiceRequest priority

- **Code System Mappings**:
  - CVX (CDC Vaccine Codes) for immunization vaccine codes
  - HL7 Table 0227 (Manufacturer) for vaccine manufacturers
  - HL7 Table 0292 (Vaccines Administered) for immunization status
  - HL7 Table 0487 (Specimen Type) for specimen classification
  - HL7 Table 0488 (Specimen Collection Method)
  - HL7 Table 0163 (Body Site) for specimen collection sites
  - HL7 Table 0340 (Reason for Study) for service request reasons

### Phase 1, Sprint 2 Complete

This release completes the second sprint of VynaHL7's enhanced feature roadmap:
- ✅ Enhanced Terser Capabilities (Sprint 1 - v0.10.0)
- ✅ FHIR Converters Expansion (Sprint 2 - v0.11.0)
- Next: Custom Z-Segment Framework (Sprint 3)

## [0.10.0] - 2025-11-20

### Added - Enhanced Terser Capabilities 🚀

- **BulkTerser API** - Efficient bulk field extraction and pattern matching:
  - `get_multiple()` - Extract multiple fields at once with HashMap results
  - `get_pattern()` - Pattern matching with wildcard support for repeating segments
  - `get_all_from_segments()` - Convenience method for common extraction patterns
  - Wildcard syntax: `OBX(*)-5` extracts field 5 from all OBX segments
  - Support for both segment wildcards and field repetition wildcards

- **Iterator API** - Standard Rust iterators for field traversal:
  - `FieldIterator` - Iterate over field values from repeating segments
  - `ComponentIterator` - Iterate over specific components across segments
  - `RepetitionIterator` - Iterate over field repetitions
  - Extension methods on `Terser`: `iter_field()`, `iter_component()`, `iter_repetitions()`
  - Automatic empty value filtering for cleaner iteration
  - Full support for standard iterator methods: `filter()`, `map()`, `count()`, `collect()`

- **TerserQuery API** - Conditional queries and filtering:
  - `find_segments()` - Find segments matching arbitrary predicates
  - `filter_repeating()` - Filter segments by field value
  - `find_first()` - Find first segment matching criteria
  - `filter_by_component()` - Filter segments by component value
  - `get_values_where()` - Extract values based on conditional filtering
  - `count_where()` - Count segments matching predicate
  - `any_match()` / `all_match()` - Boolean segment queries
  - `get_if()` - Conditional field extraction based on message state

- **Examples**:
  - `enhanced_terser.rs` - Comprehensive demonstration of all new features
    - 7 complete working examples showing real-world use cases
    - Bulk extraction, pattern matching, iteration, and conditional queries
    - ORU message with multiple OBX segments for realistic scenarios

- **Documentation**:
  - Complete README for vyna-hl7-terser crate
  - API documentation for all public types
  - Performance comparison table
  - Path indexing reference
  - Best practices guide

- **Testing**:
  - 27 new tests (9 bulk + 9 iterator + 9 query) all passing
  - Complete test coverage for pattern matching edge cases
  - Iterator behavior validation
  - Complex query scenario testing
  - All workspace tests passing (362+ total tests)

### Technical Details

- **Zero-Allocation Iteration**: Iterators use borrowed references with no heap allocations
- **Pattern Matching**: Smart pattern detection distinguishes segment wildcards from field repetition patterns
- **Lifetime Safety**: TerserQuery stores internal Terser to avoid lifetime issues
- **1-Based HL7 Indexing**: Maintains HL7 standard conventions for component and segment indexing
- **Performance**: Iterators and bulk operations optimized for high-throughput processing

### Performance Considerations

| API | Use Case | Characteristics |
|-----|----------|-----------------|
| `Terser` | Simple field access | ~200-300 ns/lookup |
| `CachedTerser` | Repeated access | ~20-40 ns/lookup (5-10x faster) |
| `BulkTerser` | Multiple fields | Efficient bulk operations |
| Iterators | Repeating segments | Zero-allocation iteration |
| `TerserQuery` | Complex filtering | Optimized predicates |

### Phase 1, Sprint 1 Complete

This release completes the first sprint of VynaHL7's enhanced feature roadmap:
- ✅ Enhanced Terser Capabilities (Week 1)
- Next: FHIR Converters expansion (Week 2)

## [0.9.0] - 2025-11-19

### Added - Conformance Profile Validation Framework 🎯

- **vyna-hl7-conformance Crate** - Comprehensive HL7 v2 conformance profile validation system achieving HAPI feature parity:
  - XML conformance profile parser supporting HL7v2xConformanceProfile format
  - Programmatic conformance profile creation API
  - Message validation against conformance profiles
  - Detailed validation reporting with precise error locations
  - Support for HL7 v2.3 through v2.7.1

- **Core Components**:
  - `ProfileParser` - Load and parse XML conformance profiles
  - `ConformanceValidator` - Validate messages against profiles
  - `ConformanceProfile` / `MessageProfile` / `SegmentProfile` / `FieldProfile` - Profile data structures
  - `ConformanceValidationResult` - Detailed validation results with errors, warnings, and info messages
  - `ProfileMetadata` - Version, organization, and conformance profile metadata

- **Validation Features**:
  - **Usage Validation**: Enforce R (Required), RE (Required if Known), O (Optional), X (Not Used)
  - **Cardinality Validation**: Check min/max occurrence constraints with unbounded support
  - **Length Validation**: Enforce maximum field lengths
  - **Location Tracking**: Precise error locations at segment, field, and component levels
  - **Severity Classification**: Error, Warning, and Info message types

- **Error Types** (via `ConformanceErrorType` enum):
  - `RequiredElementMissing` - Required elements (R) not present
  - `NotUsedElementPresent` - Explicitly forbidden elements (X) found
  - `CardinalityViolation` - Min/max occurrence constraints violated
  - `LengthExceeded` - Field exceeds maximum allowed length
  - `InvalidFormat` - Data format validation failures

- **XML Parsing**:
  - Event-based parsing using `quick-xml` for efficiency
  - Support for self-closing and nested XML elements
  - ItemNo field position parsing with leading zero handling (e.g., "00010" → 10)
  - Unbounded cardinality support with "*" notation
  - Robust error handling for malformed XML

- **Examples** (2 comprehensive examples):
  - `basic_validation.rs` - End-to-end validation workflow with programmatic profile creation
  - `xml_parser_demo.rs` - XML profile parsing and validation demonstration

- **Sample Profiles**:
  - `sample_adt_a01.xml` - Example ADT^A01 conformance profile

- **Testing**:
  - 21 tests (14 unit + 6 integration + 1 doctest) all passing
  - Complete test coverage for XML parsing edge cases
  - Integration tests for end-to-end validation scenarios
  - All workspace tests passing (335+ total tests)

- **Documentation**:
  - Complete API documentation for all public types
  - Inline code examples in documentation
  - Phase 1 MVP scope clearly defined
  - Future enhancement roadmap

### Technical Details

- **Event-Based XML Parsing**: Efficient streaming XML parser using quick-xml
- **Profile Structure**: Hierarchical profile structure matching HL7 v2 message hierarchy
- **Validation Location Tracking**: Precise error locations with segment name, field number, and component position
- **Severity-Based Classification**: Errors, warnings, and informational messages for flexible reporting
- **Extensible Design**: Foundation for future enhancements (conditional predicates, value sets, co-constraints)

### Phase 1 MVP Scope

Initial release focuses on core validation features:
- Basic XML profile parsing (segments and fields)
- Usage code validation (R, RE, O, X)
- Min/Max cardinality validation
- Field length constraints

### Future Enhancements

Planned for future releases:
- Conditional predicates (C usage codes with predicates)
- Component-level validation
- Data type flavors
- Value set bindings
- Co-constraints between fields
- IZ (Initialize) and W (Withdrawn) usage codes

## [0.8.0] - 2025-11-19

### Added - Custom Z-Segment Framework 🔧

- **vyna-hl7-custom Crate** - Type-safe framework for custom organization-specific Z-segments:
  - Declarative `z_segment!` macro for defining custom segments
  - Compile-time type checking for segment fields
  - Zero overhead for standard HL7 segments
  - Support for all HL7 field patterns and data types

- **Field Types** - Comprehensive support for all HL7 field patterns:
  - **Primitive Types**: `String`, `u32`, `i32`, `i64`, `f64`, `bool`
  - **DateTime Types**: `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Utc>` (via chrono)
  - **Optional Fields**: `Option<T>` for any supported type
  - **Repeating Fields**: `Vec<T>` for repeating simple values (HL7 `~` separator)
  - **Component Fields**: `(String, String, ...)` tuples (2-5 components, HL7 `^` separator)
  - **Optional Components**: `Option<Tuple>` for optional composite fields
  - **Repeating Components**: `Vec<Tuple>` for multiple structured values (HL7 `~` and `^` separators)

- **Core Features**:
  - `CustomSegment` trait for segment definitions
  - `CustomSegmentRegistry` for dynamic segment registration
  - `MessageExt` extension trait for seamless Message integration
  - Fluent builder API for ergonomic segment creation
  - Custom validation hooks for business rules
  - Automatic HL7 encoding/decoding with proper separator handling

- **Message Operations**:
  - `get_custom_segment<T>()` - Extract first occurrence by type
  - `get_custom_segments<T>()` - Extract all occurrences
  - `has_custom_segment<T>()` - Check existence
  - `set_custom_segment<T>()` - Replace or add segment
  - `add_custom_segment<T>()` - Append segment
  - `remove_custom_segments<T>()` - Remove all of type

- **Builder Pattern**:
  - Type-safe field setters with automatic type conversion
  - Required field validation at build time
  - Optional field support (omit builder call for None)
  - Validation runs automatically on build
  - Compile-time verification of field types

- **Examples** (9 comprehensive examples):
  - `zpv_visit_segment.rs` - Basic Z-segment usage
  - `zcu_customer_segment.rs` - Validation and error handling
  - `message_manipulation.rs` - Comprehensive message operations
  - `field_types.rs` - All supported field types demonstration
  - `component_fields.rs` - Component fields (tuples)
  - `datetime_fields.rs` - DateTime field types (NaiveDate, NaiveTime, NaiveDateTime)
  - `repeating_fields.rs` - Repeating fields (Vec<T>)
  - `repeating_components.rs` - Repeating component fields (Vec<Tuple>)
  - `real_world_adt.rs` - Complete ADT^A01 patient admission example using all field types

- **Testing**:
  - 146 total tests (all passing)
  - 43 unit tests for field type implementations
  - 25 unit tests for core functionality
  - 18 integration tests for end-to-end workflows
  - Comprehensive coverage of all field patterns
  - Zero warnings

- **Documentation**:
  - Comprehensive README with quick start guide and field type reference
  - API documentation for all public types and traits
  - Best practices and performance notes
  - Updated main README with Z-segment section
  - Field type examples for all supported patterns
  - Real-world healthcare scenario demonstrations

### Technical Details

- **Trait-Based Architecture**: Uses specialized traits for type-safe field handling:
  - `ParseSegmentField` - Parse from HL7 segment fields
  - `SerializeSegmentField` - Encode to HL7 format
  - `BuildableField` - Builder pattern support
  - `BuilderFieldType` - Type information for builders
- **Macro Implementation**: Declarative macro generates struct, CustomSegment impl, and builder struct
- **Validation Integration**: Custom validation functions run on parsing and building
- **Zero Runtime Cost**: All type checking happens at compile time
- **HL7 Compliance**: Proper handling of all HL7 v2.x field separators (|, ^, ~, \, &)
- **DateTime Support**: Integration with chrono for robust date/time handling
- **Tuple Support**: Compile-time validation of component counts (2-5 components)

## [0.7.0] - 2025-11-19

### Changed - HL7 Standards Compliance 🔧

**BREAKING CHANGE**: Terser segment indexing changed from 0-based to 1-based to match HAPI and HL7 conventions.

- **Terser Segment Indexing (BREAKING)**:
  - Changed from 0-based to 1-based indexing for segment access
  - `OBX(1)` now refers to the **first** OBX segment (previously second)
  - `OBX(2)` now refers to the **second** OBX segment (previously third)
  - `OBX` without index still refers to the first OBX segment
  - Attempting to use index 0 (e.g., `OBX(0)`) now returns an error
  - This matches HAPI's convention and aligns with HL7's 1-based field numbering

- **Migration Guide**:
  - Review all code using segment indexing in Terser paths
  - Update indexed segment references: `OBX(N)` → `OBX(N+1)` for N > 0
  - First segment: `OBX(0)` → `OBX` or `OBX(1)`
  - Second segment: `OBX(1)` → `OBX(2)`
  - Third segment: `OBX(2)` → `OBX(3)`, etc.

### Fixed

- **ACK Message Generation**:
  - ACK messages now correctly include trigger event in MSH-9
  - Format changed from `"ACK"` to `"ACK^{trigger}"` (e.g., `"ACK^A01"` for ADT^A01)
  - Complies with HL7 v2.x acknowledgment message standards

### Documentation

- Updated all examples to use 1-based segment indexing
- Updated README.md, CLAUDE.md, and crate READMEs with correct indexing
- Clarified that component and subcomponent indexing remains 1-based (unchanged)
- Added migration notes for upgrading from 0.6.x

## [0.6.0] - 2025-10-08

### Added - CLI Tool for Message Analysis 🖥️

- **vyna-hl7-cli Crate** - Comprehensive command-line interface for HL7 message processing:
  - Professional CLI tool built with `clap` for intuitive command-line interaction
  - Colored terminal output for enhanced readability
  - Multiple output formats (text, JSON, pretty)
  - Support for stdin and file input

- **Commands**:
  - `parse` - Parse and display HL7 message structure
    - Text, JSON, and pretty-formatted output modes
    - Optional detailed segment view
    - Quick structure overview
  - `validate` - Validate messages against HL7 standards
    - Schema-based validation
    - Data type validation
    - Vocabulary/code set validation
    - Customizable HL7 version selection
    - Detailed error and warning reports
  - `extract` - Extract field values using Terser paths
    - Support for segment indexing (e.g., `OBX(0)-5`)
    - Component and subcomponent access
    - Multiple field extraction in single command
    - JSON output for programmatic processing
  - `convert` - Convert messages to different formats
    - JSON conversion (compact and pretty-printed)
    - FHIR R4 conversion (with `--features fhir`)
    - Machine-readable output for integration
  - `info` - Display comprehensive message information
    - Header details (version, type, control ID, applications)
    - Message structure analysis
    - Segment breakdown and counts
    - Size statistics

- **Features**:
  - Default: Core parsing, validation, and extraction
  - `fhir`: Optional FHIR R4 conversion support
  - Performance optimized for batch processing
  - Error handling with appropriate exit codes
  - Integration-friendly JSON output

- **Documentation**:
  - Comprehensive README with usage examples
  - Sample HL7 messages (ADT^A01, ORU^R01)
  - Examples for common workflows
  - Integration examples with `jq` and other tools
  - Building and installation instructions

### Performance

- **CLI startup**: < 50 ms for typical operations
- **Parse + display**: < 100 µs for standard messages
- **Batch processing**: Suitable for high-throughput scenarios
- **Memory footprint**: Minimal, suitable for embedded systems

## [0.5.0] - 2025-10-08

### Added - WebAssembly Support 🌐

- **vyna-hl7-wasm Crate** - Complete WebAssembly bindings for JavaScript/TypeScript:
  - Parse and manipulate HL7 messages in the browser and Node.js
  - Full TypeScript type definitions included
  - Zero-copy parsing compiled to WebAssembly for maximum performance

- **JavaScript API**:
  - `parseMessage()` - Parse HL7 messages from strings
  - `getTerserValue()` / `setTerserValue()` - Field access using Terser paths
  - `validateMessage()` - Message validation against HL7 standards
  - `createMessage()` - Create new messages programmatically
  - `extractPatientDemographics()` - Extract common patient fields from ADT messages
  - `extractObservations()` - Extract observations from ORU messages

- **TypeScript Support**:
  - Complete type definitions (`vyna-hl7.d.ts`)
  - Full IntelliSense support in VS Code and other editors
  - Type-safe message manipulation

- **Multi-Platform Build Targets**:
  - Web browsers (ES modules)
  - Node.js
  - Bundlers (webpack, rollup, vite)

- **Documentation & Examples**:
  - Comprehensive README with usage examples
  - Interactive browser example (`examples/browser.html`)
  - NPM package configuration with build scripts
  - TypeScript usage examples

### Performance

- **WASM bundle size**: ~200-300 KB (minified + gzip)
- **Parse performance**: Same as native Rust (2-5 µs for small messages)
- **Cross-platform**: Works in all modern browsers and Node.js 16+

## [0.4.0] - 2025-10-08

### Added - Performance Optimizations ⚡

- **Cached Terser** (`vyna-hl7-terser/src/cache.rs`):
  - `CachedTerser` with path and segment location caching
  - 5-10x faster repeated field access (50-100ns vs 500ns)
  - Cache warming for predictable access patterns
  - Memory efficient: ~100 bytes per cached path
  - New methods: `with_capacity()`, `warm_cache()`, `clear_cache()`, `cache_size()`

- **Optimized Parser** (`vyna-hl7-parser/src/optimized.rs`):
  - Pre-allocation with capacity hints based on delimiter counts
  - Fast path for fields without escape sequences
  - Reduced memory allocations during parsing
  - 10-30% faster parsing for component-heavy messages
  - Functions: `parse_field_optimized()`, `parse_repetition_optimized()`, `parse_component_optimized()`

- **Benchmarking Suite**:
  - `vyna-hl7-parser/benches/parser_bench.rs` - Parser performance benchmarks
    - Small, medium, and large message benchmarks
    - Scaling benchmarks (10, 50, 100, 250, 500 segments)
    - Complex field parsing benchmarks
  - `vyna-hl7-terser/benches/terser_bench.rs` - Terser performance benchmarks
    - Simple field access, component access, indexed segments
    - Sequential access patterns
    - Path parsing performance by complexity

- **Documentation**:
  - `PERFORMANCE.md` - Comprehensive performance guide
    - Optimization strategies
    - Benchmarking guide
    - Profiling instructions
    - Best practices for high-throughput and low-latency scenarios
    - Known bottlenecks and future optimizations

### Changed

- Updated README.md with performance section and benchmark instructions
- Terser module refactored into separate path and cache modules for better organization

### Performance Characteristics

- **Parser**: 2-5 µs for small messages (3 segments), 8-12 µs for medium (8 segments)
- **Throughput**: 40,000-100,000 messages/second for typical messages
- **Terser**: 80-120ns cached access vs 500-800ns uncached
- **Memory**: Minimal overhead (~100 bytes per cached Terser path)

## [0.3.0] - 2025-10-08

### Added - FHIR R4 Conversion Complete ✅

- **6 New FHIR Resource Definitions:**
  - `Encounter` - Patient visit/encounter information
  - `DiagnosticReport` - Diagnostic test reports
  - `AllergyIntolerance` - Patient allergy and intolerance records
  - `Medication` / `MedicationAdministration` - Medication information and administration records
  - `Condition` - Patient conditions, problems, and diagnoses
  - `Procedure` - Procedures performed on patients
  - `Period` - Common data type for time periods (start/end)

- **6 New Production-Ready Converters:**
  - `EncounterConverter`: PV1 segment → FHIR Encounter resource
    - Patient class mapping (inpatient/outpatient/emergency)
    - Encounter participants (attending/referring/consulting doctors)
    - Location, service provider, hospitalization details
    - Admit/discharge dates and dispositions
  - `DiagnosticReportConverter`: OBR segment → FHIR DiagnosticReport resource
    - Universal service identifier mapping
    - Result status conversion
    - Automatic linking to Observation results
  - `AllergyIntoleranceConverter`: AL1 segment → FHIR AllergyIntolerance resource
    - Allergen type categorization (medication/food/environment)
    - Severity/criticality mapping
  - `MedicationConverter`: RXA segment → FHIR MedicationAdministration resource
    - Medication codes and dosage information
    - Administration date/time and status
  - `ConditionConverter`: PRB/DG1 segments → FHIR Condition resource
    - Problem and diagnosis code mapping
    - ICD-9/ICD-10 coding system support
  - `ProcedureConverter`: PR1 segment → FHIR Procedure resource
    - Procedure code mapping (ICD-9/ICD-10/CPT)
    - Procedure date/time

- **Documentation:**
  - `CONVERTERS.md` - Comprehensive converter reference guide with all field mappings
  - `EXAMPLES.md` - Working examples for ADT and ORU message conversion
  - Updated `README.md` with complete converter listing
  - `TERSER_INDEXING.md` - Component indexing documentation

### Fixed

- **Terser Component Indexing** - All converters now correctly use 0-based component indexing:
  - PatientConverter: Fixed PID-3 (identifiers) and PID-11 (addresses) component access
  - ObservationConverter: Fixed OBX-3 (code) and OBX-5 (value) component access
  - PractitionerConverter: Fixed XCN component access for all name and identifier fields
  - All converters tested and verified with correct indexing

### Changed

- Test suite expanded from 8 to **16 tests** - all passing ✅
- FHIR resources now include 9 resource types (was 3)
- Converters now include 9 converters (was 3)

### Summary

This release completes the core FHIR R4 conversion functionality, providing production-ready converters for all major HL7 v2 message types including ADT, ORU, RAS/RDE/RDS (pharmacy), DFT (financial), and MDM (medical documents). All converters are fully tested and include proper error handling, coding system mapping, and resource linking.

## [0.2.0] - 2025-10-07

### Added
- **Complex Field Builders** - Builder patterns for HL7 composite data types:
  - `XpnBuilder` - Extended Person Name (family, given, middle, suffix, prefix, degree)
  - `XadBuilder` - Extended Address (street, city, state, postal code, country, type)
  - `XtnBuilder` - Extended Telecommunication (phone, email, use code, equipment type)
  - `CxBuilder` - Extended Composite ID (ID number, check digit, assigning authority, identifier type)
  - `XcnBuilder` - Extended Composite Name for Persons (ID, name components, credentials)
  - Fluent API for building properly formatted composite fields
  - Automatic component separator (^) handling and trailing component trimming
  - New example: `complex_fields.rs` demonstrating all field builders
  - 7 comprehensive unit tests for all builder types
  - Exported from `builders::fields` module

- **Pharmacy Message Builders** - Fluent builder API for pharmacy messages:
  - `RdeO11Builder` - Pharmacy/Treatment Encoded Order
  - `RasO17Builder` - Pharmacy/Treatment Administration
  - `RdsO13Builder` - Pharmacy/Treatment Dispense
  - `RgvO15Builder` - Pharmacy/Treatment Give
  - `RraO18Builder` - Pharmacy/Treatment Administration Acknowledgment
  - `RrdO14Builder` - Pharmacy/Treatment Dispense Information
  - Consistent fluent API with existing builders (ADT, ORU, ORM, etc.)
  - Examples added to `message_builders.rs`
  - Exported from `builders::pharmacy` module

- **Laboratory Message Builders** - Fluent builder API for laboratory messages:
  - `OulR21Builder` - Unsolicited Laboratory Observation
  - `OmlO21Builder` - Laboratory Order
  - Consistent fluent API with existing builders (ADT, ORU, ORM, etc.)
  - Examples added to `message_builders.rs`
  - Exported from `builders::laboratory` module

- **Laboratory Message Schemas** - Support for 2 laboratory message types across all HL7 versions (2.3-2.7):
  - OUL (Unsolicited Laboratory Observation): R21
  - OML (Laboratory Order): O21
  - Total of 10 new schema files (2 types × 5 versions)
  - 3 new schema loader tests
  - Total message schema count: 43 types (was 41)

- **Additional Pharmacy Schemas** - Support for 3 more pharmacy message types across all HL7 versions (2.3-2.7):
  - RGV (Pharmacy/Treatment Give): O15
  - RRD (Pharmacy/Treatment Dispense Information): O14
  - RRA (Pharmacy/Treatment Administration Acknowledgment): O18
  - Total of 15 new schema files (3 types × 5 versions)
  - 4 new schema loader tests
  - Total message schema count: 41 types (was 38)

- **Additional ADT Builders** - Expanded ADT builder API with 12 new message variants:
  - `AdtA05Builder` - Pre-admit a Patient
  - `AdtA06Builder` - Change Outpatient to Inpatient
  - `AdtA07Builder` - Change Inpatient to Outpatient
  - `AdtA09Builder` - Patient Departing - Tracking
  - `AdtA10Builder` - Patient Arriving - Tracking
  - `AdtA11Builder` - Cancel Admit/Visit Notification
  - `AdtA12Builder` - Cancel Transfer
  - `AdtA13Builder` - Cancel Discharge/End Visit
  - `AdtA17Builder` - Swap Patients
  - `AdtA28Builder` - Add Person Information
  - `AdtA31Builder` - Update Person Information
  - `AdtA40Builder` - Merge Patient - Patient Identifier List
  - All builders use composition pattern for code reuse
  - Fluent API consistent with existing ADT builders (A01, A02, A03, A04, A08)
  - Comprehensive examples added to `message_builders.rs` for all 17 ADT variants
  - Total ADT builders: 17 variants (A01-A13, A17, A28, A31, A40)

## [0.1.3] - 2025-10-07

### Added
- **Additional Message Schemas** - Support for 6 new message types across all HL7 versions (2.3-2.7):
  - BAR (Billing Account Record): P01, P02
  - RDE (Pharmacy/Treatment Encoded Order): O11
  - RAS (Pharmacy/Treatment Administration): O17
  - RDS (Pharmacy/Treatment Dispense): O13
  - MFN (Master File Notification): M01
  - Total of 30 new schema files (6 types × 5 versions)
  - 6 new schema loader tests
  - Updated `list_available_schemas()` function
  - Total message schema count: 38 types (was 32)

- **Vocabulary/Code Set Validation** - Validation against HL7 standard tables:
  - TableRegistry with 13 built-in HL7 tables
  - Table 0001: Administrative Sex (M, F, O, U, etc.)
  - Table 0002: Marital Status
  - Table 0004: Patient Class (I, O, E, etc.)
  - Table 0007: Admission Type
  - Table 0061: Check Digit Scheme
  - Table 0063: Relationship
  - Table 0078: Interpretation Codes
  - Table 0085: Observation Result Status
  - Table 0103: Processing ID (P, D, T)
  - Table 0119: Order Control Codes (NW, CA, OK, etc.)
  - Table 0201: Telecommunication Use Code
  - Table 0203: Identifier Type (MR, SS, DL, etc.)
  - Table 0301: Universal ID Type
  - Support for custom/local tables
  - Deprecated code detection
  - Integration with schema-based validation
  - Field-to-table mapping via schema table_id field
- New examples: `vocabulary_validation.rs` and `complete_validation.rs`
- 8 new tests for vocabulary validation (total: 101 tests across all crates)

- **Data Type Validation** - Format validation for all HL7 data types:
  - Date/Time types (DT, TM, DTM, TS) with format verification
  - Numeric types (NM, SI) with range and format validation
  - String types (ST, TX, FT) with basic validation
  - Identifier types (ID, EI, CX, HD) with format rules
  - Coded elements (CE, CWE, CNE) with component structure validation
  - Composite types (XPN, XAD, XTN) for names, addresses, and telecom
  - Message type (MSG) and processing type (PT) validation
  - Numeric array (NA) validation
- Integrated data type validation into the schema-based validator
- New examples: `datatype_validation.rs` and `enhanced_validation.rs`
- Comprehensive test suite for data type validation (19 new tests)

## [0.1.1] - 2025-10-07

### Added
- **Message Builders** - Fluent builder API for creating HL7 messages programmatically:
  - `AdtBuilder` with support for A01, A02, A03, A04, A08 (expanded to A05-A13 in later version)
  - `OruR01Builder` for observation results
  - `OrmO01Builder` for orders
  - `SiuS12Builder` for scheduling
  - `MdmT01Builder` for medical documents
  - `DftP03Builder` for financial transactions
  - `QryA19Builder`, `QryQ01Builder`, `QryQ02Builder` for query messages
- **29 new message schemas** across all HL7 versions (v2.3-v2.7):
  - **ADT messages**: A02, A03, A04, A05, A06, A07, A08, A09, A10, A11, A12, A13, A17, A28, A31, A40
  - **SIU messages**: S12, S13, S14, S15 (Scheduling Information)
  - **MDM messages**: T01, T02, T04 (Medical Document Management)
  - **DFT messages**: P03, P11 (Detailed Financial Transactions)
  - **QRY messages**: A19, Q01, Q02 (Query Messages)
- Expanded trigger event constants in `message::trigger_events` module with all new message types
- Updated `list_available_schemas()` to include all 32 message schemas
- New example: `message_builders.rs` demonstrating builder API usage
- Comprehensive documentation updates in README.md and schemas/README.md

### Changed
- Schema loader now supports 32 total message schemas (up from 4)
- Total of 160 schema files across all HL7 versions (32 schemas × 5 versions)

## [0.1.0] - Initial Release

### Added
- Core HL7 v2.x data structures (Message, Segment, Field, Component, Subcomponent)
- Delimiter-based parser for efficient HL7 parsing
- Support for HL7 v2.3, v2.3.1, v2.4, v2.5, v2.5.1, v2.6, v2.7, v2.7.1
- Terser API for path-based field access
- Message validation against HL7 standards
- Schema-based validation with initial schemas:
  - ADT^A01 (Admit/Visit Notification)
  - ORU^R01 (Observation Result)
  - ORM^O01 (Order Message)
  - ACK (Acknowledgment)
- MLLP protocol support for network transmission
- HL7 encoding and escape sequence handling
- ACK message generation
- Comprehensive test coverage
- Documentation and examples

[0.22.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.19.0...v0.22.0
[0.19.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.18.0...v0.19.0
[0.18.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.17.0...v0.18.0
[0.17.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.16.0...v0.17.0
[0.16.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.15.0...v0.16.0
[0.15.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.14.0...v0.15.0
[0.14.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.13.0...v0.14.0
[0.13.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.12.0...v0.13.0
[0.12.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.11.0...v0.12.0
[0.11.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.10.0...v0.11.0
[0.10.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.9.0...v0.10.0
[0.9.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.8.0...v0.9.0
[0.8.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.7.0...v0.8.0
[0.7.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.6.0...v0.7.0
[0.6.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.5.0...v0.6.0
[0.5.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.4.0...v0.5.0
[0.4.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.3.0...v0.4.0
[0.3.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.2.0...v0.3.0
[0.2.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.1.3...v0.2.0
[0.1.3]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.1.1...v0.1.3
[0.1.1]: https://gitlab.flostel.com/alexshao/vyna-hl7/compare/v0.1.0...v0.1.1
[0.1.0]: https://gitlab.flostel.com/alexshao/vyna-hl7/releases/tag/v0.1.0
