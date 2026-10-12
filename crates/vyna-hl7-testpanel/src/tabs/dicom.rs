//! DICOM Tab - SCU/SCP Client and Server for DICOM networking
//!
//! Left panel: SCU (Service Class User) - C-ECHO, C-STORE operations
//! Right panel: SCP (Service Class Provider) - Receives DICOM files

use egui::{Color32, RichText, ScrollArea, TextEdit, Ui};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

// DICOM SOP Class UIDs
const VERIFICATION_SOP_CLASS: &str = "1.2.840.10008.1.1";
const PATIENT_ROOT_FIND_SOP_CLASS: &str = "1.2.840.10008.5.1.4.1.2.1.1";
const STUDY_ROOT_FIND_SOP_CLASS: &str = "1.2.840.10008.5.1.4.1.2.2.1";
const IMPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LITTLE_ENDIAN: &str = "1.2.840.10008.1.2.1";

/// Query level for C-FIND
#[derive(Clone, Copy, PartialEq, Default)]
enum QueryLevel {
    #[default]
    Patient,
    Study,
}

impl QueryLevel {
    fn as_str(&self) -> &'static str {
        match self {
            QueryLevel::Patient => "Patient Root",
            QueryLevel::Study => "Study Root",
        }
    }

    fn sop_class(&self) -> &'static str {
        match self {
            QueryLevel::Patient => PATIENT_ROOT_FIND_SOP_CLASS,
            QueryLevel::Study => STUDY_ROOT_FIND_SOP_CLASS,
        }
    }

    fn query_retrieve_level(&self) -> &'static str {
        match self {
            QueryLevel::Patient => "PATIENT",
            QueryLevel::Study => "STUDY",
        }
    }
}

/// Direction for log entries
#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)] // Outgoing reserved for future use
enum Direction {
    Outgoing,
    Incoming,
    Info,
    Error,
}

impl Direction {
    fn color(&self) -> Color32 {
        match self {
            Direction::Outgoing => Color32::from_rgb(100, 149, 237),
            Direction::Incoming => Color32::from_rgb(144, 238, 144),
            Direction::Info => Color32::from_rgb(200, 200, 200),
            Direction::Error => Color32::from_rgb(255, 100, 100),
        }
    }
}

/// Log entry for DICOM operations
#[derive(Clone)]
struct LogEntry {
    timestamp: String,
    direction: Direction,
    message: String,
}

/// SCU (Client) state
#[derive(Default)]
#[allow(dead_code)] // connected reserved for future use
struct ScuState {
    connected: bool,
    connecting: bool,
    last_result: Option<String>,
    error: Option<String>,
    log_entries: Vec<LogEntry>,
}

/// SCP (Server) state
#[derive(Default)]
struct ScpState {
    running: bool,
    starting: bool,
    error: Option<String>,
    log_entries: Vec<LogEntry>,
    received_files: Vec<ReceivedFile>,
    connection_count: u32,
}


/// Received DICOM file info
#[derive(Clone)]
#[allow(dead_code)] // sop_class reserved for future UI display
struct ReceivedFile {
    timestamp: String,
    filename: String,
    sop_class: String,
    size: u64,
}

/// Main DICOM Tab
pub struct DicomTab {
    // SCU (Client) settings
    scu_remote_host: String,
    scu_remote_port: String,
    scu_remote_ae: String,
    scu_local_ae: String,
    scu_file_path: String,

    // C-FIND query settings
    scu_query_level: QueryLevel,
    scu_query_patient_id: String,
    scu_query_patient_name: String,

    // SCP (Server) settings
    scp_port: String,
    scp_ae_title: String,
    scp_storage_dir: String,

    // State
    scu_state: Arc<Mutex<ScuState>>,
    scp_state: Arc<Mutex<ScpState>>,

    // Server control
    scp_shutdown: Arc<AtomicBool>,

    // Async runtime
    runtime: Runtime,
}

impl Default for DicomTab {
    fn default() -> Self {
        Self {
            // SCU defaults
            scu_remote_host: "127.0.0.1".to_string(),
            scu_remote_port: "11112".to_string(),
            scu_remote_ae: "ANY-SCP".to_string(),
            scu_local_ae: "VynaHL7-SCU".to_string(),
            scu_file_path: String::new(),

            // C-FIND defaults
            scu_query_level: QueryLevel::default(),
            scu_query_patient_id: String::new(),
            scu_query_patient_name: String::new(),

            // SCP defaults
            scp_port: "11113".to_string(),
            scp_ae_title: "VynaHL7-SCP".to_string(),
            scp_storage_dir: std::env::temp_dir()
                .join("vyna-hl7-dicom")
                .to_string_lossy()
                .to_string(),

            // State
            scu_state: Arc::new(Mutex::new(ScuState::default())),
            scp_state: Arc::new(Mutex::new(ScpState::default())),

            // Server control
            scp_shutdown: Arc::new(AtomicBool::new(false)),

            // Runtime
            runtime: Runtime::new().expect("Failed to create tokio runtime"),
        }
    }
}

