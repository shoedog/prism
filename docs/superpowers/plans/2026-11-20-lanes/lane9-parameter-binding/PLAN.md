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

**Spec.** [DESIGN §5](DESIGN.md) fixes the rules after Stage M. [RULES-INPUT-NOTES.md](RULES-INPUT-NOTES.md) assigns each of the 425 fixture IDs a rule and a required disposition. Issue #434 §3–5 supplies the original E algorithm. This plan uses the Stage D storage, value and stability rules. Quote both the slice brief and its design subsection when dispatching an implementation.

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
  algorithm in `refuse_new_sources_exposing_free_single_calls` (slice A adds a stderr trace only, and the ledger records changed results when reachability moves), call resolution
  outside E2 (the unimported-export defect is filed, DESIGN 2), `src/cpg/trace.rs` and lane 8's descent gate (lane 8 owns the accepted I-guard check in DESIGN 5), `src/reasoning/scope_honesty.rs` (its "JS/TS callback" warning
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
  comment, one expectation and bare-target admission there, and rebases over lane 3. Its existing preceding-spread and Exact-singleton gates govern literal leaves. Record the local losses s1-1/s5-3 and s5-28. A change to that shared contract needs owner agreement.

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

Stage M is complete. The tasks below preserve the measurement definitions that produced the baseline. All five slices passed. The actuals and eligible denominators are in [DESIGN §§5–6](DESIGN.md). The original M4 role sample measured the element slot. Slice D also grades first-accumulator seeding and an element collected by rest on its own head.

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
      none. Prove the rest destination from the argument or dense element’s least offset. An operand earns CORRECT only with an independent positive yield witness for this iteration. Grade a witnessed zero-yield transfer WRONG and an unwitnessed yield UNDECIDED. Compare a selected `import_qualified` target with the actual value export by declaration identity. A witnessed private decoy grades WRONG. Include empty/nonempty tuple, array and string controls, erased wrappers and an alias mutated before spread. Accept matched member-span operands. Otherwise an unknown offset is UNDECIDED).
      `literal` (Use → Use: a literal leaf to a read of the formal's member path, CORRECT when the key path is a prefix
      or extension, WRONG for a sibling field);
      `spread` (a LOST row: `proved-wrong` when its offset is exact and differs, `unproved` when unknown, `correct` when
      exact and equal; `chain` when the row's Use lies outside the resolved call's span);
      `builtin` (a second program with the default lib: CORRECT only when `getResolvedSignature` of the call gives a
      declaration whose source file is a TypeScript library file (`lib.*.d.ts`) and the fed formal has the element role
      there. On the D head also check the no-initializer seed, the rest Def covering a delivered slot and the independent delivery evidence. A witnessed contrary delivery grades WRONG, including an installed method whose static signature still comes from the library. For h12a/b count one WRONG emitted receiver transfer, zero CORRECT and zero UNDECIDED. For x3-9 count the emitted slot-0 transfer WRONG and record the missed slot-1 delivery separately. An opaque override with no delivery witness grades UNDECIDED. Require positive delivery evidence for CORRECT);
      `argfollow` (call target: each holder caller resolved by the checker, independently of the census: CORRECT when
      the checker resolves the caller to the holder, the passed value to the target's declaration and the slot or key
      path matches; WRONG when it resolves the caller or the value elsewhere; UNDECIDED without a witness. The DFG
      rows into its formals require the independent target join in DESIGN §7 before `binding` can grade the payload offset). The census's product target set is kept as a
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

