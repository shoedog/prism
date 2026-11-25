#!/usr/bin/env bash
# Re-run the truth runner into a temp file and diff it against the committed truth.jsonl.
# Exit 0: identical (and no row inadmissible). Exit 1: drift or an inadmissible row.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fixtures="$(dirname "$here")"
tmp="$(mktemp -d "${TMPDIR:-/tmp}/check-truth-XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
status=0
node "$here/run_truth.mjs" "$fixtures/cases" "$fixtures/rows.jsonl" "$here/families.json" --out "$tmp/truth.jsonl" || status=$?
if [ "$status" -ge 2 ]; then
  echo "check.sh: the runner failed (exit $status)" >&2
  exit 1
fi
if ! diff -u "$here/truth.jsonl" "$tmp/truth.jsonl"; then
  echo "check.sh: truth.jsonl differs from a fresh run" >&2
  exit 1
fi
if [ "$status" -ne 0 ]; then
  echo "check.sh: truth.jsonl matches, but a row is inadmissible" >&2
  exit 1
fi
echo "check.sh: truth.jsonl is current"
