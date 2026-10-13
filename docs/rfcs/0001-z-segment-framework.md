# RFC 0001: Z-Segment Framework

**Status**: Implemented
**Author**: VynaHL7 Team
**Created**: 2025-11-19
**Implemented**: 2025-11-19
**Version**: 0.8.0

## Summary

This RFC proposes a comprehensive framework for handling custom Z-segments in VynaHL7, enabling users to define, register, and parse site-specific HL7 segments that are not part of the standard specification.

## Motivation

Almost every real-world HL7 v2.x integration uses custom Z-segments for organization-specific data. Currently, VynaHL7 can parse unknown segments as generic `Segment` structures, but this lacks:

1. **Type safety**: No strongly-typed access to Z-segment fields
2. **Validation**: Cannot validate Z-segment structure or data types
3. **Builder API**: No fluent builders for creating Z-segments
4. **Documentation**: Z-segments remain undocumented in code
5. **Reusability**: Cannot share Z-segment definitions across projects

This limitation makes VynaHL7 difficult to use in production environments where Z-segments are essential.

## Prior Art

- **HAPI**: Uses `CustomModelClassFactory` that searches packages for custom segment classes
- **NHapi (.NET)**: Similar plugin architecture for custom segments
- **Industry Practice**: Most organizations maintain internal libraries of Z-segment definitions

## Design Goals

1. **Ease of Use**: Defining Z-segments should be as easy as standard segments
2. **Type Safety**: Strongly-typed field access with compile-time checking
3. **Flexibility**: Support any Z-segment structure without restrictions
4. **Performance**: Zero runtime overhead for standard segments
5. **Backward Compatibility**: Existing code continues to work unchanged

## Detailed Design

### 1. Architecture Overview

```
┌─────────────────┐
│  User Code      │
│  (defines ZPV)  │
└────────┬────────┘
         │
         │ #[z_segment] macro
         ↓
┌─────────────────┐
│  Generated      │
│  ZPV Struct     │
└────────┬────────┘
         │
         │ implements CustomSegment
         ↓
┌─────────────────┐      ┌──────────────┐
│  Registry       │◄─────│  Parser      │
│  (HashMap)      │      │  (queries)   │
└─────────────────┘      └──────────────┘
```

### 2. Core Trait: `CustomSegment`

```rust
/// Trait for custom Z-segments
pub trait CustomSegment: Send + Sync {
    /// Segment ID (e.g., "ZPV", "ZCU")
    fn segment_id() -> &'static str;

    /// Parse segment from fields
    fn from_segment(segment: &Segment) -> Result<Self>
    where
        Self: Sized;

    /// Convert to generic segment
    fn to_segment(&self) -> Segment;

    /// Validate segment structure
    fn validate(&self) -> ValidationResult {
        Ok(()) // Default: no validation
    }

    /// Get field definitions for documentation
    fn field_definitions() -> Vec<FieldDefinition> {
        Vec::new() // Default: no definitions
    }
}
```

### 3. Registry API

```rust
/// Global registry for custom segments
pub struct CustomSegmentRegistry {
    segments: HashMap<String, Box<dyn CustomSegmentFactory>>,
}

impl CustomSegmentRegistry {
    /// Get the global registry instance
    pub fn global() -> &'static CustomSegmentRegistry;

    /// Register a custom segment type
    pub fn register<T: CustomSegment + 'static>(&mut self);

    /// Get segment factory by ID
    pub fn get(&self, id: &str) -> Option<&dyn CustomSegmentFactory>;

    /// Check if segment is registered
    pub fn is_registered(&self, id: &str) -> bool;

    /// List all registered segment IDs
    pub fn registered_ids(&self) -> Vec<&str>;
}

/// Factory trait for creating custom segments
pub trait CustomSegmentFactory: Send + Sync {
    fn create(&self, segment: &Segment) -> Result<Box<dyn Any>>;
    fn segment_id(&self) -> &'static str;
}
```