The T-1 table below uses parent IDs. Split cases and enumerated IDs use the [complete design ledger](RULES-INPUT-NOTES.md). Use the reviewed matrix at `5767ceb8` with the separately pinned original RED record. Thirteen model-boundary skips and the narrowed x1-6/x3-5 checks postdate that receipt. The implementer applies its seven runtime corrections before enabling feature groups and keeps external checks in their own harnesses.

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
| b02 | `f(a, v, w)` into `function f(p, ...rest)`; `f(...xs)` into `f(...rest)`; `f(a, ...xs)` | Keep the three scalar edges in b02a. Withhold the opaque `xs → rest` in b02b/c. Keep `a → p` in b02c |
| b03 | `f(...xs, v)` into `f(p, ...rest)` | no edge from xs or v into p or rest. The runtime position of v may be zero |
| b04 | TS `function f(this: T, a, ...rest)` with `f(x, y)` | `x → a`, `y → rest` |
| b09 | `g(...xs, v, w)` into `function g(p, ...r)` (P11); `g(...[], v)`; `g(...[0], v)` | first call only `w → r`, with no edge into p and no v → r. Empty spread gives `v → p`. Singleton gives `v → r` |
| b10 | `collect(1)`; `collect({})` into `function collect(...r)` (P11) | no edge; M2 does not count the sites |
| b05 | `function f(...[a, b])`; rest rebound by a nested callable (D11) | nothing |
| b06 | same-file callee resolves NameOnly | keep the resolution and require no `v → rest` edge |
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
| h09 | `Object.keys(o).forEach(k => …)` (p07); `arr.map(f).filter(y => …)`; `fetch(u).then(r => …)` | enable all three negatives. No D edge into the outer callback from a call result or from any of its constituents |
| h10 | `.catch(e => …)`; `.finally(…)`; `emitter.on('x', r => …)` (p11) | no edge (not in the table) |
| h11 | h01 through both engines | `reaches` from `arr` to `use(e)`; `trace_boundary` at the edge |
| h12 | TS `const xs: number[] = [0]; xs.forEach = cb => { cb(99, 0, []); };` then `xs.forEach(v =>`/`use(v))` | the product edge, `builtin_callback`; `--mode builtin` grades it WRONG. A companion with an opaque override grades UNDECIDED |
| h13 | `arr.forEach(handler); handler(0);` on one line, `function handler(e) {`/`use(e);`/`}` | `trace_boundary` at the builtin edge (the direct `handler(0)` site does not lend its Exact resolution) |
| h14 | a bare-formal source flowing into a named callback whose formal already reaches a cross-file `free_single` Exact argument | the source's outgoing edges as the refusal decides; refusal accounting lists the change as a reachability change with seed and path |
| e01 | `function run(cb) {`/`const input = read();`/`cb(input);`/`}`, `run(handler)`, `function handler(e) {`/`use(e);`/`}` (p10b) | `cb(input)` resolves to `handler`, `nameonly`, kind `argument_follow`; `run:input → handler:e@1` labelled `nameonly/call_nameonly` |
| e02 | `run(w =>`/`use(w))` (p10a); `run((w) =>`/`use(w))` | both resolve to their byte-identified anonymous callable and bind the payload, including the bare arrow with ordinary C disabled |
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

- [x] The design seat recorded rules for B–E, costs, all 425 matrix dispositions, all 73 M3 UNDECIDED records and the seven runtime disputes in DESIGN §§4–9 and RULES-INPUT-NOTES. The owner defaults for decisions 2, 5 and 6 are recorded in DESIGN §8.
- [x] Lane 8's published answer accepts the edge-local descent check in its I-guard slice after I-1. By doubt kind h13 corresponds to n02 and e14 to n01. Each product merge still needs the landed guard and its own integrated test.
- [ ] At the two-round cap the owner classified round 2 as converging and authorized one further controller review. Review the targeted amendments in DESIGN’s Round 2 fold. Keep accepted losses, hard sentinels and outstanding measurements visible.
- [ ] The test-only matrix PR below applies the full Loss/Set manifest, including b03, b09a, e02a, h09a–c and each disputed row. Re-run each changed assertion on the pinned base and identify the applicable slice or external probe. Preserve the original outcome receipt and record its replacement with the corrected assertion. A runtime witness, a missing endpoint and a known product approximation receive separate dispositions. Do not skip a mixed row's supported assertions.

## Stage I: slices

A is the unconditional correctness slice. Stage T-2 reached all three five-round caps. DESIGN §5 and the row ledger supply the open-class disposition for review. Slice A is complete and verified per the integration handoff. Preserve the frozen rule and the three mandatory RED sentinels in DESIGN §9. Run B and C next with the matrix harness prerequisite available, D after them, then the E0 move, E1 facts and E2 activation. Stage M gave each slice GO. Re-forecast parent credit after A and after B/C as before.

Every slice's differential runs on the same pinned inputs as its base and reports, besides its own classes:
refusal accounting (DESIGN 7) from `PRISM_E5_TRACE=1` on base and head, each restored or removed outgoing edge of a
guarded source with its reason (seed change or reachability change), seed and witness path; `call-stats` aggregate and
`--dump-sites` compared, with only the counters DESIGN 7 lists for the slice moving and each `return_flow_suppression_*`
change reconstructed from the sites whose bindings changed. Reconcile every counted DFG label to signed before/after edge multiplicities, including refusal changes and restored edges. A delta of one without a matching witness stops the slice.

