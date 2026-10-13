//! Core trait and types for custom Z-segments

use crate::error::Result;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use vyna_hl7_core::Segment;

/// Trait for custom Z-segments
///
/// This trait must be implemented by all custom Z-segment types. It provides
/// methods for parsing segments from HL7 structures, encoding them back,
/// validation, and documentation.
///
/// # Example
///
/// ```rust,ignore
/// use vyna_hl7_custom::CustomSegment;
/// use vyna_hl7_core::Segment;
///
/// struct ZPV {
///     visit_type: String,
///     visit_number: String,
/// }
///
/// impl CustomSegment for ZPV {
///     fn segment_id() -> &'static str {
///         "ZPV"
///     }
///
///     fn from_segment(segment: &Segment) -> Result<Self> {
///         Ok(ZPV {
///             visit_type: segment.get_field(1)?.to_string(),
///             visit_number: segment.get_field(2)?.to_string(),
///         })
///     }
///
///     fn to_segment(&self) -> Segment {
///         let mut segment = Segment::new("ZPV");
///         segment.set_field_value(1, &self.visit_type)?;
///         segment.set_field_value(2, &self.visit_number)?;
///         segment
///     }
/// }
/// ```
pub trait CustomSegment: Send + Sync {
    /// Get the segment ID (e.g., "ZPV", "ZCU")
    ///
    /// This must start with 'Z' as per HL7 convention for custom segments.
    fn segment_id() -> &'static str
    where
        Self: Sized;

    /// Parse a custom segment from an HL7 segment structure
    ///
    /// # Arguments
    ///
    /// * `segment` - The HL7 segment to parse
    ///
    /// # Returns
    ///
    /// The parsed custom segment or an error if parsing fails
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required fields are missing
    /// - Field values are invalid
    /// - Segment structure doesn't match expected format
    fn from_segment(segment: &Segment) -> Result<Self>
    where
        Self: Sized;

    /// Convert the custom segment to an HL7 segment structure
    ///
    /// # Returns
    ///
    /// An HL7 segment that can be encoded to ER7 format
    fn to_segment(&self) -> Segment;

    /// Validate the segment's business rules
    ///
    /// This is called after parsing and can be used to implement custom
    /// validation logic beyond basic field presence checks.
    ///
    /// # Returns
    ///
    /// Ok(()) if validation passes, or an error describing what failed
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn validate(&self) -> Result<()> {
    ///     if self.balance.unwrap_or(0.0) < 0.0 {
    ///         return Err(CustomSegmentError::validation_failed(
    ///             "ZCU",
    ///             "Balance cannot be negative"
    ///         ));
    ///     }
    ///     Ok(())
    /// }
    /// ```
    fn validate(&self) -> Result<()> {
        Ok(()) // Default: no validation
    }

    /// Get field definitions for documentation purposes
    ///
    /// This is optional but recommended for generating documentation
    /// and supporting tools.
    ///
    /// # Returns
    ///
    /// A vector of field definitions describing each field in the segment
    fn field_definitions() -> Vec<FieldDefinition>
    where
        Self: Sized,
    {
        Vec::new() // Default: no definitions
    }

    /// Get the segment type name for debugging
    ///
    /// Returns the Rust type name by default, can be overridden for
    /// custom display.
    fn type_name() -> &'static str
    where
        Self: Sized,
    {
        std::any::type_name::<Self>()
    }
}

/// Definition of a field in a custom segment
///
/// This is used for documentation, validation, and tooling support.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDefinition {
    /// Field number (1-based)
    pub number: usize,

    /// Human-readable field name
    pub name: String,

    /// HL7 data type (e.g., "ST", "NM", "CWE")
    pub data_type: String,

    /// Whether this field is required
    pub required: bool,

    /// Whether this field can repeat
    pub repeatable: bool,

    /// Maximum length (None for unlimited)
    pub max_length: Option<usize>,

    /// Human-readable description
    pub description: String,
}

impl FieldDefinition {
    /// Create a new field definition builder
    pub fn builder() -> FieldDefinitionBuilder {
        FieldDefinitionBuilder::default()
    }
}

/// Builder for creating field definitions
#[derive(Default)]
pub struct FieldDefinitionBuilder {
    number: Option<usize>,
    name: Option<String>,
    data_type: Option<String>,
    required: bool,
    repeatable: bool,
    max_length: Option<usize>,
    description: Option<String>,
}

impl FieldDefinitionBuilder {
    /// Set the field number
    pub fn number(mut self, number: usize) -> Self {
        self.number = Some(number);
        self
    }

    /// Set the field name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the HL7 data type
    pub fn data_type(mut self, data_type: impl Into<String>) -> Self {
        self.data_type = Some(data_type.into());
        self
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Mark field as optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Mark field as repeatable
    pub fn repeatable(mut self) -> Self {
        self.repeatable = true;
        self
    }

    /// Set maximum length
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Set description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Build the field definition
    pub fn build(self) -> FieldDefinition {
        FieldDefinition {
            number: self.number.expect("Field number is required"),
            name: self.name.expect("Field name is required"),
            data_type: self.data_type.expect("Data type is required"),
            required: self.required,
            repeatable: self.repeatable,
            max_length: self.max_length,
            description: self.description.unwrap_or_default(),
        }
    }
}

/// Trait for parsing field values from segments
pub trait ParseSegmentField: Sized {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self>;
}

/// Trait for serializing field values to segments
pub trait SerializeSegmentField {
    fn set_field(&self, segment: &mut Segment, field_num: usize);
}

/// Trait to determine the inner type for builder fields
pub trait BuilderFieldType {
    type Inner;
}

/// Wrapper for builder fields
#[derive(Debug, Default)]
pub struct BuilderField<T: BuildableField> {
    value: T::Storage,
}

/// Trait for types that can be built from builder fields
pub trait BuildableField: Sized {
    type Storage: Default;
    fn set_value(storage: &mut Self::Storage, value: Self::Inner);
    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self>;
    type Inner;
}

impl<T: BuildableField> BuilderField<T> {
    pub fn set(&mut self, value: T::Inner) {
        T::set_value(&mut self.value, value);
    }

    pub fn build(self, field_name: &str, seg_id: &str) -> Result<T> {
        T::build_value(self.value, field_name, seg_id)
    }
}

// Implement BuildableField for required types
impl BuildableField for String {
    type Storage = Option<String>;
    type Inner = String;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

impl BuildableField for f64 {
    type Storage = Option<f64>;
    type Inner = f64;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

impl BuildableField for u32 {
    type Storage = Option<u32>;
    type Inner = u32;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

// Implement BuildableField for optional types
impl BuildableField for Option<String> {
    type Storage = Option<String>;
    type Inner = String;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl BuildableField for Option<f64> {
    type Storage = Option<f64>;
    type Inner = f64;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl BuildableField for Option<u32> {
    type Storage = Option<u32>;
    type Inner = u32;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

// Implementations for String
impl ParseSegmentField for String {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        segment
            .get_field_value(field_num)
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::error::CustomSegmentError::missing_field(
                    format!("{}-{}", seg_id, field_num),
                    seg_id,
                )
            })
    }
}

impl ParseSegmentField for Option<String> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        Ok(segment.get_field_value(field_num).map(|s| s.to_string()))
    }
}

impl SerializeSegmentField for String {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let _ = segment.set_field_value(field_num, self);
    }
}

impl SerializeSegmentField for Option<String> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(v) = self {
            let _ = segment.set_field_value(field_num, v);
        }
    }
}

impl BuilderFieldType for String {
    type Inner = String;
}

impl BuilderFieldType for Option<String> {
    type Inner = String;
}

// Implementations for f64
impl ParseSegmentField for f64 {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, |s| s.parse::<f64>().ok())
    }
}

impl ParseSegmentField for Option<f64> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, |s| s.parse::<f64>().ok())
    }
}

impl SerializeSegmentField for f64 {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let _ = segment.set_field_value(field_num, self.to_string());
    }
}

impl SerializeSegmentField for Option<f64> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(v) = self {
            let _ = segment.set_field_value(field_num, v.to_string());
        }
    }
}

impl BuilderFieldType for f64 {
    type Inner = f64;
}

impl BuilderFieldType for Option<f64> {
    type Inner = f64;
}

// Implementations for u32
impl ParseSegmentField for u32 {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, |s| s.parse::<u32>().ok())
    }
}

impl ParseSegmentField for Option<u32> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, |s| s.parse::<u32>().ok())
    }
}

impl SerializeSegmentField for u32 {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let _ = segment.set_field_value(field_num, self.to_string());
    }
}

impl SerializeSegmentField for Option<u32> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(v) = self {
            let _ = segment.set_field_value(field_num, v.to_string());
        }
    }
}

impl BuilderFieldType for u32 {
    type Inner = u32;
}

impl BuilderFieldType for Option<u32> {
    type Inner = u32;
}

// Implementations for i32
impl BuildableField for i32 {
    type Storage = Option<i32>;
    type Inner = i32;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

impl BuildableField for Option<i32> {
    type Storage = Option<i32>;
    type Inner = i32;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for i32 {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, |s| s.parse::<i32>().ok())
    }
}

impl ParseSegmentField for Option<i32> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, |s| s.parse::<i32>().ok())
    }
}

impl SerializeSegmentField for i32 {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let _ = segment.set_field_value(field_num, self.to_string());
    }
}

impl SerializeSegmentField for Option<i32> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(v) = self {
            let _ = segment.set_field_value(field_num, v.to_string());
        }
    }
}

impl BuilderFieldType for i32 {
    type Inner = i32;
}

impl BuilderFieldType for Option<i32> {
    type Inner = i32;
}

// Implementations for i64
impl BuildableField for i64 {
    type Storage = Option<i64>;
    type Inner = i64;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

impl BuildableField for Option<i64> {
    type Storage = Option<i64>;
    type Inner = i64;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for i64 {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, |s| s.parse::<i64>().ok())
    }
}

impl ParseSegmentField for Option<i64> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, |s| s.parse::<i64>().ok())
    }
}

impl SerializeSegmentField for i64 {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let _ = segment.set_field_value(field_num, self.to_string());
    }
}

impl SerializeSegmentField for Option<i64> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(v) = self {
            let _ = segment.set_field_value(field_num, v.to_string());
        }
    }
}

impl BuilderFieldType for i64 {
    type Inner = i64;
}

impl BuilderFieldType for Option<i64> {
    type Inner = i64;
}

// Implementations for bool
impl BuildableField for bool {
    type Storage = Option<bool>;
    type Inner = bool;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

impl BuildableField for Option<bool> {
    type Storage = Option<bool>;
    type Inner = bool;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for bool {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        // HL7 typically uses Y/N or 1/0 for booleans
        parse_required_scalar(segment, field_num, seg_id, parse_hl7_bool)
    }
}

impl ParseSegmentField for Option<bool> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, parse_hl7_bool)
    }
}

