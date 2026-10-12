//! FHIR Tab - Convert between HL7 v2.x and FHIR R4

use egui::{self, RichText, Color32};
use egui_extras::{StripBuilder, Size};
use vyna_hl7_parser::parse_message;
#[allow(unused_imports)]
use vyna_hl7_core::Message;
use vyna_hl7_fhir::converters::{
    PatientConverter, ObservationConverter, EncounterConverter,
    PractitionerConverter, DiagnosticReportConverter,
};
use vyna_hl7_fhir::converters::reverse::{
    PatientReverseConverter, ObservationReverseConverter,
    EncounterReverseConverter,
};
use vyna_hl7_core::Delimiters;
use crate::samples;
use std::sync::{Arc, Mutex};

/// State shared between UI and async FHIR send task
#[derive(Default)]
struct FhirSendState {
    sending: bool,
    response: Option<FhirSendResponse>,
    error: Option<String>,
}

#[derive(Clone)]
struct FhirSendResponse {
    status_code: u16,
    status_text: String,
    body: String,
}

#[derive(Clone, Copy, PartialEq, Default)]
enum FhirAuthType {
    #[default]
    None,
    Basic,
    ApiKey,
    Bearer,
}

pub struct FhirTab {
    // HL7 to FHIR
    hl7_input: String,
    fhir_output: String,
    hl7_to_fhir_error: Option<String>,
    selected_resource: ResourceType,

    // FHIR to HL7 (reverse)
    fhir_input: String,
    hl7_output: String,
    fhir_to_hl7_error: Option<String>,
    reverse_resource: ReverseResourceType,

    // UI state
    active_direction: ConversionDirection,

