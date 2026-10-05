# IMPLEMENTOR dispatch — lane js-param-defs PR-A "binding shapes" (Sonnet)

You implement PR-A from the committed prototype. Read `SPEC-prA.md` §0 first. It is the design of record, and the reasons below are there so you can catch a prescription bug rather than copy one. If the code disagrees with this brief, or the brief disagrees with the SPEC, stop and report it. Do not pick one yourself.

## Goal (decision served)
Make prism register a parameter Def for two JS/TS/TSX binding shapes that SecBench showed it misses:
- the unparenthesised arrow formal (`x => …`);
- the identifier rest formal (`...xs`).

The measured effect is 6/6 rest conversions and 1/3 arrow conversions now trace on unmodified packages (`MEASUREMENTS-prA.md`). Exact remains a static-binding grade. Every ambiguity resolves toward **no** Def.

## Where
- Clone `~/code/prism-pd-impl`, branch `feat/js-param-defs-a` off the controller's commit of the prototype.
- Cache dir `~/prism-evidence/js-param-defs/cache`.
- Never open `~/code/frontend-portal`.
- Commit at each stable point. The controller pushes.

## The change (prototype already in the tree; port, review and own it)
| Site | Change | Why (the trap it avoids) |
|---|---|---|
| `ast.rs` new `js_ts_bare_arrow_parameter`, `js_ts_rest_identifier`, `parameter_binding_region` | Add helpers. They are gated on JS/TS/TSX, `arrow_function` and an `identifier` kind | **Do not** add a `parameter` fallback to `find_parameters_node`, even though EVALUATION §3.2 says to. It has ~16 callers that iterate the returned list's children; an identifier has none, so they would silently see an empty list. Two of them treat `None` as "no list" and already carry their own `parameter` fallback (`collect_js_ts_parameter_bindings`, the S1b F3 single-param binding) or a `?` early return (`js_ts_parameter_receiver_binding`). Changing `find_parameters_node` would drop that binding and change call resolution. The same-base `call-stats` control would show it; the suite might not. |
| `ast.rs::function_parameter_occurrences` (JS path) | A bare arrow returns its one formal. A `rest_pattern` child yields its identifier only when the whole list is clean (no recovery, no duplicate binding) | Destructured rest stays refused. A duplicate list is an early error once a rest element makes the list non-simple, so the new occurrence is refused. The **pre-existing** sloppy-mode plain duplicates stay byte-identical. Do not "fix" them in this PR. |
| `parameter_slots.rs::typescript_parameter_bindings` | Use `parameter_binding_region`. An identifier region returns one occurrence unless it is escaped (`\`). `required_parameter` with `pattern: rest_pattern(identifier)` is admitted under the existing child allowlist | The pinned grammar has **no** `rest_parameter` node, despite what EVALUATION §3.2 says. TS rest is `required_parameter` + `rest_pattern` (probe in `PROBE-LOG.md` P2). The binding bytes are the inner identifier, not the `...xs` pattern. |
| `parameter_slots.rs` new `js_ts_parameter_list_is_clean`, `js_ts_is_last_parameter` | `!contains_recovery && !has_duplicate_js_ts_bindings`; rest must be the last named parameter | Reuse the existing helpers; do not write a third duplicate detector. `(...xs, y)` is an early error that tree-sitter parses **without** recovery, so `contains_recovery` does not catch it. The pre-existing test `reviewer_optional_inert_complete_allowlist_and_old_path_controls` pins it. |
| `ast.rs` new `js_ts_nested_callable_binds_formal`, called at all 4 new-shape admissions (JS bare, JS rest, TS bare, TS rest) | Refuse the new Def when a nested callable re-binds the name as a formal (SPEC D11) | The legacy reference walk does not fence nested formals (E3). Without the guard, T gained a false **Exact** row (`documentRegistry.ts:197→200`). **Do not** apply it to pre-existing plain formals: that changes existing rows and is PR-B's containment job. |
| `cpg/build.rs::compute_param_def_nodes` | Drop the bare formal from `supported` (SPEC D12) | The ladder's `free_single` resolves bare calls Exact to name-inferred callables (E5). Filling the slot gave 26 Exact arg→formal rows on T, and only 1 was checker-confirmed. **Do not** "fix" this by changing `slots()`; that would change other consumers. The parenthesised control must keep its edge, and the test pins both. |
| `ast.rs::exact_read_is_plain_required_parameter` | The bare arrow formal counts as plain required for **JS and TS/TSX**. Rest stays excluded (its parent is `rest_pattern`) | EVALUATION lists the JS arm only. The TS arm needs it too, because the bare formal has no `required_parameter` wrapper. |
| `cpg/reaching/scope.rs::declaration_seed` (one site) | `parameter_binding_region` instead of `find_parameters_node` | Without this, the bare-arrow Def is seeded as a non-parameter binding, and its labels diverge from the identical `(x) =>` control: 9 T rows were Exact where the parenthesised control is NameOnly(Killed). **Leave `introduction_is_classified` (≈ line 588) alone.** The root callable's own fields are never examined there, so changing it is an equivalent mutant (PROBE-LOG P6). |
| `cpg_cache.rs` | `CACHE_VERSION` 106 → 107 with a doc line, and pin test 107 | One transition. **Do not** bump `NAV_CALL_EDGE_CACHE_VERSION`: call-site rows are byte-identical on every measured corpus. |
| `cpg/required_parameter_tests.rs`, `ast_required_parameter_tests.rs`, `ast_nested_execution_owner_tests.rs` | Four assertions that pinned the gap now expect the new occurrence: `rests(...items)` → `items`, `take(...a)` → `a` (the unsupported list keeps `...[a]` instead), `(value?, ...rest)` → `value, rest`, and `p=>sink(p)` → `p`@9..10 | These are intended behaviour changes, not re-baselines. Every other assertion in those tests is untouched. |
| `mutants/lane-p-tsconfig-paths.json` | Rebind `P2-M11-cpg-cache-version` to the 107 anchor, with an `intent_revisions` entry | Its 106 anchor no longer exists, so the full gate would report it INADMISSIBLE. The obligation is unchanged. This is a cross-lane registry edit; the controller should confirm it. |

**Do not touch:** `slots()`/`typescript_slots`/`javascript_slots` (rest must keep stopping positional binding; it is variadic), `has_bare_references` (member-only is PR-C), `languages::function_name` (anonymous callables are PR-B), `seeds.rs` (it gains the new Defs through occurrences, intentionally), and anything outside JS/TS/TSX.

## Tests (already in the tree; each must fail on pre-change code)
- `src/ast_js_param_defs_tests.rs` has 6 tests. They cover: occurrences for bare/async/`get`/curried arrows; rest in JS/TS/TSX with `this` and a comment; destructured, duplicate and escaped refusals; slots unchanged; the `exact_read` arm; and helpers inert for Rust/Python.
- `src/cpg/js_param_defs_tests.rs` has 9 tests. They cover:
  - Defs at identifier bytes;
  - Step 5b never binds an argument to rest or to the bare formal, while the parenthesised control keeps its edge;
  - the nested-formal refusal, with an unrelated-name control;
  - **label parity**: bare vs parenthesised arrow, and rest vs plain formal, give byte-identical `dfg_edge_dump`;
  - the curried inner arrow gets no Def (that is PR-B);
  - refused shapes, and whole-list refusal of a TS duplicate.
- Mutants: `mutants/js-param-defs.json` registers 18 mutants (PD-01 to PD-18). Each must be KILLED. The prototype run gave 18/18 killed (authoritative), and the full gate gave 139/139 (PROBE-LOG P13).

## Gates (report totals, with `[MEASURED]` command + output path)
1. Touched tests: `cargo test --lib js_param_defs` and `cargo test --lib required_parameter`.
2. One run of `cargo nextest run --features mcp`. Report pass/fail/skip totals. A failure outside this diff is reported, not fixed.
3. Advisory mutgate: `python3 scripts/mutgate/mutgate.py --lane mutants/js-param-defs.json --since origin/main --scope fn`, plus the authoritative lane run `--lane mutants/js-param-defs.json` (≤ 15 min).
4. `cargo fmt --check` and `cargo clippy --all-targets --features mcp -- -D warnings`.
5. Fresh `cargo build --release`, then `cd eval && UV_CACHE_DIR=/Users/wesleyjinks/.local/share/prism/uv-cache uv run tier-a --matrix-only --allow-stale-sut`.
6. Row controls with `probes/measure-public.sh BASE HEAD OUT`:
   - Python, Go and Rust DFG and call-site rows must be byte-identical;
   - X/Xi/T/SecBench must show ADDED/RELABELLED only;
   - run `probes/adjudicate.cjs` on every changed-row file. Any `ADDED|…|WRONG` or LOST row is a STOP. Each `EXACT_PRIOR_WRITE` hit gets the same-shape plain-parameter control; it is WRONG only if the control differs (SPEC D13).

## STOP and report (do not work around)
- A LOST row, or an ADDED row the checker calls WRONG.
- Any non-JS byte difference.
- Any `call-stats --dump-sites` difference.
- A test that needs re-baselining for a reason other than the `rests` Def above.
- The prototype and the SPEC disagree.
