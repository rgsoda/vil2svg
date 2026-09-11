//! Reading a Vial `.vil` file into the drawing model.

use crate::keymap::{Combo, Keymap, Layer, LayoutKey};
use crate::legend::{grid_legend, grid_tap_hold, key_legend, wrapped_arg};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// Number of keys in LAYOUT_split_3x6_3.
pub const N_KEYS: usize = 42;

/// A matrix cell. Vial writes `-1` for positions the board does not have.
#[derive(Deserialize)]
#[serde(untagged)]
enum Cell {
    Code(String),
    Unused(#[allow(dead_code)] i64),
}

impl Cell {
    fn as_code(&self) -> &str {
        match self {
            Cell::Code(s) => s,
            Cell::Unused(_) => "KC_NO",
        }
    }
}

#[derive(Deserialize)]
struct VilFile {
    layout: Vec<Vec<Vec<Cell>>>,
    #[serde(default)]
    tap_dance: Vec<Vec<serde_json::Value>>,
    #[serde(default)]
    combo: Vec<Vec<String>>,
}

/// One Vial layer's matrix -> the 42 keys in LAYOUT_split_3x6_3 order.
fn flatten(layer: &[Vec<Cell>]) -> Result<Vec<String>, String> {
    if layer.len() < 8 || layer.iter().take(8).any(|r| r.len() < 6) {
        return Err(format!(
            "expected an 8x6 Vial matrix, got {}x{}",
            layer.len(),
            layer.first().map_or(0, |r| r.len())
        ));
    }
    let mut flat: Vec<String> = Vec::with_capacity(N_KEYS);
    for r in 0..3 {
        // left, outer -> inner
        flat.extend(layer[r][..6].iter().map(|c| c.as_code().to_string()));
        // right, mirrored
        flat.extend(layer[4 + r][..6].iter().rev().map(|c| c.as_code().to_string()));
    }
    for c in [3, 4, 5] {
        flat.push(layer[3][c].as_code().to_string()); // left thumbs
    }
    for c in [5, 4, 3] {
        flat.push(layer[7][c].as_code().to_string()); // right thumbs
    }
    Ok(flat)
}

/// A tap-dance entry: tap, hold, double-tap, tap-hold, timeout.
struct TapDance {
    tap: String,
    hold: String,
    double: String,
}

fn tap_dance_table(raw: &[Vec<serde_json::Value>]) -> Vec<TapDance> {
    raw.iter()
        .map(|e| {
            let s = |i: usize| {
                e.get(i)
                    .and_then(|v| v.as_str())
                    .unwrap_or("KC_NO")
                    .to_string()
            };
            TapDance {
                tap: s(0),
                hold: s(1),
                double: s(2),
            }
        })
        .collect()
}

/// The mod-tap prefixes Vial emits, e.g. `LSFT_T(KC_S)`.
fn mod_tap(code: &str) -> Option<(&str, &str)> {
    let open = code.find('(')?;
    let name = &code[..open];
    let modifier = name.strip_suffix("_T")?;
    let inner = code[open + 1..].strip_suffix(')')?;
    Some((modifier, inner))
}

/// `LT1(KC_A)` -> (1, "KC_A"). Vial's spelling of QMK's `LT(1,KC_A)`.
fn layer_tap(code: &str) -> Option<(usize, &str)> {
    let rest = code.strip_prefix("LT")?;
    let open = rest.find('(')?;
    let n: usize = rest[..open].parse().ok()?;
    let inner = rest[open + 1..].strip_suffix(')')?;
    Some((n, inner))
}

/// Resolve a tap dance that only really does one thing, as the Python did.
fn collapse_tap_dance(td: &TapDance) -> Option<String> {
    let mut parts = vec![td.tap.clone()];
    if td.double != "KC_NO" {
        parts.push(td.double.clone());
    }
    if td.hold != "KC_NO" {
        parts.push(td.hold.clone());
    }
    if parts.len() == 1 {
        Some(parts.remove(0))
    } else {
        None
    }
}

struct Converter<'a> {
    tap_dance: &'a [TapDance],
    layer_names: &'a [String],
    /// layer index -> key positions held to reach it
    activated_from: HashMap<usize, HashSet<usize>>,
}

