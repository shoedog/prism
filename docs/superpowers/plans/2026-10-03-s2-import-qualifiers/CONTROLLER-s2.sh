#!/bin/bash
# READ: controller-only F census; stdout is a fixed aggregate schema only.
# Set CORPUS_F_ROOT and a NEW PRIVATE_EVIDENCE_ROOT privately. No source writes.
# Usage: bash CONTROLLER-s2.sh BASE_BIN HEAD_BIN HEAD_FACTS_BIN TS_JS
set -eEuo pipefail
umask 077
exec 3>&1
exec 2>/dev/null
STAGE=arguments
trap 'printf "{\"claim\":\"MEASURED\",\"status\":\"INADMISSIBLE\",\"stage\":\"%s\"}\n" "$STAGE" >&3' ERR
if [ "$#" -ne 4 ] || [ -z "${CORPUS_F_ROOT:-}" ] || [ -z "${PRIVATE_EVIDENCE_ROOT:-}" ]; then
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
python3 - "$PACKET" "$OUT" "$1" "$2" "$3" <<'PY'
import hashlib,json,sys
from pathlib import Path
packet,out=map(Path,sys.argv[1:3])
manifest=json.loads((packet/'probes/reference-binaries.json').read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
assert sha(packet/'probes/census.cjs')==manifest['census_sha256'], 'oracle drift'
binding={role:{'path':str(Path(p).resolve()),'sha256':sha(p)} for role,p in zip(('base','head','headfacts'),sys.argv[3:])}
assert binding['base']['sha256']==manifest['binaries']['base']['sha256'], 'base binary drift'
adopted=json.loads((packet/'BUILD-MANIFEST.json').read_text())['binaries']
for role,name in [('head','head-prism'),('headfacts','head-dump_imports')]:
    assert binding[role]['sha256']==adopted[name], 'head binary drift'
(out/'binary-binding.json').write_text(json.dumps(binding,indent=2)+'\n')
PY
STAGE=base_census
"$1" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/base-sites.jsonl"
STAGE=head_census
"$2" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/head-sites.jsonl"
"$3" "$CORPUS_F_ROOT" > "$OUT/facts.jsonl"
STAGE=native_oracle
node "$PACKET/probes/census.cjs" "$4" "$CORPUS_F_ROOT" "$OUT/base-sites.jsonl" "$OUT/facts.jsonl" "$OUT"
STAGE=changed_row_correctness
python3 - "$PACKET" "$OUT" <<'PY'
import hashlib,importlib.util,json,sys
from pathlib import Path
packet,out=map(Path,sys.argv[1:])
sys.path.insert(0,str(packet/'probes'))
spec=importlib.util.spec_from_file_location('s2compare',packet/'probes/compare-head.py')
c=importlib.util.module_from_spec(spec);spec.loader.exec_module(c)
c.compare(out)  # Includes complete population, ownership and full terminal span.
binding=json.loads((out/'binary-binding.json').read_text())
assert all(hashlib.sha256(Path(b['path']).read_bytes()).hexdigest()==b['sha256'] for b in binding.values()), 'binary changed during run'
PY
STAGE=publish_aggregates
python3 - "$OUT" <<'PY' >&3
import json,sys
from pathlib import Path
out=Path(sys.argv[1]);s=json.loads((out/'summary.json').read_text())
# Never publish keys, paths, names, config names, source/binary hashes or logs.
allowed=('total_sites','source_files','s2_sites','low_sites','callable_low','positive_filter_ceiling',
         'mechanisms','binding_kinds','terminal_classes','exclusions','unjoinable','unjoinable_reasons')
assert s['total_sites']>0
comparison=json.loads((out/'comparison.json').read_text())
print(json.dumps({'claim':'MEASURED','status':'COMPLETE','corpus':'F',**{k:s[k] for k in allowed},'head_comparison':comparison},sort_keys=True))
PY
