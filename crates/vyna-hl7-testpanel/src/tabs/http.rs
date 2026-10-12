//! HTTP Tab - Send and receive HL7 messages via HTTP/HTTPS

use crate::samples;
use egui::{self, Color32, RichText};
use vyna_hl7_core::CharacterSet;
use egui_extras::{Size, StripBuilder};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// State shared between UI and async tasks
#[derive(Default)]
struct HttpState {
    server_running: bool,
    server_messages: Vec<LogEntry>,
    client_response: Option<HttpResponse>,
    client_error: Option<String>,
    client_sending: bool,
    connection_count: usize,
    // Load test state
    load_test_running: bool,
    load_test_sent: u32,
    load_test_received: u32,
    load_test_errors: u32,
    load_test_target: u32,
    load_test_start_time: Option<std::time::Instant>,
    load_test_end_time: Option<std::time::Instant>,
    load_test_throughput: f64,
}

#[derive(Clone)]
struct HttpResponse {
    status_code: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: String,
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

#[derive(Clone, Copy, PartialEq, Default)]
enum HttpMethod {
    #[default]
    Post,
    Put,
}

impl HttpMethod {
    fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
enum AuthType {
    #[default]
    None,
    Basic,
    ApiKey,
    Bearer,
}

/// Load test message count options
#[derive(Clone, Copy, PartialEq)]
#[derive(Default)]
enum LoadTestCount {
    #[default]
    Count100,
    Count1000,
    Count10000,
}

impl LoadTestCount {
    fn value(&self) -> u32 {
        match self {
            LoadTestCount::Count100 => 100,
            LoadTestCount::Count1000 => 1000,
            LoadTestCount::Count10000 => 10000,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            LoadTestCount::Count100 => "100 messages",
            LoadTestCount::Count1000 => "1,000 messages",
            LoadTestCount::Count10000 => "10,000 messages",
        }
    }
}


pub struct HttpTab {
    // Client settings
    client_url: String,
    client_method: HttpMethod,
    client_message: String,
    client_timeout: u32,
    client_auth_type: AuthType,
    client_username: String,
    client_password: String,
    client_api_key_header: String,
    client_api_key: String,
    client_bearer_token: String,
    client_headers: Vec<(String, String)>,
    new_header_key: String,
    new_header_value: String,

    // Server settings
    server_port: String,
    server_auto_ack: bool,

    // Load test settings
    load_test_count: LoadTestCount,

    // State
    state: Arc<Mutex<HttpState>>,

    // Runtime handle for async operations
    runtime: Option<tokio::runtime::Runtime>,

    // Server shutdown signal
    server_shutdown: Arc<AtomicBool>,

    // Load test stop signal
    load_test_shutdown: Arc<AtomicBool>,
}

impl Default for HttpTab {
    fn default() -> Self {
        Self {
            client_url: "http://127.0.0.1:8080/hl7".to_string(),
            client_method: HttpMethod::Post,
            client_message: samples::ADT_A01.to_string(),
            client_timeout: 30,
            client_auth_type: AuthType::None,
            client_username: String::new(),
            client_password: String::new(),
            client_api_key_header: "X-API-Key".to_string(),
            client_api_key: String::new(),
            client_bearer_token: String::new(),
            client_headers: vec![(
                "Content-Type".to_string(),
                "x-application/hl7-v2+er7".to_string(),
            )],
            new_header_key: String::new(),
            new_header_value: String::new(),
            server_port: "8080".to_string(),
            server_auto_ack: true,
            load_test_count: LoadTestCount::default(),
            state: Arc::new(Mutex::new(HttpState::default())),
            runtime: tokio::runtime::Runtime::new().ok(),
            server_shutdown: Arc::new(AtomicBool::new(false)),
            load_test_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl HttpTab {
    pub fn ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("HTTP Client/Server");
        ui.label("Send and receive HL7 messages using HTTP/HTTPS (HL7-over-HTTP specification).");
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
        ui.heading("HTTP Client");
        ui.label("Send HL7 messages via HTTP/HTTPS POST.");
        ui.add_space(10.0);

        let is_sending = {
            let state = self.state.lock().unwrap();
            state.client_sending
        };

        // URL and method
        ui.horizontal(|ui| {
            ui.label("URL:");
            ui.add_enabled(
                !is_sending,
                egui::TextEdit::singleline(&mut self.client_url).desired_width(300.0),
            );
        });

        ui.horizontal(|ui| {
            ui.label("Method:");
            ui.add_enabled_ui(!is_sending, |ui| {
                egui::ComboBox::from_id_salt("http_method")
                    .selected_text(self.client_method.as_str())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.client_method, HttpMethod::Post, "POST");
                        ui.selectable_value(&mut self.client_method, HttpMethod::Put, "PUT");
                    });
            });

            ui.label("Timeout (sec):");
            ui.add_enabled(
                !is_sending,
                egui::DragValue::new(&mut self.client_timeout).range(1..=120),
            );
        });

