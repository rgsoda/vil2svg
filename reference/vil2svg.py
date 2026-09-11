#!/usr/bin/env python3
"""Draw a Vial .vil keymap as an SVG, via keymap-drawer.

    vil2svg corne.vil              -> corne.svg
    vil2svg corne.vil -o board.svg
    vil2svg corne.vil --open       -> also opens it in imv
    vil2svg corne.vil --yaml       -> also keeps the intermediate .yaml to hand-edit
    vil2svg corne.vil --first-layer --print -> just the base layer, filling an A4 page
    vil2svg corne.yaml             -> redraw from an edited yaml, skipping conversion

Assumes a Corne / crkbd 3x6+3 matrix (LAYOUT_split_3x6_3). The physical layout is
fetched from QMK once and cached in ~/.cache/vil2svg, so later runs work offline.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

CACHE = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "vil2svg"
INFO_URL = "https://keyboards.qmk.fm/v1/keyboards/crkbd/rev1/info.json"

# Vial writes some long-deprecated QMK spellings; keymap-drawer knows the modern ones.
KEYCODE_FIX = {
    "KC_GESC": "QK_GESC", "KC_LSPO": "SC_LSPO", "KC_RSPC": "SC_RSPC",
    "KC_BSPACE": "KC_BSPC", "KC_SCOLON": "KC_SCLN", "KC_LBRACKET": "KC_LBRC",
    "KC_RBRACKET": "KC_RBRC", "KC_BSLASH": "KC_BSLS", "KC_CAPSLOCK": "KC_CAPS",
    "KC_PGDOWN": "KC_PGDN", "KC_LSHIFT": "KC_LSFT", "KC_RSHIFT": "KC_RSFT",
    "KC_LCTRL": "KC_LCTL", "KC_RCTRL": "KC_RCTL", "KC_ENTER": "KC_ENT",
    "KC_SPACE": "KC_SPC", "KC_QUOTE": "KC_QUOT", "KC_COMMA": "KC_COMM",
    "KC_MINUS": "KC_MINS", "KC_EQUAL": "KC_EQL", "KC_TRANSPARENT": "KC_TRNS",
    "QK_CLEAR_EEPROM": "EE_CLR",
    "0x7e40": "QK_BOOT", "0x7e41": "QK_RBT", "0x7e45": "VIAL_LOCK",
}

# Legends keymap-drawer leaves in raw QMK form, rewritten for humans.
LEGEND_FIX = {
    "Sft+9": "'('", "Sft+0": "')'", "Sft+MINS": "'_'", "Sft+EQL": "'+'",
    "QK GESC": "{t: Esc, h: '`~'}",
    "SC LSPO": "{t: '(', h: Shift}", "SC RSPC": "{t: ')', h: Shift}",
    "QK BOOT": "Bootloader", "QK RBT": "Reboot", "EE CLR": "'EEPROM reset'",
    "VIAL LOCK": "'Vial lock'",
    "LSFT": "Shift", "RSFT": "Shift", "LCTL": "Ctrl", "RCTL": "Ctrl",
    "LGUI": "Super", "RGUI": "Super", "LALT": "Alt", "RALT": "AltGr",
    "BSPC": "Bksp", "ENT": "Enter", "SPC": "Space", "TAB": "Tab", "CAPS": "Caps",
    "PGUP": "PgUp", "PGDN": "PgDn", "HOME": "Home", "END": "End",
    "UP": "'↑'", "DOWN": "'↓'", "LEFT": "'←'", "RIGHT": "'→'",
    "VOLU": "'Vol +'", "VOLD": "'Vol −'", "MUTE": "Mute",
    "tap-toggle": "Toggle",
}


# Punctuation whose QMK name is nothing like the character it types.
SYMBOLS = {
    "KC_MINUS": "-", "KC_EQUAL": "=", "KC_SLASH": "/", "KC_BSLASH": "\\",
    "KC_QUOTE": "'", "KC_SCOLON": ";", "KC_COMMA": ",", "KC_DOT": ".",
    "KC_GRAVE": "`", "KC_LBRACKET": "[", "KC_RBRACKET": "]",
    "KC_SPACE": "Space", "KC_ENTER": "Enter", "KC_BSPACE": "Bksp", "KC_TAB": "Tab",
}


# Shifted pairs, so a combo that emits LSFT(KC_9) draws as "(" and not "Sft+9".
SHIFTED = {
    "KC_9": "(", "KC_0": ")", "KC_LBRACKET": "{", "KC_RBRACKET": "}",
    "KC_MINUS": "_", "KC_EQUAL": "+", "KC_COMMA": "<", "KC_DOT": ">",
    "KC_SLASH": "?", "KC_SCOLON": ":", "KC_QUOTE": '"', "KC_GRAVE": "~",
    "KC_BSLASH": "|", "KC_1": "!", "KC_2": "@", "KC_3": "#", "KC_4": "$",
    "KC_5": "%", "KC_6": "^", "KC_7": "&", "KC_8": "*",
}


# Correct legends that are simply too wide for a 52px key.
# (LEGEND_FIX only reaches keymap-drawer's own output, so mod names that this
# script title-cases itself are spelled out again here.)
ABBREV = {"Escape": "Esc", "Delete": "Del", "Insert": "Ins",
          "Printscreen": "PrtSc", "Application": "Menu",
          "Lctl": "Ctrl", "Rctl": "Ctrl", "Lsft": "Shift", "Rsft": "Shift",
          "Lgui": "Super", "Rgui": "Super", "Lalt": "Alt", "Ralt": "AltGr",
          "Bspc": "Bksp", "Ent": "Enter", "Spc": "Space",
          "Pgup": "PgUp", "Pgdn": "PgDn", "Trns": None}


def key_legend(code):
    """A short human legend for one Vial keycode, or None for an empty slot."""
    if code in ("KC_NO", "KC_TRNS", "KC_TRANSPARENT", None):
        return None
    m = re.match(r"^LSFT\((KC_\w+)\)$", code)
    if m and m.group(1) in SHIFTED:
        return SHIFTED[m.group(1)]
    if code in SYMBOLS:
        return SYMBOLS[code]
    # layer switches title-case into nonsense ("Mo(2)", "Tt(2)"); the layer
    # number is the only part worth the space
    m = re.match(r"^(?:MO|TO|TG|TT|DF|OSL|LM)\((\d+)", code)
    if m:
        return f"L{m.group(1)}"
    bare = KEYCODE_FIX.get(code, code).replace("KC_", "")
    legend = bare if len(bare) == 1 else bare.replace("_", " ").title()
    return ABBREV.get(legend, legend)


def convert_keycode(code, tap_dance):
    """Vial keycode string -> QMK keycode string."""
    m = re.match(r"^LT(\d+)\((.+)\)$", code)
    if m:
        return f"LT({m.group(1)},{convert_keycode(m.group(2), tap_dance)})"
    m = re.match(r"^TD\((\d+)\)$", code)
    if m:
        # keymap-drawer has no tap-dance concept; show tap / double-tap inline.
        td = tap_dance[int(m.group(1))]
        tap, hold, dbl = td[0], td[1], td[2]
        parts = [p for p in (tap, dbl if dbl != "KC_NO" else None,
                             hold if hold != "KC_NO" else None) if p]
        return convert_keycode(parts[0], tap_dance) if len(parts) == 1 else code
    m = re.match(r"^([A-Z]+(?:_T)?)\((.+)\)$", code)
    if m:
        return f"{m.group(1)}({convert_keycode(m.group(2), tap_dance)})"
    return KEYCODE_FIX.get(code, code)


def tap_dance_legend(code, tap_dance):
    """(tap, hold, double-tap) legends for TD(n), or None."""
    m = re.match(r"^TD\((\d+)\)$", code)
    if not m:
        return None
    td = tap_dance[int(m.group(1))]
    return key_legend(td[0]), key_legend(td[1]), key_legend(td[2])


def flatten(layer):
    """One .vil layer's matrix -> the 42 keys in LAYOUT_split_3x6_3 order."""
    flat = []
    for r in range(3):
        flat += list(layer[r])                  # left, outer -> inner
        flat += list(reversed(layer[4 + r]))    # right, mirrored
    flat += [layer[3][c] for c in (3, 4, 5)]    # left thumbs
    flat += [layer[7][c] for c in (5, 4, 3)]    # right thumbs
    return flat


