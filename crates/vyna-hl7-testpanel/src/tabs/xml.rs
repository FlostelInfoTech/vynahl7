//! XML Tab - Convert between ER7 and XML formats

use crate::samples;
use egui::{self, Color32, RichText};
use egui_extras::{Size, StripBuilder};
use vyna_hl7_parser::parse_message;
use vyna_hl7_xml::{XmlDecoder, XmlEncoder, XmlEncoderConfig};

pub struct XmlTab {
    // ER7 to XML
    er7_input: String,
    xml_output: String,
    er7_to_xml_error: Option<String>,

    // XML to ER7
    xml_input: String,
    er7_output: String,
    xml_to_er7_error: Option<String>,

    // Options
    pretty_print: bool,
    include_declaration: bool,
    active_direction: XmlDirection,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum XmlDirection {
    #[default]
    Er7ToXml,
    XmlToEr7,
}

impl Default for XmlTab {
    fn default() -> Self {
        Self {
            er7_input: samples::MINIMAL.to_string(),
            xml_output: String::new(),
            er7_to_xml_error: None,
            xml_input: Self::sample_xml(),
            er7_output: String::new(),
            xml_to_er7_error: None,
            pretty_print: true,
            include_declaration: true,
            active_direction: XmlDirection::Er7ToXml,
        }
    }
}

impl XmlTab {
    fn sample_xml() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<HL7Message xmlns="urn:hl7-org:v2xml">
  <MSH>
    <MSH.1>|</MSH.1>
    <MSH.2>^~\&amp;</MSH.2>
    <MSH.3>SND</MSH.3>
    <MSH.4>SND_FAC</MSH.4>
    <MSH.5>RCV</MSH.5>
    <MSH.6>RCV_FAC</MSH.6>
    <MSH.7>20240101120000</MSH.7>
    <MSH.9>
      <MSH.9.1>ADT</MSH.9.1>
      <MSH.9.2>A01</MSH.9.2>
    </MSH.9>
    <MSH.10>12345</MSH.10>
    <MSH.11>P</MSH.11>
    <MSH.12>2.5</MSH.12>
  </MSH>
  <PID>
    <PID.1>1</PID.1>
    <PID.3>12345</PID.3>
    <PID.5>
      <PID.5.1>DOE</PID.5.1>
      <PID.5.2>JOHN</PID.5.2>
    </PID.5>
    <PID.7>19800101</PID.7>
    <PID.8>M</PID.8>
  </PID>
</HL7Message>"#
            .to_string()
    }

    /// Set the ER7 input message content (used by File > Open)
    pub fn set_message(&mut self, content: String) {
        self.er7_input = content;
        self.xml_output.clear();
        self.er7_to_xml_error = None;
    }

    /// Get the current ER7 input message content (used by File > Save)
    pub fn get_message(&self) -> &str {
        &self.er7_input
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("XML Encoding");
        ui.label("Convert between HL7 ER7 (pipe-delimited) and XML formats.");
        ui.add_space(10.0);

        // Direction selector
        ui.horizontal(|ui| {
            ui.label("Conversion:");
            ui.selectable_value(
                &mut self.active_direction,
                XmlDirection::Er7ToXml,
                "ER7 -> XML",
            );
            ui.selectable_value(
                &mut self.active_direction,
                XmlDirection::XmlToEr7,
                "XML -> ER7",
            );

            ui.separator();

            ui.checkbox(&mut self.pretty_print, "Pretty Print");
            ui.checkbox(&mut self.include_declaration, "XML Declaration");
        });

        ui.add_space(10.0);

        match self.active_direction {
            XmlDirection::Er7ToXml => self.er7_to_xml_ui(ui),
            XmlDirection::XmlToEr7 => self.xml_to_er7_ui(ui),
        }
    }

    fn er7_to_xml_ui(&mut self, ui: &mut egui::Ui) {
        // Controls
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("Convert to XML").strong())
                .clicked()
            {
                self.convert_to_xml();
            }

            ui.separator();