impl<'a> Converter<'a> {
    fn layer_legend(&self, n: usize) -> String {
        self.layer_names
            .get(n)
            .cloned()
            .unwrap_or_else(|| format!("L{n}"))
    }

    /// Record that `pos` is held down to reach `to_layer`, transitively.
    /// Mirrors keymap-drawer's `update_layer_activated_from`; first
    /// activator wins, and reverse activations are ignored.
    fn note_activation(&mut self, from_layer: usize, to_layer: usize, pos: usize) {
        if from_layer >= to_layer {
            return; // ignore reverse layer order activation
        }
        if self.activated_from.contains_key(&to_layer) {
            return; // already have a way to get to this layer
        }
        let mut set = HashSet::new();
        set.insert(pos);
        // also consider how the layer we are coming from got activated
        if let Some(prev) = self.activated_from.get(&from_layer) {
            set.extend(prev.iter().copied());
        }
        self.activated_from.insert(to_layer, set);
    }

    fn convert(&mut self, code: &str, layer: usize, pos: usize) -> LayoutKey {
        if code == "KC_NO" {
            return LayoutKey::default();
        }
        if code == "KC_TRNS" || code == "KC_TRANSPARENT" {
            return LayoutKey::trans();
        }

        // tap dance: tap in the middle, double-tap above, hold below
        if let Some(arg) = wrapped_arg(code, "TD")
            && let Ok(i) = arg.parse::<usize>()
                && let Some(td) = self.tap_dance.get(i) {
                    if let Some(single) = collapse_tap_dance(td) {
                        return self.convert(&single, layer, pos);
                    }
                    let tap = key_legend(&td.tap).unwrap_or_default();
                    let double = key_legend(&td.double);
                    let hold = key_legend(&td.hold)
                        // a dance that holds what it taps says nothing
                        .filter(|h| *h != tap);
                    return LayoutKey {
                        tap,
                        shifted: double.map(|d| format!("×2 {d}")).unwrap_or_default(),
                        hold: hold.unwrap_or_default(),
                        ..Default::default()
                    };
                }

        // layer-tap: tap the key, hold for the layer
        if let Some((n, inner)) = layer_tap(code) {
            self.note_activation(layer, n, pos);
            let inner_key = self.convert(inner, layer, pos);
            return LayoutKey {
                tap: inner_key.tap,
                hold: self.layer_legend(n),
                shifted: inner_key.shifted,
                ..Default::default()
            };
        }

        // momentary and toggled layer switches
        for kind in ["MO", "TO", "TG", "DF", "TT", "OSL"] {
            if let Some(arg) = wrapped_arg(code, kind)
                && let Ok(n) = arg.parse::<usize>() {
                    let tap = self.layer_legend(n);
                    return match kind {
                        "MO" => {
                            self.note_activation(layer, n, pos);
                            LayoutKey::tap(tap)
                        }
                        "OSL" => {
                            self.note_activation(layer, n, pos);
                            LayoutKey::tap_hold(tap, "sticky")
                        }
                        // `tap-toggle` is rewritten to `Toggle` by LEGEND_FIX
                        "TT" => {
                            self.note_activation(layer, n, pos);
                            LayoutKey::tap_hold(tap, "Toggle")
                        }
                        // TG/TO/DF just switch, they are not held
                        _ => LayoutKey::tap_hold(tap, "toggle"),
                    };
                }
        }

        // mod-tap: tap the key, hold the modifier
        if let Some((modifier, inner)) = mod_tap(code) {
            let inner_key = self.convert(inner, layer, pos);
            return LayoutKey {
                tap: inner_key.tap,
                hold: grid_legend(modifier),
                shifted: inner_key.shifted,
                ..Default::default()
            };
        }

        // a few keycodes carry a hold legend of their own (e.g. QK_GESC)
        if let Some((tap, hold)) = grid_tap_hold(code) {
            return LayoutKey::tap_hold(tap, hold);
        }
        LayoutKey::tap(grid_legend(code))
    }
}

