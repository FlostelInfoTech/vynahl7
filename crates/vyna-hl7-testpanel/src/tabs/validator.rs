//! Validator Tab - Validate HL7 messages against schemas

use crate::samples;
use crate::utils::{TreeNode, format_message_tree};
use egui::{self, Color32, RichText};
use egui_extras::{Size, StripBuilder};
use vyna_hl7_core::{Message, Version};
use vyna_hl7_parser::parse_message;
use vyna_hl7_validator::{ValidationContext, Validator};

pub struct ValidatorTab {
    input_message: String,
    parsed_message: Option<Message>,
    parse_error: Option<String>,
    validation_results: Vec<ValidationItem>,
    selected_version: VersionOption,
    validate_datatypes: bool,
    validate_schema: bool,
    is_valid: Option<bool>,
    tree_nodes: Vec<TreeNode>,
    show_tree_view: bool,
}

#[derive(Clone)]
struct ValidationItem {
    severity: Severity,
    location: String,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum VersionOption {
    #[default]
    Auto,
    V2_1,
    V2_2,
    V23,
    V231,
    V24,
    V25,
    V251,
    V26,
    V27,
    V2_7_1,
    V2_8,
    V2_8_1,
    V2_8_2,
}

impl VersionOption {
    fn to_version(self) -> Option<Version> {
        match self {
            VersionOption::Auto => None,
            VersionOption::V2_1 => Some(Version::V2_1),
            VersionOption::V2_2 => Some(Version::V2_2),
            VersionOption::V23 => Some(Version::V2_3),
            VersionOption::V231 => Some(Version::V2_3_1),
            VersionOption::V24 => Some(Version::V2_4),
            VersionOption::V25 => Some(Version::V2_5),
            VersionOption::V251 => Some(Version::V2_5_1),
            VersionOption::V26 => Some(Version::V2_6),
            VersionOption::V27 => Some(Version::V2_7),
            VersionOption::V2_7_1 => Some(Version::V2_7_1),
            VersionOption::V2_8 => Some(Version::V2_8),
            VersionOption::V2_8_1 => Some(Version::V2_8_1),
            VersionOption::V2_8_2 => Some(Version::V2_8_2),
        }
    }

    fn label(&self) -> &str {
        match self {
            VersionOption::Auto => "Auto-detect",
            VersionOption::V2_1 => "2.1",
            VersionOption::V2_2 => "2.2",
            VersionOption::V23 => "2.3",
            VersionOption::V231 => "2.3.1",
            VersionOption::V24 => "2.4",
            VersionOption::V25 => "2.5",
            VersionOption::V251 => "2.5.1",
            VersionOption::V26 => "2.6",
            VersionOption::V27 => "2.7",
            VersionOption::V2_7_1 => "2.7.1",
            VersionOption::V2_8 => "2.8",
            VersionOption::V2_8_1 => "2.8.1",
            VersionOption::V2_8_2 => "2.8.2",
        }
    }
}

impl Default for ValidatorTab {
    fn default() -> Self {
        Self {
            input_message: samples::ADT_A01.to_string(),
            parsed_message: None,
            parse_error: None,
            validation_results: Vec::new(),
            selected_version: VersionOption::Auto,
            validate_datatypes: true,
            validate_schema: true,
            is_valid: None,
            tree_nodes: Vec::new(),
            show_tree_view: true,
        }
    }
}

impl ValidatorTab {
    /// Set the input message content (used by File > Open)
    pub fn set_message(&mut self, content: String) {
        self.input_message = content;
        self.parsed_message = None;
        self.parse_error = None;
        self.validation_results.clear();
        self.is_valid = None;
        self.tree_nodes.clear();
    }

