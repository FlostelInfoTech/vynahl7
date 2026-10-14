//! Message Builder Tab - Build HL7 messages visually

use egui::{self, RichText, Color32};
use egui_extras::{StripBuilder, Size};
use vyna_hl7_core::builders::adt::AdtA01Builder;
use vyna_hl7_core::{CharacterSet, Field, Message, Version};
use vyna_hl7_parser::parse_message;
use vyna_hl7_templates::{MessageTemplate, TemplateEngine, TemplateLibrary};
use vyna_hl7_terser::TerserMut;
use std::collections::{HashMap, HashSet};

pub struct BuilderTab {
    // Message type selection
    message_type: MessageType,

    // ADT A01 fields
    patient_id: String,
    patient_id_system: String,
    family_name: String,
    given_name: String,
    middle_name: String,
    birth_date: String,
    gender: Gender,
    street_address: String,
    city: String,
    state: String,
    zip_code: String,
    country: String,
    phone_home: String,
    phone_work: String,

    // Visit info
    patient_class: PatientClass,
    attending_doctor_id: String,
    attending_doctor_name: String,
    room: String,
    bed: String,

    // Message header
    sending_app: String,
    sending_facility: String,
    receiving_app: String,
    receiving_facility: String,

    // Output
    built_message: Option<String>,
    build_error: Option<String>,

    // Display options
    show_line_feeds: bool,

    // Template mode
    build_mode: BuildMode,
    template_library: TemplateLibrary,
    selected_template: String,
    template_values: HashMap<String, HashMap<String, String>>,
    template_notice: Option<String>,
    /// Names loaded from files (reload overwrites; built-ins are protected).
    loaded_templates: HashSet<String>,

    // Message options (both modes); None leaves the build default in place
    hl7_version: Option<Version>,
    charset: Option<CharacterSet>,
}

/// HL7 v2.x versions offered by the version picker, oldest first.
const HL7_VERSIONS: [Version; 13] = [
    Version::V2_1,
    Version::V2_2,
    Version::V2_3,
    Version::V2_3_1,
    Version::V2_4,
    Version::V2_5,
    Version::V2_5_1,
    Version::V2_6,
    Version::V2_7,
    Version::V2_7_1,
    Version::V2_8,
    Version::V2_8_1,
    Version::V2_8_2,
];

/// Character sets offered by the charset picker: ASCII, UTF-8, and the
/// ISO-8859 parts (8859/12 has no registered encoding and is skipped).
fn charset_options() -> Vec<Option<CharacterSet>> {
    let mut options = vec![None];
    options.push(Some(CharacterSet::Ascii));
    options.push(Some(CharacterSet::Utf8));
    for part in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16] {
        options.push(Some(CharacterSet::Iso8859(part)));
    }
    options
}

fn charset_label(charset: Option<CharacterSet>) -> String {
    match charset {
        None => "Default (none)".to_string(),
        Some(set) => set.msh18(),
    }
}

/// Encode display text (`\n` separators) into wire bytes for saving.
///
/// Separators become `\r`, then the text is encoded in the selected
/// character set; `None` writes UTF-8 bytes as-is.
fn encode_for_save(text: &str, charset: Option<CharacterSet>) -> Result<Vec<u8>, String> {
    let wire = text.replace('\n', "\r");
    match charset {
        Some(set) => set
            .encode(&wire)
            .map_err(|e| format!("Cannot encode message in {}: {}", set.msh18(), e)),
        None => Ok(wire.into_bytes()),
    }
}

/// Form mode builds ADT messages from fixed fields; template mode builds
/// any library template from its `{{variables}}`.
#[derive(Default, PartialEq, Clone, Copy)]
enum BuildMode {
    #[default]
    Form,
    Template,
}

/// One `{{variable}}` found in a template, with its required flag.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TemplateVariable {
    name: String,
    required: bool,
    /// Message locations using the variable, for loading values back.
    locations: Vec<VariableLocation>,
}

/// A segment field (or one of its components) holding a variable.
#[derive(Debug, Clone, PartialEq, Eq)]
struct VariableLocation {
    segment: String,
    /// HL7 1-based field position.
    field: usize,
    /// HL7 1-based component position, or `None` for the whole field.
    component: Option<usize>,
}

/// Collect every `{{variable}}` used by a template, in first-seen order.
///
/// Placeholders and defaults of fields and components are scanned; a
/// variable used by any required field or component is marked required.
fn collect_template_variables(template: &MessageTemplate) -> Vec<TemplateVariable> {
    let mut variables: Vec<TemplateVariable> = Vec::new();
    let mut push = |text: &str, required: bool, location: &VariableLocation| {
        for name in extract_placeholders(text) {
            match variables.iter_mut().find(|v| v.name == name) {
                Some(existing) => {
                    existing.required |= required;
                    if !existing.locations.contains(location) {
                        existing.locations.push(location.clone());
                    }
                }
                None => variables.push(TemplateVariable {
                    name,
                    required,
                    locations: vec![location.clone()],
                }),
            }
        }
    };

    for segment in &template.segments {
        if let Some(fields) = &segment.fields {
            let mut positions: Vec<_> = fields.keys().collect();
            positions.sort();
            for position in positions {
                let field = &fields[position];
                let field_location = VariableLocation {
                    segment: segment.id.clone(),
                    field: *position,
                    component: None,
                };
                if let Some(placeholder) = &field.placeholder {
                    push(placeholder, field.required, &field_location);
                }
                if let Some(default) = &field.default {
                    push(default, field.required, &field_location);
                }
                if let Some(components) = &field.components {
                    for component in components.iter() {
                        let location = VariableLocation {
                            segment: segment.id.clone(),
                            field: *position,
                            component: Some(component.position),
                        };
                        if let Some(placeholder) = &component.placeholder {
                            push(placeholder, component.required, &location);
                        }
                        if let Some(default) = &component.default {
                            push(default, component.required, &location);
                        }
                    }
                }
            }
        }
    }
    if let Some(defaults) = &template.variables {
        let mut names: Vec<_> = defaults.keys().collect();
        names.sort();
        for name in names {
            if !variables.iter().any(|v| &v.name == name) {
                variables.push(TemplateVariable {
                    name: name.clone(),
                    required: false,
                    locations: Vec::new(),
                });
            }
        }
    }
    variables
}