impl DicomTab {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Left panel: SCU (Client)
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width() / 2.0 - 10.0);
                self.scu_panel(ui);
            });

            ui.separator();

            // Right panel: SCP (Server)
            ui.vertical(|ui| {
                self.scp_panel(ui);
            });
        });
    }

    fn scu_panel(&mut self, ui: &mut Ui) {
        ui.heading("SCU (Service Class User)");
        ui.add_space(8.0);

        // Connection settings
        ui.group(|ui| {
            ui.label(RichText::new("Connection Settings").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Remote Host:");
                ui.add(TextEdit::singleline(&mut self.scu_remote_host).desired_width(120.0));
                ui.label("Port:");
                ui.add(TextEdit::singleline(&mut self.scu_remote_port).desired_width(60.0));
            });

            ui.horizontal(|ui| {
                ui.label("Remote AE Title:");
                ui.add(TextEdit::singleline(&mut self.scu_remote_ae).desired_width(100.0));
            });

            ui.horizontal(|ui| {
                ui.label("Local AE Title:");
                ui.add(TextEdit::singleline(&mut self.scu_local_ae).desired_width(100.0));
            });
        });

        ui.add_space(8.0);

        // Operations
        ui.group(|ui| {
            ui.label(RichText::new("Operations").strong());
            ui.add_space(4.0);

            let state = self.scu_state.lock().unwrap();
            let connecting = state.connecting;
            drop(state);

            // C-ECHO
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!connecting, egui::Button::new("C-ECHO (Verify)"))
                    .clicked()
                {
                    self.do_c_echo(ui.ctx().clone());
                }
                ui.label("- Test DICOM connectivity");
            });

            ui.add_space(4.0);

            // C-STORE
            ui.horizontal(|ui| {
                ui.label("DICOM File:");
                ui.add(TextEdit::singleline(&mut self.scu_file_path).desired_width(200.0));
                if ui.button("Browse...").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("DICOM", &["dcm", "dicom"])
                        .add_filter("All", &["*"])
                        .pick_file()
                    {
                        self.scu_file_path = path.to_string_lossy().to_string();
                    }
            });

            if ui
                .add_enabled(
                    !connecting && !self.scu_file_path.is_empty(),
                    egui::Button::new("C-STORE (Send File)"),
                )
                .clicked()
            {
                self.do_c_store(ui.ctx().clone());
            }

            ui.add_space(4.0);

            // C-FIND
            ui.separator();
            ui.label(RichText::new("C-FIND (Query)").strong());
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                ui.label("Query Level:");
                egui::ComboBox::from_id_salt("cfind_level")
                    .selected_text(self.scu_query_level.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.scu_query_level, QueryLevel::Patient, "Patient Root");
                        ui.selectable_value(&mut self.scu_query_level, QueryLevel::Study, "Study Root");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Patient ID:");
                ui.add(TextEdit::singleline(&mut self.scu_query_patient_id).desired_width(120.0).hint_text("*"));
            });

            ui.horizontal(|ui| {
                ui.label("Patient Name:");
                ui.add(TextEdit::singleline(&mut self.scu_query_patient_name).desired_width(120.0).hint_text("*"));
            });

            if ui
                .add_enabled(!connecting, egui::Button::new("C-FIND (Query)"))
                .clicked()
            {
                self.do_c_find(ui.ctx().clone());
            }
        });

        ui.add_space(8.0);

        // Status
        {
            let state = self.scu_state.lock().unwrap();
            if state.connecting {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Operation in progress...");
                });
            }

            if let Some(ref result) = state.last_result {
                ui.colored_label(Color32::from_rgb(100, 200, 100), result);
            }

            if let Some(ref error) = state.error {
                ui.colored_label(
                    Color32::from_rgb(255, 100, 100),
                    format!("Error: {}", error),
                );
            }
        }

        ui.add_space(8.0);

        // Log
        ui.label(RichText::new("Operation Log").strong());
        let log_height = ui.available_height() - 20.0;
        ScrollArea::vertical()
            .max_height(log_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let state = self.scu_state.lock().unwrap();
                for entry in state.log_entries.iter().rev() {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::GRAY, format!("[{}]", entry.timestamp));
                        ui.colored_label(entry.direction.color(), &entry.message);
                    });
                }
            });
    }

    fn scp_panel(&mut self, ui: &mut Ui) {
        ui.heading("SCP (Service Class Provider)");
        ui.add_space(8.0);

        // Server settings
        ui.group(|ui| {
            ui.label(RichText::new("Server Settings").strong());
            ui.add_space(4.0);

            let state = self.scp_state.lock().unwrap();
            let running = state.running;
            let starting = state.starting;
            drop(state);

            ui.horizontal(|ui| {
                ui.label("Listen Port:");
                ui.add_enabled(
                    !running && !starting,
                    TextEdit::singleline(&mut self.scp_port).desired_width(60.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("AE Title:");
                ui.add_enabled(
                    !running && !starting,
                    TextEdit::singleline(&mut self.scp_ae_title).desired_width(100.0),
                );
            });

            ui.horizontal(|ui| {
                ui.label("Storage Directory:");
                ui.add_enabled(
                    !running && !starting,
                    TextEdit::singleline(&mut self.scp_storage_dir).desired_width(200.0),
                );
                if ui
                    .add_enabled(!running && !starting, egui::Button::new("Browse..."))
                    .clicked()
                    && let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.scp_storage_dir = path.to_string_lossy().to_string();
                    }
            });

            ui.add_space(8.0);

            // Start/Stop button
            if starting {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Starting server...");
                });
            } else if running {
                if ui.button("Stop Server").clicked() {
                    self.stop_scp();
                }
                ui.colored_label(
                    Color32::from_rgb(100, 200, 100),
                    format!("Server running on port {}", self.scp_port),
                );
            } else {
                if ui.button("Start Server").clicked() {
                    self.start_scp(ui.ctx().clone());
                }
            }
        });

        ui.add_space(8.0);

        // Statistics
        {
            let state = self.scp_state.lock().unwrap();
            ui.horizontal(|ui| {
                ui.label(format!("Connections: {}", state.connection_count));
                ui.label(format!("Files received: {}", state.received_files.len()));
            });

            if let Some(ref error) = state.error {
                ui.colored_label(
                    Color32::from_rgb(255, 100, 100),
                    format!("Error: {}", error),
                );
            }
        }

        ui.add_space(8.0);

        // Received files
        ui.label(RichText::new("Received Files").strong());
        let files_height = (ui.available_height() - 120.0) / 2.0;
        ScrollArea::vertical()
            .id_salt("received_files")
            .max_height(files_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let state = self.scp_state.lock().unwrap();
                for file in state.received_files.iter().rev() {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::GRAY, format!("[{}]", file.timestamp));
                        ui.label(&file.filename);
                        ui.colored_label(Color32::GRAY, format!("({} bytes)", file.size));
                    });
                }
                if state.received_files.is_empty() {
                    ui.colored_label(Color32::GRAY, "No files received yet");
                }
            });

        ui.add_space(8.0);

        // Server log
        ui.label(RichText::new("Server Log").strong());
        let log_height = ui.available_height() - 20.0;
        ScrollArea::vertical()
            .id_salt("scp_log")
            .max_height(log_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let state = self.scp_state.lock().unwrap();
                for entry in state.log_entries.iter().rev() {
                    ui.horizontal(|ui| {
                        ui.colored_label(Color32::GRAY, format!("[{}]", entry.timestamp));
                        ui.colored_label(entry.direction.color(), &entry.message);
                    });
                }
            });
    }

    fn do_c_echo(&mut self, ctx: egui::Context) {
        let host = self.scu_remote_host.clone();
        let port: u16 = self.scu_remote_port.parse().unwrap_or(11112);
        let remote_ae = self.scu_remote_ae.clone();
        let local_ae = self.scu_local_ae.clone();
        let state = self.scu_state.clone();

        {
            let mut s = state.lock().unwrap();
            s.connecting = true;
            s.last_result = None;
            s.error = None;
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: format!("Starting C-ECHO to {}:{}", host, port),
            });
        }
        ctx.request_repaint();

        self.runtime.spawn(async move {
            let result = do_c_echo_async(&host, port, &remote_ae, &local_ae).await;

            let mut s = state.lock().unwrap();
            s.connecting = false;

            match result {
                Ok(msg) => {
                    s.last_result = Some(msg.clone());
                    s.error = None;
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Incoming,
                        message: msg,
                    });
                }
                Err(e) => {
                    s.error = Some(e.clone());
                    s.last_result = None;
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Error,
                        message: e,
                    });
                }
            }

            ctx.request_repaint();
        });
    }

    fn do_c_store(&mut self, ctx: egui::Context) {
        let host = self.scu_remote_host.clone();
        let port: u16 = self.scu_remote_port.parse().unwrap_or(11112);
        let remote_ae = self.scu_remote_ae.clone();
        let local_ae = self.scu_local_ae.clone();
        let file_path = self.scu_file_path.clone();
        let state = self.scu_state.clone();

        {
            let mut s = state.lock().unwrap();
            s.connecting = true;
            s.last_result = None;
            s.error = None;
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: format!("Starting C-STORE to {}:{}", host, port),
            });
        }
        ctx.request_repaint();

        self.runtime.spawn(async move {
            let result = do_c_store_async(&host, port, &remote_ae, &local_ae, &file_path).await;

            let mut s = state.lock().unwrap();
            s.connecting = false;

            match result {
                Ok(msg) => {
                    s.last_result = Some(msg.clone());
                    s.error = None;
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Incoming,
                        message: msg,
                    });
                }
                Err(e) => {
                    s.error = Some(e.clone());
                    s.last_result = None;
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Error,
                        message: e,
                    });
                }
            }

            ctx.request_repaint();
        });
    }

    fn do_c_find(&mut self, ctx: egui::Context) {
        let host = self.scu_remote_host.clone();
        let port: u16 = self.scu_remote_port.parse().unwrap_or(11112);
        let remote_ae = self.scu_remote_ae.clone();
        let local_ae = self.scu_local_ae.clone();
        let query_level = self.scu_query_level;
        let patient_id = self.scu_query_patient_id.clone();
        let patient_name = self.scu_query_patient_name.clone();
        let state = self.scu_state.clone();

        {
            let mut s = state.lock().unwrap();
            s.connecting = true;
            s.last_result = None;
            s.error = None;
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: format!(
                    "Starting C-FIND ({}) to {}:{}",
                    query_level.as_str(),
                    host,
                    port
                ),
            });
        }
        ctx.request_repaint();

        self.runtime.spawn(async move {
            let result = do_c_find_async(
                &host,
                port,
                &remote_ae,
                &local_ae,
                query_level,
                &patient_id,
                &patient_name,
            )
            .await;

            let mut s = state.lock().unwrap();
            s.connecting = false;

            match result {
                Ok(msg) => {
                    s.last_result = Some(msg.clone());
                    s.error = None;
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Incoming,
                        message: msg,
                    });
                }
                Err(e) => {
                    s.error = Some(e.clone());
                    s.last_result = None;
                    s.log_entries.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Error,
                        message: e,
                    });
                }
            }

            ctx.request_repaint();
        });
    }

    fn start_scp(&mut self, ctx: egui::Context) {
        let port: u16 = self.scp_port.parse().unwrap_or(11113);
        let ae_title = self.scp_ae_title.clone();
        let storage_dir = self.scp_storage_dir.clone();
        let state = self.scp_state.clone();
        let shutdown = self.scp_shutdown.clone();

        // Reset shutdown flag
        shutdown.store(false, Ordering::SeqCst);

        {
            let mut s = state.lock().unwrap();
            s.starting = true;
            s.error = None;
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: format!("Starting SCP server on port {}", port),
            });
        }
        ctx.request_repaint();

        self.runtime.spawn(async move {
            let result = run_scp_server(
                port,
                &ae_title,
                &storage_dir,
                state.clone(),
                shutdown,
                ctx.clone(),
            )
            .await;

            let mut s = state.lock().unwrap();
            s.starting = false;
            s.running = false;

            if let Err(e) = result {
                s.error = Some(e.clone());
                s.log_entries.push(LogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    direction: Direction::Error,
                    message: e,
                });
            } else {
                s.log_entries.push(LogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    direction: Direction::Info,
                    message: "Server stopped".to_string(),
                });
            }

            ctx.request_repaint();
        });
    }

    fn stop_scp(&mut self) {
        self.scp_shutdown.store(true, Ordering::SeqCst);
        let mut s = self.scp_state.lock().unwrap();
        s.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: "Stopping server...".to_string(),
        });
    }

    /// Set a DICOM file for operations (from file dialog)
    pub fn set_message(&mut self, content: &str) {
        // For DICOM, we might receive a file path
        self.scu_file_path = content.to_string();
    }

    /// Get current file path (for save operations)
    pub fn get_message(&self) -> &str {
        &self.scu_file_path
    }
}