### Test-only PR before D — per-assertion matrix staging and losses

Land the small harness PR before D, early enough for A/B/C fixture reconciliation. Limit it to the matrix harness, expectations and its tests. No production graph or resolver change. Consume the 425-row source digest and the ordinal/stage tables in RULES-INPUT-NOTES.

- [ ] Add two optional fields on each existing assertion object. `slice` is a validated slice ID (control, A, B, C, D, E2) overriding the row's default; map the legacy row group E to E2. `loss` is a nonempty reason identifying the accepted mismatch in the ledger. Absence of both means the original assertion and stage. Keep `deny_unknown_fields` and existing row-level `unexecuted`.
- [ ] Validate every record, enabled or disabled. Select individual assertions across **all rows** by active slices; a row's group cannot hide an earlier assertion. Keep per-ID, language, ordinal, slice, expected/actual outcome and loss reason. Run every selected sibling even after a failure. An expected semantic failure is reported as a loss, a matching assertion marked loss is an unexpected pass and fails the suite. Setup/schema/endpoint errors and panics fail independently. An ordinary unmarked mismatch stays RED. Skipped or inactive assertions earn no pass or conversion credit.
- [ ] Generate and reconcile the manifest from **every Loss and Set row** against the actual assertion objects, include the ten group-A dispositions, retain mandatory x1-4/x2-2/x5-2, and retain the historical review skips. Exercise expected failure, unexpected pass, ordinary RED, invalid metadata, setup error and untouched positive/negative siblings. Test B alone selecting k06a #2 and k07a/b #1, then B+C adding k06a #1 and k07a/b #2. Test mixed rows under D alone and D+E2 (s3-14, s4-12, s4-13, x2-11, x4-9 and amended s2-15). Assert the exact selected ordinals in each phase and complete integrated coverage. Do not infer activation from product output. Record the base run with the corrected expectations before attributing any feature result. In the PR report count expected losses, hard RED sentinels and blocked-prerequisite assertions separately. For constructors report the integration-base site/target pin per row, ordinal and language, plus the deduplicated ordinal and row counts in the ledger. Show fixed-slot loss overlaps and the x5-3 skip separately. A blocked required assertion stays RED. Run the supplied-site binding tests and independent negatives and report their own results.

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
      A emits no fixed binding from a spread operand or its individual elements. B alone adds the trailing rest-element case.
      Re-run the A ledger, including t3-1 and the three mandatory RED A checks in DESIGN §9. Keep the reviewed model-boundary skips and the post-spread literal-leaf losses s1-1/s5-3. Preserve the base findings if iterator overrides, exceptional control flow or an upstream resolver contradict the positional rule. Do not clear a WRONG Exact by changing the expected target. Keep c03, c07, c08 and a08 as controls.
- [ ] `spread_and_rest_graph_is_identical_across_every_route`, modelled on
      `object_literal_parity_tests::literal_bearing_graph_is_identical_across_every_route`: production, legacy twin,
      serial reference, full hit, partial hit; pinned edges for a01, a02, a05 and a06, forbidden rows included.
- [ ] Differential (X, T, SB, G): `spread-removed` (`--mode spread` per LOST row: proved-wrong and unproved listed
      apart, any `correct` is a STOP; site and row totals equal M1's), `spread-moved` (a known-length row moved to its
      correct formal, ADDED graded `--mode binding`, 0 WRONG), `chain-removed` (`--mode spread` `chain`; equals M1's
      chain rows), refusal accounting (Stage I preamble). Reconcile counted label deltas and return-flow certification under DESIGN §7. Anything else is a STOP.
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
/// Bind scalars and dense elements by least offset. An Exact opaque operand also needs a positive yield witness.
/// The receiver is the identifier rest Def. Apply the site's lexical admission before emitting each edge.
pub(super) fn collect(out: &mut Vec<PendingArgumentFlow>, site: &object_literal_args::SiteInputs<'_>,
    rest: (usize, NodeIndex), offsets: &spread_args::Offsets, label: FlowConfidence);
