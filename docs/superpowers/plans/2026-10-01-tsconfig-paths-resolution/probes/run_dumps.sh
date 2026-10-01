#!/bin/bash
# Public defaults only. Private F is exclusively dispatched by CONTROLLER-paths.sh.
set -euo pipefail
B="$1"; OUT="$2"; shift 2
mkdir -p "$OUT"
if [ "$#" -eq 0 ]; then set -- X R T; fi
for c in "$@"; do
  case "$c" in
    X) root="$HOME/prism-evidence/inputs/excalidraw-0642e72c/source";;
    R) root="$HOME/code/bench-repos/ruff/playground";;
    T) root="$HOME/code/bench-repos/TypeScript/src";;
    *) echo 'Public corpus labels only: X R T' >&2; exit 2;;
  esac
  "$B" nav --no-cache call-stats --repo "$root" --dump-sites > "$OUT/$c-dump-sites.jsonl" 2> "$OUT/$c-dump-sites.stderr"
  python3 - "$OUT/$c-dump-sites.jsonl" "$c" <<'PY'
import json, sys
rows=[json.loads(s) for s in open(sys.argv[1])]
sites=[r for r in rows if r.get('record_kind')=='call_site']
assert sites, 'zero sites is inadmissible'
print(sys.argv[2], 'rows',len(sites))
PY
done
