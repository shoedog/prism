# Lane 9: finish JavaScript parameter binding — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** arguments reach parameters for spread arguments (fixed: no wrong edge), rest parameters, unparenthesised
arrow parameters, callbacks of built-in higher-order methods and calls through a callback parameter; each slice
after A only if its census gate passes; every added edge graded by the TypeScript checker.

**Architecture:** slices A to C change the positional loop of the argument-binding step (production and its two
test-only twins) through new `src/cpg/` modules; slice D adds one pass beside that loop with a new name-only doubt
kind; slice E adds a derived index of what Exact callers pass to a parameter and a name-only fallback at the one
public resolution entry, after the #434 plan.

**Tech stack:** Rust (`src/cpg`, `src/resolution`, `src/call_graph`), tree-sitter JS/TS/TSX; Node with the pinned
TypeScript 5.9.3 (censuses, oracles); Python 3 (comparators, benchmark harness).

**Spec:** `DESIGN.md` here (section 5 slices, 6 census and go/no-go, 7 slice gates). Slice E carries the #434 plan's
interfaces, cited "#434 §n" (`~/prism-evidence/lanes-2026-11-20/research/scratch/lane9/argfollow-PLAN.md`).

## Global constraints

- Worktree `~/code/prism-param-binding` (sparse, no `eval/secbench/`); measurement worktree
  `~/code/prism-param-binding-meas` (full checkout: harness, pinned builds). Evidence `~/prism-evidence/param-binding/`.
- Corpora: X = Excalidraw `0642e72c` (`~/prism-evidence/inputs/excalidraw-0642e72c/source`; M4 types X with the
  installed copy `…-installed/source`); T = `~/code/bench-repos/TypeScript/src` at `7964e22f`; SB = benchmark packages
  (`~/prism-evidence/inputs/secbench-pkgs`, one capture per package root, rows prefixed `pkg`); G =
  `~/code/bench-repos/gin` (control: no change); `private-corpus` locally, aggregate-only, Claude seats only. Checker:
  `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.
- **Not changed:** the slot builders in `src/parameter_slots.rs`, `function_parameter_occurrences`, `CallSite.arg_spread`
  and the arity checks reading it (setting it for JavaScript would change call resolution, a DESIGN anti-goal),
  `refuse_new_sources_exposing_free_single_calls`, `src/cpg/trace.rs`, `src/reasoning/scope_honesty.rs` (its
  "JS/TS callback" warning describes touched functions, not missing flow), taint code, `arguments` handling.
- Weighted sizes (`python3 scripts/file_size.py`): `src/call_graph.rs` 5,653 (nothing added: see E0);
  `src/resolution.rs` 4,171.5 and `src/navigation/queries.rs` 2,502.5 (one-line edits only, listed in the PR with
  sizes before and after); `src/cpg/build.rs` 1,777 (module lines, calls, one stats field, match arms);
  `src/cpg_cache.rs` 1,544. Logic goes in new modules.
- New `src/cpg/` modules that use the build's private items are declared in `src/cpg/build.rs` with `#[path = "…"]`
  (as `callee_params`); new test files get a `#[cfg(test)] mod` line in `src/cpg.rs`, except tests that use
  `callee_legacy::with_legacy`, which are declared in `src/cpg/build.rs` under `#[cfg(test)]`.
- **Twin rule:** every change to argument binding is made at the same position, with the same emission order, in
  `step5b_edges_for_caller_with_exact` (`src/cpg/build.rs:1567`), `step5b_caller_legacy` (`src/cpg/callee_legacy.rs`)
  and `collect_step5b_edges_reference` (`src/cpg/step5b_reference.rs`).
- A slice that changes the graph bumps `CACHE_VERSION` in `src/cpg_cache.rs` (list line and the
  `assert_eq!(super::CACHE_VERSION, …)` test); E1 and E2 also bump `NAV_CALL_EDGE_CACHE_VERSION`
  (`src/navigation/call_edge_cache.rs`). Take the next free number at rebase (ORCHESTRATION section 5 item 7).
- Tier-A on every slice; for E2 the `--quick` regressions and flip candidates go in the PR body, never re-baselined.
- Lane 3 owns `src/cpg/object_literal_args.rs` and edits `object_literal_arg_tests.rs`: slice C changes one doc
  comment and one expectation there and rebases over lane 3.

## Review focus

1. `f(...list, v)` into `function f(a, b)`: no edge into `b`; `f(v, ...list)` keeps `v → a`; production, twins and both
   cache paths agree. Rows a01, a02; `param_binding_parity_tests::spread_and_rest_graph_is_identical_across_every_route` (A, B).
2. A bare-arrow callable whose name is only inferred (`{ Array: t => … }`, `o.readFile = f => …`), called by a bare
   name from another script: no `exact` edge into its formal. Rows k02 to k04 (C).
