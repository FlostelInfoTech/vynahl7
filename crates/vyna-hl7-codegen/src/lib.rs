//! Schema-driven code generation for HAPI-style typed HL7 models.
//!
//! This crate renders the JSON message schemas embedded in
//! [`vyna_hl7_validator`] as Rust structs annotated with the
//! `vyna_hl7_macros` derive macros, giving every message a strongly-typed
//! API in the spirit of HAPI's generated model classes.
//!
//! ```rust
//! use vyna_hl7_codegen::generate_for;
//! use vyna_hl7_core::Version;
//!
//! let source = generate_for(Version::V2_5, "ADT", "A01").unwrap();
//! assert!(source.contains("pub struct AdtA01"));
//! assert!(source.contains("pub struct Pid"));
//! ```
//!
//! ## Mapping rules
//!
//! - One struct per segment (`PID` -> `Pid`), one struct per message
//!   (`ADT` + `A01` -> `AdtA01`), using the `Segment` / `Message` derives.
//! - Required fields become `String`, optional fields `Option<String>`,
//!   repeating fields `Vec<String>`; required repeating fields and
//!   segments carry `required` so absence fails the conversion.
//! - Composite fields whose schema defines components map one struct field
//!   per component; repeating composite fields fall back to whole-field
//!   `Vec<String>` because `Vec` cannot combine with `component=`.
//! - Segment order follows a canonical HL7 order with `MSH` first so that
//!   `to_message` output stays valid; unknown segments sort alphabetically
//!   after the known ones.

use std::collections::HashSet;

use vyna_hl7_core::{Version, error::Result};
use vyna_hl7_validator::{
    ComponentDefinition, FieldDefinition, MessageSchema, SegmentDefinition, load_schema,
};

/// Canonical segment order for generated message structs.
///
/// The validator schemas store segments in a map without order information,
/// so codegen imposes this deterministic order. `MSH` must stay first:
/// parsers and validators reject messages that do not start with it.
const SEGMENT_ORDER: &[&str] = &[
    "MSH", "SFT", "EVN", "SCH", "RGS", "PID", "PD1", "ROL", "NK1", "PV1", "PV2", "DB1",
    "TXA", "ORC", "OBR", "OBX", "AL1", "DG1", "DRG", "PR1", "PRB", "GT1", "IN1", "IN2",
    "IN3", "ACC", "UB1", "UB2", "PDA", "RXO", "RXE", "RXR", "RXC", "RXD", "RXG", "RXA",
    "TQ1", "RDS", "RDE", "RGV", "RRD", "RRA", "RAS", "MSA", "ERR", "QAK", "QPD", "RCP",
    "DSC", "FHS", "BHS", "FTS", "BTS", "NTE",
];

/// Strict and reserved Rust keywords that a generated identifier may collide with.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while", "abstract", "become", "box", "do", "final", "macro", "override", "priv",
    "try", "typeof", "unsized", "virtual", "yield",
];

/// Sort key for segment IDs: canonical position first, then alphabetical.
fn segment_rank(id: &str) -> (usize, &str) {
    match SEGMENT_ORDER.iter().position(|known| *known == id) {
        Some(index) => (index, id),
        None => (usize::MAX, id),
    }
}

/// Convert an HL7 element name ("Patient Identifier List") to snake_case.
pub fn to_snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut need_separator = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if need_separator && !out.is_empty() {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            need_separator = false;
        } else {
            need_separator = true;
        }
    }
    out
}

/// Capitalize a segment ID or message type ("PID" -> "Pid", "ADT" -> "Adt").
fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = String::with_capacity(word.len());
            out.extend(first.to_uppercase());
            out.push_str(&chars.as_str().to_lowercase());
            out
        }
        None => String::new(),
    }
}

/// Keep only characters valid in a Rust identifier.
///
/// Schema text is caller-controlled; anything else would break out of
/// the struct-name position and inject arbitrary items or attributes.
fn sanitize_type_chars(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect()
}

