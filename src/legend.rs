//! Turning Vial/QMK keycode strings into human legends.
//!
//! There are two legend paths, inherited from the Python script:
//!
//!   * grid keys go through keymap-drawer's keycode map, then `LEGEND_FIX`
//!   * tap-dance sub-legends and combo outputs go through `key_legend`
//!
//! They disagree in places (`Escape` vs `Esc`), so both are kept.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Vial writes some long-deprecated QMK spellings; keymap-drawer knows the modern ones.
pub const KEYCODE_FIX: &[(&str, &str)] = &[
    ("KC_GESC", "QK_GESC"), ("KC_LSPO", "SC_LSPO"), ("KC_RSPC", "SC_RSPC"),
    ("KC_BSPACE", "KC_BSPC"), ("KC_SCOLON", "KC_SCLN"), ("KC_LBRACKET", "KC_LBRC"),
    ("KC_RBRACKET", "KC_RBRC"), ("KC_BSLASH", "KC_BSLS"), ("KC_CAPSLOCK", "KC_CAPS"),
    ("KC_PGDOWN", "KC_PGDN"), ("KC_LSHIFT", "KC_LSFT"), ("KC_RSHIFT", "KC_RSFT"),
    ("KC_LCTRL", "KC_LCTL"), ("KC_RCTRL", "KC_RCTL"), ("KC_ENTER", "KC_ENT"),
    ("KC_SPACE", "KC_SPC"), ("KC_QUOTE", "KC_QUOT"), ("KC_COMMA", "KC_COMM"),
    ("KC_MINUS", "KC_MINS"), ("KC_EQUAL", "KC_EQL"), ("KC_TRANSPARENT", "KC_TRNS"),
    ("QK_CLEAR_EEPROM", "EE_CLR"),
    ("0x7e40", "QK_BOOT"), ("0x7e41", "QK_RBT"), ("0x7e45", "VIAL_LOCK"),
];

/// Legends keymap-drawer leaves in raw QMK form, rewritten for humans.
/// Applied to the legend *after* prefix stripping, not to the keycode.
pub const LEGEND_FIX: &[(&str, &str)] = &[
    ("Sft+9", "("), ("Sft+0", ")"), ("Sft+MINS", "_"), ("Sft+EQL", "+"),
    ("QK BOOT", "Bootloader"), ("QK RBT", "Reboot"), ("EE CLR", "EEPROM reset"),
    ("VIAL LOCK", "Vial lock"),
    ("LSFT", "Shift"), ("RSFT", "Shift"), ("LCTL", "Ctrl"), ("RCTL", "Ctrl"),
    ("LGUI", "Super"), ("RGUI", "Super"), ("LALT", "Alt"), ("RALT", "AltGr"),
    ("BSPC", "Bksp"), ("ENT", "Enter"), ("SPC", "Space"), ("TAB", "Tab"), ("CAPS", "Caps"),
    ("PGUP", "PgUp"), ("PGDN", "PgDn"), ("HOME", "Home"), ("END", "End"),
    ("UP", "↑"), ("DOWN", "↓"), ("LEFT", "←"), ("RIGHT", "→"),
    ("VOLU", "Vol +"), ("VOLD", "Vol −"), ("MUTE", "Mute"),
    ("tap-toggle", "Toggle"),
];

/// `LEGEND_FIX` entries that set a hold legend as well as a tap one.
/// In the Python these were YAML mappings such as `{t: Esc, h: '`~'}`.
pub const LEGEND_FIX_TAP_HOLD: &[(&str, &str, &str)] = &[
    ("QK GESC", "Esc", "`~"),
    ("SC LSPO", "(", "Shift"),
    ("SC RSPC", ")", "Shift"),
];

/// Punctuation whose QMK name is nothing like the character it types.
pub const SYMBOLS: &[(&str, &str)] = &[
    ("KC_MINUS", "-"), ("KC_EQUAL", "="), ("KC_SLASH", "/"), ("KC_BSLASH", "\\"),
    ("KC_QUOTE", "'"), ("KC_SCOLON", ";"), ("KC_COMMA", ","), ("KC_DOT", "."),
    ("KC_GRAVE", "`"), ("KC_LBRACKET", "["), ("KC_RBRACKET", "]"),
    ("KC_SPACE", "Space"), ("KC_ENTER", "Enter"), ("KC_BSPACE", "Bksp"), ("KC_TAB", "Tab"),
];