3. A user-defined method named `map` or `then` gets the callback edge labelled `builtin_callback`, never `exact`,
   and the trace engine does not cross it. Rows h08, h11 (D).
4. A holder called with nine distinct callables drops as `ArgFanout` with no edge, anonymous targets counted. Row e05 (E2).
5. A partial cache hit where only a caller's file changed gives the same argument-follow index as a cold build.
   `arg_follow_tests::index_partial_hit_equals_cold` (E1).

## Stage M: census, oracle, benchmark (4 days)

### Task M-1: workspaces, binaries, capture script and oracle (evidence directory)

**Files:** `bin/BINARIES.md`, `census/capture.sh`, `tools/adjudicate9.cjs`, copies of `rowdiff.py` and `byte_dump.rs`,
`tools/fixtures/`.

- [ ] Both worktrees and the evidence directory (ORCHESTRATION section 4). Build main in the measurement worktree through
      memguard; copy to `bin/prism-base`; record `--version` and SHA-256. Comment on #595, #434, #604: lane started.
- [ ] `capture.sh <binary> <corpus> <out>`, through memguard: `nav --no-cache call-stats --repo <corpus> --dump-sites`
      to `sites.jsonl`, and `PRISM_EDGES_BYTES=1 nav --no-cache dfg-stats --repo <corpus> --edges | LC_ALL=C sort` to
      `edges.jsonl`; for SB one run per package root.
- [ ] Copy `adjudicate.cjs`, `rowdiff.py`, `byte_dump.rs` from `docs/superpowers/plans/2026-10-06-js-param-defs/probes/`.
      `byte_dump.rs` is built as `examples/r1_byte_dump.rs` in a scratch copy of each binary's commit (its header says
      how) and never enters the repository. `adjudicate9.cjs` keeps `adjudicate.cjs`'s grading and adds modes:
      `rest` (an argument at index ≥ the rest index into the rest parameter is CORRECT; `useDef` today grades it
      `position_or_rest` WRONG); `builtin` (a second program with the default lib: receiver type, callback, fed
      parameter index per DESIGN 4 B); `argfollow --targets <param-sites.jsonl>` (CORRECT when the Def's callable is in
      the census's target set for the site and the indexes match; #434 §7); `spread` (a LOST row is spread-misbound
      when its Use lies in an argument after a spread element of the same call).
- [ ] Self-test every mode on fixtures with one CORRECT and one WRONG row before use; outputs in `tools/SELFTEST.md`.

### Task M-2: the six measures

Scripts in `census/` read `capture.sh` output, run `node --max-old-space-size=12288` with the pinned checker, never
execute corpus code, accept `--aggregate-only`, exclude directories as `~/prism-evidence/js-args/census.cjs` does, and
write `census/<Mn>/<corpus>/aggregate.json`. Write expected magnitudes in `census/PREDICTIONS.md` first.

| Id | Script | Counted | Grading |
|---|---|---|---|
| M1 | `m1_spread.cjs <root> sites.jsonl edges.jsonl <out>` | call and `new` sites with a spread element followed by another argument, by resolution (exact singleton, name-only, several, dropped); cross-function edges today whose Use lies in an argument after the spread | each such edge WRONG by construction |
| M2 | `m2_rest.cjs` | exact-singleton sites whose in-repo callee has an identifier rest formal and that pass an argument at or after its index; split by a spread before the index and by whether prism has the rest Def | sites with the Def count for the gate |
| M3 | `m3/lift-d12.patch`, `m3/run.sh` | a probe of main without the filter `Some((occurrence.1, occurrence.2)) != bare_formal` in `compute_param_def_nodes` (scratch worktree, never pushed); `rowdiff.py` base vs probe on X, T, SB; the three PROBE-LOG P11 rows (`debug.ts:1057 Array(height)`, `incrementalUtils.ts:493 readFile(fileName)`, `deprecations.ts:97 bind(args)`) reported by name | ADDED and LOST by label and plain-mode verdict; WRONG among `exact` is the gate number; UNDECIDED `exact` rows listed, up to 50 read with a reason each |
| M4 | `m4_builtin.cjs` | member calls named in DESIGN 4 B with an inline function or identifier at argument 0: callback kind (inline arrow, bare arrow, function expression, same-file named, other); whether it reads the fed parameter (first; second for `reduce`, `reduceRight`); receiver form (identifier, member, `this` path, call, literal, chain); receiver type with the default lib: array (array, tuple, `ReadonlyArray`, unions of these), promise (`Promise`, `PromiseLike`), other (`Map`, `Set`, classes), untyped (`any`, error, unresolved) | wrong-receiver rate = other / (array + promise + other) |
| M5 | `m5_argfollow.cjs` (copy of `~/prism-evidence/js-args/census.cjs` plus `follow.product.prism_reverse_edge_sites_cap8`) | #434 census on current dumps: product parameter sites dropped today whose holder's Exact callers pass followable values, anonymous targets included, target set of 1 to 8; set sizes; holder exported or escaping | capped sites with a target |
| M6 | harness (Task M-3) | all classes on `prism-base`: outcome per class; `first_break.category` and `mechanism` per non-traced case beside the R1 rows (`~/prism-evidence/meas/secbench/repair-r1/final/entries.jsonl`) | first breaks at a lane shape; `callback_argument_parameter_registration` cases listed for hand-over |

