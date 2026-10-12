//! Declarative macros for defining Z-segments
//!
//! This module provides the `z_segment!` macro for easily defining custom Z-segments
//! with automatic implementation of the `CustomSegment` trait and builder pattern.

/// Define a custom Z-segment with fluent builder API
///
/// This macro generates:
/// - A struct with the specified fields
/// - Implementation of `CustomSegment` trait
/// - A builder struct with fluent API
/// - Conversion methods
///
/// # Basic Usage
///
/// ```rust,ignore
/// use vyna_hl7_custom::z_segment;
///
/// z_segment! {
///     ZPV,              // Struct name
///     id = "ZPV",       // Segment ID
///     fields = {
///         1 => visit_type: String,
///         2 => visit_number: String,
///         3 => patient_class: Option<String>,
///     }
/// }
///
/// // Use it:
/// let zpv = ZPV::builder()
///     .visit_type("OUTPATIENT")
///     .visit_number("V12345")
///     .patient_class("E")
///     .build()?;
/// ```
///
/// # With Validation
///
/// ```rust,ignore
/// z_segment! {
///     ZCU,
///     id = "ZCU",
///     fields = {
///         1 => customer_id: String,
///         2 => balance: Option<f64>,
///     },
///     validate = |s: &ZCU| {
///         if let Some(balance) = s.balance {
///             if balance < 0.0 {
///                 return Err(CustomSegmentError::validation_failed(
///                     "ZCU",
///                     "Balance cannot be negative"
///                 ));
///             }
///         }
///         Ok(())
///     }
/// }
/// ```
#[macro_export]
macro_rules! z_segment {
    // Pattern: Basic segment without validation
    (
        $name:ident,
        id = $id:expr,
        fields = {
            $($field_num:literal => $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::z_segment! {
            $name,
            id = $id,
            fields = {
                $($field_num => $field_name : $field_type),*
            },
            validate = |_: &$name| Ok(())
        }
    };

    // Pattern: Segment with validation
    (
        $name:ident,
        id = $id:expr,
        fields = {
            $($field_num:literal => $field_name:ident : $field_type:ty),* $(,)?
        },
        validate = $validate_fn:expr
    ) => {
        /// Custom Z-segment
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $(
                pub $field_name: $field_type,
            )*
        }

        paste::paste! {
            impl $name {
                /// Create a new builder for this segment
                #[allow(dead_code)]
                pub fn builder() -> [<$name Builder>] {
                    [<$name Builder>]::default()
                }
            }
        }

        impl $crate::segment::CustomSegment for $name {
            fn segment_id() -> &'static str {
                $id
            }

            fn from_segment(segment: &vyna_hl7_core::Segment) -> $crate::error::Result<Self> {
                use $crate::segment::ParseSegmentField;

                Ok($name {
                    $(
                        $field_name: ParseSegmentField::parse_field(segment, $field_num, $id)?,
                    )*
                })
            }

            fn to_segment(&self) -> vyna_hl7_core::Segment {
                use $crate::segment::SerializeSegmentField;
                let mut segment = vyna_hl7_core::Segment::new($id);
                $(
                    SerializeSegmentField::set_field(&self.$field_name, &mut segment, $field_num);
                )*
                segment
            }

            fn validate(&self) -> $crate::error::Result<()> {
                let validate_fn = $validate_fn;
                validate_fn(self)
            }
        }

        paste::paste! {
            /// Builder for the custom segment
            #[allow(dead_code)]
            #[derive(Debug, Default)]
            pub struct [<$name Builder>] {
                $(
                    $field_name: $crate::segment::BuilderField<$field_type>,
                )*
            }

            #[allow(dead_code)]
            impl [<$name Builder>] {
                $(
                    /// Set field value
                    pub fn $field_name(mut self, value: impl Into<<$field_type as $crate::segment::BuildableField>::Inner>) -> Self {
                        self.$field_name.set(value.into());
                        self
                    }
                )*

                /// Build the segment
                pub fn build(self) -> $crate::error::Result<$name> {
                    let segment = $name {
                        $(
                            $field_name: self.$field_name.build(stringify!($field_name), $id)?,
                        )*
                    };

                    segment.validate()?;
                    Ok(segment)
                }
            }
        }
    };
}