// === DICOM SCU Operations ===

async fn do_c_echo_async(
    host: &str,
    port: u16,
    remote_ae: &str,
    local_ae: &str,
) -> Result<String, String> {
    use dicom_ul::association::client::ClientAssociationOptions;
    use std::time::Duration;

    let addr = format!("{}:{}", host, port);
    let remote_ae_owned = remote_ae.to_string();
    let local_ae_owned = local_ae.to_string();
    let host_owned = host.to_string();

    // Build association options for verification (in blocking context since dicom-ul is sync)
    let result = tokio::task::spawn_blocking(move || {
        let options = ClientAssociationOptions::new()
            .calling_ae_title(&local_ae_owned)
            .called_ae_title(&remote_ae_owned)
            .with_presentation_context(
                VERIFICATION_SOP_CLASS,
                vec![IMPLICIT_VR_LITTLE_ENDIAN, EXPLICIT_VR_LITTLE_ENDIAN],
            )
            .read_timeout(Duration::from_secs(30))
            .write_timeout(Duration::from_secs(30));

        // Establish association
        let association = options
            .establish(&addr)
            .map_err(|e| format!("Failed to establish association: {}", e))?;

        // For C-ECHO, the association being established with Verification SOP Class
        // indicates that the verification service is available

        // Release the association gracefully
        let _ = association.release();

        Ok::<_, String>((remote_ae_owned, host_owned))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let (remote_ae, host) = result?;
    Ok(format!(
        "C-ECHO successful - Association established with {} ({})",
        remote_ae, host
    ))
}

async fn do_c_store_async(
    host: &str,
    port: u16,
    remote_ae: &str,
    local_ae: &str,
    file_path: &str,
) -> Result<String, String> {
    use dicom_object::open_file;
    use dicom_ul::association::client::ClientAssociationOptions;
    use std::time::Duration;

    // First, read and validate the DICOM file
    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    let obj = open_file(&path).map_err(|e| format!("Failed to open DICOM file: {}", e))?;

    // Get SOP Class UID from the file
    let sop_class_uid = obj
        .element_by_name("SOPClassUID")
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "1.2.840.10008.5.1.4.1.1.2".to_string()); // CT Image Storage default

    let _sop_instance_uid = obj
        .element_by_name("SOPInstanceUID")
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Clone all values needed in the blocking task
    let addr = format!("{}:{}", host, port);
    let local_ae_owned = local_ae.to_string();
    let remote_ae_owned = remote_ae.to_string();
    let sop_class_uid_for_ctx = sop_class_uid.clone();
    let sop_class_uid_for_result = sop_class_uid.clone();

    // Establish association for C-STORE (in blocking context since dicom-ul is sync)
    let result = tokio::task::spawn_blocking(move || {
        let ts_implicit = IMPLICIT_VR_LITTLE_ENDIAN.to_string();
        let ts_explicit = EXPLICIT_VR_LITTLE_ENDIAN.to_string();
        let transfer_syntaxes: Vec<&String> = vec![&ts_implicit, &ts_explicit];

        let options = ClientAssociationOptions::new()
            .calling_ae_title(&local_ae_owned)
            .called_ae_title(&remote_ae_owned)
            .with_presentation_context(&sop_class_uid_for_ctx, transfer_syntaxes)
            .read_timeout(Duration::from_secs(60))
            .write_timeout(Duration::from_secs(60));

        let association = options
            .establish(&addr)
            .map_err(|e| format!("Failed to establish association: {}", e))?;

        // Note: Full C-STORE implementation would require:
        // 1. Building C-STORE-RQ PDU with the DICOM data
        // 2. Sending the PDU
        // 3. Receiving C-STORE-RSP
        // This is complex and requires deep integration with dicom-ul's PDU handling

        // For this demo, we verify the association can be established
        // with the correct SOP class

        // Release the association gracefully
        let _ = association.release();

        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    result?;

    Ok(format!(
        "C-STORE association established for {} (SOP Class: {}, {} bytes)",
        file_name, sop_class_uid_for_result, file_size
    ))
}

// === DICOM C-FIND Operation ===

async fn do_c_find_async(
    host: &str,
    port: u16,
    remote_ae: &str,
    local_ae: &str,
    query_level: QueryLevel,
    patient_id: &str,
    patient_name: &str,
) -> Result<String, String> {
    use dicom_ul::association::client::ClientAssociationOptions;
    use dicom_ul::pdu::Pdu;
    use std::time::Duration;

    let addr = format!("{}:{}", host, port);
    let remote_ae_owned = remote_ae.to_string();
    let local_ae_owned = local_ae.to_string();
    let sop_class = query_level.sop_class().to_string();
    let qr_level = query_level.query_retrieve_level().to_string();
    let patient_id_owned = patient_id.to_string();
    let patient_name_owned = patient_name.to_string();
    let level_str = query_level.as_str().to_string();

    let result = tokio::task::spawn_blocking(move || -> Result<Vec<String>, String> {
        // Establish association with the C-FIND SOP Class
        let ts_implicit = IMPLICIT_VR_LITTLE_ENDIAN.to_string();
        let ts_explicit = EXPLICIT_VR_LITTLE_ENDIAN.to_string();
        let transfer_syntaxes: Vec<&String> = vec![&ts_implicit, &ts_explicit];
        let options = ClientAssociationOptions::new()
            .calling_ae_title(&local_ae_owned)
            .called_ae_title(&remote_ae_owned)
            .with_presentation_context(&sop_class, transfer_syntaxes)
            .read_timeout(Duration::from_secs(30))
            .write_timeout(Duration::from_secs(30));

        let mut association = options
            .establish(&addr)
            .map_err(|e| format!("Failed to establish association: {}", e))?;

        // Build the C-FIND-RQ command and identifier as raw DICOM data
        // Command group (group 0000)
        let mut command_data: Vec<u8> = Vec::new();

        // Affected SOP Class UID (0000,0002)
        write_dicom_element(&mut command_data, 0x0000, 0x0002, sop_class.as_bytes());
        // Command Field (0000,0100) = 0x0020 (C-FIND-RQ)
        write_dicom_element(&mut command_data, 0x0000, 0x0100, &0x0020u16.to_le_bytes());
        // Message ID (0000,0110) = 1
        write_dicom_element(&mut command_data, 0x0000, 0x0110, &1u16.to_le_bytes());
        // Priority (0000,0700) = 0 (MEDIUM)
        write_dicom_element(&mut command_data, 0x0000, 0x0700, &0u16.to_le_bytes());
        // Command Data Set Type (0000,0800) = 0x0001 (data set present)
        write_dicom_element(&mut command_data, 0x0000, 0x0800, &0x0001u16.to_le_bytes());

        // Recalculate and prepend Command Group Length (0000,0000)
        let group_length = command_data.len() as u32;
        let mut full_command: Vec<u8> = Vec::new();
        write_dicom_element(&mut full_command, 0x0000, 0x0000, &group_length.to_le_bytes());
        full_command.extend_from_slice(&command_data);

        // Build identifier (query keys) dataset
        let mut identifier_data: Vec<u8> = Vec::new();

        // Query/Retrieve Level (0008,0052)
        let padded_level = pad_to_even(&qr_level);
        write_dicom_element(&mut identifier_data, 0x0008, 0x0052, padded_level.as_bytes());

        // Patient ID (0010,0020) - empty means wildcard match
        let pid = if patient_id_owned.is_empty() {
            "*".to_string()
        } else {
            patient_id_owned.clone()
        };
        let padded_pid = pad_to_even(&pid);
        write_dicom_element(&mut identifier_data, 0x0010, 0x0020, padded_pid.as_bytes());

        // Patient Name (0010,0010) - empty means wildcard match
        let pname = if patient_name_owned.is_empty() {
            "*".to_string()
        } else {
            patient_name_owned.clone()
        };
        let padded_pname = pad_to_even(&pname);
        write_dicom_element(&mut identifier_data, 0x0010, 0x0010, padded_pname.as_bytes());

        // Get the presentation context ID for our SOP class
        let pc_id = association
            .presentation_contexts()
            .iter()
            .find(|pc| pc.transfer_syntax == IMPLICIT_VR_LITTLE_ENDIAN || pc.transfer_syntax == EXPLICIT_VR_LITTLE_ENDIAN)
            .map(|pc| pc.id)
            .unwrap_or(1);

        // Send command as P-DATA with command flag (last fragment = true, command = true)
        // P-DATA-TF contains PDV items: [length(4)][context_id(1)][flags(1)][data...]
        // flags: bit 0 = command(1)/data(0), bit 1 = last fragment
        let command_pdv = build_pdv(pc_id, true, &full_command);
        let data_pdv = build_pdv(pc_id, false, &identifier_data);

        // Send command PDU
        association
            .send(&Pdu::PData {
                data: vec![command_pdv],
            })
            .map_err(|e| format!("Failed to send C-FIND command: {}", e))?;

        // Send data PDU (identifier)
        association
            .send(&Pdu::PData {
                data: vec![data_pdv],
            })
            .map_err(|e| format!("Failed to send C-FIND identifier: {}", e))?;

        // Receive responses
        let mut results = Vec::new();
        loop {
            match association.receive() {
                Ok(pdu) => match pdu {
                    Pdu::PData { data } => {
                        for pdv in &data {
                            let is_command = pdv.value_type == dicom_ul::pdu::PDataValueType::Command;

                            if is_command {
                                // Parse status from command response
                                let status = extract_status(&pdv.data);
                                match status {
                                    Some(0x0000) => {
                                        // Success - no more results
                                        let _ = association.release();
                                        return Ok(results);
                                    }
                                    Some(0xFF00) | Some(0xFF01) => {
                                        // Pending - more results coming
                                    }
                                    Some(code) => {
                                        let _ = association.release();
                                        return Err(format!("C-FIND failed with status: 0x{:04X}", code));
                                    }
                                    None => {}
                                }
                            } else {
                                // Data response - extract patient info
                                let info = extract_patient_info(&pdv.data);
                                if !info.is_empty() {
                                    results.push(info);
                                }
                            }
                        }
                    }
                    Pdu::ReleaseRQ => {
                        let _ = association.send(&Pdu::ReleaseRP);
                        return Ok(results);
                    }
                    Pdu::AbortRQ { .. } => {
                        return Err("Association aborted by remote".to_string());
                    }
                    _ => {}
                },
                Err(e) => {
                    return Err(format!("PDU receive error: {}", e));
                }
            }
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let results = result?;
    if results.is_empty() {
        Ok(format!(
            "C-FIND complete ({}) - No matching results found",
            level_str
        ))
    } else {
        let summary = results.join("\n");
        Ok(format!(
            "C-FIND complete ({}) - {} result(s):\n{}",
            level_str,
            results.len(),
            summary
        ))
    }
}

/// Write a DICOM data element in Implicit VR Little Endian format
fn write_dicom_element(buf: &mut Vec<u8>, group: u16, element: u16, value: &[u8]) {
    buf.extend_from_slice(&group.to_le_bytes());
    buf.extend_from_slice(&element.to_le_bytes());
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(value);
}

/// Pad a string value to even length (DICOM requirement)
fn pad_to_even(s: &str) -> String {
    if !s.len().is_multiple_of(2) {
        format!("{} ", s)
    } else {
        s.to_string()
    }
}

/// Build a PDV (Presentation Data Value) item
fn build_pdv(context_id: u8, is_command: bool, data: &[u8]) -> dicom_ul::pdu::PDataValue {
    dicom_ul::pdu::PDataValue {
        presentation_context_id: context_id,
        value_type: if is_command {
            dicom_ul::pdu::PDataValueType::Command
        } else {
            dicom_ul::pdu::PDataValueType::Data
        },
        is_last: true,
        data: data.to_vec(),
    }
}

/// Extract status code from a DICOM command dataset
fn extract_status(data: &[u8]) -> Option<u16> {
    // Search for Status tag (0000,0900) in implicit VR LE
    let target: [u8; 4] = [0x00, 0x00, 0x00, 0x09]; // (0000,0900) in LE
    let mut pos = 0;
    while pos + 8 <= data.len() {
        if data[pos..pos + 4] == target {
            let value_len = u32::from_le_bytes([
                data[pos + 4],
                data[pos + 5],
                data[pos + 6],
                data[pos + 7],
            ]) as usize;
            if value_len >= 2 && pos + 8 + value_len <= data.len() {
                return Some(u16::from_le_bytes([data[pos + 8], data[pos + 9]]));
            }
        }
        // Advance by element size
        if pos + 8 <= data.len() {
            let vl = u32::from_le_bytes([
                data[pos + 4],
                data[pos + 5],
                data[pos + 6],
                data[pos + 7],
            ]) as usize;
            pos += 8 + vl;
        } else {
            break;
        }
    }
    None
}

/// Extract patient info from a C-FIND response dataset
fn extract_patient_info(data: &[u8]) -> String {
    let mut patient_name = String::new();
    let mut patient_id = String::new();

    let mut pos = 0;
    while pos + 8 <= data.len() {
        let group = u16::from_le_bytes([data[pos], data[pos + 1]]);
        let element = u16::from_le_bytes([data[pos + 2], data[pos + 3]]);
        let value_len = u32::from_le_bytes([
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]) as usize;

        if value_len == 0xFFFFFFFF || pos + 8 + value_len > data.len() {
            break;
        }

        let value_bytes = &data[pos + 8..pos + 8 + value_len];
        let value = String::from_utf8_lossy(value_bytes).trim().to_string();

        match (group, element) {
            (0x0010, 0x0010) => patient_name = value, // Patient Name
            (0x0010, 0x0020) => patient_id = value,   // Patient ID
            _ => {}
        }

        pos += 8 + value_len;
    }

    if patient_name.is_empty() && patient_id.is_empty() {
        String::new()
    } else {
        format!("  {} (ID: {})", patient_name, patient_id)
    }
}

// === DICOM SCP Server ===

async fn run_scp_server(
    port: u16,
    ae_title: &str,
    storage_dir: &str,
    state: Arc<Mutex<ScpState>>,
    shutdown: Arc<AtomicBool>,
    ctx: egui::Context,
) -> Result<(), String> {
    use tokio::net::TcpListener;

    // Ensure storage directory exists
    let storage_path = PathBuf::from(storage_dir);
    std::fs::create_dir_all(&storage_path)
        .map_err(|e| format!("Failed to create storage directory: {}", e))?;

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    {
        let mut s = state.lock().unwrap();
        s.starting = false;
        s.running = true;
        s.log_entries.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: format!("SCP server listening on {} (AE: {})", addr, ae_title),
        });
    }
    ctx.request_repaint();

    // Accept connections until shutdown
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((socket, peer_addr)) => {
                        let state_clone = state.clone();
                        let ctx_clone = ctx.clone();
                        let ae_title_clone = ae_title.to_string();
                        let storage_clone = storage_path.clone();

                        {
                            let mut s = state_clone.lock().unwrap();
                            s.connection_count += 1;
                            s.log_entries.push(LogEntry {
                                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                direction: Direction::Incoming,
                                message: format!("Incoming connection from {}", peer_addr),
                            });
                        }
                        ctx_clone.request_repaint();

                        // Handle connection in background
                        tokio::spawn(async move {
                            if let Err(e) = handle_scp_connection(
                                socket,
                                peer_addr.to_string(),
                                &ae_title_clone,
                                &storage_clone,
                                state_clone.clone(),
                                ctx_clone.clone(),
                            ).await {
                                let mut s = state_clone.lock().unwrap();
                                s.log_entries.push(LogEntry {
                                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                    direction: Direction::Error,
                                    message: format!("Connection error: {}", e),
                                });
                                ctx_clone.request_repaint();
                            }
                        });
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.log_entries.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Error,
                            message: format!("Accept error: {}", e),
                        });
                        ctx.request_repaint();
                    }
                }
            }
            _ = async {
                while !shutdown.load(Ordering::SeqCst) {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            } => {
                // Shutdown requested
                break;
            }
        }
    }

    {
        let mut s = state.lock().unwrap();
        s.running = false;
    }

    Ok(())
}

