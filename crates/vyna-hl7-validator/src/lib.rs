//! Validation for HL7 messages
//!
//! This crate provides validation functionality for HL7 v2.x messages
//! against their respective standards (v2.1 through v2.8).
//!
//! ## Supported Message Schemas
//!
//! The validator embeds versioned JSON schemas for ADT (A01-A13, A17,
//! A28, A31, A34, A40), SIU (S12-S15), MDM (T01, T02, T04), DFT (P03,
//! P11), QRY (A19, Q01, Q02), BAR (P01, P02), pharmacy (RDE, RDS, RRD,
//! RGV, RAS, RRA, OMP), laboratory (OML, OUL), ORU (R01), ORM (O01),
//! ACK, MFN (M01), VXU (V04), and PPR (PC1). Coverage varies by
//! version: DFT (P11) and the pharmacy/laboratory order triggers
//! postdate v2.3, while QRY, ORM, MFN (M01), and OUL (R21) were
//! withdrawn in v2.7+. Use [`list_available_schemas`] for the exact
//! per-version list.
//!
//! Message types introduced after 2.2 (VXU, PPR, ...) have no v2.1/v2.2
//! definition and validate against the v2.3 structure; see [`load_schema`].
//!
//! ## Data Type Validation
//!
//! The validator supports format validation for HL7 data types including:
//! - Date/Time types (DT, TM, DTM, TS)
//! - Numeric types (NM, SI)
//! - String types (ST, TX, FT)
//! - Identifiers (ID) and message/processing types (MSG, PT)
//! - Composite values (CE, CWE, CNE, IS, XPN, XAD, XTN, CX, XCN, EI,
//!   HD, PL, VID, CP, CQ, DR, TQ, XON, MO, FC, SN), accepted as
//!   scalars here while schema component definitions enforce their
//!   component structure from the parsed field
//!
//! ## Vocabulary Validation
//!
//! The validator includes support for HL7 standard tables including:
//! - Table 0001: Administrative Sex
//! - Table 0004: Patient Class
//! - Table 0103: Processing ID
//! - Table 0085: Observation Result Status
//! - And many more standard HL7 tables
//!
//! ## Strict by default
//!
//! All constructors return a strict validator: every finding class is an
//! error, including MSH-12 version drift, so a clean result means the message
//! complies with the HL7 specification for the validated version. Parsing is
//! unaffected — the parser never validates, and validation never blocks
//! parsing: other tools (parse, extract, convert) run without validation.
//!
//! Strictness has two documented escape hatches, both explicit:
//! - [`ValidationContext`]: downgrade finding classes (e.g. `standard()` lets
//!   version drift warn, `lenient()` warns on everything but structure).
//! - [`Validator::table_registry_mut`]: extend the standard code tables with
//!   site-local codes (e.g. NDC drug codes alongside CVX in table 0292).

pub mod context;
pub mod datatype;
pub mod rules;
pub mod schema_loader;
pub mod vocabulary;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vyna_hl7_core::{
    Version, charset::CharacterSet, error::Result, message::Message, segment::Segment,
    types::DataType,
};

pub use context::{Severity, ValidationContext};
pub use datatype::{DataTypeValidation, validate_data_type};
pub use rules::{
    BuiltinRules, ConditionConfig, CrossFieldValidator, DeclarativeError, RuleConfig,
    RuleDefinition, RuleSeverity, RuleViolation, RulesEngine, RulesValidationResult,
    ValidationRule,
};
pub use schema_loader::{list_available_schemas, load_schema};
pub use vocabulary::{Hl7Table, TableRegistry, VocabularyValidation};

/// Validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    /// False once any error is recorded; warnings alone keep this true
    pub is_valid: bool,
    /// Findings that fail validation
    pub errors: Vec<ValidationError>,
    /// Non-failing findings
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: ValidationError) {
        self.is_valid = false;
        self.errors.push(error);
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.is_valid
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Terser-style location of the finding (e.g. `PID-5`)
    pub location: String,
    /// Human-readable description of the finding
    pub message: String,
    /// Machine-readable class of the finding
    pub error_type: ValidationErrorType,
}

impl ValidationError {
    /// Record a validation error at `location` with the given class
    pub fn new(location: String, message: String, error_type: ValidationErrorType) -> Self {
        Self {
            location,
            message,
            error_type,
        }
    }
}

/// Type of validation error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationErrorType {
    /// A required segment, field, or component is absent or empty
    MissingRequiredField,
    /// A value does not match its HL7 data type format
    InvalidDataType,
    /// A value exceeds its schema `max_length`
    InvalidLength,
    /// Reserved for schema cardinality violations (wrong repetition counts)
    InvalidCardinality,
    /// A vocabulary, coded-value, character-set, or version-drift problem
    InvalidValue,
    /// The message shape itself is wrong (empty, first segment not MSH)
    StructuralError,
}

/// Validation warning
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationWarning {
    /// Terser-style location of the finding (e.g. `PID-5`)
    pub location: String,
    /// Human-readable description of the finding
    pub message: String,
}

impl ValidationWarning {
    /// Record a non-failing validation finding at `location`
    pub fn new(location: String, message: String) -> Self {
        Self { location, message }
    }
}

/// HL7 message validator, strict by default (see crate docs)
pub struct Validator {
    version: Version,
    schema: Option<MessageSchema>,
    table_registry: TableRegistry,
    rules_engine: Option<RulesEngine>,
    context: ValidationContext,
}

impl Validator {
    /// Create a new validator for the given version
    pub fn new(version: Version) -> Self {
        Self {
            version,
            schema: None,
            table_registry: TableRegistry::new(),
            rules_engine: None,
            context: ValidationContext::strict(),
        }
    }

    /// Create a validator with a custom schema
    pub fn with_schema(version: Version, schema: MessageSchema) -> Self {
        Self {
            version,
            schema: Some(schema),
            table_registry: TableRegistry::new(),
            rules_engine: None,
            context: ValidationContext::strict(),
        }
    }

    /// Create a validator with auto-loaded schema for a specific message type
    pub fn for_message_type(
        version: Version,
        message_type: &str,
        trigger_event: &str,
    ) -> Result<Self> {
        let schema = load_schema(version, message_type, trigger_event)?;
        Ok(Self {
            version,
            schema: Some(schema),
            table_registry: TableRegistry::new(),
            rules_engine: None,
            context: ValidationContext::strict(),
        })
    }

    /// Set the severity context (see [`ValidationContext`]).
    pub fn with_context(mut self, context: ValidationContext) -> Self {
        self.context = context;
        self
    }

    /// Replace the severity context on a borrowed validator.
    pub fn set_context(&mut self, context: ValidationContext) {
        self.context = context;
    }

    /// The active severity context.
    pub fn context(&self) -> &ValidationContext {
        &self.context
    }

    /// Get a reference to the table registry
    pub fn table_registry(&self) -> &TableRegistry {
        &self.table_registry
    }

    /// Get a mutable reference to the table registry (for adding custom tables)
    ///
    /// Strict validation enforces the standard code tables, which real-world
    /// feeds sometimes extend: HL7 v2.5 tables RXE-2/RXA-5 to 0292 (CVX
    /// vaccines), while pharmacy systems routinely send NDC drug codes. Extend
    /// the standard table with site-local codes instead of weakening the
    /// validator — standard values keep validating, and the extension is
    /// explicit at the call site:
    ///
    /// ```rust
    /// use vyna_hl7_core::Version;
    /// use vyna_hl7_validator::Validator;
    ///
    /// let mut validator = Validator::for_message_type(Version::V2_5, "RDE", "O11")?;
    /// validator
    ///     .table_registry_mut()
    ///     .get_table_mut("0292")
    ///     .unwrap()
    ///     .add_value("00093-0058", "Lorazepam 1mg tablet (site NDC)", false);
    ///
    /// // Standard CVX codes still validate alongside the site code.
    /// use vyna_hl7_validator::VocabularyValidation;
    /// assert!(matches!(
    ///     validator.table_registry().validate("0292", "08"),
    ///     VocabularyValidation::Valid
    /// ));
    /// assert!(matches!(
    ///     validator.table_registry().validate("0292", "00093-0058"),
    ///     VocabularyValidation::Valid
    /// ));
    /// # Ok::<(), vyna_hl7_core::Error>(())
    /// ```
    pub fn table_registry_mut(&mut self) -> &mut TableRegistry {
        &mut self.table_registry
    }