    // FHIR send settings
    fhir_server_url: String,
    fhir_auth_type: FhirAuthType,
    fhir_username: String,
    fhir_password: String,
    fhir_api_key_header: String,
    fhir_api_key: String,
    fhir_bearer_token: String,
    fhir_send_state: Arc<Mutex<FhirSendState>>,
    fhir_runtime: Option<tokio::runtime::Runtime>,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum ConversionDirection {
    #[default]
    Hl7ToFhir,
    FhirToHl7,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum ResourceType {
    #[default]
    Patient,
    Observation,
    Encounter,
    Practitioner,
    DiagnosticReport,
    All,
}

impl ResourceType {
    fn label(&self) -> &str {
        match self {
            ResourceType::Patient => "Patient (PID)",
            ResourceType::Observation => "Observation (OBX)",
            ResourceType::Encounter => "Encounter (PV1)",
            ResourceType::Practitioner => "Practitioner (PV1/ORC)",
            ResourceType::DiagnosticReport => "DiagnosticReport (OBR)",
            ResourceType::All => "All Resources",
        }
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
enum ReverseResourceType {
    #[default]
    Patient,
    Observation,
    Encounter,
}

impl ReverseResourceType {
    fn label(&self) -> &str {
        match self {
            ReverseResourceType::Patient => "Patient -> PID",
            ReverseResourceType::Observation => "Observation -> OBX",
            ReverseResourceType::Encounter => "Encounter -> PV1",
        }
    }
}

impl Default for FhirTab {
    fn default() -> Self {
        Self {
            hl7_input: samples::ADT_A01.to_string(),
            fhir_output: String::new(),
            hl7_to_fhir_error: None,
            selected_resource: ResourceType::Patient,
            fhir_input: Self::sample_patient_fhir(),
            hl7_output: String::new(),
            fhir_to_hl7_error: None,
            reverse_resource: ReverseResourceType::Patient,
            active_direction: ConversionDirection::Hl7ToFhir,
            fhir_server_url: "https://localhost:8443/fhir/R4".to_string(),
            fhir_auth_type: FhirAuthType::None,
            fhir_username: String::new(),
            fhir_password: String::new(),
            fhir_api_key_header: "X-API-Key".to_string(),
            fhir_api_key: String::new(),
            fhir_bearer_token: String::new(),
            fhir_send_state: Arc::new(Mutex::new(FhirSendState::default())),
            fhir_runtime: tokio::runtime::Runtime::new().ok(),
        }
    }
}

impl FhirTab {
    fn sample_patient_fhir() -> String {
        r#"{
  "resourceType": "Patient",
  "id": "example",
  "identifier": [
    {
      "use": "official",
      "type": {
        "coding": [
          {
            "system": "http://terminology.hl7.org/CodeSystem/v2-0203",
            "code": "MR"
          }
        ]
      },
      "system": "urn:oid:1.2.3.4",
      "value": "12345678"
    }
  ],
  "name": [
    {
      "use": "official",
      "family": "DOE",
      "given": ["JOHN", "A"]
    }
  ],
  "gender": "male",
  "birthDate": "1980-01-15",
  "address": [
    {
      "use": "home",
      "line": ["123 MAIN ST"],
      "city": "SPRINGFIELD",
      "state": "IL",
      "postalCode": "62701",
      "country": "USA"
    }
  ]
}"#.to_string()
    }

    /// Set the HL7 input message content (used by File > Open)
    pub fn set_message(&mut self, content: String) {
        self.hl7_input = content;
        self.fhir_output.clear();
        self.hl7_to_fhir_error = None;
    }

    /// Get the current HL7 input message content (used by File > Save)
    pub fn get_message(&self) -> &str {
        &self.hl7_input
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("FHIR Converter");
        ui.label("Convert between HL7 v2.x messages and FHIR R4 resources.");
        ui.add_space(10.0);

        // Direction selector
        ui.horizontal(|ui| {
            ui.label("Conversion Direction:");
            ui.selectable_value(
                &mut self.active_direction,
                ConversionDirection::Hl7ToFhir,
                "HL7 v2.x -> FHIR R4"
            );
            ui.selectable_value(
                &mut self.active_direction,
                ConversionDirection::FhirToHl7,
                "FHIR R4 -> HL7 v2.x"
            );
        });

        ui.add_space(10.0);

        match self.active_direction {
            ConversionDirection::Hl7ToFhir => self.hl7_to_fhir_ui(ui),
            ConversionDirection::FhirToHl7 => self.fhir_to_hl7_ui(ui),
        }
    }

    fn hl7_to_fhir_ui(&mut self, ui: &mut egui::Ui) {
        // Controls
        ui.horizontal(|ui| {
            ui.label("Resource Type:");
            egui::ComboBox::from_id_salt("resource_type")
                .selected_text(self.selected_resource.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.selected_resource, ResourceType::Patient, "Patient (PID)");
                    ui.selectable_value(&mut self.selected_resource, ResourceType::Observation, "Observation (OBX)");
                    ui.selectable_value(&mut self.selected_resource, ResourceType::Encounter, "Encounter (PV1)");
                    ui.selectable_value(&mut self.selected_resource, ResourceType::Practitioner, "Practitioner");
                    ui.selectable_value(&mut self.selected_resource, ResourceType::DiagnosticReport, "DiagnosticReport (OBR)");
                    ui.selectable_value(&mut self.selected_resource, ResourceType::All, "All Resources");
                });

            ui.separator();

            if ui.button(RichText::new("Convert to FHIR").strong()).clicked() {
                self.convert_to_fhir();
            }

            // Sample loader
            ui.separator();
            ui.label("Sample:");
            let samples = samples::get_sample_messages();
            egui::ComboBox::from_id_salt("fhir_sample")
                .selected_text("Load...")
                .show_ui(ui, |ui| {
                    for (name, _, msg) in &samples {
                        if ui.selectable_label(false, *name).clicked() {
                            self.hl7_input = msg.to_string();
                            self.fhir_output.clear();
                            self.hl7_to_fhir_error = None;
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
                // Left: HL7 Input
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());

                            ui.heading("HL7 v2.x Input");

                            egui::ScrollArea::vertical()
                                .id_salt("fhir_hl7_input")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.hl7_input)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(28)
                                            .code_editor()
                                    );
                                });
                        });
                });

                // Right: FHIR Output
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());

                            ui.heading("FHIR R4 Output (JSON)");

                            if let Some(ref error) = self.hl7_to_fhir_error {
                                ui.colored_label(Color32::RED, format!("Error: {}", error));
                            }

                            if !self.fhir_output.is_empty() {
                                ui.horizontal(|ui| {
                                    if ui.button("Copy to Clipboard").clicked() {
                                        ui.ctx().copy_text(self.fhir_output.clone());
                                    }
                                });
                            }

                            // Send to FHIR Server section
                            ui.add_space(5.0);
                            ui.collapsing("Send to FHIR Server", |ui| {
                                let is_sending = {
                                    let state = self.fhir_send_state.lock().unwrap();
                                    state.sending
                                };

                                ui.horizontal(|ui| {
                                    ui.label("URL:");
                                    ui.add_enabled(
                                        !is_sending,
                                        egui::TextEdit::singleline(&mut self.fhir_server_url)
                                            .desired_width(250.0),
                                    );
                                });

                                ui.horizontal(|ui| {
                                    ui.label("Auth:");
                                    ui.add_enabled_ui(!is_sending, |ui| {
                                        egui::ComboBox::from_id_salt("fhir_auth_type")
                                            .selected_text(match self.fhir_auth_type {
                                                FhirAuthType::None => "None",
                                                FhirAuthType::Basic => "Basic",
                                                FhirAuthType::ApiKey => "API Key",
                                                FhirAuthType::Bearer => "Bearer",
                                            })
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut self.fhir_auth_type, FhirAuthType::None, "None");
                                                ui.selectable_value(&mut self.fhir_auth_type, FhirAuthType::Basic, "Basic Auth");
                                                ui.selectable_value(&mut self.fhir_auth_type, FhirAuthType::ApiKey, "API Key");
                                                ui.selectable_value(&mut self.fhir_auth_type, FhirAuthType::Bearer, "Bearer Token");
                                            });
                                    });

                                    match self.fhir_auth_type {
                                        FhirAuthType::Basic => {
                                            ui.label("User:");
                                            ui.add_enabled(
                                                !is_sending,
                                                egui::TextEdit::singleline(&mut self.fhir_username).desired_width(80.0),
                                            );
                                            ui.label("Pass:");
                                            ui.add_enabled(
                                                !is_sending,
                                                egui::TextEdit::singleline(&mut self.fhir_password)
                                                    .password(true)
                                                    .desired_width(80.0),
                                            );
                                        }
                                        FhirAuthType::ApiKey => {
                                            ui.label("Header:");
                                            ui.add_enabled(
                                                !is_sending,
                                                egui::TextEdit::singleline(&mut self.fhir_api_key_header)
                                                    .desired_width(80.0),
                                            );
                                            ui.label("Key:");
                                            ui.add_enabled(
                                                !is_sending,
                                                egui::TextEdit::singleline(&mut self.fhir_api_key)
                                                    .password(true)
                                                    .desired_width(150.0),
                                            );
                                        }
                                        FhirAuthType::Bearer => {
                                            ui.label("Token:");
                                            ui.add_enabled(
                                                !is_sending,
                                                egui::TextEdit::singleline(&mut self.fhir_bearer_token)
                                                    .password(true)
                                                    .desired_width(200.0),
                                            );
                                        }
                                        FhirAuthType::None => {}
                                    }
                                });

                                ui.horizontal(|ui| {
                                    if is_sending {
                                        ui.spinner();
                                        ui.label("Sending...");
                                    } else if ui.add_enabled(
                                        !self.fhir_output.is_empty(),
                                        egui::Button::new(RichText::new("Send FHIR Resource").strong()),
                                    ).clicked() {
                                        self.send_fhir_resource();
                                    }
                                });

                                // Show response
                                let state = self.fhir_send_state.lock().unwrap();
                                if let Some(ref error) = state.error {
                                    ui.colored_label(Color32::RED, format!("Error: {}", error));
                                }
                                if let Some(ref resp) = state.response {
                                    let status_color = if resp.status_code >= 200 && resp.status_code < 300 {
                                        Color32::GREEN
                                    } else {
                                        Color32::RED
                                    };
                                    ui.colored_label(
                                        status_color,
                                        format!("Status: {} {}", resp.status_code, resp.status_text),
                                    );
                                    if !resp.body.is_empty() {
                                        ui.label("Response:");
                                        let body_preview = resp.body.clone();
                                        ui.add(
                                            egui::TextEdit::multiline(&mut body_preview.as_str())
                                                .font(egui::TextStyle::Monospace)
                                                .desired_width(f32::INFINITY)
                                                .desired_rows(4)
                                                .interactive(false),
                                        );
                                    }
                                }
                            });

                            ui.add_space(5.0);

                            egui::ScrollArea::vertical()
                                .id_salt("fhir_output")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.fhir_output.as_str())
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(20)
                                            .interactive(false)
                                    );
                                });
                        });
                });
            });
    }

    fn fhir_to_hl7_ui(&mut self, ui: &mut egui::Ui) {
        // Controls
        ui.horizontal(|ui| {
            ui.label("Resource Type:");
            egui::ComboBox::from_id_salt("reverse_resource")
                .selected_text(self.reverse_resource.label())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.reverse_resource, ReverseResourceType::Patient, "Patient -> PID");
                    ui.selectable_value(&mut self.reverse_resource, ReverseResourceType::Observation, "Observation -> OBX");
                    ui.selectable_value(&mut self.reverse_resource, ReverseResourceType::Encounter, "Encounter -> PV1");
                });

            ui.separator();

            if ui.button(RichText::new("Convert to HL7").strong()).clicked() {
                self.convert_to_hl7();
            }

            if ui.button("Load Sample").clicked() {
                self.fhir_input = Self::sample_patient_fhir();
            }
        });

        ui.add_space(10.0);

        // Get available height for full-height panels
        let available_height = ui.available_height();

        StripBuilder::new(ui)
            .size(Size::relative(0.5).at_least(350.0))
            .size(Size::remainder().at_least(350.0))
            .horizontal(|mut strip| {
                // Left: FHIR Input
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());

                            ui.heading("FHIR R4 Input (JSON)");

                            egui::ScrollArea::vertical()
                                .id_salt("fhir_input")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.fhir_input)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(28)
                                            .code_editor()
                                    );
                                });
                        });
                });

                // Right: HL7 Output
                strip.cell(|ui| {
                    let panel_height = available_height - 10.0;
                    egui::Frame::group(ui.style())
                        .show(ui, |ui| {
                            ui.set_height(panel_height);
                            ui.set_width(ui.available_width());

                            ui.heading("HL7 v2.x Output");

                            if let Some(ref error) = self.fhir_to_hl7_error {
                                ui.colored_label(Color32::RED, format!("Error: {}", error));
                            }

                            if !self.hl7_output.is_empty()
                                && ui.button("Copy to Clipboard").clicked() {
                                    ui.ctx().copy_text(self.hl7_output.clone());
                                }

                            egui::ScrollArea::vertical()
                                .id_salt("hl7_output")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::multiline(&mut self.hl7_output.as_str())
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .desired_rows(28)
                                            .interactive(false)
                                    );
                                });
                        });
                });
            });
    }

    fn convert_to_fhir(&mut self) {
        self.fhir_output.clear();
        self.hl7_to_fhir_error = None;

        // Parse the HL7 message
        let normalized = self.hl7_input
            .replace("\r\n", "\r")
            .replace("\n", "\r");

        let message = match parse_message(&normalized) {
            Ok(m) => m,
            Err(e) => {
                self.hl7_to_fhir_error = Some(format!("Parse error: {}", e));
                return;
            }
        };

        let mut results = Vec::new();

        match self.selected_resource {
            ResourceType::Patient => {
                match PatientConverter::convert(&message) {
                    Ok(patient) => {
                        if let Ok(json) = serde_json::to_string_pretty(&patient) {
                            results.push(json);
                        }
                    }
                    Err(e) => {
                        self.hl7_to_fhir_error = Some(format!("Patient conversion: {}", e));
                        return;
                    }
                }
            }
            ResourceType::Observation => {
                match ObservationConverter::convert_all(&message) {
                    Ok(observations) => {
                        for obs in observations {
                            if let Ok(json) = serde_json::to_string_pretty(&obs) {
                                results.push(json);
                            }
                        }
                    }
                    Err(e) => {
                        self.hl7_to_fhir_error = Some(format!("Observation conversion: {}", e));
                        return;
                    }
                }
            }
            ResourceType::Encounter => {
                match EncounterConverter::convert(&message) {
                    Ok(encounter) => {
                        if let Ok(json) = serde_json::to_string_pretty(&encounter) {
                            results.push(json);
                        }
                    }
                    Err(e) => {
                        self.hl7_to_fhir_error = Some(format!("Encounter conversion: {}", e));
                        return;
                    }
                }
            }
            ResourceType::Practitioner => {
                // Try different practitioner extraction methods
                if let Ok(pract) = PractitionerConverter::convert_attending_doctor(&message)
                    && let Ok(json) = serde_json::to_string_pretty(&pract) {
                        results.push(format!("// Attending Doctor\n{}", json));
                    }
                if let Ok(pract) = PractitionerConverter::convert_referring_doctor(&message)
                    && let Ok(json) = serde_json::to_string_pretty(&pract) {
                        results.push(format!("// Referring Doctor\n{}", json));
                    }
                if results.is_empty() {
                    self.hl7_to_fhir_error = Some("No practitioners found in message".to_string());
                    return;
                }
            }
            ResourceType::DiagnosticReport => {
                match DiagnosticReportConverter::convert_all(&message) {
                    Ok(reports) => {
                        for report in reports {
                            if let Ok(json) = serde_json::to_string_pretty(&report) {
                                results.push(json);
                            }
                        }
                    }
                    Err(e) => {
                        self.hl7_to_fhir_error = Some(format!("DiagnosticReport conversion: {}", e));
                        return;
                    }
                }
            }
            ResourceType::All => {
                // Convert all available resources
                if let Ok(patient) = PatientConverter::convert(&message)
                    && let Ok(json) = serde_json::to_string_pretty(&patient) {
                        results.push(format!("// Patient\n{}", json));
                    }
                if let Ok(encounter) = EncounterConverter::convert(&message)
                    && let Ok(json) = serde_json::to_string_pretty(&encounter) {
                        results.push(format!("// Encounter\n{}", json));
                    }
                if let Ok(observations) = ObservationConverter::convert_all(&message) {
                    for (i, obs) in observations.iter().enumerate() {
                        if let Ok(json) = serde_json::to_string_pretty(&obs) {
                            results.push(format!("// Observation {}\n{}", i + 1, json));
                        }
                    }
                }
            }
        }

        if results.is_empty() {
            self.hl7_to_fhir_error = Some("No resources could be extracted".to_string());
        } else {
            self.fhir_output = results.join("\n\n");
        }
    }

    fn convert_to_hl7(&mut self) {
        self.hl7_output.clear();
        self.fhir_to_hl7_error = None;

        match self.reverse_resource {
            ReverseResourceType::Patient => {
                match serde_json::from_str::<vyna_hl7_fhir::resources::patient::Patient>(&self.fhir_input) {
                    Ok(patient) => {
                        match PatientReverseConverter::convert(&patient) {
                            Ok(segment) => {
                                let delimiters = Delimiters::default();
                                self.hl7_output = segment.encode(&delimiters);
                            }
                            Err(e) => {
                                self.fhir_to_hl7_error = Some(format!("Conversion error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.fhir_to_hl7_error = Some(format!("JSON parse error: {}", e));
                    }
                }
            }
            ReverseResourceType::Observation => {
                match serde_json::from_str::<vyna_hl7_fhir::resources::observation::Observation>(&self.fhir_input) {
                    Ok(obs) => {
                        match ObservationReverseConverter::convert(&obs, 1) {
                            Ok(segment) => {
                                let delimiters = Delimiters::default();
                                self.hl7_output = segment.encode(&delimiters);
                            }
                            Err(e) => {
                                self.fhir_to_hl7_error = Some(format!("Conversion error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.fhir_to_hl7_error = Some(format!("JSON parse error: {}", e));
                    }
                }
            }
            ReverseResourceType::Encounter => {
                match serde_json::from_str::<vyna_hl7_fhir::resources::encounter::Encounter>(&self.fhir_input) {
                    Ok(encounter) => {
                        match EncounterReverseConverter::convert(&encounter) {
                            Ok(segment) => {
                                let delimiters = Delimiters::default();
                                self.hl7_output = segment.encode(&delimiters);
                            }
                            Err(e) => {
                                self.fhir_to_hl7_error = Some(format!("Conversion error: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        self.fhir_to_hl7_error = Some(format!("JSON parse error: {}", e));
                    }
                }
            }
        }
    }

    fn send_fhir_resource(&mut self) {
        let state = self.fhir_send_state.clone();

        // Clear previous results
        {
            let mut s = state.lock().unwrap();
            s.sending = true;
            s.response = None;
            s.error = None;
        }

        let request = FhirSendRequest {
            url: self.fhir_server_url.clone(),
            body: self.fhir_output.clone(),
            auth_type: self.fhir_auth_type,
            username: self.fhir_username.clone(),
            password: self.fhir_password.clone(),
            api_key_header: self.fhir_api_key_header.clone(),
            api_key: self.fhir_api_key.clone(),
            bearer_token: self.fhir_bearer_token.clone(),
        };

        if let Some(ref runtime) = self.fhir_runtime {
            runtime.spawn(async move {
                let result = send_fhir_request(request).await;
                let mut s = state.lock().unwrap();
                s.sending = false;
                match result {
                    Ok(resp) => s.response = Some(resp),
                    Err(e) => s.error = Some(e),
                }
            });
        }
    }
}

/// Parameters for sending a FHIR resource to a server.
struct FhirSendRequest {
    url: String,
    body: String,
    auth_type: FhirAuthType,
    username: String,
    password: String,
    api_key_header: String,
    api_key: String,
    bearer_token: String,
}

/// Send a FHIR resource to a FHIR server via HTTP/HTTPS POST
async fn send_fhir_request(request: FhirSendRequest) -> Result<FhirSendResponse, String> {
    let FhirSendRequest {
        url,
        body,
        auth_type,
        username,
        password,
        api_key_header,
        api_key,
        bearer_token,
    } = request;
    let url = url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut req = client
        .post(url)
        .header("Content-Type", "application/fhir+json")
        .header("Accept", "application/fhir+json");

    match auth_type {
        FhirAuthType::Basic if !username.is_empty() => {
            req = req.basic_auth(username, Some(password));
        }
        FhirAuthType::ApiKey if !api_key.is_empty() => {
            let header_name = if api_key_header.is_empty() {
                "X-API-Key"
            } else {
                api_key_header.as_str()
            };
            req = req.header(header_name, api_key);
        }
        FhirAuthType::Bearer if !bearer_token.is_empty() => {
            req = req.bearer_auth(bearer_token);
        }
        _ => {}
    }

    let response = req
        .body(body)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status_code = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("")
        .to_string();

    let resp_body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(FhirSendResponse {
        status_code,
        status_text,
        body: resp_body,
    })
}
