//! MLLP Tab - Send and receive messages via MLLP protocol

use egui::{self, RichText, Color32};
use egui_extras::{StripBuilder, Size};
use vyna_hl7_core::CharacterSet;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use crate::samples;

/// State shared between UI and async tasks
#[derive(Default)]
struct MllpState {
    server_running: bool,
    server_messages: Vec<LogEntry>,
    client_response: Option<String>,
    client_error: Option<String>,
    client_sending: bool,
    connection_count: usize,
}

#[derive(Clone)]
struct LogEntry {
    timestamp: String,
    direction: Direction,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
enum Direction {
    Received,
    Sent,
    Info,
    Error,
}

pub struct MllpTab {
    // Client settings
    client_host: String,
    client_port: String,
    client_message: String,
    client_timeout: u32,

    // Server settings
    server_port: String,
    server_auto_ack: bool,

    // State
    state: Arc<Mutex<MllpState>>,

    // Runtime handle for async operations
    runtime: Option<tokio::runtime::Runtime>,

    // Server shutdown signal
    server_shutdown: Arc<AtomicBool>,
}

impl Default for MllpTab {
    fn default() -> Self {
        Self {
            client_host: "127.0.0.1".to_string(),
            client_port: "2575".to_string(),
            client_message: samples::ADT_A01.to_string(),
            client_timeout: 30,
            server_port: "2575".to_string(),
            server_auto_ack: true,
            state: Arc::new(Mutex::new(MllpState::default())),
            runtime: tokio::runtime::Runtime::new().ok(),
            server_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl MllpTab {
    /// Set the client message content (used by File > Open)
    pub fn set_message(&mut self, content: String) {
        self.client_message = content;
    }

    /// Get the current client message content (used by File > Save)
    pub fn get_message(&self) -> &str {
        &self.client_message
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("MLLP Client/Server");
        ui.label("Send and receive HL7 messages using the MLLP (Minimal Lower Layer Protocol).");
        ui.add_space(10.0);

        // Get available height for full-height panels
        let available_height = ui.available_height();

        StripBuilder::new(ui)
            .size(Size::relative(0.5).at_least(350.0))
            .size(Size::remainder().at_least(350.0))
            .horizontal(|mut strip| {
                // Left: Client
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());
                            self.client_ui(ui, ctx);
                        });
                });

                // Right: Server
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());
                            self.server_ui(ui, ctx);
                        });
                });
            });
    }

    fn client_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("MLLP Client");
        ui.label("Send messages to an MLLP server.");
        ui.add_space(10.0);

        let is_sending = {
            let state = self.state.lock().unwrap();
            state.client_sending
        };

        // Connection settings
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.add_enabled(
                !is_sending,
                egui::TextEdit::singleline(&mut self.client_host).desired_width(150.0)
            );
            ui.label("Port:");
            ui.add_enabled(
                !is_sending,
                egui::TextEdit::singleline(&mut self.client_port).desired_width(60.0)
            );
        });

        ui.horizontal(|ui| {
            ui.label("Timeout (sec):");
            ui.add_enabled(
                !is_sending,
                egui::DragValue::new(&mut self.client_timeout).range(1..=120)
            );
        });

        ui.add_space(10.0);

        // Sample message selector
        ui.horizontal(|ui| {
            ui.label("Load Sample:");
            let samples = samples::get_sample_messages();
            egui::ComboBox::from_id_salt("client_sample")
                .selected_text("Select...")
                .show_ui(ui, |ui| {
                    for (name, _, msg) in &samples {
                        if ui.selectable_label(false, *name).clicked() {
                            self.client_message = msg.to_string();
                        }
                    }
                });
        });

        // Message input
        ui.label("Message to Send:");
        egui::ScrollArea::vertical()
            .id_salt("client_message")
            .auto_shrink([false, false])
            .max_height(250.0)
            .show(ui, |ui| {
                ui.add_enabled(
                    !is_sending,
                    egui::TextEdit::multiline(&mut self.client_message)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(12)
                        .code_editor()
                );
            });

        ui.add_space(10.0);

        // Send button
        ui.horizontal(|ui| {
            if is_sending {
                ui.spinner();
                ui.label("Sending...");
            } else {
                if ui.button(RichText::new("Send Message").strong()).clicked() {
                    self.send_message(ctx.clone());
                }
            }
        });

        ui.add_space(10.0);

        // Response display
        let state = self.state.lock().unwrap();
        if let Some(ref error) = state.client_error {
            ui.colored_label(Color32::RED, format!("Error: {}", error));
        }

        if let Some(ref response) = state.client_response {
            ui.label(RichText::new("Response:").strong());
            let mut formatted_response = format_hl7_for_display(response);
            egui::ScrollArea::vertical()
                .id_salt("client_response")
                .auto_shrink([false, false])
                .max_height(150.0)
                .show(ui, |ui| {
                    egui::TextEdit::multiline(&mut formatted_response)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .show(ui);
                });
        }
    }

    fn server_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("MLLP Server");
        ui.label("Listen for incoming MLLP connections.");
        ui.add_space(10.0);

        let server_running = {
            let state = self.state.lock().unwrap();
            state.server_running
        };

        // Server settings
        ui.horizontal(|ui| {
            ui.label("Listen Port:");
            ui.add_enabled(
                !server_running,
                egui::TextEdit::singleline(&mut self.server_port).desired_width(60.0)
            );
        });

        ui.checkbox(&mut self.server_auto_ack, "Auto-generate ACK responses");

        ui.add_space(10.0);

        // Start/Stop buttons
        ui.horizontal(|ui| {
            if server_running {
                if ui.button(RichText::new("Stop Server").color(Color32::RED)).clicked() {
                    self.stop_server();
                }
                ui.colored_label(Color32::GREEN, "Server Running");
            } else {
                if ui.button(RichText::new("Start Server").color(Color32::GREEN)).clicked() {
                    self.start_server(ctx.clone());
                }
                ui.colored_label(Color32::GRAY, "Server Stopped");
            }
        });

        ui.add_space(10.0);

        // Connection info
        {
            let state = self.state.lock().unwrap();
            ui.label(format!("Total connections: {}", state.connection_count));
        }

        ui.add_space(10.0);

        // Message log
        ui.label(RichText::new("Message Log:").strong());

        if ui.button("Clear Log").clicked() {
            let mut state = self.state.lock().unwrap();
            state.server_messages.clear();
        }

        egui::ScrollArea::vertical()
            .id_salt("server_log")
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let state = self.state.lock().unwrap();
                for entry in &state.server_messages {
                    let (prefix, color) = match entry.direction {
                        Direction::Received => ("IN  ", Color32::GREEN),
                        Direction::Sent => ("OUT ", Color32::LIGHT_BLUE),
                        Direction::Info => ("INFO", Color32::YELLOW),
                        Direction::Error => ("ERR ", Color32::RED),
                    };

                    ui.horizontal(|ui| {
                        ui.colored_label(color, prefix);
                        ui.label(&entry.timestamp);
                    });

                    // Format HL7 messages for display (convert \r to \n)
                    let mut formatted_message = format_hl7_for_display(&entry.message);
                    egui::TextEdit::multiline(&mut formatted_message)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(3)
                        .show(ui);
                    ui.add_space(5.0);
                }
            });
    }

    fn send_message(&mut self, ctx: egui::Context) {
        let host = self.client_host.clone();
        let port = self.client_port.clone();
        let message = self.client_message.clone();
        let timeout_secs = self.client_timeout;
        let state = self.state.clone();

        // Clear previous results and set sending state
        {
            let mut s = state.lock().unwrap();
            s.client_response = None;
            s.client_error = None;
            s.client_sending = true;
        }

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                let addr = format!("{}:{}", host, port);

                // Use actual MLLP client
                match send_mllp_message(&addr, &message, timeout_secs).await {
                    Ok(response) => {
                        let mut s = state.lock().unwrap();
                        s.client_response = Some(response);
                        s.client_error = None;
                        s.client_sending = false;
                    }
                    Err(e) => {
                        let mut s = state.lock().unwrap();
                        s.client_error = Some(e);
                        s.client_response = None;
                        s.client_sending = false;
                    }
                }

                ctx.request_repaint();
            });
        }
    }

    fn start_server(&mut self, ctx: egui::Context) {
        let port: u16 = match self.server_port.parse() {
            Ok(p) => p,
            Err(_) => {
                let mut state = self.state.lock().unwrap();
                state.server_messages.push(LogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    direction: Direction::Error,
                    message: "Invalid port number".to_string(),
                });
                return;
            }
        };

        let state = self.state.clone();
        let auto_ack = self.server_auto_ack;
        let shutdown = self.server_shutdown.clone();

        // Reset shutdown flag
        shutdown.store(false, Ordering::SeqCst);

        {
            let mut s = state.lock().unwrap();
            s.server_running = true;
            s.server_messages.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: format!("Starting server on port {}...", port),
            });
        }

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                run_mllp_server(port, auto_ack, state, shutdown, ctx).await;
            });
        }
    }

    fn stop_server(&mut self) {
        // Signal shutdown
        self.server_shutdown.store(true, Ordering::SeqCst);

        let mut state = self.state.lock().unwrap();
        state.server_running = false;
        state.server_messages.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: "Server stopping...".to_string(),
        });
    }
}

