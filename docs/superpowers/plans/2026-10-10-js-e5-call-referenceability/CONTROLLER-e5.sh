#!/bin/bash
# Controller-only private capture; planner tests only synthetic PUBLIC data.
# census TS_JS BASE_BIN BASE_FACTS_BIN
# diff TS_JS BASE_BIN HEAD_BIN BASE_FACTS_BIN HEAD_FACTS_BIN BASE_GRAPH_BIN HEAD_GRAPH_BIN
# Supply CORPUS_F_ROOT and a NEW PRIVATE_EVIDENCE_ROOT. Aggregate-only stdout.
set -eEuo pipefail
umask 077
exec 3>&1
exec 2>/dev/null
STAGE=arguments
trap 'printf "{\"status\":\"INADMISSIBLE\",\"stage\":\"%s\"}\n" "$STAGE" >&3' ERR
MODE="${1:-}"
case "$MODE:$#" in census:4|diff:8) ;; *) false ;; esac
: "${CORPUS_F_ROOT:?controller supplies private root}"
: "${PRIVATE_EVIDENCE_ROOT:?controller supplies NEW evidence directory}"
test ! -e "$PRIVATE_EVIDENCE_ROOT"
PACKET="$(cd "$(dirname "$0")" && pwd)"
PROBES="$PACKET/probes"
OUT="$PRIVATE_EVIDENCE_ROOT"
mkdir -p "$OUT"
exec 1>"$OUT/controller.log"
exec 2>"$OUT/controller.stderr"
STAGE=binding
python3 - "$OUT" "$PROBES" "${@:2}" <<'PY'
import hashlib,json,sys,os
from pathlib import Path
out,probes=map(Path,sys.argv[1:3]);sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
b={'inputs':{str(Path(p).resolve()):sha(p) for p in sys.argv[3:]},'probes':{p.name:sha(p) for p in probes.iterdir() if p.is_file()}}
(out/'binding.json').write_text(json.dumps(b,indent=2)+'\n')
sys.path.insert(0,str(probes));from capture import check_root
check_root(os.environ['CORPUS_F_ROOT'],controller_private=True)
PY
produce() {
  python3 - "$PROBES" "$OUT/$1" "${@:2}" <<'PY'
import json,sys
from pathlib import Path
sys.path.insert(0,sys.argv[1]);from capture import run
out=Path(sys.argv[2]);r=run(sys.argv[3:],out,1500)
out.with_suffix(out.suffix+'.receipt.json').write_text(json.dumps(r,indent=2)+'\n')
if r['exit']:raise SystemExit(r['exit'])
PY
}
STAGE=base_sites
produce base.sites.jsonl "$3" nav --cache-dir "$HOME/prism-evidence/js-e5/cache" call-stats --repo "$CORPUS_F_ROOT" --dump-sites
if [ "$MODE" = census ]; then BASE_FACTS="$4"; else BASE_FACTS="$5"; fi
STAGE=base_facts
produce base.facts.jsonl "$BASE_FACTS" "$CORPUS_F_ROOT"
STAGE=base_census
node --max-old-space-size=8000 "$PROBES/census-e5.cjs" "$2" "$CORPUS_F_ROOT" "$OUT/base.sites.jsonl" "$OUT/base.census.jsonl" --facts "$OUT/base.facts.jsonl" --controller-private
if [ "$MODE" = diff ]; then
  STAGE=head_sites
  produce head.sites.jsonl "$4" nav --cache-dir "$HOME/prism-evidence/js-e5/cache" call-stats --repo "$CORPUS_F_ROOT" --dump-sites
  STAGE=head_facts
  produce head.facts.jsonl "$6" "$CORPUS_F_ROOT"
  STAGE=head_census
  node --max-old-space-size=8000 "$PROBES/census-e5.cjs" "$2" "$CORPUS_F_ROOT" "$OUT/head.sites.jsonl" "$OUT/head.census.jsonl" --facts "$OUT/head.facts.jsonl" --controller-private
  STAGE=call_comparison
  python3 "$PROBES/compare-e5.py" "$OUT/base.sites.jsonl" "$OUT/head.sites.jsonl" "$OUT/base.census.jsonl" "$OUT/head.census.jsonl" "$OUT/comparison"
  for SIDE in base head; do
    node --max-old-space-size=8000 "$PROBES/census-e5.cjs" "$2" "$CORPUS_F_ROOT" "$OUT/comparison/$SIDE.changed-sites.jsonl" "$OUT/comparison/$SIDE.changed-census.jsonl" --all-resolved --facts "$OUT/$SIDE.facts.jsonl" --controller-private
  done
  python3 "$PROBES/compare-e5.py" "$OUT/base.sites.jsonl" "$OUT/head.sites.jsonl" "$OUT/comparison/base.changed-census.jsonl" "$OUT/comparison/head.changed-census.jsonl" "$OUT/comparison"
  STAGE=graph_rows
  produce base.graph.jsonl "$7" "$CORPUS_F_ROOT"
  produce head.graph.jsonl "$8" "$CORPUS_F_ROOT"
  python3 - "$OUT" <<'PY'
import json,sys
from pathlib import Path
from collections import Counter
out=Path(sys.argv[1]);b=Counter((out/'base.graph.jsonl').read_text().splitlines());h=Counter((out/'head.graph.jsonl').read_text().splitlines());counts=Counter();changes=[]
for kind,rows in [('removed',b-h),('added',h-b)]:
 for text,n in sorted(rows.items()):
  row=json.loads(text);counts[kind+'|'+row['kind']]+=n
  changes.extend({'change':kind,'row':row} for _ in range(n))
(out/'graph.changes.jsonl').write_text(''.join(json.dumps(r,sort_keys=True)+'\n' for r in changes));(out/'graph.comparison.json').write_text(json.dumps(dict(counts))+'\n')
PY
  STAGE=graph_adjudication
  node --max-old-space-size=8000 "$PROBES/adjudicate-graphs.cjs" "$2" "$CORPUS_F_ROOT" "$OUT/graph.changes.jsonl" "$OUT/comparison/target-changes.jsonl" "$OUT/adjudication" --controller-private
fi
STAGE=publish_aggregates
python3 - "$OUT" "$MODE" "$PROBES" <<'PY' >&3
import hashlib,json,subprocess,sys
from pathlib import Path
out=Path(sys.argv[1]);binding=json.loads((out/'binding.json').read_text())
assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest()==h for p,h in binding['inputs'].items()),'input bytes changed'
assert all(hashlib.sha256((Path(sys.argv[3])/p).read_bytes()).hexdigest()==h for p,h in binding['probes'].items()),'probe bytes changed'
summary=lambda p:json.loads(subprocess.check_output(['node',str(Path(sys.argv[3])/'controller-summary.cjs'),str(p)]))
result={'corpus':'F','mode':sys.argv[2],'status':'COMPLETE','base':summary(out/'base.census.jsonl')}
if sys.argv[2]=='diff':
 result.update(head=summary(out/'head.census.jsonl'),comparison=json.loads((out/'comparison/comparison.json').read_text()),graph_changes=json.loads((out/'graph.comparison.json').read_text()),graph_adjudication=json.loads((out/'adjudication/graph.adjudication-summary.json').read_text()))
print(json.dumps(result,sort_keys=True))
PY
