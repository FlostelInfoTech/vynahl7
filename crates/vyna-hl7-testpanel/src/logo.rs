//! Logo loading and rendering for VynaHL7 branding

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// The embedded SVG icon data (VynaHL7 mark)
const ICON_SVG: &str = include_str!("../assets/vyna-icon.svg");

/// Cached logo textures, one per theme
static LOGO_TEXTURE_LIGHT: OnceCell<Arc<LogoData>> = OnceCell::new();
static LOGO_TEXTURE_DARK: OnceCell<Arc<LogoData>> = OnceCell::new();

/// Logo data including the rendered image
///
/// Holds one theme-resolved render of the embedded SVG icon as raw
/// RGBA bytes, ready to upload as an egui texture. A 1x1 transparent
/// image marks the render-failed fallback (see [`get_logo_texture`]).
pub struct LogoData {
    /// Rendered image width in pixels.
    pub width: u32,
    /// Rendered image height in pixels.
    pub height: u32,
    /// Row-major RGBA bytes (`width * height * 4` entries).
    pub pixels: Vec<u8>,
}

/// Get or create the logo texture for the current theme
pub fn get_logo_texture(ctx: &egui::Context) -> Option<TextureHandle> {
    let dark = ctx.theme() == egui::Theme::Dark;
    let cache = if dark {
        &LOGO_TEXTURE_DARK
    } else {
        &LOGO_TEXTURE_LIGHT
    };
    // Try to render the SVG to pixels - use larger size for better quality
    let logo_data = cache.get_or_init(|| {
        Arc::new(render_themed_icon(dark).unwrap_or_else(|| LogoData {
            width: 1,
            height: 1,
            pixels: vec![0, 0, 0, 0],
        }))
    });

    if logo_data.width <= 1 {
        return None;
    }

    // Create the texture
    let image = ColorImage::from_rgba_unmultiplied(
        [logo_data.width as usize, logo_data.height as usize],
        &logo_data.pixels,
    );

    let name = if dark {
        "vyna_logo_dark"
    } else {
        "vyna_logo_light"
    };
    Some(ctx.load_texture(name, image, TextureOptions::LINEAR))
}

/// Render the icon with `currentColor` resolved for the theme.
///
/// The mark's base uses `currentColor`, which resvg would render black;
/// navy on light backgrounds and white on dark keeps it legible in both.
fn render_themed_icon(dark: bool) -> Option<LogoData> {
    let base = if dark { "#FFFFFF" } else { "#0A1F44" };
    let svg = ICON_SVG.replace("currentColor", base);
    render_svg_to_pixels(&svg, 128, 128)
}

/// Render an SVG string to RGBA pixels
fn render_svg_to_pixels(svg_data: &str, width: u32, height: u32) -> Option<LogoData> {
    // Create a font database with system fonts for text rendering
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    // Set fallback fonts - use Arial which is available on Windows
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_serif_family("Times New Roman");

    // Parse the SVG with font database in options
    // Use Arial as default font family for any unresolved fonts
    let options = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        font_family: "Arial".to_string(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg_data, &options).ok()?;

    // Create a pixmap to render into
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    // Calculate the transform to fit the SVG into the target size
    let svg_size = tree.size();
    let scale_x = width as f32 / svg_size.width();
    let scale_y = height as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    let transform = tiny_skia::Transform::from_scale(scale, scale);

    // Render the SVG
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert to RGBA bytes
    let pixels: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| [p.red(), p.green(), p.blue(), p.alpha()])
        .collect();

    Some(LogoData {
        width,
        height,
        pixels,
    })
}

/// Display the project logo in the UI
/// Returns true if the logo was displayed, false if fallback text should be used
pub fn show_logo(ui: &mut egui::Ui, size: f32) -> bool {
    if let Some(texture) = get_logo_texture(ui.ctx()) {
        // Display the texture at the requested size
        ui.image(egui::load::SizedTexture::new(
            texture.id(),
            egui::vec2(size, size),
        ));
        true
    } else {
        // Fallback to text logo
        show_text_logo(ui, size);
        false
    }
}

/// Display a text-based logo (fallback when SVG loading fails)
pub fn show_text_logo(ui: &mut egui::Ui, size: f32) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("V")
                .size(size)
                .strong()
                .color(Color32::from_rgb(0, 128, 128)),
        );
        ui.add_space(-size * 0.25);
        ui.label(
            egui::RichText::new("7")
                .size(size * 0.875)
                .strong()
                .color(Color32::from_rgb(255, 107, 0)),
        );
    });
}

/// Display the project name
pub fn show_company_name(ui: &mut egui::Ui, size: f32) {
    ui.label(
        egui::RichText::new("VynaHL7")
            .size(size)
            .strong()
            .color(Color32::from_rgb(0, 128, 128)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_renders_for_both_themes() {
        let light = render_themed_icon(false).expect("light icon must render");
        let dark = render_themed_icon(true).expect("dark icon must render");
        assert_eq!((light.width, light.height), (128, 128));
        // Non-trivial content: some pixels lit, some transparent.
        assert!(light.pixels.iter().any(|&b| b != 0));
        assert!(dark.pixels.iter().any(|&b| b != 0));
        // currentColor resolves differently per theme.
        assert_ne!(light.pixels, dark.pixels);
    }

    #[test]
    fn favicon_renders_with_opaque_tile() {
        const FAVICON_SVG: &str = include_str!("../assets/favicon.svg");
        let data = render_svg_to_pixels(FAVICON_SVG, 64, 64).expect("favicon must render");
        assert_eq!((data.width, data.height), (64, 64));
        // Navy rounded-square tile: center pixel fully opaque.
        let center = (32 * 64 + 32) * 4;
        assert_eq!(data.pixels[center + 3], 255);
    }
}