/// Name each layer, marking ones that are entirely transparent/empty.
fn layer_names(layers: &[Vec<String>]) -> Vec<String> {
    const DEFAULTS: [&str; 4] = ["Base", "Nav", "Num", "Fn"];
    layers
        .iter()
        .enumerate()
        .map(|(i, layer)| {
            let live = layer.iter().any(|k| k != "KC_NO" && k != "KC_TRNS");
            if !live {
                format!("Empty{i}")
            } else if i < DEFAULTS.len() {
                DEFAULTS[i].to_string()
            } else {
                format!("L{i}")
            }
        })
        .collect()
}

pub struct Parsed {
    pub keymap: Keymap,
    /// Combos whose triggers could not be located, for a warning.
    pub skipped_combos: Vec<String>,
}

pub fn parse(path: &std::path::Path, combos_separate: bool) -> Result<Parsed, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let vil: VilFile = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;

    let tap_dance = tap_dance_table(&vil.tap_dance);
    let raw_layers: Vec<Vec<String>> = vil
        .layout
        .iter()
        .map(|l| flatten(l))
        .collect::<Result<_, _>>()?;
    for raw in &raw_layers {
        if raw.len() != N_KEYS {
            return Err(format!(
                "expected a {N_KEYS}-key 3x6+3 matrix, got {} keys",
                raw.len()
            ));
        }
    }

    let names = layer_names(&raw_layers);
    let mut conv = Converter {
        tap_dance: &tap_dance,
        layer_names: &names,
        activated_from: HashMap::new(),
    };

    let mut layers: Vec<Layer> = Vec::with_capacity(raw_layers.len());
    for (i, raw) in raw_layers.iter().enumerate() {
        let keys = raw
            .iter()
            .enumerate()
            .map(|(pos, code)| conv.convert(code, i, pos))
            .collect();
        layers.push(Layer {
            name: names[i].clone(),
            keys,
        });
    }

    // mark the keys held down to reach each layer
    let activated = std::mem::take(&mut conv.activated_from);
    for (layer_idx, positions) in activated {
        if let Some(layer) = layers.get_mut(layer_idx) {
            for pos in positions {
                if let Some(key) = layer.keys.get_mut(pos) {
                    key.mark_held();
                }
            }
        }
    }

    let (combos, skipped_combos) =
        build_combos(&vil.combo, &raw_layers, &names, &tap_dance, combos_separate);

    Ok(Parsed {
        keymap: Keymap { layers, combos },
        skipped_combos,
    })
}

/// Vial's combo table -> drawable combos.
///
/// Vial stores combos as keycodes, not positions, so each trigger is looked up
/// in the layers to find where it sits. A combo is pinned to the first layer
/// holding all of its triggers, which is the base layer in almost every case.
fn build_combos(
    table: &[Vec<String>],
    raw_layers: &[Vec<String>],
    names: &[String],
    tap_dance: &[TapDance],
    separate: bool,
) -> (Vec<Combo>, Vec<String>) {
    let mut combos = Vec::new();
    let mut skipped = Vec::new();
    for (slot, row) in table.iter().enumerate() {
        if row.len() < 5 || row[4] == "KC_NO" {
            continue;
        }
        let triggers: Vec<&String> = row[..4].iter().filter(|k| *k != "KC_NO").collect();
        let Some(output) = key_legend(&row[4]) else {
            continue;
        };
        if triggers.is_empty() {
            continue;
        }

        let found = raw_layers.iter().enumerate().find_map(|(i, raw)| {
            let positions: Option<Vec<usize>> = triggers
                .iter()
                .map(|t| find_trigger(raw, t, tap_dance))
                .collect::<Option<Vec<_>>>();
            positions.map(|p| (i, p))
        });

        match found {
            Some((i, positions)) => combos.push(Combo {
                positions,
                key: LayoutKey::tap(output),
                layer: names[i].clone(),
                // in-place labels sit in the gap between keys and crowd the
                // legends; this gives each combo its own mini diagram
                draw_separate: separate,
            }),
            None => skipped.push(format!(
                "combo {slot} ({} -> {})",
                triggers
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("+"),
                row[4]
            )),
        }
    }
    (combos, skipped)
}

