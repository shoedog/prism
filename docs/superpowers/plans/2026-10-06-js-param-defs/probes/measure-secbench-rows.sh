#!/bin/bash
# SecBench per-package base/head DFG + call-site rows (planner; public packages only).
# Usage: measure-secbench-rows.sh BASE_BIN HEAD_BIN OUT_DIR [JOBS]
# Each authenticated package root (manifest status ok) is measured separately; row `file`
# fields are rewritten to `<class>/<entry>/src/package/<file>` so rows from different packages
# never merge and adjudicate.cjs can resolve them against the packages root.
set -euo pipefail
BASE="$1"; HEAD="$2"; OUT="$3"; JOBS="${4:-6}"
PKGS="$HOME/prism-evidence/inputs/secbench-pkgs"
PACKET="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$OUT/per"
python3 - "$PKGS" > "$OUT/roots.txt" <<'PY'
import json, sys
from pathlib import Path
root = Path(sys.argv[1])
for e in json.loads((root / 'manifest.json').read_text())['entries']:
    if e['status'] == 'ok':
        print(f"{e['class']}/{e['entry']}")
PY
run_one() {
  local rel="$1" side="$2" bin="$3" tag="${1//\//__}"
  local repo="$PKGS/$rel/src/package"
  timeout 300 "$bin" nav --no-cache dfg-stats --repo "$repo" --edges 2>/dev/null > "$OUT/per/$tag.$side.dfg" || echo "$rel $side dfg_failed $?" >> "$OUT/failures.txt"
  timeout 300 "$bin" nav --no-cache call-stats --repo "$repo" --dump-sites 2>/dev/null > "$OUT/per/$tag.$side.sites" || echo "$rel $side sites_failed $?" >> "$OUT/failures.txt"
}
export -f run_one
export OUT PKGS
: > "$OUT/failures.txt"
while read -r rel; do
  echo "$rel base $BASE"; echo "$rel head $HEAD"
done < "$OUT/roots.txt" | xargs -P "$JOBS" -n 3 bash -c 'run_one "$0" "$1" "$2"'
python3 - "$OUT" <<'PY'
import json, sys
from pathlib import Path
out = Path(sys.argv[1])
roots = out.joinpath('roots.txt').read_text().split()
sites_diff = []
for side in ('base', 'head'):
    rows = []
    for rel in roots:
        tag = rel.replace('/', '__')
        f = out / 'per' / f'{tag}.{side}.dfg'
        for line in f.read_text().splitlines() if f.exists() else []:
            r = json.loads(line)
            for end in ('from', 'to'):
                r[end]['file'] = f"{rel}/src/package/{r[end]['file']}"
            rows.append(json.dumps(r, separators=(',', ':')))
    rows.sort()
    (out / f'{side}.dfg.jsonl').write_text(''.join(r + '\n' for r in rows))
for rel in roots:
    tag = rel.replace('/', '__')
    b, h = out / 'per' / f'{tag}.base.sites', out / 'per' / f'{tag}.head.sites'
    if not b.exists() or not h.exists() or b.read_bytes() != h.read_bytes():
        sites_diff.append(rel)
(out / 'sites-diff.txt').write_text(''.join(r + '\n' for r in sites_diff))
print(json.dumps({'packages': len(roots), 'call_site_packages_differing': len(sites_diff),
                  'failures': len(out.joinpath('failures.txt').read_text().splitlines())}))
PY
python3 "$PACKET/rowdiff.py" "$OUT/base.dfg.jsonl" "$OUT/head.dfg.jsonl" "$OUT/dfg-diff.json" --rows "$OUT/dfg-changed.jsonl"
