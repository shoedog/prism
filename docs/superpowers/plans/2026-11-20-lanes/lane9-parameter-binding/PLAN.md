# Lane 9: finish JavaScript parameter binding — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** arguments reach parameters for spread arguments (by runtime offset: no misbound edge), rest parameters,
unparenthesised arrow parameters, callbacks of built-in higher-order methods and calls through a callback parameter;
each slice after A only if its census gate passes; every added edge graded by the TypeScript checker.

**Architecture:** slices A to C change the positional loop of the argument-binding step (production and its two
test-only twins) through new `src/cpg/` modules, under DESIGN 5's call identity, runtime offset and binding
admission; slice D adds one pass beside that loop with a new name-only doubt kind; slice E adds a derived index of
what admitted Exact callers pass to a parameter and a name-only fallback in the one public resolution tail, after the
#434 plan.

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
  and the arity checks reading it (setting it for JavaScript would change call resolution, a DESIGN anti-goal), the
  results of `refuse_new_sources_exposing_free_single_calls` (slice A adds a stderr trace only), call resolution
  outside E2 (the unimported-export defect is filed, DESIGN 2), `src/cpg/trace.rs` and lane 8's descent gate (the
  check of DESIGN 5 is requested from lane 8), `src/reasoning/scope_honesty.rs` (its "JS/TS callback" warning
  describes touched functions, not missing flow), taint code, `arguments` handling.
- Weighted sizes (`python3 scripts/file_size.py`): `src/call_graph.rs` 5,653, `src/resolution.rs` 4,171.5,
  `src/navigation/queries.rs` 2,502.5: no PR grows any of them, measured against that PR's base and listed in the PR
  body before and after (a reduction in an earlier PR does not count); E0 moves code out first.
  `src/cpg/build.rs` 1,777 (module lines, calls, one stats field, match arms); `src/cpg_cache.rs` 1,544. Logic goes
  in new modules.
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
  comment, one expectation and the binding-admission skip there, and rebases over lane 3.

## Review focus

1. `f(...list, v)` into `function f(a, b)`: no edge into `b`; `f(...[], v)` gives `v → a`, `f(...[0, 1], v)` `v → c`;
   `this.m(...xs, v).m(w, z)` gives the inner `m` nothing from `w, z`; production, twins and both cache paths agree.
   Rows a01, a02, a05, a06; `param_binding_parity_tests::spread_and_rest_graph_is_identical_across_every_route` (A, B).
2. A bare-arrow or rest formal of a module export named like a global, called by that bare name from a module that
   does not import it: no edge into the formal; an importing caller binds. Rows k02 to k04, k06, k07 (B, C).
3. A user-defined method named `map` or `then` gets the callback edge labelled `builtin_callback`, never `exact`;
   the edge starts at the receiver node the graph has; the trace engine does not cross it, even with a direct call of
   the callback on the same line. Rows h07, h08, h11, h13 (D).
4. A holder called with nine distinct callables drops as `ArgFanout` with no edge, anonymous targets counted; a false
   Exact caller (an unimported global spelling) contributes no target. Rows e05, e12 (E2).
5. A partial cache hit where only a caller's file changed gives the same argument-follow index and labelled edges as a
   cold build. `arg_follow_tests::index_partial_hit_equals_cold`, `activated_route_parity` (E1, E2).

## Stage M: census, oracle, benchmark (5 days)

### Task M-1: workspaces, binaries, capture script and oracle (evidence directory)

**Files:** `bin/BINARIES.md`, `census/capture.sh`, `tools/adjudicate9.cjs`, `tools/enrich.py`, copies of `rowdiff.py`,
`byte_dump.rs`, `oracle-environments.cjs`, `oracle-read-role.cjs`, `tools/fixtures/`.

- [ ] Both worktrees and the evidence directory (ORCHESTRATION section 4). Build main in the measurement worktree through
      memguard; copy to `bin/prism-base`; record `--version` and SHA-256. Comment on #595, #434, #604: lane started.
- [ ] `capture.sh <binary> <corpus> <out>`, through memguard: `nav --no-cache call-stats --repo <corpus> --dump-sites`
      to `sites.jsonl`, and `PRISM_EDGES_BYTES=1 nav --no-cache dfg-stats --repo <corpus> --edges | LC_ALL=C sort` to
      `edges.jsonl`; for SB one run per package root.
- [ ] Copy `adjudicate.cjs`, `rowdiff.py`, `byte_dump.rs`, `oracle-environments.cjs`, `oracle-read-role.cjs` from
      `docs/superpowers/plans/2026-10-06-js-param-defs/probes/` (the grader `require`s the last two). `byte_dump.rs` is
      built as `examples/r1_byte_dump.rs` in a scratch copy of each binary's commit (its header says how) and never
      enters the repository; change its synthetic owner spelling from `<cb@L:C>` to the graph's `<anon@L:C>`.