### Task M-3: harness options (eval-only PR shared with lane 4)

If lane 4's Task M-2 (`--class`, `--binary-sha`, `--sut-commit` in `eval/secbench/run.py`) has merged, use it.
Otherwise do that task exactly as lane 4's `PLAN.md` states it (same options, same three tests in
`eval/secbench/test_cli.py`, one reviewer) in the measurement worktree, and tell lane 4: one PR serves both lanes.
Then, host free, through memguard: `python3 -m eval.secbench --no-build --binary <prism-base> --binary-sha <sha>
--sut-commit <commit> --inspection ~/prism-evidence/meas/secbench/repair-r1/inspection-final.jsonl --out
~/prism-evidence/param-binding/m6`. The harness seeds entry formals directly (DESIGN 2), so call-site binding rarely
moves its counts.

### Task M-4: go or no-go and memo

- [ ] Apply DESIGN section 6: A always; B when M2 ≥ 300 sites over X, T, SB or M6 ≥ 5 cases first-broken at a rest
      parameter; C when M3 has no checker-WRONG `exact` row; D when M4 ≥ 1,000 callbacks reading the fed parameter and
      array plus promise ≥ 90% of typed receivers; E when M5 ≥ 300 capped sites.
- [ ] Parent gate (DESIGN 6): sum over passing slices the predicted sites (B: M2; C: distinct call sites of M3's
      CORRECT rows; D: M4 reading callbacks; E: M5) and the predicted benchmark conversions (M6 first breaks at their
      shape). Below 500 sites and below 30 traces, the lane stops after slice A.
- [ ] Write `CENSUS.md` here: per-corpus tables (aggregate counts; `private-corpus` as a count column), the M3 named
      rows, the go/no-go table with numbers beside thresholds, the parent-gate sum, the registration case count.
      Post it on #604. Thresholds are never adjusted by the orchestrator; a contradicted design premise goes to
      ORCHESTRATION section 8. A no-go group stays disabled, its census number in the README.

## Stage T and E: test matrix (3 days)

### Task T-1: matrix data and one data-driven test

**Files:** create `tests/fixtures/param_binding_matrix/rows.jsonl`, `tests/fixtures/param_binding_matrix/README.md`,
`src/cpg/param_binding_matrix_tests.rs` (declared with `#[cfg(test)] mod` in `src/cpg.rs`).

Each row: `id`, `group` (`control`, `A`, `B`, `C`, `D`, `E`), `langs` (`js`, `ts`, `tsx`; `go`, `py` in controls),
`files` (name → source), `expect`, `class` (the census class), `truth`. Assertions, all evaluable on main with
`object_literal_arg_tests::{build, spell}` and spelling `function:path@line` (anonymous owners `<anon@L:C>`): `def`,
`edge` (Use → Def, optional label `exact` or `nameonly/<doubt>`), `no_edge`, `no_edge_into`, `resolves` (site line and
callee → targets, confidence, kind, via `call_graph.resolve_call_site_full`), `drop` (site → reason), `reaches` and
`not_reaches` (`dfg_forward_reachable`, which review taint and slicing follow), `trace_boundary` (the Exact-gated
`taint_trace` stops there), `counter` (a key of `dfg_label_stats` or `navigation::queries::call_stats`; absent is 0).

One test, `param_binding_matrix_rows`, with `const ENABLED: &[&str] = &["control"];`. Control rows pass on main; each
other group is enabled once by hand, its RED run recorded in the ledger, and the constant restored. Rows asserting
`exact` use callee bodies of two or more lines. `/` in the tables separates source lines.

**control:** c01 `f(v)` into `function f(a) {`/`use(a);`/`}`: edge exact. c02 `const g = (x) =>`/`use(x);`: exact.
c03 Go `f(xs...)`, Python `f(*xs, v)`: edges as main. c04 `f(...xs, { a: v })`: no literal edge. c05
`arr.forEach(e =>`/`use(e))`: the site drops `UnknownName`. c06 `function f() {`/`use(arguments[0]);`/`}`: no edge.