impl SerializeSegmentField for bool {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let value = if *self { "Y" } else { "N" };
        let _ = segment.set_field_value(field_num, value);
    }
}

impl SerializeSegmentField for Option<bool> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(v) = self {
            let value = if *v { "Y" } else { "N" };
            let _ = segment.set_field_value(field_num, value);
        }
    }
}

impl BuilderFieldType for bool {
    type Inner = bool;
}

impl BuilderFieldType for Option<bool> {
    type Inner = bool;
}

// ============================================================================
// NaiveDateTime implementations (timestamp without timezone)
// ============================================================================

// BuildableField for required NaiveDateTime
impl BuildableField for NaiveDateTime {
    type Storage = Option<NaiveDateTime>;
    type Inner = NaiveDateTime;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

// BuildableField for optional NaiveDateTime
impl BuildableField for Option<NaiveDateTime> {
    type Storage = Option<NaiveDateTime>;
    type Inner = NaiveDateTime;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

// ParseSegmentField for NaiveDateTime
// Supports HL7 datetime formats: YYYYMMDDHHMMSS[.SSSS]
impl ParseSegmentField for NaiveDateTime {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, parse_hl7_datetime)
    }
}

// ParseSegmentField for Option<NaiveDateTime>
impl ParseSegmentField for Option<NaiveDateTime> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, parse_hl7_datetime)
    }
}

// SerializeSegmentField for NaiveDateTime
impl SerializeSegmentField for NaiveDateTime {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let formatted = self.format("%Y%m%d%H%M%S").to_string();
        let _ = segment.set_field_value(field_num, &formatted);
    }
}

// SerializeSegmentField for Option<NaiveDateTime>
impl SerializeSegmentField for Option<NaiveDateTime> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(dt) = self {
            dt.set_field(segment, field_num);
        }
    }
}

// BuilderFieldType for NaiveDateTime
impl BuilderFieldType for NaiveDateTime {
    type Inner = NaiveDateTime;
}

// BuilderFieldType for Option<NaiveDateTime>
impl BuilderFieldType for Option<NaiveDateTime> {
    type Inner = NaiveDateTime;
}

// ============================================================================
// NaiveDate implementations (date without time)
// ============================================================================

// BuildableField for required NaiveDate
impl BuildableField for NaiveDate {
    type Storage = Option<NaiveDate>;
    type Inner = NaiveDate;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

// BuildableField for optional NaiveDate
impl BuildableField for Option<NaiveDate> {
    type Storage = Option<NaiveDate>;
    type Inner = NaiveDate;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

// ParseSegmentField for NaiveDate
// Supports HL7 date formats: YYYYMMDD, YYYYMM, YYYY
impl ParseSegmentField for NaiveDate {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, parse_hl7_date)
    }
}

// ParseSegmentField for Option<NaiveDate>
impl ParseSegmentField for Option<NaiveDate> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, parse_hl7_date)
    }
}

// SerializeSegmentField for NaiveDate
impl SerializeSegmentField for NaiveDate {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let formatted = self.format("%Y%m%d").to_string();
        let _ = segment.set_field_value(field_num, &formatted);
    }
}

// SerializeSegmentField for Option<NaiveDate>
impl SerializeSegmentField for Option<NaiveDate> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(date) = self {
            date.set_field(segment, field_num);
        }
    }
}

// BuilderFieldType for NaiveDate
impl BuilderFieldType for NaiveDate {
    type Inner = NaiveDate;
}

// BuilderFieldType for Option<NaiveDate>
impl BuilderFieldType for Option<NaiveDate> {
    type Inner = NaiveDate;
}

// ============================================================================
// NaiveTime implementations (time without date)
// ============================================================================

// BuildableField for required NaiveTime
impl BuildableField for NaiveTime {
    type Storage = Option<NaiveTime>;
    type Inner = NaiveTime;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

// BuildableField for optional NaiveTime
impl BuildableField for Option<NaiveTime> {
    type Storage = Option<NaiveTime>;
    type Inner = NaiveTime;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

// ParseSegmentField for NaiveTime
// Supports HL7 time formats: HHMMSS[.SSSS], HHMM
impl ParseSegmentField for NaiveTime {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, parse_hl7_time)
    }
}

// ParseSegmentField for Option<NaiveTime>
impl ParseSegmentField for Option<NaiveTime> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, parse_hl7_time)
    }
}

// SerializeSegmentField for NaiveTime
impl SerializeSegmentField for NaiveTime {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let formatted = self.format("%H%M%S").to_string();
        let _ = segment.set_field_value(field_num, &formatted);
    }
}

// SerializeSegmentField for Option<NaiveTime>
impl SerializeSegmentField for Option<NaiveTime> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(time) = self {
            time.set_field(segment, field_num);
        }
    }
}

// BuilderFieldType for NaiveTime
impl BuilderFieldType for NaiveTime {
    type Inner = NaiveTime;
}

// BuilderFieldType for Option<NaiveTime>
impl BuilderFieldType for Option<NaiveTime> {
    type Inner = NaiveTime;
}

// ============================================================================
// DateTime<Utc> implementations (timezone-aware timestamp)
// ============================================================================

// BuildableField for required DateTime<Utc>
impl BuildableField for DateTime<Utc> {
    type Storage = Option<DateTime<Utc>>;
    type Inner = DateTime<Utc>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        storage.ok_or_else(|| {
            crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )
        })
    }
}

// BuildableField for optional DateTime<Utc>
impl BuildableField for Option<DateTime<Utc>> {
    type Storage = Option<DateTime<Utc>>;
    type Inner = DateTime<Utc>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = Some(value);
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

// ParseSegmentField for DateTime<Utc>
// Supports HL7 datetime formats with timezone: YYYYMMDDHHMMSS[.SSSS][+/-ZZZZ]
impl ParseSegmentField for DateTime<Utc> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_required_scalar(segment, field_num, seg_id, parse_hl7_datetime_utc)
    }
}

// ParseSegmentField for Option<DateTime<Utc>>
impl ParseSegmentField for Option<DateTime<Utc>> {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        parse_optional_scalar(segment, field_num, seg_id, parse_hl7_datetime_utc)
    }
}

// SerializeSegmentField for DateTime<Utc>
// Writes the UTC wall time without an offset suffix; values without a
// suffix parse back as UTC, so the round trip is stable.
impl SerializeSegmentField for DateTime<Utc> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        let formatted = self.format("%Y%m%d%H%M%S").to_string();
        let _ = segment.set_field_value(field_num, &formatted);
    }
}

// SerializeSegmentField for Option<DateTime<Utc>>
impl SerializeSegmentField for Option<DateTime<Utc>> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some(dt) = self {
            dt.set_field(segment, field_num);
        }
    }
}

// BuilderFieldType for DateTime<Utc>
impl BuilderFieldType for DateTime<Utc> {
    type Inner = DateTime<Utc>;
}

// BuilderFieldType for Option<DateTime<Utc>>
impl BuilderFieldType for Option<DateTime<Utc>> {
    type Inner = DateTime<Utc>;
}

// ============================================================================
// Helper functions for parsing HL7 date/time formats
// ============================================================================

/// Parse one scalar field value, telling "absent" from "invalid".
///
/// Returns `Ok(None)` when the field is absent or empty, `Ok(Some(v))`
/// when it parses, and `InvalidFieldValue` when a non-empty value fails
/// to parse — never `MissingField` for a value that is present.
fn parse_optional_scalar<T, F>(
    segment: &Segment,
    field_num: usize,
    seg_id: &str,
    parse: F,
) -> Result<Option<T>>
where
    F: FnOnce(&str) -> Option<T>,
{
    match segment.get_field_value(field_num) {
        None | Some("") => Ok(None),
        Some(s) => parse(s).map(Some).ok_or_else(|| {
            crate::error::CustomSegmentError::invalid_field(
                format!("{seg_id}-{field_num}"),
                seg_id,
                format!("cannot parse {s:?}"),
            )
        }),
    }
}

/// Parse a required scalar field: absent/empty is `MissingField`,
/// present-but-unparseable is `InvalidFieldValue`.
fn parse_required_scalar<T, F>(
    segment: &Segment,
    field_num: usize,
    seg_id: &str,
    parse: F,
) -> Result<T>
where
    F: FnOnce(&str) -> Option<T>,
{
    parse_optional_scalar(segment, field_num, seg_id, parse)?.ok_or_else(|| {
        crate::error::CustomSegmentError::missing_field(format!("{seg_id}-{field_num}"), seg_id)
    })
}

/// Parse HL7 boolean formats (Y/N, YES/NO, T/F, TRUE/FALSE, 1/0).
fn parse_hl7_bool(s: &str) -> Option<bool> {
    match s.to_uppercase().as_str() {
        "Y" | "YES" | "T" | "TRUE" | "1" => Some(true),
        "N" | "NO" | "F" | "FALSE" | "0" => Some(false),
        _ => None,
    }
}

/// Parse HL7 datetime format: YYYYMMDDHHMMSS[.SSSS]
fn parse_hl7_datetime(s: &str) -> Option<NaiveDateTime> {
    // Try full format with optional fractional seconds
    // (.get: non-ASCII input yields None, never panics)
    if s.len() >= 14 {
        let year = s.get(0..4)?.parse::<i32>().ok()?;
        let month = s.get(4..6)?.parse::<u32>().ok()?;
        let day = s.get(6..8)?.parse::<u32>().ok()?;
        let hour = s.get(8..10)?.parse::<u32>().ok()?;
        let minute = s.get(10..12)?.parse::<u32>().ok()?;
        let second = s.get(12..14)?.parse::<u32>().ok()?;

        let date = NaiveDate::from_ymd_opt(year, month, day)?;
        let time = NaiveTime::from_hms_opt(hour, minute, second)?;

        return Some(NaiveDateTime::new(date, time));
    }

    None
}

/// Parse HL7 date format: YYYYMMDD, YYYYMM, or YYYY
fn parse_hl7_date(s: &str) -> Option<NaiveDate> {
    match s.len() {
        8 => {
            // YYYYMMDD (.get: non-ASCII yields None, never panics)
            let year = s.get(0..4)?.parse::<i32>().ok()?;
            let month = s.get(4..6)?.parse::<u32>().ok()?;
            let day = s.get(6..8)?.parse::<u32>().ok()?;
            NaiveDate::from_ymd_opt(year, month, day)
        }
        6 => {
            // YYYYMM - default to first day of month
            let year = s.get(0..4)?.parse::<i32>().ok()?;
            let month = s.get(4..6)?.parse::<u32>().ok()?;
            NaiveDate::from_ymd_opt(year, month, 1)
        }
        4 => {
            // YYYY - default to January 1st
            let year = s.get(0..4)?.parse::<i32>().ok()?;
            NaiveDate::from_ymd_opt(year, 1, 1)
        }
        _ => None,
    }
}

