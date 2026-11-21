# Lane 9: finish JavaScript parameter binding — design

Status: the census design (section 6) is final. Which slices run, and their rules, are decided by the census
and fixed at stage D. Tracking issues: #595 (rest and bare-arrow formals), #434 (calls through a callback
parameter), parent #604. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane9-parameter-binding.md`.

## 1. Intent

Close the remaining holes between a call's arguments and the callee's parameters in JavaScript and
TypeScript: rest parameters, unparenthesised arrow parameters, spread arguments, parameters of callbacks
given to higher-order functions, and calls made through a callback parameter. These decide whether a value
can be followed through ordinary code such as `items.forEach(v => use(v))`.

The lane was proposed on benchmark numbers that the research found to be partly stale and partly about a
different mechanism. It therefore starts with a census on the current build, and each slice has its own
go or no-go.

## 2. What exists today (verified on `origin/main` at 1179055f)

| Fact | Reference |
|---|---|
| Arguments bind to parameters by position. Rest parameters and unparenthesised arrow parameters have a definition but are skipped as targets; parenthesised arrow parameters bind. | `src/cpg/build.rs:1567-1690`, `src/cpg/callee_params.rs:155-174`, `src/parameter_slots.rs:476-520` |
| **A spread argument is treated as one positional argument**: `f(...list, v)` into `function f(a, b)` binds `v` to `b`. The spread flag is set only for Go. This is a wrong edge on main. | `src/ast/calls.rs:237-262`; probe |
| The unparenthesised arrow parameter was left unbound on purpose (decision D12): with it bound, a measurement on the TypeScript sources gave 57 new edges of which 26 were graded Exact and 1 was correct. The cause was calls resolving Exact to functions whose name is only inferred from where they are assigned. A later repair refused such targets edge by edge. A minimal reproduction of that cause no longer reproduces on main; the full measurement has not been repeated. | `docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prA.md` (D12, probe P11); probe |
| Functions that read `arguments` have no definition to bind; `arguments` is used only as a refusal signal. | `src/ast/js_write_census/refusals.rs` |
| Callbacks: an inline function passed to `forEach`, `map`, `then`, `Object.keys(x).forEach`, or an event emitter gets no edge into its parameter; the higher-order call itself is unresolved. A call through a callback parameter (`cb(input)` inside `function run(cb)`) is unresolved, so nothing reaches the function that was passed. No model of built-in higher-order methods exists. Anonymous functions are graph nodes, and calls inside them are recorded. | probes; `src/anon_owner.rs`; `src/call_graph.rs:2392` (Go only: parameter-callback targets) |
| The existing plan for #434 resolves `cb()` to the functions that Exact callers pass, as name-only targets. Its census found 13 sites on Excalidraw and 232 on the TypeScript sources for named targets; with anonymous targets 84 and 357. It was deferred because anonymous targets did not exist then; they do now. | branch `plan/js-arg-follow` |
| Counts from earlier censuses: rest parameters 46 (Excalidraw), 134 (TypeScript sources), 534 (benchmark packages); unparenthesised arrow parameters 4, 3,556 and 1,845. Of the calls found inside callbacks, 95% (Excalidraw) and 82% (TypeScript sources) sit in callbacks handed to functions outside the repository. | `docs/superpowers/plans/2026-10-06-js-param-defs/CENSUS.md`; `~/prism-evidence/js-args/` |
| Benchmark first breaks, measured on a build that predates three parameter slices now merged: rest parameters 8 (6 converted by a rewrite, and 6 of 8 reported as tracing after the first of those slices), `arguments` 6 (none converted), plain argument 12 (none), and 97 path-traversal cases whose first break is the registration of an HTTP handler callback (93 converted, 92 only together with a member-read fix that has since merged). No case is first-broken at a built-in higher-order call. The harness seeds the entry parameter directly, so call-site binding cannot change its counts for most cases. | `docs/superpowers/plans/2026-10-04-secbench-ground-truth/R1.md`; research section 4 |

Consequences:

