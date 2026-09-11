//! Drawing configuration: the subset of keymap-drawer's `DrawConfig` that
//! vil2svg actually varies, with the rest pinned to its defaults.

/// keymap-drawer's default stylesheet, emitted verbatim so output stays
/// byte-identical to the Python pipeline.
pub const SVG_STYLE: &str = include_str!("svg_style.css");

/// Keys are narrow, so the default 14px tap legend crowds out the 11px hold and
/// double-tap ones. Level them off — the base letter is legible from position
/// alone, while the alternates are the part you actually need to look up.
pub const LEGEND_CSS: &str = "text { font-size: 12px; }\n\
text.combo, text.hold, text.shifted,\n\
text.left, text.right, text.tl, text.tr, text.bl, text.br { font-size: 12px; }\n";

pub struct DrawConfig {
    /// `1u` in pixels. QMK `info.json` layouts scale both axes by this.
    pub key_h: f64,
    pub combo_w: f64,
    pub combo_h: f64,
    pub key_rx: f64,
    pub key_ry: f64,
    pub n_columns: usize,
    pub separate_combo_diagrams: bool,
    pub combo_diagrams_scale: usize,
    pub inner_pad_w: f64,
    pub inner_pad_h: f64,
    pub outer_pad_w: f64,
    pub outer_pad_h: f64,
    pub line_spacing: f64,
    pub append_colon_to_layer_header: bool,
    pub small_pad: f64,
    pub legend_rel_x: f64,
    pub legend_rel_y: f64,
    pub shrink_wide_legends: usize,
    pub style_layer_activators: bool,
    pub svg_extra_style: String,
}

impl Default for DrawConfig {
    fn default() -> Self {
        // upstream's `key_w`, used only to derive the paddings below; QMK
        // layouts themselves are scaled uniformly by `key_h`
        let key_w = 60.0;
        let key_h = 56.0;
        Self {
            key_h,
            combo_w: key_w / 2.0 - 2.0,
            combo_h: key_h / 2.0 - 2.0,
            key_rx: 6.0,
            key_ry: 6.0,
            n_columns: 1,
            separate_combo_diagrams: false,
            combo_diagrams_scale: 2,
            inner_pad_w: 2.0,
            inner_pad_h: 2.0,
            outer_pad_w: key_w / 2.0,
            outer_pad_h: key_h,
            line_spacing: 1.2,
            append_colon_to_layer_header: true,
            small_pad: 2.0,
            legend_rel_x: 0.0,
            legend_rel_y: 0.0,
            shrink_wide_legends: 7,
            style_layer_activators: true,
            svg_extra_style: String::new(),
        }
    }
}

impl DrawConfig {
    /// The equivalent of the Python script's cached `draw-config.yaml`.
    pub fn for_vil2svg(combos_separate: bool) -> Self {
        let mut cfg = Self {
            svg_extra_style: LEGEND_CSS.to_string(),
            ..Self::default()
        };
        if combos_separate {
            // in-place labels sit in the gap between keys and crowd the legends;
            // this gives each combo its own mini diagram
            cfg.separate_combo_diagrams = true;
            cfg.combo_diagrams_scale = 3;
        }
        cfg
    }
}
