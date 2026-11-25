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
| **A spread argument is treated as one positional argument**: `f(...list, v)` into `function f(a, b)` binds `v` to `b` whatever the length of `list`. The spread flag is set only for Go. The edge is right only when the spread yields exactly one element: `f(...[0], v)` → `b` is correct, `f(...[], v)` (runtime slot `a`) and `f(...[0, 1], v)` (slot `c`) are wrong, and for an unknown `list` it is unproved. Builtin `.call`/`.apply` drop today and bind nothing. | `src/ast/calls.rs:237-262`; review probes P1, P15 |
| **Chained calls share an argument list.** The argument index is keyed by (start byte, callee name) and keeps the outermost call, so in `this.m(...xs, v).m(w, z)` the inner `m` (Exact to `A.m`) receives the outer `w, z`: two wrong `exact` edges on main. `cb(x)(y)` gives two sites with one start byte. | `src/ast/call_args.rs:19-25`, `src/cpg/build.rs:1582-1603`; probes P7, P8, P15 |
| **An unimported module export named like a global is an Exact target.** `parseInt(v)` or `setTimeout(f, 0)` in a module that imports nothing resolves `free_single` Exact to another module's export of that name; with fixed-slot formals the positional binding gives a wrong `exact` edge on main. | probes P3, P6, P10 |
| Admitting an argument binding can change return-flow certification (`return_flow_suppression_*` under `call-stats.return_flow`): Step 5c reads the Step 5b edges. | `src/cpg/step5c.rs:254-289`; probe P14 |
| For a member-only formal the only node the graph has for `items` in `items.forEach(..)` is the projection Use `items.forEach` spanning the callee; for a mixed formal it is a zero-width base Use at the receiver's start byte. | `src/data_flow/build/file.rs:215-271`, `src/cpg/build.rs:908-911`; probe P5 |
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
- The spread misbinding is a correctness defect and is fixed regardless of any count; the chain misbinding
  is fixed with it for the JS family (section 5, call identity).
- The unimported-export Exact target is a resolver defect on main. This lane files it, pins it as a known
  wrong row, and never builds a new binding on it (section 5, binding admission); changing call resolution
  for it is out of scope.

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
second), `then` (the receiver's value into the first). The edge runs from the receiver's node (section 5,
slice D) to the callback parameter's definition. A user-defined method of the same name gets the same edge;
that is why the label is not `exact`. The oracle credits an edge only when the checker resolves the call to
the method declared in the TypeScript library files and the fed parameter has the element role there.

**C. Treat the callback as called by the enclosing function** (a synthetic call edge with no data). It
helps navigation but moves no data. Not chosen for this lane.

For calls through a parameter: the existing #434 plan (targets derived from what Exact callers pass,
name-only, capped) is adopted as written, with anonymous targets, after its census is repeated.

For unparenthesised arrow parameters: bind them as parenthesised ones are bound, under the binding
admission of section 5 and keeping the per-edge refusal for targets whose name is only inferred, and only if
a repeat of the D12 measurement finds no wrong Exact edge.

## 5. Architecture — *rules tentative*

Three rules hold for every slice.

- **Call identity.** An argument list, receiver or callback is read from the call node the site resolved
  from, identified by its full byte span (start and end) and callee, never from a start byte or a line. For
  the JS family the binding step's argument lookup moves to that key (slice A), which also removes main's
  chain misbinding (section 2); other languages keep today's key. Chained calls (`a.m(..).m(..)`,
  `cb(x)(y)`, optional and constructor chains) are matrix rows.
- **Runtime offset.** An argument's position is its runtime offset: the sum of the lengths of the spreads
  before it. A spread of an array literal with no nested spread and no holes has its syntactic length; any
  other spread has an unknown length (at least 0). An argument whose offset is exact binds by position; after
  an unknown-length spread positional binding stops (slice A); an argument whose least offset reaches the rest
  index belongs to the rest parameter (slice B). Builtin `.call`/`.apply` stay measured exclusions.
