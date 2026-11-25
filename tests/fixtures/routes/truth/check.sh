#!/bin/sh
set -eu

here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd -P)
if [ "$#" -gt 2 ]; then
    printf '%s\n' "Usage: $0 [fixture-root [committed-truth.jsonl]]" >&2
    exit 2
fi
root=${1:-$(dirname -- "$here")}
expected=${2:-"$here/truth.jsonl"}
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

python3 -B "$here/run_truth.py" --root "$root" --rows "$root/rows.jsonl" \
    --verify-sources --out "$tmp/truth.jsonl"
diff -u "$expected" "$tmp/truth.jsonl"
