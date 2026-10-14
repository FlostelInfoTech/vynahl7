//! Main application state and UI rendering

use crate::logo;
use crate::tabs::{
    BuilderTab, FhirTab, FtpTab, HttpTab, MllpTab, ParserTab, TerserTab, ValidatorTab, XmlTab,
};
use eframe::egui::{self, Color32, RichText, Stroke};
use std::path::PathBuf;
use vyna_hl7_core::CharacterSet;

/// The active tab in the application
#[derive(Default, PartialEq, Clone, Copy)]
pub enum ActiveTab {
    #[default]
    Parser,
    Builder,
    Terser,
    Validator,
    Mllp,
    Http,
    Ftp,
    Fhir,
    Xml,
}

impl ActiveTab {
    fn label(&self) -> &str {
        match self {
            ActiveTab::Parser => "Parser",
            ActiveTab::Builder => "Builder",
            ActiveTab::Terser => "Terser",
            ActiveTab::Validator => "Validator",
            ActiveTab::Mllp => "MLLP",
            ActiveTab::Http => "HTTP",
            ActiveTab::Ftp => "FTP",
            ActiveTab::Fhir => "FHIR",
            ActiveTab::Xml => "XML",
        }
    }

    fn icon(&self) -> &str {
        match self {
            ActiveTab::Parser => "\u{1F50D}",   // Magnifying glass
            ActiveTab::Builder => "\u{1F3D7}",  // Construction
            ActiveTab::Terser => "\u{1F4CD}",   // Pin/locator
            ActiveTab::Validator => "\u{2705}", // Check mark
            ActiveTab::Mllp => "\u{1F4E1}",     // Antenna/network
            ActiveTab::Http => "\u{1F310}",     // Globe
            ActiveTab::Ftp => "\u{1F4C1}",      // Folder
            ActiveTab::Fhir => "\u{1F525}",     // Fire (FHIR)
            ActiveTab::Xml => "\u{1F4C4}",      // Document
        }
    }

    fn tooltip(&self) -> &str {
        match self {
            ActiveTab::Parser => "Parse and analyze HL7 messages (Ctrl+1)",
            ActiveTab::Builder => "Build HL7 messages visually (Ctrl+2)",
            ActiveTab::Terser => "Access fields using path notation (Ctrl+3)",
            ActiveTab::Validator => "Validate messages against schemas (Ctrl+4)",
            ActiveTab::Mllp => "Send/receive via MLLP protocol (Ctrl+5)",
            ActiveTab::Http => "Send/receive via HTTP protocol (Ctrl+6)",
            ActiveTab::Ftp => "Transfer files via FTP/SFTP (Ctrl+7)",
            ActiveTab::Fhir => "Convert to/from FHIR R4 (Ctrl+8)",
            ActiveTab::Xml => "Convert between ER7 and XML (Ctrl+9)",
        }
    }

    fn all() -> &'static [ActiveTab] {
        &[
            ActiveTab::Parser,
            ActiveTab::Builder,
            ActiveTab::Terser,
            ActiveTab::Validator,
            ActiveTab::Mllp,
            ActiveTab::Http,
            ActiveTab::Ftp,
            ActiveTab::Fhir,
            ActiveTab::Xml,
        ]
    }
}

/// Main application state
pub struct VynaHL7TestPanel {
    active_tab: ActiveTab,
    parser_tab: ParserTab,
    builder_tab: BuilderTab,
    terser_tab: TerserTab,
    validator_tab: ValidatorTab,
    mllp_tab: MllpTab,
    http_tab: HttpTab,
    ftp_tab: FtpTab,
    fhir_tab: FhirTab,
    xml_tab: XmlTab,
    show_about: bool,
    show_shortcuts: bool,
    current_file: Option<PathBuf>,
    status_message: Option<String>,
}

