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
#[derive(Debug, Default)]
struct FieldAttr {
    field_num: Option<usize>,
    component: Option<usize>,
    subcomponent: Option<usize>,
    repetition: Option<usize>,
    optional: bool,
    required: bool,
}

/// Parsed segment attributes
#[derive(Debug, Default)]
struct SegmentAttr {
    id: Option<String>,
}

/// Parsed message attributes
#[derive(Debug, Default)]
struct MessageAttr {
    message_type: Option<String>,
    trigger_event: Option<String>,
}

/// Parsed `#[hl7(...)]` markers on a Message derive field
#[derive(Debug, Default)]
struct MessageFieldAttr {
    segment: bool,
    optional: bool,
    repeating: bool,
    required: bool,
}

/// Parse a `key = N` value where `N` is a 1-based HL7 position.
///
/// Mistyped values and `0` are errors: silently defaulting them would
/// generate code that reads or writes the wrong location (or panic on
/// the `N - 1` conversion to a 0-based index).
fn parse_position_value(input: syn::parse::ParseStream<'_>, key: &str) -> syn::Result<usize> {
    let _: syn::Token![=] = input.parse()?;
    let lit: Lit = input.parse()?;
    let Lit::Int(lit_int) = lit else {
        return Err(syn::Error::new_spanned(
            &lit,
            format!("expected integer literal for `{key}`"),
        ));
    };
    let value: usize = lit_int.base10_parse()?;
    if value == 0 {
        return Err(syn::Error::new_spanned(
            &lit_int,
            format!("hl7 {key} numbers are 1-based; `{key} = 0` addresses nothing"),
        ));
    }
    Ok(value)
}

/// Reject `flag = value`: bare flags take no value.
///
/// Note: `meta.input` still holds the separator remainder (`,` or end)
/// after a bare path, so emptiness is not the test; a following `=`
/// proves a value was given.
fn require_bare_flag(input: syn::parse::ParseStream<'_>, flag: &str) -> syn::Result<()> {
    if input.peek(syn::Token![=]) {
        Err(input.error(format!("`{flag}` takes no value")))
    } else {
        Ok(())
    }
}

/// Parse HL7 attributes from a field (syn 2.x API)
///
/// Unknown keys and malformed values are errors rather than silent
/// no-ops, so typos fail the build instead of generating wrong code.
fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttr> {
    let mut result = FieldAttr::default();

    for attr in attrs {
        if !attr.path().is_ident("hl7") {
            continue;
        }

        // Parse the nested meta using syn 2.x API
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("field") {
                result.field_num = Some(parse_position_value(meta.input, "field")?);
            } else if meta.path.is_ident("component") {
                result.component = Some(parse_position_value(meta.input, "component")?);
            } else if meta.path.is_ident("subcomponent") {
                result.subcomponent = Some(parse_position_value(meta.input, "subcomponent")?);
            } else if meta.path.is_ident("repetition") {
                result.repetition = Some(parse_position_value(meta.input, "repetition")?);
            } else if meta.path.is_ident("optional") {
                require_bare_flag(meta.input, "optional")?;
                result.optional = true;
            } else if meta.path.is_ident("required") {
                require_bare_flag(meta.input, "required")?;
                result.required = true;
            } else {
                return Err(syn::Error::new_spanned(
                    &meta.path,
                    "unknown hl7 field attribute; expected one of: field, \
                     component, subcomponent, repetition, optional, required",
                ));
            }
            Ok(())
        })?;
    }

    Ok(result)
}

/// Parse HL7 attributes from a segment struct (syn 2.x API)
fn parse_segment_attrs(attrs: &[Attribute]) -> syn::Result<SegmentAttr> {
    let mut result = SegmentAttr::default();

    for attr in attrs {
        if !attr.path().is_ident("hl7") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                let _: syn::Token![=] = meta.input.parse()?;
                let lit: Lit = meta.input.parse()?;
                let Lit::Str(lit_str) = lit else {
                    return Err(syn::Error::new_spanned(
                        &lit,
                        "expected string literal for `id`",
                    ));
                };
                let id = lit_str.value();
                if id.is_empty() {
                    return Err(syn::Error::new_spanned(&lit_str, "`id` must not be empty"));
                }
                result.id = Some(id);
            } else {
                return Err(syn::Error::new_spanned(
                    &meta.path,
                    "unknown hl7 segment attribute; expected `id`",
                ));
            }
            Ok(())
        })?;
    }

    Ok(result)
}

/// Parse a `key = "value"` string attribute.
fn parse_string_value(input: syn::parse::ParseStream<'_>, key: &str) -> syn::Result<String> {
    let _: syn::Token![=] = input.parse()?;
    let lit: Lit = input.parse()?;
    let Lit::Str(lit_str) = lit else {
        return Err(syn::Error::new_spanned(
            &lit,
            format!("expected string literal for `{key}`"),
        ));
    };
    Ok(lit_str.value())
}

/// Parse HL7 attributes from a message struct (syn 2.x API)
fn parse_message_attrs(attrs: &[Attribute]) -> syn::Result<MessageAttr> {
    let mut result = MessageAttr::default();

    for attr in attrs {
        if !attr.path().is_ident("hl7") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("message_type") {
                result.message_type = Some(parse_string_value(meta.input, "message_type")?);
            } else if meta.path.is_ident("trigger_event") {
                result.trigger_event = Some(parse_string_value(meta.input, "trigger_event")?);
            } else {
                return Err(syn::Error::new_spanned(
                    &meta.path,
                    "unknown hl7 message attribute; expected one of: \
                     message_type, trigger_event",
                ));
            }
            Ok(())
        })?;
    }

    Ok(result)
}

/// Parse HL7 markers from a Message derive field.
///
/// Every message field must carry `segment`, with `optional` or
/// `repeating` matching its `Option<T>` / `Vec<T>` type.
fn parse_message_field_attrs(attrs: &[Attribute]) -> syn::Result<MessageFieldAttr> {
    let mut result = MessageFieldAttr::default();

    for attr in attrs {
        if !attr.path().is_ident("hl7") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("segment") {
                require_bare_flag(meta.input, "segment")?;
                result.segment = true;
            } else if meta.path.is_ident("optional") {
                require_bare_flag(meta.input, "optional")?;
                result.optional = true;
            } else if meta.path.is_ident("repeating") {
                require_bare_flag(meta.input, "repeating")?;
                result.repeating = true;
            } else if meta.path.is_ident("required") {
                require_bare_flag(meta.input, "required")?;
                result.required = true;
            } else {
                return Err(syn::Error::new_spanned(
                    &meta.path,
                    "unknown hl7 message-field marker; expected one of: \
                     segment, optional, repeating, required",
                ));
            }
            Ok(())
        })?;
    }

    Ok(result)
}

/// Check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    is_std_wrapper(ty, "Option", "option")
}

/// Check if a type is Vec<T>
fn is_vec_type(ty: &Type) -> bool {
    is_std_wrapper(ty, "Vec", "vec")
}

/// Match a std wrapper type by full path: the bare name plus
/// `std::`/`core::`/`alloc::`-rooted spellings (with or without a
/// leading `::`, which syn records outside the segments).
///
/// A first-segment-only check misreads qualified spellings like
/// `std::option::Option<T>` as required scalars, producing
/// downstream E0308/E0599 errors. Same-named types from other
/// crates never match.
fn is_std_wrapper(ty: &Type, name: &str, module: &str) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };
    if type_path.qself.is_some() {
        return false;
    }
    let segments = &type_path.path.segments;
    match segments.len() {
        1 => segments[0].ident == name,
        3 => {
            (segments[0].ident == "std"
                || segments[0].ident == "core"
                || segments[0].ident == "alloc")
                && segments[1].ident == module
                && segments[2].ident == name
        }
        _ => false,
    }
}

