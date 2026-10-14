//! FTP/SFTP Tab - Transfer HL7 files via FTP or SFTP (Client + Server)

use egui::{self, Color32, RichText};
use egui_extras::{Column, Size, StripBuilder, TableBuilder};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use vyna_hl7_core::CharacterSet;

/// Connection protocol
#[derive(Clone, Copy, PartialEq, Default)]
pub enum Protocol {
    #[default]
    Ftp,
    Sftp,
}

impl Protocol {
    fn as_str(&self) -> &'static str {
        match self {
            Protocol::Ftp => "FTP",
            Protocol::Sftp => "SFTP",
        }
    }

    fn default_port(&self) -> u16 {
        match self {
            Protocol::Ftp => 21,
            Protocol::Sftp => 22,
        }
    }
}

/// File entry from remote directory
#[derive(Clone, Default)]
#[allow(dead_code)] // modified field reserved for future UI display
struct FileEntry {
    name: String,
    size: u64,
    is_dir: bool,
    modified: Option<String>,
}

/// Log entry for transfer log
#[derive(Clone)]
struct LogEntry {
    timestamp: String,
    direction: Direction,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Upload,
    Download,
    Info,
    Error,
    Connection,
}

/// Client state
#[derive(Default)]
struct ClientState {
    connected: bool,
    connecting: bool,
    file_list: Vec<FileEntry>,
    current_path: String,
    error: Option<String>,
    transfer_in_progress: bool,
    transfer_filename: Option<String>,
    downloaded_content: Option<String>,
    log_entries: Vec<LogEntry>,
}

/// Server state
#[derive(Default)]
struct ServerState {
    running: bool,
    starting: bool,
    log_entries: Vec<LogEntry>,
    connection_count: usize,
    transfer_count: usize,
    error: Option<String>,
}

pub struct FtpTab {
    // Protocol selector
    protocol: Protocol,

    // Client settings
    client_host: String,
    client_port: String,
    client_username: String,
    client_password: String,
    client_path: String,
    upload_filename: String,
    upload_content: String,
    selected_file: Option<usize>,
    sftp_verify_host_key: bool,

    // Server settings
    server_port: String,
    server_bind: String,
    server_root: String,
    server_username: String,
    server_password: String,

    // State
    client_state: Arc<Mutex<ClientState>>,
    server_state: Arc<Mutex<ServerState>>,

    // Runtime
    runtime: Option<tokio::runtime::Runtime>,

    // Shutdown signals
    client_disconnect: Arc<AtomicBool>,
    server_shutdown: Arc<AtomicBool>,
}