- **Binding admission.** A binding this lane adds (a rest or bare-arrow formal target in B or C, a literal
  leaf into such a formal in C, an Exact caller that feeds argument-follow targets in E) is admitted only when
  the site's resolution proves the binding lexically: never at a cross-file `free_single` site (an
  unqualified name resolved to a declaration in another file; `cross_file_free_single_exact` in the binding
  step). Locals, imports and receivers keep their routes. The cost is legitimate shared-script globals, counted
  by the census. Existing fixed-slot bindings keep main's behaviour; the defect is filed (section 2).

| Slice | Change | Edge and label |
|---|---|---|
| A. Spread arguments | the binding step computes runtime offsets (rule above): arguments after a known-length array-literal spread keep their correct positions; after any other spread positional binding stops; the spread itself binds nothing (whether a literal's own elements bind is a stage D item). The spread is detected from the argument's syntax; the call site's spread flag is not set for JavaScript, because arity-based resolution reads it. The JS-family argument lookup uses the call identity rule | removes proved-wrong and unproved edges and the chain-misbound ones; moves a misplaced known-length edge to its correct formal. It removes no edge whose offset is exact and correct; an unproved removal can be correct at run time, and the oracle reports every removed edge as proved-wrong, unproved or correct (a correct removal is a STOP). Removing an edge can change the per-edge refusal that guards new parameter sources (section 7, refusal accounting); each such change is listed |
| B. Rest parameters | an argument whose least runtime offset is at or after the rest index binds to the rest parameter's definition; a spread there binds its source where stage D admits it; an argument whose offset is unknown and may fall before the index binds nothing. A rest edge means "an element of", not array identity. Binding admission applies | the call's label, as for other arguments |
| C. Unparenthesised arrow parameters | the D12 hole is lifted under binding admission; the per-edge refusal for name-inferred targets stays | the call's label |
| D. Built-in higher-order methods | table of section 4 B for the JS family; the callback is the inline function or a named function resolved Exact in the same file. An inline callback is located by its graph identity: the parameter lookup finds callees by name today and never finds an anonymous one. The receiver node is the one the graph has (section 2): a nonzero Use of the receiver path inside the receiver span; else the projection Use of `receiver.method` spanning the callee member expression; else a zero-width base Use at the receiver's start byte; else no edge (counted). Whether an unparenthesised arrow callback binds here while slice C's refusal stands is fixed at stage D | `NameOnly`, with a doubt kind of its own (`builtin_callback`) |
| E. Calls through a parameter | the #434 design: a derived index of what admitted Exact callers pass to that parameter gives `cb(x)` its targets; argument binding then follows, through the same identity lookup as slice D for anonymous targets. The holder's formal is found from the binder's answer at the call occurrence (the declaration found, then its runtime slot and destructuring key), not from the walk's position. The route runs after anonymous-caller admission and before the family filter, so a valid multi-target site inside an anonymous callable is not cleared. Three steps: a move-only split of the oversized files, then the facts (read from parsed files, not stored on call sites), then the route | name-only resolution kind `argument_follow`; edges inherit it |

Slices D and E add edges that the trace engine must not cross (it requires Exact resolution) but that
default review taint and the slicing algorithms follow. The trace engine matches a binding edge to a call by
line, so a name-only edge could borrow another call's Exact resolution on the same line (`cb(x); handler(0);`,
probe P13). The check that prevents it: the descent gate refuses a binding edge whose label carries doubt
`call_nameonly` or `builtin_callback`. The gate is lane 8's code (`descend_target`); this lane asks lane 8
for the check at stage D, and D and E2 do not merge until rows h13 and e14 hold. The wider asymmetry stays
lane 8's subject.

