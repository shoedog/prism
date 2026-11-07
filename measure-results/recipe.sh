#!/bin/bash
# #660 S1c cloud recipe. usage: measure660-s1c-cloud.sh BASE_BIN HEAD_BIN CORPORA OUT
# Expects compare_edges.py, join_trace.py and timewrap.py next to it. Corpora roots under CORPORA: excalidraw/, express/,
# TypeScript/src/. Base = merge-base of the PR with main (61cd41f5 at authoring; recheck), head = PR head; both
# binaries built in the SAME environment (`cargo build --release` in each checkout).
# Per corpus and arm, one process at a time:
#   stats  : `nav --no-cache dfg-stats`, timed with timewrap.py (python3 subprocess: wall seconds and ru_maxrss in KiB; no /usr/bin/time needed) -> $tag.$arm.stats.json/.time
#   edges  : `nav --no-cache dfg-stats --edges` (base untimed; head with PRISM_RD_CENSUS_TRACE=1, stderr KEPT in
#            $tag.head.edges.err; the traced run is not used for wall/RSS)
# then compare_edges.py (exit 0 iff identical edge sets, labels only move) and join_trace.py (exit 0 iff every moved
# edge carries a census trace mechanism that explains its move kind (see PASS below); the moves come from the
# dumps, the trace is a superset reported as a count plus a sample, never as moves).
# PASS (all three corpora): compare_edges exit 0; join_trace exit 0 (every moved edge carries a mechanism that can explain
# its move kind: kill:<form> -> nameonly/killed; owner-refuse:escape|parse|target[:kind] and site-unmapped ->
# nameonly/cfg_incomplete; no promotion; no OTHER move); over_cap, fallback_functions_capped, fallback_edges_capped
# unchanged between arms (the census-only availability flip, a kill-only def entering the one-node distinct-bases belt or the
# 2048-def cap, would move them); no conflict line in *.err other than rd-census trace lines; wall and peak RSS within +3%
# (flag beyond). Every SITE-UNMAPPED demotion is listed in full in $tag.join.txt and must be hand-read; sample >= 5 per
# mechanism and kind (kill:update, kill:delete, kill:pattern, kill:for-head, kill:for-increment, owner-refuse:*).
# S1b-2 adds d3-use|d3-def|d3-loop|d3-unknown -> nameonly/sameline (from exact only); every D3-FULL (d3-loop, d3-unknown)
# demotion is listed in full and hand-read (DESIGN 10). Base also carries S1b-1's census, so S1b-1 mechanisms should not move.
# S1c adds d2c -> nameonly/cfg_incomplete (from exact or sameline only, never into exact; a killed or already incomplete
# label is kept). Sample >= 5 per corpus (DESIGN 10), including unit shapes: field initializer, method, getter/setter, a
# parameter default, a write two callables deep, a hoisted declaration, and the cost classes (b03 read before creation).
# Base also carries S1b-2's D3, so d3-* should not move.
# Expectation (DESIGN 9, S1c): about 411 / 16 heuristic on excalidraw / express (245 of excalidraw's in one generated
# emscripten file); the trace re-counts. TypeScript src only the cloud measures. Base = merge-base (61cd41f5 at authoring; recheck).
set -u
BASE_BIN=$1 HEAD_BIN=$2 C=$3 OUT=$4
D=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OUT"
TIME="python3 $D/timewrap.py"
for pair in X:excalidraw EX:express T:TypeScript/src; do
  tag=${pair%%:*}; root=$C/${pair#*:}
  [ -d "$root" ] || { echo "MISSING corpus $tag $root"; continue; }
  for arm in base head; do
    bin=$BASE_BIN; [ $arm = head ] && bin=$HEAD_BIN
    s=$(date +%s)
    $TIME "$OUT/$tag.$arm.time" "$bin" nav --no-cache dfg-stats --repo "$root" \
      > "$OUT/$tag.$arm.stats.json" 2> "$OUT/$tag.$arm.stats.err"; e1=$?
    if [ $arm = head ]; then
      PRISM_RD_CENSUS_TRACE=1 "$bin" nav --no-cache dfg-stats --repo "$root" --edges \
        > "$OUT/$tag.$arm.edges.jsonl" 2> "$OUT/$tag.$arm.edges.err"; e2=$?
    else
      "$bin" nav --no-cache dfg-stats --repo "$root" --edges \
        > "$OUT/$tag.$arm.edges.jsonl" 2> "$OUT/$tag.$arm.edges.err"; e2=$?
    fi
    echo "$tag $arm stats_exit=$e1 edges_exit=$e2 edges=$(wc -l < "$OUT/$tag.$arm.edges.jsonl") secs=$(( $(date +%s)-s )) \
wall_rss=$(grep -E 'Elapsed|Maximum resident' "$OUT/$tag.$arm.time" | tr -s ' ' | tr '\n' ';')"
  done
  python3 "$D/compare_edges.py" "$OUT/$tag.base.edges.jsonl" "$OUT/$tag.head.edges.jsonl" --sample 12 > "$OUT/$tag.compare.txt" 2>&1
  echo "$tag compare_exit=$? (0 = identical edge sets, only labels move)"
  python3 "$D/join_trace.py" "$OUT/$tag.base.edges.jsonl" "$OUT/$tag.head.edges.jsonl" "$OUT/$tag.head.edges.err" --list > "$OUT/$tag.join.txt" 2>&1
  echo "$tag join_exit=$? (0 = every moved edge carries a mechanism)"
  grep -c '^rd-census' "$OUT/$tag.head.edges.err" | sed "s/^/$tag trace_lines=/"
  grep -v '^rd-census' "$OUT/$tag.head.edges.err" | head -5 | sed "s/^/$tag head.err: /"
  gzip -9 "$OUT/$tag.base.edges.jsonl" "$OUT/$tag.head.edges.jsonl" "$OUT/$tag.head.edges.err"
done
