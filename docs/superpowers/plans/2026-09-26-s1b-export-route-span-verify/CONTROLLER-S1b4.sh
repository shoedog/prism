#!/bin/bash
# Controller only. The planner must never execute this or read its private files.
# CORPUS_F_ROOT and PRIVATE_F_EVIDENCE are set privately by the controller.
set -euo pipefail
: "${CORPUS_F_ROOT:?controller sets the private corpus root}"
: "${PRIVATE_F_EVIDENCE:?controller sets a private durable evidence directory}"
P=$(cd "$(dirname "$0")" && pwd)
ROOT=$(git -C "$P" rev-parse --show-toplevel)
B="$ROOT/target/plan-s1b4/base/prism"
H="$ROOT/target/plan-s1b4/head/prism"
test -x "$B"
test -x "$H"
mkdir -p "$PRIVATE_F_EVIDENCE/base" "$PRIVATE_F_EVIDENCE/head"
shasum -a 256 "$B" "$H" > "$PRIVATE_F_EVIDENCE/binaries.sha256"
"$B" --version > "$PRIVATE_F_EVIDENCE/base.version"
"$H" --version > "$PRIVATE_F_EVIDENCE/head.version"
bash "$P/probes/run_dumps.sh" "$B" "$PRIVATE_F_EVIDENCE/base" F > "$PRIVATE_F_EVIDENCE/base.receipt"
bash "$P/probes/run_dumps.sh" "$H" "$PRIVATE_F_EVIDENCE/head" F > "$PRIVATE_F_EVIDENCE/head.receipt"
test -s "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl"
test -s "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl"
python3 "$P/probes/rowdiff.py" "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/rowdiff.json" > "$PRIVATE_F_EVIDENCE/rowdiff.log"
python3 "$P/probes/audit_s1b4.py" "$CORPUS_F_ROOT" "$PRIVATE_F_EVIDENCE/rowdiff.json" "$PRIVATE_F_EVIDENCE/audit.json" --private > "$PRIVATE_F_EVIDENCE/audit-counts.json"
python3 "$P/probes/valueflow_guard_s1b4.py" "$PRIVATE_F_EVIDENCE/base/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/head/F-dump-sites.jsonl" "$PRIVATE_F_EVIDENCE/valueflow.json" --private > "$PRIVATE_F_EVIDENCE/valueflow-counts.json"
cat "$PRIVATE_F_EVIDENCE/audit-counts.json" "$PRIVATE_F_EVIDENCE/valueflow-counts.json"
# Hand-audit every gained/lost identity privately; return final class counts,
# accepted-cost counts, key-completeness, stderr byte counts, source/binary/rowdiff
# hashes and the custody SHAs. Never return paths, names or source from F.