/// Send a message via MLLP to the specified address
async fn send_mllp_message(addr: &str, message: &str, timeout_secs: u32) -> Result<String, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};

    // MLLP frame bytes
    const VT: u8 = 0x0B;  // Start byte
    const FS: u8 = 0x1C;  // End byte 1
    const CR: u8 = 0x0D;  // End byte 2

    // Normalize line endings to CR
    let normalized_message = message.replace("\r\n", "\r").replace('\n', "\r");

    // Frame the message with MLLP envelope
    let mut framed = Vec::with_capacity(normalized_message.len() + 3);
    framed.push(VT);
    framed.extend_from_slice(normalized_message.as_bytes());
    framed.push(FS);
    framed.push(CR);

    // Connect with timeout
    let connect_timeout = Duration::from_secs(timeout_secs as u64);
    let stream = timeout(connect_timeout, TcpStream::connect(addr))
        .await
        .map_err(|_| format!("Connection timeout after {} seconds", timeout_secs))?
        .map_err(|e| format!("Connection failed: {}", e))?;

    let (mut reader, mut writer) = stream.into_split();

    // Send the framed message
    writer.write_all(&framed).await
        .map_err(|e| format!("Failed to send message: {}", e))?;
    writer.flush().await
        .map_err(|e| format!("Failed to flush: {}", e))?;

    // Read response with timeout
    let read_timeout = Duration::from_secs(timeout_secs as u64);
    let mut response_buf = Vec::new();
    let mut buf = [0u8; 4096];

    let response = timeout(read_timeout, async {
        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            response_buf.extend_from_slice(&buf[..n]);

            // Check for end of MLLP frame
            if response_buf.len() >= 2
                && response_buf[response_buf.len() - 2] == FS
                && response_buf[response_buf.len() - 1] == CR
            {
                break;
            }
        }
        Ok::<_, std::io::Error>(())
    }).await
        .map_err(|_| format!("Read timeout after {} seconds", timeout_secs))?
        .map_err(|e| format!("Failed to read response: {}", e));

    response?;

    // Extract message from MLLP frame
    if response_buf.is_empty() {
        return Err("Empty response from server".to_string());
    }

    // Remove MLLP framing
    let start = if response_buf.first() == Some(&VT) { 1 } else { 0 };
    let end = if response_buf.len() >= 2
        && response_buf[response_buf.len() - 2] == FS
        && response_buf[response_buf.len() - 1] == CR
    {
        response_buf.len() - 2
    } else {
        response_buf.len()
    };

    CharacterSet::decode_message(&response_buf[start..end]).map_err(|e| e.to_string())
}

