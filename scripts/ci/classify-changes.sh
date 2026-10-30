#!/usr/bin/env bash
# Classify a change for CI. Reads changed paths on stdin, one per line (produce them with
# `git diff --name-only --no-renames` so a rename lists both its old and new path), and prints
# two lines:
#   code=<bool>  the Rust test, build, clippy and format jobs
#   eval=<bool>  the Python eval-harness test job
# `code=false` only when every path is documentation or eval-harness code that neither the build
# nor any Rust test reads. `eval=false` only when every path is documentation that no test reads;
# any other change also runs the (cheap, hermetic) Python tests, because they read a few
# repo-level paths outside eval/ (tests/fixtures/c, git history of src/). Anything unrecognised,
# including an empty list, prints `code=true eval=true`: the full gate set is the default.
set -euo pipefail

code=false
eval_changed=false
count=0
while IFS= read -r path; do
  [ -n "$path" ] || continue
  count=$((count + 1))
  case "$path" in
    docs/eval/*.md | docs/contracts/*.md) ;;
    # Data files here are read by the build or by tests: include_str! fixtures, the targets schema.
    docs/eval/* | docs/contracts/*) code=true ;;
    docs/*) ;;
    # Read by Rust tests: tests/cli/min_confidence_test.rs builds paths under eval/fixtures.
    # The Python tests read them too.
    eval/fixtures/*) code=true ;;
    # The rest of the eval harness is Python and data that no Rust code, build script or Cargo
    # manifest reads. It needs only the Python test job.
    eval/*) eval_changed=true ;;
    # Parsed by tests/cli/readme_test.rs and checked by the Language Coverage Matrix job.
    README.md) code=true ;;
    */*) code=true ;;
    # Root-level Markdown (CLAUDE.md, AGENTS.md, plans, status notes).
    *.md) ;;
    *) code=true ;;
  esac
done
[ "$count" -gt 0 ] || code=true
# A change the Rust jobs run for is never skipped by the Python job either.
[ "$code" = false ] || eval_changed=true
echo "code=$code"
echo "eval=$eval_changed"