            ui.label("Sample:");
            let samples = samples::get_sample_messages();
            egui::ComboBox::from_id_salt("xml_er7_sample")
                .selected_text("Load...")
                .show_ui(ui, |ui| {
                    for (name, _, msg) in &samples {
                        if ui.selectable_label(false, *name).clicked() {
                            self.er7_input = msg.to_string();
                            self.xml_output.clear();
                            self.er7_to_xml_error = None;
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Get available height for full-height panels
        let available_height = ui.available_height();

        StripBuilder::new(ui)
            .size(Size::relative(0.5).at_least(350.0))
            .size(Size::remainder().at_least(350.0))
            .horizontal(|mut strip| {
                // Left: ER7 Input
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        ui.heading("ER7 Input (Pipe-delimited)");

                        egui::ScrollArea::vertical()
                            .id_salt("er7_input")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.er7_input)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(30)
                                        .code_editor(),
                                );
                            });
                    });
                });

                // Right: XML Output
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        ui.heading("XML Output");

                        if let Some(ref error) = self.er7_to_xml_error {
                            ui.colored_label(Color32::RED, format!("Error: {}", error));
                        }

                        if !self.xml_output.is_empty() {
                            ui.horizontal(|ui| {
                                if ui.button("Copy to Clipboard").clicked() {
                                    ui.ctx().copy_text(self.xml_output.clone());
                                }
                                ui.label(format!("Size: {} bytes", self.xml_output.len()));
                            });
                        }

                        egui::ScrollArea::vertical()
                            .id_salt("xml_output")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.xml_output.as_str())
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(30)
                                        .interactive(false),
                                );
                            });
                    });
                });
            });
    }

    fn xml_to_er7_ui(&mut self, ui: &mut egui::Ui) {
        // Controls
        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("Convert to ER7").strong())
                .clicked()
            {
                self.convert_to_er7();
            }

            if ui.button("Load Sample XML").clicked() {
                self.xml_input = Self::sample_xml();
            }
        });

        ui.add_space(10.0);

        // Get available height for full-height panels
        let available_height = ui.available_height();

        StripBuilder::new(ui)
            .size(Size::relative(0.5).at_least(350.0))
            .size(Size::remainder().at_least(350.0))
            .horizontal(|mut strip| {
                // Left: XML Input
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        ui.heading("XML Input");

                        egui::ScrollArea::vertical()
                            .id_salt("xml_input")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.xml_input)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(30)
                                        .code_editor(),
                                );
                            });
                    });
                });

                // Right: ER7 Output
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());

                        ui.heading("ER7 Output (Pipe-delimited)");

                        if let Some(ref error) = self.xml_to_er7_error {
                            ui.colored_label(Color32::RED, format!("Error: {}", error));
                        }

                        if !self.er7_output.is_empty() {
                            ui.horizontal(|ui| {
                                if ui.button("Copy to Clipboard").clicked() {
                                    ui.ctx().copy_text(self.er7_output.clone());
                                }
                                ui.label(format!("Size: {} bytes", self.er7_output.len()));
                            });
                        }

                        egui::ScrollArea::vertical()
                            .id_salt("er7_output")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.er7_output.as_str())
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(30)
                                        .interactive(false),
                                );
                            });
                    });
                });
            });
    }

    fn convert_to_xml(&mut self) {
        self.xml_output.clear();
        self.er7_to_xml_error = None;

        // Parse the ER7 message
        let normalized = self.er7_input.replace("\r\n", "\r").replace("\n", "\r");

        let message = match parse_message(&normalized) {
            Ok(m) => m,
            Err(e) => {
                self.er7_to_xml_error = Some(format!("Parse error: {}", e));
                return;
            }
        };

        // Configure encoder
        let config = XmlEncoderConfig {
            pretty_print: self.pretty_print,
            include_declaration: self.include_declaration,
            ..Default::default()
        };

        let encoder = XmlEncoder::with_config(config);

        match encoder.encode(&message) {
            Ok(xml) => {
                self.xml_output = xml;
            }
            Err(e) => {
                self.er7_to_xml_error = Some(format!("XML encoding error: {}", e));
            }
        }
    }

    fn convert_to_er7(&mut self) {
        self.er7_output.clear();
        self.xml_to_er7_error = None;

        let decoder = XmlDecoder::new();

        match decoder.decode(&self.xml_input) {
            Ok(message) => {
                self.er7_output = message.encode_with_separator("\n");
            }
            Err(e) => {
                self.xml_to_er7_error = Some(format!("XML decode error: {}", e));
            }
        }
    }
}
