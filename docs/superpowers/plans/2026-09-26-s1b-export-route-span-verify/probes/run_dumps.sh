#!/bin/bash
# usage: run_dumps.sh <binary> <outdir> [corpora...]   (dump-sites only; env passes through)
set -u
B="$1"; OUT="$2"; shift 2; mkdir -p "$OUT"
root() { case "$1" in
  X) echo "$HOME/prism-evidence/inputs/excalidraw-0642e72c/source";;
  F) echo "${CORPUS_F_ROOT:?set CORPUS_F_ROOT (private)}";;
  R) echo "$HOME/code/bench-repos/ruff/playground";;
  T) echo "$HOME/code/bench-repos/TypeScript/src";;
esac; }
for c in ${*:-X F R T}; do
  "$B" nav --no-cache call-stats --repo "$(root $c)" --dump-sites > "$OUT/$c-dump-sites.jsonl" 2> "$OUT/$c-dump-sites.stderr"
  echo "$c rc=$? rows=$(wc -l < "$OUT/$c-dump-sites.jsonl") stderr=$(wc -c < "$OUT/$c-dump-sites.stderr")"
done