/// Generate code that parses a delimited string value and appends the resulting
/// repetitions to the `__field` variable in scope.
///
/// Each atom is HL7-decoded after splitting, the inverse of
/// [`repetition_to_escaped_tokens`]: a literal `^` in the data arrives as
/// `\S\` and must not become structure on re-split. Malformed escapes
/// survive literally — `to_segment` is infallible, so a bad sequence must
/// not fail the write.
fn append_reps_tokens(value_expr: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    quote! {
        {
            let __delims = vyna_hl7_core::delimiters::Delimiters::default();
            for __rep_str in (#value_expr).split('~') {
                let mut __rep = vyna_hl7_core::field::Repetition::new();
                for __comp_str in __rep_str.split('^') {
                    let mut __comp = vyna_hl7_core::field::Component::new();
                    for __sub in __comp_str.split('&') {
                        let __decoded = vyna_hl7_core::encoding::Encoding::decode(
                            __sub, &__delims,
                        )
                        .unwrap_or_else(|_| __sub.to_string());
                        __comp.add_subcomponent(
                            vyna_hl7_core::field::SubComponent::new(__decoded)
                        );
                    }
                    __rep.add_component(__comp);
                }
                __field.add_repetition(__rep);
            }
        }
    }
}

/// Generate a closure rendering one repetition as escape-aware composite text.
///
/// Subcomponent values are HL7-escaped, then joined with `&` and `^`.
/// Trailing empty components are trimmed and a repetition without
/// components yields `None`, matching `full_value` semantics.
fn repetition_to_escaped_tokens() -> proc_macro2::TokenStream {
    quote! {
        |__rep: &vyna_hl7_core::field::Repetition| -> Option<String> {
            if __rep.components.is_empty() {
                return None;
            }
            let __delims = vyna_hl7_core::delimiters::Delimiters::default();
            let __comps: Vec<String> = __rep
                .components
                .iter()
                .map(|__comp| {
                    __comp
                        .subcomponents
                        .iter()
                        .map(|__sub| {
                            vyna_hl7_core::encoding::Encoding::encode(
                                __sub.as_str(),
                                &__delims,
                            )
                        })
                        .collect::<Vec<String>>()
                        .join("&")
                })
                .collect();
            // Encoding never maps empty to non-empty (or vice versa), so
            // trimming the escaped text matches trimming decoded values.
            let mut __end = __comps.len();
            while __end > 0 && __comps[__end - 1].is_empty() {
                __end -= 1;
            }
            Some(__comps[..__end].join("^"))
        }
    }
}