/// Parse HL7 time format: HHMMSS[.SSSS] or HHMM
fn parse_hl7_time(s: &str) -> Option<NaiveTime> {
    if s.len() >= 6 {
        // HHMMSS format (.get: non-ASCII yields None, never panics)
        let hour = s.get(0..2)?.parse::<u32>().ok()?;
        let minute = s.get(2..4)?.parse::<u32>().ok()?;
        let second = s.get(4..6)?.parse::<u32>().ok()?;
        NaiveTime::from_hms_opt(hour, minute, second)
    } else if s.len() == 4 {
        // HHMM format
        let hour = s.get(0..2)?.parse::<u32>().ok()?;
        let minute = s.get(2..4)?.parse::<u32>().ok()?;
        NaiveTime::from_hms_opt(hour, minute, 0)
    } else {
        None
    }
}

/// Parse HL7 datetime with timezone: YYYYMMDDHHMMSS[+/-ZZZZ]
///
/// A trailing `+/-ZZZZ` offset is applied and the result converted to
/// UTC; values without an offset are interpreted as UTC.
fn parse_hl7_datetime_utc(s: &str) -> Option<DateTime<Utc>> {
    use chrono::TimeZone;

    let (head, offset_secs) = split_hl7_offset(s)?;
    let naive_dt = parse_hl7_datetime(head)?;
    if offset_secs == 0 {
        return Some(Utc.from_utc_datetime(&naive_dt));
    }
    let offset = chrono::FixedOffset::east_opt(offset_secs)?;
    offset
        .from_local_datetime(&naive_dt)
        .single()
        .map(|dt| dt.to_utc())
}

/// Split a trailing `[+/-]ZZZZ` offset from an HL7 datetime.
///
/// Returns the head and the offset in seconds east of UTC, or `None`
/// when the value ends with a malformed offset.
fn split_hl7_offset(s: &str) -> Option<(&str, i32)> {
    if s.len() < 5 {
        return Some((s, 0));
    }
    let (head, tail) = s.split_at_checked(s.len() - 5)?;
    // Only +/- lead bytes take the offset branch, so tail[1..] below
    // always starts on a character boundary.
    let sign = match tail.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return Some((s, 0)),
    };
    let digits: i32 = tail[1..].parse().ok()?;
    let hours = digits / 100;
    let minutes = digits % 100;
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) {
        return None;
    }
    Some((head, sign * (hours * 3600 + minutes * 60)))
}

// ============================================================================
// Vec<T> implementations for repeating fields
// ============================================================================

// Vec<String> - Repeating text fields
impl BuildableField for Vec<String> {
    type Storage = Vec<String>;
    type Inner = Vec<String>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for Vec<String> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let values: Vec<String> = field
                .repetitions
                .iter()
                .filter_map(|rep| rep.value().map(|s| s.to_string()))
                .collect();
            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<String> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Field, Repetition};

        let mut field = Field::new();
        for value in self {
            field.add_repetition(Repetition::from_value(value));
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for Vec<String> {
    type Inner = Vec<String>;
}

// Vec<u32> - Repeating unsigned integers
impl BuildableField for Vec<u32> {
    type Storage = Vec<u32>;
    type Inner = Vec<u32>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for Vec<u32> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let values: Vec<u32> = field
                .repetitions
                .iter()
                .filter_map(|rep| rep.value().and_then(|s| s.parse::<u32>().ok()))
                .collect();
            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<u32> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Field, Repetition};

        let mut field = Field::new();
        for value in self {
            field.add_repetition(Repetition::from_value(value.to_string()));
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for Vec<u32> {
    type Inner = Vec<u32>;
}

// Vec<i32> - Repeating signed integers
impl BuildableField for Vec<i32> {
    type Storage = Vec<i32>;
    type Inner = Vec<i32>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for Vec<i32> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let values: Vec<i32> = field
                .repetitions
                .iter()
                .filter_map(|rep| rep.value().and_then(|s| s.parse::<i32>().ok()))
                .collect();
            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<i32> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Field, Repetition};

        let mut field = Field::new();
        for value in self {
            field.add_repetition(Repetition::from_value(value.to_string()));
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for Vec<i32> {
    type Inner = Vec<i32>;
}

// Vec<i64> - Repeating large integers
impl BuildableField for Vec<i64> {
    type Storage = Vec<i64>;
    type Inner = Vec<i64>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for Vec<i64> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let values: Vec<i64> = field
                .repetitions
                .iter()
                .filter_map(|rep| rep.value().and_then(|s| s.parse::<i64>().ok()))
                .collect();
            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<i64> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Field, Repetition};

        let mut field = Field::new();
        for value in self {
            field.add_repetition(Repetition::from_value(value.to_string()));
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for Vec<i64> {
    type Inner = Vec<i64>;
}

// Vec<f64> - Repeating floating point numbers
impl BuildableField for Vec<f64> {
    type Storage = Vec<f64>;
    type Inner = Vec<f64>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for Vec<f64> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let values: Vec<f64> = field
                .repetitions
                .iter()
                .filter_map(|rep| rep.value().and_then(|s| s.parse::<f64>().ok()))
                .collect();
            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<f64> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Field, Repetition};

        let mut field = Field::new();
        for value in self {
            field.add_repetition(Repetition::from_value(value.to_string()));
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for Vec<f64> {
    type Inner = Vec<f64>;
}

// Vec<bool> - Repeating boolean flags
impl BuildableField for Vec<bool> {
    type Storage = Vec<bool>;
    type Inner = Vec<bool>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl ParseSegmentField for Vec<bool> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let values: Vec<bool> = field
                .repetitions
                .iter()
                .filter_map(|rep| {
                    rep.value().and_then(|s| match s.to_uppercase().as_str() {
                        "Y" | "YES" | "T" | "TRUE" | "1" => Some(true),
                        "N" | "NO" | "F" | "FALSE" | "0" => Some(false),
                        _ => None,
                    })
                })
                .collect();
            Ok(values)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<bool> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Field, Repetition};

        let mut field = Field::new();
        for value in self {
            let str_value = if *value { "Y" } else { "N" };
            field.add_repetition(Repetition::from_value(str_value));
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for Vec<bool> {
    type Inner = Vec<bool>;
}

// ============================================================================
// Tuple types for field components
// ============================================================================
//
// Components are sub-parts of fields separated by ^ (caret) in HL7.
// Example: "Smith^John^A" represents last name^first name^middle initial
//
// We support tuples of 2-5 elements to represent common component structures.

// Tuple of 2 components: (String, String)
impl BuildableField for (String, String) {
    type Storage = (Option<String>, Option<String>);
    type Inner = (String, String);

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = (Some(value.0), Some(value.1));
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1)) => Ok((c0, c1)),
            _ => Err(crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )),
        }
    }
}

impl ParseSegmentField for (String, String) {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep
                .get_component(0)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c1 = rep
                .get_component(1)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();

            if !c0.is_empty() && !c1.is_empty() {
                return Ok((c0, c1));
            }
        }
        Err(crate::error::CustomSegmentError::missing_field(
            format!("{}-{}", seg_id, field_num),
            seg_id,
        ))
    }
}

impl SerializeSegmentField for (String, String) {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.0));
        rep.add_component(Component::from_value(&self.1));

        let mut field = Field::new();
        field.add_repetition(rep);
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for (String, String) {
    type Inner = (String, String);
}

// Tuple of 3 components: (String, String, String)
impl BuildableField for (String, String, String) {
    type Storage = (Option<String>, Option<String>, Option<String>);
    type Inner = (String, String, String);

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = (Some(value.0), Some(value.1), Some(value.2));
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1), Some(c2)) => Ok((c0, c1, c2)),
            _ => Err(crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )),
        }
    }
}

impl ParseSegmentField for (String, String, String) {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep
                .get_component(0)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c1 = rep
                .get_component(1)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c2 = rep
                .get_component(2)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();

            if !c0.is_empty() && !c1.is_empty() && !c2.is_empty() {
                return Ok((c0, c1, c2));
            }
        }
        Err(crate::error::CustomSegmentError::missing_field(
            format!("{}-{}", seg_id, field_num),
            seg_id,
        ))
    }
}

impl SerializeSegmentField for (String, String, String) {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.0));
        rep.add_component(Component::from_value(&self.1));
        rep.add_component(Component::from_value(&self.2));

        let mut field = Field::new();
        field.add_repetition(rep);
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for (String, String, String) {
    type Inner = (String, String, String);
}

// Tuple of 4 components: (String, String, String, String)
impl BuildableField for (String, String, String, String) {
    type Storage = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    type Inner = (String, String, String, String);

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = (Some(value.0), Some(value.1), Some(value.2), Some(value.3));
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1), Some(c2), Some(c3)) => Ok((c0, c1, c2, c3)),
            _ => Err(crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )),
        }
    }
}

impl ParseSegmentField for (String, String, String, String) {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep
                .get_component(0)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c1 = rep
                .get_component(1)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c2 = rep
                .get_component(2)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c3 = rep
                .get_component(3)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();

            if !c0.is_empty() && !c1.is_empty() && !c2.is_empty() && !c3.is_empty() {
                return Ok((c0, c1, c2, c3));
            }
        }
        Err(crate::error::CustomSegmentError::missing_field(
            format!("{}-{}", seg_id, field_num),
            seg_id,
        ))
    }
}

impl SerializeSegmentField for (String, String, String, String) {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.0));
        rep.add_component(Component::from_value(&self.1));
        rep.add_component(Component::from_value(&self.2));
        rep.add_component(Component::from_value(&self.3));

        let mut field = Field::new();
        field.add_repetition(rep);
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for (String, String, String, String) {
    type Inner = (String, String, String, String);
}

// Tuple of 5 components: (String, String, String, String, String)
impl BuildableField for (String, String, String, String, String) {
    type Storage = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    type Inner = (String, String, String, String, String);

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = (
            Some(value.0),
            Some(value.1),
            Some(value.2),
            Some(value.3),
            Some(value.4),
        );
    }

    fn build_value(storage: Self::Storage, field_name: &str, seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1), Some(c2), Some(c3), Some(c4)) => Ok((c0, c1, c2, c3, c4)),
            _ => Err(crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", seg_id, field_name),
                seg_id,
            )),
        }
    }
}