        // Authentication
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Auth:");
            ui.add_enabled_ui(!is_sending, |ui| {
                egui::ComboBox::from_id_salt("auth_type")
                    .selected_text(match self.client_auth_type {
                        AuthType::None => "None",
                        AuthType::Basic => "Basic",
                        AuthType::ApiKey => "API Key",
                        AuthType::Bearer => "Bearer",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.client_auth_type, AuthType::None, "None");
                        ui.selectable_value(
                            &mut self.client_auth_type,
                            AuthType::Basic,
                            "Basic Auth",
                        );
                        ui.selectable_value(
                            &mut self.client_auth_type,
                            AuthType::ApiKey,
                            "API Key",
                        );
                        ui.selectable_value(
                            &mut self.client_auth_type,
                            AuthType::Bearer,
                            "Bearer Token",
                        );
                    });
            });

            match self.client_auth_type {
                AuthType::Basic => {
                    ui.label("User:");
                    ui.add_enabled(
                        !is_sending,
                        egui::TextEdit::singleline(&mut self.client_username).desired_width(80.0),
                    );
                    ui.label("Pass:");
                    ui.add_enabled(
                        !is_sending,
                        egui::TextEdit::singleline(&mut self.client_password)
                            .password(true)
                            .desired_width(80.0),
                    );
                }
                AuthType::ApiKey => {
                    ui.label("Header:");
                    ui.add_enabled(
                        !is_sending,
                        egui::TextEdit::singleline(&mut self.client_api_key_header)
                            .desired_width(80.0),
                    );
                    ui.label("Key:");
                    ui.add_enabled(
                        !is_sending,
                        egui::TextEdit::singleline(&mut self.client_api_key)
                            .password(true)
                            .desired_width(150.0),
                    );
                }
                AuthType::Bearer => {
                    ui.label("Token:");
                    ui.add_enabled(
                        !is_sending,
                        egui::TextEdit::singleline(&mut self.client_bearer_token)
                            .password(true)
                            .desired_width(200.0),
                    );
                }
                AuthType::None => {}
            }
        });

        // Headers
        ui.add_space(5.0);
        ui.collapsing("Headers", |ui| {
            let mut to_remove = None;
            for (i, (key, value)) in self.client_headers.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add_enabled(
                        !is_sending,
                        egui::TextEdit::singleline(key).desired_width(120.0),
                    );
                    ui.label(":");
                    ui.add_enabled(
                        !is_sending,
                        egui::TextEdit::singleline(value).desired_width(180.0),
                    );
                    if ui
                        .add_enabled(!is_sending, egui::Button::new("\u{2716}"))
                        .clicked()
                    {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(i) = to_remove {
                self.client_headers.remove(i);
            }

            ui.horizontal(|ui| {
                ui.add_enabled(
                    !is_sending,
                    egui::TextEdit::singleline(&mut self.new_header_key)
                        .hint_text("Key")
                        .desired_width(120.0),
                );
                ui.label(":");
                ui.add_enabled(
                    !is_sending,
                    egui::TextEdit::singleline(&mut self.new_header_value)
                        .hint_text("Value")
                        .desired_width(180.0),
                );
                if ui
                    .add_enabled(
                        !is_sending && !self.new_header_key.is_empty(),
                        egui::Button::new("Add"),
                    )
                    .clicked()
                {
                    self.client_headers.push((
                        std::mem::take(&mut self.new_header_key),
                        std::mem::take(&mut self.new_header_value),
                    ));
                }
            });
        });

        ui.add_space(5.0);

        // Sample message selector
        ui.horizontal(|ui| {
            ui.label("Load Sample:");
            let samples = samples::get_sample_messages();
            egui::ComboBox::from_id_salt("http_client_sample")
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
        ui.label("Request Body:");
        egui::ScrollArea::vertical()
            .id_salt("http_client_message")
            .auto_shrink([false, false])
            .max_height(150.0)
            .show(ui, |ui| {
                ui.add_enabled(
                    !is_sending,
                    egui::TextEdit::multiline(&mut self.client_message)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(8)
                        .code_editor(),
                );
            });

        ui.add_space(10.0);

        // Send button
        ui.horizontal(|ui| {
            if is_sending {
                ui.spinner();
                ui.label("Sending...");
            } else if ui.button(RichText::new("Send Request").strong()).clicked() {
                self.send_request(ctx.clone());
            }
        });

        ui.add_space(10.0);

        // Response display (scoped to drop lock before load test UI)
        {
            let state = self.state.lock().unwrap();
            if let Some(ref error) = state.client_error {
                ui.colored_label(Color32::RED, format!("Error: {}", error));
            }

            if let Some(ref response) = state.client_response {
                let status_color = if response.status_code >= 200 && response.status_code < 300 {
                    Color32::GREEN
                } else if response.status_code >= 400 {
                    Color32::RED
                } else {
                    Color32::YELLOW
                };

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Status:").strong());
                    ui.colored_label(
                        status_color,
                        format!("{} {}", response.status_code, response.status_text),
                    );
                });

                ui.collapsing("Response Headers", |ui| {
                    for (key, value) in &response.headers {
                        ui.label(format!("{}: {}", key, value));
                    }
                });

                ui.label(RichText::new("Response Body:").strong());
                egui::ScrollArea::vertical()
                    .id_salt("http_client_response")
                    .auto_shrink([false, false])
                    .max_height(100.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut response.body.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .interactive(false),
                        );
                    });
            }
        }

        // Load Test Section
        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new("Load Test").strong());
        ui.add_space(5.0);

        let load_test_running = {
            let state = self.state.lock().unwrap();
            state.load_test_running
        };

        ui.horizontal(|ui| {
            ui.label("Messages:");
            ui.add_enabled_ui(!load_test_running && !is_sending, |ui| {
                egui::ComboBox::from_id_salt("http_load_test_count")
                    .selected_text(self.load_test_count.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.load_test_count,
                            LoadTestCount::Count100,
                            LoadTestCount::Count100.label(),
                        );
                        ui.selectable_value(
                            &mut self.load_test_count,
                            LoadTestCount::Count1000,
                            LoadTestCount::Count1000.label(),
                        );
                        ui.selectable_value(
                            &mut self.load_test_count,
                            LoadTestCount::Count10000,
                            LoadTestCount::Count10000.label(),
                        );
                    });
            });
        });

        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if load_test_running {
                if ui
                    .button(RichText::new("Stop Load Test").color(Color32::RED))
                    .clicked()
                {
                    self.stop_load_test();
                }
                ui.spinner();
            } else if ui
                .add_enabled(
                    !is_sending,
                    egui::Button::new(RichText::new("Start Load Test").color(Color32::LIGHT_BLUE)),
                )
                .clicked()
            {
                self.start_load_test(ctx.clone());
            }
        });

        // Progress display
        {
            let state = self.state.lock().unwrap();
            if state.load_test_running || state.load_test_sent > 0 {
                ui.add_space(5.0);

                let progress = if state.load_test_target > 0 {
                    state.load_test_sent as f32 / state.load_test_target as f32
                } else {
                    0.0
                };

                ui.add(egui::ProgressBar::new(progress).text(format!(
                    "{} / {} messages",
                    state.load_test_sent, state.load_test_target
                )));

                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Sent: {} | ACKs: {} | Errors: {}",
                        state.load_test_sent, state.load_test_received, state.load_test_errors
                    ));
                });

                if state.load_test_throughput > 0.0 {
                    ui.label(format!(
                        "Throughput: {:.1} msg/sec",
                        state.load_test_throughput
                    ));
                }

                // Show final results
                if !state.load_test_running && state.load_test_sent > 0
                    && let (Some(start), Some(end)) =
                        (state.load_test_start_time, state.load_test_end_time)
                    {
                        let duration = end.duration_since(start);
                        let final_throughput = state.load_test_sent as f64 / duration.as_secs_f64();
                        ui.colored_label(
                            Color32::GREEN,
                            format!(
                                "Complete! {:.2}s total, {:.1} msg/sec avg",
                                duration.as_secs_f64(),
                                final_throughput
                            ),
                        );
                    }
            }
        }
    }

    fn server_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("HTTP Server");
        ui.label("Listen for incoming HTTP POST requests with HL7 messages.");
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
                egui::TextEdit::singleline(&mut self.server_port).desired_width(60.0),
            );
        });

        ui.checkbox(&mut self.server_auto_ack, "Auto-generate ACK responses");

        ui.add_space(10.0);

        // Start/Stop buttons
        ui.horizontal(|ui| {
            if server_running {
                if ui
                    .button(RichText::new("Stop Server").color(Color32::RED))
                    .clicked()
                {
                    self.stop_server();
                }
                ui.colored_label(Color32::GREEN, "Server Running");
            } else if ui
                .button(RichText::new("Start Server").color(Color32::GREEN))
                .clicked()
            {
                self.start_server(ctx.clone());
            } else {
                ui.colored_label(Color32::GRAY, "Server Stopped");
            }
        });

        ui.add_space(10.0);

        // Connection info
        {
            let state = self.state.lock().unwrap();
            ui.label(format!("Total requests: {}", state.connection_count));
        }

        ui.add_space(10.0);

        // Message log
        ui.label(RichText::new("Request Log:").strong());

        if ui.button("Clear Log").clicked() {
            let mut state = self.state.lock().unwrap();
            state.server_messages.clear();
        }

        egui::ScrollArea::vertical()
            .id_salt("http_server_log")
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

                    ui.add(
                        egui::TextEdit::multiline(&mut entry.message.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .interactive(false),
                    );
                    ui.add_space(5.0);
                }
            });
    }

    fn send_request(&mut self, ctx: egui::Context) {
        let state = self.state.clone();

        // Clear previous results and set sending state
        {
            let mut s = state.lock().unwrap();
            s.client_response = None;
            s.client_error = None;
            s.client_sending = true;
        }

        let request = SendHttpRequest {
            url: self.client_url.clone(),
            method: self.client_method,
            message: self.client_message.clone(),
            timeout_secs: self.client_timeout,
            auth_type: self.client_auth_type,
            username: self.client_username.clone(),
            password: self.client_password.clone(),
            api_key_header: self.client_api_key_header.clone(),
            api_key: self.client_api_key.clone(),
            bearer_token: self.client_bearer_token.clone(),
            headers: self.client_headers.clone(),
        };

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                match send_http_request(request).await
                {
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
                message: format!("Starting HTTP server on port {}...", port),
            });
        }

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                run_http_server(port, auto_ack, state, shutdown, ctx).await;
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

    fn start_load_test(&mut self, ctx: egui::Context) {
        let state = self.state.clone();
        let shutdown = self.load_test_shutdown.clone();
        let count = self.load_test_count.value();

        // Reset shutdown flag and state
        shutdown.store(false, Ordering::SeqCst);

        {
            let mut s = state.lock().unwrap();
            s.load_test_running = true;
            s.load_test_sent = 0;
            s.load_test_received = 0;
            s.load_test_errors = 0;
            s.load_test_target = count;
            s.load_test_start_time = Some(std::time::Instant::now());
            s.load_test_end_time = None;
            s.load_test_throughput = 0.0;
        }

        let params = HttpLoadTest {
            url: self.client_url.clone(),
            method: self.client_method,
            message: self.client_message.clone(),
            timeout_secs: self.client_timeout,
            auth_type: self.client_auth_type,
            username: self.client_username.clone(),
            password: self.client_password.clone(),
            api_key_header: self.client_api_key_header.clone(),
            api_key: self.client_api_key.clone(),
            bearer_token: self.client_bearer_token.clone(),
            headers: self.client_headers.clone(),
            count,
            state,
            shutdown,
            ctx,
        };

        if let Some(ref runtime) = self.runtime {
            runtime.spawn(async move {
                run_http_load_test(params).await;
            });
        }
    }

    fn stop_load_test(&mut self) {
        self.load_test_shutdown.store(true, Ordering::SeqCst);

        let mut state = self.state.lock().unwrap();
        state.load_test_running = false;
        state.load_test_end_time = Some(std::time::Instant::now());
    }

    /// Set the message content (for file open support)
    pub fn set_message(&mut self, content: String) {
        self.client_message = content;
    }

    /// Get the current message content (for file save support)
    pub fn get_message(&self) -> &str {
        &self.client_message
    }
}

