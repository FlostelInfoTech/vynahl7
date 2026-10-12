//! Build script for vyna-hl7-testpanel
//!
//! Keeps `assets/vyna_hl7.ico` fresh from the favicon SVG on every platform,
//! and embeds the icon into the executable on Windows.

fn main() {
    use std::path::Path;

    // Generate the ICO file from SVG if it doesn't exist or is older than the SVG
    let svg_path = Path::new("assets/favicon.svg");
    let ico_path = Path::new("assets/vyna_hl7.ico");

    let should_generate = if ico_path.exists() {
        // Check if SVG is newer than ICO
        let svg_modified = svg_path.metadata().and_then(|m| m.modified()).ok();
        let ico_modified = ico_path.metadata().and_then(|m| m.modified()).ok();
        match (svg_modified, ico_modified) {
            (Some(svg_time), Some(ico_time)) => svg_time > ico_time,
            _ => true,
        }
    } else {
        true
    };

    if should_generate
        && let Err(e) = generate_ico_from_svg(svg_path, ico_path)
    {
        eprintln!("Warning: Failed to generate ICO from SVG: {}", e);
    }

    #[cfg(windows)]
    {
        // Embed the icon into the Windows executable
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/vyna_hl7.ico");
        res.set("ProductName", "VynaHL7 Test Panel");
        res.set("FileDescription", "VynaHL7 HL7 v2.x Test Panel GUI");
        res.set("LegalCopyright", "Copyright © FLOSTEL INFOTECH");
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }

    // Tell cargo to re-run the build script if the SVG changes
    println!("cargo::rerun-if-changed=assets/favicon.svg");
}

fn generate_ico_from_svg(
    svg_path: &std::path::Path,
    ico_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    let svg_data = fs::read_to_string(svg_path)?;

    // Parse the SVG
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();

    // Use Windows-compatible fallback fonts
    // Arial and Segoe UI are standard on Windows
    fontdb.set_sans_serif_family("Arial");
    fontdb.set_serif_family("Times New Roman");

    let options = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        font_family: "Arial".to_string(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg_data, &options)?;

    // ICO files typically contain multiple sizes, we'll include common ones
    let sizes = [16u32, 32, 48, 64, 128, 256];
    let mut images: Vec<(u32, Vec<u8>)> = Vec::new();

    for &size in &sizes {
        if let Some(pixmap) = render_svg_to_rgba(&tree, size, size) {
            images.push((size, pixmap));
        }
    }

    if images.is_empty() {
        return Err("Failed to render any icon sizes".into());
    }

    // Write ICO file
    let mut ico_file = fs::File::create(ico_path)?;
    write_ico(&mut ico_file, &images)?;

    Ok(())
}

fn render_svg_to_rgba(tree: &usvg::Tree, width: u32, height: u32) -> Option<Vec<u8>> {
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;

    let svg_size = tree.size();
    let scale_x = width as f32 / svg_size.width();
    let scale_y = height as f32 / svg_size.height();
    let scale = scale_x.min(scale_y);

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(tree, transform, &mut pixmap.as_mut());

    // Convert to RGBA bytes
    Some(
        pixmap
            .pixels()
            .iter()
            .flat_map(|p| [p.red(), p.green(), p.blue(), p.alpha()])
            .collect(),
    )
}

fn write_ico<W: std::io::Write>(writer: &mut W, images: &[(u32, Vec<u8>)]) -> std::io::Result<()> {
    // ICO Header
    writer.write_all(&[0, 0])?; // Reserved
    writer.write_all(&[1, 0])?; // Type: 1 = ICO
    writer.write_all(&(images.len() as u16).to_le_bytes())?; // Number of images

    // Calculate offsets
    let header_size = 6 + (images.len() * 16);
    let mut data_offset = header_size;

    // Write directory entries
    for (size, rgba) in images {
        let size_byte = if *size >= 256 { 0 } else { *size as u8 };
        let bmp_size = 40 + rgba.len(); // BITMAPINFOHEADER + pixel data

        writer.write_all(&[size_byte])?; // Width
        writer.write_all(&[size_byte])?; // Height
        writer.write_all(&[0])?; // Color palette (0 = no palette)
        writer.write_all(&[0])?; // Reserved
        writer.write_all(&1u16.to_le_bytes())?; // Color planes
        writer.write_all(&32u16.to_le_bytes())?; // Bits per pixel
        writer.write_all(&(bmp_size as u32).to_le_bytes())?; // Size of image data
        writer.write_all(&(data_offset as u32).to_le_bytes())?; // Offset to image data

        data_offset += bmp_size;
    }

    // Write image data (as BMP without file header)
    for (size, rgba) in images {
        // BITMAPINFOHEADER
        writer.write_all(&40u32.to_le_bytes())?; // Header size
        writer.write_all(&(*size as i32).to_le_bytes())?; // Width
        writer.write_all(&((*size as i32) * 2).to_le_bytes())?; // Height (doubled for ICO)
        writer.write_all(&1u16.to_le_bytes())?; // Planes
        writer.write_all(&32u16.to_le_bytes())?; // Bits per pixel
        writer.write_all(&0u32.to_le_bytes())?; // Compression
        writer.write_all(&(rgba.len() as u32).to_le_bytes())?; // Image size
        writer.write_all(&0u32.to_le_bytes())?; // X pixels per meter
        writer.write_all(&0u32.to_le_bytes())?; // Y pixels per meter
        writer.write_all(&0u32.to_le_bytes())?; // Colors used
        writer.write_all(&0u32.to_le_bytes())?; // Important colors

        // Write pixel data (BMP is bottom-up, and uses BGRA)
        let row_size = *size as usize;
        for y in (0..row_size).rev() {
            for x in 0..row_size {
                let idx = (y * row_size + x) * 4;
                let r = rgba[idx];
                let g = rgba[idx + 1];
                let b = rgba[idx + 2];
                let a = rgba[idx + 3];
                writer.write_all(&[b, g, r, a])?; // BGRA order
            }
        }
    }

    Ok(())
}