impl ParseSegmentField for (String, String, String, String, String) {
    fn parse_field(segment: &Segment, field_num: usize, seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep
                .get_component(0)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c1 = rep
                .get_component(1)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c2 = rep
                .get_component(2)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c3 = rep
                .get_component(3)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();
            let c4 = rep
                .get_component(4)
                .and_then(|c| c.value())
                .unwrap_or("")
                .to_string();

            if !c0.is_empty()
                && !c1.is_empty()
                && !c2.is_empty()
                && !c3.is_empty()
                && !c4.is_empty()
            {
                return Ok((c0, c1, c2, c3, c4));
            }
        }
        Err(crate::error::CustomSegmentError::missing_field(
            format!("{}-{}", seg_id, field_num),
            seg_id,
        ))
    }
}

impl SerializeSegmentField for (String, String, String, String, String) {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut rep = Repetition::new();
        rep.add_component(Component::from_value(&self.0));
        rep.add_component(Component::from_value(&self.1));
        rep.add_component(Component::from_value(&self.2));
        rep.add_component(Component::from_value(&self.3));
        rep.add_component(Component::from_value(&self.4));

        let mut field = Field::new();
        field.add_repetition(rep);
        let _ = segment.set_field(field_num, field);
    }
}

impl BuilderFieldType for (String, String, String, String, String) {
    type Inner = (String, String, String, String, String);
}

// ============================================================================
// Optional tuple types for optional component fields
// ============================================================================

// Option<(String, String)> - Optional 2-component field
impl BuildableField for Option<(String, String)> {
    type Storage = (Option<String>, Option<String>);
    type Inner = Option<(String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        if let Some((c0, c1)) = value {
            *storage = (Some(c0), Some(c1));
        } else {
            *storage = (None, None);
        }
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1)) => Ok(Some((c0, c1))),
            _ => Ok(None),
        }
    }
}

impl ParseSegmentField for Option<(String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep.get_component(0).and_then(|c| c.value());
            let c1 = rep.get_component(1).and_then(|c| c.value());

            if let (Some(v0), Some(v1)) = (c0, c1)
                && !v0.is_empty()
                && !v1.is_empty()
            {
                return Ok(Some((v0.to_string(), v1.to_string())));
            }
        }
        Ok(None)
    }
}

impl SerializeSegmentField for Option<(String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some((c0, c1)) = self {
            use vyna_hl7_core::{Component, Field, Repetition};

            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));

            let mut field = Field::new();
            field.add_repetition(rep);
            let _ = segment.set_field(field_num, field);
        }
    }
}

impl BuilderFieldType for Option<(String, String)> {
    type Inner = Option<(String, String)>;
}

// Option<(String, String, String)> - Optional 3-component field
impl BuildableField for Option<(String, String, String)> {
    type Storage = (Option<String>, Option<String>, Option<String>);
    type Inner = Option<(String, String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        if let Some((c0, c1, c2)) = value {
            *storage = (Some(c0), Some(c1), Some(c2));
        } else {
            *storage = (None, None, None);
        }
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1), Some(c2)) => Ok(Some((c0, c1, c2))),
            _ => Ok(None),
        }
    }
}

impl ParseSegmentField for Option<(String, String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep.get_component(0).and_then(|c| c.value());
            let c1 = rep.get_component(1).and_then(|c| c.value());
            let c2 = rep.get_component(2).and_then(|c| c.value());

            if let (Some(v0), Some(v1), Some(v2)) = (c0, c1, c2)
                && !v0.is_empty()
                && !v1.is_empty()
                && !v2.is_empty()
            {
                return Ok(Some((v0.to_string(), v1.to_string(), v2.to_string())));
            }
        }
        Ok(None)
    }
}

impl SerializeSegmentField for Option<(String, String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some((c0, c1, c2)) = self {
            use vyna_hl7_core::{Component, Field, Repetition};

            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));
            rep.add_component(Component::from_value(c2));

            let mut field = Field::new();
            field.add_repetition(rep);
            let _ = segment.set_field(field_num, field);
        }
    }
}

impl BuilderFieldType for Option<(String, String, String)> {
    type Inner = Option<(String, String, String)>;
}

// Option<(String, String, String, String)> - Optional 4-component field
impl BuildableField for Option<(String, String, String, String)> {
    type Storage = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    type Inner = Option<(String, String, String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        if let Some((c0, c1, c2, c3)) = value {
            *storage = (Some(c0), Some(c1), Some(c2), Some(c3));
        } else {
            *storage = (None, None, None, None);
        }
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1), Some(c2), Some(c3)) => Ok(Some((c0, c1, c2, c3))),
            _ => Ok(None),
        }
    }
}

impl ParseSegmentField for Option<(String, String, String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep.get_component(0).and_then(|c| c.value());
            let c1 = rep.get_component(1).and_then(|c| c.value());
            let c2 = rep.get_component(2).and_then(|c| c.value());
            let c3 = rep.get_component(3).and_then(|c| c.value());

            if let (Some(v0), Some(v1), Some(v2), Some(v3)) = (c0, c1, c2, c3)
                && !v0.is_empty()
                && !v1.is_empty()
                && !v2.is_empty()
                && !v3.is_empty()
            {
                return Ok(Some((
                    v0.to_string(),
                    v1.to_string(),
                    v2.to_string(),
                    v3.to_string(),
                )));
            }
        }
        Ok(None)
    }
}

impl SerializeSegmentField for Option<(String, String, String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some((c0, c1, c2, c3)) = self {
            use vyna_hl7_core::{Component, Field, Repetition};

            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));
            rep.add_component(Component::from_value(c2));
            rep.add_component(Component::from_value(c3));

            let mut field = Field::new();
            field.add_repetition(rep);
            let _ = segment.set_field(field_num, field);
        }
    }
}

impl BuilderFieldType for Option<(String, String, String, String)> {
    type Inner = Option<(String, String, String, String)>;
}

// Option<(String, String, String, String, String)> - Optional 5-component field
impl BuildableField for Option<(String, String, String, String, String)> {
    type Storage = (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    );
    type Inner = Option<(String, String, String, String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        if let Some((c0, c1, c2, c3, c4)) = value {
            *storage = (Some(c0), Some(c1), Some(c2), Some(c3), Some(c4));
        } else {
            *storage = (None, None, None, None, None);
        }
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        match storage {
            (Some(c0), Some(c1), Some(c2), Some(c3), Some(c4)) => Ok(Some((c0, c1, c2, c3, c4))),
            _ => Ok(None),
        }
    }
}

impl ParseSegmentField for Option<(String, String, String, String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num)
            && let Some(rep) = field.get_repetition(0)
        {
            let c0 = rep.get_component(0).and_then(|c| c.value());
            let c1 = rep.get_component(1).and_then(|c| c.value());
            let c2 = rep.get_component(2).and_then(|c| c.value());
            let c3 = rep.get_component(3).and_then(|c| c.value());
            let c4 = rep.get_component(4).and_then(|c| c.value());

            if let (Some(v0), Some(v1), Some(v2), Some(v3), Some(v4)) = (c0, c1, c2, c3, c4)
                && !v0.is_empty()
                && !v1.is_empty()
                && !v2.is_empty()
                && !v3.is_empty()
                && !v4.is_empty()
            {
                return Ok(Some((
                    v0.to_string(),
                    v1.to_string(),
                    v2.to_string(),
                    v3.to_string(),
                    v4.to_string(),
                )));
            }
        }
        Ok(None)
    }
}

impl SerializeSegmentField for Option<(String, String, String, String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        if let Some((c0, c1, c2, c3, c4)) = self {
            use vyna_hl7_core::{Component, Field, Repetition};

            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));
            rep.add_component(Component::from_value(c2));
            rep.add_component(Component::from_value(c3));
            rep.add_component(Component::from_value(c4));

            let mut field = Field::new();
            field.add_repetition(rep);
            let _ = segment.set_field(field_num, field);
        }
    }
}

impl BuilderFieldType for Option<(String, String, String, String, String)> {
    type Inner = Option<(String, String, String, String, String)>;
}

// ============================================================================
// Vec<Tuple> Implementations - Repeating Component Fields
// ============================================================================
//
// These implementations support repeating component fields in HL7, which combine
// both repetitions (~ separator) and components (^ separator).
//
// Example HL7 encoding: "555-1234^Home~555-5678^Work~555-9999^Mobile"
//                       └─ repetition 1 ┘ └─ repetition 2 ┘ └─ repetition 3 ┘
//                          └─ 2 components in each repetition
//
// Common use cases:
// - Multiple phone numbers with number and type
// - Multiple addresses with all address components
// - Multiple identifiers with ID, type, and authority
// ============================================================================