impl Default for FtpTab {
    fn default() -> Self {
        Self {
            protocol: Protocol::Ftp,

            // Client defaults
            client_host: "127.0.0.1".to_string(),
            client_port: "21".to_string(),
            client_username: "anonymous".to_string(),
            client_password: String::new(),
            client_path: "/".to_string(),
            upload_filename: "message.hl7".to_string(),
            upload_content: String::new(),
            selected_file: None,
            sftp_verify_host_key: true,

            // Server defaults
            server_port: "2121".to_string(),
            server_bind: "127.0.0.1".to_string(),
            server_root: default_ftp_root().to_string_lossy().to_string(),
            server_username: "test".to_string(),
            server_password: "test".to_string(),

            // State
            client_state: Arc::new(Mutex::new(ClientState {
                current_path: "/".to_string(),
                ..Default::default()
            })),
            server_state: Arc::new(Mutex::new(ServerState::default())),

            // Runtime
            runtime: tokio::runtime::Runtime::new().ok(),

            // Signals
            client_disconnect: Arc::new(AtomicBool::new(false)),
            server_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl FtpTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("FTP/SFTP Client & Server");
        ui.label("Transfer HL7 files via FTP or SFTP protocols.");
        ui.add_space(5.0);

        // Protocol selector
        ui.horizontal(|ui| {
            ui.label("Protocol:");
            if ui
                .selectable_label(self.protocol == Protocol::Ftp, "FTP")
                .clicked()
            {
                self.protocol = Protocol::Ftp;
                self.client_port = "21".to_string();
                self.server_port = "2121".to_string();
            }
            if ui
                .selectable_label(self.protocol == Protocol::Sftp, "SFTP")
                .clicked()
            {
                self.protocol = Protocol::Sftp;
                self.client_port = "22".to_string();
                self.server_port = "2222".to_string();
            }
        });

        ui.add_space(5.0);

        let available_height = ui.available_height();

        StripBuilder::new(ui)
            .size(Size::relative(0.5).at_least(350.0))
            .size(Size::remainder().at_least(350.0))
            .horizontal(|mut strip| {
                // Left: Client
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());
                        self.client_ui(ui, ctx);
                    });
                });

                // Right: Server
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_height(panel_height);
                        ui.set_width(ui.available_width());
                        self.server_ui(ui, ctx);
                    });
                });
            });
    }

    fn client_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (connected, connecting, transfer_in_progress) = {
            let state = self.client_state.lock().unwrap();
            (
                state.connected,
                state.connecting,
                state.transfer_in_progress,
            )
        };

        ui.heading(format!("{} Client", self.protocol.as_str()));
        ui.add_space(5.0);

        // Connection settings
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.add_enabled(
                !connected && !connecting,
                egui::TextEdit::singleline(&mut self.client_host).desired_width(120.0),
            );
            ui.label("Port:");
            ui.add_enabled(
                !connected && !connecting,
                egui::TextEdit::singleline(&mut self.client_port).desired_width(50.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("User:");
            ui.add_enabled(
                !connected && !connecting,
                egui::TextEdit::singleline(&mut self.client_username).desired_width(80.0),
            );
            ui.label("Pass:");
            ui.add_enabled(
                !connected && !connecting,
                egui::TextEdit::singleline(&mut self.client_password)
                    .password(true)
                    .desired_width(80.0),
            );
        });

        if self.protocol == Protocol::Sftp {
            ui.horizontal(|ui| {
                ui.add_enabled(
                    !connected && !connecting,
                    egui::Checkbox::new(
                        &mut self.sftp_verify_host_key,
                        "Verify SSH host key via known_hosts (uncheck only for test servers)",
                    ),
                );
            });
        }

        ui.add_space(5.0);

        // Connect/Disconnect buttons
        ui.horizontal(|ui| {
            if connecting {
                ui.spinner();
                ui.label("Connecting...");
            } else if connected {
                if ui
                    .button(RichText::new("Disconnect").color(Color32::RED))
                    .clicked()
                {
                    self.disconnect_client(ctx.clone());
                }
                ui.colored_label(Color32::GREEN, "Connected");
            } else if ui
                .button(RichText::new("Connect").color(Color32::GREEN))
                .clicked()
            {
                self.connect_client(ctx.clone());
            }
        });

        ui.add_space(5.0);
        ui.separator();

        // File browser (only when connected)
        if connected {
            ui.add_space(5.0);
            ui.label(RichText::new("Remote Files").strong());

            // Path navigation
            ui.horizontal(|ui| {
                ui.label("Path:");
                ui.add_enabled(
                    !transfer_in_progress,
                    egui::TextEdit::singleline(&mut self.client_path).desired_width(150.0),
                );
                if ui
                    .add_enabled(!transfer_in_progress, egui::Button::new("Go"))
                    .clicked()
                {
                    self.list_directory(ctx.clone());
                }
                if ui
                    .add_enabled(!transfer_in_progress, egui::Button::new("↑"))
                    .on_hover_text("Parent directory")
                    .clicked()
                    && self.client_path != "/"
                {
                    if let Some(parent) = std::path::Path::new(&self.client_path).parent() {
                        self.client_path = parent.to_string_lossy().to_string();
                        if self.client_path.is_empty() {
                            self.client_path = "/".to_string();
                        }
                    }
                    self.list_directory(ctx.clone());
                }
            });

            // File list table
            let file_list = {
                let state = self.client_state.lock().unwrap();
                state.file_list.clone()
            };

            let table_height = 120.0;
            let table = TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto().at_least(20.0))
                .column(Column::remainder().at_least(100.0))
                .column(Column::auto().at_least(60.0))
                .min_scrolled_height(0.0)
                .max_scroll_height(table_height);

            table
                .header(18.0, |mut header| {
                    header.col(|_ui| {});
                    header.col(|ui| {
                        ui.strong("Name");
                    });
                    header.col(|ui| {
                        ui.strong("Size");
                    });
                })
                .body(|mut body| {
                    for (i, entry) in file_list.iter().enumerate() {
                        body.row(16.0, |mut row| {
                            row.col(|ui| {
                                let icon = if entry.is_dir { "📁" } else { "📄" };
                                ui.label(icon);
                            });
                            row.col(|ui| {
                                let is_selected = self.selected_file == Some(i);
                                if ui
                                    .selectable_label(is_selected, &entry.name)
                                    .double_clicked()
                                {
                                    if entry.is_dir {
                                        self.client_path = if self.client_path.ends_with('/') {
                                            format!("{}{}", self.client_path, entry.name)
                                        } else {
                                            format!("{}/{}", self.client_path, entry.name)
                                        };
                                        self.list_directory(ctx.clone());
                                    } else {
                                        self.download_file(&entry.name, ctx.clone());
                                    }
                                } else if ui
                                    .interact(ui.max_rect(), ui.id().with(i), egui::Sense::click())
                                    .clicked()
                                {
                                    self.selected_file = Some(i);
                                }
                            });
                            row.col(|ui| {
                                if !entry.is_dir {
                                    ui.label(format_size(entry.size));
                                }
                            });
                        });
                    }
                });

            // Action buttons
            ui.horizontal(|ui| {
                let has_file_selected = self
                    .selected_file
                    .is_some_and(|i| file_list.get(i).is_some_and(|e| !e.is_dir));

                if ui
                    .add_enabled(
                        has_file_selected && !transfer_in_progress,
                        egui::Button::new("Download"),
                    )
                    .clicked()
                    && let Some(i) = self.selected_file
                    && let Some(entry) = file_list.get(i)
                {
                    self.download_file(&entry.name, ctx.clone());
                }

                if transfer_in_progress {
                    ui.spinner();
                }
            });

            ui.add_space(5.0);
            ui.separator();

            // Upload section
            ui.add_space(5.0);
            ui.label(RichText::new("Upload File").strong());

            ui.horizontal(|ui| {
                ui.label("Filename:");
                ui.add(egui::TextEdit::singleline(&mut self.upload_filename).desired_width(120.0));
            });

            egui::ScrollArea::vertical()
                .id_salt("ftp_upload")
                .max_height(80.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut self.upload_content)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(4)
                            .code_editor(),
                    );
                });

            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !transfer_in_progress && !self.upload_content.is_empty(),
                        egui::Button::new("Upload"),
                    )
                    .clicked()
                {
                    self.upload_file(ctx.clone());
                }

                if ui.button("Load File...").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("HL7 Messages", &["hl7", "txt"])
                        .pick_file()
                    && let Ok(bytes) = std::fs::read(&path)
                    && let Ok(content) = CharacterSet::decode_message(&bytes)
                {
                    self.upload_content = content;
                    if let Some(filename) = path.file_name() {
                        self.upload_filename = filename.to_string_lossy().to_string();
                    }
                }
            });
        }

        // Downloaded content display
        {
            let state = self.client_state.lock().unwrap();
            if let Some(ref content) = state.downloaded_content {
                ui.add_space(5.0);
                ui.label(RichText::new("Downloaded:").strong());
                egui::ScrollArea::vertical()
                    .id_salt("ftp_downloaded")
                    .max_height(60.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut content.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    });
            }
        }

        // Error display
        {
            let state = self.client_state.lock().unwrap();
            if let Some(ref error) = state.error {
                ui.add_space(5.0);
                ui.colored_label(Color32::RED, format!("Error: {}", error));
            }
        }

        // Client log
        ui.add_space(5.0);
        ui.separator();
        self.client_log_ui(ui);
    }

    fn client_log_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Transfer Log").strong());
            if ui.small_button("Clear").clicked() {
                let mut state = self.client_state.lock().unwrap();
                state.log_entries.clear();
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("ftp_client_log")
            .stick_to_bottom(true)
            .max_height(80.0)
            .show(ui, |ui| {
                let state = self.client_state.lock().unwrap();
                for entry in &state.log_entries {
                    let (prefix, color) = match entry.direction {
                        Direction::Upload => ("UP  ", Color32::LIGHT_BLUE),
                        Direction::Download => ("DOWN", Color32::GREEN),
                        Direction::Info => ("INFO", Color32::YELLOW),
                        Direction::Error => ("ERR ", Color32::RED),
                        Direction::Connection => ("CONN", Color32::LIGHT_GREEN),
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(color, prefix);
                        ui.label(&entry.timestamp);
                        ui.label("-");
                        ui.label(&entry.message);
                    });
                }
            });
    }

    fn server_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let (running, starting) = {
            let state = self.server_state.lock().unwrap();
            (state.running, state.starting)
        };

        ui.heading(format!("{} Server", self.protocol.as_str()));
        if self.protocol == Protocol::Sftp {
            ui.colored_label(
                Color32::YELLOW,
                "SFTP server mode is not implemented, so Start is disabled. \
                 Use FTP server mode for file-transfer testing.",
            );
        }
        ui.add_space(5.0);

        // Server settings
        ui.horizontal(|ui| {
            ui.label("Listen Port:");
            ui.add_enabled(
                !running && !starting,
                egui::TextEdit::singleline(&mut self.server_port).desired_width(60.0),
            );
            ui.label("Bind:");
            ui.add_enabled(
                !running && !starting,
                egui::TextEdit::singleline(&mut self.server_bind).desired_width(90.0),
            )
            .on_hover_text("Bind address (127.0.0.1 = this machine only; 0.0.0.0 = LAN)");
        });

        ui.horizontal(|ui| {
            ui.label("Root Dir:");
            ui.add_enabled(
                !running && !starting,
                egui::TextEdit::singleline(&mut self.server_root).desired_width(150.0),
            );
            if ui
                .add_enabled(!running && !starting, egui::Button::new("Browse"))
                .clicked()
                && let Some(path) = rfd::FileDialog::new().pick_folder()
            {
                self.server_root = path.display().to_string();
            }
        });

        ui.horizontal(|ui| {
            ui.label("User:");
            ui.add_enabled(
                !running && !starting,
                egui::TextEdit::singleline(&mut self.server_username).desired_width(80.0),
            );
            ui.label("Pass:");
            ui.add_enabled(
                !running && !starting,
                egui::TextEdit::singleline(&mut self.server_password)
                    .password(true)
                    .desired_width(80.0),
            );
        });

        ui.add_space(10.0);

        // Start/Stop buttons
        ui.horizontal(|ui| {
            if starting {
                ui.spinner();
                ui.label("Starting...");
            } else if running {
                if ui
                    .button(RichText::new("Stop Server").color(Color32::RED))
                    .clicked()
                {
                    self.stop_server(ctx.clone());
                }
                ui.colored_label(Color32::GREEN, "Running");
            } else if ui
                .add_enabled(
                    self.protocol != Protocol::Sftp,
                    egui::Button::new(RichText::new("Start Server").color(Color32::GREEN)),
                )
                .clicked()
            {
                self.start_server(ctx.clone());
            }
        });

        // Server stats
        {
            let state = self.server_state.lock().unwrap();
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.label(format!("Connections: {}", state.connection_count));
                ui.label(format!("Transfers: {}", state.transfer_count));
            });

            if let Some(ref error) = state.error {
                ui.colored_label(Color32::RED, format!("Error: {}", error));
            }
        }

        ui.add_space(10.0);
        ui.separator();

        // Server log
        ui.horizontal(|ui| {
            ui.label(RichText::new("Server Log").strong());
            if ui.small_button("Clear").clicked() {
                let mut state = self.server_state.lock().unwrap();
                state.log_entries.clear();
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("ftp_server_log")
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let state = self.server_state.lock().unwrap();
                for entry in &state.log_entries {
                    let (prefix, color) = match entry.direction {
                        Direction::Upload => ("UP  ", Color32::LIGHT_BLUE),
                        Direction::Download => ("DOWN", Color32::GREEN),
                        Direction::Info => ("INFO", Color32::YELLOW),
                        Direction::Error => ("ERR ", Color32::RED),
                        Direction::Connection => ("CONN", Color32::LIGHT_GREEN),
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(color, prefix);
                        ui.label(&entry.timestamp);
                        ui.label("-");
                        ui.label(&entry.message);
                    });
                }
            });
    }

    // === Client Operations ===

    fn connect_client(&mut self, ctx: egui::Context) {
        let protocol = self.protocol;
        let host = self.client_host.clone();
        let port: u16 = self.client_port.parse().unwrap_or(protocol.default_port());
        let username = self.client_username.clone();
        let password = self.client_password.clone();
        let verify_host_key = self.sftp_verify_host_key;
        let path = self.client_path.clone();
        let state = self.client_state.clone();

        {
            let mut s = state.lock().unwrap();
            s.connecting = true;
            s.error = None;
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: format!(
                    "Connecting to {}:{} via {}...",
                    host,
                    port,
                    protocol.as_str()
                ),
            });
        }

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                let result = match protocol {
                    Protocol::Ftp => {
                        ftp_connect_and_list(&host, port, &username, &password, &path).await
                    }
                    Protocol::Sftp => {
                        sftp_connect_and_list(
                            &host,
                            port,
                            &username,
                            &password,
                            &path,
                            verify_host_key,
                        )
                        .await
                    }
                };

                match result {
                    Ok(files) => {
                        let mut s = state.lock().unwrap();
                        s.connected = true;
                        s.connecting = false;
                        s.file_list = files;
                        s.current_path = path;
                        s.log_entries.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Connection,
                            message: "Connected successfully".to_string(),
                        });
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.connected = false;
                        s.connecting = false;
                        s.error = Some(e.clone());
                        s.log_entries.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Error,
                            message: format!("Connection failed: {}", e),
                        });
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    fn disconnect_client(&mut self, ctx: egui::Context) {
        self.client_disconnect.store(true, Ordering::SeqCst);

        let mut state = self.client_state.lock().unwrap();
        state.connected = false;
        state.file_list.clear();
        state.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: "Disconnected".to_string(),
        });
        ctx.request_repaint();
    }

    fn list_directory(&mut self, ctx: egui::Context) {
        let protocol = self.protocol;
        let host = self.client_host.clone();
        let port: u16 = self.client_port.parse().unwrap_or(protocol.default_port());
        let username = self.client_username.clone();
        let password = self.client_password.clone();
        let verify_host_key = self.sftp_verify_host_key;
        let path = self.client_path.clone();
        let state = self.client_state.clone();

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                let result = match protocol {
                    Protocol::Ftp => {
                        ftp_connect_and_list(&host, port, &username, &password, &path).await
                    }
                    Protocol::Sftp => {
                        sftp_connect_and_list(
                            &host,
                            port,
                            &username,
                            &password,
                            &path,
                            verify_host_key,
                        )
                        .await
                    }
                };

                match result {
                    Ok(files) => {
                        let mut s = state.lock().unwrap();
                        s.file_list = files;
                        s.current_path = path;
                        s.error = None;
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.error = Some(e);
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    fn upload_file(&mut self, ctx: egui::Context) {
        let protocol = self.protocol;
        let host = self.client_host.clone();
        let port: u16 = self.client_port.parse().unwrap_or(protocol.default_port());
        let username = self.client_username.clone();
        let password = self.client_password.clone();
        let verify_host_key = self.sftp_verify_host_key;
        let remote_path = format!(
            "{}/{}",
            self.client_path.trim_end_matches('/'),
            self.upload_filename
        );
        let content = self.upload_content.clone();
        let filename = self.upload_filename.clone();
        let state = self.client_state.clone();
        let list_path = self.client_path.clone();

        {
            let mut s = state.lock().unwrap();
            s.transfer_in_progress = true;
            s.transfer_filename = Some(filename.clone());
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Upload,
                message: format!("Uploading {} ({} bytes)...", filename, content.len()),
            });
        }

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                let result = match protocol {
                    Protocol::Ftp => {
                        ftp_upload(&host, port, &username, &password, &remote_path, &content).await
                    }
                    Protocol::Sftp => {
                        sftp_upload(
                            &host,
                            port,
                            &username,
                            &password,
                            &remote_path,
                            &content,
                            verify_host_key,
                        )
                        .await
                    }
                };

                match result {
                    Ok(()) => {
                        // Refresh file list
                        let files = match protocol {
                            Protocol::Ftp => {
                                ftp_connect_and_list(&host, port, &username, &password, &list_path)
                                    .await
                            }
                            Protocol::Sftp => {
                                sftp_connect_and_list(
                                    &host,
                                    port,
                                    &username,
                                    &password,
                                    &list_path,
                                    verify_host_key,
                                )
                                .await
                            }
                        };

                        let mut s = state.lock().unwrap();
                        s.transfer_in_progress = false;
                        s.transfer_filename = None;
                        s.error = None;
                        if let Ok(f) = files {
                            s.file_list = f;
                        }
                        s.log_entries.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Upload,
                            message: "Upload complete".to_string(),
                        });
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.transfer_in_progress = false;
                        s.transfer_filename = None;
                        s.error = Some(e.clone());
                        s.log_entries.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Error,
                            message: format!("Upload failed: {}", e),
                        });
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    fn download_file(&mut self, filename: &str, ctx: egui::Context) {
        let protocol = self.protocol;
        let host = self.client_host.clone();
        let port: u16 = self.client_port.parse().unwrap_or(protocol.default_port());
        let username = self.client_username.clone();
        let password = self.client_password.clone();
        let verify_host_key = self.sftp_verify_host_key;
        let remote_path = format!("{}/{}", self.client_path.trim_end_matches('/'), filename);
        let filename = filename.to_string();
        let state = self.client_state.clone();

        {
            let mut s = state.lock().unwrap();
            s.transfer_in_progress = true;
            s.transfer_filename = Some(filename.clone());
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Download,
                message: format!("Downloading {}...", filename),
            });
        }

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                let result = match protocol {
                    Protocol::Ftp => {
                        ftp_download(&host, port, &username, &password, &remote_path).await
                    }
                    Protocol::Sftp => {
                        sftp_download(
                            &host,
                            port,
                            &username,
                            &password,
                            &remote_path,
                            verify_host_key,
                        )
                        .await
                    }
                };

                match result {
                    Ok(content) => {
                        let mut s = state.lock().unwrap();
                        s.transfer_in_progress = false;
                        s.transfer_filename = None;
                        s.downloaded_content = Some(content.clone());
                        s.error = None;
                        s.log_entries.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Download,
                            message: format!("Downloaded {} ({} bytes)", filename, content.len()),
                        });
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.transfer_in_progress = false;
                        s.transfer_filename = None;
                        s.error = Some(e.clone());
                        s.log_entries.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Error,
                            message: format!("Download failed: {}", e),
                        });
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    // === Server Operations ===

    fn start_server(&mut self, ctx: egui::Context) {
        // The SFTP server path is locked (see the SFTP server section
        // NOTE): refuse to start rather than run a server that can
        // never complete a transfer.
        if self.protocol == Protocol::Sftp {
            let mut s = self.server_state.lock().unwrap();
            s.starting = false;
            s.running = false;
            s.error = Some(SFTP_SERVER_UNSUPPORTED.to_string());
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Error,
                message: SFTP_SERVER_UNSUPPORTED.to_string(),
            });
            ctx.request_repaint();
            return;
        }
        let protocol = self.protocol;
        let port: u16 = self.server_port.parse().unwrap_or(protocol.default_port());
        let bind = self.server_bind.clone();
        let root = self.server_root.clone();
        let username = self.server_username.clone();
        let password = self.server_password.clone();
        let state = self.server_state.clone();
        let shutdown = self.server_shutdown.clone();

        shutdown.store(false, Ordering::SeqCst);

        {
            let mut s = state.lock().unwrap();
            s.starting = true;
            s.error = None;
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: format!("Starting {} server on port {}...", protocol.as_str(), port),
            });
        }

        if let Some(ref runtime) = self.runtime {
            let ctx_clone = ctx.clone();
            runtime.spawn(async move {
                let result = match protocol {
                    Protocol::Ftp => {
                        run_ftp_server(
                            port,
                            &bind,
                            &root,
                            &username,
                            &password,
                            state.clone(),
                            shutdown,
                            ctx_clone,
                        )
                        .await
                    }
                    Protocol::Sftp => {
                        run_sftp_server(
                            port,
                            &bind,
                            &root,
                            &username,
                            &password,
                            state.clone(),
                            shutdown,
                            ctx_clone,
                        )
                        .await
                    }
                };

                if let Err(e) = result {
                    let mut s = state.lock().unwrap();
                    s.running = false;
                    s.starting = false;
                    s.error = Some(e.clone());
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Error,
                        message: format!("Server error: {}", e),
                    });
                }
                ctx.request_repaint();
            });
        }
    }

    fn stop_server(&mut self, ctx: egui::Context) {
        self.server_shutdown.store(true, Ordering::SeqCst);

        let mut state = self.server_state.lock().unwrap();
        state.running = false;
        state.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: "Server stopped".to_string(),
        });
        ctx.request_repaint();
    }

    /// Set the message content (for file open support)
    pub fn set_message(&mut self, content: String) {
        self.upload_content = content;
    }

    /// Get the current message content (for file save support)
    pub fn get_message(&self) -> &str {
        &self.upload_content
    }
}