def vil_to_qmk(vil_path):
    """.vil matrix -> flat QMK keymap.json layers in LAYOUT_split_3x6_3 order."""
    d = json.loads(Path(vil_path).read_text())
    td = d.get("tap_dance", [])
    conv = lambda k: convert_keycode(k, td)
    layers, raw_layers, td_legends = [], [], {}
    for layer in d["layout"]:
        raw = flatten(layer)
        if len(raw) != 42:
            sys.exit(f"vil2svg: expected a 42-key 3x6+3 matrix, got {len(raw)} keys")
        raw_layers.append(raw)
        layers.append([conv(k) for k in raw])
        for k in raw:
            if isinstance(k, str) and k.startswith("TD("):
                td_legends[k] = tap_dance_legend(k, td)
    qmk = {"keyboard": "crkbd/rev1", "keymap": Path(vil_path).stem,
           "layout": "LAYOUT_split_3x6_3", "layers": layers, "version": 1}
    return qmk, td_legends, raw_layers, d.get("combo", [])


def build_combos(combo_table, raw_layers, names, separate=False):
    """Vial's combo table -> keymap-drawer `combos:` entries.

    Vial stores combos as keycodes, not positions, so each trigger is looked up
    in the layers to find where it sits. A combo is pinned to the first layer
    holding all of its triggers, which is the base layer in almost every case.
    """
    combos, skipped = [], []
    for slot, row in enumerate(combo_table):
        if len(row) < 5 or row[4] == "KC_NO":
            continue
        triggers = [k for k in row[:4] if k != "KC_NO"]
        output = key_legend(row[4])
        if not triggers or output is None:
            continue
        for i, raw in enumerate(raw_layers):
            if all(t in raw for t in triggers):
                entry = {"p": [raw.index(t) for t in triggers],
                         "k": output, "l": [names[i]]}
                if separate:
                    # in-place labels sit in the gap between keys and crowd the
                    # legends; this gives each combo its own mini diagram
                    entry["draw_separate"] = True
                combos.append(entry)
                break
        else:
            skipped.append(f"combo {slot} ({'+'.join(triggers)} -> {row[4]})")
    return combos, skipped