// Vec<(String, String)> - Multiple 2-component fields
impl ParseSegmentField for Vec<(String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let mut results = Vec::new();
            for rep in &field.repetitions {
                let c0 = rep.get_component(0).and_then(|c| c.value()).unwrap_or("");
                let c1 = rep.get_component(1).and_then(|c| c.value()).unwrap_or("");

                // Only include complete tuples (all components present)
                if !c0.is_empty() && !c1.is_empty() {
                    results.push((c0.to_string(), c1.to_string()));
                }
            }
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<(String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut field = Field::new();
        for (c0, c1) in self {
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));
            field.add_repetition(rep);
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuildableField for Vec<(String, String)> {
    type Storage = Vec<(String, String)>;
    type Inner = Vec<(String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl BuilderFieldType for Vec<(String, String)> {
    type Inner = Vec<(String, String)>;
}

// Vec<(String, String, String)> - Multiple 3-component fields
impl ParseSegmentField for Vec<(String, String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let mut results = Vec::new();
            for rep in &field.repetitions {
                let c0 = rep.get_component(0).and_then(|c| c.value()).unwrap_or("");
                let c1 = rep.get_component(1).and_then(|c| c.value()).unwrap_or("");
                let c2 = rep.get_component(2).and_then(|c| c.value()).unwrap_or("");

                // Only include complete tuples (all components present)
                if !c0.is_empty() && !c1.is_empty() && !c2.is_empty() {
                    results.push((c0.to_string(), c1.to_string(), c2.to_string()));
                }
            }
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<(String, String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut field = Field::new();
        for (c0, c1, c2) in self {
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));
            rep.add_component(Component::from_value(c2));
            field.add_repetition(rep);
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuildableField for Vec<(String, String, String)> {
    type Storage = Vec<(String, String, String)>;
    type Inner = Vec<(String, String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl BuilderFieldType for Vec<(String, String, String)> {
    type Inner = Vec<(String, String, String)>;
}

// Vec<(String, String, String, String)> - Multiple 4-component fields
impl ParseSegmentField for Vec<(String, String, String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let mut results = Vec::new();
            for rep in &field.repetitions {
                let c0 = rep.get_component(0).and_then(|c| c.value()).unwrap_or("");
                let c1 = rep.get_component(1).and_then(|c| c.value()).unwrap_or("");
                let c2 = rep.get_component(2).and_then(|c| c.value()).unwrap_or("");
                let c3 = rep.get_component(3).and_then(|c| c.value()).unwrap_or("");

                // Only include complete tuples (all components present)
                if !c0.is_empty() && !c1.is_empty() && !c2.is_empty() && !c3.is_empty() {
                    results.push((
                        c0.to_string(),
                        c1.to_string(),
                        c2.to_string(),
                        c3.to_string(),
                    ));
                }
            }
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<(String, String, String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut field = Field::new();
        for (c0, c1, c2, c3) in self {
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));
            rep.add_component(Component::from_value(c2));
            rep.add_component(Component::from_value(c3));
            field.add_repetition(rep);
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuildableField for Vec<(String, String, String, String)> {
    type Storage = Vec<(String, String, String, String)>;
    type Inner = Vec<(String, String, String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl BuilderFieldType for Vec<(String, String, String, String)> {
    type Inner = Vec<(String, String, String, String)>;
}

// Vec<(String, String, String, String, String)> - Multiple 5-component fields
impl ParseSegmentField for Vec<(String, String, String, String, String)> {
    fn parse_field(segment: &Segment, field_num: usize, _seg_id: &str) -> Result<Self> {
        if let Some(field) = segment.get_field(field_num) {
            let mut results = Vec::new();
            for rep in &field.repetitions {
                let c0 = rep.get_component(0).and_then(|c| c.value()).unwrap_or("");
                let c1 = rep.get_component(1).and_then(|c| c.value()).unwrap_or("");
                let c2 = rep.get_component(2).and_then(|c| c.value()).unwrap_or("");
                let c3 = rep.get_component(3).and_then(|c| c.value()).unwrap_or("");
                let c4 = rep.get_component(4).and_then(|c| c.value()).unwrap_or("");

                // Only include complete tuples (all components present)
                if !c0.is_empty()
                    && !c1.is_empty()
                    && !c2.is_empty()
                    && !c3.is_empty()
                    && !c4.is_empty()
                {
                    results.push((
                        c0.to_string(),
                        c1.to_string(),
                        c2.to_string(),
                        c3.to_string(),
                        c4.to_string(),
                    ));
                }
            }
            Ok(results)
        } else {
            Ok(Vec::new())
        }
    }
}

impl SerializeSegmentField for Vec<(String, String, String, String, String)> {
    fn set_field(&self, segment: &mut Segment, field_num: usize) {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut field = Field::new();
        for (c0, c1, c2, c3, c4) in self {
            let mut rep = Repetition::new();
            rep.add_component(Component::from_value(c0));
            rep.add_component(Component::from_value(c1));
            rep.add_component(Component::from_value(c2));
            rep.add_component(Component::from_value(c3));
            rep.add_component(Component::from_value(c4));
            field.add_repetition(rep);
        }
        let _ = segment.set_field(field_num, field);
    }
}

impl BuildableField for Vec<(String, String, String, String, String)> {
    type Storage = Vec<(String, String, String, String, String)>;
    type Inner = Vec<(String, String, String, String, String)>;

    fn set_value(storage: &mut Self::Storage, value: Self::Inner) {
        *storage = value;
    }

    fn build_value(storage: Self::Storage, _field_name: &str, _seg_id: &str) -> Result<Self> {
        Ok(storage)
    }
}

impl BuilderFieldType for Vec<(String, String, String, String, String)> {
    type Inner = Vec<(String, String, String, String, String)>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_definition_builder() {
        let field = FieldDefinition::builder()
            .number(1)
            .name("Visit Type")
            .data_type("ST")
            .required()
            .max_length(20)
            .description("Type of patient visit")
            .build();

        assert_eq!(field.number, 1);
        assert_eq!(field.name, "Visit Type");
        assert_eq!(field.data_type, "ST");
        assert!(field.required);
        assert!(!field.repeatable);
        assert_eq!(field.max_length, Some(20));
        assert_eq!(field.description, "Type of patient visit");
    }

    #[test]
    fn test_field_definition_optional() {
        let field = FieldDefinition::builder()
            .number(3)
            .name("Patient Class")
            .data_type("ST")
            .optional()
            .build();

        assert!(!field.required);
    }

    // Tests for i32 type
    #[test]
    fn test_i32_parse_positive() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "42").unwrap();

        let result = i32::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_i32_parse_negative() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "-123").unwrap();

        let result = i32::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, -123);
    }

    #[test]
    fn test_i32_parse_zero() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "0").unwrap();

        let result = i32::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_i32_serialize() {
        let mut segment = Segment::new("TEST");
        let value: i32 = -456;

        value.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "-456");
    }

    #[test]
    fn test_i32_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original: i32 = 789;

        original.set_field(&mut segment, 1);
        let parsed = i32::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_i32_parse_some() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "100").unwrap();

        let result = Option::<i32>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, Some(100));
    }

    #[test]
    fn test_option_i32_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<i32>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, None);
    }

    // Tests for i64 type
    #[test]
    fn test_i64_parse_large_positive() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "9223372036854775807").unwrap(); // i64::MAX

        let result = i64::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, 9223372036854775807);
    }

    #[test]
    fn test_i64_parse_large_negative() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "-9223372036854775808").unwrap(); // i64::MIN

        let result = i64::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, -9223372036854775808);
    }

    #[test]
    fn test_i64_serialize() {
        let mut segment = Segment::new("TEST");
        let value: i64 = 123456789012345;

        value.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "123456789012345");
    }

    #[test]
    fn test_i64_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original: i64 = -987654321098765;

        original.set_field(&mut segment, 1);
        let parsed = i64::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_i64_parse_some() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "555555").unwrap();

        let result = Option::<i64>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, Some(555555));
    }

    #[test]
    fn test_option_i64_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<i64>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, None);
    }

    // Tests for bool type - parsing all supported formats
    #[test]
    fn test_bool_parse_y() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "Y").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result);
    }

    #[test]
    fn test_bool_parse_yes() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "YES").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result);
    }

    #[test]
    fn test_bool_parse_t() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "T").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result);
    }

    #[test]
    fn test_bool_parse_true() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "TRUE").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result);
    }

    #[test]
    fn test_bool_parse_one() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "1").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result);
    }

    #[test]
    fn test_bool_parse_n() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "N").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_bool_parse_no() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "NO").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_bool_parse_f() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "F").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_bool_parse_false() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "FALSE").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_bool_parse_zero() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "0").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_bool_parse_case_insensitive() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "yes").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result);

        segment.set_field_value(1, "No").unwrap();
        let result = bool::parse_field(&segment, 1, "TEST").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_bool_serialize_true() {
        let mut segment = Segment::new("TEST");
        let value = true;

        value.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "Y");
    }

    #[test]
    fn test_bool_serialize_false() {
        let mut segment = Segment::new("TEST");
        let value = false;

        value.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "N");
    }

    #[test]
    fn test_bool_roundtrip_true() {
        let mut segment = Segment::new("TEST");
        let original = true;

        original.set_field(&mut segment, 1);
        let parsed = bool::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_bool_roundtrip_false() {
        let mut segment = Segment::new("TEST");
        let original = false;

        original.set_field(&mut segment, 1);
        let parsed = bool::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_bool_parse_some() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "Y").unwrap();

        let result = Option::<bool>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_option_bool_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<bool>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_bool_parse_invalid_value() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "INVALID").unwrap();

        let result = bool::parse_field(&segment, 1, "TEST");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_values_report_invalid_not_missing() {
        use crate::error::CustomSegmentError;

        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "ABC").unwrap();

        // Required scalars: present-but-unparseable is InvalidFieldValue.
        let err = f64::parse_field(&segment, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::InvalidFieldValue { .. }));
        let err = i32::parse_field(&segment, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::InvalidFieldValue { .. }));

        // Optional scalars: invalid is an error, not a silent None.
        let err = Option::<f64>::parse_field(&segment, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::InvalidFieldValue { .. }));

        // Absent fields are still MissingField (required) / None (optional).
        let empty = Segment::new("TEST");
        let err = f64::parse_field(&empty, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::MissingField { .. }));
        assert_eq!(Option::<f64>::parse_field(&empty, 1, "TEST").unwrap(), None);
    }

    #[test]
    fn test_bool_invalid_reports_invalid_not_missing() {
        use crate::error::CustomSegmentError;

        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "MAYBE").unwrap();

        let err = bool::parse_field(&segment, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::InvalidFieldValue { .. }));
        let err = Option::<bool>::parse_field(&segment, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::InvalidFieldValue { .. }));
    }

    #[test]
    fn test_datetime_invalid_reports_invalid_not_missing() {
        use crate::error::CustomSegmentError;

        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "not-a-date").unwrap();

        let err = NaiveDateTime::parse_field(&segment, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::InvalidFieldValue { .. }));
        let err = Option::<NaiveDate>::parse_field(&segment, 1, "TEST").unwrap_err();
        assert!(matches!(err, CustomSegmentError::InvalidFieldValue { .. }));
    }

    #[test]
    fn test_datetime_utc_applies_offset() {
        use chrono::TimeZone;

        // 12:00 at +0200 is 10:00 UTC.
        let dt = parse_hl7_datetime_utc("20240101120000+0200").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap());
        // 12:00 at -0500 is 17:00 UTC.
        let dt = parse_hl7_datetime_utc("20240101120000-0500").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 1, 17, 0, 0).unwrap());
        // No offset: interpreted as UTC (unchanged behavior).
        let dt = parse_hl7_datetime_utc("20240101120000").unwrap();
        assert_eq!(dt, Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap());
        // Malformed offsets are rejected, not silently ignored.
        assert!(parse_hl7_datetime_utc("20240101120000+9999").is_none());
        assert!(parse_hl7_datetime_utc("20240101120000+02AB").is_none());
    }

    // Tests for NaiveDateTime type
    #[test]
    fn test_naive_datetime_parse() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "20250119143000").unwrap(); // Jan 19, 2025 14:30:00

        let result = NaiveDateTime::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.format("%Y%m%d%H%M%S").to_string(), "20250119143000");
    }

    #[test]
    fn test_naive_datetime_serialize() {
        use chrono::NaiveDate;
        let mut segment = Segment::new("TEST");
        let dt = NaiveDate::from_ymd_opt(2025, 1, 19)
            .unwrap()
            .and_hms_opt(14, 30, 0)
            .unwrap();

        dt.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "20250119143000");
    }

    #[test]
    fn test_naive_datetime_roundtrip() {
        use chrono::NaiveDate;
        let mut segment = Segment::new("TEST");
        let original = NaiveDate::from_ymd_opt(2025, 12, 31)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();

        original.set_field(&mut segment, 1);
        let parsed = NaiveDateTime::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_naive_datetime_parse_some() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "20250101000000").unwrap();

        let result = Option::<NaiveDateTime>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_option_naive_datetime_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<NaiveDateTime>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, None);
    }

    // Tests for NaiveDate type
    #[test]
    fn test_naive_date_parse_yyyymmdd() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "20250119").unwrap(); // Jan 19, 2025

        let result = NaiveDate::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.format("%Y%m%d").to_string(), "20250119");
    }

    #[test]
    fn test_naive_date_parse_yyyymm() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "202501").unwrap(); // Jan 2025 (defaults to 1st)

        let result = NaiveDate::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.format("%Y%m%d").to_string(), "20250101");
    }

    #[test]
    fn test_naive_date_parse_yyyy() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "2025").unwrap(); // 2025 (defaults to Jan 1st)

        let result = NaiveDate::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.format("%Y%m%d").to_string(), "20250101");
    }

    #[test]
    fn test_naive_date_serialize() {
        let mut segment = Segment::new("TEST");
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

        date.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "20250615");
    }

    #[test]
    fn test_naive_date_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original = NaiveDate::from_ymd_opt(1980, 1, 15).unwrap();

        original.set_field(&mut segment, 1);
        let parsed = NaiveDate::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_naive_date_parse_some() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "20250119").unwrap();

        let result = Option::<NaiveDate>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_option_naive_date_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<NaiveDate>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, None);
    }

    // Tests for NaiveTime type
    #[test]
    fn test_naive_time_parse_hhmmss() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "143000").unwrap(); // 14:30:00

        let result = NaiveTime::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.format("%H%M%S").to_string(), "143000");
    }

    #[test]
    fn test_naive_time_parse_hhmm() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "1430").unwrap(); // 14:30 (seconds default to 00)

        let result = NaiveTime::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.format("%H%M%S").to_string(), "143000");
    }

    #[test]
    fn test_naive_time_serialize() {
        let mut segment = Segment::new("TEST");
        let time = NaiveTime::from_hms_opt(23, 59, 59).unwrap();

        time.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "235959");
    }

    #[test]
    fn test_naive_time_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original = NaiveTime::from_hms_opt(8, 15, 30).unwrap();

        original.set_field(&mut segment, 1);
        let parsed = NaiveTime::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_naive_time_parse_some() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "120000").unwrap();

        let result = Option::<NaiveTime>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_option_naive_time_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<NaiveTime>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, None);
    }

    // Tests for DateTime<Utc> type
    #[test]
    fn test_datetime_utc_parse() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "20250119143000").unwrap();

        let result = DateTime::<Utc>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.format("%Y%m%d%H%M%S").to_string(), "20250119143000");
    }

    #[test]
    fn test_datetime_utc_serialize() {
        use chrono::TimeZone;
        let mut segment = Segment::new("TEST");
        let dt = Utc.with_ymd_and_hms(2025, 1, 19, 14, 30, 0).unwrap();

        dt.set_field(&mut segment, 1);
        assert_eq!(segment.get_field_value(1).unwrap(), "20250119143000");
    }

    #[test]
    fn test_datetime_utc_roundtrip() {
        use chrono::TimeZone;
        let mut segment = Segment::new("TEST");
        let original = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();

        original.set_field(&mut segment, 1);
        let parsed = DateTime::<Utc>::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_datetime_utc_parse_some() {
        let mut segment = Segment::new("TEST");
        segment.set_field_value(1, "20250101000000").unwrap();

        let result = Option::<DateTime<Utc>>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_option_datetime_utc_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<DateTime<Utc>>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, None);
    }

    // Tests for HL7 datetime parsing helper functions
    #[test]
    fn test_parse_hl7_datetime_valid() {
        let result = parse_hl7_datetime("20250119143000").unwrap();
        assert_eq!(result.format("%Y%m%d%H%M%S").to_string(), "20250119143000");
    }

    #[test]
    fn test_parse_hl7_datetime_invalid() {
        assert!(parse_hl7_datetime("invalid").is_none());
        assert!(parse_hl7_datetime("2025").is_none());
        assert!(parse_hl7_datetime("20250119").is_none()); // Date only, not datetime
    }

    #[test]
    fn test_parse_hl7_date_formats() {
        // YYYYMMDD
        let date1 = parse_hl7_date("20250119").unwrap();
        assert_eq!(date1.format("%Y%m%d").to_string(), "20250119");

        // YYYYMM
        let date2 = parse_hl7_date("202501").unwrap();
        assert_eq!(date2.format("%Y%m%d").to_string(), "20250101");

        // YYYY
        let date3 = parse_hl7_date("2025").unwrap();
        assert_eq!(date3.format("%Y%m%d").to_string(), "20250101");
    }

    #[test]
    fn test_parse_hl7_time_formats() {
        // HHMMSS
        let time1 = parse_hl7_time("143000").unwrap();
        assert_eq!(time1.format("%H%M%S").to_string(), "143000");

        // HHMM
        let time2 = parse_hl7_time("1430").unwrap();
        assert_eq!(time2.format("%H%M%S").to_string(), "143000");
    }

    #[test]
    fn test_parse_hl7_time_invalid() {
        assert!(parse_hl7_time("invalid").is_none());
        assert!(parse_hl7_time("14").is_none());
        assert!(parse_hl7_time("999999").is_none()); // Invalid hour
    }

    // Tests for Vec<String> repeating fields
    #[test]
    fn test_vec_string_parse_multiple() {
        use vyna_hl7_core::{Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("value1"));
        field.add_repetition(Repetition::from_value("value2"));
        field.add_repetition(Repetition::from_value("value3"));
        segment.set_field(1, field).unwrap();

        let result = Vec::<String>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, vec!["value1", "value2", "value3"]);
    }

    #[test]
    fn test_vec_string_parse_empty() {
        let segment = Segment::new("TEST");

        let result = Vec::<String>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn test_vec_string_serialize() {
        let mut segment = Segment::new("TEST");
        let values = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];

        values.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        assert_eq!(field.repetitions.len(), 3);
        assert_eq!(field.repetitions[0].value().unwrap(), "alpha");
        assert_eq!(field.repetitions[1].value().unwrap(), "beta");
        assert_eq!(field.repetitions[2].value().unwrap(), "gamma");
    }

    #[test]
    fn test_vec_string_roundtrip() {
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("TEST");
        let original = vec!["phone1".to_string(), "phone2".to_string()];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<String>::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);

        // Verify HL7 encoding uses ~ separator
        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);
        assert!(encoded.contains("phone1~phone2"));
    }

    // Tests for Vec<u32> repeating fields
    #[test]
    fn test_vec_u32_parse_multiple() {
        use vyna_hl7_core::{Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("10"));
        field.add_repetition(Repetition::from_value("20"));
        field.add_repetition(Repetition::from_value("30"));
        segment.set_field(1, field).unwrap();

        let result = Vec::<u32>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, vec![10, 20, 30]);
    }

    #[test]
    fn test_vec_u32_parse_empty() {
        let segment = Segment::new("TEST");

        let result = Vec::<u32>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, Vec::<u32>::new());
    }

    #[test]
    fn test_vec_u32_serialize() {
        let mut segment = Segment::new("TEST");
        let values = vec![100, 200, 300];

        values.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        assert_eq!(field.repetitions.len(), 3);
        assert_eq!(field.repetitions[0].value().unwrap(), "100");
        assert_eq!(field.repetitions[1].value().unwrap(), "200");
        assert_eq!(field.repetitions[2].value().unwrap(), "300");
    }

    #[test]
    fn test_vec_u32_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original = vec![42, 99, 1000];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<u32>::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    // Tests for Vec<i32> repeating fields
    #[test]
    fn test_vec_i32_parse_with_negatives() {
        use vyna_hl7_core::{Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("-10"));
        field.add_repetition(Repetition::from_value("0"));
        field.add_repetition(Repetition::from_value("10"));
        segment.set_field(1, field).unwrap();

        let result = Vec::<i32>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, vec![-10, 0, 10]);
    }

    #[test]
    fn test_vec_i32_serialize() {
        let mut segment = Segment::new("TEST");
        let values = vec![-100, 50, -75];

        values.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        assert_eq!(field.repetitions[0].value().unwrap(), "-100");
        assert_eq!(field.repetitions[1].value().unwrap(), "50");
        assert_eq!(field.repetitions[2].value().unwrap(), "-75");
    }

    #[test]
    fn test_vec_i32_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original = vec![-456, 0, 789];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<i32>::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    // Tests for Vec<i64> repeating fields
    #[test]
    fn test_vec_i64_parse_large_numbers() {
        use vyna_hl7_core::{Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("9223372036854775807")); // i64::MAX
        field.add_repetition(Repetition::from_value("-9223372036854775808")); // i64::MIN
        field.add_repetition(Repetition::from_value("0"));
        segment.set_field(1, field).unwrap();

        let result = Vec::<i64>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, vec![9223372036854775807, -9223372036854775808, 0]);
    }

    #[test]
    fn test_vec_i64_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original = vec![1000000000000i64, -1000000000000i64];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<i64>::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    // Tests for Vec<f64> repeating fields
    // Allow: 3.14/2.718 are truncated test inputs, not PI/E approximations.
    #[allow(clippy::approx_constant)]
    #[test]
    fn test_vec_f64_parse_decimals() {
        use vyna_hl7_core::{Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("3.14"));
        field.add_repetition(Repetition::from_value("2.718"));
        field.add_repetition(Repetition::from_value("-0.5"));
        segment.set_field(1, field).unwrap();

        let result = Vec::<f64>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[0] - 3.14).abs() < 0.001);
        assert!((result[1] - 2.718).abs() < 0.001);
        assert!((result[2] + 0.5).abs() < 0.001);
    }

    #[test]
    fn test_vec_f64_serialize() {
        let mut segment = Segment::new("TEST");
        let values = vec![123.45, 678.90, -99.99];

        values.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        assert_eq!(field.repetitions.len(), 3);
        assert_eq!(field.repetitions[0].value().unwrap(), "123.45");
        assert_eq!(field.repetitions[1].value().unwrap(), "678.9");
        assert_eq!(field.repetitions[2].value().unwrap(), "-99.99");
    }

    #[test]
    fn test_vec_f64_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original = vec![1.5, 2.5, 3.5];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<f64>::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original.len(), parsed.len());
        for (a, b) in original.iter().zip(parsed.iter()) {
            assert!((a - b).abs() < 0.0001);
        }
    }

    // Tests for Vec<bool> repeating fields
    #[test]
    fn test_vec_bool_parse_yn_values() {
        use vyna_hl7_core::{Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("Y"));
        field.add_repetition(Repetition::from_value("N"));
        field.add_repetition(Repetition::from_value("TRUE"));
        field.add_repetition(Repetition::from_value("FALSE"));
        segment.set_field(1, field).unwrap();

        let result = Vec::<bool>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, vec![true, false, true, false]);
    }

    #[test]
    fn test_vec_bool_parse_numeric_values() {
        use vyna_hl7_core::{Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        field.add_repetition(Repetition::from_value("1"));
        field.add_repetition(Repetition::from_value("0"));
        field.add_repetition(Repetition::from_value("1"));
        segment.set_field(1, field).unwrap();

        let result = Vec::<bool>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result, vec![true, false, true]);
    }

    #[test]
    fn test_vec_bool_serialize() {
        let mut segment = Segment::new("TEST");
        let values = vec![true, false, true, true, false];

        values.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        assert_eq!(field.repetitions.len(), 5);
        assert_eq!(field.repetitions[0].value().unwrap(), "Y");
        assert_eq!(field.repetitions[1].value().unwrap(), "N");
        assert_eq!(field.repetitions[2].value().unwrap(), "Y");
        assert_eq!(field.repetitions[3].value().unwrap(), "Y");
        assert_eq!(field.repetitions[4].value().unwrap(), "N");
    }

    #[test]
    fn test_vec_bool_roundtrip() {
        let mut segment = Segment::new("TEST");
        let original = vec![true, false, false, true];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<bool>::parse_field(&segment, 1, "TEST").unwrap();

        assert_eq!(original, parsed);
    }

    // Test HL7 encoding with ~ separator
    #[test]
    fn test_vec_hl7_encoding_with_tilde_separator() {
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("ZRP");

        // Add multiple string values
        let phones = vec![
            "555-1234".to_string(),
            "555-5678".to_string(),
            "555-9999".to_string(),
        ];
        phones.set_field(&mut segment, 1);

        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);

        // Verify the encoded string contains ~ separators
        assert!(encoded.contains("555-1234~555-5678~555-9999"));
    }

    #[test]
    fn test_vec_empty_serializes_correctly() {
        let mut segment = Segment::new("TEST");
        let empty: Vec<String> = vec![];

        empty.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        assert_eq!(field.repetitions.len(), 0);
    }

    // Tests for tuple component types
    #[test]
    fn test_tuple2_parse() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result = <(String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.0, "Smith");
        assert_eq!(result.1, "John");
    }

    #[test]
    fn test_tuple2_serialize() {
        let mut segment = Segment::new("TEST");
        let value = ("Smith".to_string(), "John".to_string());

        value.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.get_component(0).unwrap().value().unwrap(), "Smith");
        assert_eq!(rep.get_component(1).unwrap().value().unwrap(), "John");
    }

    #[test]
    fn test_tuple2_roundtrip() {
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("ZPN");
        let original = ("Doe".to_string(), "Jane".to_string());

        original.set_field(&mut segment, 1);
        let parsed = <(String, String)>::parse_field(&segment, 1, "ZPN").unwrap();

        assert_eq!(original, parsed);

        // Verify HL7 encoding uses ^ separator
        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);
        assert!(encoded.contains("Doe^Jane"));
    }

    #[test]
    fn test_tuple3_parse() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        rep.add_component(Component::from_value("A"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result = <(String, String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.0, "Smith");
        assert_eq!(result.1, "John");
        assert_eq!(result.2, "A");
    }

    #[test]
    fn test_tuple3_serialize() {
        let mut segment = Segment::new("TEST");
        let value = ("Smith".to_string(), "John".to_string(), "A".to_string());

        value.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.get_component(0).unwrap().value().unwrap(), "Smith");
        assert_eq!(rep.get_component(1).unwrap().value().unwrap(), "John");
        assert_eq!(rep.get_component(2).unwrap().value().unwrap(), "A");
    }

    #[test]
    fn test_tuple3_roundtrip() {
        let mut segment = Segment::new("ZPN");
        let original = ("Doe".to_string(), "Jane".to_string(), "Marie".to_string());

        original.set_field(&mut segment, 1);
        let parsed = <(String, String, String)>::parse_field(&segment, 1, "ZPN").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_tuple4_parse() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        rep.add_component(Component::from_value("A"));
        rep.add_component(Component::from_value("Jr"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result = <(String, String, String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.0, "Smith");
        assert_eq!(result.1, "John");
        assert_eq!(result.2, "A");
        assert_eq!(result.3, "Jr");
    }

    #[test]
    fn test_tuple4_serialize() {
        let mut segment = Segment::new("TEST");
        let value = (
            "Smith".to_string(),
            "John".to_string(),
            "A".to_string(),
            "Jr".to_string(),
        );

        value.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.get_component(0).unwrap().value().unwrap(), "Smith");
        assert_eq!(rep.get_component(1).unwrap().value().unwrap(), "John");
        assert_eq!(rep.get_component(2).unwrap().value().unwrap(), "A");
        assert_eq!(rep.get_component(3).unwrap().value().unwrap(), "Jr");
    }

    #[test]
    fn test_tuple4_roundtrip() {
        let mut segment = Segment::new("ZPN");
        let original = (
            "Doe".to_string(),
            "Jane".to_string(),
            "Marie".to_string(),
            "III".to_string(),
        );

        original.set_field(&mut segment, 1);
        let parsed = <(String, String, String, String)>::parse_field(&segment, 1, "ZPN").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_tuple5_parse() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        rep.add_component(Component::from_value("A"));
        rep.add_component(Component::from_value("Jr"));
        rep.add_component(Component::from_value("Dr"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result =
            <(String, String, String, String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert_eq!(result.0, "Smith");
        assert_eq!(result.1, "John");
        assert_eq!(result.2, "A");
        assert_eq!(result.3, "Jr");
        assert_eq!(result.4, "Dr");
    }

    #[test]
    fn test_tuple5_serialize() {
        let mut segment = Segment::new("TEST");
        let value = (
            "Smith".to_string(),
            "John".to_string(),
            "A".to_string(),
            "Jr".to_string(),
            "Dr".to_string(),
        );

        value.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.get_component(0).unwrap().value().unwrap(), "Smith");
        assert_eq!(rep.get_component(1).unwrap().value().unwrap(), "John");
        assert_eq!(rep.get_component(2).unwrap().value().unwrap(), "A");
        assert_eq!(rep.get_component(3).unwrap().value().unwrap(), "Jr");
        assert_eq!(rep.get_component(4).unwrap().value().unwrap(), "Dr");
    }

    #[test]
    fn test_tuple5_roundtrip() {
        let mut segment = Segment::new("ZPN");
        let original = (
            "Doe".to_string(),
            "Jane".to_string(),
            "Marie".to_string(),
            "III".to_string(),
            "Prof".to_string(),
        );

        original.set_field(&mut segment, 1);
        let parsed =
            <(String, String, String, String, String)>::parse_field(&segment, 1, "ZPN").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_tuple_hl7_encoding_with_caret_separator() {
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("ZPN");

        // Patient name: Last^First^Middle^Suffix^Prefix
        let name = (
            "Smith".to_string(),
            "John".to_string(),
            "Alexander".to_string(),
            "Jr".to_string(),
            "Dr".to_string(),
        );
        name.set_field(&mut segment, 1);

        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);

        // Verify the encoded string contains ^ separators
        assert!(encoded.contains("Smith^John^Alexander^Jr^Dr"));
    }

    // Tests for optional tuple component types
    #[test]
    fn test_option_tuple2_parse_some() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result = Option::<(String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_some());
        let (c0, c1) = result.unwrap();
        assert_eq!(c0, "Smith");
        assert_eq!(c1, "John");
    }

    #[test]
    fn test_option_tuple2_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<(String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_option_tuple2_serialize_some() {
        let mut segment = Segment::new("TEST");
        let value = Some(("Doe".to_string(), "Jane".to_string()));

        value.set_field(&mut segment, 1);

        let field = segment.get_field(1).unwrap();
        let rep = field.get_repetition(0).unwrap();
        assert_eq!(rep.get_component(0).unwrap().value().unwrap(), "Doe");
        assert_eq!(rep.get_component(1).unwrap().value().unwrap(), "Jane");
    }

    #[test]
    fn test_option_tuple2_serialize_none() {
        let mut segment = Segment::new("TEST");
        let value: Option<(String, String)> = None;

        value.set_field(&mut segment, 1);

        // Should not create a field
        assert!(segment.get_field(1).is_none() || segment.get_field(1).unwrap().is_empty());
    }

    #[test]
    fn test_option_tuple2_roundtrip() {
        let mut segment = Segment::new("ZOP");
        let original = Some(("Brown".to_string(), "Alice".to_string()));

        original.set_field(&mut segment, 1);
        let parsed = Option::<(String, String)>::parse_field(&segment, 1, "ZOP").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_tuple3_parse_some() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        rep.add_component(Component::from_value("A"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result = Option::<(String, String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_some());
        let (c0, c1, c2) = result.unwrap();
        assert_eq!(c0, "Smith");
        assert_eq!(c1, "John");
        assert_eq!(c2, "A");
    }

    #[test]
    fn test_option_tuple3_parse_none() {
        let segment = Segment::new("TEST");

        let result = Option::<(String, String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_option_tuple3_roundtrip() {
        let mut segment = Segment::new("ZOP");
        let original = Some(("Doe".to_string(), "Jane".to_string(), "Marie".to_string()));

        original.set_field(&mut segment, 1);
        let parsed = Option::<(String, String, String)>::parse_field(&segment, 1, "ZOP").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_tuple4_parse_some() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        rep.add_component(Component::from_value("A"));
        rep.add_component(Component::from_value("Jr"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result =
            Option::<(String, String, String, String)>::parse_field(&segment, 1, "TEST").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_option_tuple4_roundtrip() {
        let mut segment = Segment::new("ZOP");
        let original = Some((
            "Williams".to_string(),
            "Sarah".to_string(),
            "Jane".to_string(),
            "III".to_string(),
        ));

        original.set_field(&mut segment, 1);
        let parsed =
            Option::<(String, String, String, String)>::parse_field(&segment, 1, "ZOP").unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_tuple5_parse_some() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("TEST");
        let mut field = Field::new();
        let mut rep = Repetition::new();
        rep.add_component(Component::from_value("Smith"));
        rep.add_component(Component::from_value("John"));
        rep.add_component(Component::from_value("A"));
        rep.add_component(Component::from_value("Jr"));
        rep.add_component(Component::from_value("Dr"));
        field.add_repetition(rep);
        segment.set_field(1, field).unwrap();

        let result =
            Option::<(String, String, String, String, String)>::parse_field(&segment, 1, "TEST")
                .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_option_tuple5_roundtrip() {
        let mut segment = Segment::new("ZOP");
        let original = Some((
            "Brown".to_string(),
            "Robert".to_string(),
            "James".to_string(),
            "Sr".to_string(),
            "Prof".to_string(),
        ));

        original.set_field(&mut segment, 1);
        let parsed =
            Option::<(String, String, String, String, String)>::parse_field(&segment, 1, "ZOP")
                .unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_option_tuple_builder_with_some() {
        // This test verifies optional component fields work in the builder pattern
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("ZOP");
        let value = Some(("Test".to_string(), "Value".to_string()));

        value.set_field(&mut segment, 1);

        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);

        assert!(encoded.contains("Test^Value"));
    }

    #[test]
    fn test_option_tuple_builder_with_none() {
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("ZOP");
        let value: Option<(String, String)> = None;

        value.set_field(&mut segment, 1);

        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);

        // When None, should not add components
        assert!(!encoded.contains("^"));
    }

    // ========================================================================
    // Vec<Tuple> Tests - Repeating Component Fields
    // ========================================================================

    #[test]
    fn test_vec_tuple2_parse_multiple() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("ZVT");

        // Create field with multiple 2-component repetitions
        let mut field = Field::new();

        let mut rep1 = Repetition::new();
        rep1.add_component(Component::from_value("555-1234"));
        rep1.add_component(Component::from_value("Home"));
        field.add_repetition(rep1);

        let mut rep2 = Repetition::new();
        rep2.add_component(Component::from_value("555-5678"));
        rep2.add_component(Component::from_value("Work"));
        field.add_repetition(rep2);

        let mut rep3 = Repetition::new();
        rep3.add_component(Component::from_value("555-9999"));
        rep3.add_component(Component::from_value("Mobile"));
        field.add_repetition(rep3);

        segment.set_field(1, field).unwrap();

        let result = Vec::<(String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], ("555-1234".to_string(), "Home".to_string()));
        assert_eq!(result[1], ("555-5678".to_string(), "Work".to_string()));
        assert_eq!(result[2], ("555-9999".to_string(), "Mobile".to_string()));
    }

    #[test]
    fn test_vec_tuple2_serialize() {
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("ZVT");
        let phones = vec![
            ("555-1234".to_string(), "Home".to_string()),
            ("555-5678".to_string(), "Work".to_string()),
        ];

        phones.set_field(&mut segment, 1);

        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);

        assert!(encoded.contains("555-1234^Home~555-5678^Work"));
    }

    #[test]
    fn test_vec_tuple2_roundtrip() {
        let mut segment = Segment::new("ZVT");
        let original = vec![
            ("AAA".to_string(), "BBB".to_string()),
            ("CCC".to_string(), "DDD".to_string()),
            ("EEE".to_string(), "FFF".to_string()),
        ];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<(String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_vec_tuple3_parse_multiple() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("ZVT");

        let mut field = Field::new();

        let mut rep1 = Repetition::new();
        rep1.add_component(Component::from_value("123 Main"));
        rep1.add_component(Component::from_value("Springfield"));
        rep1.add_component(Component::from_value("IL"));
        field.add_repetition(rep1);

        let mut rep2 = Repetition::new();
        rep2.add_component(Component::from_value("456 Oak"));
        rep2.add_component(Component::from_value("Chicago"));
        rep2.add_component(Component::from_value("IL"));
        field.add_repetition(rep2);

        segment.set_field(1, field).unwrap();

        let result = Vec::<(String, String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            (
                "123 Main".to_string(),
                "Springfield".to_string(),
                "IL".to_string()
            )
        );
        assert_eq!(
            result[1],
            (
                "456 Oak".to_string(),
                "Chicago".to_string(),
                "IL".to_string()
            )
        );
    }

    #[test]
    fn test_vec_tuple3_roundtrip() {
        let mut segment = Segment::new("ZVT");
        let original = vec![
            ("Smith".to_string(), "John".to_string(), "MD".to_string()),
            ("Doe".to_string(), "Jane".to_string(), "PhD".to_string()),
        ];

        original.set_field(&mut segment, 1);
        let parsed = Vec::<(String, String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_vec_tuple4_parse_multiple() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("ZVT");

        let mut field = Field::new();

        let mut rep1 = Repetition::new();
        rep1.add_component(Component::from_value("MRN123"));
        rep1.add_component(Component::from_value("MRN"));
        rep1.add_component(Component::from_value("Hospital"));
        rep1.add_component(Component::from_value("Main"));
        field.add_repetition(rep1);

        let mut rep2 = Repetition::new();
        rep2.add_component(Component::from_value("ACC456"));
        rep2.add_component(Component::from_value("Account"));
        rep2.add_component(Component::from_value("Billing"));
        rep2.add_component(Component::from_value("West"));
        field.add_repetition(rep2);

        segment.set_field(1, field).unwrap();

        let result =
            Vec::<(String, String, String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            (
                "MRN123".to_string(),
                "MRN".to_string(),
                "Hospital".to_string(),
                "Main".to_string()
            )
        );
        assert_eq!(
            result[1],
            (
                "ACC456".to_string(),
                "Account".to_string(),
                "Billing".to_string(),
                "West".to_string()
            )
        );
    }

    #[test]
    fn test_vec_tuple4_roundtrip() {
        let mut segment = Segment::new("ZVT");
        let original = vec![
            (
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
            ),
            (
                "E".to_string(),
                "F".to_string(),
                "G".to_string(),
                "H".to_string(),
            ),
        ];

        original.set_field(&mut segment, 1);
        let parsed =
            Vec::<(String, String, String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_vec_tuple5_parse_multiple() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("ZVT");

        let mut field = Field::new();

        let mut rep1 = Repetition::new();
        rep1.add_component(Component::from_value("123 Main"));
        rep1.add_component(Component::from_value("Springfield"));
        rep1.add_component(Component::from_value("IL"));
        rep1.add_component(Component::from_value("62701"));
        rep1.add_component(Component::from_value("USA"));
        field.add_repetition(rep1);

        let mut rep2 = Repetition::new();
        rep2.add_component(Component::from_value("456 Oak"));
        rep2.add_component(Component::from_value("Chicago"));
        rep2.add_component(Component::from_value("IL"));
        rep2.add_component(Component::from_value("60601"));
        rep2.add_component(Component::from_value("USA"));
        field.add_repetition(rep2);

        segment.set_field(1, field).unwrap();

        let result =
            Vec::<(String, String, String, String, String)>::parse_field(&segment, 1, "ZVT")
                .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            (
                "123 Main".to_string(),
                "Springfield".to_string(),
                "IL".to_string(),
                "62701".to_string(),
                "USA".to_string()
            )
        );
        assert_eq!(
            result[1],
            (
                "456 Oak".to_string(),
                "Chicago".to_string(),
                "IL".to_string(),
                "60601".to_string(),
                "USA".to_string()
            )
        );
    }

    #[test]
    fn test_vec_tuple5_roundtrip() {
        let mut segment = Segment::new("ZVT");
        let original = vec![
            (
                "Williams".to_string(),
                "Sarah".to_string(),
                "Jane".to_string(),
                "MD".to_string(),
                "Dr".to_string(),
            ),
            (
                "Brown".to_string(),
                "Robert".to_string(),
                "Lee".to_string(),
                "PhD".to_string(),
                "Prof".to_string(),
            ),
        ];

        original.set_field(&mut segment, 1);
        let parsed =
            Vec::<(String, String, String, String, String)>::parse_field(&segment, 1, "ZVT")
                .unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn test_vec_tuple_empty() {
        let segment = Segment::new("ZVT");

        let result = Vec::<(String, String)>::parse_field(&segment, 1, "ZVT").unwrap();
        assert_eq!(result.len(), 0);

        let result3 = Vec::<(String, String, String)>::parse_field(&segment, 1, "ZVT").unwrap();
        assert_eq!(result3.len(), 0);
    }

    #[test]
    fn test_vec_tuple_hl7_encoding_with_both_separators() {
        use vyna_hl7_core::Delimiters;

        let mut segment = Segment::new("ZVT");
        let phones = vec![
            ("555-1111".to_string(), "Home".to_string()),
            ("555-2222".to_string(), "Work".to_string()),
            ("555-3333".to_string(), "Mobile".to_string()),
        ];

        phones.set_field(&mut segment, 1);

        let delimiters = Delimiters::default();
        let encoded = segment.encode(&delimiters);

        // Should have both ~ (repetition separator) and ^ (component separator)
        assert!(encoded.contains("~"));
        assert!(encoded.contains("^"));
        assert!(encoded.contains("555-1111^Home"));
        assert!(encoded.contains("555-2222^Work"));
        assert!(encoded.contains("555-3333^Mobile"));
    }

    #[test]
    fn test_vec_tuple_incomplete_components_skipped() {
        use vyna_hl7_core::{Component, Field, Repetition};

        let mut segment = Segment::new("ZVT");

        // Mix of complete and incomplete tuples
        let mut field = Field::new();

        // First tuple complete (AAA^BBB)
        let mut rep1 = Repetition::new();
        rep1.add_component(Component::from_value("AAA"));
        rep1.add_component(Component::from_value("BBB"));
        field.add_repetition(rep1);

        // Second tuple incomplete (CCC only - missing second component)
        let mut rep2 = Repetition::new();
        rep2.add_component(Component::from_value("CCC"));
        field.add_repetition(rep2);

        // Third tuple complete (DDD^EEE)
        let mut rep3 = Repetition::new();
        rep3.add_component(Component::from_value("DDD"));
        rep3.add_component(Component::from_value("EEE"));
        field.add_repetition(rep3);

        segment.set_field(1, field).unwrap();

        let result = Vec::<(String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        // Should only include complete tuples (AAA^BBB and DDD^EEE)
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], ("AAA".to_string(), "BBB".to_string()));
        assert_eq!(result[1], ("DDD".to_string(), "EEE".to_string()));
    }

    #[test]
    fn test_vec_tuple_builder() {
        let mut segment = Segment::new("ZVT");
        let phones = vec![
            ("555-0001".to_string(), "Primary".to_string()),
            ("555-0002".to_string(), "Secondary".to_string()),
        ];

        phones.set_field(&mut segment, 1);
        let parsed = Vec::<(String, String)>::parse_field(&segment, 1, "ZVT").unwrap();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "555-0001");
        assert_eq!(parsed[0].1, "Primary");
        assert_eq!(parsed[1].0, "555-0002");
        assert_eq!(parsed[1].1, "Secondary");
    }

    #[test]
    fn test_datetime_helpers_reject_non_ascii_without_panic() {
        // Byte lengths match the slicing paths but land mid-character.
        assert_eq!(parse_hl7_datetime("1234567é901234"), None);
        assert!(parse_hl7_datetime("20240315143045").is_some());
        assert_eq!(parse_hl7_date("123456é"), None);
        assert!(parse_hl7_date("20240315").is_some());
        assert_eq!(parse_hl7_time("1234é"), None);
        assert!(parse_hl7_time("143045").is_some());
    }
}
