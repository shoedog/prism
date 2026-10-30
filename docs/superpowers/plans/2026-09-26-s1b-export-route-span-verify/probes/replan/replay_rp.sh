#!/bin/bash
# Replay the re-plan fixtures against a binary. Usage: replay_rp.sh <binary> <out_dir>
# Generates the fixtures fresh (gen_rp.py asserts the scenario set), then asserts the generated directory holds
# exactly RP-SCENARIOS.txt before running anything.
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"; B="$1"; O="$2"; G="$O/fixtures"
rm -rf "$G"; mkdir -p "$O"; python3 "$HERE/gen_rp.py" "$G" >/dev/null
diff <(ls "$G" | LC_ALL=C sort) <(LC_ALL=C sort "$HERE/RP-SCENARIOS.txt")
for d in "$G"/*/; do n=$(basename "$d")
  "$B" nav --no-cache call-stats --repo "$d" --dump-sites > "$O/$n.dump.jsonl" 2> "$O/$n.stderr"
  "$B" nav --no-cache functions --repo "$d" > "$O/$n.functions.json" 2>> "$O/$n.stderr"
done
cd "$O" && for f in *.dump.jsonl; do mv "$f" "C_${f}"; done 2>/dev/null; for f in *.functions.json; do mv "$f" "C_${f}"; done
python3 "$HERE/../controls_summarize.py" > "$O/SUMMARY.txt"; echo "stderr bytes: $(cat "$O"/*.stderr | wc -c)"
