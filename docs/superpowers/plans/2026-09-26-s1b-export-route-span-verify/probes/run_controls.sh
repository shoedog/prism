#!/bin/bash
# usage: run_controls.sh <binary> <gen_dir> <out_dir>   (env passes through)
B="$1"; G="$2"; O="$3"; mkdir -p "$O"
HERE="$(cd "$(dirname "$0")" && pwd)"
cd "$G"
for d in C*/; do d=${d%/}
  "$B" nav --no-cache call-stats --repo "$G/$d" --dump-sites > "$O/$d.dump.jsonl" 2> "$O/$d.dump.stderr"
  "$B" nav --no-cache functions --repo "$G/$d" > "$O/$d.functions.json" 2>> "$O/$d.dump.stderr"
done
cd "$O" && python3 "$HERE/controls_summarize.py" > "$O/SUMMARY.txt"
echo "stderr bytes: $(cat "$O"/*.stderr | wc -c)"