/// Shifted pairs, so a combo that emits LSFT(KC_9) draws as "(" and not "Sft+9".
pub const SHIFTED: &[(&str, &str)] = &[
    ("KC_9", "("), ("KC_0", ")"), ("KC_LBRACKET", "{"), ("KC_RBRACKET", "}"),
    ("KC_MINUS", "_"), ("KC_EQUAL", "+"), ("KC_COMMA", "<"), ("KC_DOT", ">"),
    ("KC_SLASH", "?"), ("KC_SCOLON", ":"), ("KC_QUOTE", "\""), ("KC_GRAVE", "~"),
    ("KC_BSLASH", "|"), ("KC_1", "!"), ("KC_2", "@"), ("KC_3", "#"), ("KC_4", "$"),
    ("KC_5", "%"), ("KC_6", "^"), ("KC_7", "&"), ("KC_8", "*"),
];

/// Correct legends that are simply too wide for a 52px key.
pub const ABBREV: &[(&str, &str)] = &[
    ("Escape", "Esc"), ("Delete", "Del"), ("Insert", "Ins"),
    ("Printscreen", "PrtSc"), ("Application", "Menu"),
    ("Lctl", "Ctrl"), ("Rctl", "Ctrl"), ("Lsft", "Shift"), ("Rsft", "Shift"),
    ("Lgui", "Super"), ("Rgui", "Super"), ("Lalt", "Alt"), ("Ralt", "AltGr"),
    ("Bspc", "Bksp"), ("Ent", "Enter"), ("Spc", "Space"),
    ("Pgup", "PgUp"), ("Pgdn", "PgDn"),
];

/// keymap-drawer's built-in QMK keycode -> legend map, applied after the
/// `KC_` prefix is stripped.
fn qmk_keycode_map() -> &'static HashMap<String, String> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    MAP.get_or_init(|| {
        serde_json::from_str(include_str!("qmk_keycode_map.json"))
            .expect("embedded qmk_keycode_map.json is malformed")
    })
}

fn lookup(table: &'static [(&'static str, &'static str)], key: &str) -> Option<&'static str> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// Apply the deprecated-spelling fixups to a bare keycode.
pub fn fix_keycode(code: &str) -> String {
    lookup(KEYCODE_FIX, code)
        .map(str::to_string)
        .unwrap_or_else(|| code.to_string())
}

/// keymap-drawer's legend for a bare QMK keycode, then `LEGEND_FIX`.
/// This is the grid-key path.
pub fn grid_legend(code: &str) -> String {
    let raw = raw_grid_legend(code);
    lookup(LEGEND_FIX, &raw).map(str::to_string).unwrap_or(raw)
}

/// The legend keymap-drawer produces, before vil2svg's own fixups.
pub fn raw_grid_legend(code: &str) -> String {
    let fixed = fix_keycode(code);
    // modifier functions wrap a keycode: LSFT(KC_RBRC) -> "Sft+]"
    let (base, mods) = strip_modifier_fns(&fixed);
    let base = fix_keycode(&base);
    let bare = base.strip_prefix("KC_").unwrap_or(&base);
    let mapped = qmk_keycode_map()
        .get(bare)
        .cloned()
        .unwrap_or_else(|| bare.replace('_', " "));
    format_modified(&mapped, &mods)
}

/// A grid legend that also carries a hold, if this keycode has one.
pub fn grid_tap_hold(code: &str) -> Option<(&'static str, &'static str)> {
    let raw = raw_grid_legend(code);
    LEGEND_FIX_TAP_HOLD
        .iter()
        .find(|(k, _, _)| *k == raw)
        .map(|(_, t, h)| (*t, *h))
}

/// A short human legend for one Vial keycode, or None for an empty slot.
/// This is the tap-dance / combo-output path.
pub fn key_legend(code: &str) -> Option<String> {
    if matches!(code, "KC_NO" | "KC_TRNS" | "KC_TRANSPARENT") {
        return None;
    }
    // a shifted symbol reads better as the character it types
    if let Some(inner) = wrapped_arg(code, "LSFT")
        && let Some(sym) = lookup(SHIFTED, inner) {
            return Some(sym.to_string());
        }
    if let Some(sym) = lookup(SYMBOLS, code) {
        return Some(sym.to_string());
    }
    // layer switches title-case into nonsense ("Mo(2)", "Tt(2)"); the layer
    // number is the only part worth the space
    for kind in ["MO", "TO", "TG", "TT", "DF", "OSL", "LM"] {
        if let Some(arg) = wrapped_arg(code, kind) {
            let digits: String = arg.chars().take_while(char::is_ascii_digit).collect();
            if !digits.is_empty() {
                return Some(format!("L{digits}"));
            }
        }
    }
    let fixed = fix_keycode(code);
    let bare = fixed.strip_prefix("KC_").unwrap_or(&fixed);
    let legend = if bare.chars().count() == 1 {
        bare.to_string()
    } else {
        title_case(&bare.replace('_', " "))
    };
    if legend == "Trns" {
        return None;
    }
    Some(
        lookup(ABBREV, &legend)
            .map(str::to_string)
            .unwrap_or(legend),
    )
}

/// Python's `str.title()`: capitalise the first letter of each run of letters.
fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_alpha = false;
    for c in s.chars() {
        if c.is_alphabetic() {
            if prev_alpha {
                out.extend(c.to_lowercase());
            } else {
                out.extend(c.to_uppercase());
            }
            prev_alpha = true;
        } else {
            out.push(c);
            prev_alpha = false;
        }
    }
    out
}

