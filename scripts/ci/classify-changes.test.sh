#!/usr/bin/env bash
# Tests for classify-changes.sh. Run: bash scripts/ci/classify-changes.test.sh
set -euo pipefail

classify="$(cd "$(dirname "$0")" && pwd)/classify-changes.sh"
failures=0

# check <code> <eval> <name>, change list on stdin. Compares both output lines.
check() {
  local expected_code="$1" expected_eval="$2" name="$3" actual expected
  expected="code=$expected_code"$'\n'"eval=$expected_eval"
  actual="$(bash "$classify")"
  if [ "$actual" = "$expected" ]; then
    echo "ok   $name"
  else
    echo "FAIL $name: expected ${expected//$'\n'/ }, got ${actual//$'\n'/ }"
    failures=$((failures + 1))
  fi
}

# Documentation only: the reduced gate set.
check false false "plan packet markdown" <<'EOF'
docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prB.md
EOF
check false false "plan packet probes are not built or tested" <<'EOF'
docs/superpowers/plans/2026-10-06-js-param-defs/probes/census.cjs
docs/superpowers/plans/2026-10-06-js-param-defs/MEASUREMENTS.json
EOF
check false false "root-level markdown" <<'EOF'
CLAUDE.md
AGENTS.md
docs/MCP.md
EOF
check false false "markdown beside the embedded fixtures" <<'EOF2'
docs/eval/jsts-workstream-value/README.md
docs/contracts/NOTES.md
EOF2

# Anything the build or a test reads: the full gate set.
check true true "Rust source" <<'EOF'
src/cpg.rs
EOF
check true true "docs mixed with source" <<'EOF'
docs/MCP.md
src/mcp/mod.rs
EOF
check true true "fixtures embedded with include_str!" <<'EOF'
docs/eval/receiver-closure/executable-owner-fixtures.json
EOF
check true true "targets schema read by tests" <<'EOF'
docs/contracts/targets.schema.json
EOF
check true true "README is parsed by tests" <<'EOF'
README.md
EOF
check true true "skill text is parsed by tests" <<'EOF'
skills/prism-code-slicing/SKILL.md
EOF
check true true "markdown outside docs/ and the root" <<'EOF'
tests/fixtures/notes.md
EOF
check true true "workflow change" <<'EOF'
.github/workflows/ci.yml
EOF
check true true "root-level non-markdown" <<'EOF'
Cargo.lock
EOF
check true true "a rename out of src lists the old path" <<'EOF'
docs/moved.md
src/moved.rs
EOF
check true true "a directory merely named like docs" <<'EOF'
docs-internal/tool.rs
EOF
check true true "empty change list" </dev/null
check true true "blank lines only" <<'EOF'


EOF

# The eval harness (Python, outside the build): the short path. Only the Python job runs.
check false true "eval harness only" <<'EOF'
eval/tier_a/cli.py
eval/tests/test_matrix.py
eval/uv.lock
eval/pyproject.toml
EOF
check false true "eval harness data and secbench" <<'EOF'
eval/secbench/run.py
eval/secbench/inspect.test.mjs
eval/corpora.toml
eval/snapshots/x.json
EOF
check false true "docs mixed with the eval harness" <<'EOF'
docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prB.md
eval/secbench/run.py
EOF
check false true "root markdown mixed with the eval harness" <<'EOF'
CLAUDE.md
eval/README.md
EOF
# Rust tests read eval/fixtures (tests/cli/min_confidence_test.rs): those paths keep the full set.
check true true "eval fixtures are read by Rust tests" <<'EOF'
eval/fixtures/go/closure/main.go
EOF
check true true "eval fixtures mixed with harness code" <<'EOF'
eval/tier_a/cli.py
eval/fixtures/rust/closure_call/expected.toml
EOF
check true true "eval harness mixed with Rust source" <<'EOF'
eval/tier_a/cli.py
src/cpg.rs
EOF
check true true "eval harness mixed with Cargo.lock" <<'EOF'
eval/tier_a/cli.py
Cargo.lock
EOF
check true true "eval harness mixed with a workflow change" <<'EOF'
eval/tier_a/cli.py
.github/workflows/ci.yml
EOF
check true true "a directory merely named like eval" <<'EOF'
evaluation/tool.py
EOF
check true true "a root file named eval" <<'EOF'
eval
EOF
check true true "a nested eval directory is not the harness" <<'EOF'
tests/eval/helper.py
EOF

# Renames are listed as old and new path (git diff --no-renames).
check true true "a rename from src into eval lists both paths" <<'EOF'
src/tool.rs
eval/tool.rs
EOF
check false true "a rename from eval into docs" <<'EOF'
eval/notes.md
docs/notes.md
EOF
check false true "a rename within eval" <<'EOF'
eval/tier_a/old.py
eval/tier_c/new.py
EOF
check true true "a rename from eval fixtures into the harness" <<'EOF'
eval/fixtures/go/a.go
eval/tools/a.go
EOF

# Non-docs changes also run the Python job; docs-only changes run neither.
check true true "Rust source also runs the Python job" <<'EOF'
src/cpg.rs
EOF
check false false "docs only runs neither job" <<'EOF'
docs/MCP.md
CLAUDE.md
EOF

if [ "$failures" -gt 0 ]; then
  echo "$failures failed"
  exit 1
fi
echo "all passed"
