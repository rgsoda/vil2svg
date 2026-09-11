//! `--print`: black on white, on a single A4 page turned to suit the drawing.

const PRINT_CSS: &str = r#"
/* --- vil2svg --print: pure black on white --- */
svg.keymap { fill: #000000; }
rect.key, rect.combo, rect.combo-separate {
    fill: #ffffff; stroke: #000000; stroke-width: 1.2;
}
rect.held, rect.combo.held { fill: #ffffff; stroke-width: 3; }
rect.ghost, rect.combo.ghost { fill: #ffffff; stroke-dasharray: 4,4; }
rect.side { filter: none; fill: #ffffff; }
text, text.label, text.combo, text.hold, text.shifted { fill: #000000; }
text.label { stroke: #ffffff; }
"#;

pub fn make_printable(svg: &str, pad: i64) -> Result<String, String> {
    let header_end = svg
        .find('>')
        .ok_or_else(|| "unexpected SVG header, cannot apply --print".to_string())?;
    let header = &svg[..=header_end];
    let (w, h) = parse_dims(header)
        .ok_or_else(|| "unexpected SVG header, cannot apply --print".to_string())?;

    let vb_w = w + 2 * pad;
    let vb_h = h + 2 * pad;
    // one layer alone is far wider than it is tall, and would sit as a thin
    // strip on a portrait page; turn the paper instead of shrinking the keys
    let (page_w, page_h) = if vb_w > vb_h {
        ("297mm", "210mm")
    } else {
        ("210mm", "297mm")
    };

    let root = format!(
        "<svg width=\"{page_w}\" height=\"{page_h}\" viewBox=\"{} {} {vb_w} {vb_h}\" \
         preserveAspectRatio=\"xMidYMid meet\" class=\"keymap\" \
         xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">\
         <rect x=\"{}\" y=\"{}\" width=\"{vb_w}\" height=\"{vb_h}\" fill=\"#ffffff\"/>",
        -pad, -pad, -pad, -pad
    );

    let body = format!("{root}{}", &svg[header_end + 1..]);
    Ok(body.replacen("</style>", &format!("{PRINT_CSS}</style>"), 1))
}

/// Pull `width` and `height` out of the opening `<svg ...>` tag.
fn parse_dims(header: &str) -> Option<(i64, i64)> {
    let get = |name: &str| -> Option<i64> {
        let needle = format!("{name}=\"");
        let start = header.find(&needle)? + needle.len();
        let rest = &header[start..];
        let end = rest.find('"')?;
        rest[..end].parse().ok()
    };
    Some((get("width")?, get("height")?))
}
