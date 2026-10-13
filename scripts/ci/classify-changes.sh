#!/usr/bin/env bash
# Classify a change for CI. Reads changed paths on stdin, one per line (produce them with
# `git diff --name-only --no-renames` so a rename lists both its old and new path), and prints
# `code=false` only when every path is documentation that neither the build nor any test reads.
# Anything else, including an empty list, prints `code=true`: the full gate set is the default.
set -euo pipefail

code=false
count=0
while IFS= read -r path; do
  [ -n "$path" ] || continue
  count=$((count + 1))
  case "$path" in
    docs/eval/*.md | docs/contracts/*.md) ;;
    # Data files here are read by the build or by tests: include_str! fixtures, the targets schema.
    docs/eval/* | docs/contracts/*) code=true ;;
    docs/*) ;;
    # Parsed by tests/cli/readme_test.rs and checked by the Language Coverage Matrix job.
    README.md) code=true ;;
    */*) code=true ;;
    # Root-level Markdown (CLAUDE.md, AGENTS.md, plans, status notes).
    *.md) ;;
    *) code=true ;;
  esac
done
[ "$count" -gt 0 ] || code=true
echo "code=$code"
