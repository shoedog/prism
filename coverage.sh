#!/usr/bin/env bash
# coverage.sh — Generate test coverage reports using cargo-llvm-cov
#
# Usage:
#   ./coverage.sh              # Run all reports (text, HTML, JSON)
#   ./coverage.sh --check 80   # Fail if overall line coverage is below 80%
#
# Prerequisites:
#   cargo install cargo-llvm-cov
#   rustup component add llvm-tools-preview

set -euo pipefail

CHECK_THRESHOLD=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --check)
            if [[ -z "${2:-}" ]]; then
                echo "Error: --check requires a percentage argument (e.g. --check 80)" >&2
                exit 1
            fi
            CHECK_THRESHOLD="$2"
            shift 2
            ;;
        *)
            echo "Unknown argument: $1" >&2
            echo "Usage: $0 [--check N]" >&2
            exit 1
            ;;
    esac
done

# Verify cargo-llvm-cov is available
if ! cargo llvm-cov --version &>/dev/null; then
    echo "Error: cargo-llvm-cov is not installed." >&2
    echo "Install it with: cargo install cargo-llvm-cov" >&2
    echo "Then add the LLVM tools component: rustup component add llvm-tools-preview" >&2
    exit 1
fi

mkdir -p target/llvm-cov

echo "==> Generating HTML coverage report..."
cargo llvm-cov --html
echo "    Written to: target/llvm-cov/html/index.html"

echo ""
echo "==> Generating JSON coverage report..."
cargo llvm-cov --json --output-path target/llvm-cov/coverage.json
echo "    Written to: target/llvm-cov/coverage.json"

echo ""
echo "==> Coverage summary:"
cargo llvm-cov --text

# Optional threshold check
if [[ -n "$CHECK_THRESHOLD" ]]; then
    echo ""
    echo "==> Checking line coverage >= ${CHECK_THRESHOLD}%..."

    # Extract overall line coverage percentage from JSON output
    ACTUAL=$(python3 -c "
import json, sys
with open('target/llvm-cov/coverage.json') as f:
    d = json.load(f)
totals = d.get('data', [{}])[0].get('totals', {})
lines = totals.get('lines', {})
count = lines.get('count', 0)
covered = lines.get('covered', 0)
if count == 0:
    print('0')
else:
    print(f'{covered / count * 100:.1f}')
" 2>/dev/null || echo "0")

    # Compare using awk (handles floats)
    PASS=$(awk -v actual="$ACTUAL" -v threshold="$CHECK_THRESHOLD" \
        'BEGIN { print (actual + 0 >= threshold + 0) ? "yes" : "no" }')

    if [[ "$PASS" == "yes" ]]; then
        echo "    PASS: ${ACTUAL}% >= ${CHECK_THRESHOLD}%"
    else
        echo "    FAIL: ${ACTUAL}% < ${CHECK_THRESHOLD}%" >&2
        exit 1
    fi
fi