- [ ] `enrich.py <edges.jsonl> <byte_dump.jsonl> <out>`: one enriched schema, the only grader input: join the wire rows
      (bytes, no owner) with the dumper's rows by file and bytes as multisets, adding owner, owner bytes, access kind
      and parameter flag (the grader finds the call from the Use's bytes). A row that joins to none or to
      several is `unjoined` and graded UNDECIDED, never dropped.
- [ ] `adjudicate9.cjs` keeps `adjudicate.cjs`'s grading and dispatches by transfer shape (DESIGN 7), with call
      identity by full span (`CallExpression` or `NewExpression`, the innermost one whose argument holds the Use) and
      the runtime implementation behind TS overload declarations. Modes:
      `binding` (Use → Def: the argument's runtime offset (DESIGN 5) against the formal's slot, a TS `this` taking
      none; at or after a rest index the rest formal is CORRECT when the least offset reaches it; member-span operands
      accepted; an unknown offset UNDECIDED; replaces `useDef`'s `position_or_rest`);
      `literal` (Use → Use: a literal leaf to a read of the formal's member path, CORRECT when the key path is a prefix
      or extension, WRONG for a sibling field);
      `spread` (a LOST row: `proved-wrong` when its offset is exact and differs, `unproved` when unknown, `correct` when
      exact and equal; `chain` when the row's Use lies outside the resolved call's span);
      `builtin` (a second program with the default lib: CORRECT only when `getResolvedSignature` of the call gives a
      declaration whose source file is a TypeScript library file (`lib.*.d.ts`) and the fed formal has the element role
      there; WRONG when the declaration is in the repository and its body demonstrably does not pass the receiver's
      element to that slot; UNDECIDED otherwise, including a receiver whose method is assigned in the file);
      `argfollow` (call target: each holder caller resolved by the checker, independently of the census: CORRECT when
      the checker resolves the caller to the holder, the passed value to the target's declaration and the slot or key
      path matches; WRONG when it resolves the caller or the value elsewhere; UNDECIDED without a witness. The DFG
      rows into the target's formals are graded by `binding` separately). The census's product target set is kept as a
      second ledger for yield only.
- [ ] Self-test every mode on hand-labelled fixtures with a positive, a wrong-target or wrong-slot mutation and an
      undecided case per shape (constructor, overload, TS `this`, member span, nested call, same-line same-name owners,
      synthetic owner, Use → Use literal, P6's holder) before use; outputs in `tools/SELFTEST.md`.

### Task M-2: the six measures

Scripts in `census/` read `capture.sh` output, run `node --max-old-space-size=12288` with the pinned checker, never
execute corpus code, accept `--aggregate-only`, exclude directories as `~/prism-evidence/js-args/census.cjs` does, and
write `census/<Mn>/<corpus>/aggregate.json`. Write expected magnitudes in `census/PREDICTIONS.md` first.

| Id | Script | Counted | Grading |
|---|---|---|---|
| M1 | `m1_spread.cjs <root> sites.jsonl edges.jsonl <out>` | call and `new` sites with a spread element followed by another argument, by resolution (exact singleton, name-only, several, dropped) and spread kind (known-length literal, other); cross-function rows today whose Use lies in an argument after the spread, sites and rows apart; rows whose Use lies outside the resolved call's full span (chains) apart | `--mode spread`: proved-wrong, unproved, correct per row |
| M2 | `m2_rest.cjs` | exact-singleton sites whose in-repo callee has an identifier rest formal with a Def, passing a variable argument (a Use in its span) whose least runtime offset reaches the rest index; split by a spread before the index; cross-file `free_single` sites apart (DESIGN 5, binding admission) | admitted sites with a variable argument count for the gate |
| M3 | `m3/lift-d12.patch`, `m3/run.sh` | a probe of main without the filter `Some((occurrence.1, occurrence.2)) != bare_formal` in `compute_param_def_nodes` (scratch worktree, never pushed); `rowdiff.py` base vs probe on X, T, SB; the three PROBE-LOG P11 rows (`debug.ts:1057 Array(height)`, `incrementalUtils.ts:493 readFile(fileName)`, `deprecations.ts:97 bind(args)`) reported by name; rows at cross-file `free_single` sites in a class of their own (binding admission removes them) | ADDED and LOST by label, class and mode (`binding`, `literal`); WRONG among `exact` outside the `free_single` class is the gate number; UNDECIDED `exact` rows listed, up to 50 read with a reason each; distinct sites whose rows are all CORRECT |
| M4 | `m4_builtin.cjs` | member calls named in DESIGN 4 B with an inline function or identifier at argument 0 whose fed formal (first; second for `reduce`, `reduceRight`) has a Def and is read, and whose receiver node exists by DESIGN 5's rule (the rest counted as `receiver-absent`); callback kind (inline arrow, bare arrow, function expression, same-file named, other); receiver form (identifier, member, `this` path, call, literal, chain); per method and receiver class (array, tuple, typed array, `Map`, `Set`, promise, class, untyped), the checker's declaration: library file, repository, none; whole-array and `thisArg` reads recorded | wrong-receiver rate = repository / (library + repository); `--mode builtin` on a sample of 200 for the role check |
| M5 | `m5_argfollow.cjs` (copy of `~/prism-evidence/js-args/census.cjs` plus `follow.product.prism_reverse_edge_sites_cap8`) | #434 census on current dumps: parameter call sites dropped today with a payload argument that has a Use node, whose holder's admitted Exact callers (no cross-file `free_single`) pass followable values, anonymous targets included, target set of 1 to 8, inside a named or anonymous caller (P12); set sizes; holder exported or escaping; `ArgFanout` drops apart | capped sites with a target |
| M6 | harness (Task M-3) | all classes on `prism-base`: outcome per class; `first_break.category` and `mechanism` per non-traced case beside the R1 rows (`~/prism-evidence/meas/secbench/repair-r1/final/entries.jsonl`), each with fresh per-case evidence; historical annotations without it stay unknown | first breaks at a lane shape as opportunities (rest: only a missing edge into an existing rest Def); `callback_argument_parameter_registration` cases listed for hand-over |

### Task M-3: harness options (eval-only PR shared with lane 4)

If lane 4's Task M-2 (`--class`, `--binary-sha`, `--sut-commit` in `eval/secbench/run.py`) has merged, use it.
Otherwise do that task exactly as lane 4's `PLAN.md` states it (same options, same three tests in
`eval/secbench/test_cli.py`, one reviewer) in the measurement worktree, as a harness task under ORCHESTRATION
section 6 (a Claude implementer; its brief carries the harness sentence, the options and the three tests), and tell
lane 4: one PR serves both lanes.
Then, host free, through memguard: `python3 -m eval.secbench --no-build --binary <prism-base> --binary-sha <sha>
--sut-commit <commit> --inspection ~/prism-evidence/meas/secbench/repair-r1/inspection-final.jsonl --out
~/prism-evidence/param-binding/m6`. The harness seeds entry formals directly (DESIGN 2), so call-site binding rarely
moves its counts.

### Task M-4: go or no-go and memo

- [ ] Apply DESIGN section 6: A always; B when M2 ≥ 300 sites over X, T, SB or M6 ≥ 5 cases first-broken at a missing
      edge into an existing rest Def; C when M3 has no checker-WRONG `exact` row outside the `free_single` class; D when
      M4 ≥ 1,000 callbacks and the library declaration ≥ 90% of typed receivers; E when M5 ≥ 300 capped sites.
- [ ] Parent-gate forecast (DESIGN 6): the union over passing slices of stable site ids (file and call span) predicted
      to convert (B: M2; C: M3 sites whose rows are all CORRECT; D: M4 sites; E: M5), each site once, and the predicted
      new traces (M6 opportunities at their shape, not counted as conversions). Below 500 sites and below 30 traces,
      the lane stops after slice A. Re-forecast after A merges and after B and C.
- [ ] Write `CENSUS.md` here: per-corpus tables (aggregate counts; `private-corpus` as a count column), the M3 named
      rows, the go/no-go table with numbers beside thresholds, the parent-gate forecast with overlaps removed, the
      registration case count.
      Post it on #604. Thresholds are never adjusted by the orchestrator; a contradicted design premise goes to
      ORCHESTRATION section 8. A no-go group stays disabled, its census number in the README.

## Stage T and E: test matrix (4 days)

### Task T-1: matrix data and one data-driven test

**Files:** create `tests/fixtures/param_binding_matrix/rows.jsonl`, `tests/fixtures/param_binding_matrix/README.md`,
`src/cpg/param_binding_matrix_tests.rs` (declared with `#[cfg(test)] mod` in `src/cpg.rs`).

Each row: `id`, `group` (`control`, `A`, `B`, `C`, `D`, `E`), `langs` (`js`, `ts`, `tsx`; `go`, `py` in controls),
`files` (name → source), `expect`, `class` (the census class), `truth` (`correct`, `wrong`, `ambiguous`; a row that
pins a defect on main has `truth: wrong` and names its issue). Assertions, all evaluable on main with
`object_literal_arg_tests::build` and a complete endpoint identity `file:function:path@line:col` (anonymous owners
`<anon@L:C>`; `col` is the occurrence's 1-based column, converted to bytes from the source and compared with the
node's `file`, `start_byte`, `end_byte`), the same identity for positive and negative assertions: `def`, `edge` (Use
→ Def or Use → Use, optional label `exact` or `nameonly/<doubt>`, optional multiplicity), `no_edge`, `no_edge_into`,
`resolves` (site by file, line, column and callee → targets, confidence, kind, via
`call_graph.resolve_call_site_full`), `drop` (site → reason), `reaches` and `not_reaches` (`dfg_forward_reachable`,
which review taint and slicing follow, from a source's prior Def), `trace_boundary` (the Exact-gated `taint_trace`
stops there), `counter` (a key of `dfg_label_stats` or `navigation::queries::call_stats`, `return_flow` keys included;
absent is 0).

One test, `param_binding_matrix_rows`, with `const ENABLED: &[&str] = &["control"];`. Control rows pass on main; each
other group is enabled once by hand, its RED run recorded in the ledger, and the constant restored. Rows asserting
`exact` use callee bodies of two or more lines. `/` in the tables separates source lines.

**control:** c01 `f(v)` into `function f(a) {`/`use(a);`/`}`: edge exact. c02 `const g = (x) =>`/`use(x);`: exact.
c03 Go `f(xs...)`, Python `f(*xs, v)`: edges as main. c04 `f(...xs, { a: v })`: no literal edge. c05
`arr.forEach(e =>`/`use(e))`: the site drops `UnknownName`. c06 `function f() {`/`use(arguments[0]);`/`}`: no edge.
c07 (known wrong on main, probes P3/P10, issue filed at close-out) module `a.js` `export function parseInt(vals) {`/
`inspect(vals);`/`}` and `export const escape = (s) =>`/`inspect(s);`, module `b.js` calling `parseInt(v)`,
`escape(v)` without an import: both sites `free_single` Exact, edges `v → vals`, `v → s` exact, truth wrong; no slice
changes this row. c08 (known wrong on main, P6) module `a.js` `export function setTimeout(cb, ms) {`/`cb(input);`/`}`,
module `b.js` with `import { setTimeout as localTimer }`, `localTimer(real, 0)` and a bare `setTimeout(other, 0)`: the
bare site resolves `free_single` Exact to `a.js`'s holder, truth wrong; no slice changes it.

| Row | Shape | Required |
|---|---|---|
| a01 | `f(...list, v)` into `function f(a, b) {`/`use(a, b);`/`}` | no edge into `f:b@1` (main: unproved `exact` edge, probe p04b) |
| a02 | `f(...list)`; `f(v, ...list)` | no edge from `list` (p04a); `v → f:a@1` exact, nothing into `b` |
| a03 | `f(v, ...list, w)`; `f(.../*c*/list, v)`; `f(...(xs as any[]), v)`; `f(`/`...xs,`/`v`/`)` | `v → a` in the first; nothing from `w`; the rest as a01 |
| a04 | `new C(...xs, v)` into `constructor(p, q)`; `this.m(...xs, v)` into a method; `f?.(...xs, v)` | no edge into the second formal |
| a05 | `this.m(...xs, v).m(w, z)` with `A.m(a1, a2)` returning a `B` (P8, TS and JS); `this.m(x).m(w, z)`; `cb(x)(y)` with a resolved `cb`; `new C(x).m(w)` | the inner call binds only its own arguments: no edge from `w`, `z` into `A.m` (main: two wrong `exact` edges); `x → a1`; outer arguments bind only through the outer call's own resolution |
| a06 | into `function f(a, b, c)` (P1): `f(...[0], v)`; `f(...[], v)`; `f(...[0, 1], v)`; `f(v, ...[0], w)`; `f(...[, 1], v)`; `f(...[...ys], v)`; `opt(...[1] as const, v)` into `opt(a = 1, b = 2)`; TS overloads | `v → b`; `v → a`; `v → c`; `v → a`, `w → c`; nothing from `v` (holes); nothing (nested spread); `v → opt:b`; `v →` the implementation's `b` |
| a07 | `f(...list, o.data)` | both of today's rows (`o`, `o.data`) removed; M1 counts one site, two rows |
| a08 | `f.call(null, ...[0], v)`; `f.apply(null, [0, v])` | dropped as main, no edge (measured exclusion) |
| a09 | `const origin = x =>`/`observe(x);`/`mid(...[], x);` and `function mid(unused, y) {`/`freeSink(y);`/`}`, `freeSink` cross-file (P2) | `x → mid:unused` (offset 0); `origin:x`'s outgoing edges follow the refusal's reachability: refusal accounting lists the change as a reachability change with its seed `y → freeSink:p` and witness path |
| b01 | `f(v, 2)` into `function f(...args) {`/`use(args[0]);`/`}` (p01); `const f = (...args) =>` (p13); method; TS `...args: any[]` | `v → f:args@1`, the call's label |
| b02 | `f(a, v, w)` into `function f(p, ...rest)`; `f(...xs)` into `f(...rest)`; `f(a, ...xs)` | `a → p`, `v → rest`, `w → rest`; `xs → rest`; `a → p`, `xs → rest` |
| b03 | `f(...xs, v)` into `f(p, ...rest)` | nothing into `p`; truth `ambiguous` for `v` (stage D) |
| b04 | TS `function f(this: T, a, ...rest)` with `f(x, y)` | `x → a`, `y → rest` |
| b09 | `g(...xs, v, w)` into `function g(p, ...r)` (P11); `g(...[], v)`; `g(...[0], v)` | `w → r` (least offset 1), `v` per stage D; `v → p`; `v → r` |
| b10 | `collect(1)`; `collect({})` into `function collect(...r)` (P11) | no edge; M2 does not count the sites |
| b05 | `function f(...[a, b])`; rest rebound by a nested callable (D11) | nothing |
| b06 | callee resolves name-only | `v → rest` labelled `nameonly/call_nameonly` |
| b07 | `f(q, { a: x })` into `function f(q, ...p)` | no edge from `x` (as main) |
| b08 | a rest Def whose value reaches a cross-file `free_single` Exact argument edge | its outgoing edges refused, as main does for new sources |
| k01 | `const g = x =>`/`use(x);` with `g(v)` (p03a); `async x =>` | `v → g:x@1` exact |
| k02 | script `a.js` `var o = { Array: t =>`/`sink(t) };`, script `b.js` `Array(h)` | no `exact` edge into `t` |
| k03 | `o.readFile = fileName =>`/`read(fileName);`, a bare `readFile(n)` elsewhere | no `exact` edge into `fileName` |
| k04 | `{ bind: binder =>`/`use(binder) }`, a bare `bind(args)` | no `exact` edge into `binder` |
| k05 | `f({ a: x })` into `const f = p => {`/`sinkA(p.a);`/`};`; bare formal rebound (D11) | `x → f:p.a@2` (Use → Use); nothing |
| k06 | module `a.js` `export function parseInt(...vals) {`/`inspect(vals);`/`}` and `export const escape = s =>`/`inspect(s);`; module `b.js` calling `parseInt(v)`, `escape(v)`, `escape({ a: v })` with no import (P3) | no edge into `vals`, `s` or `s.a` (binding admission, B and C); the sites still resolve as main (c07) |
| k07 | k06 with `import { parseInt, escape } from './a.js'`; the same in one script file | `v → vals`, `v → s` exact; one file: as k01 |
| k08 | `const id = x =>`/`{ return x; };` and `const out = id(input);` (P14); the same with a second unbound RHS read | after C, counter `return_flow_suppression_certified` 1 and `return_flow_suppression_void_unbound_uses` 0 (main: 0 and 1); second form: unchanged |
| h01 | `arr.forEach(`/`e =>`/`use(e)`/`)` (p05); `arr.map(function (e) {`/`return use(e);`/`})` (p06); `filter`, `find`, `findIndex`, `some`, `every`, `flatMap` | `main:arr@L → <anon@..>:e@..`, label `nameonly/builtin_callback` |
| h02 | `arr.reduce((acc, x) =>`/`use(x), init)`; `reduceRight` | `arr → x`; nothing into `acc` |
| h03 | `p.then(`/`r =>`/`use(r)`/`)` (p08); `p.then(ok, err)` | `p → r`; only `ok`'s first formal |
| h04 | `arr.forEach(handler)`, `function handler(e) {`/`use(e);`/`}` same file (p09) | `arr → handler:e@1`, builtin label |
| h05 | `arr.forEach(handler)` with `handler` imported, a parameter, or a `let` | no edge |
| h06 | `(e, i) =>`; no formals; `({ a }) =>` | only `e`; nothing; nothing (lane 3) |
| h07 | `function simple(items) {`/`items.forEach(v => use(v));`/`}` (member-only formal, P5); the same with `observe(items);` first (mixed formal); `o.list.forEach(…)`; `arr?.forEach(…)`; `this.items.forEach(…)`; two receivers on one line | edge from the node DESIGN 5 names (the `items.forEach` projection Use; the zero-width base Use at `items`; `o.list.forEach`; the receiver's own Use in the others; each receiver its own node on the shared line), each `reaches` from the receiver's prior Def to `use(v)`; no edge where no node exists |
| h08 | `class Box { map(f) {…} }` and `b.map(v =>`/`use(v))` | the edge, labelled `builtin_callback`, never `exact` |
| h09 | `Object.keys(o).forEach(k => …)` (p07); `arr.map(f).filter(y => …)`; `fetch(u).then(r => …)` | truth from the enumerators; stage D decides receiver forms |
| h10 | `.catch(e => …)`; `.finally(…)`; `emitter.on('x', r => …)` (p11) | no edge (not in the table) |
| h11 | h01 through both engines | `reaches` from `arr` to `use(e)`; `trace_boundary` at the edge |
| h12 | TS `const xs: number[] = …; xs.forEach = cb => { cb(99, 0, []); };` then `xs.forEach(v =>`/`use(v))` | the product edge, `builtin_callback`; `--mode builtin` grades it UNDECIDED, never CORRECT |
| h13 | `arr.forEach(handler); handler(0);` on one line, `function handler(e) {`/`use(e);`/`}` | `trace_boundary` at the builtin edge (the direct `handler(0)` site does not lend its Exact resolution) |
| h14 | a bare-formal source flowing into a named callback whose formal already reaches a cross-file `free_single` Exact argument | the source's outgoing edges as the refusal decides; refusal accounting lists the change as a reachability change with seed and path |
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
| e11 | `function holder(cb, x, y) {`/`cb(x)(y);`/`}` with `holder(provider, x, y)` (P7) | `cb(x)` resolves to `provider`; the outer call gets no target and binds nothing |
| e12 | c08's modules | `cb(input)` resolves to `real` only (the bare `setTimeout` caller is cross-file `free_single`, not admitted); no edge into `other:v` |
| e13 | `function run(f, x) {`/`[1].forEach(() => {`/`f(x);`/`});`/`}` with `run(h, v)`, `run(k, v)` (P12) | `f(x)` inside `<anon@2:15>` resolves to `h` and `k`, both kept |
| e14 | `function run(cb, x) {`/`cb(x); handler(0);`/`}` with `run(handler, value)` (P13) | `x → handler:p` `nameonly/call_nameonly`; `trace_boundary` at that edge |
| e15 | `function run({ a, b }) {`/`a(x);`/`b(x);`/`}` with `run({ a: h, b: k })` | `a(x)` → `h`, `b(x)` → `k`, never both (slot path kept) |
| e16 | an argument-follow edge that connects a guarded bare-formal source to an existing cross-file seed | as h14 |

- [ ] Write rows, README (schema, assertion kinds, census class per row) and the test. Control green on main; RED runs
      recorded. Commit; CI green with only `control` enabled.

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION section 6. Axes: argument syntax (spread position and count, spread length 0/1/many,
      holes, nested and multiple spreads, strings and tuples, comments, wrappers, TS assertions, multi-line, `new`,
      optional calls, tagged templates); call identity (chains with the same or different terminal names, call-result
      invocation, optional, constructor and member chains, two calls on one line); formals (rest, defaults before rest,
      TS `this`, overloads, bare and parenthesised arrows, async, generators); callee naming and binding admission
      (declaration, `const` arrow, object member, property shorthand and assignment, inferred name, class member,
      anonymous; module or script, imported or not, shadowed); callback forms and roles (inline, same-file named,
      imported, parameter, wrapped, bound with prefix, member; element, index, whole array, accumulator with and
      without initializer, `thisArg`, rejection handler); table methods, receiver classes (array, typed array, `Map`,
      `Set`, promise, thenable), instance overrides, user methods of the same name; holders (exported, escaping,
      reassigned by compound, destructuring or loop writes, captured writes, eval, mapped `arguments`, forwarded,
      recursive, defaulted vs explicit `undefined`, destructured `({ cb })` with keys, aliases and defaults); refusal
      bridges and seeds; return-flow consumers; same-line trace collisions; file kind; shadowing. Census classes are
      rows; each row's outcome is RED, already correct, accepted loss or unexecuted, never inferred from a group's first
      failure.

## Stage D: design of record (2 days)

- [ ] The design seat amends DESIGN section 5 for B to E against the full rows and `CENSUS.md`: whether a
      known-length literal spread's own elements bind (A, B); the rest rule for an argument of unknown offset (b03,
      b09); any refusal C keeps beyond binding admission and today's outgoing one (M3); for D the receiver forms (h09),
      the named-callback rule, roles beyond the element slot, whether bare callback formals bind (if C is no-go, an
      in-place literal callback still has its identity proof), the doubt kind's rank in `FlowConfidence::worst`; for E
      the #434 rules with anonymous targets, holder writes that make a passed value stale, slot paths, whether a
      bare-arrow target binds when C did not run (e02), the cap's counting (callable identities deduplicated before the
      cap; no target bootstrapped from a cycle); where E1's index lives so `src/call_graph.rs` does not grow (E1). The
      controller asks lane 8 for DESIGN 5's descent check and records lane 8's answer here. Inputs: DESIGN 4 and 5,
      #434 §3, the rows with truths and outcomes, `CENSUS.md`, the accepted-loss candidates (`ambiguous` rows, M3's
      UNDECIDED list). Slice A's rule (DESIGN 5) is final and not reopened. The implementer then updates expectations.
      Both reviewers; cap 2 rounds. Owner decisions 2, 5 and 6 end here.

## Stage I: slices

Order: A (unconditional; it starts once T-1 is committed and T-2's spread and call-identity axes are dry, since its
rule is final); B
and C (same loop, smallest); D (a separate pass; it builds the anonymous-formal lookup E reuses); E0, E1, E2 (the only
slices that change call resolution, last so their differential is not mixed with others). B to E run only if M-4
marked them go and the parent-gate forecast allows.

Every slice's differential runs on the same pinned inputs as its base and reports, besides its own classes:
refusal accounting (DESIGN 7) from `PRISM_E5_TRACE=1` on base and head, each restored or removed outgoing edge of a
guarded source with its reason (seed change or reachability change), seed and witness path; `call-stats` aggregate and
`--dump-sites` compared, with only the counters DESIGN 7 lists for the slice moving and each `return_flow_suppression_*`
change reconstructed from the sites whose bindings changed.

### Slice A: spread arguments by runtime offset, arguments by call identity (3 days)

**Files:** create `src/cpg/spread_args.rs`, `src/cpg/argument_nodes.rs`, `src/cpg/e5_trace.rs`,
`src/cpg/param_binding_parity_tests.rs` (all declared in `src/cpg/build.rs` with `#[path]`, the test under
`#[cfg(test)]`); modify `src/ast.rs` (`CallArgsIndex` gains `by_span`), `src/ast/call_args.rs`, `src/cpg/build.rs` (the
loop, one trace call in `refuse_new_sources_exposing_free_single_calls`), both twins,
`src/cpg/param_binding_matrix_tests.rs` (`ENABLED` gains `"A"`), `src/cpg_cache.rs`, docs.

```rust
// src/ast/call_args.rs — JS/TS/TSX: the arguments of the call node whose full span is start..end (DESIGN 5, call
// identity); `by_span` keyed (start, end, callee name), filled for the JS family only. Other languages keep
// `call_argument_texts_and_spans_at`.
impl ParsedFile { pub(crate) fn call_argument_texts_and_spans_of_call(&self, start: usize, end: usize,
    callee_name: &str) -> Vec<(String, std::ops::Range<usize>)>; }
// src/cpg/spread_args.rs — runtime offsets (DESIGN 5): slot[j] = exact offset of a non-spread argument j when every
// spread before it is an array literal with no nested spread and no holes, else None; least[j] counts unknown
// spreads as 0. Other languages: slot[j] = Some(j), least[j] = j (unchanged).
pub(super) struct Offsets { pub slot: Vec<Option<usize>>, pub least: Vec<usize> }
pub(super) fn offsets(caller: &ParsedFile, args: &[(String, std::ops::Range<usize>)]) -> Offsets;
// src/cpg/argument_nodes.rs — the full-path and base-path Use nodes of one argument (today inline in the loop)
impl CodePropertyGraph {
    pub(super) fn argument_nodes(caller_id: &FunctionId, caller_parsed: &ParsedFile, arg_text: &str,
        arg_span: &std::ops::Range<usize>, var_index: &VarIndex, exact_var_index: &ExactVarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>) -> Vec<NodeIndex>;
}
// src/cpg/e5_trace.rs — PRISM_E5_TRACE=1: one stderr line per refused source Def (file, bytes), its seed site and
// the reverse DataFlow witness path; results unchanged.
pub(super) fn report(graph: &DiGraph<CpgNode, CpgEdge>, source: NodeIndex, seed: NodeIndex, path: &[NodeIndex]);
```

- [ ] Step 1, no behaviour change: move the loop's full-path/base-path block into `argument_nodes` (twins keep their
      copies) and add `e5_trace`. Tests green; commit alone.