- The benchmark does not justify rest, `arguments` or built-in callback work by itself; corpus counts and a
  type-checker oracle must.
- The 97 handler-registration cases belong to request-source registration (lane 12's follow-up and #600),
  and may already trace on main. The census re-measures them and hands them over; this lane does not build
  for them.
- The spread misbinding is a correctness defect and is fixed regardless of any count.

## 3. Goals, non-goals, anti-goals

Goals:

1. A census on the current build that sizes each shape and re-measures the benchmark.
2. The spread-argument defect fixed.
3. For each remaining shape, a slice that runs only if its gate passes, with every added edge graded by the
   TypeScript checker.
4. Edges through built-in higher-order methods and through callback parameters carry a label that says they
   rest on a name or on a derived target, never `exact`.

Non-goals: `arguments`-object binding (six benchmark cases, no conversion demonstrated); promise chains
beyond a single `.then` callback; event-emitter registration; request-source registration (lane 12);
destructured formals (lane 3).

Anti-goals: lifting the D12 refusal without repeating its measurement; `exact` edges that depend on a
method's name; changing how calls resolve for anything but calls through a parameter.

## 4. Approaches considered

For built-in higher-order methods:

**A. Type-driven** (bind only when the receiver is known to be an array or a promise). Prism has no types
for JavaScript values; this would bind almost nothing. Rejected.

**B. A closed table of method names with an inline or named function argument, labelled name-only**
(chosen, subject to the census): `forEach`, `map`, `filter`, `find`, `findIndex`, `some`, `every`,
`flatMap` (receiver elements into the callback's first parameter), `reduce` and `reduceRight` (into the
second), `then` (the receiver's value into the first). The edge runs from the receiver's use to the
callback parameter's definition. A user-defined method of the same name gets the same edge; that is why
the label is not `exact`.

**C. Treat the callback as called by the enclosing function** (a synthetic call edge with no data). It
helps navigation but moves no data. Not chosen for this lane.

For calls through a parameter: the existing #434 plan (targets derived from what Exact callers pass,
name-only, capped) is adopted as written, with anonymous targets, after its census is repeated.

For unparenthesised arrow parameters: bind them exactly as parenthesised ones are bound, keeping the
per-edge refusal for targets whose name is only inferred, and only if a repeat of the D12 measurement finds
no wrong Exact edge.

## 5. Architecture — *rules tentative*

| Slice | Change | Edge and label |
|---|---|---|
| A. Spread arguments | positional binding stops at the first spread argument: arguments after it bind nothing, the spread itself binds nothing. The spread is detected in the binding step from the argument's syntax; the call site's spread flag is not set for JavaScript, because arity-based resolution reads it | removes wrong edges; adds none directly. Removing a misbound edge can lift an existing per-edge refusal elsewhere (the one that guards new parameter sources), so a few edges can reappear; each is listed |
| B. Rest parameters | every argument at or after the rest parameter's position binds to the rest parameter's definition; a spread at that position binds its source | the call's label, as for other arguments |
| C. Unparenthesised arrow parameters | the D12 hole is lifted; the per-edge refusal for name-inferred targets stays | the call's label |
| D. Built-in higher-order methods | table of section 4 B; the callback is the inline function or a named function resolved Exact in the same file. An inline callback is located by its graph identity: the parameter lookup finds callees by name today and never finds an anonymous one. Whether an unparenthesised arrow callback binds here while slice C's refusal stands is fixed at stage D | `NameOnly`, with a doubt kind of its own (`builtin_callback`) |
| E. Calls through a parameter | the #434 design: a derived index of what Exact callers pass to that parameter gives `cb(x)` its targets; argument binding then follows, through the same identity lookup as slice D for anonymous targets. Three steps: a move-only split that takes the indirect-call code out of the oversized call-graph file, then the facts (read from parsed files, not stored on call sites), then the route | name-only resolution kind `argument_follow`; edges inherit it |

Slices D and E add edges that the trace engine will not cross (it requires Exact resolution) but that
default review taint and the slicing algorithms follow. That asymmetry is lane 8's subject, not this
lane's.

Code goes in new modules under `src/cpg/` (`spread_args.rs`, `rest_params.rs`, `builtin_callbacks.rs`) and,
for slice E, under `src/resolution/js_ts/` as the #434 plan lays out; `src/cpg/build.rs` and
`src/call_graph.rs` take calls only. Where a slice needs lines in a file over the size ceiling
(`src/call_graph.rs`, `src/navigation/queries.rs`), it first moves code out, so the file ends smaller than
it started. Each slice that changes the graph bumps the cache version.

## 6. The census (stage M)

On Excalidraw, the TypeScript sources and the benchmark packages, with the current build and the pinned
TypeScript checker:

| Id | Count | Decides |
|---|---|---|
| M1 | call sites with a spread argument followed by another argument, and how many get a positional edge today (each one is a wrong edge) | size of slice A's removals |
| M2 | call sites that pass arguments at or beyond a rest parameter of an Exact in-repository callee | slice B |
| M3 | the D12 measurement repeated on the current build: edges an unparenthesised-arrow binding would add, graded by the checker | slice C |
| M4 | calls to the table's method names with a function argument: how many callbacks read their first parameter; how many receivers the checker types as an array, a promise, something else, or cannot type | slice D and its wrong-receiver rate |
| M5 | the #434 census repeated with anonymous targets | slice E |
| M6 | the benchmark re-run on the current build, all classes: first-break mechanisms now | hand-over of the handler-registration cases; whether any shape of this lane is a first break |

Go or no-go per slice (defaults):

| Slice | Runs when |
|---|---|
| A | always |
| B | M2 ≥ 300 sites across the three corpora, or M6 shows ≥ 5 cases first-broken at a rest parameter |
| C | M3 finds no checker-WRONG edge graded Exact |
| D | M4 ≥ 1,000 callbacks that read their parameter, and the checker types at least 90% of typed receivers as arrays or promises |
| E | M5 ≥ 300 sites with at least one target across the corpora |

The parent issue's gate for parameter work (at least 30 benchmark traces or 500 sites, with no
checker-wrong row) applies to the lane as a whole: if the slices that pass their own gates do not reach it
together, the lane stops after slice A and reports. A site counts toward the 500 only when every edge added
at it is graded correct.