/// Helper macro to parse field from segment (internal use)
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_parse_field {
    // Optional field (detected by Option< prefix)
    ($segment:expr, $num:literal, Option < $($inner:tt)* >, $field_name:expr, $seg_id:expr) => {
        $crate::__z_segment_parse_field_inner!($segment, $num, $($inner)*, optional)
    };

    // Required field
    ($segment:expr, $num:literal, $($field_type:tt)*, $field_name:expr, $seg_id:expr) => {
        $crate::__z_segment_parse_field_inner!($segment, $num, $($field_type)*, required, $seg_id)
    };
}

/// Inner helper for parsing fields
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_parse_field_inner {
    // Optional String
    ($segment:expr, $num:literal, String, optional) => {
        $segment.get_field_value($num).map(|s| s.to_string())
    };

    // Required String
    ($segment:expr, $num:literal, String, required, $seg_id:expr) => {
        $segment
            .get_field_value($num)
            .map(|s| s.to_string())
            .ok_or_else(|| {
                $crate::error::CustomSegmentError::missing_field(
                    format!("{}-{}", $seg_id, $num),
                    $seg_id,
                )
            })?
    };

    // Optional f64
    ($segment:expr, $num:literal, f64, optional) => {
        $segment
            .get_field_value($num)
            .and_then(|s| s.parse::<f64>().ok())
    };

    // Required f64
    ($segment:expr, $num:literal, f64, required, $seg_id:expr) => {
        $segment
            .get_field_value($num)
            .and_then(|s| s.parse::<f64>().ok())
            .ok_or_else(|| {
                $crate::error::CustomSegmentError::missing_field(
                    format!("{}-{}", $seg_id, $num),
                    $seg_id,
                )
            })?
    };

    // Optional u32
    ($segment:expr, $num:literal, u32, optional) => {
        $segment
            .get_field_value($num)
            .and_then(|s| s.parse::<u32>().ok())
    };

    // Required u32
    ($segment:expr, $num:literal, u32, required, $seg_id:expr) => {
        $segment
            .get_field_value($num)
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or_else(|| {
                $crate::error::CustomSegmentError::missing_field(
                    format!("{}-{}", $seg_id, $num),
                    $seg_id,
                )
            })?
    };
}

/// Helper macro to set field on segment (internal use)
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_set_field {
    // Optional field (detected by Option< prefix)
    ($segment:expr, $num:literal, $value:expr, Option < $($inner:tt)* >) => {
        $crate::__z_segment_set_field_inner!($segment, $num, $value, $($inner)*, optional)
    };

    // Required field
    ($segment:expr, $num:literal, $value:expr, $($field_type:tt)*) => {
        $crate::__z_segment_set_field_inner!($segment, $num, $value, $($field_type)*, required)
    };
}

/// Inner helper for setting fields
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_set_field_inner {
    // Optional String field
    ($segment:expr, $num:literal, $value:expr, String, optional) => {
        if let Some(ref v) = $value {
            $segment.set_field_value($num, v)
        } else {
            Ok(())
        }
    };

    // Required String field
    ($segment:expr, $num:literal, $value:expr, String, required) => {
        $segment.set_field_value($num, $value)
    };

    // Optional f64 field
    ($segment:expr, $num:literal, $value:expr, f64, optional) => {
        if let Some(ref v) = $value {
            $segment.set_field_value($num, &v.to_string())
        } else {
            Ok(())
        }
    };

    // Required f64 field
    ($segment:expr, $num:literal, $value:expr, f64, required) => {
        $segment.set_field_value($num, &$value.to_string())
    };

    // Optional u32 field
    ($segment:expr, $num:literal, $value:expr, u32, optional) => {
        if let Some(ref v) = $value {
            $segment.set_field_value($num, &v.to_string())
        } else {
            Ok(())
        }
    };

    // Required u32 field
    ($segment:expr, $num:literal, $value:expr, u32, required) => {
        $segment.set_field_value($num, &$value.to_string())
    };
}

/// Helper macro to require field in builder (internal use)
/// Builder stores Option<T> for all fields, this unwraps required ones and keeps optional ones wrapped
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_require_field {
    // Optional field (detected by Option< prefix) - stays as Option<T>
    ($value:expr, $field_name:expr, $seg_id:expr, Option < $($inner:tt)* >) => {
        $value
    };

    // Required field - unwrap the Option<T> to get T
    ($value:expr, $field_name:expr, $seg_id:expr, $($field_type:tt)*) => {
        $value.ok_or_else(|| {
            $crate::error::CustomSegmentError::missing_field(
                format!("{}-{}", $seg_id, $field_name),
                $seg_id,
            )
        })?
    };
}

