// Hide console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! VynaHL7 Test Panel - GUI Application for HL7 Message Testing
//!
//! A comprehensive GUI application demonstrating the capabilities of the VynaHL7 HL7 library.
//!
//! ## Features
//!
//! - **Parser**: Parse and analyze HL7 v2.x messages with tree view
//! - **Builder**: Visually build HL7 messages using the fluent API
//! - **Terser**: Access and modify fields using path notation
//! - **Validator**: Validate messages against HL7 schemas
//! - **MLLP**: Send and receive messages via MLLP protocol
//! - **FHIR**: Convert between HL7 v2.x and FHIR R4
//! - **XML**: Convert between ER7 and XML formats

mod app;
mod logo;
mod tabs;
mod samples;
mod utils;

use app::VynaHL7TestPanel;

fn main() -> eframe::Result<()> {
    // Configure logging for debug builds
    #[cfg(debug_assertions)]
    env_logger::init();

    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([1400.0, 900.0])
        .with_min_inner_size([900.0, 650.0])
        .with_title("VynaHL7 Test Panel - HL7 v2.x Testing Suite")
        .with_app_id("vyna-hl7-testpanel");

    // Load and set window icon
    let viewport = if let Some(icon) = load_window_icon() {
        viewport.with_icon(icon)
    } else {
        viewport
    };

    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "VynaHL7 Test Panel",
        options,
        Box::new(|cc| {
            // Set up custom fonts and style
            setup_custom_style(&cc.egui_ctx);
            Ok(Box::new(VynaHL7TestPanel::new(cc)))
        }),
    )
}

/// Load the window icon from the embedded favicon SVG
///
/// The favicon carries its own navy tile, so it stays legible on both
/// light and dark title bars without theme handling.
fn load_window_icon() -> Option<egui::IconData> {
    // Embed the SVG at compile time
    const FAVICON_SVG: &str = include_str!("../assets/favicon.svg");

    // Parse the SVG with system fonts
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_serif_family("Times New Roman");

    let options = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        font_family: "Arial".to_string(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(FAVICON_SVG, &options).ok()?;

    // Render at 64x64 for window icon
    let size = 64u32;
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;

    let svg_size = tree.size();
    let scale_x = size as f32 / svg_size.width();
    let scale_y = size as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert to RGBA bytes
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| [p.red(), p.green(), p.blue(), p.alpha()])
        .collect();

    Some(egui::IconData {
        rgba,
        width: size,
        height: size,
    })
}

fn setup_custom_style(ctx: &egui::Context) {
    // Set up high-contrast light theme
    let mut light_visuals = egui::Visuals::light();

    // White background for better contrast
    light_visuals.panel_fill = egui::Color32::WHITE;
    light_visuals.window_fill = egui::Color32::WHITE;
    light_visuals.extreme_bg_color = egui::Color32::WHITE;
    light_visuals.faint_bg_color = egui::Color32::from_gray(250);

    // Black text for maximum contrast
    light_visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::BLACK;
    light_visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_gray(30);
    light_visuals.widgets.hovered.fg_stroke.color = egui::Color32::BLACK;
    light_visuals.widgets.active.fg_stroke.color = egui::Color32::BLACK;

    // Slightly darker backgrounds for widgets to stand out
    light_visuals.widgets.noninteractive.bg_fill = egui::Color32::from_gray(245);
    light_visuals.widgets.inactive.bg_fill = egui::Color32::from_gray(240);
    light_visuals.widgets.hovered.bg_fill = egui::Color32::from_gray(230);
    light_visuals.widgets.active.bg_fill = egui::Color32::from_gray(220);

    // Stronger borders for better definition
    light_visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::from_gray(200);
    light_visuals.widgets.inactive.bg_stroke.color = egui::Color32::from_gray(180);
    light_visuals.widgets.hovered.bg_stroke.color = egui::Color32::from_gray(150);
    light_visuals.widgets.active.bg_stroke.color = egui::Color32::from_gray(100);

    // Selection highlight
    light_visuals.selection.bg_fill = egui::Color32::from_rgb(59, 130, 246).gamma_multiply(0.3);
    light_visuals.selection.stroke.color = egui::Color32::from_rgb(59, 130, 246);

    ctx.set_visuals_of(egui::Theme::Light, light_visuals);

    // Apply font, spacing, and corner tweaks to both themes, preserving
    // each theme's own visuals (including the custom light visuals above).
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let mut style = (*ctx.style_of(theme)).clone();

        // Use a slightly larger font for better readability
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(13.0, egui::FontFamily::Monospace),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(18.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(12.0, egui::FontFamily::Proportional),
        );

        // Increase spacing for cleaner look
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 5.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.indent = 20.0;

        // Improve visual appearance - use CornerRadius (renamed from Rounding in egui 0.33)
        style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4);
        style.visuals.window_corner_radius = egui::CornerRadius::same(8);

        ctx.set_style_of(theme, style);
    }
}