/// Read variable values back out of a parsed message.
///
/// Each variable takes the first non-empty value found at its recorded
/// locations. Whole-field locations read the full encoded field (so
/// `DOE^JOHN` round-trips intact); component locations read the first
/// repetition's component value.
fn backfill_values(
    message: &Message,
    variables: &[TemplateVariable],
) -> HashMap<String, String> {
    let mut values = HashMap::new();
    for variable in variables {
        for location in &variable.locations {
            let value = message
                .get_segment_by_id(&location.segment)
                .and_then(|segment| segment.get_field(location.field))
                .and_then(|field| match location.component {
                    None => {
                        let encoded = field.encode(&message.delimiters);
                        (!encoded.is_empty()).then_some(encoded)
                    }
                    Some(index) => field
                        .repetitions
                        .first()
                        .and_then(|repetition| {
                            index
                                .checked_sub(1)
                                .and_then(|i| repetition.components.get(i))
                        })
                        .and_then(|component| component.value())
                        .filter(|text| !text.is_empty())
                        .map(str::to_string),
                });
            if let Some(text) = value {
                values.insert(variable.name.clone(), text);
                break;
            }
        }
    }
    values
}

/// Bake current values into a template copy as variable defaults.
///
/// Only non-empty values are stored; empty fields stay unset placeholders.
fn bake_template_values(
    template: &MessageTemplate,
    name: String,
    values: &HashMap<String, String>,
) -> MessageTemplate {
    let mut baked = template.clone();
    baked.name = name;
    let defaults = baked.variables.get_or_insert_with(HashMap::new);
    for (key, value) in values {
        if !value.is_empty() {
            defaults.insert(key.clone(), value.clone());
        }
    }
    baked
}

/// Parse a template file, choosing JSON vs YAML by extension.
fn parse_template_file(path: &std::path::Path) -> Result<MessageTemplate, String> {
    let is_json = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
    if is_json {
        MessageTemplate::from_json_file(path)
    } else {
        MessageTemplate::from_yaml_file(path)
    }
    .map_err(|e| format!("Cannot parse template: {e}"))
}

/// Extract `{{name}}` placeholders from a template string.
///
/// Only bindable names are listed: unclosed (`{{name}`), empty
/// (`{{}}`, `{{ }}`), and nested-outer (`{{a {{b}}`) openers yield no
/// variable here — the template engine reports them as errors when
/// the message is built.
fn extract_placeholders(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("}}") else {
            break;
        };
        let candidate = &rest[..end];
        if let Some(nested) = candidate.find("{{") {
            // Nested opener: the outer "{{" never closes, so rescan
            // from the inner one instead of swallowing it.
            rest = &rest[nested..];
            continue;
        }
        let name = candidate.trim();
        if !name.is_empty() && !names.iter().any(|n| n == name) {
            names.push(name.to_string());
        }
        rest = &rest[end + 2..];
    }
    names
}

