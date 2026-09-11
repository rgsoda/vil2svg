//! The keymap data model: what each key shows, and where combos sit.

/// One key's legends in one layer. Mirrors keymap-drawer's `LayoutKey`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutKey {
    pub tap: String,
    pub hold: String,
    pub shifted: String,
    pub left: String,
    pub right: String,
    pub tl: String,
    pub tr: String,
    pub bl: String,
    pub br: String,
    /// CSS class: "", "trans", "held", "ghost".
    pub key_type: String,
}

impl LayoutKey {
    pub fn tap(tap: impl Into<String>) -> Self {
        Self {
            tap: tap.into(),
            ..Default::default()
        }
    }

    pub fn tap_hold(tap: impl Into<String>, hold: impl Into<String>) -> Self {
        Self {
            tap: tap.into(),
            hold: hold.into(),
            ..Default::default()
        }
    }

    /// keymap-drawer's `trans_legend` default.
    pub fn trans() -> Self {
        Self {
            tap: "▽".to_string(),
            key_type: "trans".to_string(),
            ..Default::default()
        }
    }

    /// Mark this key as one that is held down to reach the layer being drawn.
    ///
    /// A transparent key loses its legend entirely, and the type is assigned
    /// rather than appended — both match keymap-drawer's `add_held_keys`.
    pub fn mark_held(&mut self) {
        if *self == Self::trans() {
            *self = Self {
                key_type: "held".to_string(),
                ..Default::default()
            };
        } else {
            self.key_type = "held".to_string();
        }
    }
}

#[derive(Clone, Debug)]
pub struct Layer {
    pub name: String,
    pub keys: Vec<LayoutKey>,
}

#[derive(Clone, Debug)]
pub struct Combo {
    /// Key positions that trigger the combo.
    pub positions: Vec<usize>,
    /// What the combo emits.
    pub key: LayoutKey,
    /// Layer name this combo is drawn on.
    pub layer: String,
    pub draw_separate: bool,
}

#[derive(Clone, Debug)]
pub struct Keymap {
    pub layers: Vec<Layer>,
    pub combos: Vec<Combo>,
}

impl Keymap {
    pub fn combos_for_layer(&self, layer: &str) -> Vec<&Combo> {
        self.combos.iter().filter(|c| c.layer == layer).collect()
    }
}
