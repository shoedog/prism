#!/usr/bin/env bash
# Tests for classify-changes.sh. Run: bash scripts/ci/classify-changes.test.sh
set -euo pipefail

classify="$(cd "$(dirname "$0")" && pwd)/classify-changes.sh"
failures=0

check() {
  local expected="$1" name="$2" actual
  actual="$(bash "$classify")"
  if [ "$actual" = "code=$expected" ]; then
    echo "ok   $name"
  else
    echo "FAIL $name: expected code=$expected, got $actual"
    failures=$((failures + 1))
  fi
}

# Documentation only: the reduced gate set.
check false "plan packet markdown" <<'EOF'
docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prB.md
EOF
check false "plan packet probes are not built or tested" <<'EOF'
docs/superpowers/plans/2026-10-06-js-param-defs/probes/census.cjs
docs/superpowers/plans/2026-10-06-js-param-defs/MEASUREMENTS.json
EOF
check false "root-level markdown" <<'EOF'
CLAUDE.md
AGENTS.md
docs/MCP.md
EOF
check false "markdown beside the embedded fixtures" <<'EOF2'
docs/eval/jsts-workstream-value/README.md
docs/contracts/NOTES.md
EOF2

# Anything the build or a test reads: the full gate set.
check true "Rust source" <<'EOF'
src/cpg.rs
EOF
check true "docs mixed with source" <<'EOF'
docs/MCP.md
src/mcp/mod.rs
EOF
check true "docs mixed with the eval harness" <<'EOF'
docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prB.md
eval/secbench/run.py
EOF
check true "fixtures embedded with include_str!" <<'EOF'
docs/eval/receiver-closure/executable-owner-fixtures.json
EOF
check true "targets schema read by tests" <<'EOF'
docs/contracts/targets.schema.json
EOF
check true "README is parsed by tests" <<'EOF'
README.md
EOF
check true "skill text is parsed by tests" <<'EOF'
skills/prism-code-slicing/SKILL.md
EOF
check true "markdown outside docs/ and the root" <<'EOF'
tests/fixtures/notes.md
EOF
check true "workflow change" <<'EOF'
.github/workflows/ci.yml
EOF
check true "root-level non-markdown" <<'EOF'
Cargo.lock
EOF
check true "a rename out of src lists the old path" <<'EOF'
docs/moved.md
src/moved.rs
EOF
check true "a directory merely named like docs" <<'EOF'
docs-internal/tool.rs
EOF
check true "empty change list" </dev/null
check true "blank lines only" <<'EOF'


EOF

if [ "$failures" -gt 0 ]; then
  echo "$failures failed"
  exit 1
fi
echo "all passed"
