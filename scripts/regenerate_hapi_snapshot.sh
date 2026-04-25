#!/usr/bin/env bash
# Regenerate human-readable text + JSON snapshots of `--algorithm review` output
# for the hapi-4552 fixture. Used for inspection only — the
# integration_hapi_regression test does NOT assert against either file.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

cargo run --quiet -- \
    --repo tests/fixtures/hapi-4552-source \
    --diff tests/fixtures/hapi-4552.diff \
    --algorithm review \
    --format text \
  > tests/fixtures/hapi-4552-output.txt

cargo run --quiet -- \
    --repo tests/fixtures/hapi-4552-source \
    --diff tests/fixtures/hapi-4552.diff \
    --algorithm review \
    --format json \
  > tests/fixtures/hapi-4552-output.json

echo "Regenerated:"
echo "  tests/fixtures/hapi-4552-output.txt"
echo "  tests/fixtures/hapi-4552-output.json"