// === Helper Functions ===

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

// === FTP Client Implementation ===

async fn ftp_connect_and_list(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    path: &str,
) -> Result<Vec<FileEntry>, String> {
    use suppaftp::tokio::AsyncFtpStream;

    let addr = format!("{}:{}", host, port);

    let mut ftp = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    ftp.login(username, password)
        .await
        .map_err(|e| format!("Login failed: {}", e))?;

    ftp.cwd(path)
        .await
        .map_err(|e| format!("Failed to change directory: {}", e))?;

    let list = ftp
        .list(None)
        .await
        .map_err(|e| format!("Failed to list directory: {}", e))?;

    let _ = ftp.quit().await;

    Ok(parse_ftp_list(&list))
}

async fn ftp_upload(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    remote_path: &str,
    content: &str,
) -> Result<(), String> {
    use suppaftp::tokio::AsyncFtpStream;
    use tokio::io::AsyncWriteExt;

    let addr = format!("{}:{}", host, port);

    let mut ftp = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    ftp.login(username, password)
        .await
        .map_err(|e| format!("Login failed: {}", e))?;

    let mut stream = ftp
        .put_with_stream(remote_path)
        .await
        .map_err(|e| format!("Upload failed: {}", e))?;

    stream
        .write_all(content.as_bytes())
        .await
        .map_err(|e| format!("Write failed: {}", e))?;

    stream
        .finish()
        .await
        .map_err(|e| format!("Finalize failed: {}", e))?;

    let _ = ftp.quit().await;

    Ok(())
}