    /// Set the rules engine for business rules validation
    pub fn with_rules_engine(mut self, engine: RulesEngine) -> Self {
        self.rules_engine = Some(engine);
        self
    }

    /// Get a reference to the rules engine
    pub fn rules_engine(&self) -> Option<&RulesEngine> {
        self.rules_engine.as_ref()
    }

    /// Record a finding at the severity the context assigns to its class.
    fn emit(
        &self,
        result: &mut ValidationResult,
        location: String,
        message: String,
        error_type: ValidationErrorType,
    ) {
        let severity = match error_type {
            ValidationErrorType::MissingRequiredField => self.context.missing_required,
            ValidationErrorType::InvalidDataType => self.context.invalid_datatype,
            ValidationErrorType::InvalidLength => self.context.invalid_length,
            ValidationErrorType::InvalidCardinality => self.context.invalid_cardinality,
            ValidationErrorType::InvalidValue => self.context.invalid_value,
            ValidationErrorType::StructuralError => self.context.structural,
        };
        match severity {
            Severity::Error => {
                result.add_error(ValidationError::new(location, message, error_type))
            }
            Severity::Warning => result.add_warning(ValidationWarning::new(location, message)),
            Severity::Ignore => {}
        }
    }

    /// Record a version-drift finding at the context's version severity.
    fn emit_version_mismatch(
        &self,
        result: &mut ValidationResult,
        location: String,
        message: String,
    ) {
        match self.context.version_mismatch {
            Severity::Error => result.add_error(ValidationError::new(
                location,
                message,
                ValidationErrorType::InvalidValue,
            )),
            Severity::Warning => result.add_warning(ValidationWarning::new(location, message)),
            Severity::Ignore => {}
        }
    }

    /// Get a mutable reference to the rules engine
    pub fn rules_engine_mut(&mut self) -> &mut Option<RulesEngine> {
        &mut self.rules_engine
    }

    /// Add a validation rule to the rules engine
    /// If no rules engine exists, one will be created
    pub fn add_rule(&mut self, rule: ValidationRule) {
        if self.rules_engine.is_none() {
            self.rules_engine = Some(RulesEngine::new());
        }
        if let Some(engine) = &mut self.rules_engine {
            engine.add_rule(rule);
        }
    }

    /// Add multiple validation rules to the rules engine
    /// If no rules engine exists, one will be created
    pub fn add_rules(&mut self, rules: Vec<ValidationRule>) {
        if self.rules_engine.is_none() {
            self.rules_engine = Some(RulesEngine::new());
        }
        if let Some(engine) = &mut self.rules_engine {
            engine.add_rules(rules);
        }
    }

    /// Validate a message
    ///
    /// When a schema is set, segments absent from it are flagged as
    /// warnings in every mode: shipped schemas are minimal skeletons
    /// (they omit many legal segments), so absence from the schema
    /// means "not covered", never "invalid". Site-defined Z-segments
    /// always pass through silently.
    pub fn validate(&self, message: &Message) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Basic structure validation
        self.validate_structure(message, &mut result);

        // Character set declaration against content
        self.validate_character_set(message, &mut result);

        // Schema-based validation (if schema is available)
        if let Some(schema) = &self.schema {
            self.validate_against_schema(message, schema, &mut result);
        }

        // Business rules validation (if rules engine is available)
        if let Some(rules_engine) = &self.rules_engine {
            self.validate_business_rules(message, rules_engine, &mut result);
        }

