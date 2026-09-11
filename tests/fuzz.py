#!/usr/bin/env python3
"""Render randomised keymaps with both implementations and compare bytes."""
import json, os, random, subprocess, sys, tempfile

POOL = ["KC_A","KC_Z","KC_9","KC_NO","KC_TRNS","KC_MINUS","KC_BSLASH","LSFT(KC_9)",
        "LSFT(KC_RBRACKET)","RSFT(KC_4)","LCTL(KC_C)","LALT(KC_TAB)","LGUI(KC_L)",
        "MEH(KC_A)","HYPR(KC_B)","LCA(KC_D)","RCS(KC_E)","LSG(KC_F)",
        "MO(1)","MO(3)","TT(2)","TG(1)","TO(4)","DF(0)","OSL(5)",
        "LT1(KC_A)","LT3(KC_SPACE)","LT7(KC_X)","LSFT_T(KC_S)","RCTL_T(KC_SLASH)",
        "LGUI_T(KC_D)","LALT_T(KC_F)","TD(0)","TD(5)","TD(9)","TD(13)","TD(20)",
        "0x7e40","0x7e41","0x7e45","QK_CLEAR_EEPROM","KC_PRINTSCREEN",
        "KC_MEDIA_PLAY_PAUSE","KC_AUDIO_VOL_UP","KC_INTERNATIONAL_1","KC_LANGUAGE_2",
        "KC_ENTER","KC_SPACE","KC_TAB","KC_LSHIFT","KC_APPLICATION","KC_GESC"]
TRIGGERS = ["KC_A","KC_Z","KC_9","KC_MINUS","KC_ENTER","KC_TAB","KC_SPACE","KC_BSLASH"]
OUTPUTS  = ["KC_ESCAPE","LSFT(KC_9)","0x7e40","KC_BSPACE","KC_MEDIA_PLAY_PAUSE"]
FLAGS = [[], ["--all-layers"], ["--combos-separate"], ["--print"], ["--first-layer"],
         ["--all-layers","--combos-separate"]]

# The Rust port deliberately fixes two Python bugs, so on the full pool above
# the two implementations are *expected* to disagree. These strict variants
# exclude the constructs that trigger those bugs, so any difference they find
# is a genuine porting error:
#   * no tap dances and no KC_GESC/LSPO/RSPC -- these render through legend
#     fixups whose replacement starts with "{", which the Python's regex-based
#     prettify() cannot reach when the key is written in mapping form
#   * combo triggers that never appear inside a tap dance, mod-tap or
#     layer-tap, which the Python cannot see through
STRICT_POOL = [k for k in POOL
               if not k.startswith("TD(") and k not in ("KC_GESC",)]
STRICT_TRIGGERS = ["KC_Z", "KC_9", "KC_MINUS", "KC_BSLASH", "KC_ENTER",
                   "KC_TAB", "KC_LSHIFT", "KC_APPLICATION"]

def main():
    args = [a for a in sys.argv[1:] if a != "--strict"]
    strict = "--strict" in sys.argv
    pool = STRICT_POOL if strict else POOL
    triggers = STRICT_TRIGGERS if strict else TRIGGERS
    trials = int(args[0]) if args else 40
    seed = int(args[1]) if len(args) > 1 else 0
    random.seed(seed)
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    base = json.load(open(os.path.join(root, "tests/soda.vil")))
    tmp = tempfile.mkdtemp()
    compared = fails = 0
    for t in range(trials):
        d = json.loads(json.dumps(base))
        for layer in d["layout"]:
            for r in range(8):
                for c in range(6):
                    if isinstance(layer[r][c], str):
                        layer[r][c] = random.choice(pool)
        combos = [["KC_NO"] * 4 + ["KC_NO"] for _ in d["combo"]]
        for i in range(random.randint(0, 8)):
            n = random.randint(2, 4)
            trig = random.sample(triggers, n)
            combos[i] = trig + ["KC_NO"] * (4 - n) + [random.choice(OUTPUTS)]
        d["combo"] = combos
        vil = os.path.join(tmp, f"t{t}.vil")
        json.dump(d, open(vil, "w"))
        for flags in FLAGS:
            a, b = os.path.join(tmp, "a.svg"), os.path.join(tmp, "b.svg")
            for f in (a, b):
                if os.path.exists(f):
                    os.remove(f)
            subprocess.run(["python3", os.path.join(root, "reference/vil2svg.py"), vil, "-o", a] + flags,
                           capture_output=True)
            subprocess.run([os.path.join(root, "target/release/vil2svg"), vil, "-o", b] + flags,
                           capture_output=True)
            compared += 1
            ea, eb = os.path.exists(a), os.path.exists(b)
            if ea != eb:
                print(f"trial {t} {flags}: existence mismatch py={ea} rs={eb}"); fails += 1
            elif ea and open(a, "rb").read() != open(b, "rb").read():
                print(f"trial {t} {flags}: DIFFERS  ({vil})"); fails += 1
    print(f"\n{compared} renders compared, {fails} mismatches")
    return 1 if fails else 0

if __name__ == "__main__":
    sys.exit(main())
