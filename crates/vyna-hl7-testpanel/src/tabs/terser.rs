//! Terser Tab - Field access using path notation

use crate::samples;
use crate::utils::{TreeNode, format_message_tree};
use egui::{self, Color32, RichText};
use egui_extras::{Size, StripBuilder};
use vyna_hl7_core::Message;
use vyna_hl7_parser::parse_message;
use vyna_hl7_terser::{Terser, TerserMut};

pub struct TerserTab {
    input_message: String,
    parsed_message: Option<Message>,
    parse_error: Option<String>,
    terser_path: String,
    terser_result: Option<String>,
    terser_error: Option<String>,
    path_history: Vec<(String, String)>,
    new_value: String,
    show_help: bool,
    tree_nodes: Vec<TreeNode>,
    show_tree_view: bool,
}

impl Default for TerserTab {
    fn default() -> Self {
        Self {
            input_message: samples::ADT_A01.to_string(),
            parsed_message: None,
            parse_error: None,
            terser_path: "PID-5-1".to_string(),
            terser_result: None,
            terser_error: None,
            path_history: Vec::new(),
            new_value: String::new(),
            show_help: true,
            tree_nodes: Vec::new(),
            show_tree_view: false,
        }
    }
}

impl TerserTab {
    /// Set the input message content (used by File > Open)
    pub fn set_message(&mut self, content: String) {
        self.input_message = content;
        self.parsed_message = None;
        self.parse_error = None;
        self.tree_nodes.clear();
    }

    /// Get the current input message content (used by File > Save)
    pub fn get_message(&self) -> &str {
        &self.input_message
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Terser - Field Access");
        ui.label(
            "Access and modify HL7 message fields using path notation (similar to HAPI Terser).",
        );
        ui.add_space(10.0);

        // Top toolbar
        ui.horizontal(|ui| {
            if ui.button("Parse Message").clicked() {
                self.parse_message();
            }

            ui.separator();

            ui.label("Path:");
            ui.add_sized(
                [150.0, 20.0],
                egui::TextEdit::singleline(&mut self.terser_path),
            );

            if ui.button("Get Value").clicked() {
                self.get_value();
            }

            ui.separator();

            ui.label("New Value:");
            ui.add_sized(
                [150.0, 20.0],
                egui::TextEdit::singleline(&mut self.new_value),
            );

            if ui.button("Set Value").clicked() {
                self.set_value();
            }

            ui.separator();

            ui.checkbox(&mut self.show_help, "Show Help");
        });

        ui.add_space(10.0);

        // Quick path buttons
        ui.horizontal_wrapped(|ui| {
            ui.label("Quick Paths:");
            let quick_paths = [
                ("Patient Name", "PID-5"),
                ("Family Name", "PID-5-1"),
                ("Given Name", "PID-5-2"),
                ("DOB", "PID-7"),
                ("Gender", "PID-8"),
                ("Address", "PID-11"),
                ("Message Type", "MSH-9"),
                ("Sending App", "MSH-3"),
                ("OBX-5 (1st)", "OBX(1)-5"),
                ("OBX-5 (2nd)", "OBX(2)-5"),
            ];

            for (label, path) in quick_paths {
                if ui.small_button(label).clicked() {
                    self.terser_path = path.to_string();
                    self.get_value();
                }
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
                            ui.heading("Message");
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.checkbox(&mut self.show_tree_view, "Tree View");
                                },
                            );
                        });

                        if let Some(ref error) = self.parse_error {
                            ui.colored_label(Color32::RED, error);
                        } else if self.parsed_message.is_some() {
                            ui.colored_label(Color32::GREEN, "Message parsed successfully");
                        }