/// Parameters for sending an HL7 message over HTTP/HTTPS.
struct SendHttpRequest {
    url: String,
    method: HttpMethod,
    message: String,
    timeout_secs: u32,
    auth_type: AuthType,
    username: String,
    password: String,
    api_key_header: String,
    api_key: String,
    bearer_token: String,
    headers: Vec<(String, String)>,
}

/// Send an HTTP/HTTPS request with HL7 message using reqwest
async fn send_http_request(request: SendHttpRequest) -> Result<HttpResponse, String> {
    let SendHttpRequest {
        url,
        method,
        message,
        timeout_secs,
        auth_type,
        username,
        password,
        api_key_header,
        api_key,
        bearer_token,
        headers,
    } = request;
    let url = url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    // Normalize line endings to CR for HL7
    let normalized_message = message.replace("\r\n", "\r").replace('\n', "\r");

    // Build reqwest client (accepts any TLS certificate for flexibility)
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs as u64))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Build request
    let mut req = match method {
        HttpMethod::Post => client.post(url),
        HttpMethod::Put => client.put(url),
    };

    // Add authentication
    match auth_type {
        AuthType::Basic if !username.is_empty() => {
            req = req.basic_auth(username, Some(password));
        }
        AuthType::ApiKey if !api_key.is_empty() => {
            let header_name = if api_key_header.is_empty() {
                "X-API-Key"
            } else {
                api_key_header.as_str()
            };
            req = req.header(header_name, api_key);
        }
        AuthType::Bearer if !bearer_token.is_empty() => {
            req = req.bearer_auth(bearer_token);
        }
        _ => {}
    }

    // Add custom headers
    for (key, value) in &headers {
        if !key.is_empty() {
            req = req.header(key.as_str(), value.as_str());
        }
    }

    // Send request with body
    let response = req
        .body(normalized_message)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    // Extract response details
    let status_code = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    let response_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                v.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(HttpResponse {
        status_code,
        status_text,
        headers: response_headers,
        body,
    })
}