def used_layer_names(qmk):
    """Name each layer, marking ones that are entirely transparent/empty."""
    defaults = ["Base", "Nav", "Num", "Fn"]
    names = []
    for i, layer in enumerate(qmk["layers"]):
        live = [k for k in layer if k not in ("KC_NO", "KC_TRNS")]
        names.append((defaults[i] if i < len(defaults) else f"L{i}") if live else f"Empty{i}")
    return names


def yaml_layer_names(yaml_text):
    """Layer names from a keymap-drawer YAML, in order."""
    body = re.search(r"^layers:\n((?:[ \t]+.*\n|\n)*)", yaml_text, re.M)
    return re.findall(r"^  (\S+):", body.group(1), re.M) if body else []


def prettify(yaml_text, td_legends):
    """Rewrite raw QMK legends into readable ones."""
    fixes = dict(LEGEND_FIX)
    for code, legend in td_legends.items():
        if legend and legend[0]:
            tap, hold, dbl = legend
            # a key has three legend slots: tap in the middle, hold below,
            # double-tap above. Crammed onto one line they overflow a narrow
            # key and keymap-drawer ellipsises them.
            slots = [("t", tap)]
            if hold == tap:
                hold = None      # a dance that holds what it taps says nothing
            if dbl:
                slots.append(("s", f"\u00d72 {dbl}"))
            if hold:
                slots.append(("h", hold))
            fixes[code] = "{%s}" % ", ".join("%s: %r" % kv for kv in slots)
    for old in sorted(fixes, key=len, reverse=True):
        new = fixes[old]
        yaml_text = re.sub(r"(?<=[\[,\-] )" + re.escape(old) + r"(?=[,\]\n])", new, yaml_text)
        yaml_text = re.sub(r"(?<=\[)" + re.escape(old) + r"(?=[,\]])", new, yaml_text)
        if not new.startswith("{"):
            yaml_text = re.sub(r"(?<=\{t: )" + re.escape(old) + r"(?=[,}])", new, yaml_text)
            yaml_text = re.sub(r"(?<=h: )" + re.escape(old) + r"(?=[,}])", new, yaml_text)
    return yaml_text


