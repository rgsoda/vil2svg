//! SVG emission: keymap-drawer's `draw/` package, minus glyphs and ISO enter.

use crate::config::{DrawConfig, SVG_STYLE};
use crate::keymap::{Combo, Keymap, LayoutKey};
use crate::layout::{PhysicalKey, PhysicalLayout, Point};
use std::collections::HashSet;
use std::fmt::Write;

/// Python's `round()`: nearest integer, ties to even.
fn py_round(x: f64) -> i64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 && (r as i64) % 2 != 0 {
        r as i64 - x.signum() as i64
    } else {
        r as i64
    }
}

/// Python's `repr()` of a float, for the few floats that reach the output.
fn py_float(x: f64) -> String {
    let s = format!("{x}");
    if s.contains('.') || s.contains('e') {
        s
    } else {
        format!("{s}.0")
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn str_to_id(val: &str) -> String {
    if val.is_empty() {
        return "o_o".to_string();
    }
    let val = val.replace(' ', "-");
    let trimmed: String = val
        .chars()
        .skip_while(|c| !c.is_ascii_alphabetic())
        .collect();
    if trimmed.is_empty() {
        return "x_x".to_string();
    }
    trimmed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'))
        .collect()
}

fn class_str(classes: &[&str]) -> String {
    let live: Vec<&str> = classes.iter().copied().filter(|c| !c.is_empty()).collect();
    if classes.is_empty() {
        String::new()
    } else {
        format!(" class=\"{}\"", live.join(" "))
    }
}

pub struct Drawer<'a> {
    cfg: &'a DrawConfig,
    layout: &'a PhysicalLayout,
    layer_names: HashSet<String>,
    out: String,
}

impl<'a> Drawer<'a> {
    pub fn new(cfg: &'a DrawConfig, layout: &'a PhysicalLayout) -> Self {
        Self {
            cfg,
            layout,
            layer_names: HashSet::new(),
            out: String::new(),
        }
    }

    // --- low-level text helpers (draw/utils.py) ---

    /// Split a legend into lines, wrapping on word boundaries and truncating.
    fn split_text(&self, text: &str, truncate: usize, line_width: usize) -> Vec<String> {
        // do not split on double spaces, but do split on single
        let mut lines: Vec<String> = text
            .replace("  ", "\x00")
            .split_whitespace()
            .map(|w| w.replace('\x00', " "))
            .collect();

        if line_width > 0 && lines.len() < truncate {
            let mut wrapped: Vec<String> = Vec::new();
            let mut gave_up = false;
            for (i, line) in lines.iter().enumerate() {
                if line.chars().count() > line_width {
                    let pieces = wrap_words(line, line_width);
                    let new_total = wrapped.len() + pieces.len() - 1 + lines.len() - i;
                    let diff = new_total as isize - truncate as isize;
                    if diff > 0 {
                        let diff = diff as usize;
                        if diff < pieces.len() {
                            // salvage part of this line as much as we can
                            let keep = pieces.len() - diff - 1;
                            wrapped.extend(pieces[..keep].iter().cloned());
                            wrapped.push(pieces[keep..].concat());
                        } else {
                            wrapped.push(line.clone());
                        }
                        wrapped.extend(lines[i + 1..].iter().cloned());
                        gave_up = true;
                        break;
                    }
                    wrapped.extend(pieces);
                } else {
                    wrapped.push(line.clone());
                }
            }
            let _ = gave_up;
            lines = wrapped;
        }

        if truncate > 0 && lines.len() > truncate {
            lines.truncate(truncate - 1);
            lines.push("…".to_string());
        }
        lines
    }

    fn get_scaling(&self, width: usize) -> String {
        let limit = self.cfg.shrink_wide_legends;
        if limit == 0 || width <= limit {
            return String::new();
        }
        let pct = f64::max(60.0, 100.0 * limit as f64 / width as f64);
        format!(" style=\"font-size: {pct:.0}%\"")
    }

    fn truncate_word(&self, word: &str) -> String {
        let limit = (1.7 * self.cfg.shrink_wide_legends as f64) as usize;
        if self.cfg.shrink_wide_legends == 0 || word.chars().count() <= limit {
            return word.to_string();
        }
        let kept: String = word.chars().take(limit - 1).collect();
        format!("{kept}…")
    }

    fn draw_rect(&mut self, p: Point, dims: Point, radii: Point, classes: &[&str]) {
        let _ = writeln!(
            self.out,
            "<rect rx=\"{}\" ry=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>",
            py_round(radii.x),
            py_round(radii.y),
            py_round(p.x - dims.x / 2.0),
            py_round(p.y - dims.y / 2.0),
            py_round(dims.x),
            py_round(dims.y),
            class_str(classes)
        );
    }

    fn draw_key_rect(&mut self, dims: Point, classes: &[&str]) {
        let radii = Point::new(self.cfg.key_rx, self.cfg.key_ry);
        self.draw_rect(Point::new(0.0, 0.0), dims, radii, classes);
    }

    fn draw_text(&mut self, p: Point, word: &str, classes: &[&str]) {
        if word.is_empty() {
            return;
        }
        let word = self.truncate_word(word);
        let scale = self.get_scaling(word.chars().count());
        let _ = write!(
            self.out,
            "<text x=\"{}\" y=\"{}\"{}>",
            py_round(p.x),
            py_round(p.y),
            class_str(classes)
        );
        if scale.is_empty() {
            self.out.push_str(&escape(&word));
        } else {
            let _ = write!(self.out, "<tspan{scale}>{}</tspan>", escape(&word));
        }
        self.out.push_str("</text>\n");
    }

    fn draw_textblock(&mut self, p: Point, words: &[String], classes: &[&str], shift: f64) {
        let words: Vec<String> = words.iter().map(|w| self.truncate_word(w)).collect();
        let _ = writeln!(
            self.out,
            "<text x=\"{}\" y=\"{}\"{}>",
            py_round(p.x),
            py_round(p.y),
            class_str(classes)
        );
        let dy_0 = (words.len() - 1) as f64 * (self.cfg.line_spacing * (1.0 + shift / 2.0) / 2.0);
        let widest = words.iter().map(|w| w.chars().count()).max().unwrap_or(0);
        let scaling = self.get_scaling(widest);
        let _ = write!(
            self.out,
            "<tspan x=\"{}\" dy=\"-{}em\"{scaling}>{}</tspan>",
            py_round(p.x),
            py_float((dy_0 * 100.0).round() / 100.0),
            escape(&words[0])
        );
        for word in &words[1..] {
            let _ = write!(
                self.out,
                "<tspan x=\"{}\" dy=\"{}em\"{scaling}>{}</tspan>",
                py_round(p.x),
                py_float(self.cfg.line_spacing),
                escape(word)
            );
        }
        self.out.push_str("\n</text>\n");
    }

    fn draw_legend(
        &mut self,
        p: Point,
        words: &[String],
        classes: &[&str],
        legend_type: &str,
        shift: f64,
    ) {
        if words.is_empty() || (words.len() == 1 && words[0].is_empty()) {
            return;
        }
        let layer_name = words.join(" ");
        let is_layer = self.cfg.style_layer_activators && self.layer_names.contains(&layer_name);

        let mut classes: Vec<&str> = classes.to_vec();
        classes.push(legend_type);
        if is_layer {
            classes.push("layer-activator");
        }

        if is_layer {
            let _ = writeln!(self.out, "<a href=\"#{}\">", str_to_id(&layer_name));
        }
        if words.len() == 1 {
            let w = words[0].clone();
            self.draw_text(p, &w, &classes);
        } else {
            self.draw_textblock(p, words, &classes, shift);
        }
        if is_layer {
            self.out.push_str("</a>");
        }
    }

    // --- keys and layers (draw/draw.py) ---

    fn print_layer_header(&mut self, p: Point, header: &str) {
        let text = if self.cfg.append_colon_to_layer_header {
            format!("{header}:")
        } else {
            header.to_string()
        };
        let _ = writeln!(
            self.out,
            "<text x=\"{}\" y=\"{}\" class=\"label\" id=\"{}\">{}</text>",
            py_round(p.x),
            py_round(p.y),
            str_to_id(header),
            escape(&text)
        );
    }

    fn print_key(&mut self, p_key: &PhysicalKey, l_key: &LayoutKey, key_ind: usize) {
        let (p, w, h) = (p_key.pos, p_key.width, p_key.height);
        let pos_class = format!("keypos-{key_ind}");
        let _ = writeln!(
            self.out,
            "<g transform=\"translate({}, {})\"{}>",
            py_round(p.x),
            py_round(p.y),
            class_str(&["key", &l_key.key_type, &pos_class])
        );

        self.draw_key_rect(
            Point::new(
                w - 2.0 * self.cfg.inner_pad_w,
                h - 2.0 * self.cfg.inner_pad_h,
            ),
            &["key", &l_key.key_type],
        );

        let tap_words = self.split_text(&l_key.tap, 3, self.cfg.shrink_wide_legends);

        // auto-adjust vertical alignment up/down if there are two lines and
        // either hold/shifted is present
        let mut shift = 0.0;
        if tap_words.len() == 2 {
            if !l_key.shifted.is_empty() && l_key.hold.is_empty() {
                shift = -1.0; // shift down
            } else if !l_key.hold.is_empty() && l_key.shifted.is_empty() {
                shift = 1.0; // shift up
            }
        }

        let tap_shift = Point::new(self.cfg.legend_rel_x, self.cfg.legend_rel_y);
        let x_offset = w / 2.0 - self.cfg.inner_pad_w - self.cfg.small_pad;
        let y_offset = h / 2.0 - self.cfg.inner_pad_h - self.cfg.small_pad;

        let classes: Vec<&str> = vec!["key", &l_key.key_type];
        let classes: Vec<String> = classes.into_iter().map(str::to_string).collect();
        let cls: Vec<&str> = classes.iter().map(String::as_str).collect();

        self.draw_legend(tap_shift, &tap_words, &cls, "tap", shift);
        for (point, text, kind) in [
            (Point::new(0.0, y_offset), &l_key.hold, "hold"),
            (Point::new(0.0, -y_offset), &l_key.shifted, "shifted"),
            (Point::new(-x_offset, 0.0), &l_key.left, "left"),
            (Point::new(x_offset, 0.0), &l_key.right, "right"),
            (Point::new(-x_offset, -y_offset), &l_key.tl, "tl"),
            (Point::new(x_offset, -y_offset), &l_key.tr, "tr"),
            (Point::new(-x_offset, y_offset), &l_key.bl, "bl"),
            (Point::new(x_offset, y_offset), &l_key.br, "br"),
        ] {
            self.draw_legend(point, std::slice::from_ref(text), &cls, kind, 0.0);
        }

        self.out.push_str("</g>\n");
    }
}

/// Word-wrap used by `split_text`.
///
/// Mirrors Python's `TextWrapper._wrap_chunks` with the settings upstream
/// uses (`break_long_words=False`, `drop_whitespace=True`): fill greedily,
/// never split a chunk, and drop whitespace that lands at a line break.
fn wrap_words(line: &str, width: usize) -> Vec<String> {
    let mut chunks: Vec<String> = split_chunks(line);
    chunks.reverse(); // pop() takes from the front of the original order
    let mut lines: Vec<String> = Vec::new();

    while !chunks.is_empty() {
        // drop leading whitespace on every line but the first
        if !lines.is_empty()
            && let Some(last) = chunks.last()
                && last.trim().is_empty() {
                    chunks.pop();
                }

        let mut cur: Vec<String> = Vec::new();
        let mut cur_len = 0usize;
        while let Some(chunk) = chunks.last() {
            let l = chunk.chars().count();
            if cur_len + l > width {
                break;
            }
            cur_len += l;
            cur.push(chunks.pop().expect("just peeked"));
        }

        // a chunk longer than the whole line goes on a line of its own
        if let Some(chunk) = chunks.last()
            && chunk.chars().count() > width && cur.is_empty() {
                cur.push(chunks.pop().expect("just peeked"));
            }

        // drop trailing whitespace before emitting
        if cur.last().is_some_and(|c| c.trim().is_empty()) {
            cur.pop();
        }
        if !cur.is_empty() {
            lines.push(cur.concat());
        } else if chunks.is_empty() {
            break;
        }
    }
    lines
}

/// Approximates Python's `re.split(r"(?<!^.)\b", line)`: break at word
/// boundaries, never immediately after the first character.
fn split_chunks(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut start = 0usize;
    for i in 1..chars.len() {
        if i == 1 {
            continue; // the (?<!^.) guard
        }
        let prev = chars[i - 1].is_alphanumeric() || chars[i - 1] == '_';
        let cur = chars[i].is_alphanumeric() || chars[i] == '_';
        if prev != cur {
            out.push(chars[start..i].iter().collect());
            start = i;
        }
    }
    out.push(chars[start..].iter().collect());
    out
}

// --- combos (draw/combo.py) ---

impl<'a> Drawer<'a> {
    fn draw_line_dendron(&mut self, p1: Point, p2: Point, shorten: f64) {
        let mut diff = p2 - p1;
        let magn = (diff.x * diff.x + diff.y * diff.y).sqrt();
        if shorten != 0.0 && shorten < magn {
            diff = diff * (1.0 - shorten / magn);
        }
        let _ = writeln!(
            self.out,
            "<path d=\"M{},{} l{},{}\" class=\"combo\"/>",
            py_round(p1.x),
            py_round(p1.y),
            py_round(diff.x),
            py_round(diff.y)
        );
    }

    /// Draw one combo box, returning its bounding box corners.
    fn print_combo(&mut self, combo: &Combo, combo_ind: usize) -> (Point, Point) {
        let p_keys: Vec<PhysicalKey> = combo
            .positions
            .iter()
            .filter_map(|i| self.layout.keys.get(*i).cloned())
            .collect();
        if p_keys.is_empty() {
            return (Point::new(0.0, 0.0), Point::new(0.0, 0.0));
        }
        let (width, height) = (self.cfg.combo_w, self.cfg.combo_h);

        // find center of combo box; vil2svg always uses the default "mid" align
        let sum = p_keys
            .iter()
            .fold(Point::new(0.0, 0.0), |acc, k| acc + k.pos);
        let p = sum * (1.0 / p_keys.len() as f64);

        let pos_class = format!("combopos-{combo_ind}");
        let _ = writeln!(
            self.out,
            "<g{}>",
            class_str(&["combo", &combo.key.key_type, &pos_class])
        );

        // dendrons from the box out to each triggering key
        for k in &p_keys {
            let d = k.pos - p;
            if (d.x * d.x + d.y * d.y).sqrt() >= k.width - 1.0 {
                self.draw_line_dendron(p, k.pos, k.width / 3.0);
            }
        }

        let radii = Point::new(self.cfg.key_rx, self.cfg.key_ry);
        let cls_owned = ["combo".to_string(), combo.key.key_type.clone()];
        let cls: Vec<&str> = cls_owned.iter().map(String::as_str).collect();
        self.draw_rect(p, Point::new(width, height), radii, &cls);

        let words = self.split_text(&combo.key.tap, 2, self.cfg.shrink_wide_legends);
        self.draw_legend(p, &words, &cls, "tap", 0.0);
        let dy = self.cfg.combo_h / 2.0 - self.cfg.small_pad;
        let dx = self.cfg.combo_w / 2.0 - self.cfg.small_pad;
        for (point, text, kind) in [
            (p + Point::new(0.0, dy), &combo.key.hold, "hold"),
            (p - Point::new(0.0, dy), &combo.key.shifted, "shifted"),
            (p - Point::new(dx, 0.0), &combo.key.left, "left"),
            (p + Point::new(dx, 0.0), &combo.key.right, "right"),
        ] {
            self.draw_legend(point, std::slice::from_ref(text), &cls, kind, 0.0);
        }

        self.out.push_str("</g>\n");

        let dims = Point::new(width / 2.0, height / 2.0);
        (p - dims, p + dims)
    }

    fn print_combos_for_layer(&mut self, combos: &[&Combo]) -> (Option<f64>, Option<f64>) {
        let mut min_y: Option<f64> = None;
        let mut max_y: Option<f64> = None;
        for (ind, combo) in combos.iter().enumerate() {
            let (tl, br) = self.print_combo(combo, ind);
            min_y = Some(min_y.map_or(tl.y, |v: f64| v.min(tl.y)));
            max_y = Some(max_y.map_or(br.y, |v: f64| v.max(br.y)));
        }
        (min_y, max_y)
    }
}

/// A layer ready to draw: a name and its keys, with the physical layout to use.
struct DrawLayer<'k> {
    name: String,
    keys: Vec<LayoutKey>,
    combos: Vec<&'k Combo>,
}

