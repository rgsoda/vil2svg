#!/usr/bin/env bash
# Render every flag combination with both the Python reference and the Rust
# binary, and report any byte differences.
#
# The default fixture is soda_parity.vil: the real keymap with its one
# wrapped-trigger combo removed. That combo is a Python bug the Rust port
# fixes (see README), so tests/soda.vil is *expected* to differ here -- by
# exactly that one combo and nothing else.
set -uo pipefail
cd "$(dirname "$0")/.."

VIL=${1:-tests/soda_parity.vil}
RS=./target/release/vil2svg
PY=reference/vil2svg.py
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

FLAGS=(
  ""
  "--all-layers"
  "--first-layer"
  "--print"
  "--combos-separate"
  "--first-layer --print"
  "--all-layers --combos-separate"
  "--combos-separate --print"
)

pass=0; fail=0
for flags in "${FLAGS[@]}"; do
  label=${flags:-<none>}
  # clear previous outputs, so a failed render cannot compare stale bytes
  rm -f "$tmp/py.svg" "$tmp/rs.svg"
  # shellcheck disable=SC2086
  python3 "$PY" "$VIL" -o "$tmp/py.svg" $flags >/dev/null 2>&1
  # shellcheck disable=SC2086
  "$RS" "$VIL" -o "$tmp/rs.svg" $flags >/dev/null 2>&1
  if [ ! -f "$tmp/py.svg" ] || [ ! -f "$tmp/rs.svg" ]; then
    printf '  FAIL  %s (missing output: py=%s rs=%s)\n' "$label" \
      "$([ -f "$tmp/py.svg" ] && echo yes || echo no)" \
      "$([ -f "$tmp/rs.svg" ] && echo yes || echo no)"
    fail=$((fail+1))
  elif cmp -s "$tmp/py.svg" "$tmp/rs.svg"; then
    printf '  PASS  %s\n' "$label"; pass=$((pass+1))
  else
    printf '  FAIL  %s\n' "$label"; fail=$((fail+1))
    diff <(tr '<' '\n<' <"$tmp/py.svg") <(tr '<' '\n<' <"$tmp/rs.svg") | head -20
  fi
done
echo
echo "$pass passed, $fail failed"
[ "$fail" -eq 0 ]
