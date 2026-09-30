#!/bin/bash
# usage: run_corpora.sh <binary> <outdir> [corpora...]
set -u
B="$1"; OUT="$2"; shift 2
mkdir -p "$OUT"
root() { case "$1" in
  X) echo "$HOME/prism-evidence/inputs/excalidraw-0642e72c/source";;
  F) echo "${CORPUS_F_ROOT:?set CORPUS_F_ROOT (private; see the S1 private evidence file)}";;
  R) echo "$HOME/code/bench-repos/ruff/playground";;
  T) echo "$HOME/code/bench-repos/TypeScript/src";;
esac; }
LIST="${*:-X F R T}"
for c in $LIST; do
  r=$(root $c)
  "$B" nav --no-cache call-stats --repo "$r" > "$OUT/$c-call-stats.json" 2> "$OUT/$c-call-stats.stderr"; a=$?
  "$B" nav --no-cache call-stats --repo "$r" --dump-sites > "$OUT/$c-dump-sites.jsonl" 2> "$OUT/$c-dump-sites.stderr"; b=$?
  "$B" nav --no-cache functions --repo "$r" > "$OUT/$c-functions.json" 2> "$OUT/$c-functions.stderr"; d=$?
  echo "$c rc=$a/$b/$d rows=$(wc -l < "$OUT/$c-dump-sites.jsonl") stderr_bytes=$(cat "$OUT/$c"-*.stderr | wc -c)"
done
