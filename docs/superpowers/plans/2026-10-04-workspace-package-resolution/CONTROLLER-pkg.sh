#!/bin/bash
# Controller only: never execute against F in planner/implementer sessions.
# CORPUS_F_ROOT=... PRIVATE_EVIDENCE_ROOT=NEW_DIR bash CONTROLLER-pkg.sh BASE_BIN HEAD_BIN BASE_FACTS_BIN HEAD_FACTS_BIN TS_JS
# Public stdout is aggregate counts only; private details stay in the private output.
set -eEuo pipefail
umask 077
exec 3>&1
exec 2>/dev/null
STAGE=arguments
trap 'printf "{\"status\":\"INADMISSIBLE\",\"stage\":\"%s\"}\n" "$STAGE" >&3' ERR
test "$#" -eq 5
: "${CORPUS_F_ROOT:?controller supplies private root}"
: "${PRIVATE_EVIDENCE_ROOT:?controller supplies a new private output directory}"
test ! -e "$PRIVATE_EVIDENCE_ROOT"
PACKET="$(cd "$(dirname "$0")" && pwd)"
OUT="$PRIVATE_EVIDENCE_ROOT"
mkdir -p "$OUT"
exec 1> "$OUT/controller.log"
exec 2> "$OUT/controller.stderr"
STAGE=binary_binding
python3 - "$PACKET" "$OUT" "$1" "$2" "$3" "$4" <<'PY'
import hashlib,json,sys
from pathlib import Path
p,out=map(Path,sys.argv[1:3]);m=json.loads((p/'BUILD-MANIFEST.json').read_text())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
b={name:{'path':str(Path(value).resolve()),'sha256':sha(value)} for name,value in zip(('base','head','basefacts','headfacts'),sys.argv[3:])}
for name,v in b.items():assert v['sha256']==m['binaries'][name]['sha256'], 'binary drift'
for name in ('census.cjs','compare.cjs'):assert sha(p/'probes'/name)==m['probes'][name], 'oracle drift'
(out/'binary-binding.json').write_text(json.dumps(b,indent=2)+'\n')
PY
STAGE=base_rows
"$1" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/base-sites.jsonl"
STAGE=head_rows
"$2" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/head-sites.jsonl"
"$3" "$CORPUS_F_ROOT" > "$OUT/base-facts.jsonl"
"$4" "$CORPUS_F_ROOT" > "$OUT/facts.jsonl"
STAGE=native_census
node "$PACKET/probes/census.cjs" "$5" "$CORPUS_F_ROOT" "$OUT/base-sites.jsonl" "$OUT/base-facts.jsonl" "$OUT"
STAGE=changed_rows
node "$PACKET/probes/compare.cjs" "$5" "$CORPUS_F_ROOT" "$OUT/base-sites.jsonl" "$OUT/head-sites.jsonl" "$OUT/facts.jsonl" "$OUT"
STAGE=publish_aggregates
python3 - "$OUT" <<'PY' >&3
import hashlib,json,sys
from pathlib import Path
out=Path(sys.argv[1]);b=json.loads((out/'binary-binding.json').read_text())
assert all(hashlib.sha256(Path(v['path']).read_bytes()).hexdigest()==v['sha256'] for v in b.values()), 'binary changed during measurement'
c=json.loads((out/'census.json').read_text())['summary'];v=json.loads((out/'comparison.json').read_text())['summary']
assert c['sites']>0 and v['sites']==c['sites']
assert v['unproven']==v['lost_base_edges']==0
print(json.dumps({'status':'COMPLETE','corpus':'F','census':c,'prototype':v},sort_keys=True))
PY