/// Where a combo trigger keycode sits in a layer.
///
/// A trigger may be bound directly, or sit inside a tap dance, mod-tap or
/// layer-tap. Vial fires the combo in all of those cases, so look inside the
/// wrappers rather than matching the keycode string literally. A direct
/// binding always wins over a wrapped one.
fn find_trigger(raw: &[String], trigger: &str, tap_dance: &[TapDance]) -> Option<usize> {
    if let Some(i) = raw.iter().position(|k| k == trigger) {
        return Some(i);
    }
    raw.iter()
        .position(|k| tapped_keycode(k, tap_dance).as_deref() == Some(trigger))
}

/// The keycode a key emits when simply tapped, seeing through wrappers.
fn tapped_keycode(code: &str, tap_dance: &[TapDance]) -> Option<String> {
    if let Some(arg) = wrapped_arg(code, "TD") {
        let td = tap_dance.get(arg.parse::<usize>().ok()?)?;
        return tapped_keycode(&td.tap, tap_dance);
    }
    if let Some((_, inner)) = layer_tap(code) {
        return tapped_keycode(inner, tap_dance);
    }
    if let Some((_, inner)) = mod_tap(code) {
        return tapped_keycode(inner, tap_dance);
    }
    if code == "KC_NO" {
        return None;
    }
    Some(code.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tap_dances() -> Vec<TapDance> {
        vec![
            // TD(0): taps N, holds ], double-taps N
            TapDance { tap: "KC_N".into(), hold: "KC_RBRACKET".into(), double: "KC_N".into() },
            // TD(1): a dance that only ever taps -- collapses to a plain key
            TapDance { tap: "KC_Q".into(), hold: "KC_NO".into(), double: "KC_NO".into() },
        ]
    }

    #[test]
    fn tapped_keycode_sees_through_wrappers() {
        let td = tap_dances();
        assert_eq!(tapped_keycode("KC_A", &td).as_deref(), Some("KC_A"));
        assert_eq!(tapped_keycode("TD(0)", &td).as_deref(), Some("KC_N"));
        assert_eq!(tapped_keycode("LSFT_T(KC_S)", &td).as_deref(), Some("KC_S"));
        assert_eq!(tapped_keycode("LT1(KC_A)", &td).as_deref(), Some("KC_A"));
        assert_eq!(tapped_keycode("KC_NO", &td), None);
    }

    #[test]
    fn find_trigger_prefers_a_direct_binding() {
        let td = tap_dances();
        let raw: Vec<String> = ["TD(0)", "KC_M", "KC_N"].iter().map(|s| s.to_string()).collect();
        // KC_N is bound directly at 2 and wrapped at 0; the direct one wins
        assert_eq!(find_trigger(&raw, "KC_N", &td), Some(2));
        assert_eq!(find_trigger(&raw, "KC_M", &td), Some(1));
    }

    #[test]
    fn find_trigger_resolves_through_a_tap_dance() {
        let td = tap_dances();
        let raw: Vec<String> = ["TD(0)", "KC_M"].iter().map(|s| s.to_string()).collect();
        // this is the case the Python missed entirely
        assert_eq!(find_trigger(&raw, "KC_N", &td), Some(0));
    }

    #[test]
    fn find_trigger_reports_a_genuinely_absent_key() {
        let td = tap_dances();
        let raw: Vec<String> = ["TD(0)", "KC_M"].iter().map(|s| s.to_string()).collect();
        assert_eq!(find_trigger(&raw, "KC_Z", &td), None);
    }

    #[test]
    fn collapse_tap_dance_only_folds_single_action_dances() {
        let td = tap_dances();
        assert_eq!(collapse_tap_dance(&td[0]), None);
        assert_eq!(collapse_tap_dance(&td[1]).as_deref(), Some("KC_Q"));
    }

    #[test]
    fn layer_tap_and_mod_tap_parse_vials_spellings() {
        assert_eq!(layer_tap("LT1(KC_A)"), Some((1, "KC_A")));
        assert_eq!(layer_tap("LT13(KC_SPACE)"), Some((13, "KC_SPACE")));
        assert_eq!(layer_tap("KC_A"), None);
        assert_eq!(mod_tap("LSFT_T(KC_S)"), Some(("LSFT", "KC_S")));
        assert_eq!(mod_tap("LSFT(KC_S)"), None);
    }
}