/// Run the MLLP server
async fn run_mllp_server(
    port: u16,
    auto_ack: bool,
    state: Arc<Mutex<MllpState>>,
    shutdown: Arc<AtomicBool>,
    ctx: egui::Context,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::time::{timeout, Duration};

    // MLLP frame bytes
    const VT: u8 = 0x0B;
    const FS: u8 = 0x1C;
    const CR: u8 = 0x0D;

    let addr = format!("0.0.0.0:{}", port);

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            let mut s = state.lock().unwrap();
            s.server_running = false;
            s.server_messages.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Error,
                message: format!("Failed to bind to port {}: {}", port, e),
            });
            ctx.request_repaint();
            return;
        }
    };

    {
        let mut s = state.lock().unwrap();
        s.server_messages.push(LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            direction: Direction::Info,
            message: format!("Server listening on port {}", port),
        });
    }
    ctx.request_repaint();

    // Accept connections until shutdown
    loop {
        if shutdown.load(Ordering::SeqCst) {
            let mut s = state.lock().unwrap();
            s.server_messages.push(LogEntry {
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                direction: Direction::Info,
                message: "Server stopped".to_string(),
            });
            s.server_running = false;
            ctx.request_repaint();
            break;
        }

        // Accept with short timeout to allow checking shutdown flag
        let accept_result = timeout(Duration::from_millis(100), listener.accept()).await;

        match accept_result {
            Ok(Ok((mut stream, peer_addr))) => {
                {
                    let mut s = state.lock().unwrap();
                    s.connection_count += 1;
                    s.server_messages.push(LogEntry {
                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                        direction: Direction::Info,
                        message: format!("Connection from {}", peer_addr),
                    });
                }
                ctx.request_repaint();

                // Handle this connection
                let state_clone = state.clone();
                let ctx_clone = ctx.clone();
                let shutdown_clone = shutdown.clone();

                tokio::spawn(async move {
                    let mut buf = [0u8; 8192];
                    let mut message_buf = Vec::new();

                    loop {
                        if shutdown_clone.load(Ordering::SeqCst) {
                            break;
                        }

                        // Read with timeout
                        let read_result = timeout(Duration::from_secs(30), stream.read(&mut buf)).await;

                        match read_result {
                            Ok(Ok(0)) => {
                                // Connection closed
                                {
                                    let mut s = state_clone.lock().unwrap();
                                    s.server_messages.push(LogEntry {
                                        timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                        direction: Direction::Info,
                                        message: format!("Connection from {} closed", peer_addr),
                                    });
                                }
                                ctx_clone.request_repaint();
                                break;
                            }
                            Ok(Ok(n)) => {
                                message_buf.extend_from_slice(&buf[..n]);

                                // Check for complete MLLP frame
                                if message_buf.len() >= 2
                                    && message_buf[message_buf.len() - 2] == FS
                                    && message_buf[message_buf.len() - 1] == CR
                                {
                                    // Extract message
                                    let start = if message_buf.first() == Some(&VT) { 1 } else { 0 };
                                    let end = message_buf.len() - 2;

                                    if let Ok(hl7_message) = CharacterSet::decode_message(&message_buf[start..end]) {
                                        // Log received message
                                        {
                                            let mut s = state_clone.lock().unwrap();
                                            s.server_messages.push(LogEntry {
                                                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                                direction: Direction::Received,
                                                message: hl7_message.clone(),
                                            });
                                        }
                                        ctx_clone.request_repaint();

                                        // Send ACK if auto_ack is enabled
                                        if auto_ack {
                                            let ack = generate_ack(&hl7_message);
                                            let mut ack_frame = Vec::new();
                                            ack_frame.push(VT);
                                            ack_frame.extend_from_slice(ack.as_bytes());
                                            ack_frame.push(FS);
                                            ack_frame.push(CR);

                                            if let Err(e) = stream.write_all(&ack_frame).await {
                                                let mut s = state_clone.lock().unwrap();
                                                s.server_messages.push(LogEntry {
                                                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                                    direction: Direction::Error,
                                                    message: format!("Failed to send ACK: {}", e),
                                                });
                                            } else {
                                                let mut s = state_clone.lock().unwrap();
                                                s.server_messages.push(LogEntry {
                                                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                                    direction: Direction::Sent,
                                                    message: ack,
                                                });
                                            }
                                            ctx_clone.request_repaint();
                                        }
                                    }

                                    // Clear buffer for next message
                                    message_buf.clear();
                                }
                            }
                            Ok(Err(e)) => {
                                let mut s = state_clone.lock().unwrap();
                                s.server_messages.push(LogEntry {
                                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                    direction: Direction::Error,
                                    message: format!("Read error from {}: {}", peer_addr, e),
                                });
                                ctx_clone.request_repaint();
                                break;
                            }
                            Err(_) => {
                                // Timeout, just continue to check shutdown flag
                            }
                        }
                    }
                });
            }
            Ok(Err(e)) => {
                let mut s = state.lock().unwrap();
                s.server_messages.push(LogEntry {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    direction: Direction::Error,
                    message: format!("Accept error: {}", e),
                });
                ctx.request_repaint();
            }
            Err(_) => {
                // Timeout, continue to check shutdown flag
            }
        }
    }
}