impl<'a> Drawer<'a> {
    fn print_layers(
        &mut self,
        start: Point,
        layout: &PhysicalLayout,
        layers: &[DrawLayer],
        n_cols: usize,
        draw_header: bool,
        pad_divisor: f64,
    ) -> Point {
        let outer_pad_w = (self.cfg.outer_pad_w / pad_divisor).floor();
        let mut p = start + Point::new(self.cfg.outer_pad_w - outer_pad_w, 0.0);
        let original_x = p.x;
        let col_width = layout.width() + 2.0 * outer_pad_w;
        let mut max_height = 0.0f64;

        for (ind, layer) in layers.iter().enumerate() {
            let outer_pad_h = if ind > n_cols - 1 {
                (self.cfg.outer_pad_h / pad_divisor).floor()
            } else {
                self.cfg.outer_pad_h
            };

            let _ = writeln!(
                self.out,
                "<g transform=\"translate({}, {})\" class=\"layer-{}\">",
                py_round(p.x + outer_pad_w),
                py_round(p.y),
                escape(&layer.name)
            );

            if draw_header {
                self.print_layer_header(Point::new(0.0, outer_pad_h / 2.0), &layer.name);
            }

            // draw keys and combos into a temp buffer, so the whole group can be
            // shifted down by however far the combos stick out above the board
            let saved = std::mem::take(&mut self.out);
            for (key_ind, (p_key, l_key)) in
                layout.keys.iter().zip(layer.keys.iter()).enumerate()
            {
                self.print_key(p_key, l_key, key_ind);
            }
            let (min_y, max_y) = self.print_combos_for_layer(&layer.combos);
            let body = std::mem::replace(&mut self.out, saved);

            let top_y = min_y.map_or(0.0, |v| v.min(0.0));
            let bottom_y = max_y.map_or(layout.height(), |v| v.max(layout.height()));

            let _ = writeln!(
                self.out,
                "<g transform=\"translate(0, {})\">",
                py_round(outer_pad_h - top_y)
            );
            self.out.push_str(&body);
            self.out.push_str("</g>\n");
            self.out.push_str("</g>\n");

            max_height = max_height.max(bottom_y - top_y);

            if ind % n_cols == n_cols - 1 || ind == layers.len() - 1 {
                p = Point::new(original_x, p.y + outer_pad_h + max_height);
                max_height = 0.0;
            } else {
                p = p + Point::new(col_width, 0.0);
            }
        }

        Point::new(original_x + col_width * n_cols as f64, p.y)
    }

