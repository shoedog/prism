#!/bin/bash
# READ: controller-only F census; stdout is a fixed aggregate schema only.
# Set CORPUS_F_ROOT and a NEW PRIVATE_EVIDENCE_ROOT privately. No source writes.
# Usage: bash CONTROLLER-s2.sh BASE_BIN FACTS_BIN TS_JS
set -eEuo pipefail
umask 077
exec 3>&1
exec 2>/dev/null
STAGE=arguments
trap 'printf "{\"claim\":\"MEASURED\",\"status\":\"INADMISSIBLE\",\"stage\":\"%s\"}\n" "$STAGE" >&3' ERR
if [ "$#" -ne 3 ] || [ -z "${CORPUS_F_ROOT:-}" ] || [ -z "${PRIVATE_EVIDENCE_ROOT:-}" ]; then
  printf '{"claim":"MEASURED","status":"INADMISSIBLE","stage":"arguments"}\n' >&3
  exit 2
fi
PACKET="$(cd "$(dirname "$0")" && pwd)"
OUT="$PRIVATE_EVIDENCE_ROOT"
if [ -e "$OUT" ]; then
  printf '{"claim":"MEASURED","status":"INADMISSIBLE","stage":"evidence_directory_exists"}\n' >&3
  exit 2
fi
mkdir -p "$OUT"
exec 2> "$OUT/controller.stderr"
exec 1> "$OUT/controller.log"
STAGE=binary_binding
python3 - "$PACKET/probes/reference-binaries.json" "$1" "$2" <<'PY'
import hashlib,json,sys
from pathlib import Path
manifest=json.loads(Path(sys.argv[1]).read_text())
assert hashlib.sha256((Path(sys.argv[1]).parent/'census.cjs').read_bytes()).hexdigest()==manifest['census_sha256'], 'oracle drift'
for role,p in zip(('base','facts'),sys.argv[2:]):
    assert hashlib.sha256(Path(p).read_bytes()).hexdigest()==manifest['binaries'][role]['sha256'], 'binary drift'
PY
STAGE=base_census
"$1" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/base-sites.jsonl"
"$2" "$CORPUS_F_ROOT" > "$OUT/facts.jsonl"
STAGE=native_oracle
node "$PACKET/probes/census.cjs" "$3" "$CORPUS_F_ROOT" "$OUT/base-sites.jsonl" "$OUT/facts.jsonl" "$OUT"
STAGE=publish_aggregates
python3 - "$OUT/summary.json" <<'PY' >&3
import json,sys
from pathlib import Path
s=json.loads(Path(sys.argv[1]).read_text())
# Never publish keys, paths, names, config names, source/binary hashes or logs.
allowed=('total_sites','source_files','s2_sites','low_sites','callable_low','positive_filter_ceiling',
         'mechanisms','binding_kinds','terminal_classes','exclusions','unjoinable','unjoinable_reasons')
assert s['total_sites']>0
print(json.dumps({'claim':'MEASURED','status':'COMPLETE','corpus':'F',**{k:s[k] for k in allowed}},sort_keys=True))
PY
