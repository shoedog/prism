#!/bin/bash
# Controller only. Never execute in a planner/implementer session.
# Set the private root/evidence path privately. Returns no F paths/names/source.
set -euo pipefail
: "${CORPUS_F_ROOT:?controller sets the private corpus root}"
: "${PRIVATE_F_EVIDENCE:?controller sets a private durable evidence directory}"
P=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$P" rev-parse --show-toplevel)
B="$ROOT/target/repair-r1/prism-base"
H="$ROOT/target/repair-r1/prism-head"
M="$ROOT/target/repair-r1/BUILD-MANIFEST.json"
test -x "$B"; test -x "$H"; test -s "$M"
# Reject overwritten/unbound executables before private measurement.
python3 - "$M" "$B" "$H" "$ROOT" <<'PY'
import hashlib,json,sys
m=json.load(open(sys.argv[1]))
for lane,path in zip(('base','head'),sys.argv[2:]):
 assert hashlib.sha256(open(path,'rb').read()).hexdigest()==m[lane+'_binary_sha256'],lane+' binary custody mismatch'
assert m['source_base_sha']=='5048f443'
# Base binary was built at 915fca43; its src equals main 5048f443.
assert m['source_head_sha'].startswith('8796dc55')
assert m['measurement_round']=='r5-impl-repair-r1'
assert m['head_binary_relative_path']=='target/repair-r1/prism-head'
assert m['cache_versions']==[104,60]
from pathlib import Path
root=Path(sys.argv[4])
for name,digest in m['crate_input_sha256'].items():
 assert hashlib.sha256((root/name).read_bytes()).hexdigest()==digest,'crate input custody mismatch: '+name
for name,digest in m['verification_file_sha256'].items():
 assert hashlib.sha256((root/name).read_bytes()).hexdigest()==digest,'repair file custody mismatch: '+name
PY
mkdir -p "$PRIVATE_F_EVIDENCE/base" "$PRIVATE_F_EVIDENCE/head"
cp "$M" "$PRIVATE_F_EVIDENCE/BUILD-MANIFEST.json"
shasum -a 256 "$B" "$H" > "$PRIVATE_F_EVIDENCE/binaries.sha256"
"$B" --version > "$PRIVATE_F_EVIDENCE/base.version"
"$H" --version > "$PRIVATE_F_EVIDENCE/head.version"
git -C "$ROOT" rev-parse HEAD > "$PRIVATE_F_EVIDENCE/plan.head"
git -C "$ROOT" diff --stat > "$PRIVATE_F_EVIDENCE/repair.diff-stat"
# Direct commands preserve their failure statuses; run_dumps.sh masks failures.
"$B" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl" 2> "$PRIVATE_F_EVIDENCE/base/F-dump-sites.stderr"
"$H" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl" 2> "$PRIVATE_F_EVIDENCE/head/F-dump-sites.stderr"
test -s "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl"
test -s "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl"
test ! -s "$PRIVATE_F_EVIDENCE/base/F-dump-sites.stderr"
test ! -s "$PRIVATE_F_EVIDENCE/head/F-dump-sites.stderr"
python3 "$P/probes/rowdiff.py" "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/rowdiff.json" > "$PRIVATE_F_EVIDENCE/rowdiff.log"
python3 "$P/probes/audit_s1b4.py" "$CORPUS_F_ROOT" "$PRIVATE_F_EVIDENCE/rowdiff.json" "$PRIVATE_F_EVIDENCE/audit.json" --private > "$PRIVATE_F_EVIDENCE/audit-counts.json"
python3 "$P/probes/valueflow_guard_s1b4.py" "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/valueflow.json" --private > "$PRIVATE_F_EVIDENCE/valueflow-counts.json"
python3 - "$PRIVATE_F_EVIDENCE/valueflow.json" <<'PY'
import json,sys
m=json.load(open(sys.argv[1]));assert m['base_sites']>0 and m['head_sites']>0
assert m['keys_only_base']==0 and m['keys_only_head']==0,'incomplete call-site population: inadmissible comparison'
PY
python3 "$P/probes/export_counters_s1b4.py" "$B" "$H" "$PRIVATE_F_EVIDENCE/export-counters" --repo private "$CORPUS_F_ROOT" > "$PRIVATE_F_EVIDENCE/export-counter-counts.json"
shasum -a 256 "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/rowdiff.json" > "$PRIVATE_F_EVIDENCE/rows.sha256"
cat "$PRIVATE_F_EVIDENCE/audit-counts.json" "$PRIVATE_F_EVIDENCE/valueflow-counts.json" "$PRIVATE_F_EVIDENCE/export-counter-counts.json"
# Before acceptance: hand-audit privately EVERY removal/retarget/addition and
# every row the lexical auditor cannot certify. Audit does not infer value flow.
# Return only final class counts, right/wrong/accepted-cost identities, complete
# site populations, stderr sizes, binary/source/rowdiff hashes and custody SHAs.
# Reconcile the historical four-demotion forecast; E7 only changes grades,
# including former MayCall/write grading exceptions. Retain expected rows privately.
# A timed-out aggregate telemetry probe is an exclusion, never a green counter.
