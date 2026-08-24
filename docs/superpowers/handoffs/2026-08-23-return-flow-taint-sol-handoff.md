# Handoff — roadmap #2 return-flow taint (spec v5)

**Written:** 2026-08-24T02:35:14Z · **Updated:** 2026-08-24T02:57:47Z · **By:** Codex `/root` · **Provider:** codex
**Workspace:** `/Users/wesleyjinks/code/slicing-2rf-sol` · branch `return-flow-taint` · base `0ca571c5e80e12fb3c00e68f5ec0ea53a247d158` · code checkpoint `0487a63`
**Authority:** owner brief > untracked supplied `docs/superpowers/specs/2026-08-23-return-flow-taint-design.md` v5 §2/§3  
**Template:** the installed `bootstrap/handoff-template.md` named by steering was not present, so this follows the repository handoff layout and retains every requested raw-data field.

## 0. Gating facts

- No subagents were used. The sibling resolution lane was not inspected or modified.
- No commit was amended. The owner explicitly authorized pushing this fix wave after its handoff commit; final push evidence is returned outside this pre-push artifact.
- No test, build, or byte-gate process remains running.
- The supplied design document remains untracked and was not committed.
- Tier-A matrix/quick and corpus batteries were not run here; the owner assigned those gates to the controller.
- No semantic ambiguity or STOP condition was encountered.

## 1. Outcome

Implemented singleton-Exact return-flow taint with span-bound, nested-callable-fenced return enumeration; non-seedable canonical synthetic return-value identities; semantic-only `ReturnInput`; replacement-path-certified assignment-shortcut suppression; shared build/trace singleton predicate; same-budget ascent before cross-function boundary handling; recursion boundaries; Tier-2-on/Tier-1-opt-in/other-consumers-off mode routing; identical bypass mode; one authoritative sanitizer transition; serialized cache/counter custody; and CPG 49 to 50 with navigation sidecar unchanged at 18. Fix wave `0485296..0487a63` closes the PR #193 review findings: Go/other anonymous callable returns cannot leak into their enclosing function, and multiple callers reuse one synthetic identity plus one `ReturnInput` set.

## 2. Commit ledger

| Commit | Role |
|---|---|
| `d986ab6` | Green implementation: return-flow schema, construction, tracing, sanitizer reasoning, mode seam, counters, cache bump, and gate-1 tests |
| `58d41b3` | Green custody closure: legacy Go return compatibility, Tier-1 seam, counter assertions, cache/index round trip, and exhaustive test-helper schema consumers |
| `0485296` | RED: constructible Go closure false-reach, JS arrow boundary control, and duplicate two-caller synthetic/counter regression |
| `0487a63` | GREEN: closed callable-boundary set and build-local canonical return-value identity index |

All implementation/fix-wave commits contain the required `Co-Authored-By` and `Claude-Session` trailers.

## 3. Changed files

- `src/algorithms/taint.rs`
- `src/ast.rs`
- `src/cpg.rs`
- `src/cpg/build.rs`
- `src/cpg/tests.rs`
- `src/cpg/trace.rs`
- `src/cpg/types.rs`
- `src/cpg_cache.rs`
- `src/main.rs`
- `src/languages/mod.rs`
- `src/navigation/queries.rs` — exhaustive schema/edge rendering only; no dispatch arms changed
- `src/reasoning/sanitizer_walk.rs`
- `src/reasoning/shape.rs`
- `src/reasoning/taint_reaches.rs`
- `tests/algo/taxonomy/taint_lang_test.rs`
- `tests/ast/cpg_cache_test.rs`
- `tests/ast/cpg_test.rs`
- `tests/cli/call_stats_test.rs`
- `tests/common/mod.rs`
- `tests/reasoning/taint_reaches_test.rs`

`resolution.rs` and its Go routing were not modified.

## 4. Tests by name

### Positive return witnesses and sanitizer behavior

- `return_flow_callee_internal_source_reaches_go_caller_lhs`
- `return_flow_callee_internal_source_reaches_python_caller_lhs`
- `return_flow_callee_internal_source_reaches_javascript_caller_lhs`
- `return_flow_callee_internal_source_reaches_typescript_caller_lhs`
- `return_input_binary_expression_reaches_caller_lhs`
- `return_flow_sanitizer_suppresses_assignment_shortcut_and_is_bypass_proven`
- `tier1_return_flow_config_is_default_off_and_opt_in_on`

### Fail-closed and boundary negatives

- `return_flow_singleton_exact_predicate_rejects_nameonly_multi_and_mixed`
- `return_flow_nameonly_trait_callee_is_fail_closed`
- `return_flow_multi_exact_interface_dispatch_is_fail_closed`
- `return_flow_non_simple_lhs_is_skipped`
- `return_flow_multi_value_arity_mismatch_is_skipped`
- `return_flow_named_bare_return_is_skipped`
- `return_flow_forwarded_multi_value_return_is_skipped`
- `return_flow_nested_function_return_is_fenced`
- `return_flow_same_line_double_assignment_binds_call_to_its_ast_parent`
- `return_flow_mixed_modeled_and_bare_returns_voids_shortcut_suppression`
- `return_flow_nested_unbound_use_voids_shortcut_suppression`
- `return_flow_self_and_mutual_recursion_stay_bounded`
- `return_flow_python_keyword_label_is_not_a_semantic_return_input`
- `return_literal_synthetic_is_not_source_seedable`
- `return_flow_go_func_literal_return_is_fenced_from_outer_function`
- `return_flow_javascript_arrow_return_is_fenced_from_outer_function`
- `return_flow_two_callers_share_one_synthetic_identity_and_return_input_set`
- `callable_boundaries_cover_anonymous_forms_without_changing_function_index_types`