#[derive(Default, PartialEq, Clone, Copy)]
enum MessageType {
    #[default]
    AdtA01,
    AdtA04,
    AdtA08,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum Gender {
    #[default]
    Male,
    Female,
    Other,
    Unknown,
}

impl Gender {
    fn code(&self) -> &str {
        match self {
            Gender::Male => "M",
            Gender::Female => "F",
            Gender::Other => "O",
            Gender::Unknown => "U",
        }
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
enum PatientClass {
    #[default]
    Inpatient,
    Outpatient,
    Emergency,
    Observation,
}

impl PatientClass {
    fn code(&self) -> &str {
        match self {
            PatientClass::Inpatient => "I",
            PatientClass::Outpatient => "O",
            PatientClass::Emergency => "E",
            PatientClass::Observation => "B",
        }
    }
}

impl Default for BuilderTab {
    fn default() -> Self {
        Self {
            message_type: MessageType::AdtA01,
            patient_id: "12345678".to_string(),
            patient_id_system: "HOSPITAL".to_string(),
            family_name: "DOE".to_string(),
            given_name: "JOHN".to_string(),
            middle_name: "A".to_string(),
            birth_date: "1980-01-15".to_string(),
            gender: Gender::Male,
            street_address: "123 MAIN STREET".to_string(),
            city: "SPRINGFIELD".to_string(),
            state: "IL".to_string(),
            zip_code: "62701".to_string(),
            country: "USA".to_string(),
            phone_home: "555-123-4567".to_string(),
            phone_work: "555-987-6543".to_string(),
            patient_class: PatientClass::Inpatient,
            attending_doctor_id: "1234567890".to_string(),
            attending_doctor_name: "SMITH, JOHN D MD".to_string(),
            room: "ICU".to_string(),
            bed: "101-A".to_string(),
            sending_app: "TEST_APP".to_string(),
            sending_facility: "TEST_FACILITY".to_string(),
            receiving_app: "RECEIVING_APP".to_string(),
            receiving_facility: "RECEIVING_FAC".to_string(),
            built_message: None,
            build_error: None,
            show_line_feeds: true,  // Default to showing line feeds for readability
            build_mode: BuildMode::Form,
            template_library: TemplateLibrary::new(),
            selected_template: "ADT_A01".to_string(),
            template_values: HashMap::new(),
            template_notice: None,
            loaded_templates: HashSet::new(),
            hl7_version: None,
            charset: None,
        }
    }
}

impl BuilderTab {
    /// Get the built message content (used by File > Save)
    /// Returns the built message if available, otherwise an empty string
    pub fn get_message(&self) -> &str {
        self.built_message.as_deref().unwrap_or("")
    }

    /// Check if the builder has a message that can be saved
    pub fn has_message(&self) -> bool {
        self.built_message.is_some()
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Message Builder");
        ui.label("Build HL7 messages using the fluent builder API.");
        ui.add_space(10.0);

        // Get available size for full-height panels
        let available_height = ui.available_height();

        // Use StripBuilder for responsive proportional columns (55% form, 45% output)
        StripBuilder::new(ui)
            .size(Size::relative(0.55).at_least(400.0))
            .size(Size::remainder().at_least(300.0))
            .horizontal(|mut strip| {
                // Left column: Form
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());
                            egui::ScrollArea::vertical()
                                .id_salt("builder_form")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_min_width(ui.available_width());
                        // Build mode
                        ui.heading("Build Mode");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.build_mode, BuildMode::Form, "Form (ADT)");
                            ui.selectable_value(&mut self.build_mode, BuildMode::Template, "Template");
                        });

                        ui.add_space(10.0);
                        ui.separator();

                        // Message options shared by both modes
                        ui.heading("Message Options");
                        ui.horizontal(|ui| {
                            ui.label("HL7 version (MSH-12):");
                            let selected =
                                self.hl7_version.map(|v| v.as_str()).unwrap_or("Default");
                            egui::ComboBox::from_id_salt("version_picker")
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.hl7_version, None, "Default");
                                    for version in HL7_VERSIONS {
                                        ui.selectable_value(
                                            &mut self.hl7_version,
                                            Some(version),
                                            version.as_str(),
                                        );
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Character set (MSH-18):");
                            egui::ComboBox::from_id_salt("charset_picker")
                                .selected_text(charset_label(self.charset))
                                .show_ui(ui, |ui| {
                                    for option in charset_options() {
                                        ui.selectable_value(
                                            &mut self.charset,
                                            option,
                                            charset_label(option),
                                        );
                                    }
                                });
                        });
                        ui.label(
                            RichText::new("Options override the build defaults; file save encodes in the selected character set.")
                                .weak()
                                .small(),
                        );

                        ui.add_space(10.0);
                        ui.separator();

                        if self.build_mode == BuildMode::Template {
                            self.template_form(ui);
                        } else {
                            // Message Type
                            ui.heading("Message Type");
                            ui.horizontal(|ui| {
                                ui.selectable_value(&mut self.message_type, MessageType::AdtA01, "ADT^A01 (Admit)");
                                ui.selectable_value(&mut self.message_type, MessageType::AdtA04, "ADT^A04 (Register)");
                                ui.selectable_value(&mut self.message_type, MessageType::AdtA08, "ADT^A08 (Update)");
                            });
    
                            ui.add_space(10.0);
                            ui.separator();
    
                            // Message Header
                            ui.heading("Message Header");
                            ui.horizontal(|ui| {
                                ui.label("Sending App:");
                                ui.text_edit_singleline(&mut self.sending_app);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Sending Facility:");
                                ui.text_edit_singleline(&mut self.sending_facility);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Receiving App:");
                                ui.text_edit_singleline(&mut self.receiving_app);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Receiving Facility:");
                                ui.text_edit_singleline(&mut self.receiving_facility);
                            });
    
                            ui.add_space(10.0);
                            ui.separator();
    
                            // Patient Information
                            ui.heading("Patient Information");
    
                            ui.horizontal(|ui| {
                                ui.label("Patient ID:");
                                ui.text_edit_singleline(&mut self.patient_id);
                                ui.label("System:");
                                ui.text_edit_singleline(&mut self.patient_id_system);
                            });
    
                            ui.horizontal(|ui| {
                                ui.label("Family Name:");
                                ui.text_edit_singleline(&mut self.family_name);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Given Name:");
                                ui.text_edit_singleline(&mut self.given_name);
                                ui.label("Middle:");
                                ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut self.middle_name));
                            });
    
                            ui.horizontal(|ui| {
                                ui.label("Birth Date (YYYY-MM-DD):");
                                ui.text_edit_singleline(&mut self.birth_date);
                            });
    
                            ui.horizontal(|ui| {
                                ui.label("Gender:");
                                ui.selectable_value(&mut self.gender, Gender::Male, "Male");
                                ui.selectable_value(&mut self.gender, Gender::Female, "Female");
                                ui.selectable_value(&mut self.gender, Gender::Other, "Other");
                                ui.selectable_value(&mut self.gender, Gender::Unknown, "Unknown");
                            });
    
                            ui.add_space(10.0);
                            ui.separator();
    
                            // Address
                            ui.heading("Address");
                            ui.horizontal(|ui| {
                                ui.label("Street:");
                                ui.add_sized([300.0, 20.0], egui::TextEdit::singleline(&mut self.street_address));
                            });
                            ui.horizontal(|ui| {
                                ui.label("City:");
                                ui.text_edit_singleline(&mut self.city);
                                ui.label("State:");
                                ui.add_sized([40.0, 20.0], egui::TextEdit::singleline(&mut self.state));
                            });
                            ui.horizontal(|ui| {
                                ui.label("ZIP Code:");
                                ui.add_sized([80.0, 20.0], egui::TextEdit::singleline(&mut self.zip_code));
                                ui.label("Country:");
                                ui.add_sized([60.0, 20.0], egui::TextEdit::singleline(&mut self.country));
                            });
    
                            ui.add_space(10.0);
                            ui.separator();
    
                            // Contact
                            ui.heading("Contact Information");
                            ui.horizontal(|ui| {
                                ui.label("Home Phone:");
                                ui.text_edit_singleline(&mut self.phone_home);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Work Phone:");
                                ui.text_edit_singleline(&mut self.phone_work);
                            });
    
                            ui.add_space(10.0);
                            ui.separator();
    
                            // Visit Information
                            ui.heading("Visit Information");
                            ui.horizontal(|ui| {
                                ui.label("Patient Class:");
                                ui.selectable_value(&mut self.patient_class, PatientClass::Inpatient, "Inpatient");
                                ui.selectable_value(&mut self.patient_class, PatientClass::Outpatient, "Outpatient");
                                ui.selectable_value(&mut self.patient_class, PatientClass::Emergency, "Emergency");
                                ui.selectable_value(&mut self.patient_class, PatientClass::Observation, "Observation");
                            });
    
                            ui.horizontal(|ui| {
                                ui.label("Room:");
                                ui.add_sized([80.0, 20.0], egui::TextEdit::singleline(&mut self.room));
                                ui.label("Bed:");
                                ui.add_sized([80.0, 20.0], egui::TextEdit::singleline(&mut self.bed));
                            });
    
