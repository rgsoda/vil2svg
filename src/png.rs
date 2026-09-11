//! Rasterising the finished SVG, so `--png` needs no external converter.

use resvg::tiny_skia;
use resvg::usvg;

/// Render `svg` at `scale`x its nominal size. Fonts come from the system;
/// the stylesheet asks for a monospace family and falls back to whatever
/// the machine calls monospace.
pub fn render(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    fontdb.set_monospace_family("Liberation Mono");

    let mut opts = usvg::Options::default();
    opts.fontdb = std::sync::Arc::new(fontdb);

    let tree = usvg::Tree::from_str(svg, &opts).map_err(|e| format!("could not parse the SVG: {e}"))?;
    let size = tree.size().to_int_size().scale_by(scale).ok_or("scale produced an empty image")?;
    let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| format!("could not allocate a {}x{} image", size.width(), size.height()))?;
    // the keymap SVG has no background of its own in screen mode
    pixmap.fill(tiny_skia::Color::WHITE);
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|e| format!("could not encode the PNG: {e}"))
}
