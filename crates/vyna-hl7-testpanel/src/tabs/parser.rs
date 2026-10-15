//! Message Parser Tab - Parse and analyze HL7 messages

use crate::samples;
use crate::utils::{TreeNode, format_bytes, format_message_tree, get_message_stats};
use egui::{self, Color32, RichText};
use egui_extras::{Size, StripBuilder};
use vyna_hl7_core::Message;
use vyna_hl7_parser::parse_message;

pub struct ParserTab {
    input_message: String,
    parsed_message: Option<Message>,
    parse_error: Option<String>,
    tree_nodes: Vec<TreeNode>,
    show_raw: bool,
    selected_sample: usize,
}

impl Default for ParserTab {
    fn default() -> Self {
        let default_message = samples::ADT_A01.to_string();
        Self {
            input_message: default_message,
            parsed_message: None,
            parse_error: None,
            tree_nodes: Vec::new(),
            show_raw: false,
            selected_sample: 0,
        }
    }
}

impl ParserTab {
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
        ui.heading("Message Parser");
        ui.label("Parse HL7 v2.x messages and explore their structure.");
        ui.add_space(10.0);

        // Sample message selector
        ui.horizontal(|ui| {
            ui.label("Load Sample:");
            let samples = samples::get_sample_messages();
            egui::ComboBox::from_id_salt("sample_selector")
                .selected_text(samples[self.selected_sample].0)
                .show_ui(ui, |ui| {
                    for (idx, (name, desc, _)) in samples.iter().enumerate() {
                        let response = ui.selectable_value(&mut self.selected_sample, idx, *name);
                        if response.clicked() {
                            self.input_message = samples[idx].2.to_string();
                            self.parsed_message = None;
                            self.parse_error = None;
                            self.tree_nodes.clear();
                        }
                        response.on_hover_text(*desc);
                    }
                });

            ui.separator();

            if ui.button("Parse Message").clicked() {
                self.parse_message();
            }

            if ui.button("Clear").clicked() {
                self.input_message.clear();
                self.parsed_message = None;
                self.parse_error = None;
                self.tree_nodes.clear();
            }

            ui.separator();
            ui.checkbox(&mut self.show_raw, "Show Raw Output");
        });

        ui.add_space(10.0);

        // Get available height for full-height panels
        let available_height = ui.available_height();

        // Split view: input on left, output on right
        StripBuilder::new(ui)
            .size(Size::relative(0.5).at_least(350.0))
            .size(Size::remainder().at_least(350.0))
            .horizontal(|mut strip| {
                // Left column: Input
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        ui.heading("Input Message");
                        ui.label(format!("Size: {}", format_bytes(self.input_message.len())));

                        egui::ScrollArea::vertical()
                            .id_salt("input_scroll")
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
                    });
                });

                // Right column: Output
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        if let Some(ref error) = self.parse_error {
                            ui.heading("Parse Error");
                            ui.colored_label(Color32::RED, error);
                        } else if let Some(ref message) = self.parsed_message {
                            let stats = get_message_stats(message);

                            ui.heading("Parsed Message");

                            // Statistics panel
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("Type:").strong());
                                ui.label(&stats.message_type);
                                ui.separator();
                                ui.label(RichText::new("Version:").strong());
                                ui.label(&stats.version);
                                ui.separator();
                                ui.label(RichText::new("Segments:").strong());
                                ui.label(stats.segment_count.to_string());
                                ui.separator();
                                ui.label(RichText::new("Fields:").strong());
                                ui.label(stats.field_count.to_string());
                            });

                            ui.add_space(5.0);

                            // Segment type breakdown
                            ui.collapsing("Segment Breakdown", |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    let mut types: Vec<_> = stats.segment_types.iter().collect();
                                    types.sort_by(|a, b| a.0.cmp(b.0));
                                    for (seg_type, count) in types {
                                        ui.label(format!("{}: {}", seg_type, count));
                                    }
                                });
                            });

                            ui.add_space(10.0);

                            if self.show_raw {
                                // Raw encoded output
                                ui.label(RichText::new("Encoded Output:").strong());
                                let encoded = message.encode_with_separator("\n");
                                egui::ScrollArea::vertical()
                                    .id_salt("raw_scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(&mut encoded.as_str())
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .interactive(false),
                                        );
                                    });
                            } else {
                                // Tree view
                                ui.label(RichText::new("Message Structure:").strong());
                                egui::ScrollArea::vertical()
                                    .id_salt("tree_scroll")
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for node in &mut self.tree_nodes {
                                            node.ui(ui);
                                        }
                                    });
                            }
                        } else {
                            ui.heading("Output");
                            ui.label("Click 'Parse Message' to parse the input.");
                            ui.add_space(20.0);
                            ui.label("Supported message types:");
                            ui.label("- ADT (A01-A40)");
                            ui.label("- ORU, ORM, ORS");
                            ui.label("- SIU (S12-S26)");
                            ui.label("- VXU, RDE, RAS");
                            ui.label("- ACK, QRY, and more...");
                        }
                    });
                });
            });
    }

    fn parse_message(&mut self) {
        // Normalize line endings
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
}
