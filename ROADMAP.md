# VynaHL7 Development Roadmap

This document outlines the planned development phases for VynaHL7 to become a world-class, feature-rich HL7 v2.x library that exceeds HAPI capabilities.

## Vision

Transform VynaHL7 into the most comprehensive, performant, and feature-rich HL7 library in any language, with:
- **Feature parity** with HAPI (Java's leading HL7 library)
- **Superior features** not available in HAPI (transformation, templates, advanced rules)
- **Rust performance** and memory safety guarantees
- **Enterprise-grade** reliability and robustness

---

## Current Status (v0.16.0)

### Completed Features ✅

- **Core HL7 Parsing**: Delimiter-based parser, support for v2.3-v2.8.2
- **Terser API**: Path-based field access (e.g., `PID-5-1`, `OBX(2)-5`)
- **Cached Terser**: 5-10x faster repeated field access
- **Enhanced Terser**: Bulk extraction, pattern matching, field iteration, conditional queries ✨ v0.10.0
- **Message Builders**: ADT (13 variants), ORU, ORM, SIU, MDM, DFT, QRY, QBP, RSP
- **Complex Field Builders**: XPN, XAD, XTN, CX, XCN, QPD, RCP, QAK ✨ v0.12.0
- **Batch/File Support**: BatchBuilder, FileBuilder, parse_batch(), parse_file() ✨ v0.13.0
- **Message Transformation**: Field mapping, 15 built-in transforms, YAML/JSON config ✨ v0.14.0
- **Message Templates**: Template system with variable substitution, inheritance, 7 pre-built templates ✨ v0.15.0
- **Schema Validation**: 43 message types across 6 HL7 versions (v2.3-v2.8)
- **Data Type Validation**: All HL7 data types (dates, times, numerics, coded values)
- **Vocabulary Validation**: HL7 standard tables (13 tables)
- **Conformance Profile Validation**: Phase 1 MVP (Usage, Cardinality, Length)
- **Conformance Phase 2 Profiles**: Component validation, conditional predicates, value sets, co-constraints ✨ v0.16.0
- **Custom Z-Segments**: Type-safe custom segment framework
- **FHIR Conversion**: 12 converters (Patient, Observation, Encounter, Immunization, ServiceRequest, Specimen, etc.) ✨ v0.11.0
- **Query/Response Support**: QBP/RSP builders, QueryResultParser, pagination ✨ v0.12.0
- **MLLP Protocol**: Network transmission support
- **HTTP Transport**: HL7-over-HTTP support
- **WebAssembly**: JavaScript/TypeScript bindings
- **CLI Tool**: Parse, validate, extract, convert, info commands
- **Test Coverage**: 576+ tests across all crates

### Phase 1 Complete ✅

All Phase 1 sprints have been successfully implemented:
- ✅ **Sprint 1 (v0.10.0)**: Enhanced Terser Capabilities
- ✅ **Sprint 2 (v0.11.0)**: FHIR Converters Expansion
- ✅ **Sprint 3 (v0.12.0)**: Query/Response Support

### Current Architecture

```
vyna-hl7/
├── vyna-hl7-core        - Core data structures (Message, Segment, Field)
├── vyna-hl7-parser      - HL7 message parser (delimiter-based)
├── vyna-hl7-validator   - Message validation against HL7 standards
├── vyna-hl7-conformance - Conformance profile validation (XML-based)
├── vyna-hl7-terser      - Path-based field access API
├── vyna-hl7-transform   - Message transformation framework ✨ v0.14.0
├── vyna-hl7-templates   - Message template system ✨ v0.15.0
├── vyna-hl7-custom      - Type-safe custom Z-segment framework
├── vyna-hl7-mllp        - MLLP protocol for network transmission
├── vyna-hl7-http        - HTTP transport for inter-organization communication
├── vyna-hl7-fhir        - HL7 v2 to FHIR R4 conversion
├── vyna-hl7-wasm        - WebAssembly bindings for JavaScript/TypeScript
├── vyna-hl7-cli         - Command-line interface for message analysis
└── vyna-hl7-macros      - Derive macros for message types
```

---

## PHASE 1: Quick Wins ✅ COMPLETE

**Timeline**: 2-3 weeks
**Goal**: Deliver immediate productivity improvements
**Status**: ✅ **COMPLETED** - All 3 sprints delivered (v0.10.0, v0.11.0, v0.12.0)

### 1.1 Enhanced Terser Capabilities ✅ (v0.10.0)

**New Features:**
- **Bulk Extraction**: Extract multiple fields in one call
  - `get_multiple(&["PID-5-1", "PID-7", "PID-8"])` → HashMap
- **Pattern Matching**: Find all fields matching a pattern
  - `get_pattern("OBX(*)-5")` → All OBX observation values
  - `get_pattern("PID-11(*)-1")` → All address street components
- **Field Iteration**: Iterator API for walking through segments/fields
  - `FieldIterator` for iterating over repeating segments
- **Conditional Queries**: Get fields based on conditions
  - `get_if("OBX-5", |v| v.parse::<f64>() > 100.0)`
  - `filter_repeating("OBX", 3, "GLU")` → Find OBX where field 3 = "GLU"

**New Modules:**
- `vyna-hl7-terser/src/bulk.rs` (~200 LOC)
- `vyna-hl7-terser/src/iterator.rs` (~150 LOC)
- `vyna-hl7-terser/src/query.rs` (~250 LOC)
- Pattern parser (~100 LOC)
- Tests (~300 LOC)

**HAPI Equivalent**: `Terser.getFinder()`, `MessageIterator`

**Deliverable**: ✅ vyna-hl7-terser v0.10.0 - COMPLETED

---

### 1.2 FHIR Converters Expansion ✅ (v0.11.0)

**New Converters:**
1. **ServiceRequest** (ORC/OBR → FHIR ServiceRequest)
   - Order control codes, universal service identifiers
   - Priority, status, intent mapping

2. **Specimen** (SPM → FHIR Specimen)
   - Specimen type, collection method, body site
   - Container, handling conditions

3. **Coverage** (IN1/IN2 → FHIR Coverage)
   - Insurance plan, subscriber information
   - Coverage periods, relationship codes

4. **Location** (PV1 → FHIR Location)
   - Assigned patient location (PV1-3)
   - Building, floor, room, bed

5. **Organization** (MSH/NK1 → FHIR Organization)
   - Sending/receiving facilities (MSH-3, MSH-4)
   - Employer organization (NK1-13)

**Implementation:**
- `vyna-hl7-fhir/src/converters/service_request.rs` (~200 LOC)
- `vyna-hl7-fhir/src/converters/specimen.rs` (~150 LOC)
- `vyna-hl7-fhir/src/converters/coverage.rs` (~200 LOC)
- `vyna-hl7-fhir/src/converters/location.rs` (~150 LOC)
- `vyna-hl7-fhir/src/converters/organization.rs` (~150 LOC)
- FHIR resource structures if needed (~300 LOC)
- Tests (~400 LOC)

**Result**: 12 total FHIR converters (complete core resource coverage)

**Deliverable**: ✅ vyna-hl7-fhir v0.11.0 - COMPLETED

---

### 1.3 Query/Response Support ✅ (v0.12.0)

**New Features:**
- **QBP Builder** (Query by Parameter)
  - QPD segment support
  - Query tag and parameter management
  - Fluent builder API

- **RSP Builder** (Response)
  - Query acknowledgment codes (OK, NF, AE, AR)
  - Response to original query
  - Result segment management

- **Query Result Parser**
  - Extract results from RSP messages
  - Parse query acknowledgment status
  - Handle paginated results

**Implementation:**
- Extend `vyna-hl7-core/src/builders/qry.rs`
- `QbpBuilder` (~200 LOC)
- `RspBuilder` (~250 LOC)
- QPD segment support (~100 LOC)
- `QueryResultParser` (~200 LOC)
- Tests and examples (~300 LOC)

**HAPI Equivalent**: `QueryResponseMessageBuilder`, `MessageQuery`

**Deliverable**: ✅ vyna-hl7-core v0.12.0, vyna-hl7-terser v0.12.0 - COMPLETED

---

### Phase 1 Summary ✅ COMPLETE

**Total Code**: ~3,100 LOC (implementation + tests)
**New Crate Versions**: vyna-hl7-terser v0.10.0, vyna-hl7-fhir v0.10.0, vyna-hl7-core v0.10.0
**Examples**: 5 new comprehensive examples
**Value**: Immediate productivity boost, complete FHIR coverage, query support

---

## PHASE 2: Structural Enhancements ✅ COMPLETE

**Timeline**: 3-4 weeks
**Goal**: Add infrastructure for enterprise-scale processing
**Status**: ✅ **COMPLETED** - All 3 sprints delivered (v0.13.0, v0.14.0, v0.15.0)

### 2.1 Batch/File Support ✅ (v0.13.0)

**New Features:**
- **FHS/BHS Segments**: File Header Segment, Batch Header Segment
- **FTS/BTS Segments**: File Trailer Segment, Batch Trailer Segment
- **Batch Structures**: `Batch` with header, messages, trailer
- **File Structures**: `File` with header, batches, trailer
- **Batch Parser**: Parse batch/file HL7 messages (`parse_batch()`, `parse_file()`)
- **Batch Builders**: Fluent API for creating batches/files (`BatchBuilder`, `FileBuilder`)
- **Batch Validation**: Validate batch/file structure and counts

**Implementation:**
- FHS/BHS/FTS/BTS segment definitions in vyna-hl7-core/src/batch.rs (~590 LOC)
- `Batch` and `File` structures with validation (~250 LOC)
- Batch/file parser in vyna-hl7-parser (~350 LOC)
- Batch/file builders in vyna-hl7-core/src/builders/batch.rs (~446 LOC)
- Segment encoding enhancements for FHS/BHS (~20 LOC)
- Tests (~250 LOC) and examples (~451 LOC)

**HAPI Equivalent**: `Parser.parse()` with batch detection, `BatchParser`

**Use Cases**: High-volume message processing, EDI file handling

**Deliverable**: ✅ vyna-hl7-core v0.13.0, vyna-hl7-parser v0.13.0 - COMPLETED

---

### 2.2 Message Transformation Framework ✅ (v0.14.0)

**New Crate**: vyna-hl7-transform

**Features:**
- **Transformation Rules**: Source → Target field mapping
- **Transform Functions**: Built-in transforms (uppercase, date format, etc.)
- **Declarative Mapping**: YAML/JSON configuration files
- **Fluent Builder API**: Programmatic mapping definition
- **HL7 v2 → HL7 v2**: Transform between message types/versions
- **Custom Transforms**: User-defined transformation functions

**Architecture:**
```rust
pub struct TransformationRule {
    source_path: String,
    target_path: String,
    transform_fn: Option<TransformFn>,
}

pub struct MessageTransformer {
    rules: Vec<TransformationRule>,
}
```

**Mapping File Example (YAML):**
```yaml
rules:
  - source: "PID-5-1"
    target: "PID-5-1"
    transform: "uppercase"
  - source: "PID-7"
    target: "PID-7"
    transform: "format_date:YYYYMMDD->YYYY-MM-DD"
```

**Implementation:**
- Error types and result handling (~68 LOC)
- Transformation rules with context support (~220 LOC)
- Built-in transform functions (15 functions, ~410 LOC)
- `MessageTransformer` engine with fluent API (~380 LOC)
- YAML/JSON configuration support (~290 LOC, serde feature)
- Integration with vyna-hl7-terser for field access
- Tests (~350 LOC) and examples (~227 LOC)

**Built-in Transforms:** uppercase, lowercase, trim, remove_whitespace, substring, format_date, format_datetime, replace, regex_replace, prefix, suffix, pad, default_if_empty

**Use Cases**: Version migration, message normalization, field mapping, data anonymization, format conversion

**Deliverable**: ✅ vyna-hl7-transform v0.11.0 - COMPLETED

---

### 2.3 Segment/Message Templates ✅ (v0.15.0)

**New Crate**: vyna-hl7-templates

**Features:**
- **Message Templates**: Reusable message patterns with metadata
- **Segment Templates**: Reusable segment patterns with field definitions
- **Field Templates**: Component-level templates with placeholders
- **Template Engine**: Create messages from templates with variable substitution
- **YAML/JSON Templates**: Declarative template format with serde support
- **Template Library**: 7 pre-built standard templates (ADT, ORU, ORM, SIU, MDM)
- **Template Inheritance**: Multi-level inheritance with override support
- **Template Validation**: Validate messages against template definitions
- **Variable Substitution**: `{{variable}}` placeholder system
- **Default Values**: Template and engine-level defaults

**Template Example (YAML):**
```yaml
name: "Basic ADT A01"
version: "2.5"
message_type: "ADT"
trigger_event: "A01"
segments:
  - id: MSH
    required: true
    fields:
      3: { required: true, placeholder: "{{sending_app}}" }
      4: { required: true, placeholder: "{{sending_facility}}" }
  - id: PID
    required: true
    fields:
      5: { required: true, placeholder: "{{patient_name}}" }
      7: { required: false, placeholder: "{{dob}}" }
```

**Implementation:**
- Template data structures with serde support (~390 LOC)
- Template engine with variable substitution (~350 LOC)
- YAML/JSON parser with roundtrip support (~360 LOC)
- Template validation with errors/warnings (~400 LOC)
- Standard template library with 7 templates (~345 LOC)
- Inheritance resolver with circular detection (~360 LOC)
- Error types and handling (~68 LOC)
- 47 unit tests + 14 doc tests (~2100 LOC)
- 3 working examples (~364 LOC)

**HAPI Equivalent**: `ModelClassFactory`, message structure definitions

**Use Cases**: Rapid message creation, organization-specific patterns, message validation

**Deliverable**: ✅ vyna-hl7-templates v0.11.0 - COMPLETED

---

### Phase 2 Summary ✅

**Total Code**: ~6,723 LOC (implementation + tests + examples)
**New Crates**: vyna-hl7-transform v0.11.0 ✅, vyna-hl7-templates v0.11.0 ✅
**Updated Crates**: vyna-hl7-core v0.13.0 ✅, vyna-hl7-parser v0.13.0 ✅
**Examples**: 9 new working examples (3 batch/file, 3 transformation, 3 template)
**Tests**: 96 unit tests + 33 doc tests = 129 tests
**Value**: High-volume batch processing, message transformation, reusable template patterns

---

## PHASE 3: Advanced Validation & Orchestration (v0.12.0)

**Timeline**: 4-5 weeks
**Goal**: Enterprise-grade validation and workflow capabilities

### 3.1 Phase 2 Conformance Profiles ✅ (v0.16.0) COMPLETED

**Extensions to vyna-hl7-conformance:**

**New Features:** ✅ IMPLEMENTED
- ✅ **Component-Level Validation**: Validate components within fields
- ✅ **Conditional Predicates**: C (Conditional) usage with condition evaluation
- ✅ **Value Set Binding**: Required/Extensible/Preferred/Example strength
- ✅ **Co-Constraints**: Cross-field validation rules in profiles (structures only, evaluation TBD)
- ✅ **Predicate Engine**: Evaluate complex conditions using Terser
- ⏳ **Extended XML Parser**: Support Phase 2 XML elements (deferred to future sprint)

**New Structures:**
```rust
pub struct ComponentProfile {
    position: usize,
    name: Option<String>,
    usage: Usage,
    datatype: Option<String>,
    length: Option<usize>,
    table_id: Option<String>,
}

pub enum ConditionalUsage {
    Required,
    RequiredIfKnown,
    Optional,
    NotUsed,
    Conditional(Predicate),
}

pub struct Predicate {
    condition: String,
    true_usage: Usage,
    false_usage: Usage,
}

pub struct ValueSet {
    id: String,
    name: String,
    binding_strength: BindingStrength,
    values: Vec<ValueSetEntry>,
}

pub struct CoConstraint {
    id: String,
    description: String,
    condition: String,
}
```

**Implementation:**
- Component profile structures (~100 LOC)
- Predicate parser and evaluator (~300 LOC)
- Value set structures and validation (~200 LOC)
- Co-constraint structures and validation (~250 LOC)
- Extended XML parser (~200 LOC)
- Extended validator (~300 LOC)
- Tests (~500 LOC)

**HAPI Equivalent**: `ConformanceProfileRule`, `Predicate` evaluation

**Deliverable**: ✅ vyna-hl7-conformance v0.16.0 (Phase 2 complete)

**Actual Implementation:**
- Component profile structures (ComponentProfile) (~150 LOC)
- Predicate parser and evaluator (PredicateParser, PredicateEvaluator, Condition enum) (~400 LOC)
- Value set structures (ValueSetBinding, BindingStrength) (~80 LOC)
- Co-constraint structures (CoConstraint) (~50 LOC)
- ConditionalUsage enum with Predicate support (~100 LOC)
- Extended FieldProfile and MessageProfile (~50 LOC updates)
- Updated ConformanceValidator with conditional usage support (~100 LOC)
- 16 comprehensive predicate tests + integration tests (~600 LOC)
- predicate_validation.rs example demonstrating all Phase 2 features (~220 LOC)
- Full documentation in CHANGELOG.md

**Total**: ~1,750 LOC across Phase 2 implementation

**Deferred to Future Sprint**:
- XML parser support for Phase 2 elements (will be added in future sprint when needed)
- Value set validation against actual code tables (Phase 4)
- Co-constraint evaluation engine (Phase 4)
- Predicate expression functions like AGE(), LENGTH() (Phase 4)

---

### 3.2 Advanced Validation Rules ✅ (v0.17.0)

**Extensions to vyna-hl7-validator:**

**New Features:**
- **Custom Business Rules**: User-defined validation logic
- **Rules Engine**: Execute validation rules on messages
- **Cross-Field Validation**: Dependencies between fields
- **Declarative Rules**: YAML/JSON rule configuration
- **Rule Severity**: Error/Warning/Info classification
- **Built-in Rules**: Common validation patterns

**Architecture:**
```rust
pub struct ValidationRule {
    name: String,
    description: String,
    severity: RuleSeverity,
    condition: Box<dyn Fn(&Message) -> ValidationRuleResult>,
}

pub struct RulesEngine {
    rules: Vec<ValidationRule>,
}

pub struct CrossFieldValidator;
impl CrossFieldValidator {
    fn if_then(...) -> ValidationRule;
    fn mutually_exclusive(...) -> ValidationRule;
    fn at_least_one(...) -> ValidationRule;
}
```

**Declarative Rules Example (YAML):**
```yaml
rules:
  - name: "Patient gender required"
    severity: error
    condition: "PID-8 IS VALUED"
    message: "Patient gender must be provided"

  - name: "Adult patients need SSN"
    severity: warning
    condition: "IF AGE(PID-7) > 18 THEN PID-19 IS VALUED"
    message: "Adult patients should have SSN"
```

**Implementation:**
- RulesEngine and ValidationRule structures in vyna-hl7-validator/src/rules.rs (~280 LOC)
- CrossFieldValidator with 7 common patterns in vyna-hl7-validator/src/rules/cross_field.rs (~495 LOC)
- Declarative YAML/JSON parser in vyna-hl7-validator/src/rules/declarative.rs (~415 LOC)
- BuiltinRules library with 24 pre-built rules in vyna-hl7-validator/src/rules/builtin.rs (~410 LOC)
- Validator integration (~40 LOC)
- Tests (~900 LOC total, 83 tests passing)
- Examples: rules_basic.rs, rules_declarative.rs, rules_builtin.rs (~650 LOC)

**HAPI Equivalent**: `ValidationRule` interface, `RuleValidator`

**Use Cases**: Custom business logic validation, cross-field dependencies, organization-specific rules

**Deliverable**: ✅ vyna-hl7-validator v0.17.0 - COMPLETED

---

### 3.3 Message Routing/Orchestration ✅ (v0.18.0)

**New Crate**: vyna-hl7-orchestration

**Features:** ✅ All Implemented
- **Content-Based Routing**: Route by field values ✅
- **Message Orchestration**: Multi-step workflows ✅
- **Message Filtering**: Predicate-based filtering ✅
- **Async Workflows**: Tokio-based async orchestration ✅
- **Error Handling**: Retry logic with configurable backoff ✅
- **Workflow Builder**: Fluent API for defining workflows ✅

**Architecture:**
```rust
pub struct ContentRouter {
    routes: Vec<ContentRoute>,
}

pub struct ContentRoute {
    name: String,
    condition: Box<dyn Fn(&Message) -> bool>,
    handler: RouteHandler,
}

pub struct MessageOrchestrator {
    steps: Vec<OrchestrationStep>,
}

pub struct MessageFilter {
    filters: Vec<FilterRule>,
}
```

**Example Usage:**
```rust
let mut orchestrator = MessageOrchestrator::new();
orchestrator
    .add_step("validate", |msg| validator.validate(&msg))
    .add_step("transform", |msg| transformer.transform(&msg))
    .add_step("route", |msg| router.route(&msg));

orchestrator.execute_async(message).await?;
```

**Implementation:**
- Content-based routing (~200 LOC)
- Message filtering (~150 LOC)
- Orchestration engine (~300 LOC)
- Async orchestration (~200 LOC)
- Builder pattern (~150 LOC)
- Tests and examples (~400 LOC)

**HAPI Equivalent**: Apache Camel integration (HAPI uses Camel)

**Use Cases**: Integration engines, message processing pipelines

**Testing:**
- 18 unit tests covering all features
- 6 doctests for API examples
- 3 comprehensive examples (routing, filtering, complete workflow)

**Deliverable**: ✅ vyna-hl7-orchestration v0.18.0 - COMPLETED

---

### Phase 3 Summary ✅ COMPLETE

**Total Code**: ~4,850 LOC (implementation + tests)
**Updated Crates**: vyna-hl7-conformance v0.16.0, vyna-hl7-validator v0.17.0
**New Crates**: vyna-hl7-orchestration v0.18.0
**Examples**: 11 new examples (conformance, validation rules, orchestration)
**Value**: Production-ready validation, business rules engine, enterprise workflows
**Status**: ✅ **COMPLETED** - All 3 sprints delivered (v0.16.0, v0.17.0, v0.18.0)

---

## Feature Parity Assessment

### After Phase 3 Completion

| Feature | HAPI | VynaHL7 v0.9.0 | VynaHL7 v0.12.0 |
|---------|------|------------|-------------|
| **Core Parsing** | ✅ | ✅ | ✅ |
| **Terser Access** | ✅ | ✅ | ✅✅ (enhanced) |
| **Bulk Operations** | ✅ | ❌ | ✅ |
| **Pattern Matching** | ✅ | ❌ | ✅ |
| **Schema Validation** | ✅ | ✅ | ✅ |
| **Data Type Validation** | ✅ | ✅ | ✅ |
| **Vocabulary Validation** | ✅ | ✅ | ✅ |
| **Conformance Profiles** | ✅ Phase 1 | ✅ Phase 1 | ✅ Phase 2 |
| **Batch/File Support** | ✅ | ❌ | ✅ |
| **Query/Response** | ✅ | Partial | ✅ |
| **FHIR Conversion** | ✅ | ✅ (9) | ✅ (14+) |
| **Message Builders** | ✅ | ✅ | ✅ |
| **MLLP Protocol** | ✅ | ✅ | ✅ |
| **HTTP Transport** | ✅ | ✅ | ✅ |
| **Transformation** | ❌ | ❌ | ✅ (VynaHL7 exclusive) |
| **Templates** | ❌ | ❌ | ✅ (VynaHL7 exclusive) |
| **Rules Engine** | ❌ | ❌ | ✅ (VynaHL7 exclusive) |
| **Orchestration** | Via Camel | Basic | ✅ |

### VynaHL7 Advantages Over HAPI

**✅ Feature Parity Achieved:**
- All core HAPI features implemented
- Comprehensive conformance profile support
- Complete query/response support

**🚀 Features Exceeding HAPI:**
- **Message Transformation Framework**: Declarative field mapping, version conversion
- **Template System**: Reusable message patterns, organization-specific templates
- **Advanced Rules Engine**: Declarative validation rules, cross-field validation
- **Modern Architecture**: Rust performance, memory safety, async I/O
- **Better Developer Experience**: Fluent builders, type safety, comprehensive examples

---

## Success Metrics

### Code Quality
- ✅ Maintain >80% test coverage
- ✅ Zero compiler warnings
- ✅ All features documented with examples
- ✅ Benchmarks for performance-critical features

### Performance Targets
- ✅ Parse performance: <10 µs for medium messages
- ✅ Terser cached access: <100 ns
- ✅ Throughput: >50,000 messages/sec
- ✅ Memory efficiency: Minimal allocations

### Test Coverage
- ✅ Current: 335+ tests
- ✅ Phase 1: +150 tests (485 total)
- ✅ Phase 2: +180 tests (665 total)
- ✅ Phase 3: +270 tests (935 total)

### Documentation
- ✅ Every feature has working examples
- ✅ API documentation for all public types
- ✅ README per crate
- ✅ Migration guides for breaking changes

---

## Implementation Principles

### 1. Incremental Value Delivery
- Each phase delivers independently useful features
- No phase blocks on future phases
- Regular releases (v0.10.0, v0.11.0, v0.12.0)

### 2. Backward Compatibility
- Maintain compatibility within major versions
- Use deprecation warnings for breaking changes
- Provide migration guides

### 3. Performance First
- Zero-copy parsing where possible
- Benchmark all new features
- Profile and optimize hot paths

### 4. Comprehensive Testing
- Unit tests for all new code
- Integration tests with real HL7 messages
- Property-based testing where applicable

### 5. Clear Documentation
- Example for every feature
- API documentation with usage examples
- Architecture documentation

### 6. Following Existing Patterns
- Workspace architecture
- Builder patterns
- Error handling with thiserror
- Module organization

---

## Code Volume Summary

| Phase | Implementation LOC | Test LOC | Total LOC |
|-------|-------------------|----------|-----------|
| Phase 1 | 2,100 | 1,000 | 3,100 |
| Phase 2 | 2,650 | 1,400 | 4,050 |
| Phase 3 | 3,450 | 1,100 | 4,550 |
| **Total** | **8,200** | **3,500** | **11,700** |

---

## Timeline Summary

| Phase | Duration | Features | Release |
|-------|----------|----------|---------|
| Phase 1 | 2-3 weeks | Enhanced Terser, FHIR, Query/Response | v0.10.0 |
| Phase 2 | 3-4 weeks | Batch/File, Transformation, Templates | v0.11.0 |
| Phase 3 | 4-5 weeks | Conformance Phase 2, Rules, Orchestration | v0.12.0 |
| **Total** | **~12 weeks** | **9 major feature sets** | **3 releases** |

---

## Next Steps

1. **Start Phase 2**: Batch/File Support (Section 2.1)
2. **Track Progress**: Update this roadmap as features are completed
3. **Regular Releases**: Version bump and publish at end of each phase
4. **Documentation**: Maintain CHANGELOG.md with detailed release notes

---

## Maintenance

This roadmap is a living document and will be updated as:
- Features are completed (mark with ✅)
- Priorities change
- New requirements emerge
- Performance targets are achieved

**Last Updated**: 2025-11-20
**Current Version**: v0.13.0
**Next Milestone**: Phase 2 - Message Transformation Framework (v0.14.0)