                            ui.horizontal(|ui| {
                                ui.label("Attending Dr ID:");
                                ui.text_edit_singleline(&mut self.attending_doctor_id);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Attending Dr Name:");
                                ui.text_edit_singleline(&mut self.attending_doctor_name);
                            });
    
                            ui.add_space(20.0);
    
                            // Build button
                            ui.horizontal(|ui| {
                                if ui.button(RichText::new("Build Message").strong()).clicked() {
                                    self.build_message();
                                }
                                if ui.button("Reset to Defaults").clicked() {
                                    *self = Self::default();
                                }
                            });
                        }
                                });
                        });
                });

                // Right column: Output
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());

                            ui.heading("Generated Message");

                            if let Some(ref error) = self.build_error {
                                ui.colored_label(Color32::RED, format!("Error: {}", error));
                            } else if let Some(message) = self.built_message.clone() {
                                ui.horizontal(|ui| {
                                    ui.label(format!("Size: {} bytes", message.len()));
                                    ui.separator();
                                    ui.checkbox(&mut self.show_line_feeds, "Show segments on separate lines");
                                });

                                ui.horizontal(|ui| {
                                    if ui.button("Copy to Clipboard").clicked() {
                                        // Copy the original raw message (with \r)
                                        ui.ctx().copy_text(message.clone());
                                    }
                                    if ui.button("Save to File...").clicked() {
                                        self.save_built_message();
                                    }
                                });

                                ui.add_space(10.0);

                                // Format message for display based on option
                                let display_message = if self.show_line_feeds {
                                    message.replace('\r', "\n")
                                } else {
                                    message.clone()
                                };

                                egui::ScrollArea::both()
                                    .id_salt("built_message")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut display_message.as_str())
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .desired_rows(35)
                                                .interactive(false)
                                        );
                                    });
                            } else {
                                ui.label("Fill in the form and click 'Build Message' to generate an HL7 message.");
                                ui.add_space(20.0);
                                ui.label("The builder uses the fluent API from vyna-hl7-core:");
                                ui.add_space(10.0);
                                ui.code("AdtA01Builder::new()\n    .sending_application(\"APP\")\n    .patient_id(\"12345\")\n    .patient_name(name)\n    .build()");
                            }
                        });
                });
            });
    }

    /// Template picker with auto-generated variable fields.
    fn template_form(&mut self, ui: &mut egui::Ui) {
        let mut names = self.template_library.list_templates();
        names.sort();
        if !names.iter().any(|n| n.as_str() == self.selected_template)
            && let Some(first) = names.first()
        {
            self.selected_template = first.to_string();
        }

        ui.heading("Template");
        egui::ComboBox::from_id_salt("template_picker")
            .selected_text(self.selected_template.as_str())
            .show_ui(ui, |ui| {
                for name in &names {
                    ui.selectable_value(&mut self.selected_template, name.to_string(), name.as_str());
                }
            });

        let Some(template) = self.template_library.get(&self.selected_template).cloned() else {
            ui.colored_label(Color32::RED, "Template library is empty.");
            return;
        };
        if let Some(description) = &template.description {
            ui.label(
                RichText::new(format!(
                    "{}^{} {} — {}",
                    template.message_type, template.trigger_event, template.version, description
                ))
                .weak(),
            );
        } else {
            ui.label(
                RichText::new(format!(
                    "{}^{} {}",
                    template.message_type, template.trigger_event, template.version
                ))
                .weak(),
            );
        }

        ui.add_space(10.0);
        ui.separator();
        ui.heading("Variables");
        if let Some(notice) = &self.template_notice {
            ui.label(RichText::new(notice).weak().small());
        }

        self.ensure_template_values(&template);
        let variables = collect_template_variables(&template);
        let values = self
            .template_values
            .entry(self.selected_template.clone())
            .or_default();
        for variable in &variables {
            ui.horizontal(|ui| {
                let label = if variable.required {
                    RichText::new(format!("{} *", variable.name)).strong()
                } else {
                    RichText::new(&variable.name)
                };
                ui.label(label);
                let value = values.entry(variable.name.clone()).or_default();
                ui.add_sized(
                    [ui.available_width(), 20.0],
                    egui::TextEdit::singleline(value),
                );
            });
        }

        ui.add_space(20.0);
        ui.horizontal(|ui| {
            if ui.button(RichText::new("Build Message").strong()).clicked() {
                self.build_message();
            }
            if ui.button("Load Message...").clicked() {
                self.load_message_into_template();
            }
            if ui.button("Clear Variables").clicked() {
                self.template_values.remove(&self.selected_template.clone());
                self.template_notice = None;
                self.built_message = None;
                self.build_error = None;
            }
        });
        ui.horizontal(|ui| {
            if ui.button("Save as Template...").clicked() {
                self.save_as_template();
            }
            if ui.button("Load Template...").clicked() {
                self.load_template_file();
            }
        });
    }

    /// Prefill variable values from template defaults on first use.
    fn ensure_template_values(&mut self, template: &MessageTemplate) {
        let values = self
            .template_values
            .entry(self.selected_template.clone())
            .or_default();
        if let Some(defaults) = &template.variables {
            for (name, value) in defaults {
                values.entry(name.clone()).or_insert_with(|| value.clone());
            }
        }
    }

    /// Save the built message through a file picker.
    ///
    /// The file carries real wire format (`\r` separators) encoded in the
    /// selected character set; unmappable content is reported, never replaced.
    fn save_built_message(&mut self) {
        let Some(text) = self.built_message.clone() else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("HL7 Messages", &["hl7"])
            .add_filter("Text Files", &["txt"])
            .add_filter("All Files", &["*"])
            .set_title("Save Built Message")
            .save_file()
        else {
            return;
        };
        let bytes = match encode_for_save(&text, self.charset) {
            Ok(bytes) => bytes,
            Err(e) => {
                self.build_error = Some(e);
                return;
            }
        };
        if let Err(e) = std::fs::write(&path, bytes) {
            self.build_error = Some(format!("Save failed: {}", e));
        }
    }

    /// Load a message file into the selected template's variables.
    ///
    /// Bytes are decoded per MSH-18, parsed, and each variable is filled
    /// from its recorded template locations. Variables with no value in
    /// the file keep their current content.
    fn load_message_into_template(&mut self) {
        let Some(template) = self
            .template_library
            .get(&self.selected_template)
            .cloned()
        else {
            self.template_notice = Some("No template selected.".to_string());
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("HL7 Messages", &["hl7", "txt"])
            .add_filter("All Files", &["*"])
            .set_title("Load Message into Template")
            .pick_file()
        else {
            return;
        };
        let loaded = (|| -> Result<(usize, usize), String> {
            let bytes =
                std::fs::read(&path).map_err(|e| format!("Cannot read file: {}", e))?;
            let text = CharacterSet::decode_message(&bytes)
                .map_err(|e| format!("Cannot decode file: {}", e))?;
            let normalized = text.replace("\r\n", "\r").replace('\n', "\r");
            let message =
                parse_message(&normalized).map_err(|e| format!("Cannot parse: {}", e))?;
            let variables = collect_template_variables(&template);
            let values = backfill_values(&message, &variables);
            let found = values.len();
            self.template_values
                .entry(self.selected_template.clone())
                .or_default()
                .extend(values);
            Ok((found, variables.len()))
        })();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        self.template_notice = Some(match loaded {
            Ok((found, total)) => format!("Loaded {found} of {total} variables from {name}."),
            Err(e) => format!("{name}: {e}"),
        });
        self.built_message = None;
        self.build_error = None;
    }

    /// Save the selected template with current values as defaults.
    ///
    /// The template name comes from the chosen file name; JSON is used
    /// for `.json` files, YAML otherwise.
    fn save_as_template(&mut self) {
        let Some(template) = self
            .template_library
            .get(&self.selected_template)
            .cloned()
        else {
            self.template_notice = Some("No template selected.".to_string());
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter("YAML Templates", &["yaml", "yml"])
            .add_filter("JSON Templates", &["json"])
            .add_filter("All Files", &["*"])
            .set_title("Save as Template")
            .set_file_name(format!("{}.yaml", self.selected_template))
            .save_file()
        else {
            return;
        };
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .filter(|stem| !stem.is_empty())
            .unwrap_or_else(|| self.selected_template.clone());
        let values = self
            .template_values
            .get(&self.selected_template)
            .cloned()
            .unwrap_or_default();
        let baked = bake_template_values(&template, name.clone(), &values);
        let is_json = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
        let saved = if is_json {
            baked.to_json_file(&path)
        } else {
            baked.to_yaml_file(&path)
        };
        self.template_notice = Some(match saved {
            Ok(()) => format!("Saved template '{name}' to {}.", path.display()),
            Err(e) => format!("Cannot save template: {e}"),
        });
    }

    /// Load a template file into the library and select it.
    fn load_template_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Templates", &["yaml", "yml", "json"])
            .add_filter("All Files", &["*"])
            .set_title("Load Template")
            .pick_file()
        else {
            return;
        };
        let file_label = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let parsed = parse_template_file(&path);
        match parsed {
            Ok(template) => {
                let name = self.register_template(template);
                self.template_notice =
                    Some(format!("Loaded template '{name}' from {file_label}."));
                self.built_message = None;
                self.build_error = None;
            }
            Err(e) => {
                self.template_notice = Some(format!("{file_label}: {e}"));
            }
        }
    }

    /// Register a loaded template, pick a non-clobbering name, select it.
    ///
    /// Reloading a previously loaded template overwrites it; anything
    /// else that collides gets a numeric suffix so built-ins survive.
    fn register_template(&mut self, mut template: MessageTemplate) -> String {
        if template.name.trim().is_empty() {
            template.name = "Custom".to_string();
        }
        let name = self.resolve_template_name(&template.name);
        template.name = name.clone();
        self.template_library.add_template(template);
        self.loaded_templates.insert(name.clone());
        self.selected_template = name.clone();
        name
    }

    /// Resolve a template name against the library without clobbering.
    fn resolve_template_name(&self, base: &str) -> String {
        if self.loaded_templates.contains(base) || self.template_library.get(base).is_none() {
            return base.to_string();
        }
        let mut counter = 2;
        loop {
            let candidate = format!("{base} {counter}");
            if self.template_library.get(&candidate).is_none() {
                return candidate;
            }
            counter += 1;
        }
    }

    fn build_message(&mut self) {
        let result = match self.build_mode {
            BuildMode::Form => self.try_build_message(),
            BuildMode::Template => self.try_build_template_message(),
        };
        match result {
            Ok(msg) => {
                self.built_message = Some(msg);
                self.build_error = None;
            }
            Err(e) => {
                self.build_error = Some(e);
                self.built_message = None;
            }
        }
    }

    fn try_build_message(&self) -> Result<String, String> {
        // Format birth date from YYYY-MM-DD to YYYYMMDD
        let birth_date_hl7 = self.birth_date.replace("-", "");

        // Note: Patient name is constructed directly in the builder call below

        // Build attending doctor field
        let attending_doctor = format!("{}^{}", self.attending_doctor_id, self.attending_doctor_name);

        // Build assigned location
        let assigned_location = format!("{}^{}", self.room, self.bed);

        // One trigger per picker choice. Every arm builds through the
        // A01 builder shape (it alone carries the
        // assigned-location/attending-doctor delegates); A04/A08 are
        // restamped to their own trigger below so MSH-9/EVN-1 match.
        let trigger = match self.message_type {
            MessageType::AdtA01 => "A01",
            MessageType::AdtA04 => "A04",
            MessageType::AdtA08 => "A08",
        };
        let mut message = AdtA01Builder::new(self.hl7_version.unwrap_or(Version::V2_5_1))
            .sending_application(&self.sending_app)
            .sending_facility(&self.sending_facility)
            .receiving_application(&self.receiving_app)
            .receiving_facility(&self.receiving_facility)
            .patient_id(&self.patient_id)
            .patient_name(&self.family_name, &self.given_name)
            .date_of_birth(&birth_date_hl7)
            .sex(self.gender.code())
            .patient_class(self.patient_class.code())
            .assigned_location(&assigned_location)
            .attending_doctor(&attending_doctor)
            .build()
            .map_err(|e| format!("Build error: {}", e))?;

        self.restamp_trigger(&mut message, trigger)?;
        self.stamp_extra_fields(&mut message)?;
        self.apply_message_options(&mut message)?;
        Ok(message.encode_with_separator("\n"))
    }

    /// Restamp MSH-9/EVN-1 to the picker's trigger event.
    ///
    /// The shared A01 builder shape always emits `ADT^A01`/`A01`, so
    /// A04/A08 overwrite the trigger component and event code in place.
    fn restamp_trigger(&self, message: &mut Message, trigger: &str) -> Result<(), String> {
        if trigger == "A01" {
            return Ok(());
        }
        let mut terser = TerserMut::new(message);
        terser
            .set("MSH-9-2", trigger)
            .map_err(|e| format!("Trigger error: {}", e))?;
        terser
            .set("EVN-1", trigger)
            .map_err(|e| format!("Trigger error: {}", e))?;
        Ok(())
    }

    /// Stamp form fields the ADT builder has no delegates for.
    ///
    /// Each value lands only when non-empty, so clearing a form field
    /// leaves the built default instead of writing blank structure.
    fn stamp_extra_fields(&self, message: &mut Message) -> Result<(), String> {
        let mut terser = TerserMut::new(message);
        let mut stamp = |path: &str, value: &str| -> Result<(), String> {
            if value.is_empty() {
                return Ok(());
            }
            terser.set(path, value).map_err(|e| format!("Field error: {}", e))
        };
        stamp("PID-3-4", &self.patient_id_system)?;
        stamp("PID-5-3", &self.middle_name)?;
        stamp("PID-11-1", &self.street_address)?;
        stamp("PID-11-3", &self.city)?;
        stamp("PID-11-4", &self.state)?;
        stamp("PID-11-5", &self.zip_code)?;
        stamp("PID-11-6", &self.country)?;
        stamp("PID-13", &self.phone_home)?;
        stamp("PID-14", &self.phone_work)?;
        Ok(())
    }

    /// Stamp the selected HL7 version (MSH-12) and character set (MSH-18).
    ///
    /// Each option overrides only when explicitly picked; `None` leaves
    /// whatever the form builder or template engine produced.
    fn apply_message_options(&self, message: &mut Message) -> Result<(), String> {
        if self.hl7_version.is_none() && self.charset.is_none() {
            return Ok(());
        }
        let Some(msh) = message.get_segment_by_id_mut("MSH") else {
            return Ok(());
        };
        if let Some(version) = self.hl7_version {
            msh.set_field(12, Field::from_value(version.as_str()))
                .map_err(|e| format!("Version error: {}", e))?;
        }
        if let Some(charset) = self.charset {
            msh.set_field(18, Field::from_value(charset.msh18()))
                .map_err(|e| format!("Charset error: {}", e))?;
        }
        Ok(())
    }

    fn try_build_template_message(&self) -> Result<String, String> {
        let template = self
            .template_library
            .get(&self.selected_template)
            .ok_or_else(|| format!("Template '{}' not found", self.selected_template))?;
        let mut engine = TemplateEngine::new();
        // Every collected variable is set (empty when unfilled) so the
        // engine substitutes structurally instead of missing-variable errors.
        for variable in collect_template_variables(template) {
            let value = self
                .template_values
                .get(&self.selected_template)
                .and_then(|values| values.get(&variable.name))
                .cloned()
                .unwrap_or_default();
            engine.set_variable(variable.name, value);
        }
        let mut message = engine
            .create_message(template)
            .map_err(|e| format!("Template error: {}", e))?;
        self.apply_message_options(&mut message)?;
        Ok(message.encode_with_separator("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_extraction_trims_and_dedupes() {
        assert_eq!(extract_placeholders("{{name}}"), vec!["name".to_string()]);
        assert_eq!(
            extract_placeholders("Patient {{ name }} has ID {{id}} ({{name}})"),
            vec!["name".to_string(), "id".to_string()]
        );
        assert!(extract_placeholders("no placeholders").is_empty());
        assert!(extract_placeholders("{{unclosed").is_empty());
        assert!(extract_placeholders("{{}}").is_empty());
    }

    #[test]
    fn placeholder_extraction_skips_unclosed_empty_and_nested() {
        assert!(extract_placeholders("{{ }}").is_empty());
        assert!(extract_placeholders("Patient {{name}").is_empty());
        // Nested: the outer opener never closes, the inner still lists.
        assert_eq!(extract_placeholders("{{a {{b}}"), vec!["b".to_string()]);
        // A later unclosed opener does not hide earlier valid ones.
        assert_eq!(
            extract_placeholders("{{ok}} then {{oops"),
            vec!["ok".to_string()]
        );
    }

    #[test]
    fn form_build_restamps_trigger_for_a04_and_a08() {
        for (message_type, trigger) in [
            (MessageType::AdtA01, "A01"),
            (MessageType::AdtA04, "A04"),
            (MessageType::AdtA08, "A08"),
        ] {
            let tab = BuilderTab {
                message_type,
                ..BuilderTab::default()
            };
            let text = tab.try_build_message().unwrap();
            let msh_fields: Vec<&str> = text.lines().next().unwrap().split('|').collect();
            assert_eq!(msh_fields[8], format!("ADT^{trigger}"));
            let evn = text.lines().nth(1).unwrap();
            assert!(evn.starts_with(&format!("EVN|{trigger}|")), "unexpected EVN: {evn}");
            // Location/doctor delegates still apply to every trigger.
            let pv1_fields: Vec<&str> = text
                .lines()
                .find(|l| l.starts_with("PV1|"))
                .unwrap()
                .split('|')
                .collect();
            assert_eq!(pv1_fields[3], "ICU^101-A");
            assert_eq!(pv1_fields[7], "1234567890^SMITH, JOHN D MD");
        }
    }

    #[test]
    fn form_build_stamps_dropped_pid_fields() {
        let tab = BuilderTab::default();
        let text = tab.try_build_message().unwrap();
        let fields: Vec<&str> = text
            .lines()
            .find(|l| l.starts_with("PID|"))
            .unwrap()
            .split('|')
            .collect();
        assert_eq!(fields[3], "12345678^^^HOSPITAL");
        assert_eq!(fields[5], "DOE^JOHN^A");
        assert_eq!(fields[11], "123 MAIN STREET^^SPRINGFIELD^IL^62701^USA");
        assert_eq!(fields[13], "555-123-4567");
        assert_eq!(fields[14], "555-987-6543");
    }

    #[test]
    fn form_build_skips_empty_extra_fields() {
        let tab = BuilderTab {
            patient_id_system: String::new(),
            middle_name: String::new(),
            street_address: String::new(),
            city: String::new(),
            state: String::new(),
            zip_code: String::new(),
            country: String::new(),
            phone_home: String::new(),
            phone_work: String::new(),
            ..BuilderTab::default()
        };
        let text = tab.try_build_message().unwrap();
        let fields: Vec<&str> = text
            .lines()
            .find(|l| l.starts_with("PID|"))
            .unwrap()
            .split('|')
            .collect();
        assert_eq!(fields[3], "12345678");
        assert_eq!(fields[5], "DOE^JOHN");
        assert!(fields.get(11).is_none_or(|f| f.is_empty()));
        assert!(fields.get(13).is_none_or(|f| f.is_empty()));
        assert!(fields.get(14).is_none_or(|f| f.is_empty()));
    }

    #[test]
    fn adt_a01_variables_collected_in_segment_order() {
        let library = TemplateLibrary::new();
        let template = library.get("ADT_A01").unwrap();
        let names: Vec<String> = collect_template_variables(template)
            .iter()
            .map(|v| v.name.clone())
            .collect();
        assert_eq!(
            names,
            vec![
                "sending_app",
                "sending_facility",
                "receiving_app",
                "receiving_facility",
                "event_datetime",
                "patient_id",
                "patient_name",
                "date_of_birth",
                "sex",
                "address",
                "patient_class",
                "assigned_location",
                "attending_doctor",
                "visit_number",
            ]
        );
    }

    #[test]
    fn required_flag_comes_from_required_fields() {
        let library = TemplateLibrary::new();
        let template = library.get("ADT_A01").unwrap();
        let variables = collect_template_variables(template);
        let required: Vec<&str> = variables
            .iter()
            .filter(|v| v.required)
            .map(|v| v.name.as_str())
            .collect();
        // MSH-3/4, EVN-2, PID-3/5, PV1-2 are required in the template.
        assert!(required.contains(&"sending_app"));
        assert!(required.contains(&"patient_id"));
        assert!(required.contains(&"patient_class"));
        assert!(!required.contains(&"sex"));
        assert!(!required.contains(&"address"));
    }

    #[test]
    fn every_library_template_builds_with_empty_variables() {
        let tab = BuilderTab::default();
        let mut names = tab.template_library.list_templates();
        names.sort();
        assert!(!names.is_empty());
        for name in names {
            let tab = BuilderTab {
                build_mode: BuildMode::Template,
                selected_template: name.clone(),
                ..BuilderTab::default()
            };
            let text = tab
                .try_build_template_message()
                .unwrap_or_else(|e| panic!("template {name} must build: {e}"));
            let template = tab.template_library.get(name).unwrap();
            let msh = text.lines().next().unwrap();
            assert!(
                msh.starts_with("MSH|^~\\&|"),
                "template {name} MSH malformed: {msh}"
            );
            assert!(
                msh.contains(&format!("{}^{}", template.message_type, template.trigger_event)),
                "template {name} MSH-9 missing: {msh}"
            );
        }
    }

    #[test]
    fn template_build_applies_variable_values() {
        let mut tab = BuilderTab {
            build_mode: BuildMode::Template,
            ..BuilderTab::default()
        };
        tab.template_values.insert(
            "ADT_A01".to_string(),
            [("patient_id".to_string(), "TEST123".to_string())]
                .into_iter()
                .collect(),
        );
        let text = tab.try_build_template_message().unwrap();
        let pid = text.lines().find(|l| l.starts_with("PID|")).unwrap();
        let fields: Vec<&str> = pid.split('|').collect();
        assert_eq!(fields[3], "TEST123");
    }

    #[test]
    fn version_and_charset_options_stamp_msh() {
        let tab = BuilderTab {
            build_mode: BuildMode::Template,
            hl7_version: Some(Version::V2_8),
            charset: Some(CharacterSet::Utf8),
            ..BuilderTab::default()
        };
        let text = tab.try_build_template_message().unwrap();
        let msh = text.lines().next().unwrap();
        let fields: Vec<&str> = msh.split('|').collect();
        assert_eq!(fields[11], "2.8");
        assert_eq!(fields[17], "UNICODE UTF-8");
    }

    #[test]
    fn unset_options_leave_build_defaults() {
        let tab = BuilderTab {
            build_mode: BuildMode::Template,
            ..BuilderTab::default()
        };
        assert!(tab.hl7_version.is_none());
        assert!(tab.charset.is_none());
        let text = tab.try_build_template_message().unwrap();
        let msh = text.lines().next().unwrap();
        let fields: Vec<&str> = msh.split('|').collect();
        assert_eq!(fields[11], "2.5");
        assert!(fields.get(17).is_none_or(|f| f.is_empty()));
    }

    #[test]
    fn form_build_honors_selected_version() {
        let tab = BuilderTab {
            hl7_version: Some(Version::V2_3_1),
            ..BuilderTab::default()
        };
        let text = tab.try_build_message().unwrap();
        let msh = text.lines().next().unwrap();
        let fields: Vec<&str> = msh.split('|').collect();
        assert_eq!(fields[11], "2.3.1");
    }

    #[test]
    fn encode_for_save_uses_wire_separators_and_charset() {
        let bytes = encode_for_save("MSH|a\nPID|b", None).unwrap();
        assert_eq!(bytes, b"MSH|a\rPID|b");

        let bytes = encode_for_save("PID|Müller", Some(CharacterSet::Iso8859(1))).unwrap();
        assert_eq!(bytes, b"PID|M\xfcller");

        let error = encode_for_save("PID|王小明", Some(CharacterSet::Ascii)).unwrap_err();
        assert!(error.contains("ASCII"), "unexpected error: {error}");
    }

    #[test]
    fn variables_record_message_locations() {
        let library = TemplateLibrary::new();
        let template = library.get("ADT_A01").unwrap();
        let variables = collect_template_variables(template);
        let patient_id = variables.iter().find(|v| v.name == "patient_id").unwrap();
        assert_eq!(
            patient_id.locations,
            vec![VariableLocation {
                segment: "PID".to_string(),
                field: 3,
                component: None,
            }]
        );
        let sending_app = variables.iter().find(|v| v.name == "sending_app").unwrap();
        assert_eq!(
            sending_app.locations,
            vec![VariableLocation {
                segment: "MSH".to_string(),
                field: 3,
                component: None,
            }]
        );
    }

    #[test]
    fn backfill_round_trips_template_values() {
        let mut tab = BuilderTab {
            build_mode: BuildMode::Template,
            ..BuilderTab::default()
        };
        let template = tab.template_library.get("ADT_A01").unwrap().clone();
        let variables = collect_template_variables(&template);
        let expected: HashMap<String, String> = variables
            .iter()
            .map(|v| (v.name.clone(), format!("V_{}", v.name)))
            .collect();
        tab.template_values.insert("ADT_A01".to_string(), expected.clone());

        let text = tab.try_build_template_message().unwrap();
        let normalized = text.replace('\n', "\r");
        let message = parse_message(&normalized).unwrap();
        let restored = backfill_values(&message, &variables);
        assert_eq!(restored, expected);
    }

    #[test]
    fn backfill_keeps_multi_component_fields_intact() {
        let mut tab = BuilderTab {
            build_mode: BuildMode::Template,
            ..BuilderTab::default()
        };
        tab.template_values.insert(
            "ADT_A01".to_string(),
            [("patient_name".to_string(), "DOE^JOHN".to_string())]
                .into_iter()
                .collect(),
        );
        let text = tab.try_build_template_message().unwrap();
        let normalized = text.replace('\n', "\r");
        let message = parse_message(&normalized).unwrap();

        let template = tab.template_library.get("ADT_A01").unwrap().clone();
        let variables = collect_template_variables(&template);
        let restored = backfill_values(&message, &variables);
        assert_eq!(restored.get("patient_name").map(String::as_str), Some("DOE^JOHN"));
        // Empty locations contribute nothing.
        assert!(!restored.contains_key("visit_number"));
    }

    #[test]
    fn baked_template_round_trips_through_yaml() {
        let library = TemplateLibrary::new();
        let template = library.get("ADT_A01").unwrap();
        let values: HashMap<String, String> = [
            ("sending_app".to_string(), "MYAPP".to_string()),
            ("patient_id".to_string(), "".to_string()),
        ]
        .into_iter()
        .collect();
        let baked = bake_template_values(template, "MY_ADT".to_string(), &values);
        assert_eq!(baked.name, "MY_ADT");
        let defaults = baked.variables.as_ref().unwrap();
        assert_eq!(defaults.get("sending_app").map(String::as_str), Some("MYAPP"));
        assert!(!defaults.contains_key("patient_id"));

        let yaml = baked.to_yaml().unwrap();
        let reloaded = MessageTemplate::from_yaml(&yaml).unwrap();
        assert_eq!(reloaded.name, "MY_ADT");
        assert_eq!(reloaded.message_type, "ADT");
        assert_eq!(reloaded.trigger_event, "A01");

        // Baked defaults drive the engine: sending_app needs no variable.
        let mut engine = TemplateEngine::new();
        for variable in collect_template_variables(&reloaded) {
            if variable.name != "sending_app" {
                engine.set_variable(variable.name, "");
            }
        }
        let message = engine.create_message(&reloaded).unwrap();
        let msh = message.get_segment_by_id("MSH").unwrap();
        assert_eq!(msh.get_field_value(3), Some("MYAPP"));
    }

    #[test]
    fn register_template_protects_builtins_and_allows_reload() {
        let mut tab = BuilderTab::default();
        let library = TemplateLibrary::new();

        // A file named like a built-in gets a suffix, twice in a row.
        let first = tab.register_template(library.get("ADT_A01").unwrap().clone());
        assert_eq!(first, "ADT_A01 2");
        assert_eq!(tab.selected_template, "ADT_A01 2");
        let second = tab.register_template(library.get("ADT_A01").unwrap().clone());
        assert_eq!(second, "ADT_A01 3");

        // Reloading a loaded template overwrites it in place.
        let reload = tab.register_template(
            MessageTemplate::from_yaml(
                &tab.template_library
                    .get("ADT_A01 2")
                    .unwrap()
                    .to_yaml()
                    .unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(reload, "ADT_A01 2");

        // The built-in itself is untouched.
        let builtin = tab.template_library.get("ADT_A01").unwrap();
        assert!(builtin.variables.is_none());
    }

    #[test]
    fn backfill_uses_component_positions_not_vector_index() {
        use vyna_hl7_templates::{ComponentTemplate, FieldTemplate, SegmentTemplate};
        // Components at positions 1 and 3: "Jr" lives at vec[2], so
        // recording index+1 (2) would backfill the gap instead.
        let mut template = MessageTemplate::new("POS", "2.5", "ADT", "A01");
        let mut pid = SegmentTemplate::new("PID");
        pid.add_field(
            5,
            FieldTemplate::new().with_components(vec![
                ComponentTemplate::new(1).with_placeholder("{{family}}"),
                ComponentTemplate::new(3).with_placeholder("{{suffix}}"),
            ]),
        );
        template.add_segment(pid);
        let variables = collect_template_variables(&template);
        let suffix = variables.iter().find(|v| v.name == "suffix").unwrap();
        assert_eq!(suffix.locations[0].component, Some(3));

        let mut engine = TemplateEngine::new();
        engine.set_variable("family", "Doe");
        engine.set_variable("suffix", "Jr");
        let message = engine.create_message(&template).unwrap();
        let restored = backfill_values(&message, &variables);
        assert_eq!(restored.get("suffix").map(String::as_str), Some("Jr"));
        assert_eq!(restored.get("family").map(String::as_str), Some("Doe"));
    }
}
