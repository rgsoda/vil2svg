# vil2svg

Draw a Vial `.vil` keymap as an SVG.

```
vil2svg corne.vil                       # -> corne.svg
vil2svg corne.vil -o board.svg
vil2svg corne.vil --open                # also opens it in imv
vil2svg corne.vil --first-layer --print # just the base layer, filling an A4 page
```

Assumes a Corne / crkbd 3x6+3 matrix (`LAYOUT_split_3x6_3`). The physical layout
is fetched from QMK once and cached in `~/.cache/vil2svg`, so later runs work
offline.

## Install

```
brew install rgsoda/tap/vil2svg
```

Or from source, anywhere with a Rust toolchain:

```
cargo install --git https://github.com/rgsoda/vil2svg
```

macOS and Linux. `--open` uses `imv` on Linux and `open` on macOS; nothing else
is platform-specific.

## Flags

| Flag | Effect |
| --- | --- |
| `-o`, `--output` | Output path (default: alongside the input) |
| `--open` | Open the finished SVG in `imv` |
| `--all-layers` | Draw entirely empty layers too |
| `--first-layer` | Draw only the base layer, so it fills the page |
| `--print` | Black on white, sized to A4 |
| `--combos-separate` | Give each combo its own mini diagram |

## Build

```
cargo build --release
cargo test
```

No runtime dependencies: this is a single static binary. It replaces an earlier
Python script that shelled out to [keymap-drawer], which needed Python, pipx and
a venv on the machine doing the drawing. The parts of keymap-drawer this tool
used (the QMK keymap parser, the QMK `info.json` physical layout, and the SVG
drawer) are reimplemented in `src/`; its default stylesheet and QMK keycode map
are vendored verbatim in `src/svg_style.css` and `src/qmk_keycode_map.json`.

[keymap-drawer]: https://github.com/caksoylar/keymap-drawer

## Differences from the old Python script

The port is otherwise byte-for-byte identical, but it fixes two bugs. Both came
from the old script post-processing keymap-drawer's *text output* with regexes;
drawing in-process removes the whole class of problem.

**Combo triggers are resolved through wrappers.** Vial stores combos as
keycodes, not key positions, so each trigger has to be located in the layers.
The Python compared keycode strings literally, so a trigger bound inside a tap
dance, mod-tap or layer-tap was never found — on a typical keymap that is close
to half the board, and the combo was silently dropped with a warning. `vil2svg`
now looks through those wrappers, preferring a direct binding when one exists.

**Tap-dance and `QK_GESC` legends no longer leak.** The Python rewrote legends
by regex over the generated YAML. A key that is *also* held to reach a layer is
written in mapping form (`{t: TD(5), type: held}`), which none of those regexes
matched, so a raw `TD(5)` or `QK GESC` was drawn on the key instead of its real
legend.

## Tests

`cargo test` covers the fiddly internals — Python-compatible rounding (ties to
even), `TextWrapper`'s wrapping and whitespace rules, the two legend paths, and
combo trigger resolution.

The old script is kept in `reference/` for differential testing:

```
bash tests/compare.sh [file.vil]   # render both, compare bytes, across flags
python3 tests/fuzz.py 30 0 --strict
```

`compare.sh` defaults to `tests/soda_parity.vil` — the real keymap with its one
wrapped-trigger combo removed. Run it on `tests/soda.vil` and it reports eight
failures, all of them that single combo, which the Python drops and this port
draws.

`fuzz.py` renders randomised keymaps with both implementations. `--strict` draws
from a pool that excludes the constructs covered by the two fixes above, so the
two must agree byte-for-byte; without it they are expected to differ exactly
where the fixes apply.