Code goes in new modules under `src/cpg/` (`spread_args.rs`, `rest_params.rs`, `builtin_callbacks.rs`) and,
for slice E, under `src/resolution/js_ts/` as the #434 plan lays out; `src/cpg/build.rs` and
`src/call_graph.rs` take calls only. Every PR satisfies the size rule on its own: a slice that needs lines in
a file over the ceiling (`src/call_graph.rs`, `src/resolution.rs`, `src/navigation/queries.rs`) places them
so that file does not grow in that PR, after a separate move-only PR where needed. Each slice that changes
the graph bumps the cache version.

## 6. The census (stage M)

On Excalidraw, the TypeScript sources and the benchmark packages, with the current build and the pinned
TypeScript checker:

Gates count conversions, not sites: a site counts only when it can receive an added edge from a variable
source (an argument with a Use node prism can bind) into a target with a Def, at a site the final resolver
and binding admission accept. Calls that pass only literals never count. Sites, emitted rows and graded
rows are reported apart.

| Id | Count | Decides |
|---|---|---|
| M1 | call sites with a spread argument followed by another argument, by resolution; the rows each gets today (one site can give two rows: `o` and `o.data`), each classified by runtime offset as correct, proved-wrong or unproved; chain-misbound rows (call identity) apart | size and kind of slice A's removals |
| M2 | admitted sites of an Exact in-repository callee with an identifier rest formal whose Def exists, passing a variable argument whose least runtime offset reaches the rest index; cross-file `free_single` sites counted apart (binding admission) | slice B |
| M3 | the D12 measurement repeated on the current build: edges an unparenthesised-arrow binding would add, graded by the checker, with the cross-file `free_single` class apart (binding admission removes it); distinct sites whose added rows are all CORRECT | slice C |
| M4 | calls to the table's method names with a function argument whose fed formal has a Def and is read, and whose receiver node exists (section 5, D); per method and receiver class: the checker's declaration from the TypeScript library files, an in-repository declaration, or untyped | slice D and its wrong-receiver rate |
| M5 | the #434 census repeated with anonymous targets: admitted sites with a payload argument that has a Use node, at least one target from an admitted Exact caller, and an outcome the final admission keeps; `ArgFanout` drops counted as drop-reason moves, not resolved sites | slice E |
| M6 | the benchmark re-run on the current build, all classes: first-break mechanisms now | hand-over of the handler-registration cases; first breaks at a lane shape as opportunities, not conversions |

Go or no-go per slice (defaults):

| Slice | Runs when |
|---|---|
| A | always |
| B | M2 ≥ 300 sites across the three corpora, or M6 shows ≥ 5 cases first-broken at a missing incoming edge into an existing rest Def |
| C | M3 finds no checker-WRONG edge graded Exact among the edges binding admission keeps |
| D | M4 ≥ 1,000 callbacks, and at least 90% of the typed receivers resolve to the library method |
| E | M5 ≥ 300 sites across the corpora |

The parent issue's gate for parameter work (at least 30 benchmark traces or 500 sites) applies to the lane
as a whole, counted in conversions: the 30 are cases traced at the head and not at the base on the same
inputs (losses listed beside gains); the 500 are the union of stable site ids at which at least one row was
added and every added row is graded CORRECT. The parent gate's "no checker-wrong row" is met through the
oracle gate of section 7 (zero WRONG among `exact`; name-only at most 5%) and the site rule (a site with a
wrong or undecided row earns no credit). At M-4 the gate is a forecast that decides spending; it is
re-forecast after slice A and after B and C; if the forecast fails, the lane stops after slice A and reports.
The definitive count is taken on the merged head at close-out; a failure there goes to the owner.

## 7. Gates for the slices that run