/// Run the HTTP server
async fn run_http_server(
    port: u16,
    auto_ack: bool,
    state: Arc<Mutex<HttpState>>,
    shutdown: Arc<AtomicBool>,
    ctx: egui::Context,
) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;
    use tokio::time::{Duration, timeout};

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
            message: format!("HTTP Server listening on http://0.0.0.0:{}/", port),
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
            Ok(Ok((stream, peer_addr))) => {
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

                tokio::spawn(async move {
                    let (reader, mut writer) = stream.into_split();
                    let mut buf_reader = BufReader::new(reader);

                    // Read HTTP request
                    let mut request_line = String::new();
                    if let Err(e) = buf_reader.read_line(&mut request_line).await {
                        let mut s = state_clone.lock().unwrap();
                        s.server_messages.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Error,
                            message: format!("Read error: {}", e),
                        });
                        return;
                    }

                    // Parse request line (e.g., "POST /hl7 HTTP/1.1")
                    let parts: Vec<&str> = request_line.split_whitespace().collect();
                    let method = parts.first().unwrap_or(&"");
                    let path = parts.get(1).unwrap_or(&"/");

                    // Read headers
                    let mut content_length: usize = 0;
                    loop {
                        let mut line = String::new();
                        if buf_reader.read_line(&mut line).await.is_err() {
                            break;
                        }
                        if line.trim().is_empty() {
                            break;
                        }
                        if let Some((key, value)) = line.split_once(':')
                            && key.trim().eq_ignore_ascii_case("content-length") {
                                content_length = value.trim().parse().unwrap_or(0);
                            }
                    }

                    // Read body
                    let mut body = vec![0u8; content_length];
                    if content_length > 0 {
                        use tokio::io::AsyncReadExt;
                        if let Err(e) = buf_reader.read_exact(&mut body).await {
                            let mut s = state_clone.lock().unwrap();
                            s.server_messages.push(LogEntry {
                                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                                direction: Direction::Error,
                                message: format!("Body read error: {}", e),
                            });
                            return;
                        }
                    }

                    let body_str = CharacterSet::decode_message(&body)
                        .unwrap_or_else(|_| String::from_utf8_lossy(&body).to_string());

                    // Log received request
                    {
                        let mut s = state_clone.lock().unwrap();
                        s.server_messages.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Received,
                            message: format!("{} {}\n{}", method, path, body_str),
                        });
                    }
                    ctx_clone.request_repaint();

                    // Generate response
                    let (status_code, status_text, response_body) =
                        if *method == "POST" || *method == "PUT" {
                            if auto_ack && !body_str.is_empty() {
                                let ack = generate_ack(&body_str);
                                (200, "OK", ack)
                            } else {
                                (200, "OK", "Message received".to_string())
                            }
                        } else {
                            (
                                405,
                                "Method Not Allowed",
                                "Only POST and PUT are supported".to_string(),
                            )
                        };

                    // Build HTTP response
                    let response = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: x-application/hl7-v2+er7\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        status_code,
                        status_text,
                        response_body.len(),
                        response_body
                    );

                    // Send response
                    if let Err(e) = writer.write_all(response.as_bytes()).await {
                        let mut s = state_clone.lock().unwrap();
                        s.server_messages.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Error,
                            message: format!("Write error: {}", e),
                        });
                        return;
                    }

                    {
                        let mut s = state_clone.lock().unwrap();
                        s.server_messages.push(LogEntry {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            direction: Direction::Sent,
                            message: format!(
                                "HTTP {} {}\n{}",
                                status_code, status_text, response_body
                            ),
                        });
                    }
                    ctx_clone.request_repaint();
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
        receiving_app,
        receiving_fac,
        sending_app,
        sending_fac,
        timestamp,
        timestamp,
        version,
        control_id
    )
}

