#!/usr/bin/env bash
# Independent additive filter; never changes the code/eval classifier outputs.
set -euo pipefail
oracle=false
while IFS= read -r path; do
  case "$path" in
    src/data_flow/build/*) ;;
    src/data_flow/* | src/cpg_cache/* | src/cpg_cache.rs | src/cpg/dfg_compaction_support.rs | src/cpg/build.rs | src/cpg.rs | src/data_flow.rs | tests/dfg_compaction/* | tests/fixtures/*) oracle=true ;;
  esac
done
echo "oracle=$oracle"