                        if self.show_tree_view && !self.tree_nodes.is_empty() {
                            // Show tree view with clickable paths
                            ui.label(
                                egui::RichText::new("Click any field to query its value")
                                    .small()
                                    .italics(),
                            );
                            ui.add_space(4.0);

                            let mut clicked_path: Option<String> = None;
                            egui::ScrollArea::vertical()
                                .id_salt("terser_tree")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    for node in &mut self.tree_nodes {
                                        if let Some(path) = node.ui_interactive(ui) {
                                            clicked_path = Some(path);
                                        }
                                    }
                                });

                            // Handle clicked path - set it and query
                            if let Some(path) = clicked_path {
                                self.terser_path = path;
                                self.get_value();
                            }
                        } else {
                            // Show raw input
                            egui::ScrollArea::vertical()
                                .id_salt("terser_input")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.input_message)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(25)
                                            .code_editor(),
                                    );
                                });
                        }
                    });
                });

                // Right: Results and help
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        // Result display
                        ui.heading("Result");

                        if let Some(ref error) = self.terser_error {
                            ui.colored_label(Color32::RED, format!("Error: {}", error));
                        } else if let Some(ref result) = self.terser_result {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Path:").strong());
                                ui.label(&self.terser_path);
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Value:").strong());
                                if result.is_empty() {
                                    ui.colored_label(Color32::GRAY, "(empty)");
                                } else {
                                    ui.code(result);
                                }
                            });
                        }

                        ui.add_space(10.0);
                        ui.separator();

                        // History
                        ui.heading("Query History");
                        let mut history_clicked_path: Option<String> = None;
                        egui::ScrollArea::vertical()
                            .id_salt("terser_history")
                            .auto_shrink([false, false])
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for (path, value) in self.path_history.iter().rev().take(10) {
                                    ui.horizontal(|ui| {
                                        if ui
                                            .small_button("🔄")
                                            .on_hover_text("Re-query this path")
                                            .clicked()
                                        {
                                            history_clicked_path = Some(path.clone());
                                        }
                                        ui.label(format!(
                                            "{} = {}",
                                            path,
                                            if value.is_empty() { "(empty)" } else { value }
                                        ));
                                    });
                                }
                            });

                        // Handle history click - set path and query
                        if let Some(path) = history_clicked_path {
                            self.terser_path = path;
                            self.get_value();
                        }

                        if self.show_help {
                            ui.add_space(10.0);
                            ui.separator();

                            // Help section
                            ui.heading("Path Notation Help");
                            egui::ScrollArea::vertical()
                                .id_salt("terser_help")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Basic Syntax:").strong());
                                    ui.label("SEGMENT-FIELD[-COMPONENT[-SUBCOMPONENT]]");
                                    ui.add_space(5.0);

                                    ui.label(RichText::new("Examples:").strong());
                                    ui.code("PID-5       # Field 5 of PID");
                                    ui.code("PID-5-1     # Component 1 of field 5");
                                    ui.code("PID-5-1-2   # Subcomponent 2");
                                    ui.code("OBX(1)-5    # Field 5 of first OBX");
                                    ui.code("OBX(2)-5    # Field 5 of second OBX");
                                    ui.code("PID-11(1)-1 # First repetition, component 1");
                                    ui.add_space(5.0);

                                    ui.label(RichText::new("Notes:").strong());
                                    ui.label("- Segment indexing is 1-based: OBX(1), OBX(2)");
                                    ui.label("- Field indexing follows HL7 spec");
                                    ui.label("- MSH-1 is the field separator");
                                    ui.label("- Repetitions are 1-based (PID-11(1) = first)");
                                });
                        }
                    });
                });
            });
    }

    fn parse_message(&mut self) {
        let normalized = self.input_message.replace("\r\n", "\r").replace("\n", "\r");

        match parse_message(&normalized) {
            Ok(message) => {
                self.tree_nodes = format_message_tree(&message);
                self.parsed_message = Some(message);
                self.parse_error = None;
            }
            Err(e) => {
                self.parse_error = Some(format!("{}", e));
                self.parsed_message = None;
                self.tree_nodes.clear();
            }
        }
    }

    fn get_value(&mut self) {
        if self.parsed_message.is_none() {
            self.parse_message();
        }

        if let Some(ref message) = self.parsed_message {
            let terser = Terser::new(message);
            match terser.get(&self.terser_path) {
                Ok(Some(value)) => {
                    self.terser_result = Some(value.to_string());
                    self.terser_error = None;
                    self.path_history
                        .push((self.terser_path.clone(), value.to_string()));
                }
                Ok(None) => {
                    self.terser_result = Some(String::new());
                    self.terser_error = None;
                    self.path_history
                        .push((self.terser_path.clone(), String::new()));
                }
                Err(e) => {
                    self.terser_error = Some(format!("{}", e));
                    self.terser_result = None;
                }
            }
        } else {
            self.terser_error = Some("Please parse a message first".to_string());
        }
    }

    fn set_value(&mut self) {
        if self.parsed_message.is_none() {
            self.parse_message();
        }

        if let Some(ref mut message) = self.parsed_message {
            let mut terser = TerserMut::new(message);
            match terser.set(&self.terser_path, &self.new_value) {
                Ok(_) => {
                    // Update the input with the modified message
                    self.input_message = message.encode_with_separator("\n");
                    self.terser_result =
                        Some(format!("Set {} = {}", self.terser_path, self.new_value));
                    self.terser_error = None;
                    // Re-parse to update internal state
                    self.parse_message();
                }
                Err(e) => {
                    self.terser_error = Some(format!("{}", e));
                }
            }
        } else {
            self.terser_error = Some("Please parse a message first".to_string());
        }
    }
}