/// Struct name for a segment ID (`PID` -> `Pid`).
pub fn segment_struct_name(segment_id: &str) -> String {
    let name = capitalize(&sanitize_type_chars(segment_id));
    if name == "Self" {
        "SegmentSelf".to_string()
    } else if !name.chars().any(|c| c.is_ascii_alphanumeric()) {
        "Unknown".to_string()
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Segment{name}")
    } else {
        name
    }
}

/// Struct name for a message (`ADT` + `A01` -> `AdtA01`, `ACK` -> `Ack`).
pub fn message_struct_name(message_type: &str, trigger_event: &str) -> String {
    let name = format!(
        "{}{}",
        capitalize(&sanitize_type_chars(message_type)),
        sanitize_type_chars(trigger_event)
    );
    if name == "Self" {
        "MessageSelf".to_string()
    } else if !name.chars().any(|c| c.is_ascii_alphanumeric()) {
        "UnknownMessage".to_string()
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("Message{name}")
    } else {
        name
    }
}

/// Turn a raw HL7 element name into a unique, compilable struct field name.
///
/// Falls back to `fallback` when the name carries no usable characters,
/// prefixes digit-leading names, escapes Rust keywords with `r#`, and appends
/// `_2`, `_3`, ... on collision within the same struct.
pub fn sanitize_ident(base: &str, fallback: &str, used: &mut HashSet<String>) -> String {
    let mut name = to_snake_case(base);
    if name.is_empty() {
        // The fallback can itself carry schema text (e.g. a raw segment
        // ID), so it goes through the same sanitization.
        name = to_snake_case(fallback);
        if name.is_empty() {
            name = "field".to_string();
        }
    }
    if name.starts_with(|c: char| c.is_ascii_digit()) {
        name = format!("field_{name}");
    }
    if RUST_KEYWORDS.contains(&name.as_str()) {
        name = format!("r#{name}");
    }
    let mut candidate = name.clone();
    let mut counter = 2;
    while used.contains(&candidate) {
        candidate = format!("{name}_{counter}");
        counter += 1;
    }
    used.insert(candidate.clone());
    candidate
}

/// Strip line breaks from schema text rendered into generated comments.
///
/// Schemas are caller-controlled (custom JSON, `Validator::with_schema`);
/// a newline in a name would terminate the `///` doc comment and inject
/// arbitrary lines into the generated module.
fn sanitize_comment_text(text: &str) -> String {
    text.chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
        .collect()
}

/// Escape schema text rendered inside a `"..."` attribute string.
///
/// A `"` or `\` would terminate the string literal and inject arbitrary
/// tokens into the generated module; control characters would split it
/// across lines. Escapes decode back to the original value, so runtime
/// matching on the attribute still sees the real schema text.
fn escape_attr_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out
}

/// Render one `#[derive(Segment)]` struct from a schema segment definition.
///
/// Schema field numbers are trusted: the schema author is the code
/// operator (unlike templates, schemas are not an untrusted-input
/// boundary), so no cap is applied even though large numbers gap-fill.
pub fn generate_segment(segment_id: &str, def: &SegmentDefinition) -> String {
    generate_segment_with_name(segment_id, def, &segment_struct_name(segment_id))
}

