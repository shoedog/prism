#!/usr/bin/env bash
set -euo pipefail
filter="$(cd "$(dirname "$0")" && pwd)/dfg-oracle-paths.sh"
check() {
  local expected="$1"
  shift
  local actual
  actual="$(printf '%s\n' "$@" | bash "$filter")"
  test "$actual" = "oracle=$expected" || { echo "FAIL $*: $actual"; exit 1; }
  echo "ok $*"
}
check false docs/plan.md
check false src/data_flow/build/mod.rs src/data_flow/build/deep/file.rs
check false src/cpg/query.rs src/call_graph.rs
check false
check true src/data_flow.rs
check true src/data_flow/oracle/update.rs
check true src/cpg_cache.rs
check true src/cpg_cache/stream.rs
check true src/cpg/dfg_compaction_support.rs
check true src/cpg/build.rs
check false src/cpg/build.rs.bak
check true src/cpg.rs
check false src/cpg.rs.bak
check true tests/dfg_compaction/outputs.rs
check true src/data_flow/build/file.rs src/data_flow/types.rs
check true tests/fixtures/mcp/launcher/argv-stub.sh
check true tests/fixtures/new-fixture.py
check false tests/fixtures-other/example.py