/// If `code` is exactly `name(...)`, return the inside.
pub fn wrapped_arg<'a>(code: &'a str, name: &str) -> Option<&'a str> {
    let rest = code.strip_prefix(name)?;
    let rest = rest.strip_prefix('(')?;
    let inner = rest.strip_suffix(')')?;
    Some(inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_case_matches_python() {
        assert_eq!(title_case("MEDIA PLAY PAUSE"), "Media Play Pause");
        assert_eq!(title_case("KP ASTERISK"), "Kp Asterisk");
        assert_eq!(title_case("F12"), "F12");
    }

    #[test]
    fn grid_legend_strips_prefix_then_applies_fixups() {
        // deprecated spelling -> modern -> keymap-drawer map
        assert_eq!(grid_legend("KC_BSLASH"), "\\");
        assert_eq!(grid_legend("KC_SCOLON"), ";");
        // LEGEND_FIX applies after prefix stripping
        assert_eq!(grid_legend("KC_BSPACE"), "Bksp");
        assert_eq!(grid_legend("KC_LSHIFT"), "Shift");
        assert_eq!(grid_legend("0x7e40"), "Bootloader");
        assert_eq!(grid_legend("KC_AUDIO_VOL_UP"), "AUDIO VOL UP");
    }

    #[test]
    fn key_legend_prefers_the_character_a_key_types() {
        assert_eq!(key_legend("LSFT(KC_9)").as_deref(), Some("("));
        assert_eq!(key_legend("KC_MINUS").as_deref(), Some("-"));
        assert_eq!(key_legend("KC_ESCAPE").as_deref(), Some("Esc"));
        assert_eq!(key_legend("MO(2)").as_deref(), Some("L2"));
        assert_eq!(key_legend("TT(3)").as_deref(), Some("L3"));
    }

    #[test]
    fn key_legend_is_empty_for_blank_slots() {
        assert_eq!(key_legend("KC_NO"), None);
        assert_eq!(key_legend("KC_TRNS"), None);
        assert_eq!(key_legend("KC_TRANSPARENT"), None);
    }

    #[test]
    fn wrapped_arg_requires_an_exact_wrapper() {
        assert_eq!(wrapped_arg("TD(4)", "TD"), Some("4"));
        assert_eq!(wrapped_arg("LSFT(KC_9)", "LSFT"), Some("KC_9"));
        assert_eq!(wrapped_arg("LSFT_T(KC_S)", "LSFT"), None);
        assert_eq!(wrapped_arg("KC_A", "TD"), None);
    }
}

