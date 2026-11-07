#!/bin/bash
# Cloud mutation-gate runner. usage: cloud-gate.sh SHA BASE|- RESULT_BRANCH [LANE_JSON...]
# Runs the tier-1 gate (--since BASE --scope fn --authoritative) plus each lane on SHA, then pushes
# only the reports (summary.json per gate, logs, machine facts, resource samples) to an orphan branch.
# The controller reads the summary.json verdicts; the session's prose is not the record.
set -u
SHA=$1 BASE=$2 BR=$3; shift 3
REPO=$(git rev-parse --show-toplevel); cd "$REPO"
R=/tmp/gate-results; rm -rf $R; mkdir -p $R
st() { echo "$(date -u +%FT%TZ) $*" >> $R/status.txt; }
git fetch -q origin && git checkout -q --detach "$SHA" || { st "FAILED checkout"; echo DONE >> $R/status.txt; exit 1; }
st "head $(git rev-parse HEAD) base $BASE lanes: $*"
{ nproc; free -g; df -h /; rustc -V; cargo -V; } > $R/machine.txt 2>&1
s=$(date +%s); cargo fetch > $R/cargo-fetch.log 2>&1; st "cargo fetch exit=$? secs=$(( $(date +%s)-s ))"
(while sleep 30; do echo "$(date +%s) $(df -BG --output=used / | tail -1 | tr -d ' G') $(free -m | awk '/Mem:/{print $3}')"; done) > $R/samples.txt &
SAMP=$!
run() { name=$1; shift; s=$(date +%s)
  python3 scripts/mutgate/mutgate.py "$@" --out $R/$name > $R/$name.log 2>&1
  st "$name exit=$? wall=$(( $(date +%s)-s ))s; $(grep -E 'TOTAL' $R/$name.log | tail -1)"; }
# BASE "-" skips the tier-1 scoped gate (reviewer probe lanes only; owner 2026-10-31: no per-PR gates).
[ "$BASE" = "-" ] || run since --since "$BASE" --scope fn --authoritative
for l in "$@"; do run "lane-$(basename "$l" .json)" --lane "$l" --jobs "${LANE_JOBS:-4}"; done
kill $SAMP
st "peak disk GiB $(awk '{print $2}' $R/samples.txt | sort -n | tail -1); peak mem MiB $(awk '{print $3}' $R/samples.txt | sort -n | tail -1)"
# Keep reports small: summary.json and logs only.
find $R -type f ! -name '*.json' ! -name '*.log' ! -name '*.txt' -delete
git checkout -q --orphan "$BR" && git rm -rq --cached . && mkdir -p gate-results && cp -r $R/. gate-results/ \
  && git add gate-results && git commit -qm "gate results $SHA" && git push -q origin "$BR" \
  && st "pushed $BR" || st "FAILED push"
echo DONE >> $R/status.txt