/// Render one segment struct under an explicit struct name.
///
/// `generate_module` uses this to suffix colliding struct names
/// (`Pid`, `Pid_2`) so hostile schemas cannot emit duplicate items.
fn generate_segment_with_name(
    segment_id: &str,
    def: &SegmentDefinition,
    struct_name: &str,
) -> String {
    let mut used = HashSet::new();
    let mut body = String::new();

    let mut numbers: Vec<usize> = def.fields.keys().copied().collect();
    numbers.sort_unstable();
    for number in numbers {
        if segment_id == "MSH" && (number == 1 || number == 2) {
            // MSH-1/2 hold the message delimiters, which live on the
            // message: encode derives them from Delimiters (skipping
            // stored fields) and TerserMut::set rejects writes there,
            // so a generated mapping would fail from_segment on real
            // messages and silently drop edits.
            continue;
        }
        let field = &def.fields[&number];
        if !field.repeating
            && let Some(components) = field.components.as_ref()
        {
            let mut keys: Vec<(&String, &ComponentDefinition)> = components.iter().collect();
            keys.sort_by_key(|(key, _)| key.parse::<usize>().unwrap_or(usize::MAX));
            if !keys.is_empty() {
                let mut parsed = 0;
                for (key, component) in keys {
                    let Some(component_number) = key.parse::<usize>().ok().filter(|n| *n > 0)
                    else {
                        continue;
                    };
                    parsed += 1;
                    render_component_field(
                        &mut body,
                        segment_id,
                        number,
                        field,
                        component_number,
                        component,
                        &mut used,
                    );
                }
                if parsed > 0 {
                    continue;
                }
                // No component key parsed: fall through to the
                // whole-field render instead of emitting no field.
            }
        }
        render_whole_field(&mut body, segment_id, number, field, &mut used);
    }

    let mut out = String::new();
    out.push_str(&format!(
        "/// {} - {}\n",
        sanitize_comment_text(segment_id),
        sanitize_comment_text(&def.name)
    ));
    out.push_str("#[derive(vyna_hl7_macros::Segment, Debug, Clone, Default, PartialEq)]\n");
    out.push_str(&format!(
        "#[hl7(id = \"{}\")]\n",
        escape_attr_string(segment_id)
    ));
    out.push_str(&format!("pub struct {struct_name} {{\n"));
    out.push_str(&body);
    out.push_str("}\n");
    out
}

/// Render a struct field declaration, wrapping the type onto its own line when
/// the single-line form would exceed rustfmt's 100-column limit.
fn render_field_decl(body: &mut String, ident: &str, ty: &str) {
    let single = format!("    pub {ident}: {ty},");
    if single.len() > 100 {
        body.push_str(&format!("    pub {ident}:\n        {ty},\n"));
    } else {
        body.push_str(&single);
        body.push('\n');
    }
}

/// Render one struct field per schema component of a composite field.
#[allow(clippy::too_many_arguments)]
fn render_component_field(
    body: &mut String,
    segment_id: &str,
    field_number: usize,
    field: &FieldDefinition,
    component_number: usize,
    component: &ComponentDefinition,
    used: &mut HashSet<String>,
) {
    let base = format!("{}_{}", field.name, component.name);
    let fallback = format!("field_{field_number}_{component_number}");
    let ident = sanitize_ident(&base, &fallback, used);
    // A component is required only when its field is: an absent optional
    // field carries no components, so marking them required would reject
    // real sparse messages.
    let ty = if component.required && field.required {
        "String"
    } else {
        "Option<String>"
    };
    body.push_str(&format!(
        "    /// {segment_id}-{field_number}.{component_number} {} / {} ({})\n",
        sanitize_comment_text(&field.name),
        sanitize_comment_text(&component.name),
        sanitize_comment_text(&component.data_type)
    ));
    body.push_str(&format!(
        "    #[hl7(field = {field_number}, component = {component_number})]\n"
    ));
    render_field_decl(body, &ident, ty);
}