impl VynaHL7TestPanel {
    /// Create the test panel with every tab in its default state.
    ///
    /// The creation context is currently unused (fonts and visuals are
    /// configured by the caller); it is kept so future setup such as
    /// persisted state has a place to live.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            active_tab: ActiveTab::Parser,
            parser_tab: ParserTab::default(),
            builder_tab: BuilderTab::default(),
            terser_tab: TerserTab::default(),
            validator_tab: ValidatorTab::default(),
            mllp_tab: MllpTab::default(),
            http_tab: HttpTab::default(),
            ftp_tab: FtpTab::default(),
            fhir_tab: FhirTab::default(),
            xml_tab: XmlTab::default(),
            show_about: false,
            show_shortcuts: false,
            current_file: None,
            status_message: None,
        }
    }

    /// Check if Open is available for the current tab
    fn can_open(&self) -> bool {
        // Builder tab doesn't support opening files (it builds from scratch)
        self.active_tab != ActiveTab::Builder
    }

    /// Check if Save is available for the current tab
    fn can_save(&self) -> bool {
        match self.active_tab {
            ActiveTab::Builder => self.builder_tab.has_message(),
            _ => true,
        }
    }

    fn open_file(&mut self) {
        if !self.can_open() {
            return;
        }

        let file = rfd::FileDialog::new()
            .add_filter("HL7 Messages", &["hl7", "txt"])
            .add_filter("All Files", &["*"])
            .set_title("Open HL7 Message")
            .pick_file();

        if let Some(path) = file {
            match std::fs::read(&path) {
                Ok(bytes) => match CharacterSet::decode_message(&bytes) {
                    Ok(content) => {
                        // Set message in the current tab
                        match self.active_tab {
                            ActiveTab::Parser => self.parser_tab.set_message(content),
                            ActiveTab::Terser => self.terser_tab.set_message(content),
                            ActiveTab::Validator => self.validator_tab.set_message(content),
                            ActiveTab::Mllp => self.mllp_tab.set_message(content),
                            ActiveTab::Http => self.http_tab.set_message(content),
                            ActiveTab::Ftp => self.ftp_tab.set_message(content),
                            ActiveTab::Fhir => self.fhir_tab.set_message(content),
                            ActiveTab::Xml => self.xml_tab.set_message(content),
                            ActiveTab::Builder => {} // Builder doesn't support open
                        }
                        self.current_file = Some(path.clone());
                        let charset = CharacterSet::detect(&bytes).unwrap_or_default().msh18();
                        self.status_message =
                            Some(format!("Opened: {} [{}]", path.display(), charset));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Error decoding file: {}", e));
                    }
                },
                Err(e) => {
                    self.status_message = Some(format!("Error opening file: {}", e));
                }
            }
        }
    }

    fn save_file(&mut self) {
        if !self.can_save() {
            self.status_message = Some("No message to save. Build a message first.".to_string());
            return;
        }

        let file = rfd::FileDialog::new()
            .add_filter("HL7 Messages", &["hl7"])
            .add_filter("Text Files", &["txt"])
            .add_filter("All Files", &["*"])
            .set_title("Save HL7 Message")
            .set_file_name(
                self.current_file
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("message.hl7"),
            )
            .save_file();

        if let Some(path) = file {
            // Get content from the current tab and normalize line endings
            let content = match self.active_tab {
                ActiveTab::Parser => self.parser_tab.get_message(),
                ActiveTab::Builder => self.builder_tab.get_message(),
                ActiveTab::Terser => self.terser_tab.get_message(),
                ActiveTab::Validator => self.validator_tab.get_message(),
                ActiveTab::Mllp => self.mllp_tab.get_message(),
                ActiveTab::Http => self.http_tab.get_message(),
                ActiveTab::Ftp => self.ftp_tab.get_message(),
                ActiveTab::Fhir => self.fhir_tab.get_message(),
                ActiveTab::Xml => self.xml_tab.get_message(),
            };

            // Encode in the MSH-18 charset so a reopen decodes
            // the same bytes; unmappable text fails loudly instead
            // of saving mojibake.
            match encode_for_file_save(content) {
                Ok((bytes, charset)) => match std::fs::write(&path, &bytes) {
                    Ok(()) => {
                        self.current_file = Some(path.clone());
                        self.status_message =
                            Some(format!("Saved: {} [{}]", path.display(), charset));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Error saving file: {}", e));
                    }
                },
                Err(e) => {
                    self.status_message = Some(format!("Error saving file: {}", e));
                }
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let mut open_file = false;
        let mut save_file = false;

        ctx.input(|i| {
            if i.modifiers.ctrl {
                if i.key_pressed(egui::Key::Num1) {
                    self.active_tab = ActiveTab::Parser;
                } else if i.key_pressed(egui::Key::Num2) {
                    self.active_tab = ActiveTab::Builder;
                } else if i.key_pressed(egui::Key::Num3) {
                    self.active_tab = ActiveTab::Terser;
                } else if i.key_pressed(egui::Key::Num4) {
                    self.active_tab = ActiveTab::Validator;
                } else if i.key_pressed(egui::Key::Num5) {
                    self.active_tab = ActiveTab::Mllp;
                } else if i.key_pressed(egui::Key::Num6) {
                    self.active_tab = ActiveTab::Http;
                } else if i.key_pressed(egui::Key::Num7) {
                    self.active_tab = ActiveTab::Ftp;
                } else if i.key_pressed(egui::Key::Num8) {
                    self.active_tab = ActiveTab::Fhir;
                } else if i.key_pressed(egui::Key::Num9) {
                    self.active_tab = ActiveTab::Xml;
                } else if i.key_pressed(egui::Key::O) {
                    open_file = true;
                } else if i.key_pressed(egui::Key::S) {
                    save_file = true;
                }
            }
            if i.key_pressed(egui::Key::F1) {
                self.show_shortcuts = !self.show_shortcuts;
            }
        });

        // Handle file operations outside of input closure
        if open_file {
            self.open_file();
        }
        if save_file {
            self.save_file();
        }
    }
}

impl eframe::App for VynaHL7TestPanel {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Owned clone so panel closures can borrow it alongside `ui`.
        let ctx = ui.ctx().clone();

        // Handle keyboard shortcuts
        self.handle_keyboard_shortcuts(&ctx);

        // Top menu bar with branding
        egui::Panel::top("menu_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                // Logo/branding
                ui.add_space(4.0);
                ui.label(
                    RichText::new("VynaHL7")
                        .strong()
                        .size(16.0)
                        .color(Color32::from_rgb(59, 130, 246)),
                );
                ui.separator();

                let mut open_clicked = false;
                let mut save_clicked = false;
                let can_open = self.can_open();
                let can_save = self.can_save();

                ui.menu_button("File", |ui| {
                    ui.add_enabled_ui(can_open, |ui| {
                        if ui.button("\u{1F4C2} Open Message...").clicked() {
                            open_clicked = true;
                            ui.close();
                        }
                    });
                    ui.add_enabled_ui(can_save, |ui| {
                        if ui.button("\u{1F4BE} Save Message...").clicked() {
                            save_clicked = true;
                            ui.close();
                        }
                    });
                    ui.separator();
                    if ui.button("\u{274C} Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                // Handle file operations after menu closes
                if open_clicked {
                    self.open_file();
                }
                if save_clicked {
                    self.save_file();
                }

                ui.menu_button("View", |ui| {
                    for tab in ActiveTab::all() {
                        if ui
                            .button(format!("{} {}", tab.icon(), tab.label()))
                            .clicked()
                        {
                            self.active_tab = *tab;
                            ui.close();
                        }
                    }
                });

                ui.menu_button("Help", |ui| {
                    if ui.button("\u{2328} Keyboard Shortcuts (F1)").clicked() {
                        self.show_shortcuts = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("\u{2139} About VynaHL7 Test Panel").clicked() {
                        self.show_about = true;
                        ui.close();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).weak());
                    ui.separator();
                    egui::widgets::global_theme_preference_buttons(ui);
                });
            });
        });

        // Tab bar with icons
        egui::Panel::top("tab_bar").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                for tab in ActiveTab::all() {
                    let is_selected = self.active_tab == *tab;
                    let text = format!("{} {}", tab.icon(), tab.label());
                    let response =
                        ui.selectable_label(is_selected, RichText::new(&text).size(13.0));
                    if response.clicked() {
                        self.active_tab = *tab;
                    }
                    response.on_hover_text(tab.tooltip());
                }
            });
            ui.add_space(6.0);
            // Draw a subtle separator line
            let rect = ui.available_rect_before_wrap();
            ui.painter().hline(
                rect.x_range(),
                rect.bottom(),
                Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
            );
        });

        // Status bar with more info
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                // Project logo in status bar
                logo::show_logo(ui, 14.0);
                ui.label(RichText::new("VynaHL7").weak().small());
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        self.active_tab.icon(),
                        self.active_tab.label()
                    ))
                    .weak(),
                );
                ui.separator();
                if let Some(ref msg) = self.status_message {
                    ui.label(RichText::new(msg).weak());
                } else {
                    ui.label(RichText::new("Ready").weak());
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Press F1 for shortcuts").weak().italics());
                    ui.separator();
                    ui.label(RichText::new("HL7 v2.x Testing Suite").weak());
                });
            });
            ui.add_space(2.0);
        });

        // Main content area
        egui::CentralPanel::default().show(ui, |ui| match self.active_tab {
            ActiveTab::Parser => self.parser_tab.ui(ui),
            ActiveTab::Builder => self.builder_tab.ui(ui),
            ActiveTab::Terser => self.terser_tab.ui(ui),
            ActiveTab::Validator => self.validator_tab.ui(ui),
            ActiveTab::Mllp => self.mllp_tab.ui(ui, &ctx),
            ActiveTab::Http => self.http_tab.ui(ui, &ctx),
            ActiveTab::Ftp => self.ftp_tab.ui(ui, &ctx),
            ActiveTab::Fhir => self.fhir_tab.ui(ui),
            ActiveTab::Xml => self.xml_tab.ui(ui),
        });

        // About dialog
        if self.show_about {
            egui::Window::new("About VynaHL7 Test Panel")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .min_width(400.0)
                .show(&ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);

                        // Company logo from SVG
                        logo::show_logo(ui, 48.0);
                        logo::show_company_name(ui, 12.0);
                        ui.add_space(10.0);

                        ui.label(RichText::new("VynaHL7 Test Panel").heading().strong().size(24.0));
                        ui.label(RichText::new("HL7 v2.x Testing Suite").size(14.0).weak());
                        ui.add_space(15.0);

                        ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).strong());
                        ui.add_space(5.0);
                        ui.label("A comprehensive GUI application for testing");
                        ui.label("the VynaHL7 HL7 v2.x library capabilities.");
                        ui.add_space(15.0);

                        ui.separator();
                        ui.add_space(10.0);

                        ui.label(RichText::new("Features").strong());
                        ui.add_space(5.0);
                    });

                    egui::Grid::new("features_grid")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("\u{1F50D}");
                            ui.label("Parse and analyze HL7 messages");
                            ui.end_row();

                            ui.label("\u{1F3D7}");
                            ui.label("Build messages with visual builders");
                            ui.end_row();

                            ui.label("\u{1F4CD}");
                            ui.label("Access fields with Terser path notation");
                            ui.end_row();

                            ui.label("\u{2705}");
                            ui.label("Validate against HL7 schemas");
                            ui.end_row();

                            ui.label("\u{1F4E1}");
                            ui.label("Send/receive via MLLP protocol");
                            ui.end_row();

                            ui.label("\u{1F525}");
                            ui.label("Convert to/from FHIR R4");
                            ui.end_row();

                            ui.label("\u{1F4C4}");
                            ui.label("XML encoding/decoding");
                            ui.end_row();
                        });

                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Supported HL7 Versions").strong());
                        ui.label("v2.3, v2.3.1, v2.4, v2.5, v2.5.1, v2.6, v2.7, v2.7.1, v2.8, v2.8.1, v2.8.2");
                        ui.add_space(10.0);

                        ui.label(RichText::new("Message Types").weak());
                        ui.label(RichText::new("ADT, ORU, ORM, SIU, ACK, VXU, RDE, and more...").weak().italics());

                        ui.add_space(15.0);
                        if ui.button(RichText::new("  Close  ").size(14.0)).clicked() {
                            self.show_about = false;
                        }
                        ui.add_space(10.0);
                    });
                });
        }

        // Keyboard shortcuts dialog
        if self.show_shortcuts {
            egui::Window::new("\u{2328} Keyboard Shortcuts")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .min_width(350.0)
                .show(&ctx, |ui| {
                    ui.add_space(5.0);

                    egui::Grid::new("shortcuts_grid")
                        .num_columns(2)
                        .spacing([30.0, 8.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Navigation").strong().underline());
                            ui.label("");
                            ui.end_row();

                            for (i, tab) in ActiveTab::all().iter().enumerate() {
                                ui.label(format!("Ctrl+{}", i + 1));
                                ui.label(format!("{} {}", tab.icon(), tab.label()));
                                ui.end_row();
                            }

                            // Empty row for spacing (add_space not allowed in grid)
                            ui.label("");
                            ui.label("");
                            ui.end_row();

                            ui.label(RichText::new("File").strong().underline());
                            ui.label("");
                            ui.end_row();

                            ui.label("Ctrl+O");
                            ui.label("Open message file");
                            ui.end_row();

                            ui.label("Ctrl+S");
                            ui.label("Save message file");
                            ui.end_row();

                            // Empty row for spacing
                            ui.label("");
                            ui.label("");
                            ui.end_row();

                            ui.label(RichText::new("General").strong().underline());
                            ui.label("");
                            ui.end_row();

                            ui.label("F1");
                            ui.label("Toggle this dialog");
                            ui.end_row();

                            ui.label("Ctrl+Enter");
                            ui.label("Parse message (in Parser tab)");
                            ui.end_row();
                        });

                    ui.add_space(15.0);
                    ui.vertical_centered(|ui| {
                        if ui.button("  Close  ").clicked() {
                            self.show_shortcuts = false;
                        }
                    });
                    ui.add_space(5.0);
                });
        }
    }
}

