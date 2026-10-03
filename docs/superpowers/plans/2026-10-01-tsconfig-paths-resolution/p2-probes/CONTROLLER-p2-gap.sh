#!/bin/bash
# Controller-only. One JSON object on stdout, including on inadmissible setup.
# Usage: CORPUS_F_ROOT=... PRIVATE_EVIDENCE_ROOT=NEW_DIR TS_JS=... bash CONTROLLER-p2-gap.sh P1_BIN FACTS_BIN P2_BIN
set -eEuo pipefail
umask 077
exec 3>&1
STAGE=setup
trap 'printf "{\"status\":\"INADMISSIBLE\",\"stage\":\"%s\"}\n" "$STAGE" >&3' ERR
if [ -z "${CORPUS_F_ROOT:-}" ] || [ -z "${PRIVATE_EVIDENCE_ROOT:-}" ] || [ -z "${TS_JS:-}" ] || [ "$#" -ne 3 ]; then
  printf '{"status":"INADMISSIBLE","stage":"arguments"}\n' >&3
  exit 1
fi
PROBES="$(cd "$(dirname "$0")" && pwd)"
OUT="$PRIVATE_EVIDENCE_ROOT"
# Refuse evidence reuse; old/private input must never be silently mixed in.
if [ -e "$OUT" ]; then printf '{"status":"INADMISSIBLE","stage":"evidence_directory_exists"}\n' >&3; exit 1; fi
mkdir -p "$OUT"
exec 2> "$OUT/gap-controller.stderr"
exec 1> "$OUT/gap-controller.log"
STAGE=source_and_binary_binding
python3 - "$PROBES" "$1" "$2" "$3" <<'PY'
import hashlib,json,sys
from pathlib import Path
p=Path(sys.argv[1]);m=json.loads((p/'gap-source-binding.json').read_text())
sha=lambda f:hashlib.sha256(Path(f).read_bytes()).hexdigest()
for role,f in zip(['p1','facts','p2'],sys.argv[2:]):
 assert sha(f)==m['binary_hashes'][role], 'supplied binary drift'
PY
STAGE=prototype_and_native_oracle
bash "$PROBES/CONTROLLER-p2.sh" "$1" "$2" "$3" > "$OUT/old-wrapper-aggregate.json"
STAGE=diagnostic_build
python3 "$PROBES/build-gap-driver.py" "$OUT/gap-driver-build"
STAGE=requests
python3 - "$OUT" "$PROBES" <<'PY'
import json,sys,shutil
from pathlib import Path
out,p=map(Path,sys.argv[1:]);rs=json.loads((out/'F-native-rows.json').read_text())
ks={tuple(r['key']) for r in rs if r['reason']=='JS_EXPORT_HOP' and r['native_callable'] and r['ownership_agrees']}
a=json.loads((out/'F-alias-sites.json').read_text())
(out/'gap-requests.json').write_text(json.dumps([r for r in a if tuple(r['key']) in ks]))
shutil.copyfile(p/'gap-class-catalog.json',out/'gap-class-catalog.json')
shutil.copyfile(p/'gap-source-binding.json',out/'gap-source-binding.json')
PY
STAGE=prism_sidecar
"$OUT/gap-driver-build/gap-driver" "$CORPUS_F_ROOT" "$OUT/gap-requests.json" > "$OUT/gap-sidecar.json"
STAGE=classify
node "$PROBES/gap.cjs" "$TS_JS" "$CORPUS_F_ROOT" "$OUT" > "$OUT/gap-aggregate.json.tmp"
# Publish only the fixed aggregate schema, never logs, rows, paths or corpus hashes.
cat "$OUT/gap-aggregate.json.tmp" >&3