/// Render one struct field holding a whole HL7 field value.
fn render_whole_field(
    body: &mut String,
    segment_id: &str,
    field_number: usize,
    field: &FieldDefinition,
    used: &mut HashSet<String>,
) {
    let fallback = format!("field_{field_number}");
    let ident = sanitize_ident(&field.name, &fallback, used);
    let ty = if field.repeating {
        "Vec<String>"
    } else if field.required {
        "String"
    } else {
        "Option<String>"
    };
    let mut flags = sanitize_comment_text(&field.data_type);
    if field.repeating {
        flags.push_str(", repeating");
    }
    body.push_str(&format!(
        "    /// {segment_id}-{field_number} {} ({flags})\n",
        sanitize_comment_text(&field.name)
    ));
    if field.required && field.repeating {
        // Required repeating fields fail conversion when absent, like
        // required scalars, instead of yielding an empty Vec.
        body.push_str(&format!("    #[hl7(field = {field_number}, required)]\n"));
    } else {
        body.push_str(&format!("    #[hl7(field = {field_number})]\n"));
    }
    render_field_decl(body, &ident, ty);
}

/// Render a complete module: every segment struct plus the message struct.
///
/// Schema field numbers are trusted: the schema author is the code
/// operator (unlike templates, schemas are not an untrusted-input
/// boundary), so no cap is applied even though large numbers gap-fill.
pub fn generate_module(schema: &MessageSchema) -> String {
    let mut ids: Vec<&String> = schema.segments.keys().collect();
    ids.sort_by(|a, b| segment_rank(a).cmp(&segment_rank(b)));

    // Struct names collide when two ids sanitize alike ("PID", "P-ID"
    // both yield `Pid`): suffix collisions (`Pid_2`) like sanitize_ident
    // so the module still compiles. Message fields use the same map.
    let mut used_struct_names = HashSet::new();
    let mut struct_names: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for id in &ids {
        let base = segment_struct_name(id);
        let mut candidate = base.clone();
        let mut counter = 2;
        while used_struct_names.contains(&candidate) {
            candidate = format!("{base}_{counter}");
            counter += 1;
        }
        used_struct_names.insert(candidate.clone());
        struct_names.insert((*id).clone(), candidate);
    }

    let mut out = String::new();
    out.push_str("// Code generated by vyna-hl7-codegen - DO NOT EDIT.\n");
    out.push_str(&format!(
        "// Source: {}^{} (HL7 v{})\n\n",
        sanitize_comment_text(&schema.message_type),
        sanitize_comment_text(&schema.trigger_event),
        sanitize_comment_text(&schema.version)
    ));
    for id in &ids {
        let struct_name = &struct_names[id.as_str()];
        out.push_str(&generate_segment_with_name(
            id,
            &schema.segments[id.as_str()],
            struct_name,
        ));
        out.push('\n');
    }

    let struct_name = message_struct_name(&schema.message_type, &schema.trigger_event);
    let mut used = HashSet::new();
    out.push_str(&format!(
        "/// {}^{} (HL7 v{})\n",
        sanitize_comment_text(&schema.message_type),
        sanitize_comment_text(&schema.trigger_event),
        sanitize_comment_text(&schema.version)
    ));
    out.push_str("#[derive(vyna_hl7_macros::Message, Debug, Clone, Default, PartialEq)]\n");
    out.push_str(&format!(
        "#[hl7(message_type = \"{}\", trigger_event = \"{}\")]\n",
        escape_attr_string(&schema.message_type),
        escape_attr_string(&schema.trigger_event)
    ));
    out.push_str(&format!("pub struct {struct_name} {{\n"));
    for id in &ids {
        let def = &schema.segments[id.as_str()];
        let ident = sanitize_ident(id, &format!("segment_{}", id.to_lowercase()), &mut used);
        let target = &struct_names[id.as_str()];
        // Required repeating segments fail conversion when absent,
        // like required scalars, instead of yielding an empty Vec.
        let (attr, ty) = if def.repeating {
            let marker = if def.required {
                "repeating, required"
            } else {
                "repeating"
            };
            (marker, format!("Vec<{target}>"))
        } else if def.required {
            ("", target.to_string())
        } else {
            ("optional", format!("Option<{target}>"))
        };
        out.push_str(&format!(
            "    /// {} - {}\n",
            sanitize_comment_text(id),
            sanitize_comment_text(&def.name)
        ));
        if attr.is_empty() {
            out.push_str("    #[hl7(segment)]\n");
        } else {
            out.push_str(&format!("    #[hl7(segment, {attr})]\n"));
        }
        render_field_decl(&mut out, &ident, &ty);
    }
    out.push_str("}\n");
    out
}