/// QMK modifier-function names -> the standard modifiers they apply.
/// From keymap-drawer's `_modifier_fn_to_std`.
const MODIFIER_FN: &[(&str, &[&str])] = &[
    ("LCTL", &["left_ctrl"]), ("C", &["left_ctrl"]),
    ("LSFT", &["left_shift"]), ("S", &["left_shift"]),
    ("LALT", &["left_alt"]), ("A", &["left_alt"]), ("LOPT", &["left_alt"]),
    ("LGUI", &["left_gui"]), ("G", &["left_gui"]), ("LCMD", &["left_gui"]), ("LWIN", &["left_gui"]),
    ("RCTL", &["right_ctrl"]), ("RSFT", &["right_shift"]),
    ("RALT", &["right_alt"]), ("ROPT", &["right_alt"]), ("ALGR", &["right_alt"]),
    ("RGUI", &["right_gui"]), ("RCMD", &["right_gui"]), ("RWIN", &["right_gui"]),
    ("LSG", &["left_shift", "left_gui"]), ("SGUI", &["left_shift", "left_gui"]),
    ("SCMD", &["left_shift", "left_gui"]), ("SWIN", &["left_shift", "left_gui"]),
    ("LAG", &["left_alt", "left_gui"]),
    ("RSG", &["right_shift", "right_gui"]), ("RAG", &["right_alt", "right_gui"]),
    ("LCA", &["left_ctrl", "left_alt"]), ("LSA", &["left_shift", "left_alt"]),
    ("RSA", &["right_shift", "right_alt"]), ("SAGR", &["right_shift", "right_alt"]),
    ("RCS", &["right_ctrl", "right_shift"]),
    ("LCAG", &["left_ctrl", "left_alt", "left_gui"]),
    ("MEH", &["left_ctrl", "left_shift", "left_alt"]),
    ("HYPR", &["left_ctrl", "left_shift", "left_alt", "left_gui"]),
];

/// How each standard modifier is displayed.
const MOD_DISPLAY: &[(&str, &str)] = &[
    ("left_ctrl", "Ctl"), ("right_ctrl", "Ctl"),
    ("left_shift", "Sft"), ("right_shift", "Sft"),
    ("left_alt", "Alt"), ("right_alt", "AltGr"),
    ("left_gui", "Gui"), ("right_gui", "Gui"),
];

/// Modifier sets with a name of their own.
const SPECIAL_COMBINATIONS: &[(&[&str], &str)] = &[
    (&["left_ctrl", "left_alt", "left_gui", "left_shift"], "Hyper"),
    (&["left_ctrl", "left_alt", "left_shift"], "Meh"),
];

/// Peel modifier functions off a keycode, returning the base and the mods.
pub fn strip_modifier_fns(code: &str) -> (String, Vec<&'static str>) {
    let mut mods: Vec<&'static str> = Vec::new();
    let mut current = code.to_string();
    while let Some(open) = current.find('(') {
        let name = &current[..open];
        let Some(applied) = MODIFIER_FN.iter().find(|(n, _)| *n == name) else {
            break;
        };
        let Some(inner) = current[open + 1..].strip_suffix(')') else {
            break;
        };
        mods.extend(applied.1.iter().copied());
        current = inner.to_string();
    }
    (current, mods)
}

/// Render a modified keycode the way keymap-drawer does, e.g. `Sft+]`.
pub fn format_modified(key: &str, mods: &[&str]) -> String {
    if mods.is_empty() {
        return key.to_string();
    }
    let mut sorted: Vec<&str> = mods.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let special = SPECIAL_COMBINATIONS.iter().find(|(set, _)| {
        let mut s: Vec<&str> = set.to_vec();
        s.sort_unstable();
        s == sorted
    });
    let fns = match special {
        Some((_, name)) => name.to_string(),
        None => mods
            .iter()
            .map(|m| {
                lookup(MOD_DISPLAY, m).unwrap_or(m)
            })
            .collect::<Vec<_>>()
            .join("+"),
    };
    format!("{fns}+{key}")
}
