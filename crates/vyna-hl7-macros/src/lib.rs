//! Procedural macros for vyna_hl7
//!
//! This crate provides derive macros for creating strongly-typed HL7 message structures.
//!
//! # Overview
//!
//! The macros in this crate allow you to define Rust structs that map directly to HL7
//! segments and messages, with automatic serialization and deserialization.
//!
//! # Examples
//!
//! ## Defining a Segment
//!
//! ```ignore
//! use vyna_hl7_macros::Segment;
//!
//! #[derive(Segment)]
//! #[hl7(id = "PID")]
//! struct PatientIdentification {
//!     #[hl7(field = 1)]
//!     set_id: Option<String>,
//!
//!     #[hl7(field = 3)]
//!     patient_id: String,
//!
//!     #[hl7(field = 5)]
//!     patient_name: String,
//!
//!     #[hl7(field = 7)]
//!     date_of_birth: Option<String>,
//!
//!     #[hl7(field = 8)]
//!     sex: Option<String>,
//! }
//! ```
//!
//! ## Defining a Message
//!
//! ```ignore
//! use vyna_hl7_macros::{Message, Segment};
//!
//! #[derive(Message)]
//! #[hl7(message_type = "ADT", trigger_event = "A01")]
//! struct AdtA01 {
//!     #[hl7(segment)]
//!     msh: MessageHeader,
//!
//!     #[hl7(segment)]
//!     pid: PatientIdentification,
//!
//!     #[hl7(segment, optional)]
//!     pv1: Option<PatientVisit>,
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta, Type,
};

/// Parsed field attributes for HL7 fields
#[derive(Default)]
struct FieldAttr {
    field_num: Option<usize>,
    component: Option<usize>,
    #[allow(dead_code)]
    subcomponent: Option<usize>,
    #[allow(dead_code)]
    repetition: Option<usize>,
    optional: bool,
}


/// Parsed segment attributes
#[derive(Default)]
struct SegmentAttr {
    id: Option<String>,
}


/// Parsed message attributes
#[derive(Default)]
struct MessageAttr {
    message_type: Option<String>,
    trigger_event: Option<String>,
}


/// Parse HL7 attributes from a field (syn 2.x API)
fn parse_field_attrs(attrs: &[Attribute]) -> FieldAttr {
    let mut result = FieldAttr::default();

    for attr in attrs {
        if !attr.path().is_ident("hl7") {
            continue;
        }

        // Parse the nested meta using syn 2.x API
        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("field") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                if let Lit::Int(lit_int) = lit {
                    result.field_num = lit_int.base10_parse().ok();
                }
            } else if meta.path.is_ident("component") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                if let Lit::Int(lit_int) = lit {
                    result.component = lit_int.base10_parse().ok();
                }
            } else if meta.path.is_ident("subcomponent") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                if let Lit::Int(lit_int) = lit {
                    result.subcomponent = lit_int.base10_parse().ok();
                }
            } else if meta.path.is_ident("repetition") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                if let Lit::Int(lit_int) = lit {
                    result.repetition = lit_int.base10_parse().ok();
                }
            } else if meta.path.is_ident("optional") {
                result.optional = true;
            }
            Ok(())
        });
    }

    result
}

/// Parse HL7 attributes from a segment struct (syn 2.x API)
fn parse_segment_attrs(attrs: &[Attribute]) -> SegmentAttr {
    let mut result = SegmentAttr::default();

    for attr in attrs {
        if !attr.path().is_ident("hl7") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                if let Lit::Str(lit_str) = lit {
                    result.id = Some(lit_str.value());
                }
            }
            Ok(())
        });
    }

    result
}

/// Parse HL7 attributes from a message struct (syn 2.x API)
fn parse_message_attrs(attrs: &[Attribute]) -> MessageAttr {
    let mut result = MessageAttr::default();

    for attr in attrs {
        if !attr.path().is_ident("hl7") {
            continue;
        }

        let _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("message_type") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                if let Lit::Str(lit_str) = lit {
                    result.message_type = Some(lit_str.value());
                }
            } else if meta.path.is_ident("trigger_event") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                if let Lit::Str(lit_str) = lit {
                    result.trigger_event = Some(lit_str.value());
                }
            }
            Ok(())
        });
    }

    result
}