// src/cpg/callee_params.rs
impl ParamCache<'_> { pub(super) fn rest(&self, callee: &FunctionId, parsed: &ParsedFile, index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>) -> Option<(usize, ParamTarget)>; }
```

Read the call's own argument list. Count TS `this` as zero and each actual fixed parameter as one, including defaults and unsupported fixed patterns. Find the actual implementation's identifier rest Def. Keep captured-write protection and occurrence multiplicity. Bind an ordinary Use with least ≥ rest, an opaque path spread only if its own start has reached rest, and a dense literal element Use if least(start) + element index has reached rest. Require exact_singleton for every new Exact B edge. On `import_qualified`, also require the §5 export-declaration witness. For an otherwise Exact opaque operand, withhold the edge unless this iteration is proved to yield an element. Refuse a proved-empty operand at any confidence. Exercise zero/one-yield generators, an empty Set, array iterator and slice, and a stable proved-nonempty operand as specified in §5 B. Keep scalar sibling and dense element checks independent. Use the ordinary complete-path/base extraction. No array identity, length transfer or rest-object field flattening.

- [ ] For each otherwise eligible ordinary B NameOnly rest target, compare the call-site and declaration files. Emit the existing `nameonly/call_nameonly` edge only when they differ. Apply the comparison at rest-edge emission so the resolver, fixed-slot binder and D/E producers keep their own admission rules. In b06 keep the resolution and check the edge is absent. In s5-5 keep the Exact prohibition and check the missing edge and its dependent consumer results. Add a foreign positive for the labelled rest edge, reachability and trace boundary. Use the [assertion-level dispositions](RULES-INPUT-NOTES.md#ordinary-b-nameonly-rest-edges) when folding the fixtures and manifest.
- [ ] **Activation gate.** Perform the complete B replay in DESIGN §7, including the per-producer yield dispositions for every opaque operand at spread-only and mixed sites, straddles and dense terminal elements. Enumerate NameOnly sites outside M2’s original sole-Exact filter. Report withheld and emitted C/W/U per corpus. Reconcile the edited head against the priced NameOnly losses of X/T/SB 4/29/36 rows and the 109/18/200 retained. Pool W/(C+W+U) across emitted NameOnly additions only. Report W/(C+W) alongside it and count only C for credit. Include singleton and multi-Exact controls on local_def and import_qualified. Price the private same-name rest decoy and bare-arrow decoy classes against actual exported singletons on the pinned M2/M3 site inventories. Stop on one unwitnessed Exact declaration or operand yield, a missing stratum, any WRONG Exact or pooled W/(C+W+U) > 5% for B NameOnly. Price any further change to this predicate.
- [ ] Check the complete mixed fixed/rest inventory in the ledger, including the two independent fixed-slot losses s1-7 and s2-4. Apply the constructor table in the ledger to t4-3, s2-4, s3-3, s5-6, both new expressions in x3-5, and x5-3’s skipped ordinary control. For each active missing route report blocked/RED, exercise its binding with the pinned site/target supplied, and check the independent negatives. E2 applies the same disposition to s1-25. Every supplied-site test supplies the call's argument nodes and byte spans with the site and its Exact target (ledger "Supplied-site seam": the argument index records none for a `new` expression).
- [ ] Enable B after rerunning the affected rows on base. b03 forbids both destinations for v. b09a binds only w to rest. Test dense trailing elements s5-7/x1-5, opaque straddles s4-4, multiple spreads s1-6/s3-4, TS implementation slots, repeated Uses, name-only and refusal reachability. Record x2-4's reviewed dead-call loss. Run both remaining x1-6 positives. Recheck x3-5’s two real-site positives under the constructor prerequisite. Preserve the reasons their historical negative walk assertions were removed. M2 counted 482 admitted sites, 53 spread-operand-only, one excluded cross-file site and 991 without an eligible Use. Record marginal dense-element yield on the head.

- [ ] Differential: `rest-added` (graded `--mode binding`, 0 WRONG among `exact`, name-only at most 5%), refusal
      accounting (Stage I preamble). Reconcile signed label and return-flow deltas (DESIGN §7).
- [ ] Benchmark; Tier-A; cloud tests; two reviewers. PR body: M2 beside converted sites (sites with an added row, all
      rows CORRECT), oracle table, refusal-accounting list, counter changes reconstructed.

### Slice C: unparenthesised arrow parameters (2 days)

**Files.** Modify `src/cpg/callee_params.rs`, `src/cpg/build.rs`, both twins, `src/cpg/object_literal_args.rs`, the bare-formal, literal-leaf and matrix tests, `src/cpg_cache.rs` and docs. Factor `locate_function` and `formal_def_nodes` here for the anonymous/default-export/IIFE cases. Verify named owners by unique identity and anonymous owners through `anon_owner::node_for_id`. Other languages keep their existing slot lookup.

- [ ] Enable C with RED on the ordinary direct and literal-leaf rows in the ledger. Replace the D12 `bare_formal` filter with byte-checked eligibility. Mark a bare `ParamTarget` and check the site on each newly enabled edge. Reject `cross_file_free_single_exact`. Require exact_singleton on the complete resolution result for each newly enabled Exact edge. For `import_qualified`, independently identify the actual exported declaration as specified in §5. Pin the renamed live export with both private decoys and a genuine export positive. Test ordinary and literal-leaf admission. For an **ordinary bare target only**, also reject NameOnly when caller and target files differ. Keep same-file NameOnly, admitted imports/locals and their original label, existing inferred-name and outgoing source rules, and lane 3's Exact-singleton gate on leaves. Test ordinary same-file NameOnly, foreign NameOnly, admitted foreign Exact, foreign FreeSingle, and an unchanged parenthesised control. D/E with their own identity proofs bypass the ordinary NameOnly predicate.
- [ ] Update `bare_arrow_formal_stays_a_step5b_hole_but_the_parenthesised_control_binds` and the bare case in `destructured_rest_and_bare_arrow_formals_are_out_of_scope`. Test k01–k08, ordinary anonymous lookup, conflicting imports and qualified receivers, rebinding, and both directions of the k08 return-flow change. Preserve byte and Def identity in the twins and cache paths.
- [ ] Pin every positive in the C route inventory before testing its formal locator. Preserve runtime-truth loss assertions on s3-7, s5-11 and both x2-6 IIFEs, plus the nonunique site checks in s5-9 and x3-7. Test singleton vs multiple results on each W1 rung. Keep the locator test for an independently identified anonymous owner.
- [ ] Differential `bare-added` and `bare-literal-added`. Compare against the 195-row lift minus 14 FreeSingle Exact and 50 foreign NameOnly (6 WRONG, 44 UNDECIDED, zero CORRECT) and three multi-Exact rows (T#5 local_def, T#58/59 import_qualified, all UNDECIDED). Before the export-witness check the reference is 128 retained, 116 CORRECT and 12 UNDECIDED. T#39 and T#54 are the two retained T Exact. Price the declaration check separately against these keys and the complete singleton fallback inventory. Rejoin actual site kinds, report differences by key, apply zero WRONG Exact and at most 5% WRONG NameOnly. List no-credit rows and new/restored outgoing witnesses. The probe predicted 104 all-CORRECT SB sites before the head differential.
- [ ] Benchmark, Tier-A, cloud tests and both reviewers. Reconstruct allowed return-flow counters and re-forecast parent credit after B/C.

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
/// Use TABLE at the public attached member call, plus slot 0 for a reducer with a proved omitted initializer.
/// Bind a read identifier Def or the identifier rest Def covering the role. Label NameOnly(BuiltinCallback).
pub(super) fn collect(out: &mut Vec<PendingArgumentFlow>, site: &CallSite, caller_id: &FunctionId,
    caller_parsed: &ParsedFile, var_index: &VarIndex, exact_var_index: &ExactVarIndex, graph: &DiGraph<CpgNode, CpgEdge>);
// src/cpg/callee_params.rs (compute_param_def_nodes keeps its signature and result)
pub(super) fn locate_function<'a>(parsed: &'a ParsedFile, callee: &FunctionId) -> Option<tree_sitter::Node<'a>>;
pub(super) fn formal_def_nodes(parsed: &ParsedFile, function: &tree_sitter::Node<'_>, owner: &FunctionId,
    var_index: &VarIndex, graph: &DiGraph<CpgNode, CpgEdge>, admit_bare: bool) -> Option<Vec<Option<NodeIndex>>>;
```

