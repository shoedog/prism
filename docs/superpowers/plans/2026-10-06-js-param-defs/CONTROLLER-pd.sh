#!/bin/bash
# Controller only: never execute against F in planner/implementer/reviewer sessions.
# Lane js-param-defs. Two modes, both aggregate-only on stdout:
#   census : step-0 four-gap census on F (no prism binary needed)
#     CORPUS_F_ROOT=... PRIVATE_EVIDENCE_ROOT=NEW_DIR bash CONTROLLER-pd.sh census TS_JS
#   diff   : base/head DFG + call-site rows, §3.5 steps 1-2 adjudication of every changed row
#     CORPUS_F_ROOT=... PRIVATE_EVIDENCE_ROOT=NEW_DIR bash CONTROLLER-pd.sh diff TS_JS BASE_BIN HEAD_BIN
# TS_JS is the pinned TypeScript 5.9.3 lib/typescript.js
# (~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js).
# Rows, sites and per-row verdicts stay in PRIVATE_EVIDENCE_ROOT; only counts leave.
set -eEuo pipefail
umask 077
exec 3>&1
exec 2>/dev/null
STAGE=arguments
trap 'printf "{\"status\":\"INADMISSIBLE\",\"stage\":\"%s\"}\n" "$STAGE" >&3' ERR
MODE="$1"
test "$MODE" = census -a "$#" -eq 2 -o "$MODE" = diff -a "$#" -eq 4
: "${CORPUS_F_ROOT:?controller supplies private root}"
: "${PRIVATE_EVIDENCE_ROOT:?controller supplies a new private output directory}"
test ! -e "$PRIVATE_EVIDENCE_ROOT"
PACKET="$(cd "$(dirname "$0")" && pwd)"
PROBES="$PACKET/probes"
OUT="$PRIVATE_EVIDENCE_ROOT"
mkdir -p "$OUT"
exec 1> "$OUT/controller.log"
exec 2> "$OUT/controller.stderr"
STAGE=binding
python3 - "$OUT" "$PROBES" "${@:2}" <<'PY'
import hashlib, json, sys
from pathlib import Path
out, probes = Path(sys.argv[1]), Path(sys.argv[2])
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
b = {'inputs': {str(Path(p).resolve()): sha(p) for p in sys.argv[3:]},
     'probes': {p.name: sha(p) for p in sorted(probes.iterdir()) if p.suffix in ('.cjs', '.py')}}
(out / 'binding.json').write_text(json.dumps(b, indent=2, sort_keys=True) + '\n')
PY
STAGE=census
node --max-old-space-size=8000 "$PROBES/census.cjs" "$2" "$CORPUS_F_ROOT" "$OUT/census.json"
if [ "$MODE" = diff ]; then
  STAGE=base_rows
  "$3" nav --no-cache dfg-stats --repo "$CORPUS_F_ROOT" --edges | LC_ALL=C sort > "$OUT/base.dfg.jsonl"
  "$3" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/base.sites.jsonl"
  STAGE=head_rows
  "$4" nav --no-cache dfg-stats --repo "$CORPUS_F_ROOT" --edges | LC_ALL=C sort > "$OUT/head.dfg.jsonl"
  "$4" nav --no-cache call-stats --repo "$CORPUS_F_ROOT" --dump-sites > "$OUT/head.sites.jsonl"
  STAGE=row_diff
  python3 "$PROBES/rowdiff.py" "$OUT/base.dfg.jsonl" "$OUT/head.dfg.jsonl" "$OUT/dfg-diff.json" --rows "$OUT/dfg-changed.jsonl"
  STAGE=adjudication
  node --max-old-space-size=8000 "$PROBES/adjudicate.cjs" "$2" "$CORPUS_F_ROOT" "$OUT/dfg-changed.jsonl" "$OUT/adjudication.json" --details "$OUT/adjudication-details.jsonl"
fi
STAGE=publish_aggregates
python3 - "$OUT" "$MODE" <<'PY' >&3
import hashlib, json, sys
from pathlib import Path
out, mode = Path(sys.argv[1]), sys.argv[2]
b = json.loads((out / 'binding.json').read_text())
assert all(hashlib.sha256(Path(p).read_bytes()).hexdigest() == h for p, h in b['inputs'].items()), 'input changed during measurement'
census = json.loads((out / 'census.json').read_text())
assert census['files'] > 0
result = {'status': 'COMPLETE', 'corpus': 'F', 'mode': mode, 'census': census}
if mode == 'diff':
    diff = json.loads((out / 'dfg-diff.json').read_text())
    adj = json.loads((out / 'adjudication.json').read_text())
    sites_identical = (out / 'base.sites.jsonl').read_bytes() == (out / 'head.sites.jsonl').read_bytes()
    stops = []
    if diff.get('LOST', 0):
        stops.append('LOST rows present: adjudicate each (LOST|...|CORRECT is a STOP)')
    if any(k.startswith('LOST|') and '|CORRECT' in k for k in adj):
        stops.append('LOST correct row')
    if any(k.startswith('ADDED|') and k.endswith('|WRONG') for k in adj):
        stops.append('ADDED WRONG binding')
    if diff.get('RELABELLED:nameonly->exact', 0):
        stops.append('RELABELLED NameOnly -> Exact (unsafe direction)')
    if not sites_identical:
        stops.append('call-site rows changed (PR-A must not change call resolution)')
    # EXACT_PRIOR_WRITE rows are not a STOP by themselves: the planner checks each against the
    # same-shape plain-parameter control (SPEC D13); the count is published for that review.
    result.update({'dfg_diff': diff, 'adjudication': adj, 'call_sites_identical': sites_identical,
                   'status': 'STOP' if stops else 'COMPLETE', 'stops': stops})
print(json.dumps(result, sort_keys=True))
PY