### 4. Declarative Macro: `z_segment!`

```rust
/// Define a custom Z-segment with fluent API
#[macro_export]
macro_rules! z_segment {
    (
        $name:ident,
        id = $id:expr,
        fields = {
            $($field_num:literal => $field_name:ident : $field_type:ty $(= $default:expr)?),* $(,)?
        }
        $(, validate = $validate_fn:expr)?
    ) => {
        // Generated code:
        // 1. Struct definition
        // 2. CustomSegment implementation
        // 3. Builder pattern
        // 4. Field accessors
        // 5. Auto-registration
    };
}
```

### 5. Usage Examples

#### Example 1: Simple Z-Segment

```rust
use vyna_hl7_custom::z_segment;

z_segment! {
    ZPV,  // struct name
    id = "ZPV",  // segment ID
    fields = {
        1 => visit_type: String,
        2 => visit_number: String,
        3 => patient_class: Option<String>,
        4 => admit_datetime: Option<String>,
    }
}

// Usage:
let zpv = ZPV::builder()
    .visit_type("OUTPATIENT")
    .visit_number("12345")
    .patient_class("E")
    .build()?;

let segment = zpv.to_segment();
```

#### Example 2: Z-Segment with Validation

```rust
z_segment! {
    ZCU,
    id = "ZCU",
    fields = {
        1 => customer_id: String,
        2 => account_number: String,
        3 => balance: Option<f64>,
    },
    validate = |s: &ZCU| {
        if s.balance.unwrap_or(0.0) < 0.0 {
            return Err(ValidationError::custom("Balance cannot be negative"));
        }
        Ok(())
    }
}
```

#### Example 3: Parsing Messages with Z-Segments

```rust
use vyna_hl7_custom::CustomSegmentRegistry;

// Register custom segments at startup
fn init_custom_segments() {
    CustomSegmentRegistry::global()
        .register::<ZPV>()
        .register::<ZCU>();
}

// Parse message with Z-segments
let message = parse_message(hl7_with_zpv)?;

// Access as generic segment
let zpv_segment = message.get_segment("ZPV").unwrap();

// Or parse as typed segment
let zpv = ZPV::from_segment(zpv_segment)?;
println!("Visit Type: {}", zpv.visit_type);
```

#### Example 4: Complex Z-Segment with Components

```rust
z_segment! {
    ZDX,
    id = "ZDX",
    fields = {
        1 => diagnosis_code: CodeableConcept,  // Custom type
        2 => diagnosis_type: String,
        3 => ranking: Option<u32>,
        4 => provider: Option<XcnField>,  // HL7 composite type
    }
}

// With custom type support
impl FromSegmentField for CodeableConcept {
    fn from_field(field: &Field) -> Result<Self> {
        // Parse components
    }
}
```

### 6. Integration with Parser

The parser will be updated to check the registry when encountering unknown segments:

```rust
// In vyna-hl7-parser/src/lib.rs

fn parse_segment(segment_str: &str) -> Result<Segment> {
    let segment = parse_segment_generic(segment_str)?;

    // Check if it's a registered custom segment
    if let Some(factory) = CustomSegmentRegistry::global().get(&segment.id) {
        // Custom segment - additional validation possible
        // but still return generic Segment for backward compatibility
    }

    Ok(segment)
}
```

### 7. Builder Pattern

Each Z-segment gets a fluent builder:

```rust
impl ZPV {
    pub fn builder() -> ZPVBuilder {
        ZPVBuilder::new()
    }
}

pub struct ZPVBuilder {
    visit_type: Option<String>,
    visit_number: Option<String>,
    patient_class: Option<String>,
    admit_datetime: Option<String>,
}

impl ZPVBuilder {
    pub fn visit_type(mut self, value: impl Into<String>) -> Self {
        self.visit_type = Some(value.into());
        self
    }

    pub fn build(self) -> Result<ZPV> {
        Ok(ZPV {
            visit_type: self.visit_type
                .ok_or_else(|| Error::missing_field("ZPV-1", "ZPV"))?,
            visit_number: self.visit_number
                .ok_or_else(|| Error::missing_field("ZPV-2", "ZPV"))?,
            patient_class: self.patient_class,
            admit_datetime: self.admit_datetime,
        })
    }
}
```