## 7. Gates for the slices that run

| Gate | Statement |
|---|---|
| Oracle | every added cross-function edge is graded; no WRONG among edges labelled `exact`; for name-only edges the WRONG rate is reported and at most 5%. The existing adjudicator grades every rest-parameter edge as wrong, cannot type receivers and cannot grade edges through a parameter, so the lane extends it with one mode per slice, each checked against hand-labelled rows before it gates anything |
| Differential | only the slice's own classes of change; slice A's removals are each a spread-misbound edge; edges that reappear or disappear through the new-source refusal are classes of their own, listed edge by edge |
| Counters | call-resolution counters move only in slice E |
| Benchmark | traced cases do not decrease in any class |

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Slice thresholds | section 6 | none | at the census |
| 2 | Label of built-in callback edges | name-only with its own doubt kind | making them `exact` would need receiver types | stage D |
| 3 | `arguments`-object binding | out of scope | its own slice if a census shows demand | — |
| 4 | Handler-registration benchmark cases | re-measured here, handed to lane 12's follow-up | — | at M6 |
| 5 | The existing #434 plan | adopted for slice E after its census is repeated | a redesign goes through design escalation | stage D |

## 9. Risks and accepted limits

- **Name-based callback edges can be wrong** for user-defined methods called `map` or `then`. The label
  says so and the census measures how often receivers are not arrays or promises.
- **Slice C reopens a refusal that was added for cause.** It runs only on a clean repeat of that
  measurement and keeps the per-edge refusal.
- **The benchmark may show nothing for this lane.** The census says so before any slice other than A is
  built.
- **Slice E changes call resolution** (new name-only targets), which the accuracy harness will see; its
  regressions are pasted into the PR, not re-baselined.