Use C's formal locator, or factor it here if C was absent. The collector runs before the resolved-target loop. Match a static public dot/quoted-bracket method on an ordinary or optional attached member call by full site identity. Use a path-bearing identifier/member/`this` receiver. Match in order the complete path Use in the receiver span, the exact callee projection Use, then the zero-width base Use at the receiver start in this owner. No call result, inner producer argument, condition or index supplies an outer receiver. Assert the three h09 negatives at real callback Defs. Preserve h07 paths, occurrences and prior-Def reachability.

Read runtime slot zero, including transparent wrappers, a proved empty spread and an inline function inside a dense spread. An inline function has its own owner identity. A named candidate needs the occurrence-specific JsBinding::Callable, declaration-kind and initialization checks in DESIGN §5. Exercise stale writes and sloppy Annex B against stable and strict controls. Independently witness the live value in the grader. Reject a mutable or stale declaration, imported terminal/alias, formal, member, bound or factory value. Bind a bare Def under this independent proof even when C is disabled. Apply DESIGN §5's slot-zero table, slot-one reducers and first-accumulator seed only with a proved omitted initializer. A read identifier rest Def covering a fed role gets one edge. Leave index/key, whole array, `thisArg`, explicit init and rejection to their recorded losses.

- [ ] Apply the D activation gate in DESIGN §7 on M4 keys. Price all identity and role subtypes before enabling D. Replay h12a/b’s constant-delivering override, x3-9’s installed reducer and an opaque override on the M4 grading path. Include each emitted contrary transfer in W/(C+W+U). Use the test-only PR’s D assertion selection and corrected byte fixtures. Pin t3-3, s1-17 and the five accepted runtime/model discrepancies in DESIGN §5. Run D and E2 halves together on the integrated head. At D alone use the D ordinals, including those in group-E rows. Implement badness `(6, 0)` above `CallNameOnly` and preserve the order of all older kinds. Test `worst` in both orders, idempotence and associativity. Use the product doubt spelling in matrix/byte dumps and the new stats field. Replay h13, x3-8 and s4-9 against the recorded base byte dump; retain s4-9’s missing second receiver as a loss. Add the indented companion in the ledger and pin its missing D edge as a loss.

