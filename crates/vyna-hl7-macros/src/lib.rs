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
use syn::{Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta, Type, parse_macro_input};

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
        && let Some(segment) = type_path.path.segments.first()
    {
        return segment.ident == "Option";
    }
    false
}

/// Check if a type is Vec<T>
fn is_vec_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.first()
    {
        return segment.ident == "Vec";
    }
    false
}

/// Generate code that parses a delimited string value and appends the resulting
/// repetitions to the `__field` variable in scope.
fn append_reps_tokens(value_expr: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        for __rep_str in (#value_expr).split('~') {
            let mut __rep = vyna_hl7_core::field::Repetition::new();
            for __comp_str in __rep_str.split('^') {
                let mut __comp = vyna_hl7_core::field::Component::new();
                for __sub in __comp_str.split('&') {
                    __comp.add_subcomponent(
                        vyna_hl7_core::field::SubComponent::new(__sub)
                    );
                }
                __rep.add_component(__comp);
            }
            __field.add_repetition(__rep);
        }
    }
}

/// Extract the inner type `T` from `Option<T>` or `Vec<T>`
fn inner_generic_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.first()
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
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
/// - `#[hl7(optional)]` - Marks the field as optional (also inferred from `Option<T>`).
///
/// # Field types
///
/// - `String` - Required whole field. Composite values are preserved verbatim
///   (components joined by `^`, repetitions joined by `~`).
/// - `Option<String>` - Optional whole field (`None` when the field is absent).
/// - `Vec<String>` - Repeating field: one element per repetition, each holding
///   the full composite value. Cannot be combined with `component=`.
///
/// Whole-field serialization assumes the standard HL7 separators (`~`, `^`,
/// `&`); values containing escape sequences are treated literally.
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
///     patient_id: Vec<String>,
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
            let is_repeating = is_vec_type(field_type);

            if is_repeating
                && (attrs.component.is_some()
                    || attrs.subcomponent.is_some()
                    || attrs.repetition.is_some())
            {
                panic!(
                    "Vec<T> field `{}` supports whole-field repetitions only; \
                     component=, subcomponent= and repetition= cannot be combined with Vec<T>",
                    field_name
                );
            }

            if is_repeating {
                // Repeating field: one Vec element per repetition, each holding
                // the full composite value (components joined by `^`).
                from_segment_fields.push(quote! {
                    #field_name: segment.get_field(#field_num)
                        .map(|f| f.repetitions.iter()
                            .filter_map(|r| r.full_value())
                            .collect::<Vec<String>>())
                        .unwrap_or_default()
                });

                let append = append_reps_tokens(&quote! { __val });
                to_segment_fields.push(quote! {
                    {
                        let mut __field = vyna_hl7_core::field::Field::new();
                        for __val in self.#field_name.iter() {
                            #append
                        }
                        let _ = segment.set_field(#field_num, __field);
                    }
                });
                continue;
            }

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
                // Extract entire field value: every repetition with its full
                // composite value, joined by the repetition separator.
                quote! {
                    segment.get_field(#field_num)
                        .and_then(|f| {
                            let __parts: Vec<String> = f.repetitions.iter()
                                .filter_map(|r| r.full_value())
                                .collect();
                            if __parts.is_empty() {
                                None
                            } else {
                                Some(__parts.join("~"))
                            }
                        })
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
                    let append = append_reps_tokens(&quote! { val });
                    to_segment_fields.push(quote! {
                        if let Some(ref val) = self.#field_name {
                            let mut __field = vyna_hl7_core::field::Field::new();
                            #append
                            let _ = segment.set_field(#field_num, __field);
                        }
                    });
                }
            } else if let Some(comp) = attrs.component {
                to_segment_fields.push(quote! {
                    let _ = segment.set_component(#field_num, 0, #comp - 1, &self.#field_name);
                });
            } else {
                let append = append_reps_tokens(&quote! { &self.#field_name });
                to_segment_fields.push(quote! {
                    {
                        let mut __field = vyna_hl7_core::field::Field::new();
                        #append
                        let _ = segment.set_field(#field_num, __field);
                    }
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
/// - `#[hl7(segment, repeating)]` - Marks a field as a repeating segment (`Vec<T>`).
///
/// # Segment matching
///
/// `from_message` matches segments by ID using each field type's `SEGMENT_ID`:
/// required fields take the first match (a missing one fails the conversion),
/// `Option<T>` fields take the first match if present, and `Vec<T>` fields
/// collect every match in order. Segments sharing one ID across different
/// groups are not distinguished positionally.
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
        let is_vec = is_vec_type(field_type);

        if is_vec {
            // Repeating segment: every segment with the inner type's ID, in order.
            let inner = inner_generic_type(field_type).unwrap_or_else(|| {
                panic!(
                    "Repeating segment field `{}` must declare an element type, e.g. Vec<Observation>",
                    field_name
                )
            });
            from_message_fields.push(quote! {
                #field_name: message.get_segments_by_id(<#inner>::SEGMENT_ID)
                    .into_iter()
                    .filter_map(<#inner>::from_segment)
                    .collect()
            });

            to_message_fields.push(quote! {
                for item in &self.#field_name {
                    message.add_segment(item.to_segment());
                }
            });
        } else if is_optional {
            // Optional segment: first segment with the inner type's ID, if any.
            let inner = inner_generic_type(field_type).unwrap_or_else(|| {
                panic!(
                    "Optional segment field `{}` must declare an inner type, e.g. Option<PatientVisit>",
                    field_name
                )
            });
            from_message_fields.push(quote! {
                #field_name: message.get_segment_by_id(<#inner>::SEGMENT_ID)
                    .and_then(<#inner>::from_segment)
            });

            to_message_fields.push(quote! {
                if let Some(ref seg) = self.#field_name {
                    message.add_segment(seg.to_segment());
                }
            });
        } else {
            // Required segment: first segment with the field type's ID.
            // A missing segment fails the whole conversion (returns None).
            let seg_ty = field_type;
            from_message_fields.push(quote! {
                #field_name: <#seg_ty>::from_segment(
                    message.get_segment_by_id(<#seg_ty>::SEGMENT_ID)?
                )?
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

            /// Create from a vyna_hl7_core Message.
            ///
            /// Segments are matched by ID: required segments use the first
            /// match, optional segments use the first match if present, and
            /// repeating segments collect every match in order. Returns `None`
            /// when a required segment is missing.
            pub fn from_message(message: &vyna_hl7_core::message::Message) -> Option<Self> {
                Some(Self {
                    #(#from_message_fields),*
                })
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

        impl std::convert::TryFrom<&vyna_hl7_core::message::Message> for #name {
            type Error = vyna_hl7_core::error::Error;

            fn try_from(message: &vyna_hl7_core::message::Message) -> Result<Self, Self::Error> {
                Self::from_message(message).ok_or_else(|| {
                    vyna_hl7_core::error::Error::MessageType(format!(
                        "Message is missing required segments for {}^ {}",
                        #message_type, #trigger_event
                    ))
                })
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