### 8. Field Definitions for Documentation

```rust
impl ZPV {
    pub fn field_definitions() -> Vec<FieldDefinition> {
        vec![
            FieldDefinition {
                number: 1,
                name: "Visit Type",
                data_type: "ST",
                required: true,
                repeatable: false,
                max_length: Some(20),
                description: "Type of patient visit (INPATIENT, OUTPATIENT, EMERGENCY)",
            },
            FieldDefinition {
                number: 2,
                name: "Visit Number",
                data_type: "ST",
                required: true,
                repeatable: false,
                max_length: Some(50),
                description: "Unique identifier for the visit",
            },
            // ...
        ]
    }
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (Week 1-2)
- [ ] Create `crates/vyna-hl7-custom` crate
- [ ] Implement `CustomSegment` trait
- [ ] Implement `CustomSegmentRegistry`
- [ ] Add basic tests

### Phase 2: Macro System (Week 2-3)
- [ ] Implement `z_segment!` macro in `vyna-hl7-macros`
- [ ] Generate struct definitions
- [ ] Generate `CustomSegment` implementations
- [ ] Generate builder pattern
- [ ] Add macro tests

### Phase 3: Parser Integration (Week 3)
- [ ] Update parser to check registry
- [ ] Add validation hooks
- [ ] Ensure backward compatibility
- [ ] Add integration tests

### Phase 4: Examples & Documentation (Week 4)
- [ ] Create examples for common Z-segments
- [ ] Add comprehensive documentation
- [ ] Write migration guide
- [ ] Create tutorial

## Testing Strategy

1. **Unit Tests**: Each component tested independently
2. **Integration Tests**: Full parsing workflow with Z-segments
3. **Real-world Tests**: Test with actual Z-segments from healthcare systems
4. **Performance Tests**: Ensure zero overhead for standard segments
5. **Backward Compatibility Tests**: Existing code still works

## Performance Considerations

- **Zero overhead for standard segments**: Registry lookup only for unknown segments
- **Lazy initialization**: Registry populated on first use
- **Compile-time optimization**: Macro expansion happens at compile time
- **Cache-friendly**: Registry uses HashMap for O(1) lookups

## Backward Compatibility

- Existing code that parses Z-segments as generic `Segment` continues to work
- No breaking changes to public API
- Registration is optional - unregistered Z-segments parse as generic segments

## Documentation Requirements

1. **API Documentation**: Comprehensive rustdoc for all public APIs
2. **User Guide**: How to define and use custom Z-segments
3. **Examples**: At least 5 common Z-segment patterns
4. **Migration Guide**: For users currently using generic segments
5. **Best Practices**: Recommendations for Z-segment design

## Alternatives Considered

### Alternative 1: Runtime String-based Definition
```rust
CustomSegmentRegistry::register_from_spec(
    "ZPV",
    vec![
        ("visit_type", DataType::String, true),
        ("visit_number", DataType::String, true),
    ]
);
```
**Rejected**: No compile-time safety, harder to use

### Alternative 2: Code Generation from XML
```rust
// Generate from conformance profiles
zpv_segment_from_profile("zpv_profile.xml");
```
**Deferred**: Can be added later as optional feature

### Alternative 3: Attribute Macro
```rust
#[z_segment(id = "ZPV")]
struct ZPV {
    #[field(1)]
    visit_type: String,
}
```
**Considered**: Could be added as alternative syntax

## Open Questions

1. **Namespace Collision**: How to handle multiple definitions of same Z-segment ID?
   - **Resolution**: Last registration wins, log warning

2. **Dynamic Loading**: Support loading Z-segments from external libraries?
   - **Resolution**: Defer to v0.9.0, not critical for v0.8.0

3. **Validation Strictness**: Fail fast or collect all errors?
   - **Resolution**: Collect all errors, return list

## Future Enhancements (Post v0.8.0)

1. **Code Generation from Profiles**: Generate Z-segments from conformance profiles
2. **Z-Segment Libraries**: Shareable crates of common Z-segments
3. **Dynamic Loading**: Load Z-segments from plugins
4. **IDE Support**: Auto-completion for registered Z-segments
5. **GraphQL/REST**: Expose Z-segments in API schemas

## Success Criteria

Version 0.8.0 is successful if:

- [ ] Users can define custom Z-segments with 10 lines of code or less
- [ ] Z-segments have same developer experience as standard segments
- [ ] Zero performance overhead for standard segments
- [ ] At least 5 real-world Z-segment examples work correctly
- [ ] Documentation is comprehensive and clear
- [ ] Backward compatibility is maintained

## References

- [HL7 v2.x Z-Segment Specification](http://www.hl7.org/)
- [HAPI CustomModelClassFactory](https://hapifhir.github.io/hapi-hl7v2/)
- [NHapi Custom Segments](https://github.com/nHapiNET/nHapi)
- Rust macro system documentation

## Appendix: Complete Example

```rust
// define_custom_segments.rs