/// Format an HL7 message for display by converting segment separators to line feeds
fn format_hl7_for_display(message: &str) -> String {
    // HL7 uses \r as segment separator - convert to \n for display
    message.replace('\r', "\n").trim_end_matches('\n').to_string()
}

/// Generate an ACK response for an HL7 message
fn generate_ack(message: &str) -> String {
    // Parse the message to extract MSH fields
    let lines: Vec<&str> = message.split('\r').collect();
    let msh = lines.first().unwrap_or(&"");

    // Default values
    let mut control_id = "UNKNOWN";
    let mut version = "2.5";
    let mut sending_app = "";
    let mut sending_fac = "";
    let mut receiving_app = "";
    let mut receiving_fac = "";

    // Parse MSH fields (assuming | delimiter)
    if msh.starts_with("MSH|") {
        let fields: Vec<&str> = msh.split('|').collect();
        if fields.len() > 9 {
            sending_app = fields.get(2).unwrap_or(&"");
            sending_fac = fields.get(3).unwrap_or(&"");
            receiving_app = fields.get(4).unwrap_or(&"");
            receiving_fac = fields.get(5).unwrap_or(&"");
            control_id = fields.get(9).unwrap_or(&"UNKNOWN");
        }
        if fields.len() > 11 {
            version = fields.get(11).unwrap_or(&"2.5");
        }
    }

    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");

    format!(
        "MSH|^~\\&|{}|{}|{}|{}|{}||ACK|ACK{}|P|{}\rMSA|AA|{}|Message accepted\r",
        receiving_app, receiving_fac, sending_app, sending_fac,
        timestamp, timestamp, version, control_id
    )
}
