#!/bin/bash
# READ: controller-only private F measurement. Never run this in the planner lane.
# Required env: CORPUS_F_ROOT, PRIVATE_EVIDENCE_ROOT (private destination), TS_JS.
# Usage: bash CONTROLLER-paths.sh BASE_BIN HEAD_BIN BASE_IMPORT_FACTS_BIN
# stdout is aggregates only. Raw paths, rows, source and diagnostics stay private.
set -euo pipefail
: "${CORPUS_F_ROOT:?private root supplied by controller}"
: "${PRIVATE_EVIDENCE_ROOT:?private evidence directory supplied by controller}"
: "${TS_JS:?offline TypeScript 5.9.3 lib/typescript.js}"
BASE="$1"; HEAD="$2"; FACTS="$3"
PROBES="$(cd "$(dirname "$0")" && pwd)"
OUT="$PRIVATE_EVIDENCE_ROOT"
if ! mkdir -p "$OUT" 2>/dev/null; then
  printf '%s\n' '{"claim":"MEASURED","status":"INADMISSIBLE_PRIVATE_OUTPUT_SETUP"}'
  exit 2
fi
exec 2> "$OUT/controller.stderr"
"$BASE" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/private-base.jsonl" 2> "$OUT/private-base.stderr"
"$HEAD" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/private-head.jsonl" 2> "$OUT/private-head.stderr"
"$FACTS" "$CORPUS_F_ROOT" > "$OUT/private-imports.jsonl" 2> "$OUT/private-imports.stderr"
python3 "$PROBES/rowdiff.py" "$OUT/private-base.jsonl" "$OUT/private-head.jsonl" "$OUT/private-changes.json" > "$OUT/private-rowdiff.log" 2> "$OUT/private-rowdiff.stderr"
node "$PROBES/oracle.cjs" "$TS_JS" "$CORPUS_F_ROOT" "$OUT/private-base.jsonl" "$OUT/private-imports.jsonl" "$OUT/private" "$OUT/private-changes.json" > "$OUT/private-oracle.log" 2>&1
python3 - "$OUT/private-P0.json" "$OUT/private-base.jsonl" "$OUT/private-head.jsonl" "$PROBES" 2> "$OUT/private-aggregates.stderr" <<'PY'
import json,sys
sys.path.insert(0,sys.argv[4])
from rowdiff import load
a,b=load(sys.argv[2]),load(sys.argv[3]);assert a.keys()==b.keys(), 'private site keys changed'
p=json.load(open(sys.argv[1]));c=p['counts'];classes=p.get('classes',{})
assert c['total_sites']>0, 'zero-site private probe is inadmissible'
assert p['oracle']['version']=='5.9.3', 'oracle version drift'
assert p['oracle']['sha256']=='3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675', 'oracle byte drift'
print(json.dumps({'claim':'MEASURED','corpus':'F','counts':c,'changed_rows':p['changed_rows'],'classes':classes,'keys_added':0,'keys_removed':0,'oracle_version':p['oracle']['version'],'oracle_sha256':p['oracle']['sha256']},sort_keys=True))
if any(k not in ('CORRECT_STATIC_BINDING','CORRECT_STATIC_REFUSAL') and v for k,v in classes.items()):sys.exit(1)
PY
