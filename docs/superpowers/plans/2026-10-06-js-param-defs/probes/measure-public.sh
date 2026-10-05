#!/bin/bash
# Historical wire-only public-corpus base/head row capture for lane js-param-defs (planner; never F).
# Active byte/owner gate: measure-bytes.py plus adjudicate-captures.py.
# Usage: measure-public.sh BASE_BIN HEAD_BIN OUT_DIR
# For each corpus: `nav --cache-dir "$HOME/prism-evidence/js-param-defs/cache" dfg-stats --edges` and `nav --cache-dir "$HOME/prism-evidence/js-param-defs/cache" call-stats --dump-sites`
# from both binaries, then rowdiff.py (dfg) and a byte comparison (call sites).
# SecBench: one row capture per authenticated package root (manifest status ok), concatenated
# with a `pkg` prefix so rows from different packages never merge.
set -euo pipefail
BASE="$1"; HEAD="$2"; OUT="$3"
PACKET="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$OUT"
declare -a NAMES=(X Xi T R_black G_caddy RS_prism)
declare -a ROOTS=(
  "$HOME/prism-evidence/inputs/excalidraw-0642e72c/source"
  "$HOME/prism-evidence/inputs/excalidraw-0642e72c-installed/source"
  "$HOME/code/bench-repos/TypeScript/src"
  "$HOME/code/bench-repos/black"
  "$HOME/code/bench-repos/caddy"
  "$HOME/.local/share/prism/corpora/prism-20c8490591a3/source"
)
capture() { # name root side bin
  "$4" nav --cache-dir "$HOME/prism-evidence/js-param-defs/cache" dfg-stats --repo "$2" --edges 2>"$OUT/$1.$3.dfg.err" | LC_ALL=C sort > "$OUT/$1.$3.dfg.jsonl"
  "$4" nav --cache-dir "$HOME/prism-evidence/js-param-defs/cache" call-stats --repo "$2" --dump-sites 2>"$OUT/$1.$3.sites.err" > "$OUT/$1.$3.sites.jsonl"
}
for i in "${!NAMES[@]}"; do
  n="${NAMES[$i]}"; r="${ROOTS[$i]}"
  capture "$n" "$r" base "$BASE" &
  capture "$n" "$r" head "$HEAD" &
  wait
  python3 "$PACKET/rowdiff.py" "$OUT/$n.base.dfg.jsonl" "$OUT/$n.head.dfg.jsonl" "$OUT/$n.dfg-diff.json" --rows "$OUT/$n.dfg-changed.jsonl" > /dev/null
  if cmp -s "$OUT/$n.base.dfg.jsonl" "$OUT/$n.head.dfg.jsonl"; then d=identical; else d=differs; fi
  if cmp -s "$OUT/$n.base.sites.jsonl" "$OUT/$n.head.sites.jsonl"; then s=identical; else s=differs; fi
  echo "{\"corpus\":\"$n\",\"dfg_bytes\":\"$d\",\"call_sites_bytes\":\"$s\",\"dfg_diff\":$(cat "$OUT/$n.dfg-diff.json" | tr -d '\n ')}"
done
