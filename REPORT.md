# #630 slice 5 — JS/TS/TSX rules U and D (branch `perf/630-ud-js`)

Status: in progress (corpora being measured). Base origin/main 9f69debf, cache v168 -> v169.

## Step 0 — grounding (written before coding)
1. **Owners reaching the producer's def/use loop.** Every callable `dfg_owner_name` names: *named* owners
   (`function_declaration`, `method_definition`, `generator_function_declaration`, and an arrow or function
   expression whose name is inferred) run the legacy line-based pass; *anonymous* arrows/function/generator
   expressions run a *synthetic* `<anon@L:C>` pass. Module level is not an owner (`all_functions` has no program node).
2. **Line-anchored or exact?** Named owners are line-anchored: `get_use` returns an existing exact rvalue span on the
   line, else mints a zero-width anchor at the line start (baseline dump shows `start_byte == end_byte`, e.g. 19,19).
   Synthetic owners build each Use at the exact admitted span. U and D are therefore enabled for **named owners only**;
   synthetic and module level stay off (they are a different code path with exact spans, so no extra hazard needed
   to include them, but the brief says leave them).
3. **JS-only edge sources.** The parameter projections (`Def(p) -> Use(p.<path>)`, member-only formals) are built after
   the lexical-filter loops and call neither `drop_write_only` nor `keep_def`: D does not see them. The #594
   object-literal shortcut is derived at CPG level from definition edges that reach the callee's reads, so it
   sees the pruned set (a read after a dominating reassignment gets no shortcut edge; test updated).

## Design as implemented (`src/data_flow/build/js_prune.rs`, hooks in `lexical_prune.rs`)
- Regions: `statement_block` (not a namespace body), `switch_case`/`switch_default`; static-block and `finally`
  bodies are blocks. d*: `x = ..`/non-logical `op=` statement, or one-initialised-declarator declaration.
- Excluded as d*: `var x;`, destructuring, `for` heads, `catch`, nested assignments, logical assignment, `x++`, nested callables.
- Class field initialisers are nested scopes. Alias twins are never d*.
- Hazards: `with`/direct `eval` switch U and D off; `arguments` switches D off. Hoisted functions (Annex B / same name),
  `var` redeclaration, labels, generators/async: no switch needed — D only drops defs that a dominating write
  provably overwrote, so extra unmodelled earlier writers cannot resurrect them; a label only targets enclosing
  statements; `yield`/`await` resume in place.
- Fields: identical paths only (no prefix kills), only for a stable base (`this`, or a once-bound, never-written name).

## Tests
- `tests/fixtures/dfg_prune/js/*` + `obligations.jsonl` (150 rows): 63 drop rows fail on the pre-implementation head
  (RED commit), all pass now; frozen-producer differential: `edges_after ⊆ edges_before`, labels equal, removed count == counters.
- Fixture-tree differential extended to the JS family (subset-only + dominator invariant).
- Goldens changed, each removed edge is a U or D drop: census `w06`, recorded-kill control rows (4), s1c killed row,
  optional-parameter rows, nested-owner anchors (`cb` lines 2,3), r5 named-seam goldens (listed in
  `js_param_r5_u_d_removed.json`, 23 cases: `var f;` / pure-write lines and dominated defs), synthetic-vs-named E4 rows,
  object-literal reassignment row, `dfg_label_store_test` partial-hit row.
- Environment failures (not mine): 5 `owner_activation_test::owner_admission_*`, and
  `cpg_cache::stream::tests::a_write_that_cannot_start_fails..` (permission-denied write; root defeats it; not re-run on main).

## Corpus results