/// Helper macro to determine builder field type (internal use)
/// All builder fields are Option<T> where T is the inner type
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_builder_field_type {
    // Optional field - extract inner type and wrap in Option
    (Option < $($inner:tt)* >) => { Option<$($inner)*> };
    // Required field - wrap in Option
    ($($field_type:tt)*) => { Option<$($field_type)*> };
}

/// Helper macro to determine builder parameter type (internal use)
/// Setter methods accept the inner type (without Option wrapper)
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_builder_param_type {
    // Optional String - setter accepts String
    (Option < String >) => { impl Into<String> };
    // Required String - setter accepts String
    (String) => { impl Into<String> };
    // Optional f64 - setter accepts f64
    (Option < f64 >) => { f64 };
    // Required f64 - setter accepts f64
    (f64) => { f64 };
    // Optional u32 - setter accepts u32
    (Option < u32 >) => { u32 };
    // Required u32 - setter accepts u32
    (u32) => { u32 };
}

/// Helper macro to set builder value (internal use)
/// Always wraps the value in Some
#[doc(hidden)]
#[macro_export]
macro_rules! __z_segment_builder_set_value {
    // Optional String
    ($value:expr, Option < String >) => { Some($value.into()) };
    // Required String
    ($value:expr, String) => { Some($value.into()) };
    // Optional f64
    ($value:expr, Option < f64 >) => { Some($value) };
    // Required f64
    ($value:expr, f64) => { Some($value) };
    // Optional u32
    ($value:expr, Option < u32 >) => { Some($value) };
    // Required u32
    ($value:expr, u32) => { Some($value) };
}

// Export paste for use in the macro
#[doc(hidden)]
pub use paste;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::CustomSegment;
    use vyna_hl7_core::Segment;

    z_segment! {
        TestZPV,
        id = "ZPV",
        fields = {
            1 => visit_type: String,
            2 => visit_number: String,
            3 => patient_class: Option<String>,
        }
    }

    #[test]
    fn test_z_segment_macro_basic() {
        let zpv = TestZPV::builder()
            .visit_type("OUTPATIENT")
            .visit_number("V12345")
            .build()
            .unwrap();

        assert_eq!(zpv.visit_type, "OUTPATIENT");
        assert_eq!(zpv.visit_number, "V12345");
        assert_eq!(zpv.patient_class, None);
    }

    #[test]
    fn test_z_segment_to_segment() {
        let zpv = TestZPV::builder()
            .visit_type("EMERGENCY")
            .visit_number("V99999")
            .patient_class("I")
            .build()
            .unwrap();

        let segment = zpv.to_segment();
        assert_eq!(segment.id, "ZPV");
        assert_eq!(segment.get_field_value(1), Some("EMERGENCY"));
        assert_eq!(segment.get_field_value(2), Some("V99999"));
        assert_eq!(segment.get_field_value(3), Some("I"));
    }

    #[test]
    fn test_z_segment_from_segment() {
        let mut segment = Segment::new("ZPV");
        segment.set_field_value(1, "INPATIENT").unwrap();
        segment.set_field_value(2, "V11111").unwrap();

        let zpv = TestZPV::from_segment(&segment).unwrap();
        assert_eq!(zpv.visit_type, "INPATIENT");
        assert_eq!(zpv.visit_number, "V11111");
    }

    z_segment! {
        TestZCU,
        id = "ZCU",
        fields = {
            1 => customer_id: String,
            2 => balance: Option<f64>,
        },
        validate = |s: &TestZCU| {
            if let Some(balance) = s.balance
                && balance < 0.0 {
                    return Err(crate::error::CustomSegmentError::validation_failed(
                        "ZCU",
                        "Balance cannot be negative"
                    ));
                }
            Ok(())
        }
    }

    #[test]
    fn test_z_segment_with_validation() {
        let zcu = TestZCU::builder()
            .customer_id("CUST001")
            .balance(1500.0)
            .build();

        assert!(zcu.is_ok());
    }

    #[test]
    fn test_z_segment_validation_fails() {
        let zcu = TestZCU::builder()
            .customer_id("CUST002")
            .balance(-100.0)
            .build();

        assert!(zcu.is_err());
    }
}
