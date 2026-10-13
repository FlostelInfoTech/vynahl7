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
        // Reject non-Z ids and field 0 at expansion: get_field(0) is
        // always None and set_field(0) always fails, so field 0 would
        // silently never serialize.
        const _: () = assert!(
            $crate::macros::__is_z_segment_id($id),
            "z_segment! id must start with 'Z'"
        );
        $(
            const _: () = assert!(
                $field_num != 0,
                "z_segment! field numbers are 1-based; field 0 addresses nothing"
            );
        )*

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

                if segment.id != $id {
                    return Err($crate::error::CustomSegmentError::parse_error(
                        $id,
                        format!("expected {} segment, got {}", $id, segment.id),
                    ));
                }
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

/// Check that a `z_segment!` id names a Z-segment (const so the
/// macro can reject bad ids at expansion with a clear error).
#[doc(hidden)]
pub const fn __is_z_segment_id(id: &str) -> bool {
    !id.is_empty() && id.as_bytes()[0] == b'Z'
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

    #[test]
    fn test_z_segment_from_segment_rejects_wrong_id() {
        // A PID segment must not misparse as ZPV: fields would bind to
        // the wrong positions.
        let mut segment = Segment::new("PID");
        segment.set_field_value(1, "1").unwrap();
        segment.set_field_value(2, "V123").unwrap();

        let err = TestZPV::from_segment(&segment).unwrap_err();
        assert!(err.to_string().contains("PID"), "unexpected: {err}");
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

    #[test]
    fn test_is_z_segment_id() {
        // Predicate behind the z_segment! expansion-time checks.
        assert!(__is_z_segment_id("ZPV"));
        assert!(__is_z_segment_id("Z"));
        assert!(!__is_z_segment_id("PID"));
        assert!(!__is_z_segment_id(""));
        assert!(!__is_z_segment_id("zpv"));
    }
}