- [ ] Differential: `builtin-added` (`--mode builtin`; WRONG rate at most 5%, coverage by method and receiver class
      published, only CORRECT rows credited); refusal accounting (Stage I preamble); reconcile the signed builtin label count and every affected existing label (including h14a) under DESIGN §7.
- [ ] Merge only after lane 8 I-guard lands. Exercise repaired h13 and the represented x3-8 companion (lane 8 n02). Compare the two-space-indented companion, whose missing receiver Use precludes that D edge while the direct Exact argument still descends. The column-1 x3-8 test must identify BoundaryExited/not_exact on the builtin edge and reach the same callback body from the independent direct variable root across its own Exact edge. A missing endpoint or merely stopped walk fails the gate.
- [ ] Benchmark; Tier-A; cloud tests; two reviewers. PR body: M4 beside converted sites, declarations of added edges by
      class, receiver-absent count (10,114 at M4), 1,707 callbacks outside admission, 1,142 missing read Defs, 409 bare and 88 same-file named candidates. Rejoin the 88 named site keys to count any exclusions under the live-binding proof. Report the new role counts alongside the 3,910 original candidates and all accepted runtime mismatches.

### Slice E0: move-only split of the three oversized files (2 days)

M5 passed. Keep ArgIndex and storage changes in E1. This PR only establishes the child module where its lifecycle hook will live.

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

**Files.** Create `src/resolution/js_ts/arg_follow.rs`, its value module and tests, and `src/ast/js_ts/param_slot.rs`. Modify their module declarations, `src/resolution/js_ts/anonymous.rs` (the existing `CallGraph.js_anonymous` storage object), E0's `src/call_graph/indirect.rs`, `src/navigation/call_stats.rs`, both cache versions and the relevant tests/docs. The weighted size of `src/call_graph.rs` must not grow in E1.

```rust
// Types in src/resolution/js_ts/arg_follow.rs. Give keys a deterministic Ord.
pub(crate) enum PathStep { ObjectKey(String), ArrayIndex(usize) }
pub(crate) struct ParamSlot {
    pub holder: FunctionId,
    pub position: usize,
    pub path: Vec<PathStep>,
    pub declaration: (usize, usize), // distinguishes defaults of distinct bindings at one path
}
pub(crate) struct ArgIndex {
    pub sites: BTreeMap<(String, usize, usize, String), ParamSlot>, // file, call start/end, callee
    pub outcomes: BTreeMap<ParamSlot, ArgOutcome>,
}
pub(crate) enum ArgOutcome { Targets(BTreeSet<FunctionId>), Fanout(usize), Empty(String) }
pub(crate) fn compute(cg: &CallGraph, files: &BTreeMap<String, ParsedFile>) -> ArgIndex;
```