### Custody and compatibility

- `cache_versions_are_pinned_for_return_flow_taint`
- `cache_v50_round_trips_return_flow_nodes_edges_indexes_and_stats`
- `call_stats_emits_one_return_flow_subobject_with_all_custody_counters`
- `return_value_nodes_preserves_go_legacy_expression_list_and_adds_slots`

## 5. Verification totals

- `cargo fmt --all`: pass.
- `git diff --check`: pass.
- Focused return-flow unit filter: 14 passed, 0 failed.
- `cargo test --test reasoning`: 69 passed, 0 failed.
- `cargo test --test ast`: 447 passed, 0 failed.
- Tier-1 seam focused test: 1 passed, 0 failed.
- CLI counter focused test: 1 passed, 0 failed.
- Legacy Go return focused test: 1 passed, 0 failed.
- Full `cargo test --quiet`: **3,463 passed, 0 failed, 1 ignored** across 28 test/doc targets.
- `cargo build --release`: pass.
- Tier-A matrix/quick: not run; controller-owned.
- Corpus/call-stats batteries: not run; controller-owned.

## 6. Cache and counters

- CPG cache: **49 to 50**, one transition.
- Navigation call-edge sidecar: **18 to 18**, untouched.
- Emitted under exactly one call-stats `return_flow` sub-object:
  - `return_flow_edges`
  - `return_input_edges`
  - `return_flow_skipped_nameonly`
  - `return_flow_skipped_multi`
  - `return_flow_skipped_mixed`
  - `return_flow_skipped_non_simple_lhs`
  - `return_flow_skipped_arity_mismatch`
  - `return_flow_skipped_named_return`
  - `return_flow_skipped_forwarded_return`
  - `return_flow_suppression_certified`
  - `return_flow_suppression_void_incomplete_returns`
  - `return_flow_suppression_void_unbound_uses`
- Corrected two-caller dedup pins: `return_flow_edges=2`, `return_input_edges=1`, `return_flow_suppression_certified=2`.
- Existing single-caller CLI pins remain `return_flow_edges=1`, `return_input_edges=1`, `return_flow_suppression_certified=1`.

## 7. Same-base per-consumer byte gates

Control: detached base `0ca571c5e80e12fb3c00e68f5ec0ea53a247d158`, built in the same environment. The complete table was rerun after fix-wave release build with fresh `base-cache-v3` / `candidate-cache-v3` directories. Fixture contains `value = decorate(user)` where `decorate` returns `user + "x"`, so candidate CPG construction mints a synthetic node plus `ReturnInput`/`ReturnFlow`; it also contains a non-empty local chop path. Gate artifacts are ignored under `target/return-flow-byte-gates-58d41b3/`.

| Consumer | Result | Bytes each | SHA-256 |
|---|---:|---:|---|
| `--format review`, cold separate caches | byte-identical | 3,683 | `e83d49e104a1f307350ad33e80c7d8ee58ed6047b8736d8f01ca4032328ab2e6` |
| `--format review`, cache hit | byte-identical | 3,683 | `e83d49e104a1f307350ad33e80c7d8ee58ed6047b8736d8f01ca4032328ab2e6` |
| `fullflow` | byte-identical | 159 | `e207a73d4b795207805925a850511646c572bcd792ed30a467bb22371365b5a8` |
| `leftflow`, direct non-compact text | byte-identical | 159 | `e207a73d4b795207805925a850511646c572bcd792ed30a467bb22371365b5a8` |
| `chop`, non-empty | byte-identical | 85 | `5769288a22d12b023b62dc0e0e9203ac88831411df18683afc322c8e3f47c875` |
| `thin` | byte-identical | 76 | `f7a2398b68e3e718d060971cc1bf1c5c54210593a066aa435215354ed3340a71` |

## 8. Inadmissible probes corrected

- A first Tier-1 command named nonexistent target `taxonomy`; rerun against `algo_taxonomy` and passed.
- A first cache command named nonexistent target `ast_cpg_cache`; rerun against full target `ast` and passed 447/447.
- A first chop pair emitted zero bytes; it was discarded and replaced with a non-empty 85-byte intra-function chop control.
- The initial duplicate-function fixture was proven multi-Exact, not NameOnly; it was replaced with a Rust trait call asserting one `NameOnly` resolution before asserting the skip counter.
- A combined fix-wave focused command yielded during compilation without exposing its session handle; that partial output was discarded and the two-caller identity test rerun directly to a completed green result.

## 9. Final custody

- Worktree code is committed through `0487a63`; this fix-wave handoff refresh is the only post-gate tracked addition.
- Supplied design document remains the sole non-ignored untracked file.
- Detached base worktree remains at `/private/tmp/slicing-return-flow-base-0ca571c` for controller reuse.
- No STOP.