/// Maximum in-memory download size (10 MiB). Downloads past this
/// are rejected with a clear error instead of buffering unboundedly.
const MAX_DOWNLOAD_BYTES: u64 = 10 * 1024 * 1024;

/// Build the oversize-download error for `remote_path`.
fn download_too_large(remote_path: &str) -> String {
    format!(
        "Download of '{}' exceeds maximum size of {} bytes",
        remote_path, MAX_DOWNLOAD_BYTES
    )
}

async fn ftp_download(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    remote_path: &str,
) -> Result<String, String> {
    use suppaftp::tokio::AsyncFtpStream;
    use tokio::io::AsyncReadExt;

    let addr = format!("{}:{}", host, port);

    let mut ftp = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    ftp.login(username, password)
        .await
        .map_err(|e| format!("Login failed: {}", e))?;

    let mut stream = ftp
        .retr_as_stream(remote_path)
        .await
        .map_err(|e| format!("Download failed: {}", e))?;

    // Bound the in-memory buffer: read at most one byte past the
    // cap, then reject oversized files with a clear error.
    let mut buffer = Vec::new();
    (&mut stream)
        .take(MAX_DOWNLOAD_BYTES + 1)
        .read_to_end(&mut buffer)
        .await
        .map_err(|e| format!("Read failed: {}", e))?;
    if buffer.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(download_too_large(remote_path));
    }

    stream
        .finish()
        .await
        .map_err(|e| format!("Finalize failed: {}", e))?;

    let _ = ftp.quit().await;

    CharacterSet::decode_message(&buffer).map_err(|e| e.to_string())
}