Read facts from parsed files. At the callee occurrence use the binding walk's declaration, its holder identity, runtime position and ordered path, including in-body destructured aliases. Retain the value/default belonging to that declaration. Treat TS `this`, destructuring elisions and duplicate formals as specified in DESIGN §5. Read values from each admitted Exact Source caller with E disabled. Keep values paired with their caller. Use stable Exact terminal witnesses, including inline byte identities, ordered last-winning literal keys and explicit JSX attributes. Run DESIGN §5 E’s final-descriptor and prototype lookup at each path component before selecting a default. Test `{__proto__: {cb: selected}}`, `{"__proto__": {cb: selected}}` and `{["__proto__"]: {cb: selected}}` with `{cb = fallback}`. Pin inherited `selected` at runtime for the first two, prohibit their false fallback, and require `fallback` for the computed own data key on a witnessed ordinary chain miss. Add the full ordinary-prototype key table, nested reads, null-prototype/own undefined controls, getter/setter pairings and data↔accessor replacements. Apply #434 JsFollow terminal proof and JsBinding::Callable at local identifier reads, with independent hoisting/TDZ and declaration-kind checks. Exercise reassigned handler, Annex B in D/E, E TDZ and var-expression terminals. Complete the identifier × initialization/write × scope enumeration before the E1 fold. Defaults run left to right before the body. No source call means no contribution from the default.

Apply the declaration-specific stability rule to writes in initializers, the body and closures. Include assignments, updates, patterns, iteration targets, dynamic scope and the actual live mapped arguments object. Distinguish strict/non-simple, rightmost duplicate, detached index, shadow binding, body-only var, copied value and a mutation of a callable's property. At each entry or in-body destructure, check survival of the value along the selected path up to that read. Preserve a callable already copied before a later container write. Count vetoes by reason. Replay the original M5 candidate keys and compare newly withheld caller contributions and sites for each refined write or path check. Split sites retaining another ground from sites losing their last target, and count any known graded loss before E2 activation. Apply the two-lookup depth limit per path from an empty index, preserve independent grounds through cycles and deduplicate callable identities before applying the per-slot cap of eight. An existing derived edge cannot be a caller.

Change `anonymous::ProofIndex` from the current tuple wrapper into a serializable object with its per-file proofs and a separate default-empty `ArgIndex`. Its accessors continue to read the per-file proofs for anonymous admission. Clear the entire ArgIndex on removal/merge/indirect clear and compute once after all files and proofs are current. Full hit restores the versioned index. On partial hit rebuild on the merged snapshot. A full-graph equality test includes both fields. A test comparing per-file anonymous extraction with a direct subset compares the per-file component. No per-site storage or extra field/initializer in `src/call_graph.rs`.

- [ ] RED against the facts skeleton. Cover each E rule in the row ledger. Include e11, e12, e15, nested key collisions s1-22, ambiguous computed keys, overwritten/defaulted values, before/after writes, direct vs indirect eval, mapped vs copied arguments, bare and anonymous targets with C off, source-kind admission at each relay, grounded vs empty cycles, depth 1/2/3 and 8 vs 9 distinct targets. Test full/partial/cold identity after caller edits and deletion. Keep E resolution disabled in E1.

- [ ] Implement and price E’s stability, terminal identity and lookup subtypes under the §7 E1 → E2 activation gate. On X and T require byte-identical `call-stats --dump-sites` site records and DataFlow edges. Compare aggregate `call-stats` separately; E1's new key is `js_arg_follow` (including per-reason facts and `fanout`). Every older counter remains pinned. Record E matrix results, cache sizes on X/T and the three oversized file sizes.
- [ ] **E1 performance gate.** On T `7964e22f` and the Stage M manifest of 594 SB roots, compare E0 with E1 release binaries on the same host, loader options and thread count. Use `PRISM_PROFILE=1`, cold graph/no cache and five alternating paired builds per corpus. Publish each derived-index phase time and total build time, plus medians (sum SB package times within each pass). E0 has no index phase. Stop if a phase or corpus is unmeasured or the median paired increase in total time exceeds 10% on either corpus. Tier-A, cloud tests and both reviews remain separate gates.

### Slice E2: the argument-follow route (5 days)

