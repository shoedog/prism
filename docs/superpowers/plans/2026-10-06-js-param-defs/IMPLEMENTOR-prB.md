# IMPLEMENTOR dispatch — js-param-defs PR-B callback identity

Start from the controller's commit of the planner prototype on `plan/js-param-defs-prB` (base main `da0604b3`, PR-A merged, `CACHE_VERSION` 107). Read SPEC-prB §0 (B-D1…B-D14) and §6 before touching code. Exact is a static-binding grade; binding CORRECT does not prove runtime flow; every ambiguity resolves toward no edge. Never open `frontend-portal`, never execute corpus packages, use `--cache-dir ~/prism-evidence/js-param-defs/cache`.

## Why this shape (so you can catch prescription bugs)
- The synthetic name is **only** a DFG owner string. If you find yourself adding it to `FunctionInfo`, `function_name`, the call graph or any `functions_named`/owner-index path, stop: that is exactly the leak B-D1 forbids (E5, R2).
- Legacy named passes must stay **byte-identical** except for the E3 fence. Every new filter (own scope, E7, E10, E11, E4) is gated on `synthetic`. A change that alters a legacy row other than an E3-fenced removal is a regression.
- Do not "fix" the duplicate ownership of callbacks nested in named functions by fencing the named pass out of them: `trace.rs` assignment propagation is same-owner only, so re-owning loses existing taint paths (SPEC B-D2).
- The perf memo must stay a pure cache: the CFG edges are a function of `parsed` only; `use_byte` is a function of `(pass, line, path)` because every edge in a pass shares the owner.

## Change table
| Site | Behaviour | Boundary |
|---|---|---|
| `src/ast_callback_identity.rs` `dfg_owner_name` / `js_ts_synthetic_callable_name` / `is_synthetic_owner` | `function_name` text, else `<cb@L:C>` for unnamed JS/TS/TSX `arrow_function`/`function_expression` | Non-JS and named callables unchanged; generators excluded |
| `js_ts_span_in_own_scope` | lvalue byte span inside the callable and not inside a nested callable boundary | Uses `callable_boundary_node_types()` |
| `js_ts_reference_fenced` (+ `js_ts_scope_binds`, block/declaration/function-scope helpers) | E3 fence; scope must not contain the Def byte; a nested callable's function scope includes `for (var k in …)` heads | `var` for-heads in the owner itself are not fenced; escapes fence |
| `js_ts_this_rebound_between` | synthetic passes: `this.x` stops at nested non-arrow functions, methods, class bodies | arrows keep the owner's `this` |
| `find_path_references_fenced_in(func, path, def_line, def_byte, within, reads_only)` | legacy `is_shadowed_at` + E3 fence; optional lexical scope root; optional reads-only filter | Non-JS delegates to `find_path_references_scoped` |
| `js_ts_span_is_non_reference`, `js_ts_span_is_not_a_read`, `js_ts_identifier_is_not_a_read`, `js_ts_lexical_def_scope`, `js_ts_own_alias_target` | E7 / E4 / E10 / E11 classifiers for synthetic passes | TS `required_parameter` default values remain reads; alias twins keyed by `(name, line)` leave a 12-row minified residue (SPEC §6) — key by span start byte if the review asks |
| `src/data_flow.rs` pass loop | owner via `dfg_owner_name`; `synthetic` flag gates own-scope lvalues, own-scope alias derivation, E7/E4 rvalue filters, E10 scope, E4 reads-only walk, alias twin on its own line; every walk uses the fenced variant | Legacy: only the fence differs |
| `src/cpg/reaching.rs` / `reaching/scope.rs` | file CFG edges passed in (built once per file in `data_flow.rs` via `OnceCell`); `BindingFacts.use_byte_cache` | Pure memo; `reaching_definitions` wrapper builds its own edges |
| `src/navigation/queries.rs` `nav_visible_nodes_at` | `nodes-at` and `ego --location` skip synthetic-owned Variables | No other nav path can reach them (no cross-owner edges, no Function node) |
| `src/cpg_cache.rs` | 107 → 108, doc line | One transition |
| Mutants | PD-30…PD-54 (`mutants/js-param-defs.json`), PD-11 re-pinned to 108, P2-M11 re-pinned in `lane-p-tsconfig-paths.json` | Keep the P2-M11/PD-11 coupling (Opus F6) |

## Tests and intended re-pins
Run `cpg::callback_identity_tests` (22), `cpg::js_param_defs_tests`, `required_parameter`, `nested_execution_owner`. The only intended re-pin of an existing assertion is `js_param_defs_tests::curried_arrows_bind_only_the_named_outer_formal` (inner `y =>` now owns `y` under `<cb@1:20>`). Any other existing test that changes is a STOP for the controller, not a re-baseline.

## Gates and measurement
1. Touched tests; `cargo nextest run --features mcp` (report totals; the reserved ignored test stays ignored).
2. Mutation: advisory `python3 scripts/mutgate/mutgate.py --since origin/main --scope fn` (after commit — untracked files are invisible to `--since`) and the authoritative PR-B subset `--only PD-30… --only PD-54 --only PD-11-cache-version-not-bumped`, plus lane-P `--only P2-M11-cpg-cache-version`.
3. `cargo fmt --check`; clippy `--all-targets --features mcp -- -W clippy::all` against a same-environment base build; no new warning.
4. Fresh release build; Tier-A `--matrix-only` and `--quick` (VALID; no `<cb@` in any named frame).
5. `probes/measure_b.py` public (X, Xi, T, black, caddy, prism) and `--secbench --jobs 4`; `rowdiff.py`; `adjudicate.cjs`; `nav_identity.py`; SecBench `secbench_callback.py`, joint counterfactual, eligible sweep.
6. Perf: dedicated quiet run, base vs head, X / T / one large SecBench package; STOP if build time > +25 %.
7. Controller F: `CONTROLLER-pd.sh diff TS_JS BASE_BIN HEAD_BIN BASE_BYTES_BIN HEAD_BYTES_BIN`, both byte dumpers built from this packet's `probes/byte_dump.rs`, new `PRIVATE_EVIDENCE_ROOT`.

## STOP and disclose
STOP on: a LOST checker-correct row; any RE-OWNED row (none is expected by construction) that is not proved correct; any nav or call-site byte change; non-JS non-identity; build time > +25 % on a measured corpus; a new ADDED WRONG class. Disclose (do not fix in legacy): E4, E7, E9, E10, E11 parity.