    /// Render the whole board and return the finished SVG document.
    pub fn print_board(&mut self, keymap: &Keymap, draw_layers: &[String]) -> String {
        let layers: Vec<DrawLayer> = keymap
            .layers
            .iter()
            .filter(|l| draw_layers.is_empty() || draw_layers.contains(&l.name))
            .map(|l| DrawLayer {
                name: l.name.clone(),
                keys: l.keys.clone(),
                combos: keymap
                    .combos_for_layer(&l.name)
                    .into_iter()
                    .filter(|c| !c.draw_separate)
                    .collect(),
            })
            .collect();

        self.layer_names = layers.iter().map(|l| l.name.clone()).collect();

        let layout = self.layout;
        let mut p = self.print_layers(
            Point::new(0.0, 0.0),
            layout,
            &layers,
            self.cfg.n_columns,
            true,
            1.0,
        );

        // separate combo diagrams, each its own mini board
        let separate: Vec<&Combo> = keymap
            .combos
            .iter()
            .filter(|c| c.draw_separate && (draw_layers.is_empty() || draw_layers.contains(&c.layer)))
            .collect();
        let shrunk;
        if !separate.is_empty() {
            let scale = self.cfg.combo_diagrams_scale as f64;
            let min_h = layout
                .keys
                .iter()
                .map(|k| k.height)
                .fold(f64::INFINITY, f64::min);
            let min_w = layout
                .keys
                .iter()
                .map(|k| k.width)
                .fold(f64::INFINITY, f64::min);

            let mut keys = vec![PhysicalKey {
                pos: Point::new(min_w / 2.0, min_h / 2.0),
                width: min_w,
                height: min_h,
            }];
            let shifted = layout.scaled(1.0 / scale);
            keys.extend(shifted.keys.iter().map(|k| PhysicalKey {
                pos: k.pos + Point::new(0.0, min_h + self.cfg.inner_pad_h),
                width: k.width,
                height: k.height,
            }));
            shrunk = PhysicalLayout { keys };

            let combo_layers: Vec<DrawLayer> = separate
                .iter()
                .enumerate()
                .map(|(ind, combo)| {
                    let mut header = combo.key.clone();
                    // upstream joins with a space even when the type is empty,
                    // which shows up as a double space in the class attribute
                    header.key_type = format!("{} combo-separate", header.key_type);
                    let mut keys = vec![header];
                    let mut empty = vec![LayoutKey::default(); layout.len()];
                    for pos in &combo.positions {
                        if let Some(k) = empty.get_mut(*pos) {
                            k.key_type = "held".to_string();
                        }
                    }
                    keys.extend(empty);
                    DrawLayer {
                        name: format!("combopos-{ind}"),
                        keys,
                        combos: Vec::new(),
                    }
                })
                .collect();

            self.print_layer_header(
                Point::new(self.cfg.outer_pad_w, p.y + self.cfg.outer_pad_h / 2.0),
                "Combos",
            );
            let scale_i = self.cfg.combo_diagrams_scale;
            p = self.print_layers(
                Point::new(0.0, p.y),
                &shrunk,
                &combo_layers,
                self.cfg.n_columns * scale_i,
                false,
                scale_i as f64,
            );
        }

        let board_w = py_round(p.x);
        let board_h = py_round(p.y + self.cfg.outer_pad_h);
        let mut doc = String::new();
        let _ = writeln!(
            doc,
            "<svg width=\"{board_w}\" height=\"{board_h}\" viewBox=\"0 0 {board_w} {board_h}\" \
             class=\"keymap\" xmlns=\"http://www.w3.org/2000/svg\" \
             xmlns:xlink=\"http://www.w3.org/1999/xlink\">"
        );
        let extra = if self.cfg.svg_extra_style.is_empty() {
            String::new()
        } else {
            format!("\n{}", self.cfg.svg_extra_style)
        };
        let _ = writeln!(doc, "<style>{SVG_STYLE}{extra}</style>");
        doc.push_str(&self.out);
        doc.push_str("</svg>\n");
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn py_round_breaks_ties_to_even() {
        // Rust's f64::round() would give 1, 3, -1 here; Python gives 0, 2, -0
        assert_eq!(py_round(0.5), 0);
        assert_eq!(py_round(1.5), 2);
        assert_eq!(py_round(2.5), 2);
        assert_eq!(py_round(-0.5), 0);
        assert_eq!(py_round(-1.5), -2);
        // ordinary cases are unaffected
        assert_eq!(py_round(0.4), 0);
        assert_eq!(py_round(0.6), 1);
        assert_eq!(py_round(-2.7), -3);
    }

    #[test]
    fn py_float_keeps_a_decimal_point() {
        assert_eq!(py_float(1.2), "1.2");
        assert_eq!(py_float(0.6), "0.6");
        assert_eq!(py_float(0.0), "0.0");
    }

    #[test]
    fn str_to_id_strips_leading_non_letters() {
        assert_eq!(str_to_id("Base"), "Base");
        assert_eq!(str_to_id("Vol +"), "Vol-");
        assert_eq!(str_to_id(""), "o_o");
        assert_eq!(str_to_id("123"), "x_x");
        assert_eq!(str_to_id("1st layer"), "st-layer");
    }

    #[test]
    fn class_str_drops_empty_components_but_keeps_the_attribute() {
        assert_eq!(class_str(&["key", "", "keypos-0"]), " class=\"key keypos-0\"");
        assert_eq!(class_str(&[]), "");
        // upstream's combo-separate quirk: a leading space survives
        assert_eq!(
            class_str(&["key", " combo-separate"]),
            " class=\"key  combo-separate\""
        );
    }

    #[test]
    fn escape_handles_xml_metacharacters() {
        assert_eq!(escape("a & b"), "a &amp; b");
        assert_eq!(escape("<x>"), "&lt;x&gt;");
    }

    #[test]
    fn split_chunks_never_breaks_after_the_first_character() {
        // the (?<!^.) guard in upstream's regex
        assert_eq!(split_chunks("a-b"), vec!["a-", "b"]);
        assert_eq!(split_chunks("Vol +"), vec!["Vol", " +"]);
        assert_eq!(split_chunks("Bootloader"), vec!["Bootloader"]);
    }

    #[test]
    fn wrap_words_fills_greedily() {
        assert_eq!(wrap_words("EEPROM reset", 7), vec!["EEPROM", "reset"]);
        assert_eq!(wrap_words("short", 7), vec!["short"]);
    }
}