# Keys are narrow, so the default 14px tap legend crowds out the 11px hold and
# double-tap ones. Level them off — the base letter is legible from position
# alone, while the alternates are the part you actually need to look up.
LEGEND_CSS = """text { font-size: 12px; }
text.combo, text.hold, text.shifted,
text.left, text.right, text.tl, text.tr, text.bl, text.br { font-size: 12px; }
"""


def draw_config(combos_separate):
    """Write a keymap-drawer config to the cache and return its path."""
    CACHE.mkdir(parents=True, exist_ok=True)
    lines = ["draw_config:", "  svg_extra_style: |"]
    lines += [f"    {line}" for line in LEGEND_CSS.strip().splitlines()]
    if combos_separate:
        # in-place labels sit in the gap between keys and crowd the legends;
        # this gives each combo its own mini diagram
        lines += ["  separate_combo_diagrams: true", "  combo_diagrams_scale: 3"]
    path = CACHE / "draw-config.yaml"
    path.write_text("\n".join(lines) + "\n")
    return path


PRINT_CSS = """
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
"""


def make_printable(svg, pad=24):
    """Black on white, on a single A4 page turned to suit the drawing."""
    m = re.match(r'<svg width="(\d+)" height="(\d+)" viewBox="0 0 \d+ \d+"[^>]*>', svg)
    if not m:
        sys.exit("vil2svg: unexpected SVG header, cannot apply --print")
    w, h = int(m.group(1)), int(m.group(2))
    vb_w, vb_h = w + 2 * pad, h + 2 * pad
    # one layer alone is far wider than it is tall, and would sit as a thin
    # strip on a portrait page; turn the paper instead of shrinking the keys
    page_w, page_h = ("297mm", "210mm") if vb_w > vb_h else ("210mm", "297mm")

    root = (f'<svg width="{page_w}" height="{page_h}" '
            f'viewBox="{-pad} {-pad} {vb_w} {vb_h}" '
            f'preserveAspectRatio="xMidYMid meet" class="keymap" '
            f'xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink">'
            f'<rect x="{-pad}" y="{-pad}" width="{vb_w}" height="{vb_h}" fill="#ffffff"/>')
    svg = root + svg[m.end():]
    return svg.replace("</style>", PRINT_CSS + "</style>", 1)


def cached_info_json():
    """QMK physical layout for crkbd, fetched once then reused offline."""
    CACHE.mkdir(parents=True, exist_ok=True)
    path = CACHE / "crkbd-rev1.info.json"
    if not path.exists():
        try:
            with urllib.request.urlopen(INFO_URL, timeout=15) as r:
                payload = json.loads(r.read())
            # the QMK API wraps it; keymap-drawer wants the bare keyboard object
            info = payload.get("keyboards", {}).get("crkbd/rev1", payload)
            path.write_text(json.dumps(info))
        except Exception as e:
            sys.exit(f"vil2svg: could not fetch the crkbd layout ({e}).\n"
                     f"  Save it manually to {path} and re-run.")
    return path