/// Parameters for the HTTP load-test task.
struct HttpLoadTest {
    url: String,
    method: HttpMethod,
    message: String,
    timeout_secs: u32,
    auth_type: AuthType,
    username: String,
    password: String,
    api_key_header: String,
    api_key: String,
    bearer_token: String,
    headers: Vec<(String, String)>,
    count: u32,
    state: Arc<Mutex<HttpState>>,
    shutdown: Arc<AtomicBool>,
    ctx: egui::Context,
}

/// Run HTTP/HTTPS load test - send messages as fast as possible using reqwest
async fn run_http_load_test(params: HttpLoadTest) {
    let HttpLoadTest {
        url,
        method,
        message,
        timeout_secs,
        auth_type,
        username,
        password,
        api_key_header,
        api_key,
        bearer_token,
        headers,
        count,
        state,
        shutdown,
        ctx,
    } = params;
    let url_trimmed = url.trim().to_string();
    if !url_trimmed.starts_with("http://") && !url_trimmed.starts_with("https://") {
        let mut s = state.lock().unwrap();
        s.load_test_running = false;
        s.load_test_errors = count;
        return;
    }

    // Build reqwest client with connection pooling
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs as u64))
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(4)
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            let mut s = state.lock().unwrap();
            s.load_test_running = false;
            s.load_test_errors = count;
            return;
        }
    };

    // Normalize message once
    let normalized_message = message.replace("\r\n", "\r").replace('\n', "\r");

    // Throughput tracking
    let start_time = std::time::Instant::now();
    let mut last_update = start_time;

    for i in 0..count {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }

        // Build request
        let mut req = match method {
            HttpMethod::Post => client.post(&url_trimmed),
            HttpMethod::Put => client.put(&url_trimmed),
        };

        // Add authentication
        match auth_type {
            AuthType::Basic if !username.is_empty() => {
                req = req.basic_auth(&username, Some(&password));
            }
            AuthType::ApiKey if !api_key.is_empty() => {
                let header_name = if api_key_header.is_empty() {
                    "X-API-Key"
                } else {
                    &api_key_header
                };
                req = req.header(header_name, &api_key);
            }
            AuthType::Bearer if !bearer_token.is_empty() => {
                req = req.bearer_auth(&bearer_token);
            }
            _ => {}
        }

        // Add custom headers
        for (key, value) in &headers {
            if !key.is_empty() {
                req = req.header(key.as_str(), value.as_str());
            }
        }

        // Send
        let result = req.body(normalized_message.clone()).send().await;

        // Update sent count
        {
            let mut s = state.lock().unwrap();
            s.load_test_sent = i + 1;
        }

        match result {
            Ok(resp) if resp.status().is_success() => {
                // Consume the body to allow connection reuse
                let _ = resp.bytes().await;
                let mut s = state.lock().unwrap();
                s.load_test_received += 1;
            }
            _ => {
                let mut s = state.lock().unwrap();
                s.load_test_errors += 1;
            }
        }

        // Update throughput every 100ms
        let now = std::time::Instant::now();
        if now.duration_since(last_update).as_millis() >= 100 {
            let elapsed = now.duration_since(start_time).as_secs_f64();
            let total_sent = {
                let s = state.lock().unwrap();
                s.load_test_sent
            };

            if elapsed > 0.0 {
                let throughput = total_sent as f64 / elapsed;
                let mut s = state.lock().unwrap();
                s.load_test_throughput = throughput;
            }

            last_update = now;
            ctx.request_repaint();
        }
    }

    // Final update
    {
        let mut s = state.lock().unwrap();
        s.load_test_running = false;
        s.load_test_end_time = Some(std::time::Instant::now());

        if let Some(start) = s.load_test_start_time {
            let elapsed = s
                .load_test_end_time
                .unwrap()
                .duration_since(start)
                .as_secs_f64();
            if elapsed > 0.0 {
                s.load_test_throughput = s.load_test_sent as f64 / elapsed;
            }
        }
    }
    ctx.request_repaint();
}