        result
    }

    /// Validate basic message structure
    fn validate_structure(&self, message: &Message, result: &mut ValidationResult) {
        // Must have at least one segment (MSH)
        if message.segments.is_empty() {
            self.emit(
                result,
                "Message".to_string(),
                "Message must contain at least one segment".to_string(),
                ValidationErrorType::StructuralError,
            );
            return;
        }

        // First segment must be MSH
        if message.segments[0].id != "MSH" {
            self.emit(
                result,
                "Segment[0]".to_string(),
                "First segment must be MSH".to_string(),
                ValidationErrorType::StructuralError,
            );
        }

        // Validate segment IDs
        for (i, segment) in message.segments.iter().enumerate() {
            if let Err(e) = segment.validate_id() {
                self.emit(
                    result,
                    format!("Segment[{}]", i),
                    e.to_string(),
                    ValidationErrorType::StructuralError,
                );
            }
        }

        // An MSH-12 that names no known version is not "no drift" but
        // an unparsable value, and fails strict validation. An absent or
        // empty MSH-12 stays silent here (schemas own required-ness).
        if let Some(msh) = message.get_msh()
            && let Some(raw) = msh.get_field_value(12)
            && !raw.is_empty()
            && message.get_version().is_none()
        {
            self.emit(
                result,
                "MSH-12".to_string(),
                format!("Message version '{raw}' is not a supported HL7 version"),
                ValidationErrorType::InvalidValue,
            );
        }

        // Check version matches
        if let Some(msg_version) = message.get_version()
            && msg_version != self.version
        {
            self.emit_version_mismatch(
                result,
                "MSH-12".to_string(),
                format!(
                    "Message version ({}) differs from validator version ({})",
                    msg_version.as_str(),
                    self.version.as_str()
                ),
            );
        }
    }

    /// Validate the MSH-18 character set declaration against the content
    ///
    /// An unknown declaration is an error, as is any character the declared
    /// set cannot represent. An empty MSH-18 means UTF-8, which covers
    /// every character, so undeclared messages are never flagged here.
    fn validate_character_set(&self, message: &Message, result: &mut ValidationResult) {
        let Some(msh) = message.get_msh() else {
            return; // structure validation already reported the missing MSH
        };
        let declared = msh.get_field_value(18).unwrap_or("");
        if declared.is_empty() {
            return;
        }
        let charset = match CharacterSet::from_msh18(declared) {
            Ok(charset) => charset,
            Err(e) => {
                self.emit(
                    result,
                    "MSH-18".to_string(),
                    e.to_string(),
                    ValidationErrorType::InvalidValue,
                );
                return;
            }
        };
        if charset == CharacterSet::Utf8 {
            return;
        }
        for segment in &message.segments {
            for (field_index, field) in segment.fields.iter().enumerate() {
                for repetition in &field.repetitions {
                    for component in &repetition.components {
                        for subcomponent in &component.subcomponents {
                            if let Some(offender) = charset.find_unmappable(&subcomponent.value) {
                                self.emit(
                                    result,
                                    format!("{}-{}", segment.id, field_index + 1),
                                    format!(
                                        "character U+{:04X} cannot be represented in \
                                         the declared character set {}",
                                        offender as u32,
                                        charset.msh18()
                                    ),
                                    ValidationErrorType::InvalidValue,
                                );
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Validate against schema
    ///
    /// Segments absent from the schema warn in every mode (shipped
    /// schemas omit many legal segments, so absence means "not
    /// covered", never "invalid"), except site-defined Z-segments,
    /// which always pass through silently.
    fn validate_against_schema(
        &self,
        message: &Message,
        schema: &MessageSchema,
        result: &mut ValidationResult,
    ) {
        // Validate each segment against schema
        for (i, segment) in message.segments.iter().enumerate() {
            if let Some(seg_def) = schema.segments.get(&segment.id) {
                self.validate_segment(segment, seg_def, i, &message.delimiters, result);
            } else if !segment.id.starts_with('Z') {
                let schema_name = if schema.trigger_event.is_empty() {
                    schema.message_type.clone()
                } else {
                    format!("{}_{}", schema.message_type, schema.trigger_event)
                };
                result.add_warning(ValidationWarning::new(
                    format!("{}[{}]", segment.id, i),
                    format!(
                        "Segment {} is not covered by the {} schema; skipped",
                        segment.id, schema_name
                    ),
                ));
            }
        }

        // Check for required segments and non-repeating cardinality
        for (seg_id, seg_def) in &schema.segments {
            let count = message.segments.iter().filter(|s| &s.id == seg_id).count();
            if seg_def.required && count == 0 {
                self.emit(
                    result,
                    "Message".to_string(),
                    format!("Required segment {} is missing", seg_id),
                    ValidationErrorType::MissingRequiredField,
                );
            }
            if !seg_def.repeating && count > 1 {
                self.emit(
                    result,
                    seg_id.clone(),
                    format!(
                        "Segment {} occurs {} times but is not repeating",
                        seg_id, count
                    ),
                    ValidationErrorType::InvalidCardinality,
                );
            }
        }
    }

    /// Validate a segment
    fn validate_segment(
        &self,
        segment: &Segment,
        definition: &SegmentDefinition,
        index: usize,
        delimiters: &vyna_hl7_core::delimiters::Delimiters,
        result: &mut ValidationResult,
    ) {
        let location_prefix = format!("{}[{}]", segment.id, index);

        // Validate each field
        for (field_idx, field_def) in &definition.fields {
            let field = segment.get_field(*field_idx);

            if field_def.required && (field.is_none() || field.unwrap().is_empty()) {
                self.emit(
                    result,
                    format!("{}-{}", location_prefix, field_idx),
                    format!("Required field {} is missing or empty", field_idx),
                    ValidationErrorType::MissingRequiredField,
                );
            }

            // Validate field if it exists
            if let Some(f) = field {
                let field_location = format!("{}-{}", location_prefix, field_idx);

                // Non-repeating fields must not carry multiple values.
                // Empty repetitions mean absent, so only count content.
                if !field_def.repeating {
                    let rep_count = f.repetitions.iter().filter(|r| !r.is_empty()).count();
                    if rep_count > 1 {
                        self.emit(
                            result,
                            field_location.clone(),
                            format!(
                                "Field {} repeats {} times but is not repeating",
                                field_idx, rep_count
                            ),
                            ValidationErrorType::InvalidCardinality,
                        );
                    }
                }

                // Validate max length
                // For repeating fields, check the encoded length (with all repetitions)
                // For non-repeating fields, check the trimmed value length
                if let Some(max_len) = field_def.max_length {
                    // HL7 lengths are characters, not bytes.
                    let field_length = if field_def.repeating {
                        // For repeating fields, encode to get full length including separators
                        f.encode(delimiters).chars().count()
                    } else {
                        // For non-repeating fields, use the first repetition value (trimmed)
                        f.value().map(|v| v.trim().chars().count()).unwrap_or(0)
                    };

                    if field_length > max_len {
                        self.emit(
                            result,
                            field_location.clone(),
                            format!(
                                "Field exceeds maximum length ({} > {})",
                                field_length, max_len
                            ),
                            ValidationErrorType::InvalidLength,
                        );
                    }
                }

                // Validate data type format and vocabulary on every
                // repetition: first-repetition-only checks miss bad later
                // repetitions (e.g. MSH-18 "ASCII~!!!").
                for (rep_idx, repetition) in f.repetitions.iter().enumerate() {
                    if repetition.is_empty() {
                        continue;
                    }
                    let Some(value) = repetition.value() else {
                        continue;
                    };
                    if let Some(data_type) = DataType::from_str(&field_def.data_type) {
                        let validation = datatype::validate_data_type(value, data_type);
                        if !validation.is_valid() {
                            self.emit(
                                result,
                                field_location.clone(),
                                format!(
                                    "Invalid {} format: {}",
                                    field_def.data_type,
                                    validation.error_message().unwrap_or("unknown error")
                                ),
                                ValidationErrorType::InvalidDataType,
                            );
                        }
                    }

                    // Validate vocabulary/code set
                    if let Some(table_id) = &field_def.table_id
                        && let Some(code) = field_table_code(f, rep_idx, field_def)
                    {
                        let vocab_validation = self.table_registry.validate(table_id, code);
                        if !vocab_validation.is_valid()
                            && let Some(err_msg) = vocab_validation.error_message()
                        {
                            self.emit(
                                result,
                                field_location.clone(),
                                err_msg.to_string(),
                                ValidationErrorType::InvalidValue,
                            );
                        }
                    }
                }

                // Validate components if the schema defines them
                if let Some(components) = &field_def.components {
                    self.validate_components(f, components, &field_location, result);
                }
            }
        }
    }

    /// Validate components within a field against their definitions
    fn validate_components(
        &self,
        field: &vyna_hl7_core::field::Field,
        components: &HashMap<String, ComponentDefinition>,
        field_location: &str,
        result: &mut ValidationResult,
    ) {
        for repetition in &field.repetitions {
            // Empty means absent in HL7: skip required-component checks for
            // repetitions with no content (e.g. empty wire positions, which
            // the parser materializes as one empty repetition).
            if repetition.is_empty() {
                continue;
            }
            for (comp_key, comp_def) in components {
                let comp_idx: usize = match comp_key.parse() {
                    Ok(idx) => idx,
                    Err(_) => continue,
                };
                // Components are 1-indexed in the schema; key "0" is
                // malformed. Skip it like unparsable keys instead of
                // underflowing on `comp_idx - 1` below.
                if comp_idx == 0 {
                    continue;
                }
                // Components are 0-indexed in the repetition vec, 1-indexed in schema
                let component = repetition.get_component(comp_idx - 1);
                let comp_location = format!("{}-{}", field_location, comp_idx);

                // Check required components
                let comp_empty = component.is_none() || component.unwrap().is_empty();
                if comp_def.required && comp_empty {
                    self.emit(
                        result,
                        comp_location.clone(),
                        format!(
                            "Required component {} ({}) is missing or empty",
                            comp_idx, comp_def.name
                        ),
                        ValidationErrorType::MissingRequiredField,
                    );
                }

                if let Some(comp) = component {
                    if comp.is_empty() {
                        continue;
                    }

                    let comp_value = comp.value().unwrap_or("");

                    // Validate component max length (characters, not bytes)
                    let comp_len = comp_value.chars().count();
                    if let Some(max_len) = comp_def.max_length
                        && comp_len > max_len
                    {
                        self.emit(
                            result,
                            comp_location.clone(),
                            format!(
                                "Component {} ({}) exceeds maximum length ({} > {})",
                                comp_idx, comp_def.name, comp_len, max_len
                            ),
                            ValidationErrorType::InvalidLength,
                        );
                    }

                    // Validate component data type
                    if let Some(data_type) = DataType::from_str(&comp_def.data_type) {
                        let validation = datatype::validate_data_type(comp_value, data_type);
                        if !validation.is_valid() {
                            self.emit(
                                result,
                                comp_location.clone(),
                                format!(
                                    "Invalid {} format in component {} ({}): {}",
                                    comp_def.data_type,
                                    comp_idx,
                                    comp_def.name,
                                    validation.error_message().unwrap_or("unknown error")
                                ),
                                ValidationErrorType::InvalidDataType,
                            );
                        }
                    }

                    // Validate component vocabulary/code set
                    if let Some(table_id) = &comp_def.table_id {
                        let vocab_validation = self.table_registry.validate(table_id, comp_value);
                        if !vocab_validation.is_valid()
                            && let Some(err_msg) = vocab_validation.error_message()
                        {
                            self.emit(
                                result,
                                comp_location.clone(),
                                err_msg.to_string(),
                                ValidationErrorType::InvalidValue,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Validate business rules using the rules engine
    fn validate_business_rules(
        &self,
        message: &Message,
        rules_engine: &RulesEngine,
        result: &mut ValidationResult,
    ) {
        let rules_result = rules_engine.validate(message);

        // Convert rule violations to validation errors/warnings.
        // Rule violations keep their declared severity regardless of
        // the context (see ValidationContext docs): a rule Error is
        // always an error, even under a lenient context.
        for violation in rules_result.violations {
            match violation.severity {
                RuleSeverity::Error => {
                    result.add_error(ValidationError::new(
                        violation.location.unwrap_or_else(|| "Message".to_string()),
                        violation.message,
                        ValidationErrorType::InvalidValue,
                    ));
                }
                RuleSeverity::Warning | RuleSeverity::Info => {
                    result.add_warning(ValidationWarning::new(
                        violation.location.unwrap_or_else(|| "Message".to_string()),
                        violation.message,
                    ));
                }
            }
        }
    }
}

/// Code a field-level table check applies to.
///
/// For composite fields the table constrains component 1 (the code), read
/// literally from the parsed structure: `value()` already returns the first
/// component, so split fields check just the code. A single unsplit component
/// (e.g. via `Field::from_value("ZZ^Not a vaccine^CVX")`) is one literal value
/// and is checked whole, exactly like the length, datatype, and required
/// checks treat it; composite values must use `Field::from_delimited`.
///
/// When the schema assigns the same table to a component other than 1 (e.g.
/// XPN name-type table 0200 on component 7), the component-level checks own
/// it and there is no field-level code to check.
///
/// Exception: on CQ quantity/units fields a field-level table constrains the
/// units (component 2), never the numeric quantity in component 1.
///
/// `rep_index` selects which repetition the code is read from, so
/// field-level table checks cover every repetition.
fn field_table_code<'a>(
    field: &'a vyna_hl7_core::field::Field,
    rep_index: usize,
    field_def: &FieldDefinition,
) -> Option<&'a str> {
    let value = field.get_repetition(rep_index)?.value()?;
    if field_def.components.is_some() || is_composite_datatype(&field_def.data_type) {
        if let Some(table_id) = &field_def.table_id
            && let Some(components) = &field_def.components
            && components.iter().any(|(key, comp)| {
                comp.table_id.as_ref() == Some(table_id)
                    && key.parse::<usize>().is_ok_and(|idx| idx != 1)
            })
        {
            return None;
        }
        if field_def.data_type == "CQ" {
            return field.get_component(rep_index, 1).and_then(|c| c.value());
        }
        Some(value)
    } else {
        Some(value)
    }
}

/// HL7 datatypes with components (field-level tables constrain component 1).
fn is_composite_datatype(data_type: &str) -> bool {
    matches!(
        data_type,
        "AD" | "AUI"
            | "CCD"
            | "CCP"
            | "CD"
            | "CE"
            | "CF"
            | "CNE"
            | "CNN"
            | "CP"
            | "CQ"
            | "CSU"
            | "CWE"
            | "CX"
            | "DDI"
            | "DIN"
            | "DLN"
            | "DLT"
            | "DR"
            | "DTN"
            | "EI"
            | "EIP"
            | "ELD"
            | "ERL"
            | "FC"
            | "FN"
            | "HD"
            | "JCC"
            | "LA1"
            | "LA2"
            | "MA"
            | "MOC"
            | "MO"
            | "MSG"
            | "NA"
            | "NDL"
            | "NR"
            | "OCD"
            | "OSP"
            | "PIP"
            | "PL"
            | "PPN"
            | "PRL"
            | "PT"
            | "QIP"
            | "QSC"
            | "RCD"
            | "RFR"
            | "RI"
            | "RMC"
            | "RP"
            | "SAD"
            | "SCV"
            | "SN"
            | "SPD"
            | "TQ"
            | "UVC"
            | "VH"
            | "VID"
            | "VR"
            | "XAD"
            | "XCN"
            | "XON"
            | "XPN"
            | "XTN"
    )
}

/// Message schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSchema {
    /// HL7 message type (MSH-9.1, e.g. `ADT`)
    pub message_type: String,
    /// Trigger event (MSH-9.2, e.g. `A01`; empty for trigger-less types like `ACK`)
    pub trigger_event: String,
    /// HL7 version this structure was written against (e.g. `2.5`)
    pub version: String,
    /// Segment definitions keyed by segment id (e.g. `PID`)
    pub segments: HashMap<String, SegmentDefinition>,
}

/// Segment definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentDefinition {
    /// Human-readable segment name (e.g. `Patient Identification`)
    pub name: String,
    /// Whether the message must contain this segment
    pub required: bool,
    /// Whether the segment may repeat
    pub repeating: bool,
    /// Field definitions keyed by 1-based field number
    pub fields: HashMap<usize, FieldDefinition>,
}

/// Field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Human-readable field name (e.g. `Patient Name`)
    pub name: String,
    /// HL7 data type code (e.g. `XPN`, `NM`, `DTM`)
    pub data_type: String,
    /// Whether the field must carry a value
    pub required: bool,
    /// Whether the field may repeat (`~`-separated values)
    pub repeating: bool,
    /// Maximum value length, when the standard sets one
    pub max_length: Option<usize>,
    /// HL7 table id for coded values, when one applies (e.g. `0001`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
    /// Component definitions for composite types, keyed by 1-based position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<HashMap<String, ComponentDefinition>>,
}

/// Component definition for composite data types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDefinition {
    /// Human-readable component name (e.g. `Family Name`)
    pub name: String,
    /// HL7 data type code (e.g. `FN`, `ST`)
    pub data_type: String,
    /// Whether the component must carry a value
    #[serde(default)]
    pub required: bool,
    /// Maximum value length, when the standard sets one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// HL7 table id for coded values, when one applies (e.g. `0001`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_core::{field::Field, segment::Segment};

    fn create_test_message() -> Message {
        let mut msg = Message::new();

        let mut msh = Segment::new("MSH");
        msh.add_field(Field::from_value("^~\\&"));
        msh.add_field(Field::from_value(""));
        msh.add_field(Field::from_value("SendApp"));
        msg.add_segment(msh);

        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        msg.add_segment(pid);

        msg
    }

    #[test]
    fn test_validate_valid_message() {
        let msg = create_test_message();
        let validator = Validator::new(Version::V2_5);
        let result = validator.validate(&msg);

        assert!(result.is_valid());
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_validate_empty_message() {
        let msg = Message::new();
        let validator = Validator::new(Version::V2_5);
        let result = validator.validate(&msg);

        assert!(!result.is_valid());
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_validate_missing_msh() {
        let mut msg = Message::new();
        msg.add_segment(Segment::new("PID"));

        let validator = Validator::new(Version::V2_5);
        let result = validator.validate(&msg);

        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.message.contains("MSH")));
    }

    #[test]
    fn test_component_key_zero_does_not_panic() {
        let mut components = HashMap::new();
        components.insert(
            "0".to_string(),
            ComponentDefinition {
                name: "Bogus".to_string(),
                data_type: "ST".to_string(),
                required: false,
                max_length: None,
                table_id: None,
            },
        );
        let mut fields = HashMap::new();
        fields.insert(
            3,
            FieldDefinition {
                name: "Patient Identifier List".to_string(),
                data_type: "ST".to_string(),
                required: false,
                repeating: false,
                max_length: None,
                table_id: None,
                components: Some(components),
            },
        );
        let mut segments = HashMap::new();
        segments.insert(
            "MSH".to_string(),
            SegmentDefinition {
                name: "Message Header".to_string(),
                required: false,
                repeating: false,
                fields: HashMap::new(),
            },
        );
        segments.insert(
            "PID".to_string(),
            SegmentDefinition {
                name: "Patient Identification".to_string(),
                required: false,
                repeating: false,
                fields,
            },
        );
        let schema = MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            version: "2.5".to_string(),
            segments,
        };

        let mut msg = Message::new();
        msg.add_segment(Segment::new("MSH"));
        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        pid.add_field(Field::from_value(""));
        pid.add_field(Field::from_value("12345"));
        msg.add_segment(pid);

        // Component keys are 1-based; key "0" must be skipped, not panic.
        let validator = Validator::with_schema(Version::V2_5, schema);
        let result = validator.validate(&msg);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_invalid_segment_id() {
        let mut msg = Message::new();
        msg.add_segment(Segment::new("MSH"));
        msg.add_segment(Segment::new("X")); // Invalid ID (too short)

        let validator = Validator::new(Version::V2_5);
        let result = validator.validate(&msg);

        assert!(!result.is_valid());
    }

    /// Minimal MSH+PID schema: anything else is out of schema.
    fn create_msh_pid_schema() -> MessageSchema {
        let mut segments = HashMap::new();
        for id in ["MSH", "PID"] {
            segments.insert(
                id.to_string(),
                SegmentDefinition {
                    name: id.to_string(),
                    required: false,
                    repeating: false,
                    fields: HashMap::new(),
                },
            );
        }
        MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            version: "2.5".to_string(),
            segments,
        }
    }

    #[test]
    fn test_out_of_schema_segment_warns_even_when_strict() {
        let mut msg = Message::new();
        msg.add_segment(Segment::new("MSH"));
        msg.add_segment(Segment::new("PID"));
        msg.add_segment(Segment::new("OBX")); // Well-formed but not in schema

        let validator = Validator::with_schema(Version::V2_5, create_msh_pid_schema());
        let result = validator.validate(&msg);

        // Schemas are minimal skeletons omitting many legal segments, so
        // absence warns ("not covered") instead of failing.
        assert!(result.is_valid(), "errors: {:?}", result.errors);
        assert!(
            result.warnings.iter().any(|w| w.message.contains("OBX")),
            "expected an out-of-schema warning for OBX, got {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_out_of_schema_segment_warns_in_lenient_mode() {
        let mut msg = Message::new();
        msg.add_segment(Segment::new("MSH"));
        msg.add_segment(Segment::new("PID"));
        msg.add_segment(Segment::new("OBX"));

        let validator = Validator::with_schema(Version::V2_5, create_msh_pid_schema())
            .with_context(ValidationContext::lenient());
        let result = validator.validate(&msg);

        assert!(result.is_valid());
        assert!(result.warnings.iter().any(|w| w.message.contains("OBX")));
    }

    #[test]
    fn test_z_segments_pass_through_schema_validation() {
        let mut msg = Message::new();
        msg.add_segment(Segment::new("MSH"));
        msg.add_segment(Segment::new("PID"));
        msg.add_segment(Segment::new("ZAB")); // Site-defined, never in schema

        let validator = Validator::with_schema(Version::V2_5, create_msh_pid_schema());
        let result = validator.validate(&msg);

        assert!(result.is_valid(), "errors: {:?}", result.errors);
        assert!(result.warnings.is_empty());
    }

    /// Helper to build a schema with component definitions for testing
    fn create_schema_with_components() -> MessageSchema {
        let mut components = HashMap::new();
        components.insert(
            "1".to_string(),
            ComponentDefinition {
                name: "ID Number".to_string(),
                data_type: "ST".to_string(),
                required: true,
                max_length: Some(15),
                table_id: None,
            },
        );
        components.insert(
            "2".to_string(),
            ComponentDefinition {
                name: "Check Digit".to_string(),
                data_type: "ST".to_string(),
                required: false,
                max_length: Some(1),
                table_id: None,
            },
        );
        components.insert(
            "3".to_string(),
            ComponentDefinition {
                name: "Check Digit Scheme".to_string(),
                data_type: "ID".to_string(),
                required: false,
                max_length: Some(3),
                table_id: Some("0061".to_string()),
            },
        );
        components.insert(
            "5".to_string(),
            ComponentDefinition {
                name: "Identifier Type Code".to_string(),
                data_type: "ID".to_string(),
                required: false,
                max_length: Some(5),
                table_id: Some("0203".to_string()),
            },
        );

        let mut fields = HashMap::new();
        fields.insert(
            3,
            FieldDefinition {
                name: "Patient Identifier List".to_string(),
                data_type: "CX".to_string(),
                required: true,
                repeating: true,
                max_length: Some(250),
                table_id: None,
                components: Some(components),
            },
        );

        let mut segments = HashMap::new();
        segments.insert(
            "PID".to_string(),
            SegmentDefinition {
                name: "Patient Identification".to_string(),
                required: true,
                repeating: false,
                fields,
            },
        );
        // Add minimal MSH to avoid unrelated errors
        segments.insert(
            "MSH".to_string(),
            SegmentDefinition {
                name: "Message Header".to_string(),
                required: true,
                repeating: false,
                fields: HashMap::new(),
            },
        );

        MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            version: "2.5".to_string(),
            segments,
        }
    }

    fn create_message_with_pid3(pid3_value: &str) -> Message {
        use vyna_hl7_core::field::{Component, Repetition, SubComponent};

        let mut msg = Message::new();

        let mut msh = Segment::new("MSH");
        msh.add_field(Field::from_value("^~\\&"));
        msg.add_segment(msh);

        let mut pid = Segment::new("PID");
        // PID-1 (Set ID)
        pid.add_field(Field::from_value("1"));
        // PID-2 (empty)
        pid.add_field(Field::from_value(""));
        // PID-3 (Patient Identifier List) - build from components
        let components: Vec<&str> = pid3_value.split('^').collect();
        let mut rep = Repetition::new();
        for comp_val in components {
            let mut comp = Component::new();
            comp.add_subcomponent(SubComponent::new(comp_val));
            rep.add_component(comp);
        }
        let mut field = Field::new();
        field.add_repetition(rep);
        pid.add_field(field);

        msg.add_segment(pid);
        msg
    }

    #[test]
    fn test_component_validation_required_present() {
        // PID-3 with required component 1 (ID Number) present
        let msg = create_message_with_pid3("12345^^^AUTH^MR");
        let schema = create_schema_with_components();
        let validator = Validator::with_schema(Version::V2_5, schema);
        let result = validator.validate(&msg);

        // No errors for required component being present
        let comp_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.location.contains("PID") && e.location.contains("-3-"))
            .collect();
        assert!(
            comp_errors.is_empty(),
            "Expected no component errors, got: {:?}",
            comp_errors
        );
    }

    #[test]
    fn test_component_validation_required_missing() {
        // PID-3 with component 1 (ID Number) empty - should error
        let msg = create_message_with_pid3("^^^AUTH^MR");
        let schema = create_schema_with_components();
        let validator = Validator::with_schema(Version::V2_5, schema);
        let result = validator.validate(&msg);

        let required_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| {
                e.location.contains("-3-1")
                    && e.error_type == ValidationErrorType::MissingRequiredField
            })
            .collect();
        assert_eq!(
            required_errors.len(),
            1,
            "Expected 1 required component error for PID-3-1, got: {:?}",
            required_errors
        );
        assert!(required_errors[0].message.contains("ID Number"));
    }

    #[test]
    fn test_component_validation_max_length() {
        // PID-3 component 1 max_length is 15, give it 20 chars
        let msg = create_message_with_pid3("12345678901234567890^^^AUTH^MR");
        let schema = create_schema_with_components();
        let validator = Validator::with_schema(Version::V2_5, schema);
        let result = validator.validate(&msg);

        let length_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| {
                e.location.contains("-3-1") && e.error_type == ValidationErrorType::InvalidLength
            })
            .collect();
        assert_eq!(
            length_errors.len(),
            1,
            "Expected 1 length error for PID-3-1, got: {:?}",
            length_errors
        );
    }

    #[test]
    fn test_component_validation_data_type() {
        // Use a schema with a DT-typed component, then provide invalid date
        let mut components = HashMap::new();
        components.insert(
            "1".to_string(),
            ComponentDefinition {
                name: "Effective Date".to_string(),
                data_type: "DT".to_string(),
                required: false,
                max_length: Some(8),
                table_id: None,
            },
        );

        let mut fields = HashMap::new();
        fields.insert(
            3,
            FieldDefinition {
                name: "Test Field".to_string(),
                data_type: "CX".to_string(),
                required: false,
                repeating: false,
                max_length: None,
                table_id: None,
                components: Some(components),
            },
        );

        let mut segments = HashMap::new();
        segments.insert(
            "PID".to_string(),
            SegmentDefinition {
                name: "Patient Identification".to_string(),
                required: false,
                repeating: false,
                fields,
            },
        );
        segments.insert(
            "MSH".to_string(),
            SegmentDefinition {
                name: "Message Header".to_string(),
                required: true,
                repeating: false,
                fields: HashMap::new(),
            },
        );

        let schema = MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            version: "2.5".to_string(),
            segments,
        };

        // "NOTADATE" is not a valid DT
        let msg = create_message_with_pid3("NOTADATE");
        let validator = Validator::with_schema(Version::V2_5, schema);
        let result = validator.validate(&msg);

        let dt_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| {
                e.location.contains("-3-1") && e.error_type == ValidationErrorType::InvalidDataType
            })
            .collect();
        assert_eq!(
            dt_errors.len(),
            1,
            "Expected 1 data type error for invalid date, got: {:?}",
            dt_errors
        );
    }

    #[test]
    fn test_component_validation_with_loaded_schema() {
        // Test with a real loaded schema (ADT^A01 v2.5) to verify
        // component validation works end-to-end with actual HL7 definitions
        let schema = schema_loader::load_schema(Version::V2_5, "ADT", "A01").unwrap();

        // Verify MSH-9 components (Message Code, Trigger Event, Message Structure)
        // are defined and required
        let msh_def = schema.segments.get("MSH").unwrap();
        let msh9 = msh_def.fields.get(&9).unwrap();
        let components = msh9.components.as_ref().unwrap();
        assert!(
            components.get("1").unwrap().required,
            "MSH-9-1 should be required"
        );
        assert!(
            components.get("2").unwrap().required,
            "MSH-9-2 should be required"
        );

        // Verify PID-3 components exist
        let pid_def = schema.segments.get("PID").unwrap();
        let pid3 = pid_def.fields.get(&3).unwrap();
        let pid3_comps = pid3.components.as_ref().unwrap();
        assert_eq!(pid3_comps.len(), 10, "PID-3 (CX) should have 10 components");
        assert!(
            pid3_comps.get("1").unwrap().required,
            "PID-3-1 (ID Number) should be required"
        );
        assert_eq!(pid3_comps.get("5").unwrap().data_type, "ID");
        assert_eq!(
            pid3_comps.get("5").unwrap().table_id.as_deref(),
            Some("0203"),
            "PID-3-5 should reference Table 0203"
        );
    }

    #[test]
    fn test_component_validation_optional_empty() {
        // All optional components empty should produce no errors
        let msg = create_message_with_pid3("12345");
        let schema = create_schema_with_components();
        let validator = Validator::with_schema(Version::V2_5, schema);
        let result = validator.validate(&msg);

        let comp_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.location.contains("-3-"))
            .collect();
        assert!(
            comp_errors.is_empty(),
            "Optional empty components should not error: {:?}",
            comp_errors
        );
    }

    fn message_with_charset(declaration: Option<&str>, patient_name: &str) -> Message {
        let mut msg = Message::new();
        let mut msh = Segment::new("MSH");
        for value in [
            "|",
            "^~\\&",
            "SendApp",
            "SendFac",
            "RecApp",
            "RecFac",
            "20240315120000",
            "",
            "ADT^A01",
            "MSG001",
            "P",
            "2.5",
        ] {
            msh.add_field(Field::from_value(value));
        }
        if let Some(charset) = declaration {
            for _ in 13..=17 {
                msh.add_field(Field::from_value(""));
            }
            msh.add_field(Field::from_value(charset));
        }
        msg.add_segment(msh);

        let mut pid = Segment::new("PID");
        pid.add_field(Field::from_value("1"));
        for _ in 2..=4 {
            pid.add_field(Field::from_value(""));
        }
        pid.add_field(Field::from_value(patient_name));
        msg.add_segment(pid);

        msg
    }

    #[test]
    fn test_charset_content_matching_declaration_is_valid() {
        let validator = Validator::new(Version::V2_5);

        let ascii = message_with_charset(Some("ASCII"), "Muller");
        assert!(validator.validate(&ascii).is_valid());

        let latin1 = message_with_charset(Some("8859/1"), "Müller");
        assert!(validator.validate(&latin1).is_valid());

        // Undeclared messages assume UTF-8, which covers everything.
        let undeclared = message_with_charset(None, "Müller北京");
        assert!(validator.validate(&undeclared).is_valid());
    }

    #[test]
    fn test_charset_content_outside_declaration_is_invalid() {
        let validator = Validator::new(Version::V2_5);

        let msg = message_with_charset(Some("ASCII"), "Müller");
        let result = validator.validate(&msg);
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].location, "PID-5");
        assert!(result.errors[0].message.contains("U+00FC"));
        assert!(result.errors[0].message.contains("ASCII"));
        // Patient data is redacted: only the codepoint is reported.
        assert!(!result.errors[0].message.contains('ü'));

        let msg = message_with_charset(Some("8859/2"), "price: €");
        let result = validator.validate(&msg);
        assert!(!result.is_valid());
        assert_eq!(result.errors[0].location, "PID-5");
        assert!(result.errors[0].message.contains("U+20AC"));
        assert!(!result.errors[0].message.contains('€'));
    }

    #[test]
    fn test_charset_unknown_declaration_is_invalid() {
        let validator = Validator::new(Version::V2_5);
        let msg = message_with_charset(Some("GBK"), "Muller");
        let result = validator.validate(&msg);

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].location, "MSH-18");
        assert!(result.errors[0].message.contains("GBK"));
    }

    #[test]
    fn test_validate_vxu_v04_from_builder() {
        use vyna_hl7_core::builders::vxu::{ImmunizationDose, VxuV04Builder};

        let message = VxuV04Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .add_immunization(
                ImmunizationDose::new("08^HepB adult^CVX", "20240101100000")
                    .amount("1.0")
                    .route("IM^Intramuscular"),
            )
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "VXU", "V04").unwrap();
        let result = validator.validate(&message);
        // Builder output must validate with zero errors: MSH-9.3 is
        // optional (wires carry 2 components) and dotted IDs are legal.
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_composite_table_check_uses_component_code() {
        use vyna_hl7_core::builders::vxu::{ImmunizationDose, VxuV04Builder};

        // RXA-5/RXR-1 carry split CE composites; the field-level table
        // check validates the code in component 1 (RXA-5.1, RXR-1.1).
        let message = VxuV04Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .add_immunization(
                ImmunizationDose::new("ZZ^Not a vaccine^CVX", "20240101100000")
                    .route("XX^Not a route"),
            )
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "VXU", "V04").unwrap();
        let result = validator.validate(&message);
        let table_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| e.error_type == ValidationErrorType::InvalidValue)
            .collect();
        assert_eq!(table_errors.len(), 2);
        assert!(table_errors.iter().any(|e| e.location.starts_with("RXA")
            && e.message.contains("'ZZ'")
            && e.message.contains("0292")));
        assert!(table_errors.iter().any(|e| e.location.starts_with("RXR")
            && e.message.contains("'XX'")
            && e.message.contains("0162")));
    }

    #[test]
    fn test_field_table_code_reads_unsplit_value_literally() {
        // An unsplit composite (one component via from_value) is a single
        // literal value: the table check sees it whole, exactly like the
        // length, datatype, and required checks do.
        let field_def = FieldDefinition {
            name: "Patient Identifier List".to_string(),
            data_type: "CX".to_string(),
            required: true,
            repeating: true,
            max_length: Some(250),
            table_id: Some("0203".to_string()),
            components: None,
        };
        let unsplit = Field::from_value("12345^^^FACILITYWITHLONGNAME^MR^EXTRA");
        assert_eq!(
            field_table_code(&unsplit, 0, &field_def),
            Some("12345^^^FACILITYWITHLONGNAME^MR^EXTRA")
        );

        // Split fields still check component 1 (the code).
        let split = Field::from_delimited("12345^^^FACILITYWITHLONGNAME^MR^EXTRA");
        assert_eq!(field_table_code(&split, 0, &field_def), Some("12345"));
    }

    #[test]
    fn test_strict_context_escalates_version_drift() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.3\rPID|1||7\r",
        )
        .unwrap();

        let standard = Validator::for_message_type(Version::V2_5, "ADT", "A01")
            .unwrap()
            .with_context(ValidationContext::standard());
        let result = standard.validate(&message);
        assert!(result.errors.iter().all(|e| e.location != "MSH-12"));
        assert!(result.warnings.iter().any(|w| w.location == "MSH-12"));

        let strict = Validator::for_message_type(Version::V2_5, "ADT", "A01")
            .unwrap()
            .with_context(ValidationContext::strict());
        let result = strict.validate(&message);
        assert!(result.errors.iter().any(|e| e.location == "MSH-12"));

        let ignored = Validator::for_message_type(Version::V2_5, "ADT", "A01")
            .unwrap()
            .with_context(ValidationContext::standard().with_version_mismatch(Severity::Ignore));
        let result = ignored.validate(&message);
        assert!(
            result.errors.iter().all(|e| e.location != "MSH-12")
                && result.warnings.iter().all(|w| w.location != "MSH-12")
        );
    }

    #[test]
    fn test_lenient_context_warns_instead_of_failing() {
        // PID-8 "X" is not a valid administrative sex (table 0001).
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7||DOE^JOHN|||X\r",
        )
        .unwrap();

        let standard = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
        let result = standard.validate(&message);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.location.starts_with("PID")));

        let lenient = Validator::for_message_type(Version::V2_5, "ADT", "A01")
            .unwrap()
            .with_context(ValidationContext::lenient());
        let result = lenient.validate(&message);
        assert!(result.is_valid());
        assert!(!result.warnings.is_empty());

        // Structural problems still fail under lenient.
        let mut no_msh = vyna_hl7_core::Message::new();
        no_msh.segments.push(vyna_hl7_core::Segment::new("PID"));
        let result = lenient.validate(&no_msh);
        assert!(!result.is_valid());
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == ValidationErrorType::StructuralError)
        );
    }

    #[test]
    fn test_validate_ppr_pc1_from_builder() {
        use vyna_hl7_core::builders::ppr::{PprPc1Builder, Problem};

        let message = PprPc1Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .patient_class("O")
            .add_problem(
                // Short text: builders store composites via from_value, so the
                // whole string counts against the 20-char component limit.
                Problem::new("38341003^HTN^SCT", "PROB-1", "20240101100000")
                    .note("Stage 1, monitor"),
            )
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "PPR", "PC1").unwrap();
        let result = validator.validate(&message);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_ppr_pc1_missing_problem_is_invalid() {
        use vyna_hl7_core::builders::ppr::PprPc1Builder;

        let message = PprPc1Builder::new(Version::V2_5)
            .patient_id("12345")
            .patient_name("Smith", "John")
            .patient_class("O")
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "PPR", "PC1").unwrap();
        let result = validator.validate(&message);
        assert!(!result.is_valid());
        assert!(
            result.errors.iter().any(|e| e.message.contains("PRB")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_omp_o09_from_builder() {
        use vyna_hl7_core::builders::omp::{MedicationOrder, OmpO09Builder};

        let message = OmpO09Builder::new(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .patient_class("O")
            .add_order(
                // Short texts: builders store composites via from_value, so the
                // whole string counts against the 20-char component limit.
                MedicationOrder::new("00093-0058^Loraz^NDC", "PO").amount_minimum("1"),
            )
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "OMP", "O09").unwrap();
        let result = validator.validate(&message);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_omp_o09_missing_order_is_invalid() {
        use vyna_hl7_core::builders::omp::OmpO09Builder;

        let message = OmpO09Builder::new(Version::V2_5)
            .patient_id("12345")
            .patient_name("Smith", "John")
            .patient_class("O")
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "OMP", "O09").unwrap();
        let result = validator.validate(&message);
        assert!(!result.is_valid());
        assert!(
            result.errors.iter().any(|e| e.message.contains("RXO")
                || e.message.contains("RXR")
                || e.message.contains("ORC")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_adt_a34_from_builder() {
        use vyna_hl7_core::builders::adt::AdtBuilder;

        let message = AdtBuilder::a34(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .patient_id("NEW123")
            .patient_name("Smith", "John")
            .prior_patient_id("OLD456")
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A34").unwrap();
        let result = validator.validate(&message);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_adt_a34_missing_prior_id_is_invalid() {
        use vyna_hl7_core::builders::adt::AdtBuilder;

        let message = AdtBuilder::a34(Version::V2_5)
            .patient_id("NEW123")
            .patient_name("Smith", "John")
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A34").unwrap();
        let result = validator.validate(&message);
        assert!(!result.is_valid());
        assert!(
            result.errors.iter().any(|e| e.location.starts_with("MRG")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_adt_a40_from_builder() {
        use vyna_hl7_core::builders::adt::AdtBuilder;

        let message = AdtBuilder::a40(Version::V2_5)
            .sending_application("SEND")
            .sending_facility("SFAC")
            .receiving_application("RECV")
            .receiving_facility("RFAC")
            .patient_id("NEW123")
            .patient_name("Smith", "John")
            .patient_class("I")
            .add_prior_patient_id("OLD456")
            .add_prior_patient_id("OLD789")
            .add_prior_patient_name("Smyth", "Jon")
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A40").unwrap();
        let result = validator.validate(&message);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_validate_adt_a40_missing_prior_id_is_invalid() {
        use vyna_hl7_core::builders::adt::AdtBuilder;

        let message = AdtBuilder::a40(Version::V2_5)
            .patient_id("NEW123")
            .patient_name("Smith", "John")
            .patient_class("I")
            .build()
            .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A40").unwrap();
        let result = validator.validate(&message);
        assert!(!result.is_valid());
        assert!(
            result.errors.iter().any(|e| e.location.starts_with("MRG")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_v21_adt_a01_is_clean() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|SND|SNF|RCV|RCF|199001011200||ADT^A01|MSG00001|P|2.1\r\
             EVN|A01|199001011200\r\
             PID|||12345||DOE^JOHN||19600101|M\r\
             NK1|1|DOE^JANE\r\
             PV1||I|WARD1\r",
        )
        .unwrap();

        let validator = Validator::for_message_type(Version::V2_1, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_v22_adt_a01_is_clean() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|SND|SNF|RCV|RCF|199401011200||ADT^A01|MSG00002|P|2.2\r\
             EVN|A01|199401011200\r\
             PID|||12345||DOE^JOHN||19600101|M\r\
             PV1||I\r",
        )
        .unwrap();

        let validator = Validator::for_message_type(Version::V2_2, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert!(result.is_valid());
    }

    #[test]
    fn test_v21_requires_nk1_while_v22_does_not() {
        // NK1 is required in 2.1 ADT_A01 but optional in 2.2.
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|SND|SNF|RCV|RCF|199001011200||ADT^A01|MSG00001|P|2.1\r\
             EVN|A01|199001011200\r\
             PID|||12345||DOE^JOHN||19600101|M\r\
             PV1||I|WARD1\r",
        )
        .unwrap();

        let v21 = Validator::for_message_type(Version::V2_1, "ADT", "A01").unwrap();
        let result = v21.validate(&message);
        assert!(!result.is_valid());
        assert!(
            result.errors.iter().any(|e| e.message.contains("NK1")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_vxu_v04_missing_pid_is_invalid() {
        use vyna_hl7_core::builders::vxu::VxuV04Builder;

        // The ORDER group (ORC/RXA) is optional per the VXU_V04 grammar, so an
        // orderless message is structurally valid; a missing PID is not.
        let mut message = VxuV04Builder::new(Version::V2_5)
            .patient_id("12345")
            .patient_name("Smith", "John")
            .build()
            .unwrap();
        message.segments.retain(|s| s.id != "PID");

        let validator = Validator::for_message_type(Version::V2_5, "VXU", "V04").unwrap();
        let result = validator.validate(&message);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.message.contains("PID")));
    }

    #[test]
    fn test_parsed_empty_fields_validate_like_sparse_built_message() {
        use vyna_hl7_core::builders::vxu::{ImmunizationDose, VxuV04Builder};

        // The parser materializes empty wire positions (PID-2, PID-4) as
        // fields with one empty repetition, while sparse builders leave them
        // absent. Both mean "not present" in HL7, so both must validate
        // identically: no required-component errors for empty fields.
        let built = VxuV04Builder::new(Version::V2_5)
            .sending_application("IMM")
            .sending_facility("CLINIC")
            .receiving_application("IIS")
            .receiving_facility("REG")
            .patient_id("12345")
            .patient_name("Smith", "John")
            .add_immunization(
                ImmunizationDose::new("08^HEPB^CVX", "20240115130000")
                    .amount("0.5")
                    .units("mL"),
            )
            .build()
            .unwrap();
        let parsed = vyna_hl7_parser::parse_message(&built.encode()).unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "VXU", "V04").unwrap();
        let mut built_errors: Vec<String> = validator
            .validate(&built)
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.location, e.message))
            .collect();
        let mut parsed_errors: Vec<String> = validator
            .validate(&parsed)
            .errors
            .iter()
            .map(|e| format!("{}: {}", e.location, e.message))
            .collect();
        built_errors.sort();
        parsed_errors.sort();
        assert_eq!(
            built_errors, parsed_errors,
            "parsed message must validate like the built message"
        );
        assert!(parsed_errors.is_empty(), "errors: {:?}", parsed_errors);
    }

    #[test]
    fn test_populated_field_missing_required_component_still_errors() {
        // A field with content but a missing required component must still
        // fail: PID-3 "^ST123" populates CX.2 while required CX.1 is empty.
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||^ST123||DOE^JOHN\r",
        )
        .unwrap();

        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.location == "PID[1]-3-1" && e.message.contains("Required component 1")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_field_table_on_later_component_not_checked_at_component_one() {
        // v2.8 PID-5 carries field-level table 0200 (Name Type), which the
        // schema assigns to XPN component 7. A family name in component 1
        // must not be checked against it.
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01^ADT_A01|M1|P|2.8\rPID|1||7||DOE^JOHN\rPV1||I\r",
        )
        .unwrap();

        let validator = Validator::for_message_type(Version::V2_8, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(
            result
                .errors
                .iter()
                .all(|e| !(e.location == "PID[1]-5" && e.message.contains("0200"))),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_cq_field_table_checks_units_component() {
        // QRD-7 is CQ with field-level table 0126, which constrains the
        // units (component 2), not the quantity in component 1.
        let valid = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||QRY^A19|M1|P|2.5\rQRD|20240101|R|I|Q1|||10^RD|12345|DEM|LAB\r",
        )
        .unwrap();
        let validator = Validator::for_message_type(Version::V2_5, "QRY", "A19").unwrap();
        let result = validator.validate(&valid);
        assert!(
            result.errors.iter().all(|e| !e.message.contains("0126")),
            "errors: {:?}",
            result.errors
        );

        let invalid = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||QRY^A19|M1|P|2.5\rQRD|20240101|R|I|Q1|||10^XX|12345|DEM|LAB\r",
        )
        .unwrap();
        let result = validator.validate(&invalid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.location == "QRD[1]-7" && e.message.contains("0126")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_name_type_code_still_validated_at_component_seven() {
        // Component-level table checks still apply: XPN.7 "X" is not a
        // table 0200 code.
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01^ADT_A01|M1|P|2.8\rPID|1||7||DOE^JOHN^^^^^X\rPV1||I\r",
        )
        .unwrap();

        let validator = Validator::for_message_type(Version::V2_8, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.location == "PID[1]-5-7" && e.message.contains("0200")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_unparsable_msh12_is_invalid_value() {
        // MSH-12 "9.9" names no known version: not "no drift" but an
        // unparsable value, failing strict validation.
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|9.9\rPID|1||7\r",
        )
        .unwrap();
        let result = Validator::new(Version::V2_5).validate(&message);
        assert!(!result.is_valid(), "unparsable MSH-12 must fail");
        assert!(
            result.errors.iter().any(
                |e| e.location == "MSH-12" && e.error_type == ValidationErrorType::InvalidValue
            ),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_empty_msh12_stays_valid_without_schema() {
        let message =
            vyna_hl7_parser::parse_message("MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|\rPID|1||7\r")
                .unwrap();
        let result = Validator::new(Version::V2_5).validate(&message);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    /// Schema covering MSH (no fields) plus PID-5 with `field_def`.
    fn schema_with_pid5(field_def: FieldDefinition) -> MessageSchema {
        let mut segments = HashMap::new();
        segments.insert(
            "MSH".to_string(),
            SegmentDefinition {
                name: "Message Header".to_string(),
                required: false,
                repeating: false,
                fields: HashMap::new(),
            },
        );
        let mut fields = HashMap::new();
        fields.insert(5, field_def);
        segments.insert(
            "PID".to_string(),
            SegmentDefinition {
                name: "Patient Identification".to_string(),
                required: false,
                repeating: false,
                fields,
            },
        );
        MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            version: "2.5".to_string(),
            segments,
        }
    }

    #[test]
    fn test_field_length_counts_characters_not_bytes() {
        // "Müller": 6 chars but 7 bytes; HL7 lengths are characters.
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7||Müller\r",
        )
        .unwrap();
        let field_def = FieldDefinition {
            name: "Patient Name".to_string(),
            data_type: "ST".to_string(),
            required: false,
            repeating: false,
            max_length: Some(6),
            table_id: None,
            components: None,
        };
        let result =
            Validator::with_schema(Version::V2_5, schema_with_pid5(field_def)).validate(&message);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_component_length_counts_characters_not_bytes() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7||Müller\r",
        )
        .unwrap();
        let mut components = HashMap::new();
        components.insert(
            "1".to_string(),
            ComponentDefinition {
                name: "Family Name".to_string(),
                data_type: "FN".to_string(),
                required: false,
                max_length: Some(6),
                table_id: None,
            },
        );
        let field_def = FieldDefinition {
            name: "Patient Name".to_string(),
            data_type: "XPN".to_string(),
            required: false,
            repeating: false,
            max_length: None,
            table_id: None,
            components: Some(components),
        };
        let result =
            Validator::with_schema(Version::V2_5, schema_with_pid5(field_def)).validate(&message);
        assert!(result.is_valid(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_lenient_context_keeps_rule_error_severity() {
        // Rule-engine violations keep their declared severity: a rule
        // Error fails even under a lenient context.
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7||DOE^JOHN\r",
        )
        .unwrap();
        let rule =
            ValidationRule::new("always_fail", "Rule that always fails", RuleSeverity::Error)
                .with_condition(|_msg| false);
        let mut validator =
            Validator::new(Version::V2_5).with_context(ValidationContext::lenient());
        validator.add_rule(rule);
        let result = validator.validate(&message);
        assert!(!result.is_valid(), "rule Error must fail even when lenient");
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.warnings.is_empty(),
            "warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_field_checks_cover_all_repetitions() {
        // MSH-18 "ASCII~!!!": the bad second repetition must be
        // flagged even though the first repetition is clean.
        let mut msg = Message::new();
        let mut msh = Segment::new("MSH");
        msh.set_field(1, Field::from_value("|")).unwrap();
        msh.set_field(2, Field::from_value("^~\\&")).unwrap();
        msh.set_field(9, Field::from_delimited("ADT^A01")).unwrap();
        msh.set_field(12, Field::from_value("2.5")).unwrap();
        msh.set_field(18, Field::from_delimited("ASCII~!!!"))
            .unwrap();
        msg.add_segment(msh);

        let mut fields = HashMap::new();
        fields.insert(
            18,
            FieldDefinition {
                name: "Character Set".to_string(),
                data_type: "ID".to_string(),
                required: false,
                repeating: true,
                max_length: Some(16),
                table_id: Some("0211".to_string()),
                components: None,
            },
        );
        let mut segments = HashMap::new();
        segments.insert(
            "MSH".to_string(),
            SegmentDefinition {
                name: "Message Header".to_string(),
                required: false,
                repeating: false,
                fields,
            },
        );
        let schema = MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            version: "2.5".to_string(),
            segments,
        };
        let result = Validator::with_schema(Version::V2_5, schema).validate(&msg);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == ValidationErrorType::InvalidDataType),
            "errors: {:?}",
            result.errors
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == ValidationErrorType::InvalidValue),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_non_repeating_segment_cardinality_errors() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rEVN|A01|20240101\r\
             PID|1||7||DOE^JOHN||19600101|M\rPID|1||8||SMITH^JANE\rPV1||I\r",
        )
        .unwrap();
        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == ValidationErrorType::InvalidCardinality
                    && e.location == "PID"),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_non_repeating_field_cardinality_errors() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rEVN|A01|20240101\r\
             PID|1|AAA~BBB|7||DOE^JOHN\rPV1||I\r",
        )
        .unwrap();
        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == ValidationErrorType::InvalidCardinality
                    && e.location.contains("-2")),
            "errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_repeating_field_cardinality_passes() {
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rEVN|A01|20240101\r\
             PID|1||7||DOE^JOHN\rPV1||I\r",
        )
        .unwrap();
        let validator = Validator::for_message_type(Version::V2_5, "ADT", "A01").unwrap();
        let result = validator.validate(&message);
        assert!(
            result
                .errors
                .iter()
                .all(|e| e.error_type != ValidationErrorType::InvalidCardinality),
            "errors: {:?}",
            result.errors
        );
    }
}