| Row | Shape | Required |
|---|---|---|
| a01 | `f(...list, v)` into `function f(a, b) {`/`use(a, b);`/`}` | no edge into `f:b@1` (main: wrong `exact` edge, probe p04b) |
| a02 | `f(...list)`; `f(v, ...list)` | no edge from `list` (p04a); `v → f:a@1` exact, nothing into `b` |
| a03 | `f(v, ...list, w)`; `f(.../*c*/list, v)`; `f(...(xs as any[]), v)`; `f(`/`...xs,`/`v`/`)` | `v → a` in the first; nothing from `w`; the rest as a01 |
| a04 | `new C(...xs, v)` into `constructor(p, q)`; `this.m(...xs, v)` into a method | no edge into the second formal |
| b01 | `f(v, 2)` into `function f(...args) {`/`use(args[0]);`/`}` (p01); `const f = (...args) =>` (p13); method; TS `...args: any[]` | `v → f:args@1`, the call's label |
| b02 | `f(a, v, w)` into `function f(p, ...rest)`; `f(...xs)` into `f(...rest)`; `f(a, ...xs)` | `a → p`, `v → rest`, `w → rest`; `xs → rest`; `a → p`, `xs → rest` |
| b03 | `f(...xs, v)` into `f(p, ...rest)` | nothing into `p`; truth `ambiguous` for `v` (stage D) |
| b04 | TS `function f(this: T, a, ...rest)` with `f(x, y)` | `x → a`, `y → rest` |
| b05 | `function f(...[a, b])`; rest rebound by a nested callable (D11) | nothing |
| b06 | callee resolves name-only | `v → rest` labelled `nameonly/call_nameonly` |
| b07 | `f(q, { a: x })` into `function f(q, ...p)` | no edge from `x` (as main) |
| b08 | a rest Def whose value reaches a cross-file `free_single` Exact argument edge | its outgoing edges refused, as main does for new sources |
| k01 | `const g = x =>`/`use(x);` with `g(v)` (p03a); `async x =>` | `v → g:x@1` exact |
| k02 | script `a.js` `var o = { Array: t =>`/`sink(t) };`, script `b.js` `Array(h)` | no `exact` edge into `t` |
| k03 | `o.readFile = fileName =>`/`read(fileName);`, a bare `readFile(n)` elsewhere | no `exact` edge into `fileName` |
| k04 | `{ bind: binder =>`/`use(binder) }`, a bare `bind(args)` | no `exact` edge into `binder` |
| k05 | `f({ a: x })` into `const f = p => {`/`sinkA(p.a);`/`};`; bare formal rebound (D11) | `x → f:p.a@2`; nothing |
| h01 | `arr.forEach(`/`e =>`/`use(e)`/`)` (p05); `arr.map(function (e) {`/`return use(e);`/`})` (p06); `filter`, `find`, `findIndex`, `some`, `every`, `flatMap` | `main:arr@L → <anon@..>:e@..`, label `nameonly/builtin_callback` |
| h02 | `arr.reduce((acc, x) =>`/`use(x), init)`; `reduceRight` | `arr → x`; nothing into `acc` |
| h03 | `p.then(`/`r =>`/`use(r)`/`)` (p08); `p.then(ok, err)` | `p → r`; only `ok`'s first formal |
| h04 | `arr.forEach(handler)`, `function handler(e) {`/`use(e);`/`}` same file (p09) | `arr → handler:e@1`, builtin label |
| h05 | `arr.forEach(handler)` with `handler` imported, a parameter, or a `let` | no edge |
| h06 | `(e, i) =>`; no formals; `({ a }) =>` | only `e`; nothing; nothing (lane 3) |
| h07 | `arr?.forEach(…)`; `this.items.forEach(…)`; `o.list.map(…)` | edge from the receiver path |
| h08 | `class Box { map(f) {…} }` and `b.map(v =>`/`use(v))` | the edge, labelled `builtin_callback`, never `exact` |
| h09 | `Object.keys(o).forEach(k => …)` (p07); `arr.map(f).filter(y => …)`; `fetch(u).then(r => …)` | truth from the enumerators; stage D decides receiver forms |
| h10 | `.catch(e => …)`; `.finally(…)`; `emitter.on('x', r => …)` (p11) | no edge (not in the table) |
| h11 | h01 through both engines | `reaches` from `arr` to `use(e)`; `trace_boundary` at the edge |
| e01 | `function run(cb) {`/`const input = read();`/`cb(input);`/`}`, `run(handler)`, `function handler(e) {`/`use(e);`/`}` (p10b) | `cb(input)` resolves to `handler`, `nameonly`, kind `argument_follow`; `run:input → handler:e@1` labelled `nameonly/call_nameonly` |
| e02 | `run(w =>`/`use(w))` (p10a); `run((w) =>`/`use(w))` | resolves to `<anon@..>`; parenthesised edge exists; bare edge per stage D |
| e03 | two callers passing two functions; one passing a function and one a number | both targets; one |
| e04 | `cb(a, b)` into `handler(x, y)` and into `(x) => …` | `a → x`, `b → y`; `a → x` only |
| e05 | nine callers passing nine distinct callables | drop `ArgFanout`, no edge, counter `js_arg_follow.fanout` = 1 |
| e06 | `cb = other;` in the holder; a name-only caller; `run(...fns)`; `cb.call(this, x)` | dropped as main |
| e07 | exported holder called through an import from another file | targets from that caller |
| e08 | `function outer(f) { run(f); }` with `outer(handler)`; depth 3 | target `handler`; nothing beyond depth 2 |
| e09 | a parameter site resolved today by name search; `function run(cb = noop)` called as `run()` | as main; target `noop` (#434 §3 rule 4) |
| e10 | e01 through both engines | `reaches` into `handler`; `trace_boundary` at the call |

- [ ] Write rows, README (schema, assertion kinds, census class per row) and the test. Control green on main; RED runs
      recorded. Commit; CI green with only `control` enabled.

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION section 6. Axes: argument syntax (spread position and count, comments, wrappers, TS
      assertions, multi-line, `new`, optional calls, tagged templates); formals (rest, defaults before rest, TS `this`,
      overloads, bare and parenthesised arrows, async, generators); callee naming (declaration, `const` arrow, object
      member, assignment, inferred name, class member, anonymous); callback forms (inline, same-file named, imported,
      parameter, wrapped, bound); table methods, receivers, chains, user methods of the same name; holders (exported,
      escaping, reassigned, forwarded, defaulted, destructured `({ cb })`); file kind; shadowing. Census classes are rows.

## Stage D: design of record (1 day)

- [ ] The design seat amends DESIGN section 5 for B to E against the full rows and `CENSUS.md`: the rest rule when a
      spread precedes the rest index (b03); any refusal C keeps beyond today's outgoing one (M3); for D the receiver
      forms (h09), the named-callback rule, whether bare callback formals bind, the doubt kind's rank in
      `FlowConfidence::worst`; for E the #434 rules with anonymous targets, whether a bare-arrow target binds when C did
      not run (e02), the cap's counting. Inputs: DESIGN 4 and 5, #434 §3, the rows with truths, `CENSUS.md`, the
      accepted-loss candidates (`ambiguous` rows, M3's UNDECIDED list). Slice A's rule is final and not reopened. The
      implementer then updates expectations. Both reviewers; cap 2 rounds. Owner decisions 2 and 5 end here.

## Stage I: slices

Order: A (unconditional; it starts once T-1 is committed and T-2's spread axis is dry, since its rule is final); B
and C (same loop, smallest); D (a separate pass; it builds the anonymous-formal lookup E reuses); E0, E1, E2 (the only
slices that change call resolution, last so their differential is not mixed with others). B to E run only if M-4
marked them go and the parent gate allows.

### Slice A: spread arguments stop positional binding (2 days)

**Files:** create `src/cpg/spread_args.rs`, `src/cpg/argument_nodes.rs`, `src/cpg/param_binding_parity_tests.rs` (all
declared in `src/cpg/build.rs` with `#[path]`, the test under `#[cfg(test)]`); modify `src/cpg/build.rs`, both twins,
`src/cpg/param_binding_matrix_tests.rs` (`ENABLED` gains `"A"`), `src/cpg_cache.rs`, docs.

```rust
// src/cpg/spread_args.rs — first argument that is a JS/TS/TSX `spread_element`; None for other languages
pub(super) fn first_spread(caller: &ParsedFile, args: &[(String, std::ops::Range<usize>)]) -> Option<usize>;
// src/cpg/argument_nodes.rs — the full-path and base-path Use nodes of one argument (today inline in the loop)
impl CodePropertyGraph {
    pub(super) fn argument_nodes(caller_id: &FunctionId, caller_parsed: &ParsedFile, arg_text: &str,
        arg_span: &std::ops::Range<usize>, var_index: &VarIndex, exact_var_index: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>) -> Vec<NodeIndex>;
}
```

- [ ] Step 1, no behaviour change: move the loop's full-path/base-path block into `argument_nodes` (twins keep their
      copies). Tests green; commit alone.
- [ ] Enable `A`: RED (a01, a03, a04). In the three copies compute `first_spread` once per site and stop the positional
      loop at `min(args.len(), first_spread)`. GREEN.
- [ ] `spread_and_rest_graph_is_identical_across_every_route`, modelled on
      `object_literal_parity_tests::literal_bearing_graph_is_identical_across_every_route`: production, legacy twin,
      serial reference, full hit, partial hit; pinned edges for a01 and a02.
- [ ] Differential (X, T, SB, G): `spread-removed` (each LOST row passes `--mode spread`; total equals M1),
      `e5-unrefused` (an outgoing edge of a new source back because a removed edge was its only cross-file
      `free_single` seed; listed). Anything else, or any call-stats counter outside `dfg_labels`, is a STOP.
- [ ] Benchmark at the head (Task M-3 command, new `--out`): traced cases do not drop in any class.
- [ ] Tier-A; cloud tests at 1 and 4 threads with RED-on-base; two reviewers. PR body: prediction, M1 beside the
      removed count, the `e5-unrefused` list, benchmark table, Tier-A.

### Slice B: rest parameters (2 to 3 days)

**Files:** create `src/cpg/rest_params.rs` (declared in `build.rs`); modify `src/cpg/callee_params.rs` (a second memo
map in `ParamCache`; `ParamTarget::from_node`), `src/cpg/build.rs`, both twins, `src/cpg/js_param_defs_tests.rs`
(`identifier_rest_formal_gets_a_def_but_never_a_positional_argument` becomes
`identifier_rest_formal_binds_trailing_arguments`), the matrix test (`"B"`), the parity test (b02), `src/cpg_cache.rs`, docs.

```rust
// src/cpg/rest_params.rs
/// Identifier rest formal: index (formals before it, a TS `this` not counted) and Def, byte-verified.
pub(super) fn compute_rest_def_node(parsed: &ParsedFile, callee: &FunctionId, var_index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>) -> Option<(usize, NodeIndex)>;
/// Edges from the arguments that land in the rest array (stage D's rule) to its Def.
pub(super) fn collect(out: &mut Vec<PendingArgumentFlow>, site: &object_literal_args::SiteInputs<'_>,
    rest: (usize, NodeIndex), first_spread: Option<usize>, label: FlowConfidence);
// src/cpg/callee_params.rs
impl ParamCache<'_> { pub(super) fn rest(&self, callee: &FunctionId, parsed: &ParsedFile, index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>) -> Option<(usize, ParamTarget)>; }
```

`collect` uses `argument_nodes`, binds a spread's inner expression where stage D admits it, and sets
`cross_file_free_single_exact` as positional edges do. It runs after the positional loop in the three copies.

- [ ] Enable `B`: RED. Implement. GREEN.
- [ ] Differential: `rest-added` (graded `--mode rest`, 0 WRONG among `exact`), `e5-refused` (LOST outgoing edges of a
      new source that a new rest edge now seeds; each with its seed). Call-stats counters outside `dfg_labels` unchanged.
- [ ] Benchmark; Tier-A; cloud tests; two reviewers. PR body: M2 beside added sites, oracle table, `e5-refused` list.

### Slice C: unparenthesised arrow parameters (2 days)

**Files:** modify `src/cpg/callee_params.rs` (drop the `bare_formal` filter; rewrite the comment at lines 155-159),
`src/cpg/js_param_defs_tests.rs` (`bare_arrow_formal_stays_a_step5b_hole_but_the_parenthesised_control_binds` becomes
`bare_arrow_formal_binds_like_the_parenthesised_control`), `src/cpg/object_literal_arg_tests.rs`
(`destructured_rest_and_bare_arrow_formals_are_out_of_scope`: the bare case gains `h:x@9 -> f:p.a@2`),
`src/cpg/object_literal_args.rs` (doc lines 13-14), the matrix test (`"C"`), `src/cpg_cache.rs`, docs (twins: no edit).

- [ ] Enable `C`: RED (k01, k05). Implement. GREEN; k02 to k04 hold.
- [ ] Differential: `bare-added`, `bare-literal-added` (plain mode, 0 WRONG among `exact`), `e5-refused` as in B. The
      ADDED set equals M3's probe set on the same corpora; any difference is explained in the PR.
- [ ] Benchmark; Tier-A; cloud tests; two reviewers.

### Slice D: built-in higher-order callbacks (4 to 5 days)

**Files:** create `src/cpg/builtin_callbacks.rs` (declared in `build.rs`); modify `src/cpg/flow_confidence.rs`
(`FlowDoubt::BuiltinCallback`, its badness rank, `FlowConfidence::doubt_spelling(self) -> Option<&'static str>` and
`kill_line(self) -> Option<u32>`), `src/cpg/build.rs` (module line; `DfgLabelStats::dfg_label_nameonly_builtin_callback`;
arms in `record_label` and the stats subtraction; the call), both twins, `src/navigation/queries.rs` (the `--edges` doubt
match becomes the two helper calls, so the file shrinks), `src/cpg/callee_params.rs`,
`crates/prism-review/tests/cli/min_confidence_test.rs` (stats field list, doubt vocabulary), every other exhaustive
`FlowDoubt` match the compiler reports, the matrix test (`"D"`), the parity test (h01, h04), `src/cpg_cache.rs`, docs.

```rust
// src/cpg/builtin_callbacks.rs
pub(super) const TABLE: &[(&str, usize)] = &[("forEach", 0), ("map", 0), ("filter", 0), ("find", 0),
    ("findIndex", 0), ("some", 0), ("every", 0), ("flatMap", 0), ("reduce", 1), ("reduceRight", 1), ("then", 0)];
/// Receiver Use -> callback formal Def for a member call named in TABLE, whatever the site resolves to.
/// Label `NameOnly(BuiltinCallback)`; `cross_file_free_single_exact` false.
pub(super) fn collect(out: &mut Vec<PendingArgumentFlow>, site: &CallSite, caller_id: &FunctionId,
    caller_parsed: &ParsedFile, var_index: &VarIndex, exact_var_index: &ExactVarIndex, graph: &DiGraph<CpgNode, CpgEdge>);
// src/cpg/callee_params.rs (compute_param_def_nodes keeps its signature and result)
pub(super) fn locate_function<'a>(parsed: &'a ParsedFile, callee: &FunctionId) -> Option<tree_sitter::Node<'a>>;
pub(super) fn formal_def_nodes(parsed: &ParsedFile, function: &tree_sitter::Node<'_>, owner: &FunctionId,
    var_index: &VarIndex, graph: &DiGraph<CpgNode, CpgEdge>, admit_bare: bool) -> Option<Vec<Option<NodeIndex>>>;
```

`locate_function` finds anonymous ids with `anon_owner::node_for_id` and named ids by today's unique
line-range-and-name match. The callback is argument 0 of the call at `site.start_byte`: an inline arrow or function
expression gives its owner through `anon_owner::identity`; an identifier gives a same-file callable when
`js_ts_site_binding` returns `JsBinding::Callable`. The receiver is the call's member object, read with
`argument_nodes`. Stage D fixes the receiver forms and `admit_bare`. `collect` runs per site before the resolved-target loop.

- [ ] Step 1, no behaviour change: split `compute_param_def_nodes` into `locate_function` and `formal_def_nodes`; commit.
- [ ] Enable `D`: RED. Implement the doubt kind (serialisation, `dfg-stats` spelling `builtin_callback`, stats field),
      then `collect`. GREEN; h08 and h11 hold. Add the `builtin_callback` arm to the evidence copy of `byte_dump.rs`.
- [ ] Differential: `builtin-added` only (`--mode builtin`; WRONG rate reported, at most 5%); the new label counter
      equals the added count; call-stats counters outside `dfg_labels` unchanged.
- [ ] Benchmark; Tier-A; cloud tests; two reviewers. PR body: M4 beside the added count, receiver types of added edges.

### Slice E0: move-only split of `src/call_graph.rs` (1 day; only if E is go)

- [ ] Move `compute_indirect_call_sites` (`src/call_graph.rs:2392`) and the private helpers only it calls into the child
      module `src/call_graph/indirect.rs` (a child module sees `CallGraph`'s private fields). Byte-identity check of the
      moved text; targeted `call_graph` tests; one reviewer reading a pure move; sizes before and after in the PR. E1 adds
      back at most six lines, so the file ends the lane smaller than it started.

### Slice E1: argument-follow facts and counters, no resolution change (3 days)

**Files:** create `src/resolution/js_ts/arg_follow.rs`, `src/resolution/js_ts/arg_follow/value.rs`,
`src/resolution/js_ts/arg_follow_tests.rs` (`#[cfg(test)]` in `arg_follow.rs`), `src/ast/js_ts/param_slot.rs` (`mod
param_slot;` in `src/ast/js_ts.rs`); modify `src/resolution/js_ts.rs` (`pub(crate) mod arg_follow;`), `src/call_graph.rs` (field `#[serde(default)] pub(crate) js_ts_arg_targets:
crate::resolution::js_ts::arg_follow::ArgIndex`, its reset in `clear_indirect_calls`, one call in
`recompute_indirect_calls`), `src/navigation/queries.rs` (key `js_arg_follow` from `arg_follow::stats`), both cache files, docs.

```rust
// src/resolution/js_ts/arg_follow.rs (all derive Clone, Debug, PartialEq, Eq, Serialize, Deserialize)
pub(crate) struct ArgIndex { pub sites: BTreeMap<(String, usize), ParamSlot>,   // (file, site start byte)
                             pub outcomes: BTreeMap<ParamSlot, ArgOutcome> }   // + Default
pub(crate) struct ParamSlot { pub holder: FunctionId, pub position: usize, pub path: Option<String> } // + Ord
pub(crate) enum ArgOutcome { Targets(BTreeSet<FunctionId>), Fanout(usize), Empty(String) }
/// #434 §3 rules 2-7 and §4. Computed while the index is empty, so ArgumentFollow outcomes (never Exact)
/// cannot enter the Exact-caller sets; forwarding to depth 2; cap 8 targets.
pub(crate) fn compute(cg: &CallGraph, files: &BTreeMap<String, ParsedFile>) -> ArgIndex;
pub(crate) fn stats(cg: &CallGraph) -> BTreeMap<String, usize>;
// src/ast/js_ts/param_slot.rs — `Pos` is private to `ast::js_binding_walk`, so the binder's answer is exposed here:
// for the callee identifier of the call at `site_start`, the nearest binding is formal `position` of the callable
// starting at byte `holder_start`; None for any other binding.
impl ParsedFile { pub(crate) fn js_ts_param_slot_at(&self, site_start: usize, callee: &str) -> Option<(usize, usize)>; }
```

The facts are read from the parsed files when the index is built, not stored as the #434 plan's new `CallSite` fields
(`js_param_slot`, `js_args`, which would grow `src/call_graph.rs`): the slot from `js_ts_param_slot_at` (the binder's `Pos::Param`),
values through `js_ts_follow_alias`, inline callables through `anon_owner::identity` (`value.rs`). The index is
persisted with the call graph (a full hit restores it) and recomputed on cold and partial builds.

- [ ] Unit tests first: one per value shape of #434 §3 rule 4; rule 2 (written parameter); the cap; forwarding depth;
      a TS `this` parameter; a JSX attribute after a spread; `index_partial_hit_equals_cold`, `index_full_hit_equals_cold`.
      RED against a skeleton.
- [ ] Implement. Gate: `call-stats --dump-sites` byte-identical on X and T apart from the new key; DataFlow edges
      identical; group E rows unchanged; cache size on X and T before and after in the PR. Tier-A; cloud tests; two reviewers.

### Slice E2: the argument-follow route (4 to 5 days)

**Files:** modify `src/resolution/js_ts/anonymous.rs` (in `resolve_call_site_full`, before `admit_and_filter_family`:
`let outcome = self.js_ts_argument_follow(site, outcome);`), `arg_follow.rs` (that method), `src/resolution.rs`
(`ResolutionKind::ArgumentFollow`, `as_str` "argument_follow"; `DropReason::ArgFanout`), `src/navigation/queries.rs` (the
`ArgFanout` arm), other exhaustive matches the compiler reports (`rg -n 'ResolutionKind::ParameterCallback' src crates`),
`src/cpg/callee_params.rs` (`compute_param_def_nodes` locates anonymous callees with `locate_function`, created here if D
did not run), `tests/integration/js_alias_rule_test.rs` (no `ArgumentFollow` outcome has `via_alias_route`, #434 §4), the
matrix test (`"E"`), both cache files, docs.

`js_ts_argument_follow` changes only a drop with reason `UnknownName` or `LocalBindingUnproven`, no target, and a slot in
the index: `Targets` gives `NameOnly` targets of kind `ArgumentFollow`; `Fanout` gives `dropped(DropReason::ArgFanout)`;
`Empty` keeps the drop. Argument binding then follows in the unchanged loop (edges `nameonly/call_nameonly`).

- [ ] Enable `E`: RED. Implement. GREEN; e05 holds.
- [ ] Differential: `argfollow-call` (targets graded `--mode argfollow` against M5's `param-sites.jsonl`, WRONG rate at most
      5%), `argfollow-flow` (use→def into the targets' formals); expected counter moves (`resolved` up and the two drop
      reasons down by the same sites, `return_flow_skipped_nameonly`); 0 changes at sites resolved on main.
- [ ] Tier-A `--quick` regressions pasted; benchmark; cloud tests; two reviewers.

## Close-out (2 days)

- [ ] `REPORT.md` (evidence directory; summary appended to `CENSUS.md`): per slice the prediction, added and removed
      edges, oracle tables, benchmark per class, the parent gate. Comments: #595 (B, C; close when both merged or no-go),
      #434 (E; close or park with the census), #604 (items 1 and 4, the parent gate's numbers).
- [ ] Hand-over: one comment on #600, referenced from lane 12's follow-up, listing the cases first-broken at
      `callback_argument_parameter_registration` on the current build with outcomes; this lane builds nothing for them.
- [ ] Docs (in each slice PR, checked here): `CLAUDE.md` (JS/TS parameter Defs paragraph, `argument_follow` ladder note,
      `dfg-stats` doubt vocabulary); known-gaps section 1.1 row "A parameter" and section 6 line "Missing: rest formals…".
- [ ] Follow-up issues with census numbers: `arguments` binding (owner decision 3); `.call`/`.apply`/`.bind`; promise
      chains; D's refused receiver forms; Python `f(*xs, v)` misbinding (not verified); trace-engine crossing of
      name-only edges (lane 8). Handoff refreshed; raw dumps offloaded; `target/` removed.

## Acceptance for the lane

Slice A merged, every removed edge spread-misbound. Every other slice merged with DESIGN section 7 met (no WRONG among
`exact` edges; name-only WRONG rate reported and at most 5%; only its own change classes; resolution counters move only in
E2; traced benchmark cases not lower in any class), or recorded no-go with its census number. The parent gate on the merged
head: at least 30 benchmark traces or 500 sites, counting only sites whose added rows are all checker-CORRECT. A failed gate
is reported with its numbers and the lane stops for the owner.

## Estimates (working days, review rounds included)

Stage M 4; stages T and E 3 (1 if only A runs); stage D 1 (none if only A runs); slice A 2; B 2 to 3; C 2; D 4 to 5;
E0 1, E1 3, E2 4 to 5; close-out 2. Total: 9 if only A runs, 28 to 31 if every slice runs.
