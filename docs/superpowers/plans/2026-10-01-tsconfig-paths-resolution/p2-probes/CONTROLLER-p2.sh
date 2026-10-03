#!/bin/bash
# READ: controller-only private measurement. Stdout is aggregate counts and hashes only.
# Usage: CORPUS_F_ROOT=... PRIVATE_EVIDENCE_ROOT=... TS_JS=... bash CONTROLLER-p2.sh P1_BIN FACTS_BIN [REPAIRED_P2_BIN] [PRIOR_GAP_EVIDENCE]
# Set ACCEPTED_P2_BIN to the accepted b9fd3775 binary for complete-row repair-impact parity.
# The default P2 binary is the frozen Opus round-1 repair; historical binaries require an explicit argument.
set -euo pipefail
: "${CORPUS_F_ROOT:?controller supplies private root}"
: "${PRIVATE_EVIDENCE_ROOT:?controller supplies private output directory}"
: "${TS_JS:?offline TypeScript 5.9.3 lib/typescript.js}"
P1="${1:?P1 binary}"; FACTS="${2:?import facts binary}"
REPO="$(cd "$(dirname "$0")/../../../../.." && pwd)"
P2="${3:-$REPO/target/p2-spec-r1/bin/prism-p2-spec-r1}"; PRIOR_GAP="${4:-}"
ACCEPTED="${ACCEPTED_P2_BIN:-}"
PROBES="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$PRIVATE_EVIDENCE_ROOT"
OUT="$PRIVATE_EVIDENCE_ROOT"
exec 2> "$OUT/controller-p2.stderr"
"$P1" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/p1-sites.jsonl" 2> "$OUT/p1-sites.stderr"
"$FACTS" "$CORPUS_F_ROOT" > "$OUT/import-facts.jsonl" 2> "$OUT/import-facts.stderr"
node "$PROBES/../probes/oracle.cjs" "$TS_JS" "$CORPUS_F_ROOT" "$OUT/p1-sites.jsonl" "$OUT/import-facts.jsonl" "$OUT/F" > "$OUT/oracle.log" 2>&1
node "$PROBES/native.cjs" "$TS_JS" "$CORPUS_F_ROOT" "$OUT/F-alias-sites.json" "$OUT/import-facts.jsonl" "$OUT/F" > "$OUT/native.log" 2>&1
if [ -n "$P2" ]; then
  "$P2" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/p2-sites.jsonl" 2> "$OUT/p2-sites.stderr"
  python3 "$PROBES/compare.py" "$OUT/p1-sites.jsonl" "$OUT/p2-sites.jsonl" "$OUT/F-native-rows.json" "$OUT/p2-comparison" > "$OUT/comparison.log"
fi
if [ -n "$ACCEPTED" ]; then
  "$ACCEPTED" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/accepted-p2-sites.jsonl" 2> "$OUT/accepted-p2-sites.stderr"
fi
node "$PROBES/hop-audit.cjs" "$TS_JS" "$CORPUS_F_ROOT" "$OUT" ${PRIOR_GAP:+"$PRIOR_GAP"} > "$OUT/hop-audit.log" 2>&1
python3 - "$OUT" "$P1" "$FACTS" "$PROBES/native.cjs" "$PROBES/../probes/oracle.cjs" "$P2" "$PROBES/compare.py" "$ACCEPTED" <<'PY'
import json,sys,hashlib
from pathlib import Path
out=Path(sys.argv[1]); d=json.load(open(out/'F-native-summary.json')); p=json.load(open(out/'F-P0.json'))
assert p['counts']['total_sites']>0, 'zero-site probe inadmissible'
d.update(corpus='F', total_sites=p['counts']['total_sites'], p1_counts=p['counts'])
d['hop_audit']=json.load(open(out/'hop-audit-aggregate.json'))
d['binary_hashes']={k:hashlib.sha256(Path(v).read_bytes()).hexdigest() for k,v in zip(['p1','facts'],sys.argv[2:4])}
d['probe_hashes']={k:hashlib.sha256(Path(v).read_bytes()).hexdigest() for k,v in zip(['native','p1_oracle'],sys.argv[4:6])}
if sys.argv[6]:
 d['binary_hashes']['p2']=hashlib.sha256(Path(sys.argv[6]).read_bytes()).hexdigest()
 d['probe_hashes']['comparison']=hashlib.sha256(Path(sys.argv[7]).read_bytes()).hexdigest()
 d['prototype']=json.load(open(out/'p2-comparison.json'))
if sys.argv[8]:
 sys.path.insert(0,str(Path(sys.argv[7]).parent))
 from compare import load
 a,b=load(out/'accepted-p2-sites.jsonl'),load(out/'p2-sites.jsonl')
 assert a.keys()==b.keys(), 'repair changed site population'
 changed=sum(a[k]!=b[k] for k in a)
 d['repair_impact']={'sites':len(a),'changed_rows':changed,'keys_added':0,'keys_removed':0,'expected_changed_rows':0}
 d['binary_hashes']['accepted_p2']=hashlib.sha256(Path(sys.argv[8]).read_bytes()).hexdigest()
 assert changed==0, 'nonzero round-1 F repair impact'
(out/'F-aggregate.json').write_text(json.dumps(d,indent=2)+'\n')
print(json.dumps(d,sort_keys=True))
PY