/// Generate code storing a component value subcomponent-aware.
///
/// The value is split on `&` into subcomponents and each piece is
/// HL7-decoded, the inverse of the component read (which joins escaped
/// subcomponents), so composite component text — including a literal
/// `&` carried as `\T\` — survives a no-change round trip. Malformed
/// escapes survive literally, mirroring [`append_reps_tokens`].
/// `set_component` first gap-fills the field/repetition/component
/// structure; the built component then replaces the placeholder. The
/// generated code avoids let-chains so it compiles under older
/// editions too.
fn set_component_split_tokens(
    field_num: usize,
    comp: usize,
    value_expr: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        {
            let __delims = vyna_hl7_core::delimiters::Delimiters::default();
            let mut __comp = vyna_hl7_core::field::Component::new();
            for __sub in (#value_expr).split('&') {
                let __decoded = vyna_hl7_core::encoding::Encoding::decode(
                    __sub, &__delims,
                )
                .unwrap_or_else(|_| __sub.to_string());
                __comp.add_subcomponent(
                    vyna_hl7_core::field::SubComponent::new(__decoded),
                );
            }
            let _ = segment.set_component(#field_num, 0, #comp - 1, "");
            if let Some(__rep) = segment
                .get_field_mut(#field_num)
                .and_then(|__f| __f.get_repetition_mut(0))
            {
                if let Some(__slot) = __rep.components.get_mut(#comp - 1) {
                    *__slot = __comp;
                }
            }
        }
    }
}

/// Generate an expression reading one repetition component's
/// escape-aware text: every subcomponent HL7-escaped and joined by
/// `&`, so no subcomponent is silently dropped. A component without
/// subcomponents yields `None`, like a missing component.
fn component_to_escaped_expr(comp_idx: usize) -> proc_macro2::TokenStream {
    quote! {
        repetition.get_component(#comp_idx)
            .and_then(|c| {
                if c.subcomponents.is_empty() {
                    None
                } else {
                    let __delims =
                        vyna_hl7_core::delimiters::Delimiters::default();
                    Some(
                        c.subcomponents
                            .iter()
                            .map(|s| {
                                vyna_hl7_core::encoding::Encoding::encode(
                                    s.as_str(),
                                    &__delims,
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("&"),
                    )
                }
            })
    }
}

/// Generate code storing a value into one repetition component,
/// subcomponent-aware: split on `&` and HL7-decode each piece, the
/// inverse of [`component_to_escaped_expr`].
fn set_rep_component_split_tokens(
    comp_idx: usize,
    value_expr: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    quote! {
        while repetition.components.len() <= #comp_idx {
            repetition.add_component(vyna_hl7_core::field::Component::new());
        }
        {
            let __delims = vyna_hl7_core::delimiters::Delimiters::default();
            let mut __comp = vyna_hl7_core::field::Component::new();
            for __sub in (#value_expr).split('&') {
                let __decoded = vyna_hl7_core::encoding::Encoding::decode(
                    __sub, &__delims,
                )
                .unwrap_or_else(|_| __sub.to_string());
                __comp.add_subcomponent(
                    vyna_hl7_core::field::SubComponent::new(__decoded),
                );
            }
            repetition.components[#comp_idx] = __comp;
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
/// - `#[hl7(field = N)]` - Required on every field. Specifies which field
///   number (1-based) this struct field maps to.
/// - `#[hl7(component = N)]` - Specifies which component within the field (1-based).
/// - `#[hl7(optional)]` - Marks the field as optional; requires `Option<T>`
///   (a bare `Option<T>` is optional without the marker).
/// - `#[hl7(required)]` - On a `Vec<T>` field only: total absence fails
///   the conversion instead of yielding an empty `Vec` (scalars are
///   required by default, so `required` on them is an error).
///
/// `subcomponent =` and `repetition =` are not supported and are rejected:
/// map the whole field or a single component instead. Unknown keys and
/// non-positive positions are compile errors, reported at the attribute.
/// Fields 1 and 2 cannot be mapped on `MSH`, `FHS`, or `BHS`: they hold
/// the message delimiters, which encode derives from `Delimiters`.
///
/// # Field types
///
/// - `String` - Required whole field; a missing field fails the conversion.
/// - `Option<String>` - Optional whole field (`None` when the field is absent).
/// - `Vec<String>` - Repeating field: one element per repetition. Cannot be
///   combined with `component=`. Add `required` to fail the conversion
///   when no repetition is present.
///
/// Whole-field values are composite text: repetitions joined by `~`,
/// components by `^`, subcomponents by `&`. Literal separator characters
/// in the data are carried as HL7 escape sequences (`\F\`, `\S\`, `\T\`,
/// `\R\`, `\E\`): reads escape with the standard delimiters and writes
/// decode back, so escaped data survives the round trip. Trailing empty
/// components are trimmed on read.
///
/// A `component = N` mapping reads and writes one component's
/// escape-aware text: subcomponents are HL7-escaped and joined by `&`
/// on read, split and decoded back on write, so composite component
/// text — including a literal `&` carried as `\T\` — survives a
/// no-change round trip.
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
    match derive_segment_impl(input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// Segment derive implementation over `syn` types.
///
/// Invalid input is a spanned `syn::Error` (rendered as `compile_error!`
/// by the entry point), never a panic.
fn derive_segment_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    let segment_attrs = parse_segment_attrs(&input.attrs)?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &data.fields,
                    "Segment derive only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "Segment derive only supports structs",
            ));
        }
    };

    let Some(segment_id) = segment_attrs.id else {
        return Err(syn::Error::new_spanned(
            name,
            "Segment derive requires `#[hl7(id = \"XXX\")]` naming the segment",
        ));
    };

    // Generate field extraction code for from_segment
    let mut from_segment_fields = Vec::new();
    let mut to_segment_fields = Vec::new();
    // Mapped positions, to reject duplicates and whole-field /
    // component overlaps that would silently last-win in to_segment.
    let mut whole_mapped: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();
    let mut component_mapped: std::collections::HashMap<(usize, usize), String> =
        std::collections::HashMap::new();

    for field in fields {
        let field_name = field.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(field, "Segment derive requires named fields")
        })?;
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs)?;

        let Some(field_num) = attrs.field_num else {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "field `{field_name}` needs `#[hl7(field = N)]` to map it \
                     onto the segment",
                ),
            ));
        };

        if matches!(segment_id.as_str(), "MSH" | "FHS" | "BHS") && matches!(field_num, 1 | 2) {
            // MSH/FHS/BHS fields 1 and 2 are the message delimiters:
            // encode derives them from Delimiters and skips the stored
            // fields, so a mapping here would silently drop writes and
            // read the placeholder back. Reject at derive time like the
            // TerserMut::set, template-engine, and codegen guards.
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "field `{field_name}` maps to {segment_id}-{field_num}, \
                     but fields 1 and 2 of {segment_id} hold the message \
                     delimiters and cannot be mapped",
                ),
            ));
        }

        let is_optional = is_option_type(field_type) || attrs.optional;
        let is_repeating = is_vec_type(field_type);

        if attrs.optional && !is_option_type(field_type) {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "field `{field_name}` is marked `optional` but is not \
                     Option<T>",
                ),
            ));
        }

        if attrs.required && !is_repeating {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "field `{field_name}` is marked `required` but is not \
                     Vec<T>; `required` only applies to Vec<T> (scalars \
                     are required by default)",
                ),
            ));
        }

        if attrs.required && is_optional {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "field `{field_name}` cannot combine `required` and \
                     `optional`",
                ),
            ));
        }

        if is_repeating
            && (attrs.component.is_some()
                || attrs.subcomponent.is_some()
                || attrs.repetition.is_some())
        {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "Vec<T> field `{field_name}` supports whole-field \
                     repetitions only; component=, subcomponent= and \
                     repetition= cannot be combined with Vec<T>",
                ),
            ));
        }

        if attrs.subcomponent.is_some() || attrs.repetition.is_some() {
            // Codegen below only addresses whole fields and single
            // components; accepting these markers would silently read
            // and write the wrong location.
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "field `{field_name}` uses subcomponent=/repetition=, \
                     which Segment derive does not support; map the whole \
                     field or a single component instead",
                ),
            ));
        }

        match attrs.component {
            None => {
                if let Some(first) = whole_mapped.get(&field_num) {
                    return Err(syn::Error::new_spanned(
                        field,
                        format!(
                            "duplicate HL7 mapping: field `{field_name}` and field \
                             `{first}` both map to field {field_num}; the second \
                             write would silently overwrite the first",
                        ),
                    ));
                }
                if let Some(((_, comp), first)) =
                    component_mapped.iter().find(|((f, _), _)| *f == field_num)
                {
                    return Err(syn::Error::new_spanned(
                        field,
                        format!(
                            "overlapping HL7 mapping: field `{field_name}` maps to \
                             field {field_num} while field `{first}` maps to field \
                             {field_num} component {comp}; the whole-field write \
                             would silently overwrite the component value",
                        ),
                    ));
                }
                whole_mapped.insert(field_num, field_name.to_string());
            }
            Some(comp) => {
                if let Some(first) = whole_mapped.get(&field_num) {
                    return Err(syn::Error::new_spanned(
                        field,
                        format!(
                            "overlapping HL7 mapping: field `{field_name}` maps to \
                             field {field_num} component {comp} while field `{first}` \
                             maps to field {field_num}; the whole-field write would \
                             silently overwrite the component value",
                        ),
                    ));
                }
                if let Some(first) =
                    component_mapped.insert((field_num, comp), field_name.to_string())
                {
                    return Err(syn::Error::new_spanned(
                        field,
                        format!(
                            "duplicate HL7 mapping: field `{field_name}` and field \
                             `{first}` both map to field {field_num} component {comp}; \
                             the second write would silently overwrite the first",
                        ),
                    ));
                }
            }
        }

        if is_repeating {
            // Repeating field: one Vec element per repetition, each holding
            // the escape-aware composite value (see
            // repetition_to_escaped_tokens).
            let rep_to_str = repetition_to_escaped_tokens();
            let collect = quote! {
                segment.get_field(#field_num)
                    .map(|f| {
                        let __to_str = #rep_to_str;
                        f.repetitions.iter()
                            .map(|r| __to_str(r).unwrap_or_default())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_default()
            };
            if attrs.required {
                // Required repeating field: total absence fails the
                // conversion like a missing required scalar, instead of
                // yielding an empty Vec.
                from_segment_fields.push(quote! {
                    #field_name: match #collect {
                        __vals if __vals.is_empty() => return None,
                        __vals => __vals,
                    }
                });
            } else {
                from_segment_fields.push(quote! {
                    #field_name: #collect
                });
            }

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
            // Extract one component's escape-aware text: every
            // subcomponent HL7-escaped and joined by `&`, so "A&B"
            // reads as "A&B" while one subcomponent holding `R&D`
            // reads as `R\T\D` and a no-change round trip preserves
            // it. A component without subcomponents is still None,
            // like a missing field.
            quote! {
                segment.get_field(#field_num)
                    .and_then(|f| f.get_repetition(0))
                    .and_then(|r| r.get_component(#comp - 1))
                    .and_then(|c| {
                        if c.subcomponents.is_empty() {
                            None
                        } else {
                            let __delims =
                                vyna_hl7_core::delimiters::Delimiters::default();
                            Some(
                                c.subcomponents
                                    .iter()
                                    .map(|s| {
                                        vyna_hl7_core::encoding::Encoding::encode(
                                            s.as_str(),
                                            &__delims,
                                        )
                                    })
                                    .collect::<Vec<String>>()
                                    .join("&"),
                            )
                        }
                    })
            }
        } else {
            // Extract entire field value: every repetition rendered as
            // escape-aware composite text, joined by the repetition
            // separator.
            let rep_to_str = repetition_to_escaped_tokens();
            quote! {
                segment.get_field(#field_num)
                    .and_then(|f| {
                        let __to_str = #rep_to_str;
                        // Component-less repetitions read as "" so
                        // positions are kept ("~SMITH", not "SMITH").
                        // Spelled as a match rather than
                        // unwrap_or_default: identical semantics, and
                        // the scalar expansion keeps its
                        // no-top-level-default token contract (an
                        // absent field still fails via `?` below).
                        let __parts: Vec<String> = f.repetitions.iter()
                            .map(|r| match __to_str(r) {
                                Some(__s) => __s,
                                None => String::new(),
                            })
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
            // A missing required field fails the conversion (?); silently
            // defaulting to "" would hide absent data.
            from_segment_fields.push(quote! {
                #field_name: #extraction?
            });
        }

        // Generate serialization code
        if is_optional {
            if let Some(comp) = attrs.component {
                let store = set_component_split_tokens(field_num, comp, &quote! { val });
                to_segment_fields.push(quote! {
                    if let Some(ref val) = self.#field_name {
                        #store
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
            let store = set_component_split_tokens(field_num, comp, &quote! { &self.#field_name });
            to_segment_fields.push(quote! {
                #store
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

    let expanded = quote! {
        impl #name {
            /// The HL7 segment ID for this type
            pub const SEGMENT_ID: &'static str = #segment_id;

            /// Create from a vyna_hl7_core Segment
            pub fn from_segment(segment: &vyna_hl7_core::segment::Segment) -> Option<Self> {
                if segment.id != #segment_id {
                    return None;
                }

                Some(Self {
                    #(#from_segment_fields),*
                })
            }

            /// Convert to a vyna_hl7_core Segment
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
                // from_segment returns None both for an id mismatch and
                // for a missing required field: branch so an id-matching
                // segment names the real cause instead of reporting
                // "Expected PID segment, got PID".
                if segment.id != #segment_id {
                    return Err(vyna_hl7_core::error::Error::InvalidSegment(
                        format!("Expected {} segment, got {}", #segment_id, segment.id)
                    ));
                }
                Self::from_segment(segment).ok_or_else(|| {
                    vyna_hl7_core::error::Error::MissingRequiredField(format!(
                        "Segment {} is missing required fields",
                        #segment_id
                    ))
                })
            }
        }
    };

    Ok(expanded)
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
/// - `#[hl7(segment)]` - Required on every field. Marks a field as a segment.
/// - `#[hl7(segment, optional)]` - Marks a field as an optional segment;
///   requires `Option<T>`.
/// - `#[hl7(segment, repeating)]` - Marks a field as a repeating segment;
///   requires `Vec<T>`. Add `required` to fail the conversion when no
///   segment is present (scalar segments are required by default, so
///   `required` without `repeating` is an error).
///
/// The markers must match the field type: an `Option<T>` field without
/// `optional`, a `Vec<T>` field without `repeating`, or any field without
/// `segment` is a compile error.
///
/// # Segment matching
///
/// `from_message` matches segments by ID using each field type's `SEGMENT_ID`:
/// required fields take the first match (a missing one fails the conversion),
/// `Option<T>` fields take the first match if present, and `Vec<T>` fields
/// collect every match in order (a `required` one fails the conversion when
/// empty). A present segment that fails conversion
/// fails the whole conversion: invalid-present is never reported as absent
/// (optional) or silently dropped (repeating). Segments sharing one ID
/// across different groups are not distinguished positionally.
///
/// Before matching segments, `from_message` compares a present MSH-9
/// against the declared `message_type`/`trigger_event`: when either
/// component holds content, both must agree (a missing component
/// counts as empty), so an ORU message (even a partial `ORU` without
/// a trigger) cannot convert into an ADT struct. Messages without
/// MSH-9 skip the check.
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
    match derive_message_impl(input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// Message derive implementation over `syn` types.
///
/// Invalid input is a spanned `syn::Error` (rendered as `compile_error!`
/// by the entry point), never a panic.
fn derive_message_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    let message_attrs = parse_message_attrs(&input.attrs)?;
    let message_type = message_attrs.message_type.unwrap_or_default();
    let trigger_event = message_attrs.trigger_event.unwrap_or_default();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &data.fields,
                    "Message derive only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "Message derive only supports structs",
            ));
        }
    };

    let mut from_message_fields = Vec::new();
    let mut to_message_fields = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().ok_or_else(|| {
            syn::Error::new_spanned(field, "Message derive requires named fields")
        })?;
        let field_type = &field.ty;
        let markers = parse_message_field_attrs(&field.attrs)?;
        if !markers.segment {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "message field `{field_name}` must be marked with \
                     `#[hl7(segment)]` (`#[hl7(segment, optional)]` for \
                     Option<T>, `#[hl7(segment, repeating)]` for Vec<T>)",
                ),
            ));
        }
        if markers.optional && markers.repeating {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "message field `{field_name}` cannot combine `optional` \
                     and `repeating`",
                ),
            ));
        }
        let is_optional = is_option_type(field_type);
        let is_vec = is_vec_type(field_type);
        if markers.optional && !is_optional {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "message field `{field_name}` is marked `optional` but is \
                     not Option<T>",
                ),
            ));
        }
        if markers.repeating && !is_vec {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "message field `{field_name}` is marked `repeating` but \
                     is not Vec<T>",
                ),
            ));
        }
        if is_optional && !markers.optional {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "Option<T> message field `{field_name}` must be marked \
                     `#[hl7(segment, optional)]`",
                ),
            ));
        }
        if is_vec && !markers.repeating {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "Vec<T> message field `{field_name}` must be marked \
                     `#[hl7(segment, repeating)]`",
                ),
            ));
        }
        if markers.required && !markers.repeating {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "message field `{field_name}` is marked `required` \
                     without `repeating`; `required` only applies to \
                     `Vec<T>` (scalar segments are required by default)",
                ),
            ));
        }
        if markers.required && !is_vec {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "message field `{field_name}` is marked `required` but \
                     is not Vec<T>",
                ),
            ));
        }

        if is_vec {
            // Repeating segment: every segment with the inner type's ID, in order.
            let inner = inner_generic_type(field_type).ok_or_else(|| {
                syn::Error::new_spanned(
                    field_type,
                    format!(
                        "Repeating segment field `{field_name}` must declare an \
                         element type, e.g. Vec<Observation>",
                    ),
                )
            })?;
            let collect = quote! {
                message.get_segments_by_id(<#inner>::SEGMENT_ID)
                    .into_iter()
                    .map(<#inner>::from_segment)
                    .collect::<Option<Vec<_>>>()?
            };
            if markers.required {
                // Required repeating segment: total absence fails the
                // conversion like a missing required scalar, instead of
                // yielding an empty Vec.
                from_message_fields.push(quote! {
                    #field_name: match #collect {
                        __vals if __vals.is_empty() => return None,
                        __vals => __vals,
                    }
                });
            } else {
                from_message_fields.push(quote! {
                    #field_name: #collect
                });
            }

            to_message_fields.push(quote! {
                for item in &self.#field_name {
                    message.add_segment(item.to_segment());
                }
            });
        } else if is_optional {
            // Optional segment: first segment with the inner type's ID, if any.
            let inner = inner_generic_type(field_type).ok_or_else(|| {
                syn::Error::new_spanned(
                    field_type,
                    format!(
                        "Optional segment field `{field_name}` must declare an \
                         inner type, e.g. Option<PatientVisit>",
                    ),
                )
            })?;
            from_message_fields.push(quote! {
                #field_name: match message.get_segment_by_id(<#inner>::SEGMENT_ID) {
                    None => None,
                    Some(__seg) => Some(<#inner>::from_segment(__seg)?),
                }
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
            /// when a required segment is missing, when any present segment
            /// fails conversion, or when a present MSH-9 disagrees with the
            /// declared message type or trigger event.
            pub fn from_message(message: &vyna_hl7_core::message::Message) -> Option<Self> {
                // A present MSH-9 must agree with the declared type and
                // trigger; each declared value that is empty is not checked.
                // Only MSH-9 without any content skips the check
                // (programmatically built messages carry no MSH): a
                // partial MSH-9 still enforces the values it holds, with
                // a missing component counting as empty.
                {
                    let __msh9 = message
                        .get_segment_by_id("MSH")
                        .and_then(|__msh| __msh.get_field(9))
                        .and_then(|__field| __field.get_repetition(0));
                    let __actual_type = __msh9
                        .and_then(|__rep| __rep.get_component(0))
                        .and_then(|__comp| __comp.value());
                    let __actual_trigger = __msh9
                        .and_then(|__rep| __rep.get_component(1))
                        .and_then(|__comp| __comp.value());
                    let __has_content =
                        __actual_type.is_some_and(|__s| !__s.is_empty())
                            || __actual_trigger.is_some_and(|__s| !__s.is_empty());
                    if __has_content {
                        if !#message_type.is_empty()
                            && __actual_type.unwrap_or("") != #message_type
                        {
                            return None;
                        }
                        if !#trigger_event.is_empty()
                            && __actual_trigger.unwrap_or("") != #trigger_event
                        {
                            return None;
                        }
                    }
                }
                Some(Self {
                    #(#from_message_fields),*
                })
            }

            /// Convert to a vyna_hl7_core Message
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
                if let Some(value) = Self::from_message(message) {
                    return Ok(value);
                }
                // from_message returns None both for an MSH-9 type/trigger
                // mismatch and for missing segments: re-run the MSH-9
                // probe so a mismatched message names the disagreement
                // instead of reporting missing segments that are present.
                let __msh9 = message
                    .get_segment_by_id("MSH")
                    .and_then(|__msh| __msh.get_field(9))
                    .and_then(|__field| __field.get_repetition(0));
                let __actual_type = __msh9
                    .and_then(|__rep| __rep.get_component(0))
                    .and_then(|__comp| __comp.value());
                let __actual_trigger = __msh9
                    .and_then(|__rep| __rep.get_component(1))
                    .and_then(|__comp| __comp.value());
                let __has_content =
                    __actual_type.is_some_and(|__s| !__s.is_empty())
                        || __actual_trigger.is_some_and(|__s| !__s.is_empty());
                let __type_mismatch = __has_content
                    && ((!#message_type.is_empty()
                        && __actual_type.unwrap_or("") != #message_type)
                        || (!#trigger_event.is_empty()
                            && __actual_trigger.unwrap_or("") != #trigger_event));
                if __type_mismatch {
                    return Err(vyna_hl7_core::error::Error::MessageType(format!(
                        "Message type mismatch: expected {}^{}, got {}^{}",
                        #message_type,
                        #trigger_event,
                        __actual_type.unwrap_or(""),
                        __actual_trigger.unwrap_or("")
                    )));
                }
                Err(vyna_hl7_core::error::Error::MessageType(format!(
                    "Message is missing required segments for {}^{}",
                    #message_type, #trigger_event
                )))
            }
        }
    };

    Ok(expanded)
}

/// Attribute macro for defining HL7 data types
///
/// This macro helps define composite HL7 data types like XPN (Extended Person Name),
/// XAD (Extended Address), etc.
///
/// # Attributes
///
/// - `#[hl7_type(data_type = "XXX")]` - Required. Specifies the HL7 data type code.
///
/// Every field needs `#[hl7(component = N)]` (1-based); `optional`
/// requires `Option<T>`. `field =`, `subcomponent =`, and `repetition =`
/// are meaningless here and are rejected, as are unknown keys.
///
/// Component values are escape-aware text like the `Segment`
/// component path: subcomponents are HL7-escaped and joined by `&` on
/// read, split and decoded back on write.
///
/// `from_repetition` returns `None` when a required component is
/// missing, mirroring `Segment::from_segment`.
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
    // Parse the data_type from attribute meta
    let attr_meta: Meta = match syn::parse(attr) {
        Ok(meta) => meta,
        Err(err) => return TokenStream::from(err.to_compile_error()),
    };
    let input = parse_macro_input!(item as DeriveInput);
    match hl7_type_impl(attr_meta, input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

/// hl7_type implementation over `syn` types.
///
/// Invalid input is a spanned `syn::Error` (rendered as `compile_error!`
/// by the entry point), never a panic.
fn hl7_type_impl(attr_meta: Meta, input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    let data_type = match &attr_meta {
        Meta::NameValue(nv) if nv.path.is_ident("data_type") => {
            if let Expr::Lit(expr_lit) = &nv.value
                && let Lit::Str(lit_str) = &expr_lit.lit
            {
                let value = lit_str.value();
                if value.is_empty() {
                    return Err(syn::Error::new_spanned(
                        lit_str,
                        "`data_type` must not be empty",
                    ));
                }
                value
            } else {
                return Err(syn::Error::new_spanned(
                    &nv.value,
                    "expected string literal for `data_type`",
                ));
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                &attr_meta,
                "expected `#[hl7_type(data_type = \"XXX\")]`",
            ));
        }
    };

    let struct_fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields,
            _ => {
                return Err(syn::Error::new_spanned(
                    &data.fields,
                    "hl7_type only supports structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "hl7_type only supports structs",
            ));
        }
    };
    let fields = &struct_fields.named;

    let mut from_rep_fields = Vec::new();
    let mut to_rep_fields = Vec::new();
    // Mapped components, to reject duplicates that would silently
    // last-win in to_repetition, mirroring Segment derive.
    let mut component_mapped: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();

    for field in fields {
        let field_name = field
            .ident
            .as_ref()
            .ok_or_else(|| syn::Error::new_spanned(field, "hl7_type requires named fields"))?;
        let field_type = &field.ty;
        let attrs = parse_field_attrs(&field.attrs)?;
        let is_optional = is_option_type(field_type);

        if attrs.optional && !is_optional {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "field `{field_name}` is marked `optional` but is not \
                     Option<T>",
                ),
            ));
        }
        // `required` is meaningless here (Option-vs-plain already encodes
        // it); silently ignoring it would hide a typo, so reject it like
        // Segment derive rejects required-on-non-Vec.
        if attrs.required {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "hl7_type field `{field_name}` uses `required`, which \
                     hl7_type does not support; use a plain type for \
                     required components and Option<T> for optional ones",
                ),
            ));
        }
        if attrs.field_num.is_some() || attrs.subcomponent.is_some() || attrs.repetition.is_some() {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "hl7_type field `{field_name}` supports only \
                     `component = N` (and `optional`)",
                ),
            ));
        }
        let Some(comp) = attrs.component else {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "hl7_type field `{field_name}` needs \
                     `#[hl7(component = N)]`",
                ),
            ));
        };
        if let Some(first) = component_mapped.insert(comp, field_name.to_string()) {
            return Err(syn::Error::new_spanned(
                field,
                format!(
                    "duplicate HL7 mapping: field `{field_name}` and field \
                     `{first}` both map to component {comp}; the second \
                     write would silently overwrite the first",
                ),
            ));
        }

        let comp_idx = comp - 1; // Convert to 0-based (comp >= 1 validated)

        // Component reads join every subcomponent (escape-aware) like
        // the Segment component path, never just value() (first only).
        let read = component_to_escaped_expr(comp_idx);
        if is_optional {
            from_rep_fields.push(quote! {
                #field_name: #read
            });

            let store = set_rep_component_split_tokens(comp_idx, &quote! { val });
            to_rep_fields.push(quote! {
                if let Some(ref val) = self.#field_name {
                    #store
                }
            });
        } else {
            // A missing required component fails the conversion (?),
            // mirroring Segment derive: silently defaulting to ""
            // would hide absent data.
            from_rep_fields.push(quote! {
                #field_name: #read?
            });

            let store = set_rep_component_split_tokens(comp_idx, &quote! { &self.#field_name });
            to_rep_fields.push(quote! {
                #store
            });
        }
    }

    // Get original struct definition parts, dropping the consumed `hl7`
    // field attributes: the emitted struct carries no derive that
    // registers `hl7` as a helper attribute, so keeping them would fail
    // with "cannot find attribute `hl7` in this scope". Struct-level
    // attributes (derives, docs) and generics are re-emitted, so the
    // expansion keeps the input's traits and type parameters.
    let vis = &input.vis;
    let mut struct_fields = struct_fields.clone();
    for field in struct_fields.named.iter_mut() {
        field.attrs.retain(|attr| !attr.path().is_ident("hl7"));
    }
    let struct_attrs: Vec<&Attribute> = input
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("hl7"))
        .collect();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        #(#struct_attrs)*
        #vis struct #name #impl_generics #struct_fields #where_clause

        impl #impl_generics #name #ty_generics #where_clause {
            /// The HL7 data type code
            pub const DATA_TYPE: &'static str = #data_type;

            /// Create from a vyna_hl7_core Repetition
            ///
            /// Returns `None` when a required component is missing.
            pub fn from_repetition(
                repetition: &vyna_hl7_core::field::Repetition
            ) -> Option<Self> {
                Some(Self {
                    #(#from_rep_fields),*
                })
            }

            /// Convert to a vyna_hl7_core Repetition
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

        impl #impl_generics From<#name #ty_generics>
            for vyna_hl7_core::field::Repetition #where_clause
        {
            fn from(value: #name #ty_generics) -> Self {
                value.to_repetition()
            }
        }

        impl #impl_generics
            std::convert::TryFrom<&vyna_hl7_core::field::Repetition>
            for #name #ty_generics
            #where_clause
        {
            type Error = vyna_hl7_core::error::Error;

            fn try_from(
                repetition: &vyna_hl7_core::field::Repetition
            ) -> Result<Self, Self::Error> {
                Self::from_repetition(repetition).ok_or_else(|| {
                    vyna_hl7_core::error::Error::MissingRequiredField(format!(
                        "Repetition is missing required components for {}",
                        #data_type
                    ))
                })
            }
        }
    };

    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_input(source: &str) -> DeriveInput {
        syn::parse_str(source).expect("test input must parse")
    }

    fn field_attrs_of(source: &str) -> Vec<Attribute> {
        let wrapped = format!("struct S {{ {source} }}");
        let input: DeriveInput = syn::parse_str(&wrapped).expect("test field must parse");
        match &input.data {
            Data::Struct(data) => match &data.fields {
                Fields::Named(fields) => fields.named.first().expect("one field").attrs.clone(),
                _ => panic!("test field must be named"),
            },
            _ => panic!("test wrapper must be a struct"),
        }
    }

    fn err_message(err: syn::Error) -> String {
        err.to_string()
    }

    #[test]
    fn field_attrs_accept_valid_positions() {
        let attrs = field_attrs_of("#[hl7(field = 5, component = 2)] x: String");
        let parsed = parse_field_attrs(&attrs).expect("valid attrs must parse");
        assert_eq!(parsed.field_num, Some(5));
        assert_eq!(parsed.component, Some(2));
        assert!(!parsed.optional);
    }

    #[test]
    fn field_attrs_accept_optional_flag() {
        let attrs = field_attrs_of("#[hl7(field = 1, optional)] x: Option<String>");
        let parsed = parse_field_attrs(&attrs).expect("valid attrs must parse");
        assert_eq!(parsed.field_num, Some(1));
        assert!(parsed.optional);
    }

    #[test]
    fn field_attrs_reject_unknown_key() {
        let attrs = field_attrs_of("#[hl7(field = 1, segment)] x: String");
        let err = parse_field_attrs(&attrs).expect_err("unknown key must fail");
        assert!(err_message(err).contains("unknown hl7 field attribute"));
    }

    #[test]
    fn field_attrs_reject_zero_position() {
        for key in ["field", "component", "subcomponent", "repetition"] {
            let source = format!("#[hl7({key} = 0)] x: String");
            let attrs = field_attrs_of(&source);
            let err = parse_field_attrs(&attrs).expect_err("zero must fail");
            assert!(
                err_message(err).contains("1-based"),
                "key={key}: {}",
                err_message(parse_field_attrs(&attrs).unwrap_err())
            );
        }
    }

    #[test]
    fn field_attrs_reject_mistyped_value() {
        let attrs = field_attrs_of("#[hl7(field = \"x\")] x: String");
        let err = parse_field_attrs(&attrs).expect_err("string must fail");
        assert!(err_message(err).contains("expected integer literal"));
    }

    #[test]
    fn field_attrs_reject_flag_with_value() {
        let attrs = field_attrs_of("#[hl7(field = 1, optional = true)] x: String");
        let err = parse_field_attrs(&attrs).expect_err("valued flag must fail");
        assert!(err_message(err).contains("takes no value"));
    }

    #[test]
    fn segment_attrs_reject_unknown_key_and_bad_id() {
        let input = parse_input("#[hl7(id = \"PID\", bogus)] struct S;");
        let err = parse_segment_attrs(&input.attrs).expect_err("unknown must fail");
        assert!(err_message(err).contains("unknown hl7 segment attribute"));

        let input = parse_input("#[hl7(id = 5)] struct S;");
        let err = parse_segment_attrs(&input.attrs).expect_err("int id must fail");
        assert!(err_message(err).contains("expected string literal"));

        let input = parse_input("#[hl7(id = \"\")] struct S;");
        let err = parse_segment_attrs(&input.attrs).expect_err("empty id must fail");
        assert!(err_message(err).contains("must not be empty"));
    }

    #[test]
    fn message_attrs_reject_unknown_key() {
        let input = parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\", id = \"X\")] struct S;",
        );
        let err = parse_message_attrs(&input.attrs).expect_err("unknown must fail");
        assert!(err_message(err).contains("unknown hl7 message attribute"));
    }

    #[test]
    fn message_field_markers_parse() {
        let attrs = field_attrs_of("#[hl7(segment, optional)] x: Option<S>");
        let markers = parse_message_field_attrs(&attrs).expect("must parse");
        assert!(markers.segment && markers.optional && !markers.repeating);

        let attrs = field_attrs_of("#[hl7(segment, repeating)] x: Vec<S>");
        let markers = parse_message_field_attrs(&attrs).expect("must parse");
        assert!(markers.segment && !markers.optional && markers.repeating);
    }

    #[test]
    fn message_field_markers_reject_field_keys() {
        // Segment-style keys on a message field previously parsed as nothing.
        let attrs = field_attrs_of("#[hl7(segment, field = 3)] x: S");
        let err = parse_message_field_attrs(&attrs).expect_err("field key must fail");
        assert!(err_message(err).contains("unknown hl7 message-field marker"));
    }

    #[test]
    fn segment_impl_rejects_non_structs_and_tuple_structs() {
        let err = derive_segment_impl(parse_input("enum E { A }")).unwrap_err();
        assert!(err_message(err).contains("only supports structs"));

        let err = derive_segment_impl(parse_input("struct S(String);")).unwrap_err();
        assert!(err_message(err).contains("named fields"));
    }

    #[test]
    fn segment_impl_requires_field_marker() {
        let err = derive_segment_impl(parse_input("#[hl7(id = \"PID\")] struct S { x: String }"))
            .unwrap_err();
        assert!(err_message(err).contains("needs `#[hl7(field = N)]`"));
    }

    #[test]
    fn segment_impl_rejects_unsupported_markers() {
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 5, subcomponent = 2)] x: String }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("does not support"));

        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 3, repetition = 2)] x: String }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("does not support"));
    }

    #[test]
    fn segment_impl_rejects_vec_with_component() {
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 3, component = 1)] x: Vec<String> }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("whole-field repetitions only"));
    }

    #[test]
    fn segment_impl_rejects_optional_on_plain_type() {
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 1, optional)] x: String }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("not Option<T>"));
    }

    #[test]
    fn segment_impl_accepts_valid_struct() {
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { #[hl7(field = 3)] x: String }",
        ))
        .expect("valid struct must expand");
        assert!(tokens.to_string().contains("PID"));
    }

    #[test]
    fn message_impl_requires_segment_marker() {
        let err = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] \
             struct M { pid: Pid }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("must be marked"));
    }

    #[test]
    fn message_impl_enforces_marker_type_match() {
        // Option<T> without `optional`.
        let err = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] \
             struct M { #[hl7(segment)] pid: Option<Pid> }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("must be marked"));

        // Vec<T> without `repeating`.
        let err = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] \
             struct M { #[hl7(segment)] obx: Vec<Obx> }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("must be marked"));

        // `optional` on a required segment.
        let err = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] \
             struct M { #[hl7(segment, optional)] pid: Pid }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("not Option<T>"));

        // `optional` + `repeating` together.
        let err = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] \
             struct M { #[hl7(segment, optional, repeating)] obx: Vec<Obx> }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("cannot combine"));
    }

    #[test]
    fn segment_impl_rejects_duplicate_mapping() {
        // Two struct fields reading/writing the same field/component
        // silently last-win in to_segment; reject at derive time.
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 5, component = 1)] a: String, \
                 #[hl7(field = 5, component = 1)] b: String }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("duplicate"));

        // Whole-field duplicates collide too.
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 3)] a: String, \
                 #[hl7(field = 3)] b: Option<String> }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("duplicate"));

        // Same field, different components is fine.
        derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 5, component = 1)] a: String, \
                 #[hl7(field = 5, component = 2)] b: String }",
        ))
        .expect("distinct components must expand");
    }

    #[test]
    fn segment_impl_rejects_whole_component_overlap() {
        // A whole-field mapping plus a component mapping on the same
        // field silently loses the component value in to_segment
        // (the whole-field write overwrites it); reject in both
        // declaration orders.
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 3, component = 1)] a: String, \
                 #[hl7(field = 3)] b: String }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("overlapping"));

        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
                 #[hl7(field = 3)] b: String, \
                 #[hl7(field = 3, component = 1)] a: String }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("overlapping"));
    }

    #[test]
    fn segment_impl_rejects_delimiter_fields_on_msh_fhs_bhs() {
        // Fields 1 and 2 of MSH/FHS/BHS are the message delimiters:
        // encode skips the stored fields, so a mapping would silently
        // drop writes and read the placeholder back.
        for id in ["MSH", "FHS", "BHS"] {
            for num in [1, 2] {
                let source = format!(
                    "#[hl7(id = \"{id}\")] struct S {{ \
                     #[hl7(field = {num})] x: String }}",
                );
                let err = derive_segment_impl(parse_input(&source)).unwrap_err();
                assert!(
                    err_message(err).contains("delimiters"),
                    "id={id} field={num}"
                );
            }
        }

        // Neighboring fields and other segments still expand.
        derive_segment_impl(parse_input(
            "#[hl7(id = \"MSH\")] struct S { #[hl7(field = 3)] x: String }",
        ))
        .expect("MSH field 3 must expand");
        derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { #[hl7(field = 1)] x: String }",
        ))
        .expect("PID field 1 must expand");
    }

    #[test]
    fn segment_impl_required_field_fails_when_absent() {
        // A missing required field must fail conversion (?), never
        // silently become "" via unwrap_or_default().
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { #[hl7(field = 3)] x: String }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string();
        assert!(
            !rendered.contains("unwrap_or_default"),
            "required field must not default: {rendered}"
        );
    }

    #[test]
    fn segment_impl_whole_field_round_trip_is_escape_aware() {
        // Whole-field values are joined composite text: reads must escape
        // literal separators and writes must decode them back, or a
        // literal '^' in the data becomes structure on re-split.
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { #[hl7(field = 5)] x: String }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string().replace(' ', "");
        assert!(
            rendered.contains("Encoding::encode"),
            "whole-field read must escape: {rendered}"
        );
        assert!(
            rendered.contains("Encoding::decode"),
            "whole-field write must decode: {rendered}"
        );
    }

    #[test]
    fn message_impl_accepts_valid_struct() {
        let tokens = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] struct M { \
             #[hl7(segment)] pid: Pid, \
             #[hl7(segment, optional)] pv1: Option<Pv1>, \
             #[hl7(segment, repeating)] obx: Vec<Obx> }",
        ))
        .expect("valid struct must expand");
        assert!(tokens.to_string().contains("MESSAGE_TYPE"));
    }

    #[test]
    fn message_impl_validates_msh9_type_and_trigger() {
        // from_message must check MSH-9 against the declared message
        // type/trigger so an ORU message cannot convert into an ADT struct.
        let tokens = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] struct M { \
             #[hl7(segment)] pid: Pid }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string().replace(' ', "");
        assert!(
            rendered.contains("get_component(0)") && rendered.contains("get_component(1)"),
            "from_message must validate MSH-9 components: {rendered}"
        );
    }

    #[test]
    fn message_impl_enforces_partial_msh9_agreement() {
        // get_message_type returns None for a partial MSH-9 (type
        // without trigger), which skipped the check entirely; the
        // generated code must read the components structurally so a
        // partial "ORU" still fails against ADT/A01.
        let tokens = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] struct M { \
             #[hl7(segment)] pid: Pid }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string().replace(' ', "");
        assert!(
            !rendered.contains("get_message_type"),
            "partial MSH-9 must not skip the check: {rendered}"
        );
        assert!(
            rendered.contains("has_content"),
            "any MSH-9 content must trigger agreement: {rendered}"
        );
    }

    #[test]
    fn hl7_type_impl_validates_attr_and_fields() {
        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let tokens = hl7_type_impl(
            meta,
            parse_input("struct N { #[hl7(component = 1)] x: String }"),
        )
        .expect("valid input must expand");
        assert!(tokens.to_string().contains("DATA_TYPE"));

        let meta: Meta = syn::parse_str("data_type = \"\"").unwrap();
        let err = hl7_type_impl(
            meta,
            parse_input("struct N { #[hl7(component = 1)] x: String }"),
        )
        .unwrap_err();
        assert!(err_message(err).contains("must not be empty"));

        let meta: Meta = syn::parse_str("bogus = \"XPN\"").unwrap();
        let err = hl7_type_impl(
            meta,
            parse_input("struct N { #[hl7(component = 1)] x: String }"),
        )
        .unwrap_err();
        assert!(err_message(err).contains("expected `#[hl7_type"));

        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let err = hl7_type_impl(
            meta,
            parse_input("struct N { #[hl7(field = 1)] x: String }"),
        )
        .unwrap_err();
        assert!(err_message(err).contains("supports only"));

        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let err = hl7_type_impl(meta, parse_input("struct N { x: String }")).unwrap_err();
        assert!(err_message(err).contains("needs `#[hl7(component = N)]`"));
    }

    #[test]
    fn hl7_type_impl_required_component_fails_when_absent() {
        // A missing required component must fail the conversion,
        // mirroring Segment derive: never silently become "".
        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let tokens = hl7_type_impl(
            meta,
            parse_input("struct N { #[hl7(component = 1)] x: String }"),
        )
        .expect("valid input must expand");
        let rendered = tokens.to_string();
        assert!(
            !rendered.contains("unwrap_or_default"),
            "required component must not default: {rendered}"
        );
        let compact = rendered.replace(' ', "");
        assert!(
            compact.contains("->Option<Self>"),
            "from_repetition must return Option: {rendered}"
        );
        assert!(
            compact.contains("TryFrom"),
            "borrowed conversion must be fallible: {rendered}"
        );
    }

    #[test]
    fn segment_impl_requires_id_attribute() {
        // `id` is documented as Required: omitting it must fail the
        // derive instead of silently defaulting to the struct name.
        let err = derive_segment_impl(parse_input(
            "struct Pid { #[hl7(field = 1)] x: Option<String> }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("id"));
    }

    #[test]
    fn message_impl_repeating_propagates_segment_failure() {
        // A failing repetition must fail the whole conversion (?),
        // never be silently dropped by filter_map.
        let tokens = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ORU\", trigger_event = \"R01\")] struct M { \
             #[hl7(segment)] pid: Pid, \
             #[hl7(segment, repeating)] obx: Vec<Obx> }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string();
        assert!(
            !rendered.contains("filter_map"),
            "repeating segments must not be dropped: {rendered}"
        );
        assert!(
            rendered
                .replace(' ', "")
                .contains("collect::<Option<Vec<_>>>()?"),
            "repeating segments must propagate failure: {rendered}"
        );
    }

    #[test]
    fn message_impl_optional_propagates_present_failure() {
        // A present-but-invalid optional segment must fail the whole
        // conversion (?), never be swallowed into None by and_then.
        let tokens = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] struct M { \
             #[hl7(segment, optional)] pv1: Option<Pv1> }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string();
        let compact = rendered.replace(' ', "");
        assert!(
            !compact.contains("and_then(<Pv1>::from_segment)"),
            "present failure must not become absent: {rendered}"
        );
        assert!(
            compact.contains("Some(<Pv1>::from_segment(__seg)?)"),
            "present failure must propagate: {rendered}"
        );
    }

    #[test]
    fn segment_impl_required_vec_fails_when_empty() {
        // A required repeating field must fail the conversion when no
        // repetition is present, instead of yielding an empty Vec.
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
             #[hl7(field = 3, required)] ids: Vec<String> }",
        ))
        .expect("valid struct must expand");
        let compact = tokens.to_string().replace(' ', "");
        assert!(
            compact.contains("is_empty()=>returnNone"),
            "required Vec must fail when empty: {compact}"
        );

        // Without `required` the empty Vec stands: no fail-on-empty
        // guard is emitted.
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
             #[hl7(field = 3)] ids: Vec<String> }",
        ))
        .expect("valid struct must expand");
        let compact = tokens.to_string().replace(' ', "");
        assert!(
            !compact.contains("if__vals.is_empty"),
            "plain repeating Vec must stay total: {compact}"
        );
    }

    #[test]
    fn segment_impl_required_rejects_scalar_and_optional() {
        // Scalars are required by default; `required` on them is an error.
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
             #[hl7(field = 3, required)] id: String }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("required"));

        // `required` contradicts `Option`: a required field cannot be
        // optional.
        let err = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
             #[hl7(field = 5, required)] x: Option<String> }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("required"));
    }

    #[test]
    fn message_impl_required_repeating_fails_when_empty() {
        // A required repeating segment must fail the conversion when no
        // segment is present, instead of yielding an empty Vec.
        let tokens = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ORU\", trigger_event = \"R01\")] struct M { \
             #[hl7(segment)] pid: Pid, \
             #[hl7(segment, repeating, required)] obx: Vec<Obx> }",
        ))
        .expect("valid struct must expand");
        let compact = tokens.to_string().replace(' ', "");
        assert!(
            compact.contains("is_empty()=>returnNone"),
            "required repeating must fail when empty: {compact}"
        );
    }

    #[test]
    fn hl7_type_impl_preserves_attrs_and_generics() {
        // The expansion must re-emit struct attributes (derives, docs)
        // and generics: dropping them silently un-derives traits and
        // breaks generic data types.
        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let tokens = hl7_type_impl(
            meta,
            parse_input(
                "#[derive(Debug, Clone, PartialEq)]\n\
                 /// Family name holder.\n\
                 pub struct N<T> where T: Clone {\n\
                 #[hl7(component = 1)] x: T }",
            ),
        )
        .expect("valid input must expand");
        let rendered = tokens.to_string();
        assert!(
            rendered.contains("derive"),
            "derive attr must survive: {rendered}"
        );
        assert!(
            rendered.contains("Debug"),
            "derive list must survive: {rendered}"
        );
        assert!(
            rendered.contains("doc"),
            "doc comment must survive: {rendered}"
        );
        let compact = rendered.replace(' ', "");
        assert!(
            compact.contains("structN<T>"),
            "generic params must survive: {rendered}"
        );
        assert!(
            compact.contains("whereT:Clone"),
            "where clause must survive: {rendered}"
        );
        assert!(
            compact.contains("impl<T>N<T>"),
            "impl blocks must carry generics: {rendered}"
        );
        assert!(
            !compact.contains("#[hl7("),
            "consumed hl7 markers must stay stripped: {rendered}"
        );
    }

    #[test]
    fn segment_impl_try_from_distinguishes_missing_required_field() {
        // An id-matching segment that lacks a required field must not
        // report "Expected PID segment, got PID": the id matched, so
        // the error must name the real cause.
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { #[hl7(field = 3)] x: String }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string();
        assert!(
            rendered.contains("Expected {} segment, got {}"),
            "id mismatch keeps its message: {rendered}"
        );
        assert!(
            rendered.contains("MissingRequiredField"),
            "matched id with missing field needs a cause error: {rendered}"
        );
    }

    #[test]
    fn message_impl_try_from_distinguishes_type_mismatch() {
        // A message whose MSH-9 disagrees with the declared type must
        // not report "missing required segments" when none is missing.
        let tokens = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] struct M { \
             #[hl7(segment)] pid: Pid }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string();
        assert!(
            rendered.contains("missing required segments"),
            "absent segments keep their message: {rendered}"
        );
        assert!(
            rendered.contains("mismatch"),
            "type/trigger disagreement needs its own error: {rendered}"
        );
    }

    #[test]
    fn segment_impl_component_mapping_preserves_subcomponents() {
        // A component=N mapping reads one component's full decoded text
        // (subcomponents joined by &) and writes it back
        // subcomponent-aware, so "A&B" survives a no-change round trip.
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
             #[hl7(field = 5, component = 1)] x: String }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string();
        assert!(
            rendered.contains("join"),
            "component read must join subcomponents: {rendered}"
        );
        assert!(
            rendered.contains("split"),
            "component write must split subcomponents: {rendered}"
        );
    }

    #[test]
    fn segment_impl_component_mapping_is_escape_aware() {
        // A component=N mapping must escape subcomponents on read and
        // decode on write (mirroring the whole-field path), or a
        // single subcomponent holding `R&D` rewrites as two.
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct S { \
             #[hl7(field = 5, component = 1)] x: String }",
        ))
        .expect("valid struct must expand");
        let rendered = tokens.to_string().replace(' ', "");
        assert!(
            rendered.contains("Encoding::encode"),
            "component read must escape: {rendered}"
        );
        assert!(
            rendered.contains("Encoding::decode"),
            "component write must decode: {rendered}"
        );
    }

    #[test]
    fn hl7_type_impl_component_mapping_is_escape_aware() {
        // from_repetition must join every subcomponent (escape-aware)
        // instead of reading value() (first only), and to_repetition
        // must split them back, mirroring the Segment component path.
        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let tokens = hl7_type_impl(
            meta,
            parse_input("struct N { #[hl7(component = 1)] x: String }"),
        )
        .expect("valid input must expand");
        let rendered = tokens.to_string();
        assert!(
            rendered.contains("subcomponents"),
            "hl7_type read must join subcomponents: {rendered}"
        );
        let compact = rendered.replace(' ', "");
        assert!(
            compact.contains("Encoding::encode"),
            "hl7_type read must escape: {rendered}"
        );
        assert!(
            compact.contains("Encoding::decode"),
            "hl7_type write must decode: {rendered}"
        );
    }

    #[test]
    fn message_impl_required_rejects_scalar_and_optional() {
        // `required` without `repeating` is an error: scalar segments
        // are required by default.
        let err = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] struct M { \
             #[hl7(segment, required)] pid: Pid }",
        ))
        .unwrap_err();
        assert!(err_message(err).contains("required"));

        // `required` contradicts `optional`.
        let err = derive_message_impl(parse_input(
            "#[hl7(message_type = \"ADT\", trigger_event = \"A01\")] struct M { \
             #[hl7(segment, optional, repeating, required)] pv1: Vec<Pv1> }",
        ))
        .unwrap_err();
        let msg = err_message(err);
        assert!(
            msg.contains("optional") || msg.contains("required"),
            "{msg}"
        );
    }

    #[test]
    fn hl7_type_impl_rejects_duplicate_component() {
        // Two fields on one component silently last-win in to_repetition;
        // reject at expansion like Segment derive.
        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let err = hl7_type_impl(
            meta,
            parse_input(
                "struct N { #[hl7(component = 1)] a: String, \
                 #[hl7(component = 1)] b: String }",
            ),
        )
        .unwrap_err();
        assert!(err_message(err).contains("duplicate"));
    }

    #[test]
    fn hl7_type_impl_rejects_required_flag() {
        // `required` is silently ignored today; Option-vs-plain already
        // encodes it, so the flag is a typo to reject.
        let meta: Meta = syn::parse_str("data_type = \"XPN\"").unwrap();
        let err = hl7_type_impl(
            meta,
            parse_input("struct N { #[hl7(component = 1, required)] x: Option<String> }"),
        )
        .unwrap_err();
        assert!(err_message(err).contains("required"));
    }

    #[test]
    fn segment_repetition_reads_keep_component_less_positions() {
        // Component-less repetitions must read as "" (positions kept),
        // not be dropped: filtering them shifts Vec positions (PID-5
        // with reps ["", "SMITH"] read back as ["SMITH"]).
        let vec_tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct P { #[hl7(field = 5)] n: Vec<String> }",
        ))
        .expect("valid segment must expand")
        .to_string();
        assert!(
            vec_tokens.contains("unwrap_or_default"),
            "Vec read must default empty reps: {vec_tokens}"
        );
        assert!(
            !vec_tokens.contains("filter_map"),
            "Vec read must not drop reps: {vec_tokens}"
        );
        let scalar_tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct P { #[hl7(field = 5)] n: String }",
        ))
        .expect("valid segment must expand")
        .to_string();
        assert!(
            !scalar_tokens.contains("filter_map"),
            "scalar read must not drop reps: {scalar_tokens}"
        );
        assert!(
            scalar_tokens
                .replace(' ', "")
                .contains("None=>String::new()"),
            "scalar read must keep empty reps: {scalar_tokens}"
        );
    }

    #[test]
    fn wrapper_types_accept_qualified_std_paths() {
        for path in [
            "Option<String>",
            "std::option::Option<String>",
            "core::option::Option<String>",
            "alloc::option::Option<String>",
            "::std::option::Option<String>",
        ] {
            let ty: Type = syn::parse_str(path).expect("type must parse");
            assert!(is_option_type(&ty), "{path} must read as Option");
            assert!(!is_vec_type(&ty), "{path} must not read as Vec");
        }
        for path in [
            "Vec<String>",
            "std::vec::Vec<String>",
            "alloc::vec::Vec<String>",
            "core::vec::Vec<String>",
            "::alloc::vec::Vec<String>",
        ] {
            let ty: Type = syn::parse_str(path).expect("type must parse");
            assert!(is_vec_type(&ty), "{path} must read as Vec");
            assert!(!is_option_type(&ty), "{path} must not read as Option");
        }
        // Same-named foreign types are not std wrappers.
        for path in ["String", "mycrate::Option<String>", "foo::Vec<String>"] {
            let ty: Type = syn::parse_str(path).expect("type must parse");
            assert!(!is_option_type(&ty), "{path} must not read as Option");
            assert!(!is_vec_type(&ty), "{path} must not read as Vec");
        }
    }

    #[test]
    fn segment_derive_expands_qualified_wrappers() {
        // A qualified Option must expand with the optional shape
        // (`if let Some`), not the required-scalar shape (`?`), which
        // fails downstream with E0308/E0599.
        for ty in ["Option<String>", "std::option::Option<String>"] {
            let tokens = derive_segment_impl(parse_input(&format!(
                "#[hl7(id = \"PID\")] struct P {{ #[hl7(field = 5)] n: {ty} }}",
            )))
            .expect("valid segment must expand")
            .to_string();
            assert!(
                tokens.contains("if let Some"),
                "{ty} must expand as optional: {tokens}"
            );
        }
        // A qualified Vec must expand with the repeating shape.
        let tokens = derive_segment_impl(parse_input(
            "#[hl7(id = \"PID\")] struct P { #[hl7(field = 5)] n: std::vec::Vec<String> }",
        ))
        .expect("valid segment must expand")
        .to_string();
        assert!(
            tokens.contains("iter ()"),
            "qualified Vec must expand as repeating: {tokens}"
        );
    }
}