fn parse_ftp_list(lines: &[String]) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 9 {
            let perms = parts[0];
            let is_dir = perms.starts_with('d');
            let size: u64 = parts[4].parse().unwrap_or(0);
            let name = parts[8..].join(" ");

            if name == "." || name == ".." {
                continue;
            }

            entries.push(FileEntry {
                name,
                size,
                is_dir,
                modified: Some(format!("{} {} {}", parts[5], parts[6], parts[7])),
            });
        } else if !line.is_empty() {
            entries.push(FileEntry {
                name: line.clone(),
                size: 0,
                is_dir: false,
                modified: None,
            });
        }
    }

    entries
}

// === FTP Server Authentication ===

use async_trait::async_trait;
use unftp_core::auth::{AuthenticationError, Authenticator, Credentials, Principal};

/// Simple username/password authenticator for the test panel FTP server
/// If username is empty, allows anonymous access
#[derive(Debug, Clone)]
struct SimpleAuthenticator {
    /// Expected username (empty = allow any)
    username: String,
    /// Expected password (only checked if username is set)
    password: String,
}

impl SimpleAuthenticator {
    fn new(username: String, password: String) -> Self {
        Self { username, password }
    }
}

#[async_trait]
impl Authenticator for SimpleAuthenticator {
    async fn authenticate(
        &self,
        username: &str,
        creds: &Credentials,
    ) -> Result<Principal, AuthenticationError> {
        // If no username is configured, allow anonymous access
        if self.username.is_empty() {
            return Ok(Principal {
                username: username.to_string(),
            });
        }

        // Check if username matches
        if username != self.username {
            return Err(AuthenticationError::BadUser);
        }

        // Check if password matches
        match &creds.password {
            Some(password) => {
                if password == &self.password {
                    Ok(Principal {
                        username: username.to_string(),
                    })
                } else {
                    Err(AuthenticationError::BadPassword)
                }
            }
            None => {
                // No password provided but authentication is required
                Err(AuthenticationError::BadPassword)
            }
        }
    }
}

// === FTP Server Implementation ===

/// Build the server listen address. Loopback by default; a LAN bind
/// (`0.0.0.0`) is an explicit opt-in in the Bind box. Blank input
/// fails closed to loopback.
fn server_listen_addr(bind: &str, port: u16) -> String {
    let bind = bind.trim();
    let bind = if bind.is_empty() { "127.0.0.1" } else { bind };
    format!("{}:{}", bind, port)
}

/// Dedicated FTP root under the OS temp dir (never the shared temp
/// dir itself), created on demand.
fn default_ftp_root() -> PathBuf {
    let dir = std::env::temp_dir().join("vynahl7-testpanel-ftp");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

// Private test-harness entry point; the flat arg list mirrors its caller.
#[allow(clippy::too_many_arguments)]
async fn run_ftp_server(
    port: u16,
    bind: &str,
    root: &str,
    username: &str,
    password: &str,
    state: Arc<Mutex<ServerState>>,
    shutdown: Arc<AtomicBool>,
    ctx: egui::Context,
) -> Result<(), String> {
    use libunftp::ServerBuilder;

    let addr = server_listen_addr(bind, port);

    // Mark server as running
    {
        let mut s = state.lock().unwrap();
        s.starting = false;
        s.running = true;
        let auth_msg = if username.is_empty() {
            "anonymous access enabled".to_string()
        } else {
            format!("authentication required (user: {})", username)
        };
        s.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: format!("FTP server started on {} ({})", addr, auth_msg),
        });
    }
    ctx.request_repaint();

    let root_path = PathBuf::from(root);
    if !root_path.is_dir() {
        return Err(format!(
            "FTP root is not a directory: {}",
            root_path.display()
        ));
    }

    // Build server with custom authenticator (supports both authenticated and anonymous modes)
    let authenticator = SimpleAuthenticator::new(username.to_string(), password.to_string());
    let server = ServerBuilder::with_authenticator(
        Box::new(move || {
            unftp_sbe_fs::Filesystem::new(root_path.clone())
                .expect("FTP root was verified to exist")
        }),
        std::sync::Arc::new(authenticator),
    )
    .greeting("Welcome to VynaHL7 Test Panel FTP Server")
    .passive_ports(50000..=50099)
    .build()
    .map_err(|e| format!("Failed to build server: {}", e))?;

    // Run server until shutdown
    tokio::select! {
        result = server.listen(&addr) => {
            if let Err(e) = result {
                return Err(format!("Server error: {}", e));
            }
        }
        _ = async {
            while !shutdown.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        } => {
            // Shutdown requested
        }
    }

    {
        let mut s = state.lock().unwrap();
        s.running = false;
    }

    Ok(())
}

// === SFTP Client Implementation (using ssh2) ===

/// Verify the server's host key against the user's known_hosts file.
///
/// Must run after the handshake and before any password is sent: without
/// it, a man-in-the-middle captures the typed password.
fn verify_sftp_host_key(session: &ssh2::Session, host: &str, port: u16) -> Result<(), String> {
    use ssh2::{CheckResult, KnownHostFileKind};

    let (key, _key_type) = session
        .host_key()
        .ok_or_else(|| "SSH server presented no host key".to_string())?;
    let mut known_hosts = session
        .known_hosts()
        .map_err(|e| format!("Failed to open known hosts: {e}"))?;
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Cannot locate home directory for known_hosts lookup".to_string())?;
    let path = format!("{home}/.ssh/known_hosts");
    known_hosts
        .read_file(std::path::Path::new(&path), KnownHostFileKind::OpenSSH)
        .map_err(|e| {
            format!("Failed to read {path}: {e} — add this server with ssh-keyscan first")
        })?;
    match known_hosts.check_port(host, port, key) {
        CheckResult::Match => Ok(()),
        CheckResult::Mismatch => Err(format!(
            "SSH host key MISMATCH for {host} — possible man-in-the-middle; refusing to send the password"
        )),
        CheckResult::NotFound => Err(format!(
            "SSH host key for {host} is not in {path} — verify it out-of-band, then add it with ssh-keyscan"
        )),
        CheckResult::Failure => Err("SSH host key check failed".to_string()),
    }
}