use vyna_hl7_custom::z_segment;

// Custom visit segment
z_segment! {
    ZPV,
    id = "ZPV",
    fields = {
        1 => visit_type: String,
        2 => visit_number: String,
        3 => patient_class: Option<String>,
        4 => admit_datetime: Option<String>,
    }
}

// Custom billing segment
z_segment! {
    ZCU,
    id = "ZCU",
    fields = {
        1 => customer_id: String,
        2 => account_number: String,
        3 => balance: Option<f64>,
        4 => last_payment_date: Option<String>,
    },
    validate = |s: &ZCU| {
        if let Some(balance) = s.balance {
            if balance < 0.0 {
                return Err(ValidationError::custom("Balance cannot be negative"));
            }
        }
        Ok(())
    }
}

// main.rs

use vyna_hl7_parser::parse_message;
use vyna_hl7_custom::CustomSegmentRegistry;

fn main() -> Result<()> {
    // Register custom segments at startup
    CustomSegmentRegistry::global()
        .register::<ZPV>()
        .register::<ZCU>();

    // Parse message with Z-segments
    let hl7 = "MSH|^~\\&|...\r\
               PID|1||...\r\
               ZPV|OUTPATIENT|V12345|E|20250119120000\r\
               ZCU|CUST001|ACC123|1500.00|20250115";

    let message = parse_message(hl7)?;

    // Access Z-segments
    if let Some(zpv_seg) = message.get_segments_by_id("ZPV").first() {
        let zpv = ZPV::from_segment(zpv_seg)?;
        println!("Visit: {} - {}", zpv.visit_type, zpv.visit_number);
    }

    // Create new Z-segment
    let new_zpv = ZPV::builder()
        .visit_type("EMERGENCY")
        .visit_number("V99999")
        .patient_class("I")
        .build()?;

    println!("New visit segment: {}", new_zpv.to_segment().encode(&Delimiters::default()));

    Ok(())
}
```

## Implementation notes (deviations from the proposal)

The shipped `vyna-hl7-custom` crate follows this design with small
signature evolutions:

- `register` takes `&self` (interior mutability) and returns
  `Result<&Self>` instead of `&mut self` returning nothing.
- `get` returns `Option<FactoryInfo>` rather than
  `Option<&dyn CustomSegmentFactory>`.
- `registered_ids` returns `Vec<String>` rather than `Vec<&str>`.
- `CustomSegment` gained a `type_name()` helper with a default impl.
- `validate` returns the crate `Result<()>` alias.

## Approval

Implemented as specified above (see Implementation notes for minor
deviations).