- [ ] Enable `A`: RED (a01, a03 to a07; a05 shows main's two wrong `exact` edges). In the three copies, for the JS
      family read the arguments with `call_argument_texts_and_spans_of_call(site.start_byte, site.end_byte, ..)`,
      compute `offsets` once per site and bind argument `j` to formal `slot[j]` when it is `Some` and in range;
      spreads bind nothing. GREEN; c03, c07, c08 and a08 unchanged.
- [ ] `spread_and_rest_graph_is_identical_across_every_route`, modelled on
      `object_literal_parity_tests::literal_bearing_graph_is_identical_across_every_route`: production, legacy twin,
      serial reference, full hit, partial hit; pinned edges for a01, a02, a05 and a06, forbidden rows included.
- [ ] Differential (X, T, SB, G): `spread-removed` (`--mode spread` per LOST row: proved-wrong and unproved listed
      apart, any `correct` is a STOP; site and row totals equal M1's), `spread-moved` (a known-length row moved to its
      correct formal, ADDED graded `--mode binding`, 0 WRONG), `chain-removed` (`--mode spread` `chain`; equals M1's
      chain rows), refusal accounting (Stage I preamble). Counters: only return-flow certification may move (DESIGN
      7). Anything else is a STOP.
- [ ] Benchmark at the head (Task M-3 command, new `--out`): traced cases do not drop in any class.
- [ ] Tier-A; cloud tests at 1 and 4 threads with RED-on-base; two reviewers. PR body: prediction, M1 beside the
      removed counts by class, the refusal-accounting list, counter changes reconstructed, benchmark table, Tier-A, the
      parent-gate re-forecast.

### Slice B: rest parameters (3 days)

**Files:** create `src/cpg/rest_params.rs` (declared in `build.rs`); modify `src/cpg/callee_params.rs` (a second memo
map in `ParamCache`; `ParamTarget::from_node`), `src/cpg/build.rs`, both twins, `src/cpg/js_param_defs_tests.rs`
(`identifier_rest_formal_gets_a_def_but_never_a_positional_argument` becomes
`identifier_rest_formal_binds_trailing_arguments`), the matrix test (`"B"`), the parity test (b02), `src/cpg_cache.rs`, docs.

```rust
// src/cpg/rest_params.rs
/// Identifier rest formal: index (formals before it, a TS `this` not counted) and Def, byte-verified.
pub(super) fn compute_rest_def_node(parsed: &ParsedFile, callee: &FunctionId, var_index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>) -> Option<(usize, NodeIndex)>;
/// Edges from the arguments whose least runtime offset reaches the rest index (DESIGN 5; stage D's rule for
/// unknown offsets) to its Def. Nothing when `site.cross_file_free_single_exact` (binding admission).
pub(super) fn collect(out: &mut Vec<PendingArgumentFlow>, site: &object_literal_args::SiteInputs<'_>,
    rest: (usize, NodeIndex), offsets: &spread_args::Offsets, label: FlowConfidence);
// src/cpg/callee_params.rs
impl ParamCache<'_> { pub(super) fn rest(&self, callee: &FunctionId, parsed: &ParsedFile, index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>) -> Option<(usize, ParamTarget)>; }
```

`collect` uses `argument_nodes` and slice A's argument list, binds a spread's inner expression where stage D admits
it, and refuses every edge at a cross-file `free_single` site. It runs after the positional loop in the three copies.

- [ ] Enable `B`: RED (b01 to b04, b06, b09; k06's rest half). Implement. GREEN; b05, b07, b08, b10, k07, c07 hold.
- [ ] Differential: `rest-added` (graded `--mode binding`, 0 WRONG among `exact`, name-only at most 5%), refusal
      accounting (Stage I preamble). Counters: only return-flow certification may move (DESIGN 7).
- [ ] Benchmark; Tier-A; cloud tests; two reviewers. PR body: M2 beside converted sites (sites with an added row, all
      rows CORRECT), oracle table, refusal-accounting list, counter changes reconstructed.

### Slice C: unparenthesised arrow parameters (2 days)

**Files:** modify `src/cpg/callee_params.rs` (drop the `bare_formal` filter; rewrite the comment at lines 155-159;
`ParamTarget` gains `pub(super) admitted_only: bool`, true for the bare formal), `src/cpg/build.rs` and both twins (skip an
`admitted_only` target when `cross_file_free_single_exact`: DESIGN 5, binding admission), `src/cpg/object_literal_args.rs`
(the same skip for literal leaves; doc lines 13-14), `src/cpg/js_param_defs_tests.rs`
(`bare_arrow_formal_stays_a_step5b_hole_but_the_parenthesised_control_binds` becomes
`bare_arrow_formal_binds_like_the_parenthesised_control`), `src/cpg/object_literal_arg_tests.rs`
(`destructured_rest_and_bare_arrow_formals_are_out_of_scope`: the bare case gains `h:x@9 -> f:p.a@2`), the matrix test
(`"C"`), `src/cpg_cache.rs`, docs.

- [ ] Enable `C`: RED (k01, k05, k06's bare half, k08). Implement. GREEN; k02 to k04, k07, c07 hold.
- [ ] Differential: `bare-added` (`--mode binding`), `bare-literal-added` (`--mode literal`), 0 WRONG among `exact`;
      refusal accounting (Stage I preamble). The ADDED set equals M3's probe set minus its cross-file `free_single`
      class on the same corpora; any difference is explained in the PR. Counters: only return-flow certification may
      move (DESIGN 7; k08 shows the mechanism).
- [ ] Benchmark; Tier-A; cloud tests; two reviewers. PR body: M3 beside converted sites, oracle table,
      refusal-accounting list, counter changes reconstructed.

### Slice D: built-in higher-order callbacks (5 to 6 days)

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
/// JS/TS/TSX only: receiver node (DESIGN 5, slice D) -> callback formal Def for a member call named in TABLE,
/// whatever the site resolves to. Label `NameOnly(BuiltinCallback)`; `cross_file_free_single_exact` false.
pub(super) fn collect(out: &mut Vec<PendingArgumentFlow>, site: &CallSite, caller_id: &FunctionId,
    caller_parsed: &ParsedFile, var_index: &VarIndex, exact_var_index: &ExactVarIndex, graph: &DiGraph<CpgNode, CpgEdge>);
// src/cpg/callee_params.rs (compute_param_def_nodes keeps its signature and result)
pub(super) fn locate_function<'a>(parsed: &'a ParsedFile, callee: &FunctionId) -> Option<tree_sitter::Node<'a>>;
pub(super) fn formal_def_nodes(parsed: &ParsedFile, function: &tree_sitter::Node<'_>, owner: &FunctionId,
    var_index: &VarIndex, graph: &DiGraph<CpgNode, CpgEdge>, admit_bare: bool) -> Option<Vec<Option<NodeIndex>>>;
```

`locate_function` finds anonymous ids with `anon_owner::node_for_id` and named ids by today's unique
line-range-and-name match. The call node is the one whose full span is `site.start_byte..site.end_byte` (DESIGN 5,
call identity); the callback is its argument 0 (`call_argument_texts_and_spans_of_call`): an inline arrow or function
expression gives its owner through `anon_owner::identity`; an identifier gives a same-file callable when
`js_ts_site_binding` returns `JsBinding::Callable`. The receiver node follows DESIGN 5's order: a nonzero Use of the
receiver path in the member object's span (`argument_nodes`); else the projection Use whose path is the receiver path
plus the method name and whose bytes are the callee member expression's (exact index); else a zero-width Use of the
receiver base at the member object's start byte (legacy line index, filtered by that byte); else no edge (M4's
`receiver-absent` class; the differential reports it from the dumps, no product counter). Stage D fixes further receiver forms and `admit_bare`. `collect` runs per site before
the resolved-target loop.

- [ ] Step 1, no behaviour change: split `compute_param_def_nodes` into `locate_function` and `formal_def_nodes`; commit.
- [ ] Enable `D`: RED (h01 to h04, h07, h11, h12). Implement the doubt kind (serialisation, `dfg-stats` spelling
      `builtin_callback`, stats field, a `FlowConfidence::worst` test against `Exact` and every existing doubt in both
      operand orders), then `collect`. GREEN; h05, h06, h08, h10, h14 hold. Add the `builtin_callback` arm to the evidence
      copy of `byte_dump.rs`.
- [ ] Differential: `builtin-added` (`--mode builtin`; WRONG rate at most 5%, coverage by method and receiver class
      published, only CORRECT rows credited); refusal accounting (Stage I preamble); the new label counter equals the
      added count; counters otherwise as DESIGN 7.
- [ ] Merge only when h13 holds (lane 8's descent check, DESIGN 5); until then the PR waits, and the controller
      escalates if lane 8 declines.
- [ ] Benchmark; Tier-A; cloud tests; two reviewers. PR body: M4 beside converted sites, declarations of added edges by
      class, `receiver-absent` count, refusal-accounting list.

### Slice E0: move-only split of the three oversized files (2 days; only if E is go)

- [ ] One move-only PR, three moves, each its own commit with a byte-identity check of the moved text:
      `compute_indirect_call_sites` (`src/call_graph.rs:2392`), `recompute_indirect_calls`, `clear_indirect_calls`,
      `apply_indirect_call_sites` and the private helpers only they call into the child module
      `src/call_graph/indirect.rs` (a child module sees `CallGraph`'s private fields); `ResolutionKind`, `DropReason`
      and their `impl`s from `src/resolution.rs` into `src/resolution/kinds.rs`, re-exported by `pub use` so every path
      stays (the `include_str!("../resolution.rs")` in `src/call_graph/anon_barrier_tests.rs` follows the enum, the only
      non-move edit); `call_stats` (`src/navigation/queries.rs:278-833`) into `src/navigation/call_stats.rs`, re-exported
      from `queries`. Targeted `call_graph`, `resolution` and `navigation` tests; one reviewer reading a pure move; sizes
      of the three files and the new modules before and after in the PR.

### Slice E1: argument-follow facts and counters, no resolution change (3 days)

**Files:** create `src/resolution/js_ts/arg_follow.rs`, `src/resolution/js_ts/arg_follow/value.rs`,
`src/resolution/js_ts/arg_follow_tests.rs` (`#[cfg(test)]` in `arg_follow.rs`), `src/ast/js_ts/param_slot.rs` (`mod
param_slot;` in `src/ast/js_ts.rs`); modify `src/resolution/js_ts.rs` (`pub(crate) mod arg_follow;`),
`src/call_graph/indirect.rs` (the index's reset in `clear_indirect_calls`, one call in `recompute_indirect_calls`), the
index's holder as stage D placed it, `src/navigation/call_stats.rs` (key `js_arg_follow` from `arg_follow::stats`), both
cache files, docs. `src/call_graph.rs`, `src/resolution.rs` and `src/navigation/queries.rs` do not grow in this PR.

```rust
// src/resolution/js_ts/arg_follow.rs (all derive Clone, Debug, PartialEq, Eq, Serialize, Deserialize)
pub(crate) struct ArgIndex { pub sites: BTreeMap<(String, usize, usize), ParamSlot>,  // (file, call start, call end)
                             pub outcomes: BTreeMap<ParamSlot, ArgOutcome> }        // + Default
pub(crate) struct ParamSlot { pub holder: FunctionId, pub position: usize, pub path: Option<String> } // + Ord
pub(crate) enum ArgOutcome { Targets(BTreeSet<FunctionId>), Fanout(usize), Empty(String) }
/// #434 §3 rules 2-7 and §4, with stage D's write and staleness rules. Computed while the index is empty, so
/// ArgumentFollow outcomes (never Exact) cannot enter the Exact-caller sets; callers admitted by DESIGN 5 (no
/// cross-file `free_single`); forwarding to depth 2; callable identities deduplicated, cap 8 targets.
pub(crate) fn compute(cg: &CallGraph, files: &BTreeMap<String, ParsedFile>) -> ArgIndex;
pub(crate) fn stats(cg: &CallGraph) -> BTreeMap<String, usize>;
// src/ast/js_ts/param_slot.rs — for the callee identifier of the call whose full span is `start..end`: run
// `js_ts_binding_walk` from that occurrence; when it answers `Walk::Found` with a declaration that is a formal (or a
// destructured property of one) of the callable starting at byte `holder_start`, return that holder start, the
// formal's runtime slot (a TS `this` takes none) and the destructuring key path; None for any other binding.
impl ParsedFile { pub(crate) fn js_ts_param_slot_of_call(&self, start: usize, end: usize, callee: &str)
    -> Option<(usize, usize, Option<String>)>; }
```

The facts are read from the parsed files when the index is built, not stored as the #434 plan's new `CallSite` fields
(`js_param_slot`, `js_args`, which would grow `src/call_graph.rs`): the slot from `js_ts_param_slot_of_call` (the
binder's declaration, not the walk's `Pos`), values through `js_ts_follow_alias`, inline callables through
`anon_owner::identity` (`value.rs`). The index is persisted with the call graph (a full hit restores it) and recomputed
on cold and partial builds.

- [ ] Unit tests first: one per value shape of #434 §3 rule 4; rule 2 (written parameter) and stage D's write forms;
      the cap after deduplication; forwarding depth; a cycle with no grounded caller gives no target; a TS `this`
      parameter; `({ a, b })` gives two slots by path (e15); `cb(x)(y)` keys two calls (e11); a cross-file `free_single`
      caller contributes nothing (e12); a JSX attribute after a spread; `index_partial_hit_equals_cold`,
      `index_full_hit_equals_cold`. RED against a skeleton.
- [ ] Implement. Gate: `call-stats --dump-sites` byte-identical on X and T apart from the new key; DataFlow edges
      identical; group E rows unchanged; cache size on X and T before and after in the PR; sizes of the three
      oversized files unchanged. Tier-A; cloud tests; two reviewers.

### Slice E2: the argument-follow route (5 days)

**Files:** modify `src/resolution/free.rs` (in `admit_and_filter_family`, between `admit_anonymous_exact` and
`retain_family`: `let outcome = self.js_ts_argument_follow(site, outcome);`, so anonymous-caller admission does not
clear ArgumentFollow targets and the family filter and multi-occurrence refusal still apply; DESIGN 5),
`arg_follow.rs` (that method), `src/resolution/kinds.rs` (`ResolutionKind::ArgumentFollow`, `as_str`
"argument_follow"; `DropReason::ArgFanout`), `src/navigation/call_stats.rs` (the `ArgFanout` arm), other exhaustive
matches the compiler reports (`rg -n 'ResolutionKind::ParameterCallback' src crates`; any arm needed in an oversized
file is offset in the same PR or placed in a module), `src/cpg/callee_params.rs` (`compute_param_def_nodes` locates
anonymous callees with `locate_function`, created here if D did not run), `tests/integration/js_alias_rule_test.rs` (no
`ArgumentFollow` outcome has `via_alias_route`, #434 §4), the matrix test (`"E"`), both cache files, docs.

`js_ts_argument_follow` changes only a JS-family site with no target, drop reason `UnknownName` or
`LocalBindingUnproven`, and a slot in the index under its full span: `Targets` gives `NameOnly` targets of kind
`ArgumentFollow`; `Fanout` gives `dropped(DropReason::ArgFanout)`; `Empty` keeps the drop. Argument binding then follows
in the loop (edges `nameonly/call_nameonly`), through slice A's call-identity argument lookup.

- [ ] Enable `E`: RED (e01 to e04, e07, e08, e11 to e13, e15). Implement. GREEN; e05, e06, e09, e16, c08 hold.
- [ ] `arg_follow_tests::activated_route_parity`: cold, full hit and partial hit after a caller edit, a caller deletion
      and a fanout change from 8 to 9 targets give identical labelled DataFlow multisets and site kinds.
- [ ] Differential: `argfollow-call` (targets graded `--mode argfollow`, the independent checker ledger; WRONG rate at
      most 5%), `argfollow-flow` (use→def into the targets' formals, `--mode binding`); refusal accounting (Stage I
      preamble); expected counter moves (`resolved` up and the two drop reasons down by the same sites, `ArgFanout` drops
      as drop-reason moves, `return_flow_skipped_nameonly`, return-flow certification as DESIGN 7); 0 changes at sites
      resolved on main.
- [ ] Merge only when e14 holds (lane 8's descent check, DESIGN 5).
- [ ] Tier-A `--quick` regressions pasted; benchmark; cloud tests; two reviewers. PR body: M5 beside converted sites,
      sizes of the three oversized files before and after.

## Close-out (2 days)

- [ ] `REPORT.md` (evidence directory; summary appended to `CENSUS.md`): per slice the prediction, added and removed
      edges, oracle tables, benchmark per class, the parent gate. Comments: #595 (B, C; close when both merged or no-go),
      #434 (E; close or park with the census), #604 (items 1 and 4, the parent gate's numbers).
- [ ] Hand-over: one comment on #600, referenced from lane 12's follow-up, listing the cases first-broken at
      `callback_argument_parameter_registration` on the current build with outcomes; this lane builds nothing for them.
- [ ] Docs (in each slice PR, checked here): `CLAUDE.md` (JS/TS parameter Defs paragraph, `argument_follow` ladder note,
      `dfg-stats` doubt vocabulary); known-gaps section 1.1 row "A parameter" and section 6 line "Missing: rest formals…".
- [ ] Defects on main, filed with the review's probes as reproductions (fixture sources inlined) and their pinning rows:
      an ES module export named like a global is the cross-file `free_single` Exact target of an unimported bare call,
      giving wrong `exact` argument edges and wrong callers (P3, P6, P10; rows c07, c08; not fixed here: call resolution);
      the argument index keyed by (start byte, callee name) gives a chained inner call the outer call's arguments (P8,
      P15; row a05; fixed for the JS family by slice A, other languages not verified).
- [ ] Follow-up issues with census numbers: `arguments` binding (owner decision 3); `.call`/`.apply`/`.bind`; promise
      chains; D's refused receiver forms and roles outside the element slot; shared-script globals lost to binding
      admission; Python `f(*xs, v)` misbinding (not verified); trace-engine crossing of name-only edges beyond DESIGN 5's
      check (lane 8). Handoff refreshed; raw dumps offloaded; `target/` removed.

## Acceptance for the lane

Slice A merged, every removed edge graded proved-wrong, unproved or chain-misbound and none correct. Every other slice
merged with DESIGN section 7 met (no WRONG among added `exact` edges; name-only WRONG rate reported and at most 5%; only its
own change classes; only its listed counters move; traced benchmark cases not lower in any class), or recorded no-go with
its census number. The parent gate on the merged head (DESIGN 6): at least 30 cases traced at the head and not at the base on
the same inputs, losses listed, or 500 sites in the union of stable site ids with at least one added row and every added row
checker-CORRECT. A failed gate is reported with its numbers and the lane stops for the owner.

## Estimates (working days, review rounds included)

Stage M 5; stages T and E 4 (2 if only A runs); stage D 2 (none if only A runs); slice A 3; B 3; C 2; D 5 to 6;
E0 2, E1 3, E2 5; close-out 2. Total: 12 if only A runs, 36 to 37 if every slice runs.