    /// Get the current input message content (used by File > Save)
    pub fn get_message(&self) -> &str {
        &self.input_message
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Message Validator");
        ui.label("Validate HL7 messages against schemas and data type rules.");
        ui.add_space(10.0);

        // Options bar
        ui.horizontal(|ui| {
            ui.label("HL7 Version:");
            egui::ComboBox::from_id_salt("version_select")
                .selected_text(self.selected_version.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.selected_version,
                        VersionOption::Auto,
                        "Auto-detect",
                    );
                    ui.selectable_value(&mut self.selected_version, VersionOption::V2_1, "2.1");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V2_2, "2.2");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V23, "2.3");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V231, "2.3.1");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V24, "2.4");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V25, "2.5");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V251, "2.5.1");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V26, "2.6");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V27, "2.7");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V2_7_1, "2.7.1");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V2_8, "2.8");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V2_8_1, "2.8.1");
                    ui.selectable_value(&mut self.selected_version, VersionOption::V2_8_2, "2.8.2");
                });

            ui.separator();

            ui.checkbox(&mut self.validate_schema, "Schema Validation");
            ui.checkbox(&mut self.validate_datatypes, "Data Type Validation");

            ui.separator();

            if ui.button(RichText::new("Validate").strong()).clicked() {
                self.validate();
            }

            if ui.button("Clear Results").clicked() {
                self.validation_results.clear();
                self.is_valid = None;
            }
        });

        ui.add_space(10.0);

        // Get available height for full-height panels
        let available_height = ui.available_height();

        StripBuilder::new(ui)
            .size(Size::relative(0.5).at_least(350.0))
            .size(Size::remainder().at_least(350.0))
            .horizontal(|mut strip| {
                // Left: Message input
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        ui.horizontal(|ui| {
                            ui.heading("Message Input");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.checkbox(&mut self.show_tree_view, "Tree View");
                                },
                            );
                        });

                        if let Some(ref error) = self.parse_error {
                            ui.colored_label(Color32::RED, format!("Parse Error: {}", error));
                        }

                        if self.show_tree_view && !self.tree_nodes.is_empty() {
                            // Show tree view
                            egui::ScrollArea::vertical()
                                .id_salt("validator_tree")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for node in &mut self.tree_nodes {
                                        node.ui(ui);
                                    }
                                });
                        } else {
                            // Show raw input
                            egui::ScrollArea::vertical()
                                .id_salt("validator_input")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.input_message)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(30)
                                            .code_editor(),
                                    );
                                });
                        }
                    });
                });

                // Right: Results
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        ui.heading("Validation Results");
                        ui.label("Strict validation (HL7 specification compliance).");

                        // Summary
                        if let Some(is_valid) = self.is_valid {
                            if is_valid {
                                ui.colored_label(Color32::GREEN, RichText::new("VALID").strong());
                            } else {
                                ui.colored_label(Color32::RED, RichText::new("INVALID").strong());
                            }

                            let error_count = self
                                .validation_results
                                .iter()
                                .filter(|r| matches!(r.severity, Severity::Error))
                                .count();
                            let warning_count = self
                                .validation_results
                                .iter()
                                .filter(|r| matches!(r.severity, Severity::Warning))
                                .count();
                            let info_count = self
                                .validation_results
                                .iter()
                                .filter(|r| matches!(r.severity, Severity::Info))
                                .count();

                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::RED, format!("{} Errors", error_count));
                                ui.colored_label(
                                    Color32::YELLOW,
                                    format!("{} Warnings", warning_count),
                                );
                                ui.colored_label(
                                    Color32::LIGHT_BLUE,
                                    format!("{} Info", info_count),
                                );
                            });
                        }

                        ui.add_space(10.0);
                        ui.separator();

                        // Results list
                        egui::ScrollArea::vertical()
                            .id_salt("validation_results")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if self.validation_results.is_empty() && self.is_valid.is_none() {
                                    ui.label("Click 'Validate' to check the message.");
                                    ui.add_space(10.0);
                                    ui.label("Validation checks:");
                                    ui.label("- Message structure (required segments)");
                                    ui.label("- Segment order");
                                    ui.label("- Required fields");
                                    ui.label("- Data type formats (dates, times, IDs)");
                                    ui.label("- Field lengths");
                                } else if self.validation_results.is_empty() {
                                    ui.colored_label(Color32::GREEN, "No issues found!");
                                } else {
                                    for item in &self.validation_results {
                                        ui.horizontal(|ui| {
                                            let (icon, color) = match item.severity {
                                                Severity::Error => ("", Color32::RED),
                                                Severity::Warning => ("", Color32::YELLOW),
                                                Severity::Info => ("", Color32::LIGHT_BLUE),
                                            };

                                            ui.colored_label(color, icon);
                                            ui.vertical(|ui| {
                                                ui.label(RichText::new(&item.location).strong());
                                                ui.label(&item.message);
                                            });
                                        });
                                        ui.add_space(5.0);
                                    }
                                }
                            });
                    });
                });
            });
    }

    fn validate(&mut self) {
        self.validation_results.clear();
        self.is_valid = None;
        self.tree_nodes.clear();

        // Parse the message first
        let normalized = self.input_message.replace("\r\n", "\r").replace("\n", "\r");

        match parse_message(&normalized) {
            Ok(message) => {
                self.parsed_message = Some(message.clone());
                self.parse_error = None;
                self.tree_nodes = format_message_tree(&message);

                // Determine version
                let version = self
                    .selected_version
                    .to_version()
                    .or_else(|| message.get_version())
                    .unwrap_or(Version::V2_5_1);

                // Basic structure validation
                self.validate_basic_structure(&message);

                // Schema validation
                if self.validate_schema {
                    if let Some((msg_type, trigger)) = message.get_message_type() {
                        self.validation_results.push(ValidationItem {
                            severity: Severity::Info,
                            location: "MSH-9".to_string(),
                            message: format!(
                                "Message type: {}^{}, Version: {}",
                                msg_type,
                                trigger,
                                version.as_str()
                            ),
                        });
                    }

                    // Use the Validator for schema validation, selecting the
                    // schema from MSH-9 (ACK carries no trigger event).
                    let schema_key = message.get_message_type().map(|(msg_type, trigger)| {
                        if msg_type == "ACK" {
                            (msg_type, String::new())
                        } else {
                            (msg_type, trigger)
                        }
                    });
                    let mut validator = Validator::new(version);
                    let mut schema_applied = false;
                    if let Some((msg_type, trigger)) = &schema_key
                        && let Ok(schema_validator) =
                            Validator::for_message_type(version, msg_type, trigger)
                    {
                        validator = schema_validator;
                        schema_applied = true;
                    }
                    // Strict spec compliance, stated explicitly in the UI.
                    validator.set_context(ValidationContext::strict());
                    if !schema_applied {
                        self.validation_results.push(ValidationItem {
                            severity: Severity::Warning,
                            location: "Schema".to_string(),
                            message: "No schema for this message type; structure-only validation."
                                .to_string(),
                        });
                    }
                    let validation_result = validator.validate(&message);
                    self.push_validation_result(&validation_result);

                    if validation_result.is_valid {
                        self.validation_results.push(ValidationItem {
                            severity: Severity::Info,
                            location: "Schema".to_string(),
                            message: format!(
                                "Schema validation passed for HL7 v{}",
                                version.as_str()
                            ),
                        });
                    }
                }

                // Data type validation
                if self.validate_datatypes {
                    self.validate_data_types(&message);
                }

                // Check for any errors
                let has_errors = self
                    .validation_results
                    .iter()
                    .any(|r| matches!(r.severity, Severity::Error));

                self.is_valid = Some(!has_errors);
            }
            Err(e) => {
                self.parse_error = Some(format!("{}", e));
                self.validation_results.push(ValidationItem {
                    severity: Severity::Error,
                    location: "Parse".to_string(),
                    message: format!("Failed to parse message: {}", e),
                });
                self.is_valid = Some(false);
            }
        }
    }

    /// Copy validator findings into the results list, preserving each
    /// finding's structured location and message.
    fn push_validation_result(&mut self, result: &vyna_hl7_validator::ValidationResult) {
        if !result.is_valid() {
            for err in &result.errors {
                self.validation_results.push(ValidationItem {
                    severity: Severity::Error,
                    location: err.location.clone(),
                    message: err.message.clone(),
                });
            }
        }

        for warning in &result.warnings {
            self.validation_results.push(ValidationItem {
                severity: Severity::Warning,
                location: warning.location.clone(),
                message: warning.message.clone(),
            });
        }
    }

    fn validate_basic_structure(&mut self, message: &Message) {
        // Check MSH segment
        if message.segments.is_empty() {
            self.validation_results.push(ValidationItem {
                severity: Severity::Error,
                location: "Message".to_string(),
                message: "Message contains no segments".to_string(),
            });
            return;
        }

        if message.segments[0].id != "MSH" {
            self.validation_results.push(ValidationItem {
                severity: Severity::Error,
                location: "MSH".to_string(),
                message: "First segment must be MSH".to_string(),
            });
        }

        // Check MSH required fields
        let msh = &message.segments[0];
        if msh.get_field_value(9).is_none() {
            self.validation_results.push(ValidationItem {
                severity: Severity::Error,
                location: "MSH-9".to_string(),
                message: "Message type (MSH-9) is required".to_string(),
            });
        }

        if msh.get_field_value(10).is_none() {
            self.validation_results.push(ValidationItem {
                severity: Severity::Error,
                location: "MSH-10".to_string(),
                message: "Message control ID (MSH-10) is required".to_string(),
            });
        }

        if msh.get_field_value(12).is_none() {
            self.validation_results.push(ValidationItem {
                severity: Severity::Warning,
                location: "MSH-12".to_string(),
                message: "Version ID (MSH-12) is recommended".to_string(),
            });
        }

        // Check for PID in ADT messages
        if let Some((msg_type, _)) = message.get_message_type()
            && msg_type == "ADT"
        {
            let has_pid = message.segments.iter().any(|s| s.id == "PID");
            if !has_pid {
                self.validation_results.push(ValidationItem {
                    severity: Severity::Error,
                    location: "PID".to_string(),
                    message: "ADT messages require a PID segment".to_string(),
                });
            }
        }
    }

    fn validate_data_types(&mut self, message: &Message) {
        // Manual data type validation for common fields
        for segment in &message.segments {
            // Validate date/time fields
            if segment.id == "MSH"
                && let Some(datetime) = segment.get_field_value(7)
                && !Self::is_valid_datetime(datetime)
            {
                self.validation_results.push(ValidationItem {
                    severity: Severity::Warning,
                    location: "MSH-7".to_string(),
                    message: format!("Date/time format may be invalid: {}", datetime),
                });
            }

            if segment.id == "PID" {
                // Validate birth date
                if let Some(dob) = segment.get_field_value(7)
                    && !dob.is_empty()
                    && !Self::is_valid_date(dob)
                {
                    self.validation_results.push(ValidationItem {
                        severity: Severity::Warning,
                        location: "PID-7".to_string(),
                        message: format!("Birth date format may be invalid: {}", dob),
                    });
                }

                // Validate gender
                if let Some(gender) = segment.get_field_value(8) {
                    let valid_genders = ["M", "F", "O", "U", "A", "N"];
                    if !gender.is_empty() && !valid_genders.contains(&gender) {
                        self.validation_results.push(ValidationItem {
                            severity: Severity::Warning,
                            location: "PID-8".to_string(),
                            message: format!(
                                "Gender code '{}' is not standard (expected M/F/O/U)",
                                gender
                            ),
                        });
                    }
                }
            }
        }
    }

    fn is_valid_datetime(s: &str) -> bool {
        // HL7 datetime: YYYY[MM[DD[HH[MM[SS[.S[S[S[S]]]]]]]]][+/-ZZZZ]
        let s = s.trim();
        if s.len() < 4 {
            return false;
        }
        s.chars()
            .take(14)
            .all(|c| c.is_ascii_digit() || c == '.' || c == '+' || c == '-')
    }

    fn is_valid_date(s: &str) -> bool {
        // HL7 date: YYYY[MM[DD]]
        let s = s.trim();
        if s.len() < 4 {
            return false;
        }
        s.chars().take(8).all(|c| c.is_ascii_digit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vyna_hl7_validator::{
        ValidationError, ValidationErrorType, ValidationResult, ValidationWarning,
    };

    #[test]
    fn findings_keep_structured_location_and_message() {
        let mut tab = ValidatorTab::default();
        tab.validation_results.clear();

        let mut result = ValidationResult::new();
        result.add_error(ValidationError::new(
            "PID-5".to_string(),
            "Required field 5 is missing or empty".to_string(),
            ValidationErrorType::MissingRequiredField,
        ));
        result.add_warning(ValidationWarning::new(
            "MSH-12".to_string(),
            "Version drift: message says 2.3".to_string(),
        ));

        tab.push_validation_result(&result);

        assert_eq!(tab.validation_results.len(), 2);
        let error = &tab.validation_results[0];
        assert!(matches!(error.severity, Severity::Error));
        assert_eq!(error.location, "PID-5");
        assert_eq!(error.message, "Required field 5 is missing or empty");
        let warning = &tab.validation_results[1];
        assert!(matches!(warning.severity, Severity::Warning));
        assert_eq!(warning.location, "MSH-12");
        assert_eq!(warning.message, "Version drift: message says 2.3");
    }

    #[test]
    fn version_option_covers_every_supported_version() {
        // Every core Version must be pinnable in the dropdown.
        let cases = [
            (VersionOption::V2_1, Version::V2_1, "2.1"),
            (VersionOption::V2_2, Version::V2_2, "2.2"),
            (VersionOption::V23, Version::V2_3, "2.3"),
            (VersionOption::V231, Version::V2_3_1, "2.3.1"),
            (VersionOption::V24, Version::V2_4, "2.4"),
            (VersionOption::V25, Version::V2_5, "2.5"),
            (VersionOption::V251, Version::V2_5_1, "2.5.1"),
            (VersionOption::V26, Version::V2_6, "2.6"),
            (VersionOption::V27, Version::V2_7, "2.7"),
            (VersionOption::V2_7_1, Version::V2_7_1, "2.7.1"),
            (VersionOption::V2_8, Version::V2_8, "2.8"),
            (VersionOption::V2_8_1, Version::V2_8_1, "2.8.1"),
            (VersionOption::V2_8_2, Version::V2_8_2, "2.8.2"),
        ];
        assert_eq!(cases.len(), 13);
        for (option, version, label) in cases {
            assert_eq!(option.to_version(), Some(version));
            assert_eq!(option.label(), label);
        }
        assert_eq!(VersionOption::Auto.to_version(), None);
        assert_eq!(VersionOption::Auto.label(), "Auto-detect");
    }

    #[test]
    fn validate_reports_schema_locations() {
        // Structurally incomplete ADT: the schema errors must surface
        // with their real locations, not a blanket label.
        let mut tab = ValidatorTab {
            input_message: "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5\rPID|1||7".to_string(),
            ..ValidatorTab::default()
        };
        tab.validate();

        assert_eq!(tab.is_valid, Some(false));
        assert!(
            tab.validation_results.iter().any(|r| {
                matches!(r.severity, Severity::Error)
                    && r.location == "PID[1]-5"
                    && r.message.contains("Required field 5")
            }),
            "results: {:?}",
            tab.validation_results
                .iter()
                .map(|r| (&r.location, &r.message))
                .collect::<Vec<_>>()
        );
    }
}