/// Load the schema for a message and render its typed module.
pub fn generate_for(version: Version, message_type: &str, trigger_event: &str) -> Result<String> {
    let schema = load_schema(version, message_type, trigger_event)?;
    Ok(generate_module(&schema))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn snake_case_converts_names() {
        assert_eq!(
            to_snake_case("Patient Identifier List"),
            "patient_identifier_list"
        );
        assert_eq!(to_snake_case("Date/Time of Birth"), "date_time_of_birth");
        assert_eq!(to_snake_case("  X-Ray  (Portable) "), "x_ray_portable");
        assert_eq!(to_snake_case("MSH-9"), "msh_9");
        assert_eq!(to_snake_case("---"), "");
    }

    #[test]
    fn struct_names_follow_rust_conventions() {
        assert_eq!(segment_struct_name("PID"), "Pid");
        assert_eq!(segment_struct_name("MSH"), "Msh");
        assert_eq!(message_struct_name("ADT", "A01"), "AdtA01");
        assert_eq!(message_struct_name("ACK", ""), "Ack");
    }

    #[test]
    fn sanitize_handles_keywords_digits_and_collisions() {
        let mut used = HashSet::new();
        assert_eq!(sanitize_ident("Type", "field_1", &mut used), "r#type");
        assert_eq!(
            sanitize_ident("1st Visit", "field_2", &mut used),
            "field_1st_visit"
        );
        assert_eq!(sanitize_ident("!!!", "field_3", &mut used), "field_3");
        assert_eq!(sanitize_ident("Name", "field_4", &mut used), "name");
        assert_eq!(sanitize_ident("Name", "field_5", &mut used), "name_2");
    }

    #[test]
    fn canonical_order_keeps_msh_first() {
        let mut ids = ["PID", "ZZZ", "MSH", "AAA", "NK1"];
        ids.sort_by(|a, b| segment_rank(a).cmp(&segment_rank(b)));
        assert_eq!(ids, ["MSH", "PID", "NK1", "AAA", "ZZZ"]);
    }

    fn field(name: &str, required: bool, repeating: bool) -> FieldDefinition {
        FieldDefinition {
            name: name.to_string(),
            data_type: "ST".to_string(),
            required,
            repeating,
            max_length: None,
            table_id: None,
            components: None,
        }
    }

    #[test]
    fn long_field_declarations_wrap_like_rustfmt() {
        let mut body = String::new();
        render_field_decl(&mut body, "short", "String");
        assert_eq!(body, "    pub short: String,\n");

        let mut body = String::new();
        let long = format!("overly_long_field_name_{}", "x".repeat(80));
        render_field_decl(&mut body, &long, "Option<String>");
        assert_eq!(body, format!("    pub {long}:\n        Option<String>,\n"));
    }

    #[test]
    fn segment_maps_required_optional_and_repeating() {
        let def = SegmentDefinition {
            name: "Patient Identification".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([
                (3, field("Patient Identifier List", false, true)),
                (5, field("Patient Name", true, false)),
                (8, field("Sex", false, false)),
            ]),
        };

        let source = generate_segment("PID", &def);
        assert!(source.contains("pub struct Pid"), "{source}");
        assert!(source.contains("#[hl7(id = \"PID\")]"), "{source}");
        assert!(
            source.contains("pub patient_identifier_list: Vec<String>,"),
            "{source}"
        );
        assert!(source.contains("pub patient_name: String,"), "{source}");
        assert!(source.contains("pub sex: Option<String>,"), "{source}");
        // Fields render in field-number order.
        let id_pos = source.find("patient_identifier_list").unwrap();
        let name_pos = source.find("patient_name").unwrap();
        let sex_pos = source.find("pub sex").unwrap();
        assert!(id_pos < name_pos && name_pos < sex_pos);
    }

    #[test]
    fn required_repeating_field_carries_required_marker() {
        let def = SegmentDefinition {
            name: "Patient Identification".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([
                (3, field("Patient Identifier List", true, true)),
                (5, field("Patient Name", false, true)),
            ]),
        };

        let source = generate_segment("PID", &def);
        assert!(source.contains("#[hl7(field = 3, required)]"), "{source}");
        assert!(
            source.contains("pub patient_identifier_list: Vec<String>,"),
            "{source}"
        );
        // Optional repeating fields keep the plain marker.
        assert!(source.contains("#[hl7(field = 5)]"), "{source}");
    }

    #[test]
    fn required_repeating_segment_carries_required_marker() {
        let pid = SegmentDefinition {
            name: "Patient Identification".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::new(),
        };
        let obx = SegmentDefinition {
            name: "Observation".to_string(),
            required: true,
            repeating: true,
            fields: HashMap::new(),
        };
        let rol = SegmentDefinition {
            name: "Role".to_string(),
            required: false,
            repeating: true,
            fields: HashMap::new(),
        };
        let schema = MessageSchema {
            message_type: "ORU".to_string(),
            trigger_event: "R01".to_string(),
            version: "2.5".to_string(),
            segments: HashMap::from([
                ("PID".to_string(), pid),
                ("OBX".to_string(), obx),
                ("ROL".to_string(), rol),
            ]),
        };

        let source = generate_module(&schema);
        assert!(
            source.contains("#[hl7(segment, repeating, required)]"),
            "{source}"
        );
        assert!(source.contains("pub obx: Vec<Obx>,"), "{source}");
        // Optional repeating segments keep the plain marker.
        assert!(source.contains("#[hl7(segment, repeating)]"), "{source}");
        assert!(source.contains("pub rol: Vec<Rol>,"), "{source}");
    }

    #[test]
    fn segment_expands_components_for_composite_fields() {
        let mut composite = field("Sending Application", false, false);
        composite.data_type = "HD".to_string();
        composite.components = Some(HashMap::from([
            (
                "1".to_string(),
                ComponentDefinition {
                    name: "Namespace ID".to_string(),
                    data_type: "IS".to_string(),
                    required: false,
                    max_length: None,
                    table_id: None,
                },
            ),
            (
                "2".to_string(),
                ComponentDefinition {
                    name: "Universal ID".to_string(),
                    data_type: "ST".to_string(),
                    required: true,
                    max_length: None,
                    table_id: None,
                },
            ),
        ]));
        let def = SegmentDefinition {
            name: "Message Header".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([(3, composite)]),
        };

        let source = generate_segment("MSH", &def);
        assert!(
            source.contains("pub sending_application_namespace_id: Option<String>,"),
            "{source}"
        );
        // The field itself is optional, so even the required component
        // maps to Option: an absent field carries no components.
        assert!(
            source.contains("pub sending_application_universal_id: Option<String>,"),
            "{source}"
        );
        assert!(
            source.contains("#[hl7(field = 3, component = 1)]"),
            "{source}"
        );
        assert!(
            source.contains("#[hl7(field = 3, component = 2)]"),
            "{source}"
        );

        // A required component of a required field stays required.
        let mut required_composite = field("Sending Application", true, false);
        required_composite.data_type = "HD".to_string();
        required_composite.components = Some(HashMap::from([(
            "1".to_string(),
            ComponentDefinition {
                name: "Namespace ID".to_string(),
                data_type: "IS".to_string(),
                required: true,
                max_length: None,
                table_id: None,
            },
        )]));
        let required_def = SegmentDefinition {
            name: "Message Header".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([(3, required_composite)]),
        };
        let required_source = generate_segment("MSH", &required_def);
        assert!(
            required_source.contains("pub sending_application_namespace_id: String,"),
            "{required_source}"
        );
    }

    #[test]
    fn segment_sanitizes_newlines_in_doc_comments() {
        let mut composite = field("Field\n// injected", false, false);
        composite.data_type = "HD\n// injected".to_string();
        composite.components = Some(HashMap::from([(
            "1".to_string(),
            ComponentDefinition {
                name: "Comp\n// injected".to_string(),
                data_type: "ST".to_string(),
                required: false,
                max_length: None,
                table_id: None,
            },
        )]));
        let def = SegmentDefinition {
            name: "Seg\npub struct Evil;".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([(3, composite)]),
        };

        let source = generate_segment("PID", &def);
        for line in source.lines() {
            let trimmed = line.trim_start();
            assert_ne!(trimmed, "pub struct Evil;", "{source}");
            assert!(!trimmed.starts_with("// injected"), "{source}");
        }
    }

    #[test]
    fn module_sanitizes_newlines_in_comment_header() {
        let def = SegmentDefinition {
            name: "Seg\n// injected".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::new(),
        };
        let schema = MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01".to_string(),
            version: "2.5\n// injected".to_string(),
            segments: HashMap::from([("PID".to_string(), def)]),
        };

        let source = generate_module(&schema);
        for line in source.lines() {
            assert!(!line.trim_start().starts_with("// injected"), "{source}");
        }
    }

    #[test]
    fn segment_keeps_repeating_composite_fields_whole() {
        let mut repeating = field("Patient Name", true, true);
        repeating.data_type = "XPN".to_string();
        repeating.components = Some(HashMap::from([(
            "1".to_string(),
            ComponentDefinition {
                name: "Family Name".to_string(),
                data_type: "FN".to_string(),
                required: false,
                max_length: None,
                table_id: None,
            },
        )]));
        let def = SegmentDefinition {
            name: "Patient Identification".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([(5, repeating)]),
        };

        // Vec cannot combine with component=, so the field stays whole.
        let source = generate_segment("PID", &def);
        assert!(
            source.contains("pub patient_name: Vec<String>,"),
            "{source}"
        );
        assert!(!source.contains("component ="), "{source}");
    }

    #[test]
    fn struct_names_strip_characters_outside_idents() {
        assert_eq!(segment_struct_name("P\"ID"), "Pid");
        assert_eq!(segment_struct_name("123"), "Segment123");
        assert_eq!(message_struct_name("AD\"T", "A01"), "AdtA01");
        assert_eq!(message_struct_name("ADT", "A0\"1"), "AdtA01");
        assert_eq!(message_struct_name("", ""), "UnknownMessage");
    }

    #[test]
    fn sanitize_ident_sanitizes_fallback_text() {
        let mut used = HashSet::new();
        assert_eq!(sanitize_ident("!!!", "seg\"ment", &mut used), "seg_ment");
    }

    #[test]
    fn generated_attributes_escape_schema_text() {
        let def = SegmentDefinition {
            name: "Evil".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::new(),
        };
        let source = generate_segment("PID\")]\npub struct Evil;\n//", &def);
        assert!(
            !source.lines().any(|line| line.trim() == "pub struct Evil;"),
            "{source}"
        );
        let schema = MessageSchema {
            message_type: "ADT".to_string(),
            trigger_event: "A01\")]\npub struct Evil;\n//".to_string(),
            version: "2.5".to_string(),
            segments: HashMap::from([("PID".to_string(), def)]),
        };
        let module = generate_module(&schema);
        assert!(
            !module.lines().any(|line| line.trim() == "pub struct Evil;"),
            "{module}"
        );
        assert!(
            module.contains("pub struct AdtA01pubstructEvil {"),
            "{module}"
        );
    }

    #[test]
    fn msh_delimiter_fields_are_skipped() {
        // MSH-1/2 hold the message delimiters, which encode derives
        // from Delimiters and TerserMut::set rejects: generated Msh
        // must not map them, or from_segment fails on real messages
        // and edits to them are silently dropped.
        let def = SegmentDefinition {
            name: "Message Header".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([
                (1, field("Field Separator", true, false)),
                (2, field("Encoding Characters", true, false)),
                (3, field("Sending Application", false, false)),
            ]),
        };

        let source = generate_segment("MSH", &def);
        assert!(!source.contains("field_separator"), "{source}");
        assert!(!source.contains("encoding_characters"), "{source}");
        assert!(!source.contains("#[hl7(field = 1"), "{source}");
        assert!(!source.contains("#[hl7(field = 2"), "{source}");
        assert!(source.contains("sending_application"), "{source}");
    }

    #[test]
    fn unparseable_component_keys_fall_back_to_whole_field() {
        // Component keys that are all unparseable ('0'/non-numeric)
        // must fall back to a whole-field render, not silently emit
        // no struct field for the HL7 field at all.
        let mut composite = field("Patient Name", false, false);
        composite.components = Some(HashMap::from([
            (
                "0".to_string(),
                ComponentDefinition {
                    name: "Bogus".to_string(),
                    data_type: "ST".to_string(),
                    required: false,
                    max_length: None,
                    table_id: None,
                },
            ),
            (
                "bogus".to_string(),
                ComponentDefinition {
                    name: "Bogus Too".to_string(),
                    data_type: "ST".to_string(),
                    required: false,
                    max_length: None,
                    table_id: None,
                },
            ),
        ]));
        let def = SegmentDefinition {
            name: "Patient Identification".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::from([(5, composite)]),
        };

        let source = generate_segment("PID", &def);
        assert!(source.contains("pub patient_name"), "{source}");
        assert!(source.contains("#[hl7(field = 5)]"), "{source}");
    }

    #[test]
    fn clinical_order_segments_sort_header_before_children() {
        // ORC before OBR before OBX so to_message emits headers first
        // (core ORU builder appends OBR then OBX, ORM builder ORC then
        // OBR); otherwise GroupNavigator::order_observations mis-groups.
        let mut ids = ["OBX", "ORC", "OBR", "PID"];
        ids.sort_by(|a, b| segment_rank(a).cmp(&segment_rank(b)));
        assert_eq!(ids, ["PID", "ORC", "OBR", "OBX"]);
        // Pharmacy chain stays ordered too: ORC before RXE before RXR
        // before TQ1, matching the pharmacy builders.
        let mut rx = ["TQ1", "RXR", "RXE", "ORC"];
        rx.sort_by(|a, b| segment_rank(a).cmp(&segment_rank(b)));
        assert_eq!(rx, ["ORC", "RXE", "RXR", "TQ1"]);
    }

    #[test]
    fn module_suffixes_colliding_struct_names() {
        // "PID" and "P-ID" both sanitize to `Pid`: the module must emit
        // `Pid` and `Pid_2`, not two `pub struct Pid` (E0428).
        let pid = SegmentDefinition {
            name: "Patient Identification".to_string(),
            required: true,
            repeating: false,
            fields: HashMap::new(),
        };
        let pdash = SegmentDefinition {
            name: "P-Dash".to_string(),
            required: false,
            repeating: false,
            fields: HashMap::new(),
        };
        let schema = MessageSchema {
            message_type: "T".to_string(),
            trigger_event: "E".to_string(),
            version: "2.5".to_string(),
            segments: HashMap::from([("PID".to_string(), pid), ("P-ID".to_string(), pdash)]),
        };
        let source = generate_module(&schema);
        assert_eq!(source.matches("pub struct Pid {").count(), 1, "{source}");
        assert!(source.contains("pub struct Pid_2 {"), "{source}");
        assert!(source.contains("Option<Pid_2>"), "{source}");
    }
}