async fn sftp_connect_and_list(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    path: &str,
    verify_host_key: bool,
) -> Result<Vec<FileEntry>, String> {
    let host = host.to_string();
    let username = username.to_string();
    let password = password.to_string();
    let path = path.to_string();

    tokio::task::spawn_blocking(move || {
        sftp_connect_and_list_sync(&host, port, &username, &password, &path, verify_host_key)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Resolve an SFTP host (DNS name or IP literal) to socket addresses.
///
/// `SocketAddr::from_str` only accepts IP literals, so hostnames must
/// go through the resolver instead.
fn resolve_sftp_addrs(host: &str, port: u16) -> Result<Vec<std::net::SocketAddr>, String> {
    use std::net::ToSocketAddrs;

    let addrs: Vec<_> = format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|e| format!("Invalid address: {e}"))?
        .collect();
    if addrs.is_empty() {
        return Err(format!(
            "Invalid address: '{host}:{port}' resolved to nothing"
        ));
    }
    Ok(addrs)
}

/// Open a TCP connection to an SFTP host, trying every resolved
/// address in turn with a per-address timeout.
fn sftp_tcp_connect(host: &str, port: u16) -> Result<std::net::TcpStream, String> {
    use std::net::TcpStream;
    use std::time::Duration;

    let mut last_err = String::from("no addresses attempted");
    for addr in resolve_sftp_addrs(host, port)? {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(10)) {
            Ok(tcp) => return Ok(tcp),
            Err(e) => last_err = e.to_string(),
        }
    }
    Err(format!("Connection failed: {last_err}"))
}

fn sftp_connect_and_list_sync(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    path: &str,
    verify_host_key: bool,
) -> Result<Vec<FileEntry>, String> {
    use std::time::Duration;

    // Connect with timeout (host may be a DNS name or IP literal)
    let tcp = sftp_tcp_connect(host, port)?;

    tcp.set_read_timeout(Some(Duration::from_secs(30))).ok();
    tcp.set_write_timeout(Some(Duration::from_secs(30))).ok();

    // Create SSH session
    let mut session =
        ssh2::Session::new().map_err(|e| format!("Failed to create SSH session: {}", e))?;

    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("SSH handshake failed: {}", e))?;

    if verify_host_key {
        verify_sftp_host_key(&session, host, port)?;
    }

    // Authenticate with password
    session
        .userauth_password(username, password)
        .map_err(|e| format!("Authentication failed: {}", e))?;

    if !session.authenticated() {
        return Err("Authentication failed".to_string());
    }

    // Create SFTP channel
    let sftp = session
        .sftp()
        .map_err(|e| format!("Failed to create SFTP channel: {}", e))?;

    // List directory
    let entries = sftp
        .readdir(std::path::Path::new(path))
        .map_err(|e| format!("Failed to list directory '{}': {}", path, e))?;

    let mut result = Vec::new();
    for (entry_path, stat) in entries {
        let filename = entry_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        if filename == "." || filename == ".." {
            continue;
        }

        let is_dir = stat.is_dir();
        let size = stat.size.unwrap_or(0);
        let modified = stat.mtime.map(|t| {
            chrono::DateTime::from_timestamp(t as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default()
        });

        result.push(FileEntry {
            name: filename,
            size,
            is_dir,
            modified,
        });
    }

    // Sort: directories first, then by name
    result.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(result)
}

async fn sftp_upload(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    remote_path: &str,
    content: &str,
    verify_host_key: bool,
) -> Result<(), String> {
    let host = host.to_string();
    let username = username.to_string();
    let password = password.to_string();
    let remote_path = remote_path.to_string();
    let content = content.to_string();

    tokio::task::spawn_blocking(move || {
        sftp_upload_sync(
            &host,
            port,
            &username,
            &password,
            &remote_path,
            &content,
            verify_host_key,
        )
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

fn sftp_upload_sync(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    remote_path: &str,
    content: &str,
    verify_host_key: bool,
) -> Result<(), String> {
    use std::io::Write;

    let tcp = sftp_tcp_connect(host, port)?;

    let mut session =
        ssh2::Session::new().map_err(|e| format!("Failed to create SSH session: {}", e))?;

    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("SSH handshake failed: {}", e))?;

    if verify_host_key {
        verify_sftp_host_key(&session, host, port)?;
    }

    session
        .userauth_password(username, password)
        .map_err(|e| format!("Authentication failed: {}", e))?;

    if !session.authenticated() {
        return Err("Authentication failed".to_string());
    }

    let sftp = session
        .sftp()
        .map_err(|e| format!("Failed to create SFTP channel: {}", e))?;

    // Create file with write permissions
    let mut file = sftp
        .create(std::path::Path::new(remote_path))
        .map_err(|e| format!("Failed to create file '{}': {}", remote_path, e))?;

    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write to file: {}", e))?;

    Ok(())
}

async fn sftp_download(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    remote_path: &str,
    verify_host_key: bool,
) -> Result<String, String> {
    let host = host.to_string();
    let username = username.to_string();
    let password = password.to_string();
    let remote_path = remote_path.to_string();

    tokio::task::spawn_blocking(move || {
        sftp_download_sync(
            &host,
            port,
            &username,
            &password,
            &remote_path,
            verify_host_key,
        )
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

fn sftp_download_sync(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    remote_path: &str,
    verify_host_key: bool,
) -> Result<String, String> {
    use std::io::Read;

    let tcp = sftp_tcp_connect(host, port)?;

    let mut session =
        ssh2::Session::new().map_err(|e| format!("Failed to create SSH session: {}", e))?;

    session.set_tcp_stream(tcp);
    session
        .handshake()
        .map_err(|e| format!("SSH handshake failed: {}", e))?;

    if verify_host_key {
        verify_sftp_host_key(&session, host, port)?;
    }

    session
        .userauth_password(username, password)
        .map_err(|e| format!("Authentication failed: {}", e))?;

    if !session.authenticated() {
        return Err("Authentication failed".to_string());
    }

    let sftp = session
        .sftp()
        .map_err(|e| format!("Failed to create SFTP channel: {}", e))?;

    let mut file = sftp
        .open(std::path::Path::new(remote_path))
        .map_err(|e| format!("Failed to open file '{}': {}", remote_path, e))?;

    let mut content = Vec::new();
    (&mut file)
        .take(MAX_DOWNLOAD_BYTES + 1)
        .read_to_end(&mut content)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    if content.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(download_too_large(remote_path));
    }

    CharacterSet::decode_message(&content).map_err(|e| e.to_string())
}

// === SFTP Server Implementation (using russh) ===
//
// NOTE: the test-panel SFTP *server* is intentionally not implemented.
// An earlier revision accepted the `sftp` subsystem and then only logged
// channel bytes without speaking the SFTP packet protocol, so real SFTP
// clients hung waiting for replies while `transfer_count` miscounted raw
// SSH bytes as completed transfers. Rather than mislead, the server path
// is locked: `start_server` refuses SFTP mode, and `subsystem_request`
// rejects the subsystem so direct callers fail fast with a clear error.
// Use FTP server mode (or an external SFTP server with the SFTP client
// tab) for file-transfer testing instead.

/// User-facing explanation for the locked SFTP-server path.
const SFTP_SERVER_UNSUPPORTED: &str =
    "SFTP server mode is not implemented; use FTP server mode instead";

// Private test-harness entry point; the flat arg list mirrors its caller.
#[allow(clippy::too_many_arguments)]
async fn run_sftp_server(
    port: u16,
    bind: &str,
    root: &str,
    username: &str,
    password: &str,
    state: Arc<Mutex<ServerState>>,
    shutdown: Arc<AtomicBool>,
    ctx: egui::Context,
) -> Result<(), String> {
    use russh::keys::{Algorithm, PrivateKey};
    use russh::server::{Config, Server};
    use std::sync::Arc as StdArc;
    use std::time::Duration;

    // Generate a temporary Ed25519 key for the server
    let key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)
        .map_err(|e| format!("Failed to generate server key: {}", e))?;

    let config = Config {
        auth_rejection_time: Duration::from_secs(3),
        auth_rejection_time_initial: Some(Duration::from_secs(0)),
        keys: vec![key],
        ..Default::default()
    };

    let config = StdArc::new(config);

    // Create the server handler
    let mut server = SftpServerHandler {
        root: PathBuf::from(root),
        username: username.to_string(),
        password: password.to_string(),
        state: state.clone(),
        ctx: ctx.clone(),
    };

    let addr = server_listen_addr(bind, port);

    // Mark server as running
    {
        let mut s = state.lock().unwrap();
        s.starting = false;
        s.running = true;
        s.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: format!("SFTP server started on {}", addr),
        });
    }
    ctx.request_repaint();

    // Run server until shutdown
    tokio::select! {
        result = server.run_on_address(config, &addr) => {
            if let Err(e) = result {
                return Err(format!("Server error: {}", e));
            }
        }
        _ = async {
            while !shutdown.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        } => {
            // Shutdown requested
        }
    }

    {
        let mut s = state.lock().unwrap();
        s.running = false;
    }

    Ok(())
}

// SFTP Server Handler
#[derive(Clone)]
struct SftpServerHandler {
    root: PathBuf,
    username: String,
    password: String,
    state: Arc<Mutex<ServerState>>,
    ctx: egui::Context,
}

impl russh::server::Server for SftpServerHandler {
    type Handler = SftpSessionHandler;

    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> Self::Handler {
        {
            let mut s = self.state.lock().unwrap();
            s.connection_count += 1;
            let count = s.connection_count;
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Connection,
                message: format!("New connection (total: {})", count),
            });
        }
        self.ctx.request_repaint();

        SftpSessionHandler {
            root: self.root.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            state: self.state.clone(),
            ctx: self.ctx.clone(),
        }
    }

    fn handle_session_error(&mut self, error: <Self::Handler as russh::server::Handler>::Error) {
        let mut s = self.state.lock().unwrap();
        s.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Error,
            message: format!("Session error: {}", error),
        });
        self.ctx.request_repaint();
    }
}