def main():
    p = argparse.ArgumentParser(description="Draw a Vial .vil keymap as an SVG.")
    p.add_argument("input", help=".vil file (or a .yaml from a previous --yaml run)")
    p.add_argument("-o", "--output", help="output SVG (default: alongside the input)")
    p.add_argument("--yaml", action="store_true", help="keep the intermediate YAML")
    p.add_argument("--open", action="store_true", help="open the SVG when done")
    p.add_argument("--all-layers", action="store_true", help="draw empty layers too")
    p.add_argument("--first-layer", action="store_true",
                   help="draw only the first (base) layer, so it fills the page")
    p.add_argument("--print", dest="print_mode", action="store_true",
                   help="black on white, sized to A4")
    p.add_argument("--combos-separate", action="store_true",
                   help="draw each combo as its own mini diagram instead of "
                        "labelling it between the keys (clearer, but shrinks "
                        "everything else on the page)")
    args = p.parse_args()

    if not shutil.which("keymap"):
        sys.exit("vil2svg: keymap-drawer not found. Install it with: pipx install keymap-drawer")

    src = Path(args.input)
    if not src.exists():
        sys.exit(f"vil2svg: {src} not found")
    out = Path(args.output) if args.output else src.with_suffix(".svg")
    yaml_path = src.with_suffix(".yaml")

    if src.suffix == ".yaml":
        names = None
    else:
        qmk, td_legends, raw_layers, combo_table = vil_to_qmk(src)
        names = used_layer_names(qmk)
        combos, skipped = build_combos(combo_table, raw_layers, names,
                                       separate=args.combos_separate)
        for s in skipped:
            print(f"vil2svg: skipped {s} — trigger keys not found on any layer",
                  file=sys.stderr)
        json_path = CACHE / f"{src.stem}.keymap.json"
        CACHE.mkdir(parents=True, exist_ok=True)
        json_path.write_text(json.dumps(qmk, indent=1))

        r = subprocess.run(["keymap", "parse", "-q", str(json_path), "-c", "6",
                            "-l", *names], capture_output=True, text=True)
        if r.returncode:
            sys.exit(f"vil2svg: keymap parse failed\n{r.stderr}")
        yaml_text = prettify(r.stdout, td_legends)
        # pin the cached physical layout so later runs need no network
        yaml_text = re.sub(r"^layout: \{.*\}$",
                           "layout: {qmk_info_json: %s, layout_name: LAYOUT_split_3x6_3}"
                           % cached_info_json(),
                           yaml_text, count=1, flags=re.M)
        if combos:
            # JSON is valid YAML, so each entry can go out as a flow mapping
            yaml_text += "combos:\n" + "".join(
                "- " + json.dumps(c) + "\n" for c in combos)
        yaml_path.write_text(yaml_text)

    cmd = ["keymap", "-c", str(draw_config(args.combos_separate)),
           "draw", str(yaml_path), "-o", str(out)]
    if args.first_layer:
        # a re-drawn .yaml never went through vil_to_qmk, so read its names back
        first = (names or yaml_layer_names(yaml_path.read_text()))[:1]
        if not first:
            sys.exit(f"vil2svg: no layers found in {yaml_path}")
        cmd += ["-s", *first]
    elif names and not args.all_layers:
        cmd += ["-s", *[n for n in names if not n.startswith("Empty")]]
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode:
        sys.exit(f"vil2svg: keymap draw failed\n{r.stderr}")

    if args.print_mode:
        out.write_text(make_printable(out.read_text()))

    if not args.yaml and src.suffix != ".yaml":
        yaml_path.unlink(missing_ok=True)
    print(out)
    if args.yaml and src.suffix != ".yaml":
        print(yaml_path)
    if args.open:
        subprocess.Popen(["imv", str(out)], start_new_session=True,
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


if __name__ == "__main__":
    main()