**Files:** modify `src/resolution/free.rs` (give `admit_and_filter_family<'a>(&'a self, site: &CallSite, outcome: ResolutionOutcome<'a>) -> ResolutionOutcome<'a>` the graph borrow carried by each `ResolvedCallee.target`; in its body, between `admit_anonymous_exact` and
`retain_family`: `let outcome = self.js_ts_argument_follow(site, outcome);`, so anonymous-caller admission does not
clear ArgumentFollow targets and the family filter and multi-occurrence refusal still apply; DESIGN 5),
`arg_follow.rs` (that method), `src/resolution/kinds.rs` (`ResolutionKind::ArgumentFollow`, `as_str`
"argument_follow"; `DropReason::ArgFanout`), `src/navigation/call_stats.rs` (the `ArgFanout` arm), other exhaustive
matches the compiler reports (`rg -n 'ResolutionKind::ParameterCallback' src crates`; any arm needed in an oversized
file is offset in the same PR or placed in a module), `src/cpg/callee_params.rs` (`compute_param_def_nodes` locates
anonymous callees with the shared `locate_function`, factored here if neither C nor D ran), `tests/integration/js_alias_rule_test.rs` (no
`ArgumentFollow` outcome has `via_alias_route`, #434 §4), the matrix test (`"E2"`), both cache files, docs.

`js_ts_argument_follow` changes only an empty JS-family Source `UnknownName` or `LocalBindingUnproven` outcome for an indexed parameter call/JSX site. Look up the full file, span and callee, then the stable declaration/slot/path. `Targets` gives NameOnly `ArgumentFollow`, `Fanout` gives `ArgFanout` with no target, and `Empty` keeps the original drop. Anonymous-caller admission precedes this lookup, family and multi-occurrence filters follow. Every caller/forwarding hop is an independently admitted singleton Exact Source. Exported and escaping holders retain their witnessed subset.

Grade every payload against the independent target witness before checking its slot (DESIGN §7). Bind payloads independently for each target at A's offsets and B's identifier rest Def, as `nameonly/call_nameonly`. Allow byte-identified bare targets even with ordinary C disabled (e02a). Return-flow stays NameOnly, a D table edge never grounds an E target, and lane 3's Exact-singleton predicate still excludes literal leaves on derived calls (s5-28). Keep declaration and site byte identities when filtering each target and reporting per-site drops.

- [ ] Enable `E2` after corrected same-base runs. For s1-25 #1–2 pin the constructor caller by site and implementation. Report missing ground as blocked/RED, independently exercise both target and payload from a supplied Exact Source caller, and check absent and wrong-holder grounds. Exercise every E rule in the ledger, including mixed D/E rows s3-14, s4-12/13, x2-11 and x4-9, the independent D assertions and admission-pinned E2 pair for s2-15, plus known opaque-value losses. Test e02a with C off, a grounded recursion vs a cycle-only slot, the same-line distinct-label tests and fanout scoped to one of two formals. Every supplied-site test supplies the call's argument nodes and byte spans with the site and its Exact target (ledger "Supplied-site seam": the argument index records none for a `new` expression).
- [ ] `arg_follow_tests::activated_route_parity` compares labelled edge multisets and complete site outcomes on cold, full and partial hits after caller edit, deletion and the 8→9→8 fanout changes.

- [ ] Differential: `argfollow-call` (targets graded `--mode argfollow`, the independent checker ledger; WRONG rate at most 5%) and `argfollow-flow` (target-joined Use → Def, `--mode binding`, independently at most 5%). Self-test correct target/slot, decoy with the same slot, wrong slot, same-line anonymous target, missing/ambiguous joins and a typed FunctionType without a declaration witness. Publish both C/W/U totals and both W/N denominators. Reconcile refusal accounting (Stage I preamble) and expected counter moves (`resolved` up and the two drop reasons down by the same sites, `ArgFanout` drops
      as drop-reason moves, `return_flow_skipped_nameonly`, return-flow certification as DESIGN 7). Require zero changes at sites resolved on main.
- [ ] Merge only after lane 8 I-guard lands and e14 passes with `call_nameonly` (lane 8 n01). Require BoundaryExited/not_exact on its own root/from/to edge and actual descent to handler.p and its body read from a separate variable root through the same-line direct Exact argument.
- [ ] Tier-A `--quick` regressions pasted; benchmark; cloud tests; two reviewers. PR body: M5 beside converted sites,
      sizes of the three oversized files before and after. Report M5's 1,253 payload candidates beside the head, 135 fanout drops (105 with payload), 12 sites with a refused holder caller, and every published loss and disagreement.

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
      chains; D's missing receiver nodes, excluded callback and unsupplied roles (including collection, thisArg and explicit reducer seed); shared-script globals lost to binding
      admission; Python `f(*xs, v)` misbinding (not verified); trace-engine crossing of name-only edges beyond DESIGN 5's
      check (lane 8). Handoff refreshed; raw dumps offloaded; `target/` removed.

- [ ] Reconcile DESIGN §9’s enumerated loss rows, three mandatory RED fixture checks, seven external checks and the 73 M3 UNDECIDED grade keys with the actual differential. Attach the corrected same-base receipt for each changed assertion. Count each eligible site in the parent union only once.

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