// Session handler for each client connection
struct SftpSessionHandler {
    #[allow(dead_code)] // kept for a future SFTP implementation
    root: PathBuf,
    username: String,
    password: String,
    state: Arc<Mutex<ServerState>>,
    ctx: egui::Context,
}

impl russh::server::Handler for SftpSessionHandler {
    type Error = russh::Error;

    fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> impl std::future::Future<Output = Result<russh::server::Auth, Self::Error>> + Send {
        let is_valid = user == self.username && password == self.password;
        let user = user.to_string();
        let state = self.state.clone();
        let ctx = self.ctx.clone();

        async move {
            if is_valid {
                {
                    let mut s = state.lock().unwrap();
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Info,
                        message: format!("User '{}' authenticated", user),
                    });
                }
                ctx.request_repaint();
                Ok(russh::server::Auth::Accept)
            } else {
                {
                    let mut s = state.lock().unwrap();
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Error,
                        message: format!("Authentication failed for user '{}'", user),
                    });
                }
                ctx.request_repaint();
                Ok(russh::server::Auth::reject())
            }
        }
    }

    fn channel_open_session(
        &mut self,
        _channel: russh::Channel<russh::server::Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut russh::server::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let state = self.state.clone();
        let ctx = self.ctx.clone();

        async move {
            {
                let mut s = state.lock().unwrap();
                s.log_entries.push(LogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    direction: Direction::Info,
                    message: "Session channel opened".to_string(),
                });
            }
            ctx.request_repaint();
            reply.accept().await;
            Ok(())
        }
    }

    fn subsystem_request(
        &mut self,
        channel_id: russh::ChannelId,
        name: &str,
        session: &mut russh::server::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        // The server implements no subsystem (see the section NOTE), so
        // reject every request synchronously: clients fail fast instead
        // of hanging on a channel that can never answer.
        let name = name.to_string();
        let state = self.state.clone();
        let ctx = self.ctx.clone();
        let _ = session.channel_failure(channel_id);

        async move {
            {
                let mut s = state.lock().unwrap();
                s.log_entries.push(LogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    direction: Direction::Error,
                    message: format!("Subsystem '{}' rejected: SFTP server not implemented", name),
                });
            }
            ctx.request_repaint();
            Ok(())
        }
    }

    fn data(
        &mut self,
        channel_id: russh::ChannelId,
        data: &[u8],
        session: &mut russh::server::Session,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        let state = self.state.clone();
        let ctx = self.ctx.clone();
        let data_len = data.len();

        // No subsystem is ever accepted, so channel bytes can only arrive
        // on a half-torn-down channel. Log and drop them; they must never
        // count as transfers.
        record_unsupported_channel_data(&state, data_len);
        ctx.request_repaint();

        let _ = channel_id;
        let _ = session;

        async move { Ok(()) }
    }
}