| Gate | Statement |
|---|---|
| Oracle | every added or removed cross-function edge is graded by a grader independent of the product: added `exact` edges allow zero WRONG; name-only edges at most 5% WRONG, rate reported; only CORRECT rows earn credit. Transfers are graded by shape: formal binding (Use → Def, by runtime offset, runtime implementation behind TS overloads, TS `this` occupying no slot, constructors), literal leaf → read (Use → Use), call target (E: holder callers resolved by the checker, not by the census; a contrary declaration is WRONG, a missing witness UNDECIDED) and built-in callback (D: the library declaration and the slot role). Each mode is checked against hand-labelled rows (a positive, a wrong target or slot, an undecided case) before it gates anything |
| Differential | only the slice's own classes of change; slice A's removals are each spread-misbound (proved-wrong or unproved) or chain-misbound; refusal accounting: an outgoing edge of a source the new-source refusal guards that reappears or disappears is listed with the reason, a seed change or a reachability change, its seed and its witness path before and after, in every slice (a D or E edge can connect a guarded source to an existing seed) |
| Counters | per slice, a listed set may move: call-resolution counters only in E2; return-flow certification (`return_flow_suppression_*`) in A to E when a binding it depends on is added or removed, each change reconstructed; the slice's own label counter in D. Every other movement fails |
| Benchmark | traced cases do not decrease in any class; gains and losses listed by case id |

The Oracle gate's grading uses lane 11's oracle: lane 9 adopts the measurement kit's oracle (owner,
2026-11-24), and until that oracle has merged the lane grades with its own checker, migrating when a bound
replay of its baseline agrees (lane 11 DESIGN section 6, G-adopt). The formats to write the question module
against are in `lane11-measurement-kit/FORMATS-PROVISIONAL.md` until lane 11's stage D writes `FORMATS.md`.

## 8. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Slice thresholds | section 6 | none | at the census |
| 2 | Label of built-in callback edges | name-only with its own doubt kind | making them `exact` would need receiver types | stage D |
| 3 | `arguments`-object binding | out of scope | its own slice if a census shows demand | — |
| 4 | Handler-registration benchmark cases | re-measured here, handed to lane 12's follow-up | — | at M6 |
| 5 | The existing #434 plan | adopted for slice E after its census is repeated | a redesign goes through design escalation | stage D |
| 6 | An added `exact` edge the grader leaves UNDECIDED | no credit, listed with a reason | withholding it needs a product refusal per shape | stage D |

## 9. Risks and accepted limits

- **Name-based callback edges can be wrong** for user-defined methods called `map` or `then`, or an
  instance override of `forEach` on a typed array. The label says so, and the census measures how often the
  checker's declaration is not the library's.
- **Slice C reopens a refusal that was added for cause.** It runs only on a clean repeat of that
  measurement and keeps the per-edge refusal.
- **The benchmark may show nothing for this lane.** The census says so before any slice other than A is
  built.
- **Slice E changes call resolution** (new name-only targets), which the accuracy harness will see; its
  regressions are pasted into the PR, not re-baselined.
- **Unproved spread removals can be correct.** `f(...list, v)` with a one-element `list` loses a correct
  edge. Accepted: the edge was never provable, and the oracle reports every such removal.
- **Binding admission loses shared-script globals** (a script calling a function another script declares).
  The census counts them; a proof of a common script environment would recover them.
- **Defects on main stay open.** The unimported-export Exact target (section 2) keeps its wrong edges on
  fixed-slot formals and its wrong callers; the lane pins them, files them, and adds nothing on top.

Recorded, not pursued:

- Builtin `thisArg`, a bound callback's prepended arguments, reducer accumulator seeding without an
  initializer, and the whole-array callback slot: roles outside the table's element slot, recorded by M4 and
  left to the stage D enumeration.
- A trace or label replay matrix for every route (partial replay after target deletion, fanout 8 to 9, E with
  B, C or D disabled): E2's activated-route parity test covers the caller edit and fanout transition only.
- An `exact` row the grader leaves UNDECIDED earns no credit and is listed with a reason; whether it should
  also be withheld from the product is an owner question (section 8).