/// Normalize line endings to CR and encode in the MSH-18 charset.
///
/// Mirrors the builder tab's `encode_for_save`: raw UTF-8 bytes are
/// written only when the message carries no usable MSH-18
/// declaration, and unmappable text is a loud error, never silent
/// replacement.
fn encode_for_file_save(content: &str) -> Result<(Vec<u8>, String), String> {
    let wire = content.replace("\r\n", "\r").replace('\n', "\r");
    match declared_charset(&wire) {
        Some(set) => set
            .encode(&wire)
            .map(|bytes| (bytes, set.msh18()))
            .map_err(|e| format!("Cannot encode message in {}: {}", set.msh18(), e)),
        None => Ok((wire.into_bytes(), CharacterSet::default().msh18())),
    }
}

/// MSH-18 charset declared by wire text, when present and usable.
fn declared_charset(wire: &str) -> Option<CharacterSet> {
    let msh = wire.split('\r').find(|line| line.starts_with("MSH|"))?;
    // MSH-1 is the separator itself, so MSH-18 sits at split index 17.
    let raw = msh.split('|').nth(17)?.trim();
    if raw.is_empty() {
        return None;
    }
    CharacterSet::from_msh18(raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_save_round_trips_latin1_charset() {
        let content = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5||||||8859/1\r\
             PID|1||1||Müller\r";
        let (bytes, charset) = encode_for_file_save(content).unwrap();
        assert_eq!(charset, "8859/1");
        assert!(bytes.contains(&0xFC)); // single-byte ü, not UTF-8
        let reopened = CharacterSet::decode_message(&bytes).unwrap();
        assert!(reopened.contains("Müller"));
        assert!(!reopened.contains("Ã¼"));
    }

    #[test]
    fn file_save_rejects_unmappable_characters() {
        let content = "MSH|^~\\&|S|F|R|D|20240101||ADT^A01|M1|P|2.5||||||ASCII\r\
             PID|1||1||Müller\r";
        let err = encode_for_file_save(content).unwrap_err();
        assert!(err.contains("ASCII"), "unexpected error: {err}");
    }

    #[test]
    fn file_save_defaults_to_utf8_without_msh18() {
        let (bytes, _) = encode_for_file_save("MSH|^~\\&|S\rPID|1||1||Müller\r").unwrap();
        assert_eq!(bytes, "MSH|^~\\&|S\rPID|1||1||Müller\r".as_bytes());
    }
}