/// Log discarded channel bytes without touching `transfer_count`.
///
/// Only completed file operations may count as transfers; the
/// not-implemented SFTP server performs none, so every byte it sees is
/// protocol overhead on a rejected channel.
fn record_unsupported_channel_data(state: &Mutex<ServerState>, data_len: usize) {
    let mut s = state.lock().unwrap();
    s.log_entries.push(LogEntry {
        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        direction: Direction::Info,
        message: format!(
            "Discarded {} channel bytes: SFTP server not implemented",
            data_len
        ),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Discover a free loopback port via a throwaway listener.
    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    /// Minimal fake FTP server serving one in-memory file for
    /// download tests (USER/PASS/PASV/RETR/QUIT only).
    async fn fake_ftp_download_server(payload: Vec<u8>) -> u16 {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (reader, mut writer) = stream.into_split();
            let mut lines = BufReader::new(reader).lines();
            let mut data_listener: Option<TcpListener> = None;
            writer.write_all(b"220 Fake FTP ready\r\n").await.unwrap();
            while let Some(line) = lines.next_line().await.unwrap_or(None) {
                let verb = line
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_uppercase();
                match verb.as_str() {
                    "USER" => writer.write_all(b"331 Password?\r\n").await.unwrap(),
                    "PASS" => writer.write_all(b"230 Logged in\r\n").await.unwrap(),
                    "PASV" => {
                        let data = TcpListener::bind("127.0.0.1:0").await.unwrap();
                        let dp = data.local_addr().unwrap().port();
                        data_listener = Some(data);
                        let reply = format!(
                            "227 Entering Passive Mode (127,0,0,1,{},{}).\r\n",
                            dp / 256,
                            dp % 256
                        );
                        writer.write_all(reply.as_bytes()).await.unwrap();
                    }
                    "EPSV" => {
                        let data = TcpListener::bind("127.0.0.1:0").await.unwrap();
                        let dp = data.local_addr().unwrap().port();
                        data_listener = Some(data);
                        let reply = format!("229 Entering Extended Passive Mode (|||{}|)\r\n", dp);
                        writer.write_all(reply.as_bytes()).await.unwrap();
                    }
                    "RETR" => {
                        writer.write_all(b"150 Opening data\r\n").await.unwrap();
                        if let Some(data) = data_listener.take() {
                            let (mut conn, _) = data.accept().await.unwrap();
                            conn.write_all(&payload).await.unwrap();
                            conn.shutdown().await.unwrap();
                        }
                        writer
                            .write_all(b"226 Transfer complete\r\n")
                            .await
                            .unwrap();
                    }
                    "QUIT" => {
                        writer.write_all(b"221 Bye\r\n").await.unwrap();
                        break;
                    }
                    _ => writer.write_all(b"200 OK\r\n").await.unwrap(),
                }
            }
        });
        port
    }

    #[test]
    fn sftp_resolver_accepts_hostnames() {
        let addrs = resolve_sftp_addrs("localhost", 22).unwrap();
        assert!(!addrs.is_empty());
        assert!(addrs.iter().all(|a| a.port() == 22));
    }

    #[test]
    fn sftp_resolver_rejects_unresolvable_names() {
        let err = resolve_sftp_addrs("nonexistent.invalid", 22).unwrap_err();
        assert!(err.contains("Invalid address"), "unexpected error: {err}");
    }

    #[test]
    fn oversize_error_names_file_and_limit() {
        let e = download_too_large("big.hl7");
        assert!(e.contains("big.hl7"), "unexpected error: {e}");
        assert!(e.contains("maximum"), "unexpected error: {e}");
        assert!(e.contains(&MAX_DOWNLOAD_BYTES.to_string()));
    }

    #[tokio::test]
    async fn ftp_download_rejects_oversized_file() {
        // 1 byte over the enforced download cap.
        let payload = vec![b'X'; (MAX_DOWNLOAD_BYTES + 1) as usize];
        let port = fake_ftp_download_server(payload).await;
        match ftp_download("127.0.0.1", port, "u", "p", "big.hl7").await {
            Err(e) => assert!(e.contains("maximum"), "unexpected error: {e}"),
            Ok(s) => panic!("oversized download accepted ({} bytes)", s.len()),
        }
    }

    #[tokio::test]
    async fn ftp_download_accepts_small_file() {
        let port = fake_ftp_download_server(b"MSH|^~\\&|S\rPID|1\r".to_vec()).await;
        let content = ftp_download("127.0.0.1", port, "u", "p", "small.hl7")
            .await
            .unwrap();
        assert_eq!(content, "MSH|^~\\&|S\rPID|1\r");
    }

    #[test]
    fn discarded_channel_data_does_not_count_as_transfers() {
        let state = Mutex::new(ServerState::default());
        record_unsupported_channel_data(&state, 64);
        record_unsupported_channel_data(&state, 128);
        let s = state.lock().unwrap();
        assert_eq!(s.transfer_count, 0);
        assert_eq!(s.log_entries.len(), 2);
        assert!(
            s.log_entries
                .iter()
                .all(|e| e.message.contains("not implemented"))
        );
    }

    #[test]
    fn start_server_refuses_sftp_mode() {
        let mut tab = FtpTab {
            protocol: Protocol::Sftp,
            server_port: free_port().to_string(),
            ..Default::default()
        };
        tab.start_server(egui::Context::default());
        let s = tab.server_state.lock().unwrap();
        assert!(!s.running);
        assert!(!s.starting);
        assert_eq!(s.error.as_deref(), Some(SFTP_SERVER_UNSUPPORTED));
    }

    /// The test-panel server does not implement the SFTP packet
    /// protocol, so an SFTP subsystem request must fail fast with a
    /// clear error instead of hanging the client, and raw channel
    /// bytes must never count as completed transfers.
    #[tokio::test]
    async fn sftp_subsystem_request_fails_fast_without_counting_transfer() {
        let port = free_port();
        let state = Arc::new(Mutex::new(ServerState::default()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let root = std::env::temp_dir().to_string_lossy().to_string();
        {
            let (state, shutdown) = (state.clone(), shutdown.clone());
            tokio::spawn(async move {
                let _ = run_sftp_server(
                    port,
                    "127.0.0.1",
                    &root,
                    "test",
                    "test",
                    state,
                    shutdown,
                    egui::Context::default(),
                )
                .await;
            });
        }
        // Wait for the listener to come up.
        for _ in 0..100 {
            if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        let started = Instant::now();
        // ssh2 is blocking; run the client off the async runtime.
        let outcome: Result<(), String> = tokio::task::spawn_blocking(move || {
            let tcp = std::net::TcpStream::connect(format!("127.0.0.1:{}", port))
                .map_err(|e| format!("connect: {}", e))?;
            let mut session = ssh2::Session::new().map_err(|e| format!("session: {}", e))?;
            session.set_tcp_stream(tcp);
            session.set_timeout(5000);
            session
                .handshake()
                .map_err(|e| format!("handshake: {}", e))?;
            session
                .userauth_password("test", "test")
                .map_err(|e| format!("auth: {}", e))?;
            match session.sftp() {
                Ok(_) => Err("unexpected SFTP session".to_string()),
                Err(e) => Err(format!("sftp rejected: {}", e)),
            }
        })
        .await
        .unwrap();
        let elapsed = started.elapsed();
        shutdown.store(true, Ordering::SeqCst);

        assert!(outcome.is_err(), "SFTP must not succeed: {:?}", outcome);
        let s = state.lock().unwrap();
        assert_eq!(
            s.transfer_count, 0,
            "raw bytes are not transfers (took {:?})",
            elapsed
        );
        drop(s);
        assert!(
            elapsed < Duration::from_secs(4),
            "SFTP setup must fail fast, not hang (took {:?})",
            elapsed
        );
        let s = state.lock().unwrap();
        assert!(
            s.log_entries.iter().any(|e| e.message.contains("SFTP")),
            "server must log the SFTP outcome"
        );
    }

    #[test]
    fn ftp_default_root_is_not_shared_temp_dir() {
        // One click must not share the whole OS temp dir: the default
        // root is a dedicated subdir (created on demand).
        let tab = FtpTab::default();
        let shared = std::env::temp_dir().to_string_lossy().to_string();
        assert_ne!(tab.server_root, shared);
        assert!(
            PathBuf::from(&tab.server_root).starts_with(std::env::temp_dir()),
            "default root should stay under the temp dir: {}",
            tab.server_root
        );
        assert!(
            PathBuf::from(&tab.server_root).is_dir(),
            "default root should exist: {}",
            tab.server_root
        );
    }

    #[test]
    fn ftp_server_defaults_to_loopback_bind() {
        let tab = FtpTab::default();
        assert_eq!(tab.server_bind, "127.0.0.1");
        assert_eq!(server_listen_addr(&tab.server_bind, 2121), "127.0.0.1:2121");
        // LAN bind stays available as an explicit opt-in.
        assert_eq!(server_listen_addr("0.0.0.0", 2121), "0.0.0.0:2121");
    }
}