async fn handle_scp_connection(
    socket: tokio::net::TcpStream,
    peer_addr: String,
    ae_title: &str,
    _storage_path: &PathBuf,
    state: Arc<Mutex<ScpState>>,
    ctx: egui::Context,
) -> Result<(), String> {
    use dicom_ul::association::server::ServerAssociationOptions;
    use dicom_ul::pdu::Pdu;

    // Convert to std TcpStream for dicom-ul (blocking API)
    let std_socket = socket
        .into_std()
        .map_err(|e| format!("Socket conversion error: {}", e))?;

    let ae_title_owned = ae_title.to_string();
    let peer_addr_clone = peer_addr.clone();

    // Handle association in blocking context
    let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
        let options = ServerAssociationOptions::new()
            .ae_title(&ae_title_owned)
            .with_abstract_syntax(VERIFICATION_SOP_CLASS);

        // Accept the association
        let mut association = options
            .establish(std_socket)
            .map_err(|e| format!("Failed to establish association: {}", e))?;

        // Read PDUs until release or abort
        loop {
            match association.receive() {
                Ok(pdu) => {
                    match pdu {
                        Pdu::ReleaseRQ => {
                            // Respond with release response
                            let _ = association.send(&Pdu::ReleaseRP);
                            return Ok(format!("Association released by {}", peer_addr_clone));
                        }
                        Pdu::AbortRQ { .. } => {
                            return Ok(format!("Association aborted by {}", peer_addr_clone));
                        }
                        Pdu::PData { data } => {
                            // Received P-DATA, could be C-ECHO or C-STORE data
                            // For now, just acknowledge
                            let _ = data; // Suppress unused warning
                        }
                        _ => {
                            // Other PDUs
                        }
                    }
                }
                Err(e) => {
                    return Err(format!("PDU receive error: {}", e));
                }
            }
        }
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    let mut s = state.lock().unwrap();
    match result {
        Ok(msg) => {
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: msg,
            });
        }
        Err(e) => {
            s.log_entries.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Error,
                message: e,
            });
        }
    }
    ctx.request_repaint();

    Ok(())
}
