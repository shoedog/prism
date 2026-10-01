#!/bin/bash
# Controller only. Never execute in a planner/implementer session.
# Set the private root/evidence path privately. Returns no F paths/names/source.
set -euo pipefail
: "${CORPUS_F_ROOT:?controller sets the private corpus root}"
: "${PRIVATE_F_EVIDENCE:?controller sets a private durable evidence directory}"
P=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$P" rev-parse --show-toplevel)
B="$ROOT/target/plan-s1b4/base/prism"
H="$ROOT/target/plan-s1b4/head/prism-r3-final"
M="$ROOT/target/plan-s1b4/BUILD-MANIFEST.json"
test -x "$B"; test -x "$H"; test -s "$M"
# Reject overwritten/unbound executables before private measurement.
python3 - "$M" "$B" "$H" "$ROOT/target/plan-s1b4/proto" <<'PY'
import hashlib,json,sys
m=json.load(open(sys.argv[1]))
for lane,path in zip(('base','head'),sys.argv[2:]):
 assert hashlib.sha256(open(path,'rb').read()).hexdigest()==m[lane+'_binary_sha256'],lane+' binary custody mismatch'
assert m['source_base_sha']=='915fca43d84ea1730959453091fbf8ae97763af8'
assert m['proto_head'].startswith('fceb0b4e') and m['measurement_round']=='r3'
assert m['head_binary_relative_path']=='target/plan-s1b4/head/prism-r3-final'
assert m['cache_versions']==[106,62]
from pathlib import Path
root=Path(sys.argv[4])
for name,digest in m['owned_file_sha256'].items():
 assert hashlib.sha256((root/name).read_bytes()).hexdigest()==digest,'owned source custody mismatch: '+name
for name,digest in m['crate_input_sha256'].items():
 assert hashlib.sha256((root/name).read_bytes()).hexdigest()==digest,'crate input custody mismatch: '+name
PY
mkdir -p "$PRIVATE_F_EVIDENCE/base" "$PRIVATE_F_EVIDENCE/head"
cp "$M" "$PRIVATE_F_EVIDENCE/BUILD-MANIFEST.json"
shasum -a 256 "$B" "$H" > "$PRIVATE_F_EVIDENCE/binaries.sha256"
"$B" --version > "$PRIVATE_F_EVIDENCE/base.version"
"$H" --version > "$PRIVATE_F_EVIDENCE/head.version"
git -C "$ROOT" rev-parse HEAD > "$PRIVATE_F_EVIDENCE/plan.head"
git -C "$ROOT/target/plan-s1b4/proto" rev-parse HEAD > "$PRIVATE_F_EVIDENCE/proto.head"
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
# Reconcile the historical four-demotion forecast; retain expected rows privately.
# A timed-out aggregate telemetry probe is an exclusion, never a green counter.