/// Check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.first() {
            return segment.ident == "Option";
        }
    false
}

/// Derive macro for HL7 segments
///
/// This macro generates implementations for converting between a Rust struct
/// and an HL7 segment.
///
/// # Attributes
///
/// - `#[hl7(id = "XXX")]` - Required. Specifies the 3-character segment ID.
/// - `#[hl7(field = N)]` - Specifies which field number (1-based) this struct field maps to.
/// - `#[hl7(component = N)]` - Specifies which component within the field (1-based).
/// - `#[hl7(optional)]` - Marks the field as optional (also inferred from Option<T>).
///
/// # Example
///
/// ```ignore
/// #[derive(Segment)]
/// #[hl7(id = "PID")]
/// struct PatientIdentification {
///     #[hl7(field = 1)]
///     set_id: Option<String>,
///
///     #[hl7(field = 3)]
///     patient_id: String,
///
///     #[hl7(field = 5, component = 1)]
///     family_name: String,
///
///     #[hl7(field = 5, component = 2)]
///     given_name: Option<String>,
/// }
/// ```
#[proc_macro_derive(Segment, attributes(hl7))]
pub fn derive_segment(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let segment_attrs = parse_segment_attrs(&input.attrs);
    let segment_id = segment_attrs.id.unwrap_or_else(|| {
        // Default to struct name's first 3 chars uppercase
        name.to_string()
            .chars()
            .take(3)
            .collect::<String>()
            .to_uppercase()
    });

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Segment derive only supports named fields"),
        },
        _ => panic!("Segment derive only supports structs"),
    };

    // Generate field extraction code for from_segment
    let mut from_segment_fields = Vec::new();
    let mut to_segment_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs);

        if let Some(field_num) = attrs.field_num {
            let is_optional = is_option_type(field_type) || attrs.optional;

            // Generate extraction code based on component
            let extraction = if let Some(comp) = attrs.component {
                // Extract specific component
                quote! {
                    segment.get_field(#field_num)
                        .and_then(|f| f.get_repetition(0))
                        .and_then(|r| r.get_component(#comp - 1))
                        .and_then(|c| c.value())
                        .map(|s| s.to_string())
                }
            } else {
                // Extract entire field value
                quote! {
                    segment.get_field(#field_num)
                        .and_then(|f| f.value())
                        .map(|s| s.to_string())
                }
            };

            if is_optional {
                from_segment_fields.push(quote! {
                    #field_name: #extraction
                });
            } else {
                from_segment_fields.push(quote! {
                    #field_name: #extraction.unwrap_or_default()
                });
            }

            // Generate serialization code
            if is_optional {
                if let Some(comp) = attrs.component {
                    to_segment_fields.push(quote! {
                        if let Some(ref val) = self.#field_name {
                            let _ = segment.set_component(#field_num, 0, #comp - 1, val);
                        }
                    });
                } else {
                    to_segment_fields.push(quote! {
                        if let Some(ref val) = self.#field_name {
                            let _ = segment.set_field_value(#field_num, val);
                        }
                    });
                }
            } else if let Some(comp) = attrs.component {
                to_segment_fields.push(quote! {
                    let _ = segment.set_component(#field_num, 0, #comp - 1, &self.#field_name);
                });
            } else {
                to_segment_fields.push(quote! {
                    let _ = segment.set_field_value(#field_num, &self.#field_name);
                });
            }
        }
    }

    let expanded = quote! {
        impl #name {
            /// The HL7 segment ID for this type
            pub const SEGMENT_ID: &'static str = #segment_id;

            /// Create from an vyna_hl7_core Segment
            pub fn from_segment(segment: &vyna_hl7_core::segment::Segment) -> Option<Self> {
                if segment.id != #segment_id {
                    return None;
                }

                Some(Self {
                    #(#from_segment_fields),*
                })
            }

            /// Convert to an vyna_hl7_core Segment
            pub fn to_segment(&self) -> vyna_hl7_core::segment::Segment {
                let mut segment = vyna_hl7_core::segment::Segment::new(#segment_id);
                #(#to_segment_fields)*
                segment
            }

            /// Get the segment ID
            pub fn segment_id(&self) -> &'static str {
                #segment_id
            }
        }

        impl From<#name> for vyna_hl7_core::segment::Segment {
            fn from(value: #name) -> Self {
                value.to_segment()
            }
        }

        impl std::convert::TryFrom<&vyna_hl7_core::segment::Segment> for #name {
            type Error = vyna_hl7_core::error::Error;

            fn try_from(segment: &vyna_hl7_core::segment::Segment) -> Result<Self, Self::Error> {
                Self::from_segment(segment)
                    .ok_or_else(|| vyna_hl7_core::error::Error::InvalidSegment(
                        format!("Expected {} segment, got {}", #segment_id, segment.id)
                    ))
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for HL7 messages
///
/// This macro generates implementations for converting between a Rust struct
/// and an HL7 message.
///
/// # Attributes
///
/// - `#[hl7(message_type = "XXX")]` - Specifies the message type (e.g., "ADT").
/// - `#[hl7(trigger_event = "XXX")]` - Specifies the trigger event (e.g., "A01").
/// - `#[hl7(segment)]` - Marks a field as a segment.
/// - `#[hl7(segment, optional)]` - Marks a field as an optional segment.
/// - `#[hl7(segment, repeating)]` - Marks a field as a repeating segment (Vec<T>).
///
/// # Example
///
/// ```ignore
/// #[derive(Message)]
/// #[hl7(message_type = "ADT", trigger_event = "A01")]
/// struct AdtA01 {
///     #[hl7(segment)]
///     msh: MessageHeader,
///
///     #[hl7(segment)]
///     pid: PatientIdentification,
///
///     #[hl7(segment, optional)]
///     pv1: Option<PatientVisit>,
///
///     #[hl7(segment, repeating)]
///     obx: Vec<Observation>,
/// }
/// ```
#[proc_macro_derive(Message, attributes(hl7))]
pub fn derive_message(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let message_attrs = parse_message_attrs(&input.attrs);
    let message_type = message_attrs.message_type.unwrap_or_default();
    let trigger_event = message_attrs.trigger_event.unwrap_or_default();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("Message derive only supports named fields"),
        },
        _ => panic!("Message derive only supports structs"),
    };

    let mut from_message_fields = Vec::new();
    let mut to_message_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let is_optional = is_option_type(field_type);

        // Check if this is a Vec (repeating segment)
        let is_vec = if let Type::Path(type_path) = field_type {
            type_path
                .path
                .segments
                .first()
                .map(|s| s.ident == "Vec")
                .unwrap_or(false)
        } else {
            false
        };

        if is_vec {
            // Repeating segment
            from_message_fields.push(quote! {
                #field_name: {
                    let items = Vec::new();
                    // This would need to know the inner type's SEGMENT_ID
                    // For now, just create empty vec
                    items
                }
            });

            to_message_fields.push(quote! {
                for item in &self.#field_name {
                    message.add_segment(item.to_segment());
                }
            });
        } else if is_optional {
            // Optional segment
            from_message_fields.push(quote! {
                #field_name: None // Would need type info to extract
            });

            to_message_fields.push(quote! {
                if let Some(ref seg) = self.#field_name {
                    message.add_segment(seg.to_segment());
                }
            });
        } else {
            // Required segment - would need Default trait
            from_message_fields.push(quote! {
                #field_name: Default::default()
            });

            to_message_fields.push(quote! {
                message.add_segment(self.#field_name.to_segment());
            });
        }
    }

    let expanded = quote! {
        impl #name {
            /// The HL7 message type
            pub const MESSAGE_TYPE: &'static str = #message_type;

            /// The HL7 trigger event
            pub const TRIGGER_EVENT: &'static str = #trigger_event;

            /// Get the message type
            pub fn message_type(&self) -> &'static str {
                #message_type
            }

            /// Get the trigger event
            pub fn trigger_event(&self) -> &'static str {
                #trigger_event
            }

            /// Convert to an vyna_hl7_core Message
            pub fn to_message(&self) -> vyna_hl7_core::message::Message {
                let mut message = vyna_hl7_core::message::Message::new();
                #(#to_message_fields)*
                message
            }
        }

        impl From<#name> for vyna_hl7_core::message::Message {
            fn from(value: #name) -> Self {
                value.to_message()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for defining HL7 data types
///
/// This macro helps define composite HL7 data types like XPN (Extended Person Name),
/// XAD (Extended Address), etc.
///
/// # Attributes
///
/// - `#[hl7_type(data_type = "XXX")]` - Specifies the HL7 data type code.
///
/// # Example
///
/// ```ignore
/// #[hl7_type(data_type = "XPN")]
/// struct ExtendedPersonName {
///     #[hl7(component = 1)]
///     family_name: String,
///
///     #[hl7(component = 2)]
///     given_name: String,
///
///     #[hl7(component = 3)]
///     second_name: Option<String>,
///
///     #[hl7(component = 5)]
///     prefix: Option<String>,
///
///     #[hl7(component = 6)]
///     suffix: Option<String>,
/// }
/// ```
#[proc_macro_attribute]
pub fn hl7_type(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    // Parse the data_type from attribute meta
    let attr_meta: Meta = syn::parse(attr).expect("Failed to parse attribute");
    let data_type = match attr_meta {
        Meta::NameValue(nv) if nv.path.is_ident("data_type") => {
            if let Expr::Lit(expr_lit) = &nv.value {
                if let Lit::Str(lit_str) = &expr_lit.lit {
                    lit_str.value()
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    };

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("hl7_type only supports named fields"),
        },
        _ => panic!("hl7_type only supports structs"),
    };

    let mut from_rep_fields = Vec::new();
    let mut to_rep_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs);
        let is_optional = is_option_type(field_type);

        if let Some(comp) = attrs.component {
            let comp_idx = comp - 1; // Convert to 0-based

            if is_optional {
                from_rep_fields.push(quote! {
                    #field_name: repetition.get_component(#comp_idx)
                        .and_then(|c| c.value())
                        .map(|s| s.to_string())
                });

                to_rep_fields.push(quote! {
                    if let Some(ref val) = self.#field_name {
                        while repetition.components.len() <= #comp_idx {
                            repetition.add_component(vyna_hl7_core::field::Component::new());
                        }
                        repetition.components[#comp_idx] = vyna_hl7_core::field::Component::from_value(val);
                    }
                });
            } else {
                from_rep_fields.push(quote! {
                    #field_name: repetition.get_component(#comp_idx)
                        .and_then(|c| c.value())
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                });

                to_rep_fields.push(quote! {
                    while repetition.components.len() <= #comp_idx {
                        repetition.add_component(vyna_hl7_core::field::Component::new());
                    }
                    repetition.components[#comp_idx] = vyna_hl7_core::field::Component::from_value(&self.#field_name);
                });
            }
        }
    }

    // Get original struct definition parts
    let vis = &input.vis;
    let struct_fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => panic!("hl7_type only supports named fields"),
        },
        _ => panic!("hl7_type only supports structs"),
    };

    let expanded = quote! {
        #vis struct #name #struct_fields

        impl #name {
            /// The HL7 data type code
            pub const DATA_TYPE: &'static str = #data_type;

            /// Create from an vyna_hl7_core Repetition
            pub fn from_repetition(repetition: &vyna_hl7_core::field::Repetition) -> Self {
                Self {
                    #(#from_rep_fields),*
                }
            }

            /// Convert to an vyna_hl7_core Repetition
            pub fn to_repetition(&self) -> vyna_hl7_core::field::Repetition {
                let mut repetition = vyna_hl7_core::field::Repetition::new();
                #(#to_rep_fields)*
                repetition
            }

            /// Get the data type code
            pub fn data_type(&self) -> &'static str {
                #data_type
            }
        }

        impl From<#name> for vyna_hl7_core::field::Repetition {
            fn from(value: #name) -> Self {
                value.to_repetition()
            }
        }

        impl From<&vyna_hl7_core::field::Repetition> for #name {
            fn from(repetition: &vyna_hl7_core::field::Repetition) -> Self {
                Self::from_repetition(repetition)
            }
        }
    };

    TokenStream::from(expanded)
}
